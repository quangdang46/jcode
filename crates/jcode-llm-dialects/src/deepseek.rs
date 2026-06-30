//! DeepSeek Inband Scanner — DSML with fullwidth delimiters, ASCII DSML, and legacy JSON.
//!
//! Three formats are parsed:
//!
//! 1. **Fullwidth token format** (legacy):
//!    `<\u{ff5c}tool\u{2581}calls\u{2581}begin\u{ff5c}><\u{ff5c}tool\u{2581}call\u{2581}begin\u{ff5c}>name<\u{ff5c}tool\u{2581}sep\u{ff5c}>JSON<\u{ff5c}tool\u{2581}call\u{2581}end\u{ff5c}><\u{ff5c}tool\u{2581}calls\u{2581}end\u{ff5c}>`
//! 2. **DSML format** (fullwidth or ASCII):
//!    `<\u{ff5c}DSML\u{ff5c}tool_calls><\u{ff5c}DSML\u{ff5c}invoke name="x"><\u{ff5c}DSML\u{ff5c}parameter name="k">v</\u{ff5c}DSML\u{ff5c}parameter></\u{ff5c}DSML\u{ff5c}invoke></\u{ff5c}DSML\u{ff5c}tool_calls>`
//!    (or ASCII `<|DSML|tool_calls>` etc.)
//! 3. **Legacy JSON format**:
//!    `<\u{ff5c}tool\u{2581}call\u{2581}begin\u{ff5c}>function<\u{ff5c}tool\u{2581}sep\u{ff5c}>name\n```json\nJSON\n```<\u{ff5c}tool\u{2581}call\u{2581}end\u{ff5c}>`
//!
//! Control tokens (BOS, EOS, User, Assistant) are stripped.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::types::{InbandScanEvent, InbandScanner, InbandScannerOptions};

// ---------------------------------------------------------------------------
// Tokens — fullwidth (3-byte Unicode) and ASCII
// ---------------------------------------------------------------------------

const TOOL_CALLS_BEGIN: &str = "<\u{ff5c}tool\u{2581}calls\u{2581}begin\u{ff5c}>";
const TOOL_CALLS_END: &str = "<\u{ff5c}tool\u{2581}calls\u{2581}end\u{ff5c}>";
const TOOL_CALL_BEGIN: &str = "<\u{ff5c}tool\u{2581}call\u{2581}begin\u{ff5c}>";
const TOOL_CALL_END: &str = "<\u{ff5c}tool\u{2581}call\u{2581}end\u{ff5c}>";
const TOOL_SEPARATOR: &str = "<\u{ff5c}tool\u{2581}sep\u{ff5c}>";

const BOS: &str = "<\u{ff5c}begin\u{2581}of\u{2581}sentence\u{ff5c}>";
const USER: &str = "<\u{ff5c}User\u{ff5c}>";
const ASSISTANT: &str = "<\u{ff5c}Assistant\u{ff5c}>";
const EOS: &str = "<\u{ff5c}end\u{2581}of\u{2581}sentence\u{ff5c}>";

const THINK_OPEN: &str = "<think>";
const THINK_CLOSE: &str = "</think>";

const LEGACY_TOOL_TYPE: &str = "function";
const LEGACY_JSON_FENCE: &str = "```json";
const CODE_FENCE: &str = "```";

const DSML_TOOL_CALLS_OPEN_FULLWIDTH: &str = "<\u{ff5c}DSML\u{ff5c}tool_calls>";
const DSML_TOOL_CALLS_CLOSE_FULLWIDTH: &str = "</\u{ff5c}DSML\u{ff5c}tool_calls>";
const DSML_TOOL_CALLS_OPEN_ASCII: &str = "<|DSML|tool_calls>";
const DSML_TOOL_CALLS_CLOSE_ASCII: &str = "</|DSML|tool_calls>";

/// Control tokens that get stripped entirely.
const CONTROL_TOKENS: &[&str] = &[
    BOS, EOS, USER, ASSISTANT,
    "<\u{ff5c}\u{2581}pad\u{2581}\u{ff5c}>",
    "<|EOT|>",
    "<\u{ff5c}search\u{2581}begin\u{ff5c}>",
    "<\u{ff5c}search\u{2581}end\u{ff5c}>",
    "<\u{ff5c}fim\u{2581}hole\u{ff5c}>Tok",
];

/// Tokens scanned for in the `Outside` state.
const OUTSIDE_TOKENS: &[&str] = &[
    TOOL_CALLS_BEGIN,
    TOOL_CALL_BEGIN,
    THINK_OPEN,
    DSML_TOOL_CALLS_OPEN_FULLWIDTH,
    DSML_TOOL_CALLS_OPEN_ASCII,
    BOS, EOS, USER, ASSISTANT,
    "<\u{ff5c}\u{2581}pad\u{2581}\u{ff5c}>",
    "<|EOT|>",
    "<\u{ff5c}search\u{2581}begin\u{ff5c}>",
    "<\u{ff5c}search\u{2581}end\u{ff5c}>",
    "<\u{ff5c}fim\u{2581}hole\u{ff5c}>Tok",
];

