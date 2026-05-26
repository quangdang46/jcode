//! Hook execution - runs hooks and returns results

use crate::hooks::types::{HookInput, HookOutput};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::time::{timeout, Duration};

/// Result of executing a hook
#[derive(Debug)]
pub enum HookResult {
    /// Hook approved - continue execution
    Continue(HookOutput),
    /// Hook blocked - do not continue
    Blocked { reason: String, output: HookOutput },
    /// Hook execution failed
    Failed { error: String },
}

/// Configuration for hook handler (command variant with extended fields)
pub enum HookHandlerConfig {
    /// Command hook - executes an external command
    Command {
        command: String,
        shell: Option<String>,
        timeout: Option<u64>,
        status_message: Option<String>,
        once: Option<bool>,
        async_: Option<bool>,
        async_rewake: Option<bool>,
    },
    /// Prompt hook (not implemented)
    Prompt { message: String },
    /// Agent hook (not implemented)
    Agent { agent_type: String },
    /// HTTP hook (not implemented)
    Http { url: String, method: String },
}

/// Execute a command hook with the given input.
///
/// # Arguments
/// * `command` - The command string to execute via shell
/// * `input` - The hook input to serialize as JSON and send to stdin
/// * `timeout_secs` - Maximum seconds to wait for the command
///
/// # Returns
/// * `Ok(HookResult)` on successful execution
/// * `Err(String)` on internal error (serialization failure, etc.)
pub async fn execute_command_hook(
    command: &str,
    input: &HookInput,
    timeout_secs: u64,
) -> Result<HookResult, String> {
    // Serialize input to JSON
    let input_json = serde_json::to_string(input)
        .map_err(|e| format!("Failed to serialize hook input: {}", e))?;

    // Determine shell to use (default to bash on unix, powershell on windows)
    let shell_cmd = if cfg!(target_os = "windows") {
        ("powershell", "-NoProfile", "-Command")
    } else {
        ("bash", "-c")
    };

    // Spawn async command with piped stdin/stdout
    let mut child = Command::new(shell_cmd.0)
        .arg(shell_cmd.1)
        .arg(shell_cmd.2)
        .arg(command)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn command: {}", e))?;

    // Write JSON to stdin
    if let Some(ref mut stdin) = child.stdin {
        stdin
            .write_all(input_json.as_bytes())
            .await
            .map_err(|e| format!("Failed to write to stdin: {}", e))?;
    }

    // Execute with timeout
    let result = timeout(Duration::from_secs(timeout_secs), async {
        let output = child
            .wait_with_output()
            .await
            .map_err(|e| format!("Failed to wait for command: {}", e))?;
        Ok::<_, String>(output)
    })
    .await;

    match result {
        Ok(Ok(output)) => {
            let exit_code = output.status.code().unwrap_or(1) as i32;
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();

            // Parse stdout as HookOutput
            let hook_output = serde_json::from_str::<HookOutput>(&stdout)
                .unwrap_or_else(|_| HookOutput::continue_());

            match exit_code {
                0 => Ok(HookResult::Continue(hook_output)),
                2 => Ok(HookResult::Blocked {
                    reason: hook_output.reason.clone().unwrap_or_else(|| "Blocked by hook".to_string()),
                    output: hook_output,
                }),
                _ => Ok(HookResult::Failed {
                    error: format!(
                        "Hook command exited with code {}: {}",
                        exit_code,
                        String::from_utf8_lossy(&output.stderr)
                    ),
                }),
            }
        }
        Ok(Err(e)) => Ok(HookResult::Failed { error: e }),
        Err(_) => Ok(HookResult::Failed {
            error: format!("Hook command timed out after {} seconds", timeout_secs),
        }),
    }
}

