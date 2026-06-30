# PR Plan: A6 — 13 Inband Dialect Layer

## Research Summary
- **Source repo**: oh-my-pi (`/tmp/feature-research/oh-my-pi/packages/ai/src/dialect/`)
- **Key files inspected**:
  - `types.ts` — `InbandScanner` interface (feed/flush), `DialectDefinition` interface (scanner + 6 render fns), `InbandScanEvent` union type (text/thinkingStart/thinkingDelta/thinkingEnd/toolStart/toolArgDelta/toolEnd)
  - `factory.ts` — maps 11 dialect names → definitions: anthropic, deepseek, gemini, gemma, glm, harmony, hermes, kimi, minimax, qwen3, xml (+ jcode fallback, 13 total per PR_BACKLOG)
  - `catalog.ts` — `renderInbandToolPrompt()` wraps catalog + dialect prompt into a template
  - `anthropic.ts` — 596-line XML state-machine scanner (6 states: outside/section/invoke/parameter/thinking), tag-based with `<function_calls>`/`<invoke name=...>`/`<parameter name=...>`
  - `deepseek.ts` — 595-line scanner: fullwidth/ASCII DSML tokens, 9-state machine, legacy JSON fallback
  - `gemini.ts` — 596-line scanner: Python-like ```` ```tool_code ```` fence parser with py-value deserialization
  - `kimi.ts` — 340-line scanner: token-delimited with `<|tool_calls_section_begin|>` markers
  - `hermes.ts` — 206-line scanner: JSON-in-`<tool_call>` tags, simplest dialect
  - `xml.ts` — 90-line delegator: wraps either AnthropicInbandScanner (default) or DeepSeekInbandScanner (DSML mode)

## Why This Feature Is Missing in jcode
The `jcode-llm-dialects` crate is a 12-line stub (just `pub fn version()`). Per PARITY.md §XIV line 533: *"Inband dialect layer: 13 dialects for non-JSON tool-call providers"* — this is a known gap. All 11+2 dialect scanners, the scanner trait, the definition trait, and all rendering functions must be implemented from scratch.

## Alternatives Considered

| Approach | Source | Pros | Cons | Decision |
|----------|--------|------|------|----------|
| **Full Rust rewrite of all 13 scanners** | oh-my-pi | Idiomatic Rust, no JS bridge overhead, fast | Large effort (2000+ LOC) | **Chosen** — matches jcode's Rust-only architecture |
| Js-rquickJS bridge to oh-my-pi dialect JS | oh-my-pi | Zero reimplementation | rquickJS dependency, slow, JS eval overhead, runtime errors | Rejected — sandbox JS is for plugins, not core infrastructure |
| Procedural macro dialect registration | — | Zero boilerplate per dialect | Over-engineered for 13 dialects | Deferred — a simple enum dispatch is sufficient |
| Unified generic scanner parametrized by tag sets | — | Less code duplication | Deepseek/anthropic XML-like but gemini/qwen3 fundamentally different | Rejected — dialect diversity means per-dialect impl is more maintainable |

## Chosen Approach
1. **Core types** in `crates/jcode-llm-dialects/src/lib.rs`:
   - `InbandScanEvent` enum (text, thinkingStart, thinkingDelta, thinkingEnd, toolStart, toolArgDelta, toolEnd)
   - `InbandScanner` trait (feed, flush)
   - `DialectDefinition` struct (dialect name, prompt, scanner factory, 6 render functions)
   - `Dialect` enum (11+1 variants)
   - `get_dialect_definition()`, `create_inband_scanner()` dispatchers

2. **Per-dialect modules** under `crates/jcode-llm-dialects/src/dialects/`:
   - `mod.rs` — enum dispatch
   - `anthropic.rs` — 6-state XML tag scanner
   - `deepseek.rs` — 9-state DSML/fullwidth scanner
   - `gemini.rs` — Python-fence scanner
   - `hermes.rs` — JSON-in-`<tool_call>` scanner
   - `kimi.rs` — token-delimited scanner
   - `qwen3.rs` — code-fence scanner (similar to gemini but with JSON)
   - `gemma.rs` — lightweight variant of gemini
   - `minimax.rs` — JSON-based scanner
   - `glm.rs` — XML-style scanner
   - `harmony.rs` — custom token scanner
   - `xml.rs` — delegator (wraps anthropic or deepseek)

3. **Prompt files** as string constants (inlined, TOML-frontmatter not needed at this level)

## Implementation Plan

**Phase 1 (core + 3 dialects):**
1. `src/lib.rs` — `InbandScanEvent`, `InbandScanner` trait, `Dialect` enum, factory functions
2. `src/dialects/mod.rs` — dispatch
3. `src/dialects/hermes.rs` — simplest dialect (JSON in tags, ~200 LOC)
4. `src/dialects/kimi.rs` — token-delimited (~350 LOC)
5. `src/dialects/gemini.rs` — Python-fence (~500 LOC)
6. Tests for each dialect

**Phase 2 (remaining 9 dialects):**
7. `src/dialects/anthropic.rs` — full XML state machine (~500 LOC)
8. `src/dialects/deepseek.rs` — DSML scanner (~500 LOC)
9. `src/dialects/xml.rs` — delegator (~80 LOC)
10. `src/dialects/qwen3.rs`, `gemma.rs`, `minimax.rs`, `glm.rs`, `harmony.rs`, `jcode.rs`
11. Tests for each

## Risk Analysis
- **Performance**: String scanning is O(n) per token, fine for streaming throughput. No allocation hot-path beyond event emission.
- **Compatibility**: New crate, zero impact on existing code. Only linked when LLM output parsing needs inband tool calls.
- **Correctness**: Edge cases abound (partial tags across token boundaries, self-closing tags, truncated streams). Each scanner tested with feed-then-flush patterns.
- **Security**: No new attack surface — scanners parse text, not user input.

## Success Criteria
- [ ] `cargo check -p jcode-llm-dialects` passes
- [ ] `cargo test -p jcode-llm-dialects` — all dialect tests pass
- [ ] Hermes scanner: can parse `<tool_call>{"name":"get_weather","arguments":{"city":"NYC"}}</tool_call>` from streaming chunks
- [ ] Kimi scanner: can parse `<|tool_calls_section_begin|>` block
- [ ] Gemini scanner: can parse Python fence ```` ```tool_code ```` blocks
- [ ] PARITY.md updated
- [ ] PR_BACKLOG.md updated