/// Tokens scanned for in the legacy `Section` state.
const SECTION_TOKENS: &[&str] = &[TOOL_CALLS_END, TOOL_CALL_BEGIN];

/// Tokens scanned for in the DSML section state.
const DSML_SECTION_TOKENS: &[&str] = &[
    DSML_TOOL_CALLS_CLOSE_FULLWIDTH,
    DSML_TOOL_CALLS_CLOSE_ASCII,
    "<\u{ff5c}DSML\u{ff5c}invoke",
    "<|DSML|invoke",
    "<\u{ff5c}DSML\u{ff5c}parameter",
    "<|DSML|parameter",
];

/// Tokens scanned for in DSML invoke state (close + parameter open).
const DSML_INVOKE_TOKENS: &[&str] = &[
    "</\u{ff5c}DSML\u{ff5c}invoke>",
    "</|DSML|invoke>",
    "<\u{ff5c}DSML\u{ff5c}parameter",
    "<|DSML|parameter",
];

/// Tokens that close a DSML parameter value (parameter close tags).
const DSML_PARAMETER_CLOSE_TOKENS: &[&str] = &[
    "</\u{ff5c}DSML\u{ff5c}parameter>",
    "</|DSML|parameter>",
];

/// A parsed DSML open tag (invoke or parameter).
#[derive(Debug, Clone)]
struct DsmlOpenTag {
    name: String,
    string_attr: Option<String>,
    raw: String,
    tag_len: usize,
}

#[derive(Debug, Clone)]
enum TokInfo {
    None,
    SelfClosing,
    /// Consumed an opening tag like `<｜DSML｜invoke name="x">`.
    Opening(String, /* raw tag */ String),
    /// Found an unknown token — emit it as text.
    Unknown,
}

impl TokInfo {
    fn is_some(&self) -> bool {
        !matches!(self, TokInfo::None)
    }
}

// ---------------------------------------------------------------------------
// State machine
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Outside,
    Thinking,
    Section,
    Header,
    LegacyName,
    Args,
    LegacyArgs,
    DsmlSection,
    DsmlInvoke,
    DsmlParam,
}

/// Streaming scanner for the DeepSeek dialect.
pub struct DeepSeekInbandScanner {
    buffer: String,
    state: State,
    parse_thinking: bool,
    in_tool_section: bool,
    id: String,
    name: String,
    thinking: String,
    dsml_args: serde_json::Map<String, serde_json::Value>,
    dsml_param_name: String,
    dsml_param_is_string: bool,
    raw_block: String,
    strip_leading_ws: bool,
}

impl DeepSeekInbandScanner {
    pub fn new(options: &InbandScannerOptions) -> Self {
        Self {
            buffer: String::new(),
            state: State::Outside,
            parse_thinking: options.parse_thinking,
            in_tool_section: false,
            id: String::new(),
            name: String::new(),
            thinking: String::new(),
            dsml_args: serde_json::Map::new(),
            dsml_param_name: String::new(),
            dsml_param_is_string: true,
            raw_block: String::new(),
            strip_leading_ws: false,
        }
    }

