# Ratatui → FrankenTUI Migration: Complete Fix Workflow

## Context

The `feature/ratatui-to-frankentui` branch has 2081 compile errors across 80+ files. The migration started well (Phase 1-3 complete) but Phase 4-6 commits introduced mass errors due to incorrect ftui API paths, wrong type mappings, and incomplete ports. All 11 jcode-tui crates are already ported to ftui — the errors are concentrated in `src/tui/` files.

**Goal**: Get the branch compiling by systematically fixing every error category, working bottom-up from crates to `src/tui/` core.

---

## Phase 0: Quick Crate Wins (2 files, ~5 min)

### 0.1 Remove dead ratatui dep from `jcode-tui-render`
- **File**: `crates/jcode-tui-render/Cargo.toml`
- **Action**: Remove `ratatui = "0.28"` line — zero source references exist

### 0.2 Fix `jcode-tui-mermaid` ratatui remnants
- **Files**: `crates/jcode-tui-mermaid/Cargo.toml`, `crates/jcode-tui-mermaid/src/lib.rs`, test files
- **Actions**:
  - Remove `ratatui = "0.28"` from Cargo.toml
  - Replace 3x `ratatui::layout::Rect` → `ftui_core::geometry::Rect` in `lib.rs` stubs
  - Replace ratatui types in `mermaid_tests/part_01.rs`, `part_02.rs`

---

## Phase 1: Fix Incorrect ftui Import Paths (~200 errors)

These are mechanical find-and-replace fixes across ~30 files.

### 1.1 Fix `ftui_style::Modifier` → remove entirely
**Problem**: `Modifier` doesn't exist in ftui_style. Used ~20 times across 6 files.
**Fix**: Remove the import. Replace usage:
- `.add_modifier(Modifier::BOLD)` → `.bold()`
- `.add_modifier(Modifier::ITALIC)` → `.italic()`
- `.add_modifier(Modifier::DIM)` → `.dim()`
- `.add_modifier(Modifier::UNDERLINE)` → `.underline()`
- `.add_modifier(Modifier::REVERSED)` → `.reversed()`
- `.add_modifier(Modifier::SLOW_BLINK)` → `.slow_blink()`
- `.add_modifier(Modifier::RAPID_BLINK)` → `.rapid_blink()`

**Files**: `session_picker.rs`, `ui_pinned.rs`, `ui_overlays.rs`, `info_widget.rs`, `info_widget_git.rs`, `info_widget_model.rs`, `login_picker.rs`, `account_picker.rs`

### 1.2 Fix layout imports from wrong module
**Problem**: `ftui_widgets::block::{Layout, Constraint, Direction}` and `ftui_widgets::layout::Direction` don't exist there.
**Fix**:
- `ftui_widgets::block::Layout` → `ftui_layout::Flex`
- `ftui_widgets::block::Constraint` → `ftui_layout::Constraint`
- `ftui_widgets::block::Direction` → `ftui_layout::Direction`
- `ftui_widgets::layout::Direction` → `ftui_layout::Direction`

**Files**: `session_picker.rs`, `login_picker.rs`, `account_picker.rs`, `info_widget.rs`

### 1.3 Fix `ftui_widgets::wrap::Wrap` → `ftui_text::wrap::WrapMode`
**Problem**: No `wrap` module in ftui_widgets. `Wrap { trim: bool }` is ratatui.
**Fix**: Import `ftui_text::wrap::WrapMode`. Replace `.wrap(Wrap { trim: false })` → `.wrap(WrapMode::Word)`

**Files**: `login_picker.rs`, `account_picker.rs`, `ui_pinned.rs`

### 1.4 Fix `ftui_widgets::Paragraph` import path
**Problem**: `Paragraph` is at `ftui_widgets::paragraph::Paragraph`, not `ftui_widgets::Paragraph`.
**Fix**: `use ftui_widgets::paragraph::Paragraph;`

**Files**: Files importing Paragraph from wrong path

### 1.5 Fix `ftui_widgets::Wrap` import
**Problem**: Top-level `ftui_widgets::Wrap` doesn't exist.
**Fix**: Replace with `use ftui_text::wrap::WrapMode;`

---

## Phase 2: Fix Type Mismatches (~700 errors)

### 2.1 `Line::from(Vec<Span>)` → `Line::from_spans(Vec<Span>)` (~105 occurrences)
**Problem**: ftui's `Line` has no `From<Vec<Span>>` impl.
**Fix**: `Line::from(vec![span1, span2])` → `Line::from_spans(vec![span1, span2])`

