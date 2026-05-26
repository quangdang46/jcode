//! Hook execution - runs hooks and returns results

use crate::hooks::config::HookHandlerConfig;
use crate::hooks::types::{HookInput, HookOutput};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::time::timeout;

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

/// Execute a command hook
pub async fn execute_command_hook(
    config: &HookHandlerConfig,
    input: &HookInput,
) -> Result<HookResult, String> {
    // Serialize input to JSON
    let input_json = serde_json::to_string(input)
        .map_err(|e| format!("Failed to serialize hook input: {}", e))?;

    // Build command
    let mut cmd = if cfg!(windows) {
        let mut c = Command::new("powershell");
        c.args(["-NoProfile", "-Command", &config.command]);
        c
    } else {
        let mut c = Command::new("bash");
        c.args(["-c", &config.command]);
        c
    };

    // Set timeout if specified
    let timeout_duration = config
        .timeout_secs
        .map(|s| std::time::Duration::from_secs(s))
        .unwrap_or(std::time::Duration::from_secs(30));

    // Add env vars
    for (k, v) in &config.env {
        cmd.env(k, v);
    }

    // Set cwd if specified
    if let Some(cwd) = &config.cwd {
        cmd.current_dir(cwd);
    }

    // Execute with timeout
    let result = timeout(
        timeout_duration,
        async {
            // Spawn with piped stdin/stdout
            let mut child = cmd.stdin(std::process::Stdio::piped()).unwrap();
            let mut stdout = Vec::new();

            // Write input to stdin
            {
                let stdin = child.stdin.as_mut().unwrap();
                stdin
                    .write_all(input_json.as_bytes())
                    .await
                    .map_err(|e| e.to_string())?;
            }

            // Read stdout
            child
                .stdout
                .as_mut()
                .unwrap()
                .read_to_end(&mut stdout)
                .await
                .map_err(|e| e.to_string())?;

            // Wait for process
            let status = child.wait().await.map_err(|e| e.to_string())?;

            // Parse output
            let output_str = String::from_utf8_lossy(&stdout);
            let hook_output: HookOutput = serde_json::from_str(&output_str)
                .unwrap_or_else(|_| HookOutput::continue_());

            Ok::<_, String>((status, hook_output))
        },
    )
    .await;

    match result {
        Ok(Ok((status, output))) => {
            if status.success() {
                Ok(HookResult::Continue(output))
            } else if status.code() == Some(2) {
                let reason = output.reason.clone().unwrap_or_default();
                Ok(HookResult::Blocked { reason, output })
            } else {
                Ok(HookResult::Failed {
                    error: format!("Hook exited with code {:?}", status.code()),
                })
            }
        }
        Ok(Err(e)) => Ok(HookResult::Failed { error: e }),
        Err(_) => Ok(HookResult::Failed {
            error: "Hook execution timed out".to_string(),
        }),
    }
}

/// Dispatch to appropriate handler type
pub async fn execute_hook(
    config: &HookHandlerConfig,
    input: &HookInput,
) -> Result<HookResult, String> {
    // Command is the only implemented handler for now
    execute_command_hook(config, input).await
}