    fn gen_id() -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        format!("deepseek_{:09x}", nanos)
    }

    fn reset_tool(&mut self, next: State) {
        self.id.clear();
        self.name.clear();
        self.raw_block.clear();
        self.state = next;
    }

    fn reset_dsml_tool(&mut self) {
        self.id.clear();
        self.name.clear();
        self.dsml_args.clear();
        self.dsml_param_name.clear();
        self.dsml_param_is_string = true;
        self.raw_block.clear();
    }

    fn skip_whitespace(&mut self) -> String {
        let trimmed = self.buffer.trim_start();
        let skipped_len = self.buffer.len() - trimmed.len();
        let skipped = self.buffer[..skipped_len].to_string();
        self.buffer = trimmed.to_string();
        skipped
    }

    fn drop_one_linebreak(&mut self) -> String {
        if self.buffer.starts_with("\u{d}\u{a}") {
            let s = "\u{d}\u{a}".to_string();
            self.buffer = self.buffer[2..].to_string();
            s
        } else if self.buffer.starts_with('\u{a}') {
            let s = "\u{a}".to_string();
            self.buffer = self.buffer[1..].to_string();
            s
        } else {
            String::new()
        }
    }

    fn emit_thinking(&mut self, delta: &str, events: &mut Vec<InbandScanEvent>) {
        if delta.is_empty() {
            return;
        }
        if self.parse_thinking {
            self.thinking.push_str(delta);
            events.push(InbandScanEvent::ThinkingDelta(delta.to_string()));
        } else {
            events.push(InbandScanEvent::Text(delta.to_string()));
        }
    }

    fn end_thinking(&mut self, events: &mut Vec<InbandScanEvent>) {
        if self.parse_thinking {
            events.push(InbandScanEvent::ThinkingEnd(self.thinking.clone()));
        }
        self.thinking.clear();
        self.state = State::Outside;
    }

    fn parse_args(&self, raw: &str) -> serde_json::Value {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return serde_json::Value::Object(serde_json::Map::new());
        }
        serde_json::from_str(trimmed).unwrap_or(serde_json::Value::Object(serde_json::Map::new()))
    }

    /// Find the earliest matching token in buffer and return position + token.
    fn find_earliest<'a>(&self, tokens: &'a [&str]) -> Option<(usize, &'a str)> {
        let mut best_pos = None;
        let mut best_tok: Option<&'a str> = None;
        for tok in tokens {
            if let Some(pos) = self.buffer.find(tok) {
                match best_pos {
                    None => {
                        best_pos = Some(pos);
                        best_tok = Some(tok);
                    }
                    Some(bp) if pos < bp || (pos == bp && tok.len() > best_tok.unwrap().len()) => {
                        best_pos = Some(pos);
                        best_tok = Some(tok);
                    }
                    _ => {}
                }
            }
        }
        best_pos.zip(best_tok)
    }

    /// Match a DSML open tag (`<\u{ff5c}DSML\u{ff5c}invoke` or `<\u{ff5c}DSML\u{ff5c}parameter`) with name attribute.
    fn match_dsml_open(&self, kind: &str) -> Option<DsmlOpenTag> {
        let fullwidth_tag = format!("<\u{ff5c}DSML\u{ff5c}{}", kind);
        let ascii_tag = format!("<|DSML|{}", kind);
        if !self.buffer.starts_with(&fullwidth_tag) && !self.buffer.starts_with(&ascii_tag) {
            return None;
        }
        let close = self.buffer.find('>')?;
        let tag = &self.buffer[..=close];
        // Extract name attribute
        let name = extract_attr(tag, "name")?;
        let string_attr = extract_attr(tag, "string");
        Some(DsmlOpenTag {
            name: name.to_string(),
            string_attr,
            raw: tag.to_string(),
            tag_len: close + 1,
        })
    }

    /// Check for DSML closing tag (fullwidth or ASCII variant).
    fn matching_dsml_close(&self, full: &'static str, ascii: &'static str) -> Option<&'static str> {
        if self.buffer.starts_with(full) {
            Some(full)
        } else if self.buffer.starts_with(ascii) {
            Some(ascii)
        } else {
            None
        }
    }

    /// Check if buffer starts with a known DSML invoke/parameter open tag.
    fn is_dsml_open(&self) -> bool {
        self.buffer.starts_with("<\u{ff5c}DSML\u{ff5c}invoke")
            || self.buffer.starts_with("<|DSML|invoke")
            || self.buffer.starts_with("<\u{ff5c}DSML\u{ff5c}parameter")
            || self.buffer.starts_with("<|DSML|parameter")
    }
}

impl InbandScanner for DeepSeekInbandScanner {
    fn feed(&mut self, text: &str) -> Vec<InbandScanEvent> {
        if text.is_empty() {
            return vec![];
        }
        self.buffer.push_str(text);
        self.consume(false)
    }

    fn flush(&mut self) -> Vec<InbandScanEvent> {
        let mut events = self.consume(true);
        if self.state == State::Thinking {
            self.end_thinking(&mut events);
        }
        if !self.buffer.is_empty() {
            events.push(InbandScanEvent::Text(self.buffer.clone()));
            self.buffer.clear();
        }
        self.state = State::Outside;
        events
    }
}

impl DeepSeekInbandScanner {
    fn consume(&mut self, final_: bool) -> Vec<InbandScanEvent> {
        let mut events = Vec::new();
        loop {
            if self.buffer.is_empty() {
                break;
            }
            match self.state {
                State::Outside => self.consume_outside(final_, &mut events),
                State::Thinking => self.consume_thinking(final_, &mut events),
                State::Section => {
                    if !self.consume_section(final_) {
                        break;
                    }
                    continue;
                }
                State::Header => {
                    if !self.consume_header(final_, &mut events) {
                        break;
                    }
                    continue;
                }
                State::LegacyName => {
                    if !self.consume_legacy_name(final_, &mut events) {
                        break;
                    }
                    continue;
                }
                State::Args | State::LegacyArgs => {
                    if !self.consume_args(final_, &mut events) {
                        break;
                    }
                    continue;
                }
                State::DsmlSection => {
                    if !self.consume_dsml_section(final_, &mut events) {
                        break;
                    }
                    continue;
                }
                State::DsmlInvoke => {
                    if !self.consume_dsml_invoke(final_, &mut events) {
                        break;
                    }
                    continue;
                }
                State::DsmlParam => {
                    if !self.consume_dsml_param(final_) {
                        break;
                    }
                    continue;
                }
            }
            // If we returned from consume_outside or consume_thinking without a state change,
            // and state is still the old one, we break
            break;
        }
        if final_ && self.state == State::Thinking {
            self.end_thinking(&mut events);
        }
        events
    }

