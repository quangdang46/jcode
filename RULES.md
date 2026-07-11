# RULES.md — Critical Patterns & Pitfalls

## Rust / TUI

### 1. NEVER use `biased;` before `event_stream.next()` in `tokio::select!`

crossterm's `EventStream` uses `std::thread::spawn` with a blocking `read()` on stdin.
When the terminal loses focus (alt-tab), stdin stops forwarding data and the thread
blocks **forever**. With `biased;`, `event_stream.next()` is polled first every time,
so it prevents `redraw_interval.tick()` and all other `select!` branches from ever
firing. The TUI freezes completely — no input accepted, Ctrl+C ignored, must kill
externally.

**Resolution**: Remove `biased;` from any `select!` block that contains
`event_stream.next()`. Ensure a non-stdin branch (typically `redraw_interval.tick()`)
appears first in the `select!` so it can fire even when stdin is blocked.

**Fixed in**: `turn.rs:1291` (commit `7d7c2cab6`).

**Check all 3 nested select blocks** in `crates/jcode-tui/src/tui/app/turn.rs`:
- Line ~122: API-call select ✅ (no biased, redraw before event)
- Line ~254: streaming select ✅ (no biased, redraw before event)
- Line ~1291: tool execution select ✅ (no biased)

When merging upstream, grep for `biased;` + `event_stream` in every `select!` block.

### 2. Never add `client_focused` / `FocusLost` tracking

Same root cause as above. Downstream's `client_focused` tracking (from commit
`aca168048`) throttled redraws when unfocused — but since `biased;` already prevents
redraw, the throttling only made recovery slower.

**Fixed in**: commit `7d7c2cab6` (removed `client_focused`, `set_client_focused`,
`unfocused_redraw_warranted`, `FocusLost` handlers, and the `TuiState` trait methods).

### 3. Session picker filter modes

When adding a new `SessionFilterMode` variant (e.g. `Active`), it must be added to
`filter.rs:sessio]n_matches_filter_mode()` and all cycle/UI code. Missing a variant
causes a non-exhaustive patterns compile error.

When changing filter logic (e.g. `Cursor => session_is_open_code` → `Cursor =>
session_is_cursor`), verify each mode's predicate is correct.
