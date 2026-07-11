//! Adapter from swarm member status into the inline gallery layout.
//!
//! All presentation logic (status colors, role glyphs, age formatting, header,
//! sorting, layout config) lives in the shared
//! [`jcode_tui_render::swarm_gallery`] module so the live TUI and the
//! `swarm_gallery_live` demo render identically. This adapter only handles
//! turning a [`SwarmMemberStatus`] into a renderer-agnostic
//! [`GalleryMember`] (label + body lines).

use crate::protocol::SwarmMemberStatus;
use jcode_tui_render::swarm_gallery::{
    GalleryMember, SwarmStripHint, display_order, humanize_age, render_gallery,
    render_swarm_compact, render_swarm_dock, render_swarm_panel, render_swarm_strip,
    render_swarm_strip_vertical,
};
use ratatui::prelude::*;

fn member_label(member: &SwarmMemberStatus) -> String {
    member
        .friendly_name
        .clone()
        .unwrap_or_else(|| member.session_id.chars().take(8).collect())
}

/// Session icon (emoji) for a member, derived from its friendly name (session
/// names come from the shared `SESSION_NAMES` word list, e.g. "fox" -> 🦊).
/// Falls back to `None` when the name is unknown so the strip shows the name.
fn member_icon(member: &SwarmMemberStatus) -> Option<String> {
    let name = member.friendly_name.as_deref()?;
    let icon = crate::id::session_icon(name);
    if icon == "💫" {
        // Unknown word: don't show the generic fallback, keep the name.
        None
    } else {
        Some(icon.to_string())
    }
}

/// Age marker appended to member bodies, e.g. "· 7s ago" or "· now".
/// `humanize_age` already yields "now" for fresh updates, which reads wrong
/// with an "ago" suffix.
fn age_marker(age: u64) -> String {
    let human = humanize_age(age);
    if human == "now" {
        "· now".to_string()
    } else {
        format!("· {human} ago")
    }
}

/// Build the body lines shown inside a member's viewport. Prefers live streamed
/// output (the tail) when present; otherwise surfaces the latest detail plus a
/// status-age hint.
fn member_body(member: &SwarmMemberStatus) -> Vec<String> {
    // Live streamed output wins: show the worker's in-progress assistant text.
    if let Some(tail) = member.output_tail.as_ref().filter(|t| !t.trim().is_empty()) {
        let mut body: Vec<String> = tail.lines().map(|l| l.to_string()).collect();
        if let Some(age) = member.status_age_secs {
            body.push(age_marker(age));
        }
        return body;
    }
    let mut body: Vec<String> = Vec::new();
    if let Some(detail) = member.detail.as_ref().filter(|d| !d.trim().is_empty()) {
        body.push(detail.clone());
    }
    if let Some(age) = member.status_age_secs {
        body.push(age_marker(age));
    }
    body
}

/// Convert swarm members into renderer-agnostic gallery members.
pub(crate) fn members_to_gallery(members: &[SwarmMemberStatus]) -> Vec<GalleryMember> {
    members
        .iter()
        .map(|member| GalleryMember {
            label: member_label(member),
            icon: member_icon(member),
            status: member.status.clone(),
            task: member.task_label.clone(),
            role: member.role.clone(),
            body: member_body(member),
            sort_key: member.session_id.clone(),
            todo: member.todo_progress,
            model: member.runtime.model.clone(),
            elapsed_secs: member.runtime.elapsed_secs,
            todo_items: member
                .todo_items
                .iter()
                .map(|t| jcode_tui_render::swarm_gallery::GalleryTodo {
                    content: t.content.clone(),
                    status: t.status.clone(),
                    tool_intents: t
                        .tool_intents
                        .iter()
                        .map(|tool| jcode_tui_render::swarm_gallery::GalleryToolIntent {
                            tool_name: tool.tool_name.clone(),
                            intent: tool.intent.clone(),
                            status: tool.status.clone(),
                            progress: tool.progress.as_ref().map(|progress| {
                                (progress.current, progress.total, progress.unit.clone())
                            }),
                        })
                        .collect(),
                })
                .collect(),
        })
        .collect()
}

