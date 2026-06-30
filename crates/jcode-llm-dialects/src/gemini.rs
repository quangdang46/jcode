//! Gemini Inband Scanner — Python-fenced ```` ```tool_code ```` format.
//!
//! The model emits tool calls inside fenced Python code blocks:
//!
//! ````text
//! ```tool_code
//! tool_call(name: "get_weather", city: "NYC")
//! ```
//! ````
//!
//! Arguments use Python-style keyword notation (`key: value`). Values may be
//! strings (quoted), numbers, booleans, or nested dicts/lists.

use std::time::{SystemTime, UNIX_EPOCH};
use crate::types::{InbandScanEvent, InbandScanner, InbandScannerOptions};
use serde_json::Value;

const FENCE_OPEN: &str = "```tool_code";
const FENCE_CLOSE: &str = "```";

/// Streaming scanner for the Gemini dialect.
pub struct GeminiInbandScanner {
    buffer: String,
    inside_fence: bool,
    fence_content: String,
}

impl GeminiInbandScanner {
    pub fn new(_options: &InbandScannerOptions) -> Self {
        Self {
            buffer: String::new(),
            inside_fence: false,
            fence_content: String::new(),
        }
    }

    fn gen_id(&self) -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        format!("gemini_{:09x}", nanos)
    }
}

impl InbandScanner for GeminiInbandScanner {
    fn feed(&mut self, text: &str) -> Vec<InbandScanEvent> {
        if text.is_empty() {
            return vec![];
        }
        self.buffer.push_str(text);
        self.consume(false)
    }

    fn flush(&mut self) -> Vec<InbandScanEvent> {
        let mut events = self.consume(true);
        if self.inside_fence && !self.fence_content.is_empty() {
            // Try to parse what we have as tool calls
            events.extend(parse_tool_calls_from_body(&self.fence_content, &self.gen_id()));
            self.fence_content.clear();
            self.inside_fence = false;
        }
        if !self.buffer.is_empty() {
            events.push(InbandScanEvent::Text(self.buffer.clone()));
            self.buffer.clear();
        }
        events
    }
}

impl GeminiInbandScanner {
    fn consume(&mut self, final_: bool) -> Vec<InbandScanEvent> {
        let mut events = Vec::new();
        loop {
            if self.inside_fence {
                // Look for closing fence
                if let Some(pos) = self.buffer.find(FENCE_CLOSE) {
                    let content = &self.buffer[..pos];
                    self.fence_content.push_str(content);
                    self.buffer = self.buffer[(pos + FENCE_CLOSE.len())..].to_string();
                    self.inside_fence = false;

                    // Parse the collected fence content
                    events.extend(parse_tool_calls_from_body(&self.fence_content, &self.gen_id()));
                    self.fence_content.clear();
                    continue;
                }
                // Content before the last line that could be a close fence overlap
                if final_ {
                    self.fence_content.push_str(&self.buffer);
                    events.extend(parse_tool_calls_from_body(&self.fence_content, &self.gen_id()));
                    self.fence_content.clear();
                    self.buffer.clear();
                    self.inside_fence = false;
                    return events;
                }
                // Hold back potential partial "```"
                let hold = if self.buffer.ends_with('`') {
                    1.min(self.buffer.len())
                } else if self.buffer.ends_with("``") {
                    2.min(self.buffer.len())
                } else {
                    0
                };
                let emit_end = self.buffer.len().saturating_sub(hold);
                self.fence_content.push_str(&self.buffer[..emit_end]);
                self.buffer = self.buffer[emit_end..].to_string();
                return events;
            }

            // Outside fence — look for opening
            if let Some(pos) = self.buffer.find(FENCE_OPEN) {
                if pos > 0 {
                    events.push(InbandScanEvent::Text(self.buffer[..pos].to_string()));
                }
                // Emit text up to fence, then enter fence mode
                self.buffer = self.buffer[(pos + FENCE_OPEN.len())..].to_string();
                // Skip whitespace after fence marker
                let trimmed = self.buffer.trim_start();
                let _skipped = self.buffer.len() - trimmed.len();
                self.buffer = trimmed.to_string();
                self.inside_fence = true;
                self.fence_content.clear();
                continue;
            }

            // No fence at all
            let hold = if final_ {
                0
            } else if self.buffer.ends_with('`') {
                // Could be partial FENCE_OPEN
                let min_len = self.buffer.len().min(FENCE_OPEN.len());
                if self.buffer[FENCE_OPEN.len().saturating_sub(min_len)..]
                    .to_lowercase()
                    .as_str()
                    == &"```tool_code"[..min_len]
                {
                    min_len
                } else {
                    0
                }
            } else {
                0
            };
            let emit_end = self.buffer.len().saturating_sub(hold);
            if emit_end > 0 {
                events.push(InbandScanEvent::Text(self.buffer[..emit_end].to_string()));
            }
            self.buffer = self.buffer[emit_end..].to_string();
            return events;
        }
    }
}

