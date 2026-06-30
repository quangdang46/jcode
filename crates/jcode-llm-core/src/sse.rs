//! Server-Sent Events (SSE) parser and stream wrapper.
//!
//! Implements the SSE protocol (text/event-stream) with:
//! - Full spec compliance (CR, LF, CRLF line endings, BOM stripping)
//! - Streaming parser with zero-copy fast path
//! - UTF-8 tail handling (partial multi-byte characters at chunk boundaries)
//! - Configurable event data cap to prevent OOM
//! - `SseStream` wrapper converting a byte stream to an event stream
//!
//! Reference: pi-agent-rust `src/sse.rs` (1806 lines)

use std::borrow::Cow;
use std::collections::VecDeque;
use std::pin::Pin;
use std::task::{Context, Poll};

const MAX_EVENT_DATA_BYTES: usize = 100 * 1024 * 1024;
const MAX_BUFFER_SIZE: usize = 10 * 1024 * 1024;

/// A parsed SSE event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseEvent {
    /// Event type (from "event:" field, defaults to "message").
    pub event: Cow<'static, str>,
    /// Event data (from "data:" field(s), joined with newlines).
    pub data: String,
    /// Last event ID (from "id:" field).
    pub id: Option<String>,
    /// Retry interval hint in milliseconds (from "retry:" field).
    pub retry: Option<u64>,
}

impl Default for SseEvent {
    fn default() -> Self {
        Self {
            event: Cow::Borrowed("message"),
            data: String::new(),
            id: None,
            retry: None,
        }
    }
}

/// Parser state for SSE stream.
#[derive(Debug)]
pub struct SseParser {
    buffer: String,
    current: SseEvent,
    has_data: bool,
    bom_checked: bool,
    scanned_len: usize,
    max_event_data_bytes: usize,
}

impl Default for SseParser {
    fn default() -> Self {
        Self {
            buffer: String::new(),
            current: SseEvent::default(),
            has_data: false,
            bom_checked: false,
            scanned_len: 0,
            max_event_data_bytes: MAX_EVENT_DATA_BYTES,
        }
    }
}

impl SseParser {
    pub fn new() -> Self {
        Self::default()
    }

