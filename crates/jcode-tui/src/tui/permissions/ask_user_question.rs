//! AskUserQuestion dialog — a lightweight interactive question with numbered options,
//! a free-form "Other" input, a "Chat about this" button, and a submit/cancel confirmation.
//!
//! Rendered as a centered overlay popup in the TUI.

use crate::tui::color_support::rgb;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::*,
    style::Color,
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};
use serde::Deserialize;

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// One selectable option a user can pick.
#[derive(Debug, Clone, Deserialize)]
pub struct QuestionOption {
    /// Machine-readable value (e.g. "bash").
    pub value: String,
    /// Human-readable label (e.g. "Run a Bash command").
    pub label: String,
}

/// The question payload received from the agent.
#[derive(Debug, Clone, Deserialize)]
pub struct Question {
    /// Unique identifier for this question (used for tracking).
    pub id: String,
    /// The question text to display.
    pub text: String,
    /// Preset answer options. May be empty.
    #[serde(default)]
    pub options: Vec<QuestionOption>,
}

/// Which tab/page the user is currently on within the dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AskUserTab {
    /// The question + answer options view.
    Question,
    /// The submit confirmation view (shows warnings about unanswered / choice).
    Submit,
}

/// Interactive state for the AskUserQuestion dialog.
///
/// State machine:
///   `is_cancelled` → dialog closed without answer
///   `wants_to_chat` → user clicked "Chat about this"
///   Submitted (answers filled) → normal completion
#[derive(Debug, Clone)]
pub struct AskUserQuestionState {
    /// The question being asked.
    pub question: Question,
    /// Which tab is active.
    pub active_tab: AskUserTab,
    /// Index of the currently focused option / action row.
    pub cursor: usize,
    /// Answers keyed by option value (populated when user picks an option).
    pub answers: Vec<(String, String)>,
    /// Active free-form "Other" answer text being typed.
    pub other_answer: String,
    /// Whether the user cancelled the dialog.
    pub is_cancelled: bool,
    /// Whether the user chose to chat about the question.
    pub wants_to_chat: bool,
    /// Scroll offset within the answer text input.
    pub other_cursor: usize,
}

impl AskUserQuestionState {
    pub fn new(question: Question) -> Self {
        Self {
            question,
            active_tab: AskUserTab::Question,
            cursor: 0,
            answers: Vec::new(),
            other_answer: String::new(),
            is_cancelled: false,
            wants_to_chat: false,
            other_cursor: 0,
        }
    }

    /// Total number of interactive rows in the question view.
    fn row_count(&self) -> usize {
        let mut count = self.question.options.len();
        // "Other" row
        count += 1;
        // "Chat about this" row
        count += 1;
        count
    }

    /// Whether the user has provided any answer.
    pub fn has_answers(&self) -> bool {
        !self.answers.is_empty() || !self.other_answer.is_empty()
    }

    /// Whether the dialog is finished (submitted, cancelled, or wants_to_chat).
    pub fn is_finished(&self) -> bool {
        self.is_cancelled || self.wants_to_chat || self.answers_submitted()
    }

    /// Whether the user has submitted their answer (confirmed on submit tab).
    fn answers_submitted(&self) -> bool {
        self.active_tab == AskUserTab::Submit && self.has_answers()
    }

    /// Move cursor up.
    pub fn cursor_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    /// Move cursor down.
    pub fn cursor_down(&mut self) {
        if self.active_tab == AskUserTab::Question {
            let max = self.row_count().saturating_sub(1);
            if self.cursor < max {
                self.cursor += 1;
            }
        }
    }

    /// Act on the current cursor position in the question view.
    pub fn activate_cursor(&mut self) {
        let opt_count = self.question.options.len();
        if self.cursor < opt_count {
            // One of the numbered options
            let option = &self.question.options[self.cursor];
            self.answers.clear();
            self.answers
                .push((option.value.clone(), option.label.clone()));
            self.other_answer.clear();
            self.active_tab = AskUserTab::Submit;
            self.cursor = 0;
        } else if self.cursor == opt_count {
            // "Other" — stay on question tab, enter text mode
            // (text mode entry is handled externally)
        } else if self.cursor == opt_count + 1 {
            // "Chat about this"
            self.wants_to_chat = true;
        }
    }

    /// Submit the other-answer text.
    pub fn submit_other(&mut self) {
        let text = self.other_answer.trim().to_string();
        if !text.is_empty() {
            self.answers.clear();
            self.answers.push(("other".to_string(), text.clone()));
            self.active_tab = AskUserTab::Submit;
            self.cursor = 0;
        }
    }

    /// Confirm submission on the submit tab.
    pub fn confirm_submit(&mut self) {
        // stays on submit tab; mark finished via has_answers check
    }

