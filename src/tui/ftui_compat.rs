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

/// Extension trait that adds a short, infix-style conversion to ratatui's
/// `Color` for any `ftui_style::Color` value. Used at every consumer call
/// site that feeds a frankentui-shaped color into a ratatui `Style::fg`/`bg`:
///
/// ```ignore
/// use crate::tui::ftui_compat::FtuiColorExt;
/// let style = Style::default().fg(jcode_tui_style::theme::user_color().rata());
/// ```
///
/// The trait exists to make 500+ migration call-site rewrites grep-able and
/// mechanical: a bulk replace of `style::FN()` → `style::FN().rata()` is safe
/// because `.rata()` is only defined on `ftui_style::Color`. The trait
/// disappears once the rendering layer no longer consumes ratatui colors.
pub trait FtuiColorExt {
    fn rata(self) -> RataColor;
}

impl FtuiColorExt for ftui_style::Color {
    #[inline]
    fn rata(self) -> RataColor {
        ftui_color_to_rata(self)
    }
}

/// Mirror of [`FtuiColorExt`] for the reverse direction: turn a ratatui color
/// into the closest `ftui_style::Color`. Used inside the wrapper layer so
/// frankentui-internal helpers (e.g. `blend_color`, which takes two colors)
/// can be called from consumers that only have ratatui colors on hand.
pub trait RataColorExt {
    fn ftui(self) -> ftui_style::Color;
}

impl RataColorExt for RataColor {
    #[inline]
    fn ftui(self) -> ftui_style::Color {
        rata_color_to_ftui(self)
    }
}

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

/// Reverse of [`ftui_color_to_rata`]: map every ratatui color variant onto
/// the closest `ftui_style::Color`. `Color::Reset` has no direct equivalent
/// in `ftui_style` so it falls back to `Mono(White)` (the most neutral light
/// foreground, identical to ratatui's default-on-most-terminals behavior).
pub fn rata_color_to_ftui(c: RataColor) -> ftui_style::Color {
    use ftui_style::Ansi16 as FAnsi;
    match c {
        RataColor::Reset => ftui_style::Color::Mono(ftui_style::MonoColor::White),
        RataColor::Black => ftui_style::Color::Ansi16(FAnsi::Black),
        RataColor::Red => ftui_style::Color::Ansi16(FAnsi::Red),
        RataColor::Green => ftui_style::Color::Ansi16(FAnsi::Green),
        RataColor::Yellow => ftui_style::Color::Ansi16(FAnsi::Yellow),
        RataColor::Blue => ftui_style::Color::Ansi16(FAnsi::Blue),
        RataColor::Magenta => ftui_style::Color::Ansi16(FAnsi::Magenta),
        RataColor::Cyan => ftui_style::Color::Ansi16(FAnsi::Cyan),
        RataColor::Gray => ftui_style::Color::Ansi16(FAnsi::White),
        RataColor::DarkGray => ftui_style::Color::Ansi16(FAnsi::BrightBlack),
        RataColor::LightRed => ftui_style::Color::Ansi16(FAnsi::BrightRed),
        RataColor::LightGreen => ftui_style::Color::Ansi16(FAnsi::BrightGreen),
        RataColor::LightYellow => ftui_style::Color::Ansi16(FAnsi::BrightYellow),
        RataColor::LightBlue => ftui_style::Color::Ansi16(FAnsi::BrightBlue),
        RataColor::LightMagenta => ftui_style::Color::Ansi16(FAnsi::BrightMagenta),
        RataColor::LightCyan => ftui_style::Color::Ansi16(FAnsi::BrightCyan),
        RataColor::White => ftui_style::Color::Ansi16(FAnsi::BrightWhite),
        RataColor::Indexed(idx) => ftui_style::Color::Ansi256(idx),
        RataColor::Rgb(r, g, b) => ftui_style::Color::rgb(r, g, b),
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

    #[test]
    fn extension_trait_matches_function_form() {
        let f = ftui_style::Color::rgb(7, 8, 9);
        assert_eq!(f.rata(), ftui_color_to_rata(f));
    }

    #[test]
    fn rata_to_ftui_round_trips_rgb() {
        let r = RataColor::Rgb(11, 22, 33);
        let f = rata_color_to_ftui(r);
        assert_eq!(ftui_color_to_rata(f), r);
    }

    #[test]
    fn rata_to_ftui_round_trips_indexed() {
        let r = RataColor::Indexed(196);
        let f = rata_color_to_ftui(r);
        assert_eq!(ftui_color_to_rata(f), r);
    }

    #[test]
    fn rata_red_round_trips_through_ansi16() {
        let r = RataColor::Red;
        assert_eq!(ftui_color_to_rata(rata_color_to_ftui(r)), RataColor::Red);
    }

    #[test]
    fn extension_trait_round_trip() {
        let original = RataColor::Rgb(40, 80, 160);
        assert_eq!(original.ftui().rata(), original);
    }
}
