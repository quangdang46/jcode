//! Compatibility wrapper around `jcode_tui_style::theme`.
//!
//! `jcode_tui_style` was migrated to return `ftui_style::Color` during the
//! `experimental/ratatui-to-frankentui` migration. The rendering layer here
//! still drives ratatui, so this module wraps each theme function and
//! converts at the boundary so existing call sites continue to work without
//! modification. When the rendering layer itself is migrated, these wrappers
//! collapse back into `pub(super) use jcode_tui_style::theme::{...}`.

use crate::tui::ftui_compat::{FtuiColorExt, RataColorExt};
use ratatui::prelude::*;

// === Solid color helpers (no input) ========================================

#[inline]
pub(super) fn user_color() -> Color {
    jcode_tui_style::theme::user_color().rata()
}
#[inline]
pub(super) fn ai_color() -> Color {
    jcode_tui_style::theme::ai_color().rata()
}
#[inline]
pub(super) fn tool_color() -> Color {
    jcode_tui_style::theme::tool_color().rata()
}
#[inline]
pub(super) fn file_link_color() -> Color {
    jcode_tui_style::theme::file_link_color().rata()
}
#[inline]
pub(super) fn dim_color() -> Color {
    jcode_tui_style::theme::dim_color().rata()
}
#[inline]
pub(super) fn accent_color() -> Color {
    jcode_tui_style::theme::accent_color().rata()
}
#[inline]
pub(super) fn system_message_color() -> Color {
    jcode_tui_style::theme::system_message_color().rata()
}
#[inline]
pub(super) fn queued_color() -> Color {
    jcode_tui_style::theme::queued_color().rata()
}
#[inline]
pub(super) fn asap_color() -> Color {
    jcode_tui_style::theme::asap_color().rata()
}
#[inline]
pub(super) fn pending_color() -> Color {
    jcode_tui_style::theme::pending_color().rata()
}
#[inline]
pub(super) fn user_text() -> Color {
    jcode_tui_style::theme::user_text().rata()
}
#[inline]
pub(super) fn user_bg() -> Color {
    jcode_tui_style::theme::user_bg().rata()
}
#[inline]
pub(super) fn ai_text() -> Color {
    jcode_tui_style::theme::ai_text().rata()
}
#[inline]
pub(super) fn header_icon_color() -> Color {
    jcode_tui_style::theme::header_icon_color().rata()
}
#[inline]
pub(super) fn header_name_color() -> Color {
    jcode_tui_style::theme::header_name_color().rata()
}
#[inline]
pub(super) fn header_session_color() -> Color {
    jcode_tui_style::theme::header_session_color().rata()
}

// === Color helpers that take Color args ====================================

#[inline]
pub(super) fn blend_color(from: Color, to: Color, t: f32) -> Color {
    jcode_tui_style::theme::blend_color(from.ftui(), to.ftui(), t).rata()
}

#[inline]
pub(super) fn rainbow_prompt_color(distance: usize) -> Color {
    jcode_tui_style::theme::rainbow_prompt_color(distance).rata()
}

#[inline]
pub(super) fn prompt_entry_color(base: Color, t: f32) -> Color {
    jcode_tui_style::theme::prompt_entry_color(base.ftui(), t).rata()
}

#[inline]
pub(super) fn prompt_entry_bg_color(base: Color, t: f32) -> Color {
    jcode_tui_style::theme::prompt_entry_bg_color(base.ftui(), t).rata()
}

#[inline]
pub(super) fn prompt_entry_shimmer_color(base: Color, pos: f32, t: f32) -> Color {
    jcode_tui_style::theme::prompt_entry_shimmer_color(base.ftui(), pos, t).rata()
}

// === Activity / spinner helpers (already returned non-Color types) =========

pub(super) fn activity_indicator_frame_index(elapsed: f32, fps: f32) -> usize {
    jcode_tui_style::theme::activity_indicator_frame_index(
        elapsed,
        fps,
        crate::perf::tui_policy().enable_decorative_animations,
    )
}

pub(super) fn activity_indicator(elapsed: f32, fps: f32) -> &'static str {
    jcode_tui_style::theme::activity_indicator(
        elapsed,
        fps,
        crate::perf::tui_policy().enable_decorative_animations,
    )
}

pub(super) fn animated_tool_color(elapsed: f32) -> Color {
    jcode_tui_style::theme::animated_tool_color(
        elapsed,
        crate::perf::tui_policy().enable_decorative_animations,
    )
    .rata()
}
