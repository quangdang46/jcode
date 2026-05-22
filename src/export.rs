//! Session export to Markdown / JSON (subset of issue #10).
//!
//! This MVP covers the markdown half of the export feature: render an entire
//! session as one self-contained Markdown document with messages, tool calls,
//! tool outputs, and reasoning blocks. Suitable for pasting into PRs / bug
//! reports / docs.
//!
//! Out of scope for this MVP, tracked as follow-ups under issue #10:
//!   - HTML output with inline CSS / SVG mermaid / base64 images
//!   - Redaction (`--redact`) of API keys, bearer tokens, well-known env vars
//!   - The `/export` slash command (this PR is CLI-only via
//!     `jcode export <session> [output]`)

use anyhow::{Context, Result};
use std::path::PathBuf;

use jcode_message_types::Role;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Markdown,
    Json,
}

pub fn run(session_ref: &str, output: Option<PathBuf>, format: ExportFormat) -> Result<()> {
    let session_id = crate::session::find_session_by_name_or_id(session_ref)?;
    let session = crate::session::Session::load(&session_id)?;

    let body = match format {
        ExportFormat::Markdown => render_markdown(&session),
        ExportFormat::Json => {
            serde_json::to_string_pretty(&session).context("failed to serialize session to JSON")?
        }
    };

    let output_path = match output {
        Some(p) => p,
        None => default_output_path(&session, format),
    };

    if let Some(parent) = output_path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create parent dir {}", parent.display()))?;
    }
    std::fs::write(&output_path, body.as_bytes())
        .with_context(|| format!("failed to write {}", output_path.display()))?;

    let absolute = output_path
        .canonicalize()
        .unwrap_or_else(|_| output_path.clone());
    println!("{}", absolute.display());
    Ok(())
}

fn default_output_path(session: &crate::session::Session, format: ExportFormat) -> PathBuf {
    let stem = session
        .display_title()
        .map(slugify)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            session
                .short_name
                .as_deref()
                .map(slugify)
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| session.id.clone())
        });
    let ts = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let ext = match format {
        ExportFormat::Markdown => "md",
        ExportFormat::Json => "json",
    };
    PathBuf::from(format!("{stem}-{ts}.{ext}"))
}

fn slugify(input: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    out.trim_end_matches('-').to_string()
}

/// Render a session as a self-contained Markdown document.
pub fn render_markdown(session: &crate::session::Session) -> String {
    let mut out = String::new();
    let title = session
        .display_title()
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| session.display_name().to_string());
    out.push_str(&format!("# {title}\n\n"));
    out.push_str(&format!("- **Session ID**: `{}`\n", session.id));
    if let Some(name) = &session.short_name {
        out.push_str(&format!("- **Name**: `{}`\n", name));
    }
    if let Some(provider) = &session.provider_key {
        out.push_str(&format!("- **Provider**: `{}`\n", provider));
    }
    if let Some(model) = &session.model {
        out.push_str(&format!("- **Model**: `{}`\n", model));
    }
    out.push_str(&format!(
        "- **Created**: {}\n",
        session.created_at.to_rfc3339()
    ));
    out.push_str(&format!(
        "- **Updated**: {}\n",
        session.updated_at.to_rfc3339()
    ));
    out.push_str(&format!("- **Messages**: {}\n\n", session.messages.len()));

    if let Some(compaction) = session.compaction.as_ref() {
        let kind = if compaction.openai_encrypted_content.is_some() {
            "native/openai-encrypted"
        } else if !compaction.summary_text.is_empty() {
            "summary-text"
        } else {
            "none"
        };
        out.push_str(&format!(
            "> Active compaction artifact present (`{}` — {} chars).\n\n",
            kind,
            artifact_chars(compaction)
        ));
    }

    out.push_str("---\n\n");

    for (idx, msg) in session.messages.iter().enumerate() {
        render_stored_message(&mut out, idx, msg);
    }

    out
}

fn artifact_chars(compaction: &jcode_session_types::StoredCompactionState) -> usize {
    compaction
        .openai_encrypted_content
        .as_ref()
        .map(|s| s.len())
        .unwrap_or_else(|| compaction.summary_text.len())
}

