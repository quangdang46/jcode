//! Model-specific system prompt variant resolution.
//!
//! Different LLM families (Claude, GPT, Gemini) have different strengths,
//! weaknesses, and instruction-following preferences. This module provides
//! a mechanism to select the right prompt variant for the active model,
//! falling back to a default when no model-specific variant exists.
//!
//! # Variant resolution order
//!
//! 1. Match the model ID against known model matchers (claude, gpt, gemini).
//! 2. Return the matching variant, or `Default` if no matcher matches.

/// Supported prompt variants for different model families.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptVariant {
    /// Generic/system prompt — works well with any model.
    Default,
    /// Claude-optimized prompt (Anthropic Claude family).
    Claude,
    /// GPT-optimized prompt (OpenAI GPT family).
    Gpt,
    /// Gemini-optimized prompt (Google Gemini family).
    Gemini,
}

impl PromptVariant {
    /// All known variants (excluding `Default`, which is the catch-all).
    pub const fn known_variants() -> &'static [PromptVariant] {
        &[PromptVariant::Claude, PromptVariant::Gpt, PromptVariant::Gemini]
    }
}

/// Resolve which prompt variant to use for a given model ID.
///
/// The resolution uses a prefix-based matcher:
/// - `claude-` or `anthropic/` → `Claude`
/// - `gpt-` → `Gpt`
/// - `gemini-` → `Gemini`
/// - Anything else → `Default`
///
/// The model ID is expected to be the canonical form (lowercased, provider-qualified),
/// as returned by [`jcode_provider_core::model_id::canonical`].
pub fn resolve_prompt_variant(model_id: &str) -> PromptVariant {
    let canonical = model_id.trim().to_ascii_lowercase();

    if canonical.starts_with("claude-") || canonical.starts_with("anthropic/") {
        PromptVariant::Claude
    } else if canonical.starts_with("gpt-") {
        PromptVariant::Gpt
    } else if canonical.starts_with("gemini-") {
        PromptVariant::Gemini
    } else {
        PromptVariant::Default
    }
}

/// Get the static prompt content for a specific variant.
pub fn system_prompt_for_variant(variant: PromptVariant) -> &'static str {
    match variant {
        PromptVariant::Default => super::DEFAULT_SYSTEM_PROMPT,
        PromptVariant::Claude => super::SYSTEM_PROMPT_CLAUDE,
        PromptVariant::Gpt => super::SYSTEM_PROMPT_GPT,
        PromptVariant::Gemini => super::SYSTEM_PROMPT_GEMINI,
    }
}

/// Convenience: resolve variant from a model ID and return the prompt text.
pub fn system_prompt_for_model(model_id: &str) -> &'static str {
    let variant = resolve_prompt_variant(model_id);
    system_prompt_for_variant(variant)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_claude_model() {
        assert_eq!(resolve_prompt_variant("claude-opus-4-6"), PromptVariant::Claude);
        assert_eq!(resolve_prompt_variant("claude-sonnet-4-6[1m]"), PromptVariant::Claude);
        assert_eq!(resolve_prompt_variant("claude-haiku-4-5"), PromptVariant::Claude);
        assert_eq!(
            resolve_prompt_variant("anthropic/claude-opus-4-6"),
            PromptVariant::Claude
        );
    }

    #[test]
    fn resolve_gpt_model() {
        assert_eq!(resolve_prompt_variant("gpt-5.5"), PromptVariant::Gpt);
        assert_eq!(resolve_prompt_variant("gpt-5.3-codex"), PromptVariant::Gpt);
        assert_eq!(resolve_prompt_variant("gpt-4o"), PromptVariant::Gpt);
    }

    #[test]
    fn resolve_gemini_model() {
        assert_eq!(resolve_prompt_variant("gemini-2.5-pro"), PromptVariant::Gemini);
        assert_eq!(resolve_prompt_variant("gemini-3-1-pro"), PromptVariant::Gemini);
        assert_eq!(
            resolve_prompt_variant("gemini-2.0-flash"),
            PromptVariant::Gemini
        );
    }

    #[test]
    fn resolve_unknown_model_falls_back_to_default() {
        assert_eq!(resolve_prompt_variant("deepseek-v4"), PromptVariant::Default);
        assert_eq!(resolve_prompt_variant("llama-3-70b"), PromptVariant::Default);
        assert_eq!(resolve_prompt_variant("mistral-7b"), PromptVariant::Default);
        assert_eq!(resolve_prompt_variant(""), PromptVariant::Default);
    }

    #[test]
    fn resolve_is_case_insensitive() {
        assert_eq!(resolve_prompt_variant("Claude-Opus-4-6"), PromptVariant::Claude);
        assert_eq!(resolve_prompt_variant("GPT-5.5"), PromptVariant::Gpt);
        assert_eq!(resolve_prompt_variant("Gemini-2.5-Pro"), PromptVariant::Gemini);
    }

    #[test]
    fn resolve_trims_whitespace() {
        assert_eq!(
            resolve_prompt_variant("  claude-opus-4-6  "),
            PromptVariant::Claude
        );
    }

    #[test]
    fn system_prompt_for_model_returns_non_empty() {
        for variant in &[PromptVariant::Default, PromptVariant::Claude, PromptVariant::Gpt, PromptVariant::Gemini] {
            let prompt = system_prompt_for_variant(*variant);
            assert!(!prompt.is_empty(), "{:?} prompt should not be empty", variant);
        }
    }

    #[test]
    fn system_prompt_for_model_via_resolver() {
        let prompt = system_prompt_for_model("gpt-5.5");
        assert!(!prompt.is_empty());
        // GPT variant should not contain Claude-specific phrasing
        assert!(!prompt.contains("Claude"));
    }
}