    /// Cancel back to question tab.
    pub fn cancel_submit(&mut self) {
        self.active_tab = AskUserTab::Question;
        self.cursor = 0;
    }
}

// ---------------------------------------------------------------------------
// Color helpers
// ---------------------------------------------------------------------------

fn accent() -> Color {
    rgb(138, 180, 248)
}
fn dim() -> Color {
    rgb(80, 80, 80)
}
fn text_primary() -> Color {
    rgb(245, 245, 255)
}
fn text_secondary() -> Color {
    rgb(180, 180, 190)
}
fn text_muted() -> Color {
    rgb(120, 120, 130)
}
fn border_dim() -> Color {
    rgb(60, 60, 70)
}
fn border_accent() -> Color {
    rgb(138, 180, 248)
}
fn success_green() -> Color {
    rgb(100, 200, 100)
}
fn warning_yellow() -> Color {
    rgb(255, 200, 100)
}
fn error_red() -> Color {
    rgb(255, 100, 100)
}

fn tab_bg() -> Color {
    rgb(30, 34, 42)
}
fn tab_accent_bg() -> Color {
    rgb(40, 48, 62)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Render the question dialog into the given `Frame`, centered in `area`.
pub fn render(state: &AskUserQuestionState, frame: &mut Frame, area: Rect) {
    // Centered overlay dimensions
    let overlay_w = area.width.min(72).max(40);
    let overlay_h = area.height.min(28).max(12);
    let x = (area.width.saturating_sub(overlay_w)) / 2;
    let y = (area.height.saturating_sub(overlay_h)) / 2;

    let overlay = Rect {
        x,
        y,
        width: overlay_w,
        height: overlay_h,
    };

    clear_area(frame, overlay);

    match state.active_tab {
        AskUserTab::Question => render_question_view(state, frame, overlay),
        AskUserTab::Submit => render_submit_view(state, frame, overlay),
    }
}

/// Render the answer result for the transcript (after submission).
pub fn render_answer_result(state: &AskUserQuestionState, frame: &mut Frame, area: Rect) {
    let mut spans = Vec::new();

    // Question text
    spans.push(Span::styled(
        state.question.text.clone(),
        Style::default()
            .fg(text_primary())
            .add_modifier(Modifier::BOLD),
    ));

    if !state.answers.is_empty() {
        let answer_text = state
            .answers
            .iter()
            .map(|(_, label)| label.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            format!("[{}]", answer_text),
            Style::default().fg(success_green()),
        ));
    } else if !state.other_answer.is_empty() {
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            format!("[{}]", state.other_answer),
            Style::default().fg(success_green()),
        ));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

// ---------------------------------------------------------------------------
// Internal rendering helpers
// ---------------------------------------------------------------------------

/// Clear a rectangular area (fill with space) before drawing the overlay.
fn clear_area(frame: &mut Frame, area: Rect) {
    frame.render_widget(Clear, area);
    // Fill with a dim backdrop
    let bg_block = Block::default()
        .style(Style::default().bg(Color::Rgb(20, 22, 28)))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_dim()));
    frame.render_widget(bg_block, area);
}

// ---------------------------------------------------------------------------
// Question tab
// ---------------------------------------------------------------------------

fn render_question_view(state: &AskUserQuestionState, frame: &mut Frame, area: Rect) {
    let inner = inset(area, 1);

    // Layout: title bar, question text, options list, navigation hint
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),   // nav bar
            Constraint::Length(2),   // spacing
            Constraint::Min(3),      // question + options
            Constraint::Length(2),   // bottom spacing
        ])
        .split(inner);

    render_nav_bar(state, frame, chunks[0]);

    let mut lines: Vec<Line<'static>> = Vec::new();

    // Question text
    lines.push(Line::from(Span::styled(
        state.question.text.clone(),
        Style::default()
            .fg(text_primary())
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::raw(""));

    // Numbered options
    for (i, option) in state.question.options.iter().enumerate() {
        let selected = state.cursor == i;
        let prefix = format!(" {}. ", i + 1);
        let marker = if selected { "▸" } else { " " };
        let style = if selected {
            Style::default().fg(accent())
        } else {
            Style::default().fg(text_secondary())
        };

        lines.push(Line::from(vec![
            Span::styled(format!("{} ", marker), Style::default().fg(accent())),
            Span::styled(prefix, Style::default().fg(text_muted())),
            Span::styled(option.label.clone(), style),
        ]));
    }

    // "Other" row
    {
        let is_other = state.cursor == state.question.options.len();
        let marker = if is_other { "▸" } else { " " };
        let style = if is_other {
            Style::default().fg(accent())
        } else {
            Style::default().fg(text_secondary())
        };

        lines.push(Line::from(vec![
            Span::styled(format!("{} ", marker), Style::default().fg(accent())),
            Span::styled(
                format!(
                    " {}. ",
                    state.question.options.len() + 1
                ),
                Style::default().fg(text_muted()),
            ),
            Span::styled("Other (type your answer)", style),
        ]));

        // If cursor is on Other and there is text, show the input preview
        if is_other && !state.other_answer.is_empty() {
            lines.push(Line::from(vec![
                Span::raw("       "),
                Span::styled(
                    state.other_answer.clone(),
                    Style::default().fg(success_green()),
                ),
            ]));
        }
    }

    // "Chat about this" row
    {
        let chat_idx = state.question.options.len() + 1;
        let is_chat = state.cursor == chat_idx;
        let marker = if is_chat { "▸" } else { " " };
        let style = if is_chat {
            Style::default().fg(accent())
        } else {
            Style::default().fg(text_secondary())
        };

        lines.push(Line::from(vec![
            Span::styled(format!("{} ", marker), Style::default().fg(accent())),
            Span::styled(" 💬 Chat about this", style),
        ]));
    }

    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::styled(
            "  ↑↓ navigate · Enter select · Other: type to enter answer · Esc cancel",
            Style::default().fg(dim()),
        ),
    ]));

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), chunks[2]);
}