**Files**: `session_picker.rs` (19x), `login_picker.rs` (33x), `account_picker.rs` (22x), `ui_overlays.rs` (3x), `ui_pinned.rs` (3x), `info_widget.rs` (9x), `ui_messages.rs` (11x), `ui_viewport.rs` (5x)

### 2.2 `Text::from(Vec<Line>)` → `Text::from_lines(Vec<Line>)` (~12 occurrences)
**Problem**: ftui's `Text` has no `From<Vec<Line>>` impl.
**Fix**: `Text::from(lines)` → `Text::from_lines(lines)`, `Text::from(line)` → `Text::from_line(line)`

**Files**: `session_picker.rs`, `login_picker.rs`, `account_picker.rs`, `ui_pinned.rs`

### 2.3 `Color` enum variant mismatch (~120 occurrences)
**Problem**: ratatui `Color::White` → ftui `Color::Ansi16(Ansi16::White)` or `Color::Mono(MonoColor::White)`. ratatui `Color::Rgb(r,g,b)` → ftui `Color::Rgb(Rgb::new(r,g,b))`.
**Fix**: Use a lookup approach:
- Simple colors: `Color::White` → `Color::Mono(MonoColor::White)`, `Color::Black` → `Color::Mono(MonoColor::Black)`
- Named ANSI: `Color::Red` → `Color::Ansi16(Ansi16::Red)`, etc.
- RGB: `Color::Rgb(r,g,b)` → `Color::Rgb(Rgb::new(r,g,b))`
- DarkGray → `Color::Ansi16(Ansi16::BrightBlack)`, Gray → `Color::Ansi16(Ansi16::BrightBlack)`

**All ported files**

### 2.4 `Style::fg(Color)` → `Style::fg(PackedRgba)` (~200 occurrences)
**Problem**: ftui's `Style::fg()` takes `impl Into<PackedRgba>`, not `Color`.
**Fix options**:
- For constants: `.fg(PackedRgba::WHITE)` instead of `.fg(Color::White)`
- For computed colors: `.fg(color_to_packedrgba(&color))` using `compat.rs` helper
- Add `impl From<Color> for PackedRgba` in compat.rs or a local trait

**Best approach**: Add a `color_to_packedrgba()` call wrapper or implement a local conversion trait to minimize call-site changes.

**All ported files**

### 2.5 `Line::alignment()` doesn't exist (~50 occurrences)
**Problem**: ftui `Line` has no `.alignment()` method. Alignment is widget-level.
**Fix**: Remove `.alignment(X)` from `Line` calls. Move alignment to `Paragraph::new(text).alignment(X)` or handle at render level.

**Files**: `session_picker.rs` (19x), `ui_input.rs` (6x), `ui_header.rs` (20x), `ui_messages.rs` (5x), `ui_viewport.rs` (3x)

### 2.6 `Layout::default().direction().constraints().split()` → `Flex` API
**Problem**: ratatui `Layout` pattern is different from ftui `Flex`.
**Fix**:
```rust
// BEFORE:
Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Length(1), Constraint::Min(0)])
    .split(area)

// AFTER:
Flex::vertical()
    .constraints([Constraint::Fixed(1), Constraint::Min(0)])
    .split(area)
```

Also: `Constraint::Length(n)` → `Constraint::Fixed(n)`, `Constraint::Percentage(n)` → `Constraint::Percentage(n as f32)`, `Constraint::Fill(n)` → `Constraint::Fill`

**Files**: `session_picker.rs`, `login_picker.rs`, `account_picker.rs`, `info_widget.rs`, `info_widget_layout.rs`

### 2.7 Frame API differences
**Problem**: `frame.area()` doesn't exist, `frame.buffer_mut()` is `&mut frame.buffer`.
**Fix**:
- `frame.area()` → `Rect::new(0, 0, frame.buffer.width(), frame.buffer.height())`
- `frame.buffer_mut()` → `&mut frame.buffer`
- `frame.buffer_mut().cell(...)` → `frame.buffer.cell(...)`
- `frame.buffer_mut().get_mut(...)` → `frame.buffer.get_mut(...)`

**Files**: `session_picker.rs`, `login_picker.rs`, `account_picker.rs`, `ui_viewport.rs`, `ui_pinned.rs`, `info_widget.rs`

---

## Phase 3: Fix `src/tui/ui.rs` and `mod.rs` — The Root Cause (~500 errors)

These files import `ratatui::prelude::*` and all child modules inherit the pollution via `use super::*`.

### 3.1 Port `src/tui/ui.rs`
- Change `use ratatui::{prelude::*, widgets::Paragraph}` → explicit ftui imports
- This is the 2400-line draw function — the biggest single file
- After this fix, all `ui_*.rs` child modules lose their ratatui pollution

