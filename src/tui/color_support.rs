//! Compatibility wrapper around `jcode_tui_style::color`.
//!
//! `jcode_tui_style` was migrated to return `ftui_style::Color` during the
//! `experimental/ratatui-to-frankentui` migration. The rendering layer in
//! `src/tui/*` still drives ratatui (`Style::fg`, `Style::bg`, `Buffer`,
//! `Frame`), so this module wraps the leaf-crate functions and converts at
//! the boundary. Consumers that import via `crate::tui::color_support::rgb`
//! continue to receive `ratatui::style::Color`, unchanged from before the
//! migration.
//!
//! When the rendering layer itself is migrated, the wrapper functions can
//! be removed in favor of bare `pub use jcode_tui_style::color::*`.

use crate::tui::ftui_compat::ftui_color_to_rata;
use ratatui::style::Color;

// Direct re-exports — these don't involve `Color` at all, so no conversion
// is needed. Some are unused inside the rendering layer today but kept on
// the boundary for parity with the leaf-crate API.
#[allow(unused_imports)]
pub use jcode_tui_style::color::{
    ColorCapability, color_capability, has_truecolor, indexed_to_rgb,
};

/// Build a ratatui `Color` from RGB, downgrading to a 256-palette index when
/// the terminal does not advertise truecolor support. Wraps
/// `jcode_tui_style::color::rgb` and converts the resulting `ftui_style::Color`
/// to `ratatui::style::Color` so existing call sites do not need to change.
#[inline]
pub fn rgb(r: u8, g: u8, b: u8) -> Color {
    ftui_color_to_rata(jcode_tui_style::color::rgb(r, g, b))
}

/// Buffer-clearing helper preserved for back-compat. `jcode_tui_style` no
/// longer ships its own `clear_buf` (it would have to depend on ratatui's
/// `Buffer`/`Rect` and we just dropped that dep). The implementation stays
/// inline here since it operates on ratatui types directly.
pub fn clear_buf(area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
    for x in area.left()..area.right() {
        for y in area.top()..area.bottom() {
            buf[(x, y)].reset();
        }
    }
}