fn render_stored_message(out: &mut String, idx: usize, msg: &crate::session::StoredMessage) {
    let role_label = match msg.role {
        Role::User => "User",
        Role::Assistant => "Assistant",
    };
    let timestamp = msg
        .timestamp
        .map(|t| format!(" · {}", t.format("%Y-%m-%d %H:%M:%S")))
        .unwrap_or_default();
    out.push_str(&format!("## #{idx} {role_label}{timestamp}\n\n"));

    use jcode_message_types::ContentBlock;
    for block in &msg.content {
        match block {
            ContentBlock::Text { text, .. } => {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    out.push_str(trimmed);
                    out.push_str("\n\n");
                }
            }
            ContentBlock::Reasoning { text } => {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    out.push_str("<details><summary>thinking</summary>\n\n");
                    out.push_str(trimmed);
                    out.push_str("\n\n</details>\n\n");
                }
            }
            ContentBlock::ToolUse { name, input, .. } => {
                let pretty =
                    serde_json::to_string_pretty(input).unwrap_or_else(|_| input.to_string());
                out.push_str(&format!(
                    "<details><summary>tool: <code>{name}</code></summary>\n\n```json\n{pretty}\n```\n\n</details>\n\n"
                ));
            }
            ContentBlock::ToolResult { content, .. } => {
                let trimmed = content.trim();
                if !trimmed.is_empty() {
                    out.push_str("<details><summary>tool result</summary>\n\n```\n");
                    out.push_str(trimmed);
                    out.push_str("\n```\n\n</details>\n\n");
                }
            }
            ContentBlock::Image { media_type, .. } => {
                out.push_str(&format!("_[image: {media_type}]_\n\n"));
            }
            ContentBlock::OpenAICompaction { encrypted_content } => {
                out.push_str(&format!(
                    "_[OpenAI native compaction artifact: {} chars]_\n\n",
                    encrypted_content.len()
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jcode_message_types::{ContentBlock, Role};

    fn fake_session() -> crate::session::Session {
        let mut s = crate::session::Session::create_with_id(
            "session_test_export".to_string(),
            None,
            Some("Test Export".to_string()),
        );
        s.model = Some("gpt-5.5".to_string());
        s.messages.push(crate::session::StoredMessage {
            id: "m1".to_string(),
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: "hello jcode".to_string(),
                cache_control: None,
            }],
            display_role: None,
            timestamp: None,
            tool_duration_ms: None,
            token_usage: None,
        });
        s.messages.push(crate::session::StoredMessage {
            id: "m2".to_string(),
            role: Role::Assistant,
            content: vec![
                ContentBlock::Reasoning {
                    text: "thinking step 1".to_string(),
                },
                ContentBlock::ToolUse {
                    id: "t1".to_string(),
                    name: "read".to_string(),
                    input: serde_json::json!({"file": "src/main.rs"}),
                },
                ContentBlock::ToolResult {
                    tool_use_id: "t1".to_string(),
                    content: "fn main() {}".to_string(),
                    is_error: Some(false),
                },
                ContentBlock::Text {
                    text: "Done.".to_string(),
                    cache_control: None,
                },
            ],
            display_role: None,
            timestamp: None,
            tool_duration_ms: None,
            token_usage: None,
        });
        s
    }

    #[test]
    fn markdown_includes_title_metadata_and_messages() {
        let s = fake_session();
        let md = render_markdown(&s);
        assert!(md.starts_with("# Test Export\n"));
        assert!(md.contains("- **Session ID**: `session_test_export`"));
        assert!(md.contains("- **Model**: `gpt-5.5`"));
        assert!(md.contains("- **Messages**: 2"));
        assert!(md.contains("## #0 User"));
        assert!(md.contains("hello jcode"));
        assert!(md.contains("## #1 Assistant"));
        assert!(md.contains("Done."));
    }

    #[test]
    fn markdown_collapses_thinking_and_tools_in_details() {
        let s = fake_session();
        let md = render_markdown(&s);
        assert!(md.contains("<details><summary>thinking</summary>"));
        assert!(md.contains("thinking step 1"));
        assert!(md.contains("<details><summary>tool: <code>read</code></summary>"));
        assert!(md.contains("\"file\""));
        assert!(md.contains("<details><summary>tool result</summary>"));
        assert!(md.contains("fn main() {}"));
    }

    #[test]
    fn slugify_keeps_alpha_drops_punct() {
        assert_eq!(slugify("Test Export!"), "test-export");
        assert_eq!(slugify("a/b c"), "a-b-c");
        assert_eq!(slugify("___"), "");
        assert_eq!(slugify(""), "");
    }
}
