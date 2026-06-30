//! Kimi Inband Scanner — token-delimited `<|...|>` format.
//!
//! Model emits tool calls inside these markers:
//! `<|tool_calls_section_begin|><|tool_call_begin|>id<|tool_call_argument_begin|>JSON<|tool_call_end|><|tool_calls_section_end|>`
//! Optional `<think>`...`</think>` blocks are also parsed.

use std::time::{SystemTime, UNIX_EPOCH};
use crate::types::{InbandScanEvent, InbandScanner, InbandScannerOptions};

const SECTION_BEGIN: &str = "<|tool_calls_section_begin|>";
const SECTION_END: &str = "<|tool_calls_section_end|>";
const CALL_BEGIN: &str = "<|tool_call_begin|>";
const CALL_END: &str = "<|tool_call_end|>";
const ARG_BEGIN: &str = "<|tool_call_argument_begin|>";
const THINK_OPEN: &str = "<think>";
const THINK_CLOSE: &str = "</think>";

const TOKENS: &[&str] = &[SECTION_BEGIN, SECTION_END, CALL_BEGIN, CALL_END, ARG_BEGIN];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Outside,
    Section,
    Header,
    Args,
    Thinking,
}

/// Streaming scanner for the Kimi dialect.
pub struct KimiInbandScanner {
    buffer: String,
    state: State,
    // Accumulated call state
    call_id: String,
    call_name: String,
    raw_block: String,
    thinking: String,
    parse_thinking: bool,
}

impl KimiInbandScanner {
    pub fn new(options: &InbandScannerOptions) -> Self {
        Self {
            buffer: String::new(),
            state: State::Outside,
            call_id: String::new(),
            call_name: String::new(),
            raw_block: String::new(),
            thinking: String::new(),
            parse_thinking: options.parse_thinking,
        }
    }

    fn gen_id(&self) -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        format!("kimi_{:09x}", nanos)
    }

    fn reset_call(&mut self) {
        self.call_id.clear();
        self.call_name.clear();
        self.raw_block.clear();
    }
}

impl InbandScanner for KimiInbandScanner {
    fn feed(&mut self, text: &str) -> Vec<InbandScanEvent> {
        if text.is_empty() {
            return vec![];
        }
        self.buffer.push_str(text);
        self.consume(false)
    }

    fn flush(&mut self) -> Vec<InbandScanEvent> {
        let mut events = self.consume(true);

        // Close any pending thinking block
        if self.state == State::Thinking {
            events.push(InbandScanEvent::ThinkingEnd(self.thinking.clone()));
            self.thinking.clear();
            self.state = State::Outside;
        }

        // Emit remaining text
        if !self.buffer.is_empty() {
            events.push(InbandScanEvent::Text(self.buffer.clone()));
            self.buffer.clear();
        }
        self.reset_call();
        events
    }
}

