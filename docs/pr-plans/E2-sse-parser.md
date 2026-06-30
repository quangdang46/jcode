# PR Plan: E2 — SSE Streaming Parser with UTF-8 Tail Handling

## Research Summary
- **Source repo**: pi-agent-rust (`src/sse.rs`, 1806 lines)
- **Key files inspected**: `sse.rs` — full SSE parser with `SseParser` (state machine), `SseStream<S>` (futures Stream wrapper), `SseEvent` type with interning, BOM stripping, CR/LF/CRLF, UTF-8 tail handling
- **jcode current state**: Has `parse_sse_event()` test helper in `jcode-base/src/provider/anthropic_tests.rs:60-65`. No dedicated reusable SSE parser module.

## Why This Feature Is Missing in jcode
- SSE parsing is embedded per-provider (Anthropic, OpenRouter) without a shared parser
- No UTF-8 multi-byte character boundary handling (partial chars at chunk boundaries)
- No `SseStream` futures::Stream wrapper for ergonomic stream transformation
- No event-type interning for common Anthropic/OpenAI event types
- No BOM stripping per SSE spec
- No configurable data cap to prevent OOM on malicious streams

## Alternatives Considered
| Approach | Source Repo | Pros | Cons | Decision |
|----------|-------------|------|------|----------|
| Reuse `eventsource-stream` crate | External crate | Existing crate, battle-tested | Additional dependency, not UTF-8-aware tail handling, different API surface | Rejected |
| **Extract pi-agent-rust SSE parser** | pi-agent-rust | Already MIT-licensed, proven in LLM use, full spec compliance, event-type interning | Manual port | **Chosen** |
| Inline per-provider | jcode current | No dependency | Duplicated code, no UTF-8 tail, no stream wrapper | Rejected |

## Chosen Approach
Port pi-agent-rust's SSE parser as a shared module `sse.rs` in `jcode-llm-core`. The module provides:
- `SseEvent` — parsed event with optional id/retry
- `SseParser` — streaming state machine with BOM stripping, CR/LF/CRLF, event-type interning, data cap
- `SseStream<S>` — futures::Stream wrapper converting `Result<Vec<u8>, _>` → `Result<SseEvent, _>` with UTF-8 tail accumulation

## Implementation Plan
1. Add `pub mod sse;` to `jcode-llm-core/src/lib.rs`
2. Add `memchr = "2"` to `jcode-llm-core/Cargo.toml`
3. Write `jcode-llm-core/src/sse.rs` (535 lines: parser + stream + 15 tests)
4. Write `docs/pr-plans/E2-sse-parser.md` (this file)
5. Wire into `jcode-llm-core` public API

## Risk Analysis
- **Performance**: Uses `memchr::memchr2` for fast newline scanning; zero-copy fast path when buffer is empty; event-type interning avoids String allocation per event
- **Memory**: 100 MB per-event data cap; 10 MB buffer cap
- **Compatibility**: Full SSE spec (W3C); tested with CR, LF, CRLF, BOM, multi-line data, chunked input, flush

## Success Criteria
- [x] `cargo check -p jcode-llm-core` clean (0 new errors)
- [x] All 15 SseParser tests pass
- [x] SseStream compiles (disabled from `cargo test` due to pre-existing `route.rs`/`auth.rs` test compilation issues — verified by `cargo check`)
- [x] PARITY.md and PR_BACKLOG.md updated
- [x] Manual verification: chunked parsing, UTF-8 boundaries, flush
