//! Hook execution - runs hooks and returns results

use std::collections::HashMap;

use crate::hooks::config::{CommandHandlerConfig, HookHandlerConfig};
use crate::hooks::types::{HookInput, HookOutput};
use reqwest::Client;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::time::timeout;

/// Result of executing a hook
#[derive(Debug)]
pub enum HookResult {
    Continue(HookOutput),
    Blocked { reason: String, output: HookOutput },
    Failed { error: String },
}

/// Execute a command hook
pub async fn execute_command_hook(
    config: &CommandHandlerConfig,
    input: &HookInput,
) -> Result<HookResult, String> {
    let input_json =
        serde_json::to_string(input).map_err(|e| format!("Failed to serialize hook input: {}", e))?;

    let mut cmd = if cfg!(windows) {
        let mut c = Command::new("powershell");
        c.args(["-NoProfile", "-Command", &config.command]);
        c
    } else {
        let mut c = Command::new("bash");
        c.args(["-c", &config.command]);
        c
    };

    let timeout_duration = config
        .timeout_secs
        .map(|s| std::time::Duration::from_secs(s))
        .unwrap_or(std::time::Duration::from_secs(30));

    for (k, v) in &config.env {
        cmd.env(k, v);
    }

    if let Some(cwd) = &config.cwd {
        cmd.current_dir(cwd);
    }

    let result = timeout(
        timeout_duration,
        async {
            cmd.stdin(std::process::Stdio::piped());
            cmd.stdout(std::process::Stdio::piped());

            let mut child = cmd
                .spawn()
                .map_err(|e| format!("Failed to spawn hook process: {}", e))?;

            let mut stdout = Vec::new();

            if let Some(ref mut stdin) = child.stdin {
                stdin
                    .write_all(input_json.as_bytes())
                    .await
                    .map_err(|e| e.to_string())?;
            }

            if let Some(ref mut stdout_handle) = child.stdout {
                stdout_handle
                    .read_to_end(&mut stdout)
                    .await
                    .map_err(|e| e.to_string())?;
            }

            let status = child.wait().await.map_err(|e| e.to_string())?;

            let output_str = String::from_utf8_lossy(&stdout);
            let hook_output: HookOutput =
                serde_json::from_str(&output_str).unwrap_or_else(|_| HookOutput::continue_());

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

/// Execute an HTTP hook
pub async fn execute_http_hook(
    url: &str,
    method: &str,
    headers: &HashMap<String, String>,
    body: &serde_json::Value,
    timeout_secs: u64,
) -> Result<HookResult, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let mut request = match method.to_uppercase().as_str() {
        "GET" => client.get(url),
        "POST" => client.post(url),
        "PUT" => client.put(url),
        "DELETE" => client.delete(url),
        "PATCH" => client.patch(url),
        "HEAD" => client.head(url),
        "OPTIONS" => client.request(reqwest::Method::OPTIONS, url),
        _ => {
            return Ok(HookResult::Failed {
                error: format!("Unsupported HTTP method: {}", method),
            })
        }
    };

    for (k, v) in headers {
        request = request.header(k, v);
    }

    request = request.json(body);

    let response = timeout(std::time::Duration::from_secs(timeout_secs), async {
        request.send().await
    })
    .await;

    match response {
        Ok(Ok(resp)) => {
            let status = resp.status();
            if status.is_success() {
                let hook_output: HookOutput = resp
                    .json()
                    .await
                    .unwrap_or_else(|_| HookOutput::continue_());
                Ok(HookResult::Continue(hook_output))
            } else if status.is_client_error() || status.is_server_error() {
                Ok(HookResult::Failed {
                    error: format!("HTTP {} error: {}", status.as_u16(), status.canonical_reason().unwrap_or("Unknown")),
                })
            } else {
                Ok(HookResult::Failed {
                    error: format!("HTTP response: {}", status),
                })
            }
        }
        Ok(Err(e)) => Ok(HookResult::Failed {
            error: format!("HTTP request failed: {}", e),
        }),
        Err(_) => Ok(HookResult::Failed {
            error: "HTTP request timed out".to_string(),
        }),
    }
}

/// Dispatch to appropriate handler type
pub async fn execute_hook(
    config: &HookHandlerConfig,
    input: &HookInput,
) -> Result<HookResult, String> {
    match config {
        HookHandlerConfig::Command(cmd_config) => execute_command_hook(cmd_config, input).await,
        HookHandlerConfig::Http(http_config) => {
            let headers: HashMap<String, String> = http_config
                .headers
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            let body = http_config.body.as_ref().unwrap_or(&serde_json::Value::Null);
            let timeout_secs = http_config.timeout_secs.unwrap_or(30);
            execute_http_hook(&http_config.url, &http_config.method, &headers, body, timeout_secs).await
        }
    }
}