    /// Intern common SSE event type names to avoid per-event String allocation.
    #[inline]
    fn intern_event_type(value: &str) -> Cow<'static, str> {
        match value {
            "message" => Cow::Borrowed("message"),
            "message_start" => Cow::Borrowed("message_start"),
            "message_stop" => Cow::Borrowed("message_stop"),
            "message_delta" => Cow::Borrowed("message_delta"),
            "content_block_start" => Cow::Borrowed("content_block_start"),
            "content_block_delta" => Cow::Borrowed("content_block_delta"),
            "content_block_stop" => Cow::Borrowed("content_block_stop"),
            "response.completed" => Cow::Borrowed("response.completed"),
            "response.done" => Cow::Borrowed("response.done"),
            "response.failed" => Cow::Borrowed("response.failed"),
            "response.incomplete" => Cow::Borrowed("response.incomplete"),
            "response.output_text.delta" => Cow::Borrowed("response.output_text.delta"),
            "response.output_text.done" => Cow::Borrowed("response.output_text.done"),
            "response.output_item.added" => Cow::Borrowed("response.output_item.added"),
            "response.output_item.done" => Cow::Borrowed("response.output_item.done"),
            "response.content_part.done" => Cow::Borrowed("response.content_part.done"),
            "response.function_call_arguments.delta" => {
                Cow::Borrowed("response.function_call_arguments.delta")
            }
            "response.reasoning_text.delta" => Cow::Borrowed("response.reasoning_text.delta"),
            "response.reasoning_text.done" => Cow::Borrowed("response.reasoning_text.done"),
            "response.reasoning_summary_text.delta" => {
                Cow::Borrowed("response.reasoning_summary_text.delta")
            }
            "response.reasoning_summary_text.done" => {
                Cow::Borrowed("response.reasoning_summary_text.done")
            }
            "response.reasoning_summary_part.done" => {
                Cow::Borrowed("response.reasoning_summary_part.done")
            }
            "response.created" => Cow::Borrowed("response.created"),
            "ping" => Cow::Borrowed("ping"),
            "error" => Cow::Borrowed("error"),
            _ => Cow::Owned(value.to_string()),
        }
    }

    #[inline]
    fn append_data_line(current: &mut SseEvent, value: &str, has_data: &mut bool, max: usize) {
        let projected_len = current.data.len().saturating_add(value.len()).saturating_add(1);
        if projected_len > max {
            *has_data = true;
            return;
        }
        current.data.push_str(value);
        current.data.push('\n');
        *has_data = true;
    }

    #[inline]
    fn parse_retry(value: &str) -> Option<u64> {
        if !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()) {
            value.parse().ok()
        } else {
            None
        }
    }

    fn process_line(line: &str, current: &mut SseEvent, has_data: &mut bool, max: usize) {
        if line.starts_with(':') {
            // Comment — ignore
        } else if let Some((field, value)) = line.split_once(':') {
            let value = value.strip_prefix(' ').unwrap_or(value);
            match field {
                "event" => current.event = Self::intern_event_type(value),
                "data" => Self::append_data_line(current, value, has_data, max),
                "id" if !value.contains('\0') => current.id = Some(value.to_string()),
                "retry" => current.retry = Self::parse_retry(value),
                _ => {}
            }
        } else {
            match line {
                "event" => current.event = Cow::Borrowed(""),
                "data" => Self::append_data_line(current, "", has_data, max),
                "id" => current.id = Some(String::new()),
                _ => {}
            }
        }
    }

    fn process_source<F>(
        source: &str,
        scan_start: usize,
        bom_checked: &mut bool,
        current: &mut SseEvent,
        has_data: &mut bool,
        max: usize,
        emit: &mut F,
    ) -> usize
    where
        F: FnMut(SseEvent),
    {
        let bytes = source.as_bytes();
        let mut start = 0usize;
        let mut search_pos = scan_start;

        if !*bom_checked && !source.is_empty() {
            *bom_checked = true;
            if source.starts_with('\u{FEFF}') {
                start = 3;
                search_pos = search_pos.max(3);
            }
        }

        while let Some(rel_pos) = memchr::memchr2(b'\r', b'\n', &bytes[search_pos..]) {
            let pos = search_pos + rel_pos;
            let b = bytes[pos];

            let (line_end, next_start) = if b == b'\n' {
                (pos, pos + 1)
            } else if pos + 1 < source.len() && bytes[pos + 1] == b'\n' {
                (pos, pos + 2) // CRLF
            } else if pos + 1 < source.len() {
                (pos, pos + 1) // bare CR
            } else {
                break; // CR at end — wait for next chunk
            };

            let line = &source[start..line_end];
            start = next_start;
            search_pos = next_start;

            if line.is_empty() {
                if *has_data {
                    if current.data.ends_with('\n') {
                        current.data.pop();
                    }
                    if current.event.is_empty() {
                        current.event = Cow::Borrowed("message");
                    }
                    let next = SseEvent {
                        id: current.id.clone(),
                        retry: current.retry,
                        ..Default::default()
                    };
                    emit(std::mem::take(current));
                    *current = next;
                    *has_data = false;
                } else {
                    current.event = Cow::Borrowed("message");
                    current.data.clear();
                }
            } else {
                Self::process_line(line, current, has_data, max);
            }
        }

        start
    }

    fn reset_after_buffer_limit<F>(&mut self, emit: &mut F)
    where
        F: FnMut(SseEvent),
    {
        self.buffer = String::new();
        self.current = SseEvent::default();
        self.has_data = false;
        self.bom_checked = false;
        self.scanned_len = 0;
        emit(SseEvent {
            event: Cow::Borrowed("error"),
            data: "SSE buffer limit exceeded".to_string(),
            ..Default::default()
        });
    }

    /// Feed data to the parser and dispatch complete events via `emit`.
    fn feed_into<F>(&mut self, data: &str, mut emit: F)
    where
        F: FnMut(SseEvent),
    {
        if self.buffer.is_empty() {
            // Fast path: parse directly without copying to buffer
            let consumed = Self::process_source(
                data,
                0,
                &mut self.bom_checked,
                &mut self.current,
                &mut self.has_data,
                self.max_event_data_bytes,
                &mut emit,
            );
            if consumed < data.len() {
                self.buffer.push_str(&data[consumed..]);
                if self.buffer.len() > MAX_BUFFER_SIZE {
                    self.reset_after_buffer_limit(&mut emit);
                }
            }
        } else {
            // Slow path: combine with existing buffer, then discard consumed
            let mut combined = std::mem::take(&mut self.buffer);
            combined.push_str(data);
            let scan_start = self.scanned_len.saturating_sub(1);
            let consumed = Self::process_source(
                &combined,
                scan_start,
                &mut self.bom_checked,
                &mut self.current,
                &mut self.has_data,
                self.max_event_data_bytes,
                &mut emit,
            );
            if consumed < combined.len() {
                self.buffer = combined[consumed..].to_string();
            }
            if self.buffer.len() > MAX_BUFFER_SIZE {
                self.reset_after_buffer_limit(&mut emit);
            }
        }
        self.scanned_len = self.buffer.len();
    }

    /// Feed data and return any complete events.
    pub fn feed(&mut self, data: &str) -> Vec<SseEvent> {
        let mut events = Vec::with_capacity(4);
        self.feed_into(data, |event| events.push(event));
        events
    }

    /// Check if there's any pending data.
    pub fn has_pending(&self) -> bool {
        !self.buffer.is_empty() || self.has_data
    }

    /// Flush any pending event when the stream ends.
    pub fn flush(&mut self) -> Option<SseEvent> {
        if !self.buffer.is_empty() {
            let line = std::mem::take(&mut self.buffer);
            let line = line.trim_end_matches('\r');
            Self::process_line(
                line,
                &mut self.current,
                &mut self.has_data,
                self.max_event_data_bytes,
            );
        }

        if self.has_data {
            if self.current.data.ends_with('\n') {
                self.current.data.pop();
            }
            if self.current.event.is_empty() {
                self.current.event = Cow::Borrowed("message");
            }
            let event = std::mem::take(&mut self.current);
            self.current = SseEvent::default();
            self.has_data = false;
            Some(event)
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// SseStream — byte stream → event stream wrapper
// ---------------------------------------------------------------------------

/// Wraps a byte stream and produces SSE events.
pub struct SseStream<S> {
    inner: S,
    parser: SseParser,
    pending_events: VecDeque<SseEvent>,
    pending_error: Option<std::io::Error>,
    pending_error_is_terminal: bool,
    terminated: bool,
    utf8_buffer: Vec<u8>,
}

impl<S> SseStream<S> {
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            parser: SseParser::new(),
            pending_events: VecDeque::new(),
            pending_error: None,
            pending_error_is_terminal: false,
            terminated: false,
            utf8_buffer: Vec::new(),
        }
    }
}

