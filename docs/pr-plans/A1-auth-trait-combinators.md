# PR Plan: Auth Trait andThen + Pipe Combinators

## Research Summary
- **Source repo(s):** opencode (`/tmp/feature-research/opencode/packages/llm/src/route/auth.ts`)
- **Key files inspected:** `crates/jcode-llm-core/src/auth.rs` (existing 17,901 bytes, 670 lines, 20+ tests)
- **Reference code:** opencode `auth.ts` has `andThen`, `orElse`, `pipe` as combinator methods on auth strategies

## Why This Feature Is Missing in jcode
jcode's Auth trait already has a rich set of 8 types (None, Bearer, ApiKey, Header, Remove, Custom, Optional, Config) and the `or_else` combinator. However, it was missing:
- **`andThen`** — chain two auths sequentially (both must succeed). Needed for composing header injection + custom validation.
- **`pipe`** — apply a function to `Box<dyn Auth>` for fluent transformations. Needed for builder-style chaining.

## Alternatives Considered

| Approach | Source Repo | Pros | Cons | Decision |
|----------|-------------|------|------|----------|
| Free-standing `and_then()` fn | opencode | Simple, no trait changes | Less discoverable | Chosen: dyn Auth method |
| `pipe` as method on Auth trait | opencode | Fluent API | Only works with `Box<dyn Auth>` | Chosen: dyn Auth method (matches usage pattern) |
| Generic `AndThenAuth<S, T>` | N/A | Type-safe | Over-engineered for current needs | Deferred: `Box<dyn Auth>` is sufficient |

## Chosen Approach
1. **`AndThenAuth`** struct wrapping two `Box<dyn Auth>` — runs first then second, short-circuits on first failure
2. **`and_then(self: Box<Self>, other: Box<dyn Auth>) -> AndThenAuth`** method on `dyn Auth`
3. **`pipe(self: Box<Self>, f: impl FnOnce(Box<dyn Auth>) -> A) -> A`** method on `dyn Auth` for fluent transformation
4. **`pipe_auth()`** free function for ergonomic use without trait import

## Implementation Plan
- `crates/jcode-llm-core/src/auth.rs`:
  - Add `AndThenAuth` struct with `Auth` impl
  - Add `and_then()` and `pipe()` methods on `impl dyn Auth`
  - Add 5 new tests (both succeed, first fails, second fails, describe, pipe)
- `crates/jcode-llm-core/src/lib.rs`:
  - Fix pre-existing stale `test_auth_works()` test that used `Auth::bearer()` and `Request::new()` which no longer exist
- `crates/jcode-llm-core/src/route.rs`:
  - Fix pre-existing stale generic-type annotations on `Route<TestBody, TestEvent, TestEvent, TestState>`
- `crates/jcode-llm-core/src/schema.rs`:
  - Add `PartialEq, Eq` derive on `ModelRef` for test assertions

## Risk Analysis
- **Performance:** Negligible — two pointer dereferences for `andThen`
- **Compatibility:** Fully backward compatible — all existing API unchanged
- **Security:** No new attack surface

## Success Criteria
- [x] `cargo check -p jcode-llm-core` passes
- [x] `cargo test -p jcode-llm-core` — all 61 tests pass
- [x] `cargo test -p jcode-llm-core --doc` — 0 failed, 1 ignored (intentional)
- [x] All new tests pass: `test_and_then_both_succeed`, `test_and_then_first_fails`, `test_and_then_second_fails`, `test_and_then_describe`, `test_pipe_transforms_auth_type`
- [x] PARITY.md updated
