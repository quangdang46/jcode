//! Permission dialogs and interactive overlays.
//!
//! This module re-exports the full public API from the `jcode-tui-permissions`
//! crate (the standalone permission-request TUI) and provides additional
//! dialog components such as the AskUserQuestion interactive overlay.

pub use jcode_tui_permissions::*;

pub mod ask_user_question;