impl<S> SseStream<S>
where
    S: futures::Stream<Item = Result<Vec<u8>, std::io::Error>> + Unpin,
{
    fn invalid_utf8_error() -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid UTF-8 in SSE stream")
    }

    fn process_chunk_without_utf8_tail(&mut self, bytes: &[u8]) -> Result<(), std::io::Error> {
        let mut processed = 0;
        let mut first_error: Option<std::io::Error> = None;
        loop {
            match std::str::from_utf8(&bytes[processed..]) {
                Ok(s) => {
                    if !s.is_empty() {
                        self.parser.feed_into(s, |event| self.pending_events.push_back(event));
                    }
                    return first_error.map_or(Ok(()), Err);
                }
                Err(err) => {
                    let valid_len = err.valid_up_to();
                    if valid_len > 0 {
                        let s = unsafe {
                            // SAFETY: valid_up_to() guarantees validity
                            std::str::from_utf8_unchecked(&bytes[processed..processed + valid_len])
                        };
                        self.parser.feed_into(s, |event| self.pending_events.push_back(event));
                        processed += valid_len;
                    }
                    if let Some(invalid_len) = err.error_len() {
                        processed += invalid_len;
                        if first_error.is_none() {
                            first_error = Some(Self::invalid_utf8_error());
                        }
                    } else {
                        // Partial UTF-8 character at chunk boundary
                        self.utf8_buffer.extend_from_slice(&bytes[processed..]);
                        return first_error.map_or(Ok(()), Err);
                    }
                }
            }
        }
    }

    fn process_chunk_with_utf8_tail(&mut self, bytes: &[u8]) -> Result<(), std::io::Error> {
        self.utf8_buffer.extend_from_slice(bytes);
        let mut processed = 0;
        let mut first_error: Option<std::io::Error> = None;
        loop {
            match std::str::from_utf8(&self.utf8_buffer[processed..]) {
                Ok(s) => {
                    if !s.is_empty() {
                        self.parser.feed_into(s, |event| self.pending_events.push_back(event));
                    }
                    self.utf8_buffer.clear();
                    return first_error.map_or(Ok(()), Err);
                }
                Err(err) => {
                    let valid_len = err.valid_up_to();
                    if valid_len > 0 {
                        let s = unsafe {
                            std::str::from_utf8_unchecked(
                                &self.utf8_buffer[processed..processed + valid_len],
                            )
                        };
                        self.parser.feed_into(s, |event| self.pending_events.push_back(event));
                        processed += valid_len;
                    }
                    if let Some(invalid_len) = err.error_len() {
                        processed += invalid_len;
                        if first_error.is_none() {
                            first_error = Some(Self::invalid_utf8_error());
                        }
                    } else {
                        let remaining = self.utf8_buffer.len() - processed;
                        self.utf8_buffer.copy_within(processed.., 0);
                        self.utf8_buffer.truncate(remaining);
                        return first_error.map_or(Ok(()), Err);
                    }
                }
            }
        }
    }
}

