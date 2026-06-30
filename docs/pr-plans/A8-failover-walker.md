# PR Plan: A8 — Reactive Failover Walker

## Research Summary
- **Source repo**: oh-my-openagent (`packages/omo-opencode/src/hooks/runtime-fallback/`, ~55 files)
- **Key files inspected**:
  - `types.ts` — `FallbackState`, `FallbackConfig`, `AutoRetryDispatch`, walker types
  - `fallback-state.ts` — state machine for fallback lifecycle
  - `auto-retry-dispatch.ts` — orchestrator that catches streaming errors, aborts, picks fallback, resends
  - `fallback-retry-dispatcher.ts` — dispatcher logic for retry with cooldowns

## Why This Feature Is Missing in jcode
jcode already has the **building blocks**:
- `failover.rs` — 997 lines: `classify_failover_error_message_structured()` with 30+ error codes, `FailoverDecision` enum
- `fallback_pick.rs` — `pick_next_fallback_route()` with 3-tier ranking

But **no runtime orchestration** — no state machine that:
1. Catches a provider error mid-stream
2. Classifies it → decides to fail over
3. Aborts the current request internally
4. Picks the next available model from the fallback chain
5. Tracks per-session cooldowns and retry counts

This is what oh-my-openagent's `runtime-fallback/` module does and what this PR adds.

## Alternatives Considered
| Approach | Source Repo | Pros | Cons | Decision |
|----------|-------------|------|------|----------|
| Full state machine + walker | oh-my-openagent | Composable, handles all cases, reuse existing error classifier | More code initially | ✅ Chosen — matches existing `failover.rs` pattern |
| Inline retry in provider | oh-my-pi | Simpler, less code | No session tracking, no cooldowns, no equivalence check | Rejected — too limited |
| Error handler callbacks | claude-code | Flexible | No state machine means caller must do everything | Rejected — pushes complexity up |

## Chosen Approach
Add `failover_walker.rs` to `jcode-provider-core` that composes:
1. `classify_failover_error_message_structured` for error classification
2. `pick_next_fallback_route` for route selection
3. New `ReactiveFailoverWalker` struct with per-session state tracking

## Implementation Plan
**New file:** `crates/jcode-provider-core/src/failover_walker.rs`

Types:
- `WalkState` — per-session state (original_model, current_model, fallback_index, failed_models with cooldowns, attempt_count)
- `PreparedFallback` — result of fallback preparation
- `WalkResult` — result of walking a failover (should_failover, new_model, decision, error_code, message)
- `ReactiveFailoverWalker` — main struct with methods:
  - `register_session`, `unregister_session`, `get_state`
  - `record_failure`, `record_success` — cooldown management
  - `is_model_in_cooldown` — check if a model is cooling down
  - `find_next_available_fallback` — walk fallback chain respecting cooldowns + equivalence
  - `prepare_fallback` — pick next candidate with max-attempts check
  - `walk_failover` — **main entry point**: classify → decide → pick → update state → return result
- `is_internally_aborted` / `mark_internally_aborted` — for consumers to know if a failover was triggered

**Edit:** `crates/jcode-provider-core/src/lib.rs` — add `pub mod failover_walker;`

## Test Cases
1. Session lifecycle (register → get_state → unregister)
2. Cooldown tracking (record_failure → is_model_in_cooldown → record_success → no longer cooldown)
3. Fallback skips equivalent models
4. Fallback skips cooldown models
5. Max attempts reached
6. walk_failover for rate-limited (picks fallback)
7. walk_failover for context length (retries — RetryNextProvider)
8. walk_failover internally aborted tracking
9. walk_failover no routes available
10. models_equivalent helper
11. unregister cleans internally_aborted set

## Risk Analysis
- **Performance**: HashMap lookups per failover event — trivial, not in hot path
- **Compatibility**: New module, no existing code changed
- **Security**: No external input parsing
- **Thread safety**: Not yet — see Future Work

## Future Work (out of scope for this PR)
- Thread safety (Arc<Mutex<>> or crossbeam for concurrent access)
- Persistent cooldown storage across restarts
- Integration with `app-core` streaming loop (the hook point)
- Metrics/telemetry for failover events

## Success Criteria
- [x] cargo check -p jcode-provider-core passes
- [x] cargo test -p jcode-provider-core failover_walker passes (11 tests)
- [ ] PARITY.md updated
- [ ] docs/PR_BACKLOG.md updated
- [ ] Branch pushed and PR opened
