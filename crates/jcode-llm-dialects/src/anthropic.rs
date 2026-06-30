//! Anthropic Inband Scanner — XML `&lt;invoke&gt;` / `&lt;function_calls&gt;` dialect.
//!
//! The model emits tool calls inside XML tags:
//!
//! ```xml
//! <function_calls>
//!   <invoke name="get_weather">
//!     <parameter name="city">NYC</parameter>
//!   </invoke>
//! </function_calls>
//! ```
//!
//! Optional `<thinking>`/`<think>`/`<scratchpad>` blocks (including `antml:`-prefixed
//! variants) are also parsed. Self-closing tags are supported.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::types::{InbandScanEvent, InbandScanner, InbandScannerOptions};

// ---------------------------------------------------------------------------
// Configuration constants
// ---------------------------------------------------------------------------

const MAX_PARTIAL_TAG_LENGTH: usize = 256;

/// Tags that act as wrapper sections.
const WRAPPER_LOCAL_NAMES: &[&str] = &["function_calls", "tool_calls"];

/// Tags that indicate thinking blocks.
const THINKING_LOCAL_NAMES: &[&str] = &["thinking", "think", "scratchpad"];

/// All tag prefixes for the outside state.
const BASE_TAG_PREFIXES: &[&str] = &[
    "<function_calls",
    "</function_calls",
    "<tool_calls",
    "</tool_calls",
    "<invoke",
    "</invoke",
    "<parameter",
    "</parameter",
    "<antml:function_calls",
    "</antml:function_calls",
    "<antml:tool_calls",
    "</antml:tool_calls",
    "<antml:invoke",
    "</antml:invoke",
    "<antml:parameter",
    "</antml:parameter",
];

const THINKING_TAG_PREFIXES: &[&str] = &[
    "<thinking", "</thinking", "<think", "</think", "<scratchpad", "</scratchpad",
    "<antml:thinking", "</antml:thinking", "<antml:think", "</antml:think",
    "<antml:scratchpad", "</antml:scratchpad",
];

/// Combined list for when thinking parsing is enabled.
fn all_tag_prefixes() -> Vec<&'static str> {
    let mut v = Vec::with_capacity(BASE_TAG_PREFIXES.len() + THINKING_TAG_PREFIXES.len());
    v.extend_from_slice(BASE_TAG_PREFIXES);
    v.extend_from_slice(THINKING_TAG_PREFIXES);
    v
}

// ---------------------------------------------------------------------------
// Tag parsing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct ParsedTag {
    raw: String,
    local_name: String,
    prefix: String,
    closing: bool,
    self_closing: bool,
    attrs: HashMap<String, String>,
}

fn parse_tag(raw: &str) -> Option<ParsedTag> {
    let raw = raw.trim();
    if !raw.starts_with('<') || !raw.ends_with('>') {
        return None;
    }
    let inner = &raw[1..raw.len() - 1].trim();
    if inner.is_empty() {
        return None;
    }

    let closing = inner.starts_with('/');
    let inner = if closing { &inner[1..].trim() } else { inner };

    let self_closing = inner.ends_with('/');
    let inner = if self_closing {
        &inner[..inner.len() - 1].trim()
    } else {
        inner
    };

    // Split tag name from attributes
    let (tag_name, attrs_str) = match inner.find(char::is_whitespace) {
        Some(pos) => (&inner[..pos], inner[pos..].trim()),
        None => (inner, ""),
    };

    if tag_name.is_empty() {
        return None;
    }

    // Extract prefix and local name
    let (prefix, local_name) = if let Some(colon_pos) = tag_name.find(':') {
        (
            tag_name[..colon_pos].to_string(),
            tag_name[colon_pos + 1..].to_lowercase(),
        )
    } else {
        (String::new(), tag_name.to_lowercase())
    };

    // Parse attributes
    let attrs = parse_attributes(attrs_str);

    Some(ParsedTag {
        raw: raw.to_string(),
        local_name,
        prefix,
        closing,
        self_closing,
        attrs,
    })
}

