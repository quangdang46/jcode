//! HookRegistry - manages hook registration and lookup by event type
//!
//! Provides efficient lookup of hooks filtered by event type and
//! matcher pattern against the current execution context.

use std::collections::HashMap;

use crate::hooks::config::{HookEvent, HookHandlerConfig, HooksConfig};
use crate::hooks::matcher::{HookMatcher, MatcherContext, matches};

/// Context passed to hooks for matching decisions.
///
/// Contains all information about the current execution context
/// that hooks can use to determine if they should run.
#[derive(Debug, Clone)]
pub struct HookContext {
    /// Session identifier
    pub session_id: String,
    /// Path to the session transcript file
    pub transcript_path: String,
    /// Current working directory
    pub cwd: String,
    /// Name of the hook event being triggered
    pub hook_event_name: String,
    /// Optional agent ID
    pub agent_id: Option<String>,
    /// Optional agent type
    pub agent_type: Option<String>,
    /// Optional tool name being executed
    pub tool_name: Option<String>,
    /// Optional tool input (serialized JSON)
    pub tool_input: Option<serde_json::Value>,
    /// Optional tool use ID
    pub tool_use_id: Option<String>,
    /// Optional permission mode
    pub permission_mode: Option<String>,
}

impl HookContext {
    /// Create a new empty HookContext
    pub fn new(session_id: &str, transcript_path: &str, cwd: &str, hook_event_name: &str) -> Self {
        Self {
            session_id: session_id.to_string(),
            transcript_path: transcript_path.to_string(),
            cwd: cwd.to_string(),
            hook_event_name: hook_event_name.to_string(),
            agent_id: None,
            agent_type: None,
            tool_name: None,
            tool_input: None,
            tool_use_id: None,
            permission_mode: None,
        }
    }

    /// Create a new HookContext for a tool-related event
    pub fn for_tool(tool_name: String, session_id: String, cwd: String) -> Self {
        Self {
            session_id,
            transcript_path: String::new(),
            cwd,
            hook_event_name: "PreToolUse".to_string(),
            agent_id: None,
            agent_type: None,
            tool_name: Some(tool_name),
            tool_input: None,
            tool_use_id: None,
            permission_mode: None,
        }
    }

    pub fn for_session_start(session_id: String, cwd: String) -> Self {
        Self {
            session_id,
            transcript_path: String::new(),
            cwd,
            hook_event_name: "session_start".to_string(),
            agent_id: None,
            agent_type: None,
            tool_name: None,
            tool_input: None,
            tool_use_id: None,
            permission_mode: None,
        }
    }

    pub fn for_session_end(session_id: String) -> Self {
        Self {
            session_id,
            transcript_path: String::new(),
            cwd: String::new(),
            hook_event_name: "session_end".to_string(),
            agent_id: None,
            agent_type: None,
            tool_name: None,
            tool_input: None,
            tool_use_id: None,
            permission_mode: None,
        }
    }

    pub fn for_permission_request(
        tool_name: String,
        session_id: String,
        permission_mode: String,
    ) -> Self {
        Self {
            session_id,
            transcript_path: String::new(),
            cwd: String::new(),
            hook_event_name: "permission_request".to_string(),
            agent_id: None,
            agent_type: None,
            tool_name: Some(tool_name),
            tool_input: None,
            tool_use_id: None,
            permission_mode: Some(permission_mode),
        }
    }

    pub fn for_permission_denied(
        session_id: String,
        permission_mode: String,
    ) -> Self {
        Self {
            session_id,
            transcript_path: String::new(),
            cwd: String::new(),
            hook_event_name: "permission_denied".to_string(),
            agent_id: None,
            agent_type: None,
            tool_name: None,
            tool_input: None,
            tool_use_id: None,
            permission_mode: Some(permission_mode),
        }
    }

    pub fn for_tool_error(tool_name: String, session_id: String, error: String) -> Self {
        Self {
            session_id,
            transcript_path: String::new(),
            cwd: String::new(),
            hook_event_name: "tool_error".to_string(),
            agent_id: None,
            agent_type: None,
            tool_name: Some(tool_name),
            tool_input: Some(serde_json::json!({ "error": error })),
            tool_use_id: None,
            permission_mode: None,
        }
    }

    /// Build a MatcherContext for use with the hook matcher
    ///
    /// Uses tool_name as the primary target for pattern matching.
    /// If additional context text is needed (e.g., full command for Bash),
    /// use `with_context()` instead.
    pub fn matcher_context(&self) -> MatcherContext<'_> {
        MatcherContext::new(self.tool_name.as_deref().unwrap_or(""))
    }

    /// Build a MatcherContext with additional context text
    pub fn matcher_context_with_context<'a>(&'a self, context: &'a str) -> MatcherContext<'a> {
        MatcherContext::with_context(self.tool_name.as_deref().unwrap_or(""), context)
    }
}