/// Parse Python-style `tool_call(key: value, ...)` calls from fence body text.
fn parse_tool_calls_from_body(body: &str, id_prefix: &str) -> Vec<InbandScanEvent> {
    let mut events = Vec::new();
    let body = body.trim();
    if body.is_empty() {
        return events;
    }

    // Match `tool_call(...)` patterns
    let mut remaining = body;
    let mut counter = 0u32;

    while let Some(start) = remaining.find("tool_call(") {
        let before = &remaining[..start];
        if !before.trim().is_empty() {
            events.push(InbandScanEvent::Text(before.to_string()));
        }
        remaining = &remaining[(start + 10)..]; // skip "tool_call("

        // Find matching closing paren
        let mut depth = 1i32;
        let mut end = 0usize;
        for (i, ch) in remaining.char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        end = i;
                        break;
                    }
                }
                _ => {}
            }
        }
        if depth != 0 {
            // No closing paren — emit as text
            events.push(InbandScanEvent::Text(format!("tool_call({remaining}")));
            break;
        }

        let args_str = remaining[..end].trim();
        remaining = &remaining[(end + 1)..];
        counter += 1;
        let id = format!("{id_prefix}_{counter}");

        // Parse keyword arguments
        let (name, args_map) = parse_python_kwargs(args_str);
        events.push(InbandScanEvent::ToolStart {
            id: id.clone(),
            name: name.clone(),
        });
        events.push(InbandScanEvent::ToolEnd {
            id,
            name,
            arguments: args_map,
            raw_block: Some(format!("tool_call({args_str})")),
        });
    }

    if !remaining.trim().is_empty() {
        events.push(InbandScanEvent::Text(remaining.to_string()));
    }

    events
}

/// Parse Python-style keyword arguments like `name: "get_weather", city: "NYC"`.
fn parse_python_kwargs(input: &str) -> (String, Value) {
    let input = input.trim();
    if input.is_empty() {
        return ("unknown".to_string(), Value::Object(Default::default()));
    }

    let mut name = String::from("unknown");
    let mut map = serde_json::Map::new();

    // Split by top-level commas first, then parse key:value per part
    for part in split_by_top_level_comma(input) {
        let part = part.trim();
        if let Some(colon_pos) = part.find(':') {
            let key = part[..colon_pos].trim().trim_matches('"').trim_matches('\'').to_string();
            let val_str = part[colon_pos + 1..].trim();
            if key == "name" {
                if let Some(val) = read_python_value(val_str) {
                    if let Some(s) = val.as_str() {
                        name = s.to_string();
                    }
                }
            } else {
                let val = read_python_value(val_str).unwrap_or(Value::Null);
                map.insert(key, val);
            }
        }
    }

    (name, Value::Object(map))
}