fn parse_attributes(text: &str) -> HashMap<String, String> {
    let mut attrs = HashMap::new();
    let mut i = 0;
    let bytes = text.as_bytes();
    while i < bytes.len() {
        // Skip whitespace
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        // Read attribute name (alphanumeric, _, :, ., -)
        let name_start = i;
        while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b':' || bytes[i] == b'.' || bytes[i] == b'-') {
            i += 1;
        }
        let name = &text[name_start..i];
        if name.is_empty() {
            i += 1;
            continue;
        }
        // Skip whitespace and =
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i < bytes.len() && bytes[i] == b'=' {
            i += 1;
        } else {
            continue;
        }
        // Skip whitespace after =
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        // Read value: quoted or unquoted
        if bytes[i] == b'"' || bytes[i] == b'\'' {
            let quote = bytes[i];
            i += 1;
            let val_start = i;
            while i < bytes.len() && bytes[i] != quote {
                i += 1;
            }
            let value = &text[val_start..i];
            if i < bytes.len() {
                i += 1; // skip closing quote
            }
            attrs.insert(name.to_lowercase(), value.to_string());
        } else {
            let val_start = i;
            while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'>' {
                i += 1;
            }
            let value = &text[val_start..i];
            attrs.insert(name.to_lowercase(), value.to_string());
        }
    }
    attrs
}

/// Build list of closing prefixes for a given local name and prefix.
fn close_prefixes(local_name: &str, prefix: &str) -> Vec<String> {
    let mut prefixes = Vec::new();
    let unprefixed = format!("</{}", local_name);
    let antml = format!("</antml:{}", local_name);
    if prefix.is_empty() || prefix == "antml" {
        prefixes.push(unprefixed);
        prefixes.push(antml);
    } else {
        prefixes.push(format!("</{}:{}", prefix, local_name));
        prefixes.push(unprefixed);
        prefixes.push(antml);
    }
    prefixes
}

/// Check if buffer could be the start of a relevant tag prefix.
fn could_be_tag_prefix(buf: &str, prefixes: &[String]) -> bool {
    if !buf.starts_with('<') {
        return false;
    }
    for p in prefixes {
        if p.starts_with(buf) || buf.starts_with(p.as_str()) {
            return true;
        }
    }
    false
}

