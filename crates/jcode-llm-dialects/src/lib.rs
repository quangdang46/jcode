//! jcode inband (streaming) tool-call dialect scanners.
//!
//! This crate implements 12 inband tool-call formats used by various LLM
//! providers. Each dialect provides a state-machine scanner that parses
//! streaming text and emits structured [`InbandScanEvent`]s.

pub mod anthropic;
pub mod deepseek;
pub mod gemini;
pub mod hermes;
pub mod kimi;
pub mod types;
pub mod xml;

use types::*;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Create a scanner for the given dialect with the given options.
pub fn create_inband_scanner(dialect: Dialect, options: &InbandScannerOptions) -> Box<dyn InbandScanner> {
    match dialect {
        Dialect::Hermes | Dialect::Jcode => Box::new(hermes::HermesInbandScanner::new(options)),
        Dialect::Kimi => Box::new(kimi::KimiInbandScanner::new(options)),
        Dialect::Gemini | Dialect::Gemma => Box::new(gemini::GeminiInbandScanner::new(options)),
        Dialect::Anthropic => Box::new(anthropic::AnthropicInbandScanner::new(options)),
        Dialect::DeepSeek => Box::new(deepseek::DeepSeekInbandScanner::new(options)),
        Dialect::Xml => Box::new(xml::XmlInbandScanner::new(options)),
        Dialect::Glm | Dialect::Harmony | Dialect::MiniMax | Dialect::Qwen3 => {
            // Placeholder — actual implementations in Phase 2
            Box::new(hermes::HermesInbandScanner::new(options))
        }
    }
}

/// Return the system-prompt fragment instructing the model to use the dialect's
/// inband tool-call format.
pub fn dialect_prompt(dialect: Dialect) -> &'static str {
    match dialect {
        Dialect::Anthropic => crate::types::ANTHROPIC_PROMPT,
        Dialect::DeepSeek => crate::types::DEEPSEEK_PROMPT,
        Dialect::Gemini => crate::types::GEMINI_PROMPT,
        Dialect::Gemma => crate::types::GEMMA_PROMPT,
        Dialect::Glm => crate::types::GLM_PROMPT,
        Dialect::Harmony => crate::types::HARMONY_PROMPT,
        Dialect::Hermes => crate::types::HERMES_PROMPT,
        Dialect::Jcode => crate::types::JCODE_PROMPT,
        Dialect::Kimi => crate::types::KIMI_PROMPT,
        Dialect::MiniMax => crate::types::MINIMAX_PROMPT,
        Dialect::Qwen3 => crate::types::QWEN3_PROMPT,
        Dialect::Xml => crate::types::XML_PROMPT,
    }
}