    fn consume_outside(&mut self, final_: bool, events: &mut Vec<InbandScanEvent>) {
        loop {
            if self.buffer.is_empty() {
                return;
            }

            // Handle pending whitespace stripping after control tokens
            if self.strip_leading_ws {
                let trimmed = self.buffer.trim_start();
                let skipped = self.buffer.len() - trimmed.len();
                self.buffer = trimmed.to_string();
                self.strip_leading_ws = false;
                if self.buffer.is_empty() {
                    return;
                }
            }

            // Find earliest token
            let match_ = self.find_earliest(&OUTSIDE_TOKENS);
            let match_ = match match_ {
                Some(m) => m,
                None => {
                    let hold = if final_ {
                        0
                    } else {
                        partial_suffix_overlap_any(&self.buffer, &OUTSIDE_TOKENS)
                    };
                    let emit_end = self.buffer.len().saturating_sub(hold);
                    if emit_end > 0 {
                        events.push(InbandScanEvent::Text(self.buffer[..emit_end].to_string()));
                    }
                    self.buffer = self.buffer[emit_end..].to_string();
                    return;
                }
            };

            let (pos, token) = match_;

            // Emit text before the token
            if pos > 0 {
                events.push(InbandScanEvent::Text(self.buffer[..pos].to_string()));
            }
            self.buffer = self.buffer[pos..].to_string();

            // Dispatch on token
            if self.buffer.starts_with(TOOL_CALLS_BEGIN) {
                self.buffer = self.buffer[TOOL_CALLS_BEGIN.len()..].to_string();
                self.in_tool_section = true;
                self.state = State::Section;
                return;
            }

            if self.buffer.starts_with(TOOL_CALL_BEGIN) {
                self.buffer = self.buffer[TOOL_CALL_BEGIN.len()..].to_string();
                self.raw_block = TOOL_CALL_BEGIN.to_string();
                self.in_tool_section = false;
                self.state = State::Header;
                return;
            }

            if self.buffer.starts_with(THINK_OPEN) {
                self.buffer = self.buffer[THINK_OPEN.len()..].to_string();
                self.state = State::Thinking;
                self.thinking.clear();
                if self.parse_thinking {
                    events.push(InbandScanEvent::ThinkingStart);
                }
                return;
            }

            if self.buffer.starts_with(DSML_TOOL_CALLS_OPEN_FULLWIDTH)
                || self.buffer.starts_with(DSML_TOOL_CALLS_OPEN_ASCII)
            {
                let open = if self.buffer.starts_with(DSML_TOOL_CALLS_OPEN_FULLWIDTH) {
                    DSML_TOOL_CALLS_OPEN_FULLWIDTH
                } else {
                    DSML_TOOL_CALLS_OPEN_ASCII
                };
                self.buffer = self.buffer[open.len()..].to_string();
                self.state = State::DsmlSection;
                return;
            }

            // Check for control tokens
            if let Some(ctrl) = self.matching_control_token() {
                self.buffer = self.buffer[ctrl.len()..].to_string();
                self.strip_leading_ws = true;
                continue;
            }

            // Some other non-control token (shouldn't happen since find_earliest found it)
            self.buffer = self.buffer[token.len()..].to_string();
        }
    }

    fn consume_thinking(&mut self, final_: bool, events: &mut Vec<InbandScanEvent>) {
        if let Some(pos) = self.buffer.find(THINK_CLOSE) {
            let delta = self.buffer[..pos].to_string();
            self.emit_thinking(&delta, events);
            self.buffer = self.buffer[(pos + THINK_CLOSE.len())..].to_string();
            self.end_thinking(events);
            return;
        }
        // No close found
        let hold = if final_ {
            0
        } else {
            partial_suffix_overlap_any(&self.buffer, &[THINK_CLOSE])
        };
        let emit_end = self.buffer.len().saturating_sub(hold);
        if emit_end > 0 {
            let delta = self.buffer[..emit_end].to_string();
            self.emit_thinking(&delta, events);
        }
        self.buffer = self.buffer[emit_end..].to_string();
        if final_ {
            self.end_thinking(events);
        }
    }