fn could_be_tag_prefix_static(buf: &str, prefixes: &[&str]) -> bool {
    if !buf.starts_with('<') {
        return false;
    }
    for p in prefixes {
        if p.starts_with(buf) || buf.starts_with(p) {
            return true;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// State machine
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScannerState {
    Outside,
    Section,
    Invoke,
    Parameter,
    Thinking,
}

type ReturnState = ScannerState;

fn is_wrapper(name: &str) -> bool {
    WRAPPER_LOCAL_NAMES.contains(&name)
}

fn is_thinking(name: &str) -> bool {
    THINKING_LOCAL_NAMES.contains(&name)
}

/// Streaming scanner for the Anthropic XML dialect.
pub struct AnthropicInbandScanner {
    buffer: String,
    state: ScannerState,
    return_state: ReturnState,
    after_thinking_state: ReturnState,
    id: String,
    name: String,
    args: HashMap<String, serde_json::Value>,
    started: bool,
    param_name: String,
    param_value: String,
    param_string: Option<bool>,
    param_close_prefixes: Vec<String>,
    raw_block: String,
    thinking: String,
    thinking_tag: String,
    thinking_close_prefixes: Vec<String>,
    parse_thinking: bool,
}

impl AnthropicInbandScanner {
    pub fn new(options: &InbandScannerOptions) -> Self {
        Self {
            buffer: String::new(),
            state: ScannerState::Outside,
            return_state: ScannerState::Outside,
            after_thinking_state: ScannerState::Outside,
            id: String::new(),
            name: String::new(),
            args: HashMap::new(),
            started: false,
            param_name: String::new(),
            param_value: String::new(),
            param_string: None,
            param_close_prefixes: Vec::new(),
            raw_block: String::new(),
            thinking: String::new(),
            thinking_tag: String::new(),
            thinking_close_prefixes: Vec::new(),
            parse_thinking: options.parse_thinking,
        }
    }

    fn gen_id() -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        format!("anthropic_{:09x}", nanos)
    }

    fn reset_call(&mut self, next: ReturnState) {
        self.id.clear();
        self.name.clear();
        self.args.clear();
        self.started = false;
        self.param_name.clear();
        self.param_value.clear();
        self.param_string = None;
        self.param_close_prefixes.clear();
        self.raw_block.clear();
        self.state = next;
    }

    fn is_thinking_open(&self, tag: &ParsedTag) -> bool {
        self.parse_thinking && !tag.closing && is_thinking(&tag.local_name)
    }

    fn relevant_prefixes(&self) -> Vec<&'static str> {
        if self.parse_thinking {
            all_tag_prefixes()
        } else {
            BASE_TAG_PREFIXES.to_vec()
        }
    }

    /// Try to peek at the next tag in the buffer.
    fn peek_tag(&self, final_: bool, prefixes: &[String]) -> Option<ParsedTag> {
        let close = self.buffer.find('>')?;
        let raw = &self.buffer[..=close];
        if let Some(tag) = parse_tag(raw) {
            return Some(tag);
        }
        // Check for partial
        if !final_
            && self.buffer.len() <= MAX_PARTIAL_TAG_LENGTH
            && could_be_tag_prefix(&self.buffer, prefixes)
        {
            return None; // "partial" marker — just return None
        }
        None
    }

    fn peek_tag_static(&self, final_: bool, prefixes: &[&str]) -> Option<ParsedTag> {
        let close = self.buffer.find('>')?;
        let raw = &self.buffer[..=close];
        if let Some(tag) = parse_tag(raw) {
            return Some(tag);
        }
        if !final_
            && self.buffer.len() <= MAX_PARTIAL_TAG_LENGTH
            && could_be_tag_prefix_static(&self.buffer, prefixes)
        {
            return None;
        }
        None
    }

    fn emit_text(&self, text: &str, events: &mut Vec<InbandScanEvent>) {
        if !text.is_empty() {
            events.push(InbandScanEvent::Text(text.to_string()));
        }
    }

    fn start_invoke(&mut self, tag: &ParsedTag, return_state: ReturnState, events: &mut Vec<InbandScanEvent>) {
        self.return_state = return_state;
        self.id = Self::gen_id();
        self.name = tag.attrs.get("name").map(|s| s.trim().to_string()).unwrap_or_default();
        self.args = HashMap::new();
        self.raw_block = tag.raw.clone();
        self.started = !self.name.is_empty();
        self.state = ScannerState::Invoke;
        // NOTE: Do NOT emit ToolStart per spec
    }

    fn start_parameter(&mut self, tag: &ParsedTag) {
        self.param_name = tag.attrs.get("name").map(|s| s.trim().to_string()).unwrap_or_default();
        self.param_value = String::new();
        self.param_string = parse_bool_attr(tag.attrs.get("string").map(|s| s.as_str()));
        self.param_close_prefixes = close_prefixes("parameter", &tag.prefix);
        self.state = ScannerState::Parameter;
    }

    fn append_parameter_value(&mut self, delta: &str) {
        self.param_value.push_str(delta);
    }

    fn finish_parameter(&mut self) {
        if !self.param_name.is_empty() {
            let value = self.coerce_parameter_value(&self.param_name, &self.param_value, self.param_string);
            self.args.insert(self.param_name.clone(), value);
        }
        self.param_name.clear();
        self.param_value.clear();
        self.param_string = None;
        self.param_close_prefixes.clear();
        self.state = ScannerState::Invoke;
    }

    fn coerce_parameter_value(&self, _name: &str, raw: &str, explicit_string: Option<bool>) -> serde_json::Value {
        if explicit_string == Some(true) {
            return serde_json::Value::String(raw.to_string());
        }
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return serde_json::Value::String(raw.to_string());
        }
        // Try parsing as JSON
        serde_json::from_str(trimmed).unwrap_or_else(|_| serde_json::Value::String(raw.to_string()))
    }

    fn start_thinking(&mut self, tag: &ParsedTag, after_state: ReturnState, events: &mut Vec<InbandScanEvent>) {
        self.after_thinking_state = after_state;
        self.thinking.clear();
        self.thinking_tag = tag.local_name.clone();
        self.thinking_close_prefixes = close_prefixes(&tag.local_name, &tag.prefix);
        self.state = ScannerState::Thinking;
        events.push(InbandScanEvent::ThinkingStart);
        if tag.self_closing {
            self.finish_thinking(events);
        }
    }

    fn append_thinking(&mut self, delta: &str, events: &mut Vec<InbandScanEvent>) {
        if delta.is_empty() {
            return;
        }
        self.thinking.push_str(delta);
        events.push(InbandScanEvent::ThinkingDelta(delta.to_string()));
    }

    fn finish_thinking(&mut self, events: &mut Vec<InbandScanEvent>) {
        events.push(InbandScanEvent::ThinkingEnd(self.thinking.clone()));
        self.thinking.clear();
        self.thinking_tag.clear();
        self.thinking_close_prefixes.clear();
        self.state = self.after_thinking_state;
        self.after_thinking_state = ScannerState::Outside;
    }
}