impl KimiInbandScanner {
    fn consume(&mut self, final_: bool) -> Vec<InbandScanEvent> {
        let mut events = Vec::new();
        loop {
            match self.state {
                State::Thinking => {
                    if let Some(pos) = self.buffer.find(THINK_CLOSE) {
                        let delta = &self.buffer[..pos];
                        if !delta.is_empty() {
                            self.thinking.push_str(delta);
                            events.push(InbandScanEvent::ThinkingDelta(delta.to_string()));
                        }
                        self.buffer = self.buffer[(pos + THINK_CLOSE.len())..].to_string();
                        events.push(InbandScanEvent::ThinkingEnd(self.thinking.clone()));
                        self.thinking.clear();
                        self.state = State::Outside;
                        continue;
                    } else if final_ {
                        if !self.buffer.is_empty() {
                            events.push(InbandScanEvent::ThinkingDelta(self.buffer.clone()));
                            self.thinking.push_str(&self.buffer);
                        }
                        events.push(InbandScanEvent::ThinkingEnd(self.thinking.clone()));
                        self.thinking.clear();
                        self.buffer.clear();
                        self.state = State::Outside;
                    }
                    return events;
                }

                State::Outside => {
                    // Look for next interesting marker
                    let tok_pos = self.next_token_index();
                    let think_pos = if self.parse_thinking {
                        self.buffer.find(THINK_OPEN)
                    } else {
                        None
                    };
                    let start = match (tok_pos, think_pos) {
                        (Some(t), Some(h)) if h < t => {
                            // Thinking starts first
                            if h > 0 {
                                events.push(InbandScanEvent::Text(self.buffer[..h].to_string()));
                            }
                            self.buffer = self.buffer[(h + THINK_OPEN.len())..].to_string();
                            self.thinking.clear();
                            events.push(InbandScanEvent::ThinkingStart);
                            self.state = State::Thinking;
                            continue;
                        }
                        (Some(p), _) => p,
                        (None, Some(h)) => {
                            if h > 0 {
                                events.push(InbandScanEvent::Text(self.buffer[..h].to_string()));
                            }
                            self.buffer = self.buffer[(h + THINK_OPEN.len())..].to_string();
                            self.thinking.clear();
                            events.push(InbandScanEvent::ThinkingStart);
                            self.state = State::Thinking;
                            continue;
                        }
                        (None, None) => {
                            let hold = if final_ {
                                0
                            } else {
                                partial_suffix_overlap_any(&self.buffer, TOKENS)
                            };
                            let emit_end = self.buffer.len().saturating_sub(hold);
                            if emit_end > 0 {
                                events.push(InbandScanEvent::Text(self.buffer[..emit_end].to_string()));
                            }
                            self.buffer = self.buffer[emit_end..].to_string();
                            return events;
                        }
                    };

                    // Emit text before the marker and transition
                    if start > 0 {
                        events.push(InbandScanEvent::Text(self.buffer[..start].to_string()));
                    }
                    self.buffer = self.buffer[start..].to_string();
                    // Determine marker type
                    if let Some(token) = self.token_at_start() {
                        self.buffer = self.buffer[token.len()..].to_string();
                        if token == SECTION_BEGIN {
                            self.state = State::Section;
                        } else {
                            events.push(InbandScanEvent::Text(token.to_string()));
                        }
                    }
                    continue;
                }

                State::Section => {
                    // Inside a tool calls section — skip whitespace and look for CALL_BEGIN or SECTION_END
                    self.skip_whitespace();
                    if self.buffer.is_empty() {
                        if final_ { self.state = State::Outside; }
                        return events;
                    }
                    if let Some(token) = self.token_at_start() {
                        self.buffer = self.buffer[token.len()..].to_string();
                        if token == SECTION_END {
                            self.state = State::Outside;
                        } else if token == CALL_BEGIN {
                            self.state = State::Header;
                        }
                        // Any other token inside section is just consumed
                        continue;
                    }
                    if !final_ && partial_suffix_overlap_any(&self.buffer, TOKENS) > 0 {
                        return events;
                    }
                    // Consume one char to advance
                    self.buffer = self.buffer[1..].to_string();
                }

                State::Header => {
                    // Reading the tool call ID/name until ARG_BEGIN
                    if let Some(pos) = self.buffer.find(ARG_BEGIN) {
                        let raw_header = self.buffer[..pos].trim().to_string();
                        self.call_id = raw_header.clone();
                        self.call_name = normalize_tool_name(&raw_header);
                        self.raw_block = format!("{CALL_BEGIN}{raw_header}{ARG_BEGIN}");
                        events.push(InbandScanEvent::ToolStart {
                            id: self.call_id.clone(),
                            name: self.call_name.clone(),
                        });
                        self.buffer = self.buffer[(pos + ARG_BEGIN.len())..].to_string();
                        self.state = State::Args;
                        continue;
                    }
                    if final_ {
                        self.drop_buffered_call();
                    }
                    return events;
                }

                State::Args => {
                    // Reading the tool call arguments until CALL_END
                    if let Some(pos) = self.buffer.find(CALL_END) {
                        let raw_args_block = self.buffer[..pos].trim().to_string();
                        let args: serde_json::Value =
                            serde_json::from_str(&raw_args_block).unwrap_or_default();
                        events.push(InbandScanEvent::ToolEnd {
                            id: self.call_id.clone(),
                            name: self.call_name.clone(),
                            arguments: args,
                            raw_block: Some(format!("{}{}{}", self.raw_block, raw_args_block, CALL_END)),
                        });
                        self.buffer = self.buffer[(pos + CALL_END.len())..].to_string();
                        self.reset_call();
                        self.state = State::Section;
                        continue;
                    }
                    if final_ {
                        self.drop_buffered_call();
                    }
                    return events;
                }
            }
        }
    }