    fn consume_section(&mut self, final_: bool) -> bool {
        loop {
            if self.buffer.is_empty() {
                return final_;
            }
            self.skip_whitespace();
            if self.buffer.starts_with(TOOL_CALLS_END) {
                self.buffer = self.buffer[TOOL_CALLS_END.len()..].to_string();
                self.in_tool_section = false;
                self.state = State::Outside;
                return true;
            }
            if self.buffer.starts_with(TOOL_CALL_BEGIN) {
                self.buffer = self.buffer[TOOL_CALL_BEGIN.len()..].to_string();
                self.raw_block = TOOL_CALL_BEGIN.to_string();
                self.state = State::Header;
                return true;
            }
            if !final_
                && partial_suffix_overlap_any(&self.buffer, &SECTION_TOKENS) == self.buffer.len()
            {
                return false;
            }
            if self.buffer.is_empty() {
                return false;
            }
            // Consume one char to advance past garbage
            let next = self.buffer.chars().next().map(|c| c.len_utf8()).unwrap_or(1);
            self.buffer = self.buffer[next..].to_string();
        }
    }

    fn consume_header(&mut self, final_: bool, events: &mut Vec<InbandScanEvent>) -> bool {
        let sep_pos = self.buffer.find(TOOL_SEPARATOR);
        let sep_pos = match sep_pos {
            Some(p) => p,
            None => {
                if final_ {
                    self.reset_tool(State::Outside);
                }
                return false;
            }
        };

        let raw_head = &self.buffer[..sep_pos + TOOL_SEPARATOR.len()];
        let head = self.buffer[..sep_pos].trim().to_string();
        self.raw_block.push_str(raw_head);
        self.buffer = self.buffer[raw_head.len()..].to_string();

        if head == LEGACY_TOOL_TYPE {
            self.state = State::LegacyName;
            return true;
        }

        self.name = head;
        self.id = Self::gen_id();
        self.state = State::Args;
        true
    }

    fn consume_legacy_name(&mut self, final_: bool, events: &mut Vec<InbandScanEvent>) -> bool {
        let fence = self.buffer.find(LEGACY_JSON_FENCE);
        let fence = match fence {
            Some(f) => f,
            None => {
                if final_ {
                    self.reset_tool(State::Outside);
                }
                return false;
            }
        };

        let raw_name = &self.buffer[..fence + LEGACY_JSON_FENCE.len()];
        let name = self.buffer[..fence].trim().to_string();
        self.raw_block.push_str(raw_name);
        self.buffer = self.buffer[raw_name.len()..].to_string();
        let lb = self.drop_one_linebreak();
        self.raw_block.push_str(&lb);

        self.name = name;
        self.id = Self::gen_id();
        self.state = State::LegacyArgs;
        true
    }

    fn consume_args(&mut self, final_: bool, events: &mut Vec<InbandScanEvent>) -> bool {
        let end = self.buffer.find(TOOL_CALL_END);
        let end = match end {
            Some(e) => e,
            None => {
                if final_ {
                    self.reset_tool(if self.in_tool_section {
                        State::Section
                    } else {
                        State::Outside
                    });
                }
                return false;
            }
        };

        let mut raw_args = self.buffer[..end].to_string();
        if self.state == State::LegacyArgs {
            // Strip trailing code fence
            if let Some(fence_pos) = raw_args.rfind(CODE_FENCE) {
                raw_args = raw_args[..fence_pos].to_string();
            }
        }

        let raw_tail = &self.buffer[..end + TOOL_CALL_END.len()];
        self.raw_block.push_str(raw_tail);
        self.buffer = self.buffer[raw_tail.len()..].to_string();

        let args = self.parse_args(&raw_args);
        events.push(InbandScanEvent::ToolEnd {
            id: self.id.clone(),
            name: self.name.clone(),
            arguments: args,
            raw_block: Some(self.raw_block.clone()),
        });

        let next = if self.in_tool_section {
            State::Section
        } else {
            State::Outside
        };
        self.reset_tool(next);
        true
    }