impl InbandScanner for AnthropicInbandScanner {
    fn feed(&mut self, text: &str) -> Vec<InbandScanEvent> {
        if text.is_empty() {
            return vec![];
        }
        self.buffer.push_str(text);
        self.consume(false)
    }

    fn flush(&mut self) -> Vec<InbandScanEvent> {
        let mut events = self.consume(true);
        // Close any pending state
        match self.state {
            ScannerState::Thinking => {
                self.finish_thinking(&mut events);
            }
            ScannerState::Outside => {}
            _ => {
                self.reset_call(self.return_state);
            }
        }
        if !self.buffer.is_empty() {
            events.push(InbandScanEvent::Text(self.buffer.clone()));
            self.buffer.clear();
        }
        self.state = ScannerState::Outside;
        events
    }
}

impl AnthropicInbandScanner {
    fn consume(&mut self, final_: bool) -> Vec<InbandScanEvent> {
        let mut events = Vec::new();
        loop {
            if self.buffer.is_empty() {
                break;
            }
            let progressed = match self.state {
                ScannerState::Outside => self.consume_outside(final_, &mut events),
                ScannerState::Section => self.consume_section(final_, &mut events),
                ScannerState::Invoke => self.consume_invoke(final_, &mut events),
                ScannerState::Parameter => self.consume_parameter(final_),
                ScannerState::Thinking => self.consume_thinking(final_, &mut events),
            };
            if !progressed {
                break;
            }
        }
        if final_ {
            match self.state {
                ScannerState::Thinking => self.finish_thinking(&mut events),
                ScannerState::Outside => {}
                _ => self.reset_call(self.return_state),
            }
        }
        events
    }