// ---------------------------------------------------------------------------
// Submit tab
// ---------------------------------------------------------------------------

fn render_submit_view(state: &AskUserQuestionState, frame: &mut Frame, area: Rect) {
    let inner = inset(area, 1);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // nav bar
            Constraint::Length(2), // spacing
            Constraint::Length(3), // answer preview + warning
            Constraint::Length(1), // spacing
            Constraint::Length(1), // submit / cancel buttons
        ])
        .split(inner);

    render_nav_bar(state, frame, chunks[0]);

    let mut lines: Vec<Line<'static>> = Vec::new();

    // Answer preview
    if !state.answers.is_empty() {
        let ans = &state.answers[0];
        lines.push(Line::from(vec![
            Span::styled("Your answer: ", Style::default().fg(text_secondary())),
            Span::styled(
                ans.1.clone(),
                Style::default()
                    .fg(success_green())
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
    } else if !state.other_answer.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Your answer: ", Style::default().fg(text_secondary())),
            Span::styled(
                state.other_answer.clone(),
                Style::default()
                    .fg(success_green())
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
    } else {
        // Unanswered warning
        lines.push(Line::from(vec![
            Span::styled(
                " ⚠  No answer selected yet — submit anyway?",
                Style::default().fg(warning_yellow()),
            ),
        ]));
    }

    lines.push(Line::raw(""));

    // Submit / Cancel buttons
    let is_submit_selected = state.cursor == 0;
    let submit_style = if is_submit_selected {
        Style::default()
            .fg(rgb(20, 24, 32))
            .bg(accent())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(text_secondary())
    };
    let cancel_style = if !is_submit_selected {
        Style::default()
            .fg(rgb(20, 24, 32))
            .bg(error_red())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(text_secondary())
    };

    lines.push(Line::from(vec![
        Span::styled("  [ Submit ]  ", submit_style),
        Span::raw("  "),
        Span::styled("  [ Cancel ]  ", cancel_style),
    ]));

    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::styled(
            "  ← → navigate · Enter confirm · Esc back to options",
            Style::default().fg(dim()),
        ),
    ]));

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), chunks[2]);
}

// ---------------------------------------------------------------------------
// Nav bar
// ---------------------------------------------------------------------------

fn render_nav_bar(state: &AskUserQuestionState, frame: &mut Frame, area: Rect) {
    let tabs = ["Question", "Submit"];
    let mut spans: Vec<Span<'static>> = Vec::new();

    for (i, tab_name) in tabs.iter().enumerate() {
        let active = match state.active_tab {
            AskUserTab::Question => i == 0,
            AskUserTab::Submit => i == 1,
        };

        let (fg, bg, bold) = if active {
            (accent(), tab_accent_bg(), true)
        } else {
            (text_muted(), tab_bg(), false)
        };

        let mut style = Style::default().fg(fg).bg(bg);
        if bold {
            style = style.add_modifier(Modifier::BOLD);
        }

        if i > 0 {
            spans.push(Span::raw(" "));
        }

        let checked = if i == 1 && state.has_answers() {
            " ✓"
        } else {
            ""
        };
        spans.push(Span::styled(format!(" {}{} ", tab_name, checked), style));
    }

    frame.render_widget(
        Paragraph::new(Line::from(spans)),
        area,
    );
}

// ---------------------------------------------------------------------------
// Layout helpers
// ---------------------------------------------------------------------------

/// Shrink a rect by `n` on all sides.
fn inset(area: Rect, n: u16) -> Rect {
    Rect {
        x: area.x.saturating_add(n),
        y: area.y.saturating_add(n),
        width: area.width.saturating_sub(n * 2),
        height: area.height.saturating_sub(n * 2),
    }
}
