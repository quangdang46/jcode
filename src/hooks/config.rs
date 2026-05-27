//! Hooks configuration loading - multi-layer config support
//!
//! Loads hooks.toml from two layers:
//! 1. User level: `~/.jcode/hooks.toml`
//! 2. Project level: `.jcode/hooks.toml` (current working directory)
//!
//! Project-level hooks override user-level hooks for the same event.

use crate::hooks::matcher::HookMatcher;
use crate::storage::jcode_dir;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Directory name for hooks config (relative to jcode home)
pub const HOOKS_CONFIG_DIR: &str = ".jcode";
/// Filename for hooks configuration
pub const HOOKS_CONFIG_FILENAME: &str = "hooks.toml";

/// Hook event types that can be triggered
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HookEvent {
    PreToolUse,
    PostToolUse,
    PreSession,
    PostSession,
    Error,
    SessionStart,
    SessionEnd,
    PermissionRequest,
    PermissionDenied,
    ToolError,
    Custom(String),
}

impl HookEvent {
    /// Parse a hook event from string
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "pretooluse" | "pre_tool_use" => Some(HookEvent::PreToolUse),
            "posttooluse" | "post_tool_use" => Some(HookEvent::PostToolUse),
            "presession" | "pre_session" => Some(HookEvent::PreSession),
            "postsession" | "post_session" => Some(HookEvent::PostSession),
            "error" => Some(HookEvent::Error),
            "sessionstart" | "session_start" => Some(HookEvent::SessionStart),
            "sessionend" | "session_end" => Some(HookEvent::SessionEnd),
            "permissionrequest" | "permission_request" => Some(HookEvent::PermissionRequest),
            "permissiondenied" | "permission_denied" => Some(HookEvent::PermissionDenied),
            "toolerror" | "tool_error" => Some(HookEvent::ToolError),
            s if s.starts_with("custom:") => Some(HookEvent::Custom(s[7..].to_string())),
            s if s.starts_with("custom") => Some(HookEvent::Custom(s.to_string())),
            _ => None,
        }
    }
}

/// Handler configuration for a single hook
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum HookHandlerConfig {
    Command(CommandHandlerConfig),
    Http(HttpHandlerConfig),
}

impl Default for HookHandlerConfig {
    fn default() -> Self {
        HookHandlerConfig::Command(CommandHandlerConfig::default())
    }
}

/// Command handler configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CommandHandlerConfig {
    pub enabled: bool,
    pub command: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    pub cwd: Option<String>,
    pub timeout_secs: Option<u64>,
    pub pass_input_via_stdin: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matcher: Option<HookMatcher>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub if_: Option<String>,
}

impl Default for CommandHandlerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            command: String::new(),
            args: Vec::new(),
            env: BTreeMap::new(),
            cwd: None,
            timeout_secs: None,
            pass_input_via_stdin: true,
            matcher: None,
            if_: None,
        }
    }
}

/// HTTP handler configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HttpHandlerConfig {
    pub enabled: bool,
    pub url: String,
    pub method: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub headers: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
    pub timeout_secs: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matcher: Option<HookMatcher>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub if_: Option<String>,
}

impl Default for HttpHandlerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            url: String::new(),
            method: "GET".to_string(),
            headers: BTreeMap::new(),
            body: None,
            timeout_secs: Some(30),
            matcher: None,
            if_: None,
        }
    }
}

/// Hooks configuration containing mappings of events to their handlers
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct HooksConfig {
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub events: BTreeMap<String, HookHandlerConfig>,
}

impl HooksConfig {
    /// Merge another hooks config into this one (shallow merge by event).
    /// Values from `other` override values from `self`.
    pub fn merge(&mut self, other: HooksConfig) {
        for (event_name, handler) in other.events.into_iter() {
            self.events.insert(event_name, handler);
        }
    }
}

/// Get the user-level hooks config path (`~/.jcode/hooks.toml`)
fn user_hooks_config_path() -> Option<PathBuf> {
    jcode_dir().ok().map(|d| d.join(HOOKS_CONFIG_FILENAME))
}

