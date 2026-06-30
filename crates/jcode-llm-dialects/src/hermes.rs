//! Hermes Inband Scanner — JSON‑in‑`<tool_call>` format.
//!
//! The simplest dialect: the model emits tool calls inside
//! `<tool_call>{"name":"...","arguments":{...}}</tool_call>` tags.
//! Optional `<think>`…`</think>` blocks are also parsed.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::types::{InbandScanEvent, InbandScanner, InbandScannerOptions};

const TOOL_OPEN: &str = "<tool_call>";
const TOOL_CLOSE: &str = "</tool_call>";
const THINK_OPEN: &str = "<think>";
const THINK_CLOSE: &str = "</think>";

/// Streaming scanner for the Hermes dialect.
pub struct HermesInbandScanner {
    buffer: String,
    inside_tool: bool,
    thinking_accum: String,
    in_thinking: bool,
    parse_thinking: bool,
}

impl HermesInbandScanner {
    pub fn new(options: &InbandScannerOptions) -> Self {
        Self {
            buffer: String::new(),
            inside_tool: false,
            thinking_accum: String::new(),
            in_thinking: false,
            parse_thinking: options.parse_thinking,
        }
    }

    fn gen_id() -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        format!("hermes_{:09x}", nanos)
    }
}

impl InbandScanner for HermesInbandScanner {
    fn feed(&mut self, text: &str) -> Vec<InbandScanEvent> {
        if text.is_empty() {
            return vec![];
        }
        self.buffer.push_str(text);
        self.consume(false)
    }

    fn flush(&mut self) -> Vec<InbandScanEvent> {
        let mut events = self.consume(true);
        if self.in_thinking {
            events.push(InbandScanEvent::ThinkingEnd(self.thinking_accum.clone()));
            self.thinking_accum.clear();
            self.in_thinking = false;
        }
        if !self.buffer.is_empty() {
            events.push(InbandScanEvent::Text(self.buffer.clone()));
            self.buffer.clear();
        }
        self.inside_tool = false;
        events
    }
}