    fn consume_dsml_section(&mut self, final_: bool, events: &mut Vec<InbandScanEvent>) -> bool {
        loop {
            if self.buffer.is_empty() {
                return final_;
            }
            self.skip_whitespace();

            // Check for DSML section close
            if let Some(close) =
                self.matching_dsml_close(DSML_TOOL_CALLS_CLOSE_FULLWIDTH, DSML_TOOL_CALLS_CLOSE_ASCII)
            {
                self.buffer = self.buffer[close.len()..].to_string();
                self.state = State::Outside;
                return true;
            }

            // Check for invoke opening
            if let Some(open) = self.match_dsml_open("invoke") {
                self.buffer = self.buffer[open.tag_len..].to_string();
                self.raw_block = open.raw.clone();
                self.name = open.name;
                self.id = Self::gen_id();
                self.dsml_args.clear();
                // NOTE: Task says do NOT emit ToolStart — only ToolEnd
                self.state = State::DsmlInvoke;
                return true;
            }

            // Check if we have a partial DSML open tag
            if !final_ && self.is_dsml_open() && !self.buffer.contains('>') {
                return false;
            }
            if !final_
                && partial_suffix_overlap_any(&self.buffer, &DSML_SECTION_TOKENS)
                    == self.buffer.len()
            {
                return false;
            }

            if self.buffer.is_empty() {
                return false;
            }
            let next = self.buffer.chars().next().map(|c| c.len_utf8()).unwrap_or(1);
            self.buffer = self.buffer[next..].to_string();
        }
    }

    fn consume_dsml_invoke(&mut self, final_: bool, events: &mut Vec<InbandScanEvent>) -> bool {
        loop {
            if self.buffer.is_empty() {
                return final_;
            }

            let skipped = self.skip_whitespace();
            if !skipped.is_empty() {
                self.raw_block.push_str(&skipped);
            }

            // Check for invoke close
            if let Some(close) =
                self.matching_dsml_close("</\u{ff5c}DSML\u{ff5c}invoke>", "</|DSML|invoke>")
            {
                self.raw_block.push_str(close);
                self.buffer = self.buffer[close.len()..].to_string();

                let args_val = serde_json::Value::Object(
                    std::mem::take(&mut self.dsml_args),
                );
                events.push(InbandScanEvent::ToolEnd {
                    id: self.id.clone(),
                    name: self.name.clone(),
                    arguments: args_val,
                    raw_block: Some(self.raw_block.clone()),
                });
                self.reset_dsml_tool();
                self.state = State::DsmlSection;
                return true;
            }

            // Check for parameter open
            if let Some(param) = self.match_dsml_open("parameter") {
                self.raw_block.push_str(&param.raw);
                self.dsml_param_name = param.name;
                self.dsml_param_is_string = param.string_attr.as_deref() != Some("false");
                self.buffer = self.buffer[param.tag_len..].to_string();
                self.state = State::DsmlParam;
                return true;
            }

            if !final_ {
                if self.is_dsml_open() && !self.buffer.contains('>') {
                    return false;
                }
                if partial_suffix_overlap_any(&self.buffer, &DSML_INVOKE_TOKENS) == self.buffer.len() {
                    return false;
                }
            }

            if self.buffer.is_empty() {
                return false;
            }
            let ch = self.buffer.chars().next().map(|c| c.len_utf8()).unwrap_or(1);
            let raw_chunk = self.buffer[..ch].to_string();
            self.raw_block.push_str(&raw_chunk);
            self.buffer = self.buffer[ch..].to_string();
        }
    }

    fn consume_dsml_param(&mut self, final_: bool) -> bool {
        let close = self.find_earliest(&DSML_PARAMETER_CLOSE_TOKENS);
        let close = match close {
            Some(c) => c,
            None => {
                if final_ {
                    self.reset_dsml_tool();
                    self.state = State::Outside;
                }
                return false;
            }
        };

        let (pos, token) = close;
        let raw_value = &self.buffer[..pos];
        let value = coerce_dsml_value(raw_value, self.dsml_param_is_string);
        self.dsml_args.insert(self.dsml_param_name.clone(), value);
        self.raw_block.push_str(raw_value);
        self.raw_block.push_str(token);
        self.buffer = self.buffer[(pos + token.len())..].to_string();
        self.dsml_param_name.clear();
        self.dsml_param_is_string = true;
        self.state = State::DsmlInvoke;
        true
    }

