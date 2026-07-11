//! Session-local todo persistence (file-backed JSON store).

pub use jcode_task_types::{TodoGoal, TodoItem};

/// Completed todos whose confidence trail ends in an unearned jump: a final
/// step of [`TODO_CONFIDENCE_SPIKE`]+ points in the tool-maintained
/// `confidence_history`, or, for todos without a recorded trail, an equally
/// large gap between planning `confidence` and `completion_confidence`.
pub const TODO_CONFIDENCE_SPIKE: u8 = 50;

/// Goals with a hill-climbability score strictly below this are considered
/// low: no credible metric to iterate against. The todo tool nudges the model
/// once per goal to either reframe the objective into something measurable or
/// deliberately mark it taste-driven and plan user checkpoints.
pub const LOW_HILL_CLIMBABILITY: u8 = 40;

use anyhow::Result;
use std::path::PathBuf;

use crate::bus::{Bus, BusEvent, TodoEvent};
use crate::storage::{self, read_json, write_json_fast};

/// Prefix for the confidence summary line appended to auto-poke messages.
pub const TODO_CONFIDENCE_SUMMARY_PREFIX: &str = "Confidence history:";

/// Build the auto-poke message shown when an agent has incomplete todos.
pub fn build_auto_poke_message(incomplete: usize) -> String {
    format!("You have {incomplete} incomplete todo items. Review and update the todo tool.")
}

fn todo_path(session_id: &str) -> Result<PathBuf> {
    let base = storage::jcode_dir()?;
    Ok(base.join("todos").join(format!("{}.json", session_id)))
}

/// Load todos for a session from disk.
pub fn load_todos(session_id: &str) -> Result<Vec<TodoItem>> {
    let path = todo_path(session_id)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    read_json(&path).or_else(|_| Ok(Vec::new()))
}

/// Check if any todos exist for a session.
pub fn todos_exist(session_id: &str) -> Result<bool> {
    Ok(todo_path(session_id)?.exists())
}

/// Save todos for a session to disk + broadcast TodoUpdated event.
///
/// Returns `Ok(true)` if a verification nudge should be injected into the
/// tool result (model just closed 3+ tasks without a verification step).
/// Caller (TodoTool) reads this and appends reminder text to its output.
///
/// Returns `Ok(false)` when no nudge is warranted, or when the save completed
/// but the verification check did not fire.
pub fn save_todos(session_id: &str, todos: &[TodoItem]) -> Result<bool> {
    let path = todo_path(session_id)?;
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    // Load previous state to compute verification nudge delta.
    let previous = load_todos(session_id).unwrap_or_default();
    let nudge = needs_verification_nudge(&previous, todos);
    write_json_fast(&path, todos)?;
    // Broadcast update to subscribers (TUI panel, metrics, etc.).
    Bus::global().publish(BusEvent::TodoUpdated(TodoEvent {
        session_id: session_id.to_string(),
        todos: todos.to_vec(),
        at: chrono::Utc::now(),
    }));
    Ok(nudge)
}

/// Detect the auto-poke prompt.
pub fn is_auto_poke_message(message: &str) -> bool {
    let trimmed = message.trim();
    (trimmed.starts_with("You have ")
        && trimmed.contains(" incomplete todo")
        && trimmed.ends_with("update the todo tool."))
        || trimmed.starts_with("Confidence: ")
}

/// Detect close-out 3+ tasks không có verification step.
/// Source: claude-code v1 verificationNudgeNeeded.
///
/// Returns true khi model vừa complete ≥3 task (chưa complete trước đó)
/// và không có task nào chứa "verif" (case-insensitive). Khi false,
/// tool không cần inject reminder.
///
/// `previous` = state trước save; `updated` = state sau save.
pub fn needs_verification_nudge(previous: &[TodoItem], updated: &[TodoItem]) -> bool {
    use std::collections::HashSet;
    let was_completed: HashSet<&str> = previous
        .iter()
        .filter(|t| t.status == "completed")
        .map(|t| t.content.as_str())
        .collect();
    let newly_completed: Vec<&TodoItem> = updated
        .iter()
        .filter(|t| t.status == "completed" && !was_completed.contains(t.content.as_str()))
        .collect();
    if newly_completed.len() < 3 {
        return false;
    }
    !newly_completed.iter().any(|t| {
        t.content.to_ascii_lowercase().contains("verif")
            || t.active_form
                .as_deref()
                .map(|s| s.to_ascii_lowercase().contains("verif"))
                .unwrap_or(false)
    })
}

/// Detect completed todos whose confidence trail shows a suspicious jump:
/// a final leap of [`TODO_CONFIDENCE_SPIKE`]+ points at completion time,
/// suggesting the model retroactively inflated its confidence rather than
/// tracking genuine evidence as work progressed.
pub fn spike_completed_todos(todos: &[TodoItem]) -> Vec<&TodoItem> {
    todos
        .iter()
        .filter(|todo| todo.status == "completed")
        .filter(|todo| {
            let history = &todo.confidence_history;
            match history.len() {
                0 => {
                    todo.confidence
                        .zip(todo.completion_confidence)
                        .is_some_and(|(first, last)| {
                            last.saturating_sub(first) >= TODO_CONFIDENCE_SPIKE
                        })
                }
                1 => false,
                n => history[n - 1].saturating_sub(history[n - 2]) >= TODO_CONFIDENCE_SPIKE,
            }
        })
        .collect()
}

fn goals_path(session_id: &str) -> Result<PathBuf> {
    let base = storage::jcode_dir()?;
    Ok(base
        .join("todos")
        .join(format!("{}-goals.json", session_id)))
}