/// Get the project-level hooks config path (`.jcode/hooks.toml` in current dir)
fn project_hooks_config_path() -> Option<PathBuf> {
    std::env::current_dir()
        .ok()
        .map(|d| d.join(HOOKS_CONFIG_DIR).join(HOOKS_CONFIG_FILENAME))
}

/// Load a hooks config from a file path, returning None if file doesn't exist
fn load_hooks_config_from_path(path: &PathBuf) -> Result<Option<HooksConfig>> {
    if !path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let config = toml::from_str::<HooksConfig>(&content)
        .with_context(|| format!("Failed to parse hooks config from {}", path.display()))?;
    Ok(Some(config))
}

/// Load hooks configuration from multi-layer config.
///
/// Loads from:
/// 1. User level: `~/.jcode/hooks.toml`
/// 2. Project level: `.jcode/hooks.toml` (current directory)
///
/// Project-level hooks override user-level for the same event.
///
/// Returns a merged `HooksConfig`. If no config files are found, returns an empty config.
pub fn load_hooks_config() -> HooksConfig {
    let mut merged = HooksConfig::default();

    if let Some(path) = user_hooks_config_path() {
        match load_hooks_config_from_path(&path) {
            Ok(Some(config)) => {
                merged.merge(config);
            }
            Ok(None) => {}
            Err(e) => {
                crate::logging::warn(&format!(
                    "Failed to load user hooks config from {}: {}",
                    path.display(),
                    e
                ));
            }
        }
    }

    if let Some(path) = project_hooks_config_path() {
        match load_hooks_config_from_path(&path) {
            Ok(Some(config)) => {
                merged.merge(config);
            }
            Ok(None) => {}
            Err(e) => {
                crate::logging::warn(&format!(
                    "Failed to load project hooks config from {}: {}",
                    path.display(),
                    e
                ));
            }
        }
    }

    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_event_parse() {
        assert_eq!(
            HookEvent::parse("pre_tool_use"),
            Some(HookEvent::PreToolUse)
        );
        assert_eq!(HookEvent::parse("PostToolUse"), Some(HookEvent::PreToolUse));
        assert_eq!(HookEvent::parse("pretooluse"), Some(HookEvent::PreToolUse));
        assert_eq!(
            HookEvent::parse("post_session"),
            Some(HookEvent::PostSession)
        );
        assert_eq!(HookEvent::parse("error"), Some(HookEvent::Error));
        assert_eq!(
            HookEvent::parse("custom:my_event"),
            Some(HookEvent::Custom("my_event".to_string()))
        );
        assert_eq!(HookEvent::parse("unknown"), None);
    }

    #[test]
    fn test_hooks_config_merge() {
        let mut config1 = HooksConfig::default();
        config1.events.insert(
            "pre_tool_use".to_string(),
            HookHandlerConfig::Command(CommandHandlerConfig {
                command: "user_handler".to_string(),
                ..Default::default()
            }),
        );

        let mut config2 = HooksConfig::default();
        config2.events.insert(
            "pre_tool_use".to_string(),
            HookHandlerConfig::Command(CommandHandlerConfig {
                command: "project_handler".to_string(),
                ..Default::default()
            }),
        );
        config2.events.insert(
            "post_tool_use".to_string(),
            HookHandlerConfig::Command(CommandHandlerConfig {
                command: "post_handler".to_string(),
                ..Default::default()
            }),
        );

        config1.merge(config2);

        let pre_handler = config1.events.get("pre_tool_use").unwrap();
        let post_handler = config1.events.get("post_tool_use").unwrap();
        assert!(
            matches!(pre_handler, HookHandlerConfig::Command(cmd) if cmd.command == "project_handler"),
            "Project handler should override user handler"
        );
        assert!(
            matches!(post_handler, HookHandlerConfig::Command(cmd) if cmd.command == "post_handler"),
            "New event should be added"
        );
    }

    #[test]
    fn test_default_hooks_config() {
        let config = HooksConfig::default();
        assert!(config.events.is_empty());
    }
}