/// Read a single Python value from the start of the string.
fn read_python_value(s: &str) -> Option<Value> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let first = s.chars().next()?;

    // String
    if first == '"' || first == '\'' {
        let quote = first;
        let mut escaped = false;
        let mut end = None;
        for (i, ch) in s[1..].char_indices() {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
            } else if ch == quote {
                end = Some(i + 1);
                break;
            }
        }
        if let Some(end) = end {
            let inner = &s[1..end]; // strip quotes
            // Return the value with the consumed length via string slicing
            return Some(Value::String(
                inner.replace("\\\"", "\"").replace("\\'", "'"),
            ));
        }
        // Unterminated string
        return Some(Value::String(s[1..].to_string()));
    }

    // Number
    if first.is_ascii_digit() || first == '-' {
        let mut end = 0;
        for (i, ch) in s.char_indices() {
            if ch.is_ascii_digit() || ch == '.' || ch == '-' || ch == '+' || ch == 'e' || ch == 'E' {
                end = i + 1;
            } else {
                break;
            }
        }
        if end > 0 {
            let num_str = &s[..end];
            if let Ok(n) = num_str.parse::<i64>() {
                return Some(Value::Number(n.into()));
            }
            if let Ok(n) = num_str.parse::<f64>() {
                if let Some(v) = serde_json::Number::from_f64(n) {
                    return Some(Value::Number(v));
                }
            }
            return Some(Value::String(num_str.to_string()));
        }
    }

    // Boolean / None
    if s.starts_with("true") || s.starts_with("True") {
        return Some(Value::Bool(true));
    }
    if s.starts_with("false") || s.starts_with("False") {
        return Some(Value::Bool(false));
    }
    if s.starts_with("none") || s.starts_with("None") || s.starts_with("null") {
        return Some(Value::Null);
    }

    // List
    if first == '[' {
        let mut depth = 1i32;
        let mut end = None;
        for (i, ch) in s[1..].char_indices() {
            match ch {
                '[' => depth += 1,
                ']' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(i + 1);
                        break;
                    }
                }
                _ => {}
            }
        }
        if let Some(end) = end {
            let inner = s[1..end].trim();
            if inner.is_empty() {
                return Some(Value::Array(vec![]));
            }
            // Parse comma-separated items
            let items: Vec<Value> = inner
                .split(',')
                .filter_map(|item| read_python_value(item.trim()))
                .collect();
            return Some(Value::Array(items));
        }
        return None;
    }

    // Dict
    if first == '{' {
        let mut depth = 1i32;
        let mut end = None;
        for (i, ch) in s[1..].char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(i + 1);
                        break;
                    }
                }
                _ => {}
            }
        }
        if let Some(end) = end {
            let inner = s[1..end].trim();
            if inner.is_empty() {
                return Some(Value::Object(Default::default()));
            }
            let mut map = serde_json::Map::new();
            for part in split_by_top_level_comma(inner) {
                let part = part.trim();
                if let Some(eq_pos) = part.find(':') {
                    let k = part[..eq_pos].trim().trim_matches('"').trim_matches('\'');
                    let v = read_python_value(part[eq_pos + 1..].trim());
                    if let Some(v) = v {
                        map.insert(k.to_string(), v);
                    }
                }
            }
            return Some(Value::Object(map));
        }
        return None;
    }

    None
}

fn split_by_top_level_comma(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (i, ch) in s.char_indices() {
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    if start < s.len() {
        parts.push(&s[start..]);
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gemini_single_tool_call() {
        let mut scanner = GeminiInbandScanner::new(&InbandScannerOptions::default());
        let input = "Let me check\n```tool_code\ntool_call(name: \"get_weather\", city: \"NYC\")\n```\n";
        let events = scanner.feed(input);
        let flushed = scanner.flush();
        let all: Vec<_> = events.into_iter().chain(flushed).collect();
        assert!(all.iter().any(|e| matches!(e, InbandScanEvent::ToolStart { name, .. } if name == "get_weather")),
            "expected ToolStart for get_weather, got {all:?}");
    }

    #[test]
    fn test_gemini_no_tool_calls() {
        let mut scanner = GeminiInbandScanner::new(&InbandScannerOptions::default());
        let events = scanner.feed("Just some regular text without any tool calls.");
        let flushed = scanner.flush();
        let all: Vec<_> = events.into_iter().chain(flushed).collect();
        assert_eq!(all.len(), 1);
        assert!(matches!(&all[0], InbandScanEvent::Text(_)));
    }

    #[test]
    fn test_gemini_malformed_fence() {
        let mut scanner = GeminiInbandScanner::new(&InbandScannerOptions::default());
        let input = "```tool_code\nthis is not a valid tool call\n```";
        let events = scanner.feed(input);
        let flushed = scanner.flush();
        let all: Vec<_> = events.into_iter().chain(flushed).collect();
        // Should produce no tool events, just text
        assert!(!all.iter().any(|e| matches!(e, InbandScanEvent::ToolStart { .. })));
    }

    #[test]
    fn test_python_kwargs_simple() {
        let (name, args) = parse_python_kwargs(r#"name: "get_weather", city: "NYC""#);
        assert_eq!(name, "get_weather");
        assert_eq!(args.get("city").and_then(|v| v.as_str()), Some("NYC"));
    }

    #[test]
    fn test_python_value_string() {
        assert_eq!(
            read_python_value(r#""hello world""#),
            Some(Value::String("hello world".to_string()))
        );
        assert_eq!(read_python_value("42"), Some(Value::Number(42.into())));
        assert_eq!(read_python_value("true"), Some(Value::Bool(true)));
        assert_eq!(read_python_value("None"), Some(Value::Null));
    }
}