    fn next_token_index(&self) -> Option<usize> {
        let mut best = None;
        for token in TOKENS {
            if let Some(idx) = self.buffer.find(token) {
                match best {
                    Some(b) if idx < b => best = Some(idx),
                    None => best = Some(idx),
                    _ => {}
                }
            }
        }
        best
    }

    fn token_at_start(&self) -> Option<&'static str> {
        for token in TOKENS {
            if self.buffer.starts_with(token) {
                return Some(token);
            }
        }
        None
    }

    fn skip_whitespace(&mut self) {
        let trimmed = self.buffer.trim_start();
        let _skipped = self.buffer.len() - trimmed.len();
        self.buffer = trimmed.to_string();
    }

    fn drop_buffered_call(&mut self) {
        self.buffer.clear();
        self.reset_call();
        self.state = State::Outside;
    }
}

fn normalize_tool_name(raw: &str) -> String {
    // Strip "functions." prefix if present
    raw.strip_prefix("functions.").unwrap_or(raw).to_string()
}

fn partial_suffix_overlap_any(buf: &str, tags: &[&str]) -> usize {
    let buf_lower = buf.to_lowercase();
    let mut max_hold = 0usize;
    for tag in tags {
        let tag_lower = tag.to_lowercase();
        let min_len = buf_lower.len().min(tag_lower.len());
        if buf_lower.len() < tag_lower.len() && tag_lower.starts_with(&buf_lower) {
            max_hold = max_hold.max(buf.len());
        } else if min_len > 0 && tag_lower[..min_len] == buf_lower[buf_lower.len() - min_len..] {
            max_hold = max_hold.max(min_len);
        }
    }
    max_hold
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kimi_single_tool_call() {
        let mut scanner = KimiInbandScanner::new(&InbandScannerOptions::default());
        let input = format!(
            "What's the weather?{SECTION_BEGIN}{CALL_BEGIN}get_weather{ARG_BEGIN}{{\"city\":\"NYC\"}}{CALL_END}{SECTION_END}",
        );
        let events = scanner.feed(&input);
        let flushed = scanner.flush();
        let all: Vec<_> = events.into_iter().chain(flushed).collect();
        // Expected: Text("What's the weather?") + ToolStart + ToolEnd
        assert_eq!(all.len(), 3, "expected 3 events, got {all:?}");
        assert!(matches!(&all[0], InbandScanEvent::Text(t) if t == "What's the weather?"));
        assert!(matches!(&all[1], InbandScanEvent::ToolStart { name, .. } if name == "get_weather"));
        assert!(matches!(&all[2], InbandScanEvent::ToolEnd { name, .. } if name == "get_weather"));
    }

    #[test]
    fn test_kimi_multiple_tool_calls() {
        let mut scanner = KimiInbandScanner::new(&InbandScannerOptions::default());
        let input = format!(
            "{SECTION_BEGIN}{CALL_BEGIN}func_a{ARG_BEGIN}{{}}{CALL_END}{CALL_BEGIN}func_b{ARG_BEGIN}{{}}{CALL_END}{SECTION_END}",
        );
        let events = scanner.feed(&input);
        let flushed = scanner.flush();
        let all: Vec<_> = events.into_iter().chain(flushed).collect();
        let starts: Vec<_> = all.iter().filter_map(|e| {
            if let InbandScanEvent::ToolStart { name, .. } = e { Some(name.as_str()) } else { None }
        }).collect();
        assert_eq!(starts, vec!["func_a", "func_b"]);
    }

    #[test]
    fn test_kimi_nothing_to_parse() {
        let mut scanner = KimiInbandScanner::new(&InbandScannerOptions::default());
        let events = scanner.feed("Just a regular message with no tokens.");
        let flushed = scanner.flush();
        let all: Vec<_> = events.into_iter().chain(flushed).collect();
        assert_eq!(all.len(), 1);
        assert!(matches!(&all[0], InbandScanEvent::Text(_)));
    }
}