/// Registry of hooks organized by event type.
///
/// Provides lookup of hooks by event type and filtering by matcher pattern.
#[derive(Debug, Clone)]
pub struct HookRegistry {
    hooks: HashMap<HookEvent, Vec<HookHandlerConfig>>,
}

impl HookRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            hooks: HashMap::new(),
        }
    }

    /// Create a registry from a HooksConfig
    ///
    /// Converts the flat config entries into event-keyed vectors.
    pub fn from_config(config: HooksConfig) -> Self {
        let mut registry = Self::new();

        // HooksConfig.events maps event names to handler configs
        for (event_name, handler) in config.events.into_iter() {
            // Parse the event name to get the HookEvent enum value
            if let Some(event) = HookEvent::parse(&event_name) {
                registry.hooks.entry(event).or_default().push(handler);
            } else {
                // If event name doesn't parse, try using it as a custom event
                let custom_event = HookEvent::Custom(event_name);
                registry
                    .hooks
                    .entry(custom_event)
                    .or_default()
                    .push(handler);
            }
        }

        registry
    }

    /// Get all hooks for a specific event type
    pub fn get_hooks(&self, event: &HookEvent) -> &[HookHandlerConfig] {
        self.hooks.get(event).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Get hooks matching the given event and context criteria.
    ///
    /// Returns handlers whose matcher (if any) matches the tool_name
    /// in the provided context. All 4 matcher types are supported:
    /// - Exact: matches a single tool name exactly
    /// - Multi: matches any of several tool names  
    /// - Regex: matches tool name via regex pattern
    /// - Wildcard: matches any tool name
    pub fn get_matching(
        &self,
        event: &HookEvent,
        context: &HookContext,
    ) -> Vec<&HookHandlerConfig> {
        self.get_hooks(event)
            .iter()
            .filter(|handler| {
                // Skip handlers that have an `if_` condition that evaluates to false
                if let Some(condition) = self.get_handler_condition(handler) {
                    if !self.evaluate_condition(condition, context) {
                        return false;
                    }
                }

                // Get the matcher for this handler
                if let Some(matcher) = self.get_handler_matcher(handler) {
                    // Build matcher context - include command for regex matching
                    let ctx = context.matcher_context();
                    matches(&matcher, &ctx)
                } else {
                    // No matcher means wildcard - always match
                    true
                }
            })
            .collect()
    }

    /// Get the matcher from a handler configuration
    ///
    fn get_handler_matcher(&self, handler: &HookHandlerConfig) -> Option<&HookMatcher> {
        match handler {
            HookHandlerConfig::Command(cmd) => cmd.matcher.as_ref(),
            HookHandlerConfig::Http(http) => http.matcher.as_ref(),
        }
    }

    /// Get the condition (`if_`) from a handler configuration
    fn get_handler_condition<'a>(&self, handler: &'a HookHandlerConfig) -> Option<&'a str> {
        match handler {
            HookHandlerConfig::Command(cmd) => cmd.if_.as_deref(),
            HookHandlerConfig::Http(http) => http.if_.as_deref(),
        }
    }

    /// Evaluate a condition against the context
    ///
    /// Conditions are shell-like expressions that can check context fields.
    fn evaluate_condition(&self, condition: &str, context: &HookContext) -> bool {
        // Simple condition evaluation
        // Format: "field=value" or "field!=value"
        if let Some((field, value)) = condition.split_once('=') {
            let field = field.trim();
            let value = value.trim();
            match field {
                "tool_name" => context.tool_name.as_deref() == Some(value),
                "agent_type" => context.agent_type.as_deref() == Some(value),
                "permission_mode" => context.permission_mode.as_deref() == Some(value),
                _ => true,
            }
        } else if let Some((field, value)) = condition.split_once("!=") {
            let field = field.trim();
            let value = value.trim();
            match field {
                "tool_name" => context.tool_name.as_deref() != Some(value),
                "agent_type" => context.agent_type.as_deref() != Some(value),
                "permission_mode" => context.permission_mode.as_deref() != Some(value),
                _ => true,
            }
        } else {
            // Unknown condition format - allow by default
            true
        }
    }

    /// Check if the registry is empty (no hooks registered)
    pub fn is_empty(&self) -> bool {
        self.hooks.is_empty() || self.hooks.values().all(Vec::is_empty)
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_registry_is_empty() {
        let registry = HookRegistry::new();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_from_empty_config() {
        let config = HooksConfig::default();
        let registry = HookRegistry::from_config(config);
        assert!(registry.is_empty());
    }

    #[test]
    fn test_get_hooks_returns_empty_for_unknown_event() {
        let registry = HookRegistry::new();
        let hooks = registry.get_hooks(&HookEvent::PreToolUse);
        assert!(hooks.is_empty());
    }

    #[test]
    fn test_from_config_with_single_event() {
        let mut config = HooksConfig::default();
        config.events.insert(
            "pre_tool_use".to_string(),
            HookHandlerConfig::Command(crate::hooks::config::CommandHandlerConfig {
                command: "test_command".to_string(),
                ..Default::default()
            }),
        );

        let registry = HookRegistry::from_config(config);
        let hooks = registry.get_hooks(&HookEvent::PreToolUse);
        assert_eq!(hooks.len(), 1);
        assert!(matches!(&hooks[0], HookHandlerConfig::Command(cmd) if cmd.command == "test_command"));
    }

    #[test]
    fn test_from_config_with_custom_event() {
        let mut config = HooksConfig::default();
        config.events.insert(
            "custom:my_event".to_string(),
            HookHandlerConfig::Command(crate::hooks::config::CommandHandlerConfig {
                command: "custom_handler".to_string(),
                ..Default::default()
            }),
        );

        let registry = HookRegistry::from_config(config);
        let hooks = registry.get_hooks(&HookEvent::Custom("my_event".to_string()));
        assert_eq!(hooks.len(), 1);
    }

    #[test]
    fn test_hook_context_for_tool() {
        let context = HookContext::for_tool(
            "session-123",
            "/tmp/transcript.json",
            "/project",
            "Bash",
            serde_json::json!({ "command": "ls -la" }),
        );

        assert_eq!(context.session_id, "session-123");
        assert_eq!(context.transcript_path, "/tmp/transcript.json");
        assert_eq!(context.cwd, "/project");
        assert_eq!(context.hook_event_name, "PreToolUse");
        assert_eq!(context.tool_name, Some("Bash".to_string()));
        assert!(context.tool_input.is_some());
    }

    #[test]
    fn test_hook_context_matcher_context() {
        let context = HookContext::for_tool(
            "session-123",
            "/tmp/transcript.json",
            "/project",
            "Bash",
            serde_json::json!({}),
        );

        let ctx = context.matcher_context();
        assert_eq!(ctx.target, "Bash");
        assert!(ctx.context.is_none());
    }

    #[test]
    fn test_hook_context_matcher_context_with_context() {
        let context = HookContext::for_tool(
            "session-123",
            "/tmp/transcript.json",
            "/project",
            "Bash",
            serde_json::json!({}),
        );

        let ctx = context.matcher_context_with_context("git commit -m 'test'");
        assert_eq!(ctx.target, "Bash");
        assert_eq!(ctx.context, Some("git commit -m 'test'"));
    }

    #[test]
    fn test_get_matching_returns_all_for_wildcard() {
        let mut config = HooksConfig::default();
        config.events.insert(
            "pre_tool_use".to_string(),
            HookHandlerConfig {
                command: "test_command".to_string(),
                ..Default::default()
            },
        );

        let registry = HookRegistry::from_config(config);
        let context = HookContext::for_tool(
            "session-123",
            "/tmp/transcript.json",
            "/project",
            "Bash",
            serde_json::json!({}),
        );

        // Should return 1 handler (matches all since no matcher)
        let matching = registry.get_matching(&HookEvent::PreToolUse, &context);
        assert_eq!(matching.len(), 1);
    }

    #[test]
    fn test_get_matching_filters_by_event() {
        let mut config = HooksConfig::default();
        config.events.insert(
            "post_tool_use".to_string(),
            HookHandlerConfig {
                command: "post_handler".to_string(),
                ..Default::default()
            },
        );

        let registry = HookRegistry::from_config(config);
        let context = HookContext::for_tool(
            "session-123",
            "/tmp/transcript.json",
            "/project",
            "Bash",
            serde_json::json!({}),
        );

        // Should return empty for pre_tool_use (only post_tool_use configured)
        let matching = registry.get_matching(&HookEvent::PreToolUse, &context);
        assert!(matching.is_empty());

        // Should return 1 for post_tool_use
        let matching = registry.get_matching(&HookEvent::PostToolUse, &context);
        assert_eq!(matching.len(), 1);
    }
}
