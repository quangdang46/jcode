//! Core types for the inband dialect layer.
//!
//! Inband (streaming) tool-call parsing for non‑JSON providers.
//! Each dialect implements [`InbandScanner`] which is fed chunks of streaming
//! LLM output and emits structured [`InbandScanEvent`]s.  The scanner is a
//! state machine that buffers partial tags/tokens across chunk boundaries
//! and only emits fully‑parsed events.

use serde_json::Value;

// ---------------------------------------------------------------------------
// Scan events
// ---------------------------------------------------------------------------

/// An event emitted by an [`InbandScanner`] as it processes streaming text.
#[derive(Debug, Clone, PartialEq)]
pub enum InbandScanEvent {
    /// Plain text content.
    Text(String),
    /// The model has started a thinking/scratchpad block.
    ThinkingStart,
    /// Delta content inside a thinking block.
    ThinkingDelta(String),
    /// The thinking block ended, carrying the full accumulated text.
    ThinkingEnd(String),
    /// The model has started emitting a tool call.
    ToolStart {
        id: String,
        name: String,
    },
    /// Delta for a named argument inside an active tool call.
    ToolArgDelta {
        id: String,
        name: String,
        key: String,
        delta: String,
    },
    /// A tool call has been fully emitted.
    ToolEnd {
        id: String,
        name: String,
        arguments: Value,
        /// Optional raw block for debugging / reproducibility.
        raw_block: Option<String>,
    },
}

// ---------------------------------------------------------------------------
// Scanner trait
// ---------------------------------------------------------------------------

/// A streaming parser for a specific dialect's inband tool‑call format.
///
/// Callers feed chunks of streaming LLM output via [`feed`](InbandScanner::feed)
/// and receive a batch of events.  At end of stream [`flush`](InbandScanner::flush)
/// returns any buffered/leftover events.
pub trait InbandScanner {
    /// Feed a chunk of streaming text and return any complete events.
    fn feed(&mut self, text: &str) -> Vec<InbandScanEvent>;

    /// Flush any remaining buffered events (call at end of stream).
    fn flush(&mut self) -> Vec<InbandScanEvent>;
}

// ---------------------------------------------------------------------------
// Dialect enum + tools used by render fns
// ---------------------------------------------------------------------------

/// Supported inband tool‑call dialects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dialect {
    Anthropic,
    Gemini,
    Gemma,
    Glm,
    Harmony,
    Hermes,
    /// A jcode‑native dialect (currently the JSON‑in‑tag style, same as Hermes).
    Jcode,
    Kimi,
    MiniMax,
    Qwen3,
    /// Generic XML format – delegates to Anthropic or DeepSeek scanner.
    Xml,
    /// DeepSeek's DSML pseudo‑XML with fullwidth delimiters.
    DeepSeek,
}

impl Dialect {
    /// All known dialect variants (minus Xml which is a delegator).
    pub const ALL: &'static [Dialect] = &[
        Dialect::Anthropic,
        Dialect::DeepSeek,
        Dialect::Gemini,
        Dialect::Gemma,
        Dialect::Glm,
        Dialect::Harmony,
        Dialect::Hermes,
        Dialect::Jcode,
        Dialect::Kimi,
        Dialect::MiniMax,
        Dialect::Qwen3,
        Dialect::Xml,
    ];

    /// Human‑readable name.
    pub fn name(self) -> &'static str {
        match self {
            Dialect::Anthropic => "anthropic",
            Dialect::DeepSeek => "deepseek",
            Dialect::Gemini => "gemini",
            Dialect::Gemma => "gemma",
            Dialect::Glm => "glm",
            Dialect::Harmony => "harmony",
            Dialect::Hermes => "hermes",
            Dialect::Jcode => "jcode",
            Dialect::Kimi => "kimi",
            Dialect::MiniMax => "minimax",
            Dialect::Qwen3 => "qwen3",
            Dialect::Xml => "xml",
        }
    }
}

// ---------------------------------------------------------------------------
// Scanner options
// ---------------------------------------------------------------------------

/// Options passed to a dialect's scanner constructor.
#[derive(Debug, Clone, Default)]
pub struct InbandScannerOptions {
    /// Parse thinking/scratchpad markers as dedicated events (true ≈ mode).
    pub parse_thinking: bool,
    /// XML tagset variant for the „xml“ dialect: "anthropic" or "dsml".
    pub xml_tagset: Option<String>,
}

// ---------------------------------------------------------------------------
// Dialect prompt (inlined from oh-my-pi markdown prompts)
// ---------------------------------------------------------------------------

/// The system‑prompt fragment for a dialect's inband tool format.
/// Instructs the model on how to emit tool calls in the dialect's format.
pub const ANTHROPIC_PROMPT: &str = r#"Respond with tool calls using <function_calls> or <invoke> XML tags per the function definitions above."#;

pub const DEEPSEEK_PROMPT: &str = r#"Respond with tool calls using DSML <｜tool▁calls▁begin｜> markers."#;

pub const GEMINI_PROMPT: &str = r#"Respond with tool calls inside a ```tool_code Python fenced block."#;

pub const GEMMA_PROMPT: &str = r#"Respond with tool calls inside a ```tool_code Python fenced block (Gemma variant)."#;

pub const GLM_PROMPT: &str = r#"Respond with tool calls using XML <tool_call> tags with GLM schema."#;

pub const HARMONY_PROMPT: &str = r#"Respond with tool calls using Harmony's custom token format."#;

pub const HERMES_PROMPT: &str = r#"Respond with tool calls inside <tool_call> tags containing JSON with "name" and "arguments" fields."#;

pub const KIMI_PROMPT: &str = r#"Respond with tool calls using <|tool_calls_section_begin|> markers."#;

pub const MINIMAX_PROMPT: &str = r#"Respond with tool calls using MiniMax's JSON stream format."#;

pub const QWEN3_PROMPT: &str = r#"Respond with tool calls inside a Python-code fenced block using Qwen3's convention."#;

pub const XML_PROMPT: &str = r#"Respond with tool calls using generic XML <invoke name="..."> tags."#;

pub const JCODE_PROMPT: &str = r#"Respond with tool calls inside <tool_call> tags containing JSON."#;