    fn consume_outside(&mut self, final_: bool, events: &mut Vec<InbandScanEvent>) -> bool {
        // Find next '<'
        let tag_start = self.buffer.find('<');
        let tag_start = match tag_start {
            Some(p) => p,
            None => {
                // No tag at all — emit remaining text
                self.emit_text(&self.buffer, events);
                self.buffer.clear();
                return false;
            }
        };

        if tag_start > 0 {
            self.emit_text(&self.buffer[..tag_start], events);
            self.buffer = self.buffer[tag_start..].to_string();
            return true;
        }

        // Buffer starts with '<' — try to peek a tag
        let prefixes = self.relevant_prefixes();
        let tag = self.peek_tag_static(final_, &prefixes);
        match tag {
            None => {
                // Could be partial or just a '<' character
                if !final_
                    && self.buffer.len() <= MAX_PARTIAL_TAG_LENGTH
                    && could_be_tag_prefix_static(&self.buffer, &prefixes)
                {
                    return false;
                }
                // Not a valid tag — emit '<' as text
                self.emit_text("<", events);
                self.buffer = self.buffer[1..].to_string();
                return true;
            }
            Some(tag) => {
                if !tag.closing && is_wrapper(&tag.local_name) {
                    self.buffer = self.buffer[tag.raw.len()..].to_string();
                    self.state = ScannerState::Section;
                    return true;
                }
                if !tag.closing && tag.local_name == "invoke" {
                    self.buffer = self.buffer[tag.raw.len()..].to_string();
                    self.start_invoke(&tag, ScannerState::Outside, events);
                    return true;
                }
                if self.is_thinking_open(&tag) {
                    self.buffer = self.buffer[tag.raw.len()..].to_string();
                    self.start_thinking(&tag, ScannerState::Outside, events);
                    return true;
                }
                if tag.closing && is_wrapper(&tag.local_name) {
                    self.buffer = self.buffer[tag.raw.len()..].to_string();
                    return true;
                }
                // Unrecognized tag — emit as text
                self.emit_text("<", events);
                self.buffer = self.buffer[1..].to_string();
                return true;
            }
        }
    }

    fn consume_section(&mut self, final_: bool, _events: &mut Vec<InbandScanEvent>) -> bool {
        let tag_start = self.buffer.find('<');
        let tag_start = match tag_start {
            Some(p) => p,
            None => {
                self.buffer.clear();
                return false;
            }
        };

        if tag_start > 0 {
            self.buffer = self.buffer[tag_start..].to_string();
            return true;
        }

        let prefixes = self.relevant_prefixes();
        let tag = self.peek_tag_static(final_, &prefixes);
        match tag {
            None => {
                if !final_
                    && self.buffer.len() <= MAX_PARTIAL_TAG_LENGTH
                    && could_be_tag_prefix_static(&self.buffer, &prefixes)
                {
                    return false;
                }
                self.buffer = self.buffer[1..].to_string();
                return true;
            }
            Some(tag) => {
                self.buffer = self.buffer[tag.raw.len()..].to_string();
                if tag.closing && is_wrapper(&tag.local_name) {
                    self.state = ScannerState::Outside;
                    return true;
                }
                if !tag.closing && tag.local_name == "invoke" {
                    self.start_invoke(&tag, ScannerState::Section, _events);
                    return true;
                }
                if self.parse_thinking && !tag.closing && is_thinking(&tag.local_name) {
                    self.start_thinking(&tag, ScannerState::Section, _events);
                }
                return true;
            }
        }
    }

    fn consume_invoke(&mut self, final_: bool, events: &mut Vec<InbandScanEvent>) -> bool {
        let tag_start = self.buffer.find('<');
        let tag_start = match tag_start {
            Some(p) => p,
            None => {
                if final_ {
                    self.reset_call(self.return_state);
                } else {
                    self.raw_block.push_str(&self.buffer);
                    self.buffer.clear();
                }
                return false;
            }
        };

        if tag_start > 0 {
            let consumed = &self.buffer[..tag_start];
            self.raw_block.push_str(consumed);
            self.buffer = self.buffer[tag_start..].to_string();
            return true;
        }

        let prefixes: Vec<String> = vec!["</invoke".to_string(), "</antml:invoke".to_string(),
            "<parameter".to_string(), "<antml:parameter".to_string()];
        let tag = self.peek_tag(final_, &prefixes);
        match tag {
            None => {
                if !final_ {
                    if self.buffer.len() <= MAX_PARTIAL_TAG_LENGTH
                        && could_be_tag_prefix(&self.buffer, &prefixes)
                    {
                        return false;
                    }
                } else {
                    self.reset_call(self.return_state);
                    return false;
                }
                let consumed = &self.buffer[..1];
                self.raw_block.push_str(consumed);
                self.buffer = self.buffer[1..].to_string();
                return true;
            }
            Some(tag) => {
                self.raw_block.push_str(&tag.raw);
                self.buffer = self.buffer[tag.raw.len()..].to_string();
                if tag.closing && tag.local_name == "invoke" {
                    if self.started {
                        let args_val = serde_json::Value::Object(
                            std::mem::take(&mut self.args).into_iter().collect(),
                        );
                        events.push(InbandScanEvent::ToolEnd {
                            id: self.id.clone(),
                            name: self.name.clone(),
                            arguments: args_val,
                            raw_block: Some(self.raw_block.clone()),
                        });
                    }
                    self.reset_call(self.return_state);
                    return true;
                }
                if !tag.closing && tag.local_name == "parameter" {
                    self.start_parameter(&tag);
                    if tag.self_closing {
                        self.finish_parameter();
                    }
                    return true;
                }
                return true;
            }
        }
    }