### 3.2 Port `src/tui/mod.rs`
- Change `use ratatui::prelude::Frame; use ratatui::text::Line` → ftui equivalents
- Fix `DisplayMessage` vs `RenderedMessage` type mismatch (line 1280-1283)

### 3.3 Fix `use super::*` in child modules
After `ui.rs` is ported, child modules (`ui_header.rs`, `ui_messages.rs`, `ui_viewport.rs`, `ui_input.rs`, etc.) will lose their ratatui imports. Replace `use super::*;` with explicit imports of what's actually needed.

---

## Phase 4: Fix `src/tui/app/` Module (~400 errors)

### 4.1 Port `src/tui/app.rs`
- Replace `use ratatui::DefaultTerminal` with ftui backend type
- The `DefaultTerminal` type needs to map to whatever ftui uses (`ftui_tty::TtyBackend` or the ftui facade type)

### 4.2 Port `src/tui/app/input.rs`, `local.rs`, `remote.rs`, `run_shell.rs`
- Replace `DefaultTerminal` references
- Replace `ratatui::Terminal`, `ratatui::backend::Backend`
- Replace `ratatui::buffer::Buffer`, `ratatui::layout::Rect`, `ratatui::style::Style`

### 4.3 Port `src/tui/app/navigation.rs`, `replay.rs`
- Replace `ratatui::layout::Rect` → `ftui_core::geometry::Rect`

### 4.4 Fix remaining `src/tui/` files
- `layout_utils.rs` — `ratatui::layout::Rect`
- `permissions.rs` — full ratatui import
- `ui_layout.rs`, `ui_diff.rs`, `ui_inline.rs`, `ui_inline_interactive.rs` — `ratatui::prelude::*`
- `ui_debug_capture.rs` — `ratatui::prelude::Rect`
- `ui_theme.rs`, `ui_tools.rs` — `ratatui::prelude::*`
- `visual_debug.rs` — `ratatui::layout::Rect`
- `usage_overlay.rs` — full ratatui import, not ported at all

---

## Phase 5: Fix Workspace & Usage Overlay (~80 errors)

### 5.1 Fix `src/tui/workspace_client.rs`
**Problem**: 30+ `WorkspaceMapModel` method calls don't resolve — `is_empty()`, `focus_session_by_id()`, `visible_rows()`, `current_workspace()`, etc.
**Fix**: Check `jcode-tui-workspace` crate's actual `WorkspaceMapModel` API and update call sites. The crate is already ported to ftui but the API surface may differ from what `workspace_client.rs` expects.

### 5.2 Fix `src/tui/usage_overlay.rs`
**Problem**: Still fully uses ratatui. ~20 field access errors on `UsageOverlayItem`/`UsageOverlaySummary`.
**Fix**: Full port using same patterns as other files. The `jcode-tui-usage-overlay` crate is already ported — check its actual struct fields and update `usage_overlay.rs` accordingly.

---

## Phase 6: Fix Info Widget Files (~50 errors)

### 6.1 Fix `info_widget_*.rs` field/type mismatches
**Problem**: Various type mismatches in the info widget family.
**Files**: `info_widget.rs`, `info_widget_git.rs`, `info_widget_layout.rs`, `info_widget_memory_render.rs`, `info_widget_model.rs`, `info_widget_swarm_background.rs`, `info_widget_tips.rs`, `info_widget_todos.rs`, `info_widget_usage.rs`, `info_widget_tests.rs`

---

## Phase 7: Final Cleanup

### 7.1 Remove `ratatui` from workspace Cargo.toml
- Once all source references are gone, remove `ratatui = "0.30"` and `crossterm` from workspace deps

### 7.2 Fix `video_export.rs` type mismatches
- Line 594: `expected &str, found u64` and `expected Vec<u8>, found (_, _, _)`

### 7.3 Fix `src/cli/terminal.rs`, `src/cli/tui_launch.rs`, `src/cli/commands.rs`
- Unused `terminal` variable warnings → prefix with `_`

---

## Error Category → Fix Mapping Summary

| Error Type | Count | Phase | Root Cause |
|------------|-------|-------|------------|
| E0277 trait bound | 713 | 2.4, 2.5 | Color → PackedRgba, Style methods |
| E0308 type mismatch | 468 | 2.1-2.6 | ratatui vs ftui types |
| E0599 no method | 263 | 2.5, 2.7, 5 | Line::alignment, Frame API, WorkspaceMapModel |
| E0609 no field | 180 | 5.2, 6 | UsageOverlay fields, info widget fields |
| E0433 unresolved type | 159 | 3, 4 | ratatui types via super::* pollution |
| E0560 struct field | 142 | 2.3, 2.6 | Color enum variants, Constraint variants |
| E0432 unresolved import | 25 | 1.1-1.5 | Wrong ftui import paths |
| E0061 arg count | 49 | 2.6 | Constraint::Length vs Fixed, Percentage f32 |
| E0616 field access | 34 | 2.7 | frame.buffer_mut() vs frame.buffer |

