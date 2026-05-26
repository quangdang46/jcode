//! Color and theme primitives for the jcode TUI, built on `ftui_style::Color`.
//!
//! `clear_buf` was removed during the ratatui → frankentui migration. Use
//! `jcode_tui_workspace::color_support::clear_buf` instead — it is the
//! ratatui-shaped buffer-clearing helper while the rendering layer is still
//! ratatui-based.

pub mod color;
pub mod theme;

pub use color::{ColorCapability, color_capability, has_truecolor, indexed_to_rgb, rgb};