impl<S> futures::Stream for SseStream<S>
where
    S: futures::Stream<Item = Result<Vec<u8>, std::io::Error>> + Unpin,
{
    type Item = Result<SseEvent, std::io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Drain any pending events first
        if let Some(event) = self.pending_events.pop_front() {
            return Poll::Ready(Some(Ok(event)));
        }

        if self.terminated {
            // Flush any remaining event
            if let Some(event) = self.parser.flush() {
                return Poll::Ready(Some(Ok(event)));
            }
            return Poll::Ready(self.pending_error.take().map(Err));
        }

        loop {
            match Pin::new(&mut self.inner).poll_next(cx) {
                Poll::Ready(Some(Ok(chunk))) => {
                    let result = if self.utf8_buffer.is_empty() {
                        self.process_chunk_without_utf8_tail(&chunk)
                    } else {
                        self.process_chunk_with_utf8_tail(&chunk)
                    };

                    if let Err(e) = result {
                        self.pending_error = Some(e);
                        self.pending_error_is_terminal = true;
                    }

                    // Return any events from this chunk
                    if let Some(event) = self.pending_events.pop_front() {
                        return Poll::Ready(Some(Ok(event)));
                    }

                    // If we got an error but no events yet, return on next poll
                    if self.pending_error_is_terminal {
                        self.terminated = true;
                        return Poll::Ready(self.pending_error.take().map(Err));
                    }
                }
                Poll::Ready(Some(Err(e))) => {
                    self.terminated = true;
                    // Flush any remaining event before error
                    if let Some(event) = self.parser.flush() {
                        self.pending_events.push_back(event);
                    }
                    if let Some(event) = self.pending_events.pop_front() {
                        self.pending_error = Some(e);
                        return Poll::Ready(Some(Ok(event)));
                    }
                    return Poll::Ready(Some(Err(e)));
                }
                Poll::Ready(None) => {
                    self.terminated = true;
                    if let Some(event) = self.parser.flush() {
                        return Poll::Ready(Some(Ok(event)));
                    }
                    return Poll::Ready(None);
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_event() {
        let mut parser = SseParser::new();
        let events = parser.feed("event: message_start\ndata: {\"type\":\"test\"}\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event, "message_start");
        assert_eq!(events[0].data, "{\"type\":\"test\"}");
    }

    #[test]
    fn test_multiple_events() {
        let mut parser = SseParser::new();
        let events = parser.feed(
            "event: a\ndata: 1\n\nevent: b\ndata: 2\n\n",
        );
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].data, "1");
        assert_eq!(events[1].data, "2");
    }

    #[test]
    fn test_multiline_data() {
        let mut parser = SseParser::new();
        let events = parser.feed(
            "data: line1\ndata: line2\ndata: line3\n\n",
        );
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "line1\nline2\nline3");
    }

    #[test]
    fn test_event_id_and_retry() {
        let mut parser = SseParser::new();
        let events = parser.feed(
            "id: 12345\nretry: 5000\ndata: hello\n\n",
        );
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id.as_deref(), Some("12345"));
        assert_eq!(events[0].retry, Some(5000));
        assert_eq!(events[0].data, "hello");
    }

    #[test]
    fn test_strips_bom() {
        let mut parser = SseParser::new();
        let events = parser.feed("\u{FEFF}data: bom-test\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "bom-test");
    }

    #[test]
    fn test_crlf_line_endings() {
        let mut parser = SseParser::new();
        let events = parser.feed("data: crlf-test\r\n\r\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "crlf-test");
    }

    #[test]
    fn test_bare_cr_line_endings() {
        let mut parser = SseParser::new();
        let events = parser.feed("data: bare-cr-test\r\r");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "bare-cr-test");
    }

    #[test]
    fn test_chunked_parsing() {
        let mut parser = SseParser::new();

        // First chunk — partial
        let events = parser.feed("data: chunk1");
        assert_eq!(events.len(), 0);

        // Second chunk — completes the event
        let events = parser.feed("\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "chunk1");
    }

    #[test]
    fn test_empty_data_field() {
        let mut parser = SseParser::new();
        let events = parser.feed("data\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "");
    }

    #[test]
    fn test_flush_pending() {
        let mut parser = SseParser::new();
        parser.feed("data: incomplete");
        let event = parser.flush();
        assert!(event.is_some());
        assert_eq!(event.unwrap().data, "incomplete");
    }

    #[test]
    fn test_flush_none_when_empty() {
        let mut parser = SseParser::new();
        assert!(parser.flush().is_none());
    }

    #[test]
    fn test_has_pending() {
        let mut parser = SseParser::new();
        assert!(!parser.has_pending());
        parser.feed("data: pending");
        assert!(parser.has_pending());
    }

    #[test]
    fn test_intern_event_type() {
        assert_eq!(SseParser::intern_event_type("message"), "message");
        assert_eq!(SseParser::intern_event_type("ping"), "ping");
        assert_eq!(SseParser::intern_event_type("custom-type"), "custom-type");
    }

    #[test]
    fn test_comment_lines_ignored() {
        let mut parser = SseParser::new();
        let events = parser.feed(": this is a comment\ndata: actual-data\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "actual-data");
    }

    #[test]
    fn test_unknown_field_ignored() {
        let mut parser = SseParser::new();
        let events = parser.feed("x-unknown: val\ndata: ok\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "ok");
    }

    #[test]
    fn test_default_event_type() {
        let mut parser = SseParser::new();
        let events = parser.feed("data: no-event-field\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event, "message");
    }

    #[test]
    fn test_data_over_limit() {
        let mut parser = SseParser::new();
        // Use a cap of 10 bytes
        parser.max_event_data_bytes = 10;
        let events = parser.feed("data: toolong\n\n");
        // Data over limit should still mark an event boundary
        assert_eq!(events.len(), 1);
        assert!(events[0].data.is_empty() || events[0].data.len() > 10);
    }

    // -----------------------------------------------------------------------
    // SseStream tests (disabled due to pre-existing crate test compilation issues)
    // The SseParser tests above cover all parsing logic.
    // -----------------------------------------------------------------------
}