/// Load goals for a session from disk.
pub fn load_goals(session_id: &str) -> Result<Vec<TodoGoal>> {
    let path = goals_path(session_id)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    storage::read_json(&path).or_else(|_| Ok(Vec::new()))
}

/// Derive a concise session-title hint from the todo tool's persisted plan.
///
/// Todo groups are intended to name coherent goals, so the group containing the
/// current (or latest incomplete) item is the strongest signal. Ungrouped plans
/// fall back to their measurable objective, then the item text itself.
pub fn derive_session_title(todos: &[TodoItem], goals: &[TodoGoal]) -> Option<String> {
    fn non_empty(value: Option<&str>) -> Option<String> {
        value
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    }

    let current = todos
        .iter()
        .rev()
        .find(|todo| todo.status.eq_ignore_ascii_case("in_progress"))
        .or_else(|| {
            todos
                .iter()
                .rev()
                .find(|todo| !todo.status.eq_ignore_ascii_case("completed"))
        })
        .or_else(|| todos.last());

    if let Some(todo) = current {
        if let Some(group) = non_empty(todo.group.as_deref()) {
            return Some(group);
        }

        if let Some(objective) = goals
            .iter()
            .rev()
            .find(|goal| goal.group.is_none())
            .and_then(|goal| non_empty(goal.objective.as_deref()))
        {
            return Some(objective);
        }

        return non_empty(Some(&todo.content));
    }

    goals.iter().rev().find_map(|goal| {
        non_empty(goal.group.as_deref()).or_else(|| non_empty(goal.objective.as_deref()))
    })
}

/// Load todo state for a session and derive its best title hint.
pub fn load_session_title(session_id: &str) -> Option<String> {
    let todos = load_todos(session_id).ok()?;
    let goals = load_goals(session_id).unwrap_or_default();
    derive_session_title(&todos, &goals)
}

/// Save goals for a session to disk.
pub fn save_goals(session_id: &str, goals: &[TodoGoal]) -> Result<()> {
    let path = goals_path(session_id)?;
    storage::write_json_fast(&path, goals)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(content: &str, status: &str) -> TodoItem {
        TodoItem {
            content: content.into(),
            status: status.into(),
            ..Default::default()
        }
    }

    #[test]
    fn triggers_at_3_no_verif() {
        let prev = vec![];
        let updated = vec![
            item("write code", "completed"),
            item("run lint", "completed"),
            item("commit", "completed"),
        ];
        assert!(needs_verification_nudge(&prev, &updated));
    }

    #[test]
    fn skipped_when_verif_present() {
        let prev = vec![];
        let updated = vec![
            item("write code", "completed"),
            item("run tests", "completed"),
            item("verify build", "completed"),
        ];
        assert!(!needs_verification_nudge(&prev, &updated));
    }

    #[test]
    fn counts_only_newly_completed() {
        let prev = vec![item("old done", "completed")];
        let updated = vec![
            item("old done", "completed"),
            item("new a", "completed"),
            item("new b", "completed"),
        ];
        assert!(!needs_verification_nudge(&prev, &updated));
    }

    #[test]
    fn below_threshold() {
        let prev = vec![];
        let updated = vec![item("a", "completed"), item("b", "completed")];
        assert!(!needs_verification_nudge(&prev, &updated));
    }

    #[test]
    fn case_insensitive_verif() {
        let prev = vec![];
        let updated = vec![
            item("Run VERIFICATION", "completed"),
            item("b", "completed"),
            item("c", "completed"),
        ];
        assert!(!needs_verification_nudge(&prev, &updated));
    }

    #[test]
    fn active_form_counts_as_verif() {
        let prev = vec![];
        let mut i1 = item("a", "completed");
        i1.active_form = Some("Verifying x".into());
        let updated = vec![i1, item("b", "completed"), item("c", "completed")];
        assert!(!needs_verification_nudge(&prev, &updated));
    }

    fn todo(content: &str, status: &str, group: Option<&str>) -> TodoItem {
        TodoItem {
            content: content.to_string(),
            status: status.to_string(),
            priority: "high".to_string(),
            id: content.to_ascii_lowercase().replace(' ', "-"),
            active_form: None,
            group: group.map(str::to_string),
            confidence: None,
            completion_confidence: None,
            confidence_history: Vec::new(),
            blocked_by: Vec::new(),
            assigned_to: None,
        }
    }

    #[test]
    fn session_title_prefers_in_progress_todo_group() {
        let todos = vec![
            todo("old task", "pending", Some("Older goal")),
            todo("current task", "in_progress", Some("Fix resume names")),
            todo("later task", "pending", Some("Later goal")),
        ];

        assert_eq!(
            derive_session_title(&todos, &[]).as_deref(),
            Some("Fix resume names")
        );
    }

    #[test]
    fn session_title_uses_latest_incomplete_group_when_nothing_is_active() {
        let todos = vec![
            todo("finished", "completed", Some("Old goal")),
            todo("next", "pending", Some("Current goal")),
        ];

        assert_eq!(
            derive_session_title(&todos, &[]).as_deref(),
            Some("Current goal")
        );
    }

    #[test]
    fn ungrouped_session_title_prefers_goal_objective_then_item_content() {
        let todos = vec![todo("Run targeted tests", "in_progress", None)];
        let goals = vec![TodoGoal {
            group: None,
            hill_climbability: Some(90),
            objective: Some("All resume naming tests pass".to_string()),
        }];

        assert_eq!(
            derive_session_title(&todos, &goals).as_deref(),
            Some("All resume naming tests pass")
        );
        assert_eq!(
            derive_session_title(&todos, &[]).as_deref(),
            Some("Run targeted tests")
        );
    }
}