/// Render expanded member cards for insertion directly beneath a swarm tool
/// call in the transcript.
pub(crate) fn render_swarm_chat_card_lines(
    members: &[SwarmMemberStatus],
    spinner_frame: usize,
    width: usize,
) -> Vec<Line<'static>> {
    jcode_tui_render::swarm_gallery::render_swarm_chat_cards(
        &members_to_gallery(members),
        spinner_frame,
        width,
    )
}

/// Render the inline swarm gallery for the given members into `area`-width lines.
#[allow(dead_code)]
pub(crate) fn render_swarm_gallery_lines(
    members: &[SwarmMemberStatus],
    width: usize,
    max_height: usize,
) -> Vec<Line<'static>> {
    if members.is_empty() {
        return Vec::new();
    }
    render_gallery(&members_to_gallery(members), width, max_height)
}

/// Render the list+detail swarm panel: a compact list of managed agents plus a
/// detail viewport for the `selected` one. `focused` adds an interaction hint.
#[allow(dead_code)]
pub(crate) fn render_swarm_panel_lines(
    members: &[SwarmMemberStatus],
    selected: usize,
    focused: bool,
    width: usize,
    max_height: usize,
) -> Vec<Line<'static>> {
    if members.is_empty() {
        return Vec::new();
    }
    render_swarm_panel(
        &members_to_gallery(members),
        selected,
        focused,
        width,
        max_height,
    )
}

/// Render the compact swarm strip shown directly above the status line.
///
/// The layout follows `agents.swarm_strip_layout`: `vertical` (default) lists
/// one agent per row (session icon + task, capped to a few rows), while
/// `horizontal` packs all agents as chips on a single row.
///
/// `focus_key` is the configured chord to enter the controls (e.g. "ctrl+t"),
/// used both for the unfocused enter-hint and as the first focused hint.
/// `spinner_frame` animates active agents' glyphs. `max_height` bounds the
/// focused strip (chips + expanded hovered-agent detail + hints).
pub(crate) fn render_swarm_strip_lines(
    members: &[SwarmMemberStatus],
    selected: usize,
    focused: bool,
    focus_key: &str,
    spinner_frame: usize,
    width: usize,
    max_height: usize,
) -> Vec<Line<'static>> {
    if members.is_empty() {
        return Vec::new();
    }
    let enter_hint = format!("{focus_key} controls");
    // Focused hints: only Alt-chords (plus esc) are claimed so plain typing
    // keeps flowing to the chat input while the panel is focused.
    let hints = vec![
        SwarmStripHint {
            key: focus_key.to_string(),
            label: "next".into(),
        },
        SwarmStripHint {
            key: "alt+↑/↓".into(),
            label: "select".into(),
        },
        SwarmStripHint {
            key: "alt+o".into(),
            label: "open".into(),
        },
        SwarmStripHint {
            key: "esc".into(),
            label: "exit".into(),
        },
    ];
    let gallery = members_to_gallery(members);
    let enter = if focused {
        None
    } else {
        Some(enter_hint.as_str())
    };
    match crate::config::config().agents.swarm_strip_layout {
        crate::config::SwarmStripLayout::Vertical => render_swarm_strip_vertical(
            &gallery,
            selected,
            focused,
            &hints,
            enter,
            spinner_frame,
            width,
            SWARM_STRIP_VERTICAL_MAX_ROWS,
            max_height,
        ),
        crate::config::SwarmStripLayout::Horizontal => render_swarm_strip(
            &gallery,
            selected,
            focused,
            &hints,
            enter,
            spinner_frame,
            width,
            max_height,
        ),
    }
}

/// Row cap for the vertical strip: agents beyond this collapse into a
/// `+N more` line (the cap includes that overflow row).
const SWARM_STRIP_VERTICAL_MAX_ROWS: usize = 4;