---

## Correct ftui Import Reference

```rust
// Style
use ftui_style::{Color, Style, Rgb, Ansi16, MonoColor, StyleFlags};

// Layout
use ftui_layout::{Flex, Constraint, Direction};

// Text
use ftui_text::text::{Span, Line, Text};
use ftui_text::wrap::WrapMode;

// Widgets
use ftui_widgets::paragraph::Paragraph;
use ftui_widgets::block::Block;
use ftui_widgets::block::Alignment;
use ftui_widgets::borders::{Borders, BorderSet, BorderType};
use ftui_widgets::Widget;

// Render
use ftui_render::frame::Frame;
use ftui_render::buffer::Buffer;
use ftui_render::cell::{Cell, PackedRgba, CellAttrs};

// Runtime
use ftui_runtime::{Model, Cmd, App, AppBuilder};

// Geometry
use ftui_core::geometry::Rect;
```

## ratatui → ftui Quick Reference

| ratatui | ftui |
|---------|------|
| `Style::default().fg(c)` | `Style::new().fg(color_to_packedrgba(&c))` or `Style::new().fg(PackedRgba::WHITE)` |
| `.add_modifier(Modifier::BOLD)` | `.bold()` |
| `Color::White` | `Color::Mono(MonoColor::White)` or `PackedRgba::WHITE` |
| `Color::Rgb(r,g,b)` | `Color::Rgb(Rgb::new(r,g,b))` |
| `Line::from(vec![...])` | `Line::from_spans(vec![...])` |
| `Text::from(lines)` | `Text::from_lines(lines)` |
| `Layout::default().direction(Vertical)` | `Flex::vertical()` |
| `Constraint::Length(n)` | `Constraint::Fixed(n)` |
| `Constraint::Percentage(n)` | `Constraint::Percentage(n as f32)` |
| `frame.area()` | `Rect::new(0, 0, frame.buffer.width(), frame.buffer.height())` |
| `frame.buffer_mut()` | `&mut frame.buffer` |
| `.wrap(Wrap { trim: false })` | `.wrap(WrapMode::Word)` |
| `Line::from("text").alignment(Center)` | Set alignment on Paragraph instead |
| `frame.render_widget(widget, area)` | `widget.draw(ctx, area)` or direct buffer ops |
| `DefaultTerminal` | `ftui_tty::TtyBackend` or ftui facade type |
| `Span::styled("text", style)` | `Span::styled("text", style)` (same API) |
| `Block::bordered()` | `Block::new().borders(BorderSet::ALL)` |

---

## Beads Task Updates Needed

These beads should be **reopened** (marked incomplete) since the code doesn't compile:

| Bead | Phase | Current Status | Should Be |
|------|-------|---------------|-----------|
| `jcode-4we` | 4.3 | In-progress | In-progress (correct) |
| `jcode-hj9` | 4.1 | Open | Open — crates already ported, but src/tui/ usage needs updating |
| `jcode-qk7` | 4.2 | Open | Open — crate already ported |
| `jcode-p6d` | 4.4 | Open | Open — file has ftui imports but errors |
| `jcode-ut6` | 4.5 | Open | Open |
| `jcode-obs` | 4.6 | Open | Open |
| `jcode-vzo` | 4.7 | Open | Open |
| `jcode-ply` | 4.8 | Open | Open |

Phase 6 beads (19t, occ, zqs, 1ub, 1gy, wuy, 9ar) were "ported" in commit 85bc3014 but all have errors — they need the same import/type fixes.

---

## Execution Order

1. **Phase 0** — crate Cargo.toml cleanup (5 min)
2. **Phase 1** — fix import paths (30 min, mechanical)
3. **Phase 2.1-2.4** — batch type fixes (1-2 hr, mechanical)
4. **Phase 3** — fix ui.rs + mod.rs root imports (1 hr)
5. **Phase 2.5-2.7** — remaining type fixes (30 min)
6. **Phase 4** — app/ module port (1 hr)
7. **Phase 5** — workspace + usage overlay (30 min)
8. **Phase 6** — info widget fixes (30 min)
9. **Phase 7** — final cleanup + remove ratatui dep (15 min)

**Estimated total**: 4-6 hours of focused mechanical work.

---

## Verification

After each phase:
```bash
cargo check 2>&1 | grep "^error\[" | wc -l
```

Target: 0 errors. Then:
```bash
cargo test --workspace
```