impl HermesInbandScanner {
    fn consume(&mut self, final_: bool) -> Vec<InbandScanEvent> {
        let mut events = Vec::new();
        loop {
            if self.in_thinking {
                if let Some(pos) = self.buffer.find(THINK_CLOSE) {
                    let delta = self.buffer[..pos].to_string();
                    if !delta.is_empty() {
                        self.thinking_accum.push_str(&delta);
                        events.push(InbandScanEvent::ThinkingDelta(delta));
                    }
                    self.buffer = self.buffer[(pos + THINK_CLOSE.len())..].to_string();
                    events.push(InbandScanEvent::ThinkingEnd(self.thinking_accum.clone()));
                    self.thinking_accum.clear();
                    self.in_thinking = false;
                    continue;
                } else if final_ {
                    events.push(InbandScanEvent::ThinkingDelta(self.buffer.clone()));
                    self.thinking_accum.push_str(&self.buffer);
                    events.push(InbandScanEvent::ThinkingEnd(self.thinking_accum.clone()));
                    self.thinking_accum.clear();
                    self.buffer.clear();
                    self.in_thinking = false;
                }
                return events;
            }

            if !self.inside_tool {
                let open = self.buffer.find(TOOL_OPEN);
                let think = if self.parse_thinking {
                    self.buffer.find(THINK_OPEN)
                } else {
                    None
                };
                let _start = match (open, think) {
                    (Some(o), Some(t)) if t < o => {
                        // thinking before tool call
                        if t > 0 {
                            events.push(InbandScanEvent::Text(self.buffer[..t].to_string()));
                        }
                        self.buffer = self.buffer[(t + THINK_OPEN.len())..].to_string();
                        events.push(InbandScanEvent::ThinkingStart);
                        self.in_thinking = true;
                        continue;
                    }
                    (Some(o), _) => {
                        if o > 0 {
                            events.push(InbandScanEvent::Text(self.buffer[..o].to_string()));
                        }
                        self.buffer = self.buffer[(o + TOOL_OPEN.len())..].to_string();
                        self.inside_tool = true;
                        continue;
                    }
                    (None, Some(t)) => {
                        if t > 0 {
                            events.push(InbandScanEvent::Text(self.buffer[..t].to_string()));
                        }
                        self.buffer = self.buffer[(t + THINK_OPEN.len())..].to_string();
                        events.push(InbandScanEvent::ThinkingStart);
                        self.in_thinking = true;
                        continue;
                    }
                    (None, None) => {
                        let hold = if final_ {
                            0
                        } else {
                            partial_suffix_overlap_any(
                                &self.buffer,
                                &[TOOL_OPEN, TOOL_CLOSE, THINK_OPEN, THINK_CLOSE],
                            )
                        };
                        let emit_end = self.buffer.len().saturating_sub(hold);
                        if emit_end > 0 {
                            events.push(InbandScanEvent::Text(self.buffer[..emit_end].to_string()));
                        }
                        self.buffer = self.buffer[emit_end..].to_string();
                        return events;
                    }
                };
            }

            // Inside a tool call
            if let Some(pos) = self.buffer.find(TOOL_CLOSE) {
                let body = self.buffer[..pos].trim().to_string();
                self.buffer = self.buffer[(pos + TOOL_CLOSE.len())..].to_string();
                self.inside_tool = false;

                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&body) {
                    let name = value.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let args = value.get("arguments").cloned().unwrap_or_default();
                    let id = Self::gen_id();
                    events.push(InbandScanEvent::ToolStart { id: id.clone(), name: name.clone() });
                    events.push(InbandScanEvent::ToolEnd {
                        id,
                        name,
                        arguments: args,
                        raw_block: Some(format!("<tool_call>{body}</tool_call>")),
                    });
                }
                continue;
            }
            return events;
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hermes_simple_tool_call() {
        let mut scanner = HermesInbandScanner::new(&InbandScannerOptions::default());
        let input = r#"Hello<tool_call>{"name":"get_weather","arguments":{"city":"NYC"}}</tool_call>"#;
        let events = scanner.feed(input);
        let flushed = scanner.flush();
        let all: Vec<_> = events.into_iter().chain(flushed).collect();
        assert_eq!(all.len(), 3);
        assert!(matches!(&all[0], InbandScanEvent::Text(t) if t == "Hello"));
        assert!(matches!(&all[1], InbandScanEvent::ToolStart { name, .. } if name == "get_weather"));
        assert!(matches!(&all[2], InbandScanEvent::ToolEnd { name, .. } if name == "get_weather"));
    }

    #[test]
    fn test_hermes_streaming_chunks() {
        let mut scanner = HermesInbandScanner::new(&InbandScannerOptions::default());
        scanner.feed(r#"Some text<tool_call>{"name":"get"#);
        let events = scanner.feed(r#"","arguments":{}}</tool_call>"#);
        let flushed = scanner.flush();
        let all: Vec<_> = events.into_iter().chain(flushed).collect();
        let mut names: Vec<_> = all.iter().filter_map(|e| {
            if let InbandScanEvent::ToolStart { name, .. } = e { Some(name.as_str()) } else { None }
        }).collect();
        if names.is_empty() {
            // Try checking ToolEnd instead — the whole call might arrive in one chunk
            names = all.iter().filter_map(|e| {
                if let InbandScanEvent::ToolEnd { name, .. } = e { Some(name.as_str()) } else { None }
            }).collect();
        }
        assert_eq!(names, vec!["get"]);
    }

    #[test]
    fn test_hermes_thinking() {
        let mut scanner = HermesInbandScanner::new(&InbandScannerOptions {
            parse_thinking: true,
            ..Default::default()
        });
        let input = r#"Let me think<think>pondering</think>done"#;
        let events = scanner.feed(input);
        let flushed = scanner.flush();
        let all: Vec<_> = events.into_iter().chain(flushed).collect();
        assert!(all.iter().any(|e| matches!(e, InbandScanEvent::ThinkingStart)));
        assert!(all.iter().any(|e| matches!(e, InbandScanEvent::ThinkingEnd(t) if t == "pondering")));
    }

    #[test]
    fn test_hermes_no_tool_call() {
        let mut scanner = HermesInbandScanner::new(&InbandScannerOptions::default());
        let events = scanner.feed("Just plain text with no tags.");
        let flushed = scanner.flush();
        let all: Vec<_> = events.into_iter().chain(flushed).collect();
        assert!(all.len() <= 1);
        if let Some(ev) = all.first() {
            assert!(matches!(ev, InbandScanEvent::Text(_)));
        }
    }
}