    fn consume_parameter(&mut self, final_: bool) -> bool {
        let tag_start = self.buffer.find('<');
        let tag_start = match tag_start {
            Some(p) => p,
            None => {
                if final_ {
                    self.reset_call(self.return_state);
                    self.buffer.clear();
                    return false;
                }
                self.append_parameter_value(&self.buffer);
                self.raw_block.push_str(&self.buffer);
                self.buffer.clear();
                return false;
            }
        };

        if tag_start > 0 {
            let consumed = &self.buffer[..tag_start];
            self.append_parameter_value(consumed);
            self.raw_block.push_str(consumed);
            self.buffer = self.buffer[tag_start..].to_string();
            return true;
        }

        let tag = self.peek_tag(final_, &self.param_close_prefixes);
        match tag {
            None => {
                if !final_
                    && self.buffer.len() <= MAX_PARTIAL_TAG_LENGTH
                    && could_be_tag_prefix(&self.buffer, &self.param_close_prefixes)
                {
                    return false;
                }
                if final_ {
                    self.reset_call(self.return_state);
                    self.buffer.clear();
                    return false;
                }
                let consumed = &self.buffer[..1];
                self.append_parameter_value(consumed);
                self.raw_block.push_str(consumed);
                self.buffer = self.buffer[1..].to_string();
                return true;
            }
            Some(tag) => {
                if tag.closing && tag.local_name == "parameter" {
                    self.raw_block.push_str(&tag.raw);
                    self.buffer = self.buffer[tag.raw.len()..].to_string();
                    self.finish_parameter();
                    return true;
                }
                if final_ {
                    self.reset_call(self.return_state);
                    self.buffer.clear();
                    return false;
                }
                let consumed = &self.buffer[..1];
                self.append_parameter_value(consumed);
                self.raw_block.push_str(consumed);
                self.buffer = self.buffer[1..].to_string();
                return true;
            }
        }
    }