/// Render the compact swarm widget body: at most two lines, an agents/nodes
/// summary plus a green/yellow/empty plan progress bar. `plan` is the
/// coordinator's task-graph progress as (done, running, total).
pub(crate) fn render_swarm_compact_lines(
    members: &[SwarmMemberStatus],
    plan: Option<(u32, u32, u32)>,
    width: usize,
    max_height: usize,
) -> Vec<Line<'static>> {
    if members.is_empty() {
        return Vec::new();
    }
    render_swarm_compact(&members_to_gallery(members), plan, width, max_height)
}

/// Render the swarm dock widget body: a narrow vertical agent list for the
/// info-widget margins. `plan` is the coordinator's swarm plan progress
/// (completed, total), shown in the header when present.
#[allow(dead_code)]
pub(crate) fn render_swarm_dock_lines(
    members: &[SwarmMemberStatus],
    selected: usize,
    focused: bool,
    plan: Option<(u32, u32)>,
    spinner_frame: usize,
    width: usize,
    max_height: usize,
) -> Vec<Line<'static>> {
    if members.is_empty() {
        return Vec::new();
    }
    render_swarm_dock(
        &members_to_gallery(members),
        selected,
        focused,
        plan,
        spinner_frame,
        width,
        max_height,
    )
}

/// Session ids of `members` in the same order the panel/gallery displays them.
///
/// Delegates to the renderer's [`display_order`] on the exact same
/// [`GalleryMember`] conversion used for rendering, so the pop-out index can
/// never drift from what is on screen.
pub(crate) fn members_display_order(members: &[SwarmMemberStatus]) -> Vec<String> {
    display_order(&members_to_gallery(members))
        .into_iter()
        .map(|i| members[i].session_id.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use jcode_tui_render::swarm_gallery::members_to_tiles;

    fn member(
        id: &str,
        status: &str,
        detail: Option<&str>,
        role: Option<&str>,
    ) -> SwarmMemberStatus {
        SwarmMemberStatus {
            session_id: id.to_string(),
            friendly_name: Some(id.to_string()),
            status: status.to_string(),
            detail: detail.map(str::to_string),
            task_label: None,
            role: role.map(str::to_string),
            is_headless: Some(true),
            live_attachments: None,
            status_age_secs: Some(3),
            output_tail: None,
            report_back_to_session_id: None,
            todo_progress: None,
            todo_items: Vec::new(),
            runtime: crate::protocol::SwarmMemberRuntime::default(),
        }
    }

    #[test]
    fn coordinator_sorts_first() {
        let members = vec![
            member("zeta", "running", None, None),
            member("alpha", "running", None, Some("coordinator")),
        ];
        let tiles = members_to_tiles(&members_to_gallery(&members));
        assert_eq!(tiles[0].title, "alpha");
        assert_eq!(tiles[0].role_glyph.as_deref(), Some("★"));
    }

    #[test]
    fn renders_header_and_boxes() {
        let members = vec![
            member("alpha", "running", Some("editing config.rs"), None),
            member("beta", "done", Some("reviewed"), None),
        ];
        let lines = render_swarm_gallery_lines(&members, 80, 12);
        assert!(!lines.is_empty());
        let header: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(header.contains("swarm · 2 agents"), "got: {header}");
        for line in &lines {
            assert!(line.width() <= 80);
        }
    }

    #[test]
    fn empty_members_render_nothing() {
        assert!(render_swarm_gallery_lines(&[], 80, 12).is_empty());
    }

    #[test]
    fn output_tail_takes_priority_over_detail() {
        let mut m = member("alpha", "running", Some("the detail line"), None);
        m.output_tail = Some("line one\nline two".to_string());
        let body = member_body(&m);
        assert_eq!(body[0], "line one");
        assert_eq!(body[1], "line two");
        assert!(!body.iter().any(|l| l.contains("the detail line")));
    }
}
