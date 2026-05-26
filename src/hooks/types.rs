//! Hook JSON input/output contract types

use serde::{Deserialize, Serialize};

/// Input passed to hooks via stdin JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookInput {
    pub session_id: String,
    pub transcript_path: String,
    pub cwd: String,
    pub hook_event_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_input: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
}

impl HookInput {
    /// Create input for a tool use hook
    pub fn for_tool(
        session_id: &str,
        transcript_path: &str,
        cwd: &str,
        tool_name: &str,
        tool_input: serde_json::Value,
    ) -> Self {
        Self {
            session_id: session_id.to_string(),
            transcript_path: transcript_path.to_string(),
            cwd: cwd.to_string(),
            hook_event_name: "PreToolUse".to_string(),
            agent_id: None,
            agent_type: None,
            tool_name: Some(tool_name.to_string()),
            tool_input: Some(tool_input),
            tool_use_id: None,
            permission_mode: None,
        }
    }
}

/// Output expected from hooks via stdout JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookOutput {
    #[serde(default = "default_true")]
    pub continue_: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suppress_output: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hook_specific_output: Option<HookSpecificOutput>,
}

fn default_true() -> bool { true }

/// Event-specific output fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookSpecificOutput {
    pub hook_event_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_decision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_decision_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_input: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,
}

impl HookOutput {
    /// Create a continue output (default)
    pub fn continue_() -> Self {
        Self {
            continue_: true,
            suppress_output: None,
            stop_reason: None,
            decision: None,
            reason: None,
            system_message: None,
            hook_specific_output: None,
        }
    }

    /// Create a block output
    pub fn block(reason: &str) -> Self {
        Self {
            continue_: false,
            suppress_output: None,
            stop_reason: Some(reason.to_string()),
            decision: Some("block".to_string()),
            reason: None,
            system_message: None,
            hook_specific_output: None,
        }
    }
}