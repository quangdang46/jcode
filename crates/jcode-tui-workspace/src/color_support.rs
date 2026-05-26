//! Compatibility wrapper over `jcode_tui_style::color`.
//!
//! Pre-migration, this module duplicated the rgb / xterm256 conversion logic
//! that lived in `jcode-tui-style`. After Phase 3 of the
//! `experimental/ratatui-to-frankentui` migration, `jcode-tui-style` is
//! ratatui-free and returns `ftui_style::Color`. This module is now a thin
//! wrapper that converts back to `ratatui::style::Color` so consumers in this
//! workspace crate (notably `workspace_map_widget`) continue to drive
//! ratatui rendering unchanged.
//!
//! `clear_buf` stays here because it operates on `ratatui::Buffer`/`Rect`
//! directly. It is the only reason this crate's `Cargo.toml` still depends
//! on `ratatui`.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

// Direct re-exports — these don't involve a `Color` variant the wrapper
// needs to translate, so they pass through unchanged.
#[allow(unused_imports)]
pub use jcode_tui_style::color::{
    ColorCapability, color_capability, has_truecolor, indexed_to_rgb,
};

/// Build a ratatui `Color` from RGB, downgrading to a 256-palette index when
/// the terminal does not advertise truecolor support. Wraps
/// `jcode_tui_style::color::rgb` and converts the resulting `ftui_style::Color`
/// to `ratatui::style::Color`.
#[inline]
pub fn rgb(r: u8, g: u8, b: u8) -> Color {
    ftui_to_rata(jcode_tui_style::color::rgb(r, g, b))
}

/// Local conversion shim from `ftui_style::Color` to `ratatui::style::Color`.
/// This is a copy of `jcode::tui::ftui_compat::ftui_color_to_rata` — kept
/// inline here because this crate is below the top-level jcode crate in the
/// dep tree and cannot import from it.
fn ftui_to_rata(c: ftui_style::Color) -> Color {
    match c {
        ftui_style::Color::Rgb(rgb) => Color::Rgb(rgb.r, rgb.g, rgb.b),
        ftui_style::Color::Ansi256(idx) => Color::Indexed(idx),
        ftui_style::Color::Ansi16(c) => ansi16_to_rata(c),
        ftui_style::Color::Mono(ftui_style::MonoColor::Black) => Color::Rgb(0, 0, 0),
        ftui_style::Color::Mono(ftui_style::MonoColor::White) => Color::Rgb(255, 255, 255),
    }
}

fn ansi16_to_rata(c: ftui_style::Ansi16) -> Color {
    use ftui_style::Ansi16::*;
    match c {
        Black => Color::Black,
        Red => Color::Red,
        Green => Color::Green,
        Yellow => Color::Yellow,
        Blue => Color::Blue,
        Magenta => Color::Magenta,
        Cyan => Color::Cyan,
        White => Color::Gray,
        BrightBlack => Color::DarkGray,
        BrightRed => Color::LightRed,
        BrightGreen => Color::LightGreen,
        BrightYellow => Color::LightYellow,
        BrightBlue => Color::LightBlue,
        BrightMagenta => Color::LightMagenta,
        BrightCyan => Color::LightCyan,
        BrightWhite => Color::White,
    }
}

/// Buffer-clearing helper. Stays ratatui-shaped because the rendering layer
/// is still ratatui-based; collapses into the rendering migration when that
/// happens.
pub fn clear_buf(area: Rect, buf: &mut Buffer) {
    for x in area.left()..area.right() {
        for y in area.top()..area.bottom() {
            buf[(x, y)].reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_returns_ratatui_color() {
        // Just exercises the wrapper path; the actual variant depends on the
        // detected color capability of the host terminal at test time.
        let c = rgb(255, 0, 0);
        match c {
            Color::Rgb(_, _, _) | Color::Indexed(_) => {}
            other => panic!("unexpected variant: {:?}", other),
        }
    }

    #[test]
    fn ftui_to_rata_rgb_is_identity() {
        assert_eq!(
            ftui_to_rata(ftui_style::Color::rgb(11, 22, 33)),
            Color::Rgb(11, 22, 33)
        );
    }

    #[test]
    fn ftui_to_rata_ansi256_preserves_index() {
        assert_eq!(
            ftui_to_rata(ftui_style::Color::Ansi256(196)),
            Color::Indexed(196)
        );
    }
}
