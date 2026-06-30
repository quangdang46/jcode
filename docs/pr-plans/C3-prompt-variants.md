# PR Plan: C3 — Prompt Variants Per Model (Claude vs GPT vs Gemini)

## Research Summary
- **Source repo**: oh-my-openagent (`packages/prompts-core/src/variant-resolver.ts`, `types.ts`)
- **Key files inspected**:
  - `variant-resolver.ts` — L42-158: `resolve_variant(model_id, variants, default)` with fallback chain: exact model → family → default
  - `types.ts` — L10-45: `PromptVariant`, `VariantMap<Prompt>` types
  - `mode-prompts.ts` — How mode-level prompts use variants

## Why This Feature Is Missing in jcode
- **jcode** has a single global system prompt (`SYSTEM_PROMPT`) in `jcode-base/src/prompt.rs`. It's the same string for every model.
- **oh-my-openagent** provides model-specific prompt variants: Claude gets `system_prompt_claude.md`, GPT gets `system_prompt_gpt.md`, Gemini gets `system_prompt_gemini.md`. Each variant is tuned to that model family's instruction-following profile.
- Switching models mid-session (via failover) currently uses the same prompt, but a prompt tuned for Claude may not work optimally on GPT.

## Alternatives Considered
| Approach | Source Repo | Pros | Cons | Decision |
|----------|-------------|------|------|----------|
| **Model-specific markdown files + resolver** | oh-my-openagent | Clean separation, easy to edit per-model prompts, clear fallback chain | Needs a new file per model | **Chosen** |
| Single configurable template with model variables | — | One file | Model-specific nuances hard to express | Rejected |

## Chosen Approach
Provide per-model-family system prompt markdown files in `crates/jcode-base/src/prompt/`:
- `system_prompt_claude.md` — tuned for Claude (XML-savvy, proactive, tool-focused)
- `system_prompt_gpt.md` — tuned for GPT (markdown-oriented, step-by-step reasoning)
- `system_prompt_gemini.md` — tuned for Gemini (JSON-oriented, structured output)

A `VariantResolver` in `variant_resolver.rs` selects the right prompt based on the model ID, with fallback: exact model match → family prefix match → default.

## Implementation Plan
1. Create `system_prompt_claude.md`, `system_prompt_gpt.md`, `system_prompt_gemini.md` in `crates/jcode-base/src/prompt/` (subagent)
2. Create `variant_resolver.rs` with `VariantResolver::resolve(model_id, variants, default)` (subagent)
3. Wire into `prompt.rs` — modify `system_prompt()` to accept optional model info
4. Wire into `prompting.rs` — pass active model to prompt selection
5. Update `prompt_tests.rs` — add tests for variant resolution

## Risk Analysis
- **Performance**: Resolution is O(1) via `HashMap` or simple pattern match. No runtime cost per token.
- **Compatibility**: 100% backward compatible — if no model-specific variant exists, uses the current default prompt.

## Success Criteria
- [ ] cargo build passes
- [ ] cargo test passes
- [ ] PARITY.md updated
- [ ] Manual verification works