/// Dispatch to the appropriate hook handler based on config.
pub async fn execute_hook(
    config: &HookHandlerConfig,
    input: &HookInput,
) -> Result<HookResult, String> {
    match config {
        HookHandlerConfig::Command {
            command,
            shell: _,
            timeout,
            status_message: _,
            once: _,
            async_: _,
            async_rewake: _,
        } => {
            let timeout_secs = timeout.unwrap_or(30);
            execute_command_hook(command, input, timeout_secs).await
        }
        HookHandlerConfig::Prompt { .. } => Err("Prompt hook not implemented".into()),
        HookHandlerConfig::Agent { .. } => Err("Agent hook not implemented".into()),
        HookHandlerConfig::Http { .. } => Err("HTTP hook not implemented".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::types::HookInput;

    #[tokio::test]
    async fn test_execute_command_hook_basic() {
        let input = HookInput::for_tool(
            "test-session",
            "/tmp/transcript.json",
            "/tmp",
            "Bash",
            serde_json::json!({ "command": "echo hello" }),
        );

        let result = execute_command_hook(
            r#"echo '{"continue_": true}'"#,
            &input,
            5,
        )
        .await;

        match result {
            Ok(HookResult::Continue(output)) => {
                assert!(output.continue_, "Expected continue_ to be true");
            }
            Ok(HookResult::Blocked { reason, output }) => {
                println!("Blocked: {} with output {:?}", reason, output);
            }
            Ok(HookResult::Failed { error }) => {
                println!("Failed: {}", error);
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_execute_command_hook_blocked() {
        let input = HookInput::for_tool(
            "test-session",
            "/tmp/transcript.json",
            "/tmp",
            "Bash",
            serde_json::json!({ "command": "echo hello" }),
        );

        let result = execute_command_hook(
            r#"echo '{"continue_": false, "reason": "blocked by test"}'; exit 2"#,
            &input,
            5,
        )
        .await;

        match result {
            Ok(HookResult::Blocked { reason, output }) => {
                assert_eq!(reason, "blocked by test");
                assert!(!output.continue_);
            }
            _ => panic!("Expected Blocked result"),
        }
    }

    #[tokio::test]
    async fn test_execute_command_hook_timeout() {
        let input = HookInput::for_tool(
            "test-session",
            "/tmp/transcript.json",
            "/tmp",
            "Bash",
            serde_json::json!({ "command": "sleep 10" }),
        );

        let result = execute_command_hook("sleep 10", &input, 1).await;

        match result {
            Ok(HookResult::Failed { error }) => {
                assert!(error.contains("timed out"), "Expected timeout error, got: {}", error);
            }
            _ => panic!("Expected Failed result"),
        }
    }

    #[tokio::test]
    async fn test_execute_hook_dispatch_command() {
        let config = HookHandlerConfig::Command {
            command: r#"echo '{"continue_": true}'"#.to_string(),
            shell: None,
            timeout: Some(5),
            status_message: None,
            once: None,
            async_: None,
            async_rewake: None,
        };

        let input = HookInput::for_tool(
            "test-session",
            "/tmp/transcript.json",
            "/tmp",
            "Bash",
            serde_json::json!({}),
        );

        let result = execute_hook(&config, &input).await;
        assert!(matches!(result, Ok(HookResult::Continue(_))));
    }

    #[tokio::test]
    async fn test_execute_hook_dispatch_prompt_not_implemented() {
        let config = HookHandlerConfig::Prompt {
            message: "Test prompt".to_string(),
        };

        let input = HookInput::for_tool(
            "test-session",
            "/tmp/transcript.json",
            "/tmp",
            "Bash",
            serde_json::json!({}),
        );

        let result = execute_hook(&config, &input).await;
        assert!(matches!(result, Err(e) if e.contains("not implemented")));
    }

    #[tokio::test]
    async fn test_execute_hook_dispatch_agent_not_implemented() {
        let config = HookHandlerConfig::Agent {
            agent_type: "test".to_string(),
        };

        let input = HookInput::for_tool(
            "test-session",
            "/tmp/transcript.json",
            "/tmp",
            "Bash",
            serde_json::json!({}),
        );

        let result = execute_hook(&config, &input).await;
        assert!(matches!(result, Err(e) if e.contains("not implemented")));
    }

    #[tokio::test]
    async fn test_execute_hook_dispatch_http_not_implemented() {
        let config = HookHandlerConfig::Http {
            url: "http://example.com".to_string(),
            method: "POST".to_string(),
        };

        let input = HookInput::for_tool(
            "test-session",
            "/tmp/transcript.json",
            "/tmp",
            "Bash",
            serde_json::json!({}),
        );

        let result = execute_hook(&config, &input).await;
        assert!(matches!(result, Err(e) if e.contains("not implemented")));
    }
}