    fn matching_control_token(&self) -> Option<&'static str> {
        if self.buffer.starts_with(TOOL_CALLS_END) {
            return Some(TOOL_CALLS_END);
        }
        if self.buffer.starts_with(THINK_CLOSE) {
            return Some(THINK_CLOSE);
        }
        if self.buffer.starts_with(DSML_TOOL_CALLS_CLOSE_FULLWIDTH) {
            return Some(DSML_TOOL_CALLS_CLOSE_FULLWIDTH);
        }
        if self.buffer.starts_with(DSML_TOOL_CALLS_CLOSE_ASCII) {
            return Some(DSML_TOOL_CALLS_CLOSE_ASCII);
        }
        for tok in CONTROL_TOKENS {
            if self.buffer.starts_with(tok) {
                return Some(tok);
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn extract_attr(tag: &str, attr: &str) -> Option<String> {
    let pattern = format!("{}=\x22", attr);
    let start = tag.find(&pattern)?;
    let val_start = start + pattern.len();
    let val_end = tag[val_start..].find('"')?;
    Some(tag[val_start..val_start + val_end].to_string())
}

fn coerce_dsml_value(raw: &str, is_string: bool) -> serde_json::Value {
    if is_string {
        return serde_json::Value::String(raw.to_string());
    }
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return serde_json::Value::String(raw.to_string());
    }
    serde_json::from_str(trimmed).unwrap_or_else(|_| {
        serde_json::Value::String(raw.to_string())
    })
}

fn partial_suffix_overlap_any(buf: &str, tags: &[&str]) -> usize {
    let buf_lower = buf.to_lowercase();
    let mut max_hold = 0usize;
    for tag in tags {
        let tag_lower = tag.to_lowercase();
        let min_len = buf_lower.len().min(tag_lower.len());
        if buf_lower.len() < tag_lower.len() && tag_lower.starts_with(&buf_lower) {
            max_hold = max_hold.max(buf.len());
        } else if min_len > 0
            && tag_lower[..min_len] == buf_lower[buf_lower.len() - min_len..]
        {
            max_hold = max_hold.max(min_len);
        }
    }
    max_hold
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_text() {
        let mut scanner = DeepSeekInbandScanner::new(&InbandScannerOptions::default());
        let events = scanner.feed("Hello, this is plain text.");
        let flushed = scanner.flush();
        let all: Vec<_> = events.into_iter().chain(flushed).collect();
        assert_eq!(all.len(), 1);
        assert!(matches!(&all[0], InbandScanEvent::Text(t) if t == "Hello, this is plain text."));
    }

    #[test]
    fn test_thinking_block() {
        let mut scanner = DeepSeekInbandScanner::new(&InbandScannerOptions {
            parse_thinking: true,
            ..Default::default()
        });
        let input = "Let me think<think>pondering deeply</think>done";
        let events = scanner.feed(input);
        let flushed = scanner.flush();
        let all: Vec<_> = events.into_iter().chain(flushed).collect();
        assert!(all.iter().any(|e| matches!(e, InbandScanEvent::ThinkingStart)),
            "expected ThinkingStart, got {all:?}");
        assert!(all.iter().any(|e| matches!(e, InbandScanEvent::ThinkingEnd(t) if t == "pondering deeply")),
            "expected ThinkingEnd with 'pondering deeply', got {all:?}");
    }

    #[test]
    fn test_control_tokens_are_stripped() {
        let mut scanner = DeepSeekInbandScanner::new(&InbandScannerOptions::default());
        let input = format!("{}Hello, world!{}", BOS, EOS);
        let events = scanner.feed(&input);
        let flushed = scanner.flush();
        let all: Vec<_> = events.into_iter().chain(flushed).collect();
        assert_eq!(all.len(), 1, "expected 1 Text event, got {all:?}");
        assert!(matches!(&all[0], InbandScanEvent::Text(t) if t == "Hello, world!"));
    }

    #[test]
    fn test_strip_whitespace_after_control_tokens() {
        let mut scanner = DeepSeekInbandScanner::new(&InbandScannerOptions::default());
        let input = format!("{}  Hello", ASSISTANT);
        let events = scanner.feed(&input);
        let flushed = scanner.flush();
        let all: Vec<_> = events.into_iter().chain(flushed).collect();
        assert_eq!(all.len(), 1);
        assert!(matches!(&all[0], InbandScanEvent::Text(t) if t == "Hello"),
            "expected 'Hello', got {all:?}");
    }

    #[test]
    fn test_simple_tool_call_fullwidth() {
        let mut scanner = DeepSeekInbandScanner::new(&InbandScannerOptions::default());
        let input = format!(
            "{}{}get_weather{} {{\"city\":\"NYC\"}}{}{}",
            TOOL_CALLS_BEGIN, TOOL_CALL_BEGIN, TOOL_SEPARATOR, TOOL_CALL_END, TOOL_CALLS_END
        );
        let events = scanner.feed(&input);
        let flushed = scanner.flush();
        let all: Vec<_> = events.into_iter().chain(flushed).collect();
        // Should produce: ToolEnd (no ToolStart per spec)
        assert_eq!(all.len(), 1, "expected 1 ToolEnd, got {all:?}");
        match &all[0] {
            InbandScanEvent::ToolEnd { name, arguments, .. } => {
                assert_eq!(name, "get_weather");
                assert_eq!(arguments.get("city").and_then(|v| v.as_str()), Some("NYC"));
            }
            other => panic!("expected ToolEnd, got {other:?}"),
        }
    }

    #[test]
    fn test_flush_remaining_text() {
        let mut scanner = DeepSeekInbandScanner::new(&InbandScannerOptions::default());
        let events = scanner.feed("some leftover");
        let flushed = scanner.flush();
        let all: Vec<_> = events.into_iter().chain(flushed).collect();
        // The text is not inside any special tag, so it should be emitted as Text
        assert!(all.iter().any(|e| matches!(e, InbandScanEvent::Text(t) if t == "some leftover")),
            "expected text, got {all:?}");
    }

    #[test]
    fn test_chunked_tool_call() {
        let mut scanner = DeepSeekInbandScanner::new(&InbandScannerOptions::default());
        let part1 = format!("{}{}get_weather", TOOL_CALLS_BEGIN, TOOL_CALL_BEGIN);
        scanner.feed(&part1);
        let part2 = format!("{} {{\"city\":\"NYC\"}}{}", TOOL_SEPARATOR, TOOL_CALL_END);
        let events = scanner.feed(&part2);
        let part3 = TOOL_CALLS_END;
        let events2 = scanner.feed(part3);
        let flushed = scanner.flush();
        let all: Vec<_> = events.into_iter().chain(events2).chain(flushed).collect();
        assert!(all.iter().any(|e| matches!(e, InbandScanEvent::ToolEnd { name, .. } if name == "get_weather")),
            "expected ToolEnd for get_weather, got {all:?}");
    }

    #[test]
    fn test_legacy_json_tool_call() {
        let mut scanner = DeepSeekInbandScanner::new(&InbandScannerOptions::default());
        let input = format!(
            "{}{}{}{}get_weather\n{} {{\"city\":\"NYC\"}}\n{}{}",
            TOOL_CALLS_BEGIN,
            TOOL_CALL_BEGIN,
            LEGACY_TOOL_TYPE,
            TOOL_SEPARATOR,
            LEGACY_JSON_FENCE,
            CODE_FENCE,
            TOOL_CALL_END,
            TOOL_CALLS_END
        );
        let events = scanner.feed(&input);
        let flushed = scanner.flush();
        let all: Vec<_> = events.into_iter().chain(flushed).collect();
        assert!(all.iter().any(|e| matches!(e, InbandScanEvent::ToolEnd { name, .. } if name == "get_weather")),
            "expected ToolEnd for get_weather, got {all:?}");
        if let Some(InbandScanEvent::ToolEnd { arguments, .. }) = all.iter().find(|e| matches!(e, InbandScanEvent::ToolEnd { .. })) {
            assert_eq!(arguments.get("city").and_then(|v| v.as_str()), Some("NYC"));
        }
    }

    #[test]
    fn test_dsml_format() {
        let mut scanner = DeepSeekInbandScanner::new(&InbandScannerOptions::default());
        let input = format!(
            "{}<|DSML|invoke name=\"get_weather\"><|DSML|parameter name=\"city\">NYC</|DSML|parameter></|DSML|invoke>{}",
            DSML_TOOL_CALLS_OPEN_ASCII, DSML_TOOL_CALLS_CLOSE_ASCII
        );
        let events = scanner.feed(&input);
        let flushed = scanner.flush();
        let all: Vec<_> = events.into_iter().chain(flushed).collect();
        assert!(all.iter().any(|e| matches!(e, InbandScanEvent::ToolEnd { name, .. } if name == "get_weather")),
            "expected ToolEnd for get_weather, got {all:?}");
        if let Some(InbandScanEvent::ToolEnd { arguments, .. }) = all.iter().find(|e| matches!(e, InbandScanEvent::ToolEnd { .. })) {
            assert_eq!(arguments.get("city").and_then(|v| v.as_str()), Some("NYC"));
        }
    }

    #[test]
    fn test_dsml_fullwidth_format() {
        let mut scanner = DeepSeekInbandScanner::new(&InbandScannerOptions::default());
        let input = format!(
            "{}<\u{ff5c}DSML\u{ff5c}invoke name=\"get_weather\"><\u{ff5c}DSML\u{ff5c}parameter name=\"city\">NYC</\u{ff5c}DSML\u{ff5c}parameter></\u{ff5c}DSML\u{ff5c}invoke>{}",
            DSML_TOOL_CALLS_OPEN_FULLWIDTH, DSML_TOOL_CALLS_CLOSE_FULLWIDTH
        );
        let events = scanner.feed(&input);
        let flushed = scanner.flush();
        let all: Vec<_> = events.into_iter().chain(flushed).collect();
        assert!(all.iter().any(|e| matches!(e, InbandScanEvent::ToolEnd { name, .. } if name == "get_weather")),
            "expected ToolEnd for get_weather, got {all:?}");
        if let Some(InbandScanEvent::ToolEnd { arguments, .. }) = all.iter().find(|e| matches!(e, InbandScanEvent::ToolEnd { .. })) {
            assert_eq!(arguments.get("city").and_then(|v| v.as_str()), Some("NYC"));
        }
    }
}
