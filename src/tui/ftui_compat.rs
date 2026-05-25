//! Conversion shims between frankentui (`ftui_style`) and ratatui types.
//!
//! Used during the `experimental/ratatui-to-frankentui` migration: leaf crates
//! return frankentui types (so they can drop the ratatui dep), but the
//! rendering layer still drives ratatui `Buffer`/`Frame`/`Widget` until that
//! layer is itself migrated. Until then, this module is the single,
//! easy-to-grep boundary where every conversion happens.
//!
//! When the rendering layer is fully migrated, this module disappears.

use ratatui::style::Color as RataColor;

/// Convert a frankentui color into the corresponding ratatui color.
///
/// `ftui_style::Color` is an enum with `Rgb`, `Ansi256`, `Ansi16`, and `Mono`
/// variants. We map each onto the closest ratatui `Color` variant. RGB values
/// are 1:1, palette indices stay numeric, and `Mono` collapses to true black
/// or white in the truecolor channel.
pub fn ftui_color_to_rata(c: ftui_style::Color) -> RataColor {
    match c {
        ftui_style::Color::Rgb(rgb) => RataColor::Rgb(rgb.r, rgb.g, rgb.b),
        ftui_style::Color::Ansi256(idx) => RataColor::Indexed(idx),
        ftui_style::Color::Ansi16(c) => ansi16_to_rata(c),
        ftui_style::Color::Mono(ftui_style::MonoColor::Black) => RataColor::Rgb(0, 0, 0),
        ftui_style::Color::Mono(ftui_style::MonoColor::White) => RataColor::Rgb(255, 255, 255),
    }
}

fn ansi16_to_rata(c: ftui_style::Ansi16) -> RataColor {
    use ftui_style::Ansi16::*;
    match c {
        Black => RataColor::Black,
        Red => RataColor::Red,
        Green => RataColor::Green,
        Yellow => RataColor::Yellow,
        Blue => RataColor::Blue,
        Magenta => RataColor::Magenta,
        Cyan => RataColor::Cyan,
        White => RataColor::Gray, // ratatui calls "light gray" Gray; "White" in ratatui is bright white
        BrightBlack => RataColor::DarkGray,
        BrightRed => RataColor::LightRed,
        BrightGreen => RataColor::LightGreen,
        BrightYellow => RataColor::LightYellow,
        BrightBlue => RataColor::LightBlue,
        BrightMagenta => RataColor::LightMagenta,
        BrightCyan => RataColor::LightCyan,
        BrightWhite => RataColor::White,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_round_trips() {
        let f = ftui_style::Color::rgb(12, 34, 56);
        assert_eq!(ftui_color_to_rata(f), RataColor::Rgb(12, 34, 56));
    }

    #[test]
    fn ansi256_index_preserved() {
        let f = ftui_style::Color::Ansi256(196);
        assert_eq!(ftui_color_to_rata(f), RataColor::Indexed(196));
    }

    #[test]
    fn ansi16_red_maps_to_rata_red() {
        let f = ftui_style::Color::Ansi16(ftui_style::Ansi16::Red);
        assert_eq!(ftui_color_to_rata(f), RataColor::Red);
    }

    #[test]
    fn mono_black_is_zero_rgb() {
        let f = ftui_style::Color::Mono(ftui_style::MonoColor::Black);
        assert_eq!(ftui_color_to_rata(f), RataColor::Rgb(0, 0, 0));
    }
}