    fn consume_thinking(&mut self, final_: bool, events: &mut Vec<InbandScanEvent>) -> bool {
        let tag_start = self.buffer.find('<');
        let tag_start = match tag_start {
            Some(p) => p,
            None => {
                if final_ {
                    self.append_thinking(&self.buffer, events);
                    self.buffer.clear();
                    self.finish_thinking(events);
                    return false;
                }
                self.append_thinking(&self.buffer, events);
                self.buffer.clear();
                return false;
            }
        };

        if tag_start > 0 {
            self.append_thinking(&self.buffer[..tag_start], events);
            self.buffer = self.buffer[tag_start..].to_string();
            return true;
        }

        let tag = self.peek_tag(final_, &self.thinking_close_prefixes);
        match tag {
            None => {
                if !final_
                    && self.buffer.len() <= MAX_PARTIAL_TAG_LENGTH
                    && could_be_tag_prefix(&self.buffer, &self.thinking_close_prefixes)
                {
                    return false;
                }
                if final_ {
                    self.append_thinking(&self.buffer, events);
                    self.buffer.clear();
                    self.finish_thinking(events);
                    return false;
                }
                self.append_thinking(&self.buffer[..1], events);
                self.buffer = self.buffer[1..].to_string();
                return true;
            }
            Some(tag) => {
                if tag.closing && tag.local_name == self.thinking_tag {
                    self.buffer = self.buffer[tag.raw.len()..].to_string();
                    self.finish_thinking(events);
                    return true;
                }
                if final_ {
                    self.append_thinking(&self.buffer, events);
                    self.buffer.clear();
                    self.finish_thinking(events);
                    return false;
                }
                self.append_thinking(&self.buffer[..1], events);
                self.buffer = self.buffer[1..].to_string();
                return true;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_bool_attr(value: Option<&str>) -> Option<bool> {
    let value = value?;
    let normalized = value.trim().to_lowercase();
    if normalized == "false" || normalized == "0" || normalized == "no" {
        return Some(false);
    }
    Some(true)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_text() {
        let mut scanner = AnthropicInbandScanner::new(&InbandScannerOptions::default());
        let events = scanner.feed("Hello, this is plain text.");
        let flushed = scanner.flush();
        let all: Vec<_> = events.into_iter().chain(flushed).collect();
        assert_eq!(all.len(), 1);
        assert!(matches!(&all[0], InbandScanEvent::Text(t) if t == "Hello, this is plain text."));
    }

    #[test]
    fn test_xml_tool_call() {
        let mut scanner = AnthropicInbandScanner::new(&InbandScannerOptions::default());
        let input = "What's the weather?<function_calls><invoke name=\"get_weather\"><parameter name=\"city\">NYC</parameter></invoke></function_calls>";
        let events = scanner.feed(input);
        let flushed = scanner.flush();
        let all: Vec<_> = events.into_iter().chain(flushed).collect();
        assert!(all.iter().any(|e| matches!(e, InbandScanEvent::Text(t) if t == "What's the weather?")),
            "expected text, got {all:?}");
        assert!(all.iter().any(|e| matches!(e, InbandScanEvent::ToolEnd { name, .. } if name == "get_weather")),
            "expected ToolEnd for get_weather, got {all:?}");
        if let Some(InbandScanEvent::ToolEnd { arguments, .. }) = all.iter().find(|e| matches!(e, InbandScanEvent::ToolEnd { .. })) {
            assert_eq!(arguments.get("city").and_then(|v| v.as_str()), Some("NYC"));
        }
    }

    #[test]
    fn test_chunked_xml() {
        let mut scanner = AnthropicInbandScanner::new(&InbandScannerOptions::default());
        scanner.feed("<function_calls><invoke name=\"get_weather\"><parameter name=\"city\">");
        let events = scanner.feed("NYC");
        let events2 = scanner.feed("</parameter></invoke></function_calls>");
        let flushed = scanner.flush();
        let all: Vec<_> = events.into_iter().chain(events2).chain(flushed).collect();
        assert!(all.iter().any(|e| matches!(e, InbandScanEvent::ToolEnd { name, .. } if name == "get_weather")),
            "expected ToolEnd for get_weather, got {all:?}");
    }

    #[test]
    fn test_thinking_block() {
        let mut scanner = AnthropicInbandScanner::new(&InbandScannerOptions {
            parse_thinking: true,
            ..Default::default()
        });
        let input = "Let me <thinking>ponder deeply</thinking>done";
        let events = scanner.feed(input);
        let flushed = scanner.flush();
        let all: Vec<_> = events.into_iter().chain(flushed).collect();
        assert!(all.iter().any(|e| matches!(e, InbandScanEvent::ThinkingStart)),
            "expected ThinkingStart, got {all:?}");
        assert!(all.iter().any(|e| matches!(e, InbandScanEvent::ThinkingEnd(t) if t == "ponder deeply")),
            "expected ThinkingEnd, got {all:?}");
    }

    #[test]
    fn test_self_closing_invoke() {
        let mut scanner = AnthropicInbandScanner::new(&InbandScannerOptions::default());
        let input = "<function_calls><invoke name=\"get_weather\"/></function_calls>";
        let events = scanner.feed(input);
        let flushed = scanner.flush();
        let all: Vec<_> = events.into_iter().chain(flushed).collect();
        // Self-closing invoke with no parameters should produce ToolEnd
        assert!(all.iter().any(|e| matches!(e, InbandScanEvent::ToolEnd { name, .. } if name == "get_weather")),
            "expected ToolEnd for get_weather, got {all:?}");
    }

    #[test]
    fn test_self_closing_parameter() {
        let mut scanner = AnthropicInbandScanner::new(&InbandScannerOptions::default());
        let input = "<function_calls><invoke name=\"get_weather\"><parameter name=\"city\"/></invoke></function_calls>";
        let events = scanner.feed(input);
        let flushed = scanner.flush();
        let all: Vec<_> = events.into_iter().chain(flushed).collect();
        assert!(all.iter().any(|e| matches!(e, InbandScanEvent::ToolEnd { name, .. } if name == "get_weather")),
            "expected ToolEnd for get_weather, got {all:?}");
        if let Some(InbandScanEvent::ToolEnd { arguments, .. }) = all.iter().find(|e| matches!(e, InbandScanEvent::ToolEnd { .. })) {
            assert!(arguments.get("city").is_some(),
                "expected city arg to exist, got {arguments:?}");
        }
    }

    #[test]
    fn test_mixed_namespace() {
        let mut scanner = AnthropicInbandScanner::new(&InbandScannerOptions::default());
        let input = "<antml:function_calls><antml:invoke name=\"get_weather\"><antml:parameter name=\"city\">NYC</antml:parameter></antml:invoke></antml:function_calls>";
        let events = scanner.feed(input);
        let flushed = scanner.flush();
        let all: Vec<_> = events.into_iter().chain(flushed).collect();
        assert!(all.iter().any(|e| matches!(e, InbandScanEvent::ToolEnd { name, .. } if name == "get_weather")),
            "expected ToolEnd for get_weather, got {all:?}");
    }

    #[test]
    fn test_antml_thinking_block() {
        let mut scanner = AnthropicInbandScanner::new(&InbandScannerOptions {
            parse_thinking: true,
            ..Default::default()
        });
        let input = "<antml:thinking>deep thoughts</antml:thinking>done";
        let events = scanner.feed(input);
        let flushed = scanner.flush();
        let all: Vec<_> = events.into_iter().chain(flushed).collect();
        assert!(all.iter().any(|e| matches!(e, InbandScanEvent::ThinkingStart)),
            "expected ThinkingStart, got {all:?}");
        assert!(all.iter().any(|e| matches!(e, InbandScanEvent::ThinkingEnd(t) if t == "deep thoughts")),
            "expected ThinkingEnd, got {all:?}");
    }

    #[test]
    fn test_flush() {
        let mut scanner = AnthropicInbandScanner::new(&InbandScannerOptions::default());
        let events = scanner.feed("some leftover without close");
        let flushed = scanner.flush();
        let all: Vec<_> = events.into_iter().chain(flushed).collect();
        assert!(all.iter().any(|e| matches!(e, InbandScanEvent::Text(t) if t == "some leftover without close")),
            "expected text, got {all:?}");
    }

    #[test]
    fn test_arg_delta_via_multi_invoke() {
        let mut scanner = AnthropicInbandScanner::new(&InbandScannerOptions::default());
        let input = "<function_calls>\
            <invoke name=\"func_a\"><parameter name=\"x\">1</parameter></invoke>\
            <invoke name=\"func_b\"><parameter name=\"y\">2</parameter></invoke>\
            </function_calls>";
        let events = scanner.feed(input);
        let flushed = scanner.flush();
        let all: Vec<_> = events.into_iter().chain(flushed).collect();
        let tool_ends: Vec<_> = all.iter().filter_map(|e| {
            if let InbandScanEvent::ToolEnd { name, .. } = e { Some(name.as_str()) } else { None }
        }).collect();
        assert_eq!(tool_ends, vec!["func_a", "func_b"]);
    }
}
