# Migration feasibility: ratatui → frankentui

**Status**: ❌ **Not recommended as a 1:1 migration**. Recommend hybrid approach OR skip.

## Quick verdict

| Question | Answer |
|---|---|
| Drop-in replacement? | **No** — frankentui's authors explicitly state "no backwards compatibility, no upgrade path" |
| Type compatibility (Color, Buffer, Rect, Style)? | **Different shapes** — `Color` enum nesting differs, `Buffer` cell layout differs, `Widget::render` signature differs |
| Effort estimate (full migration) | **2-4 weeks** of focused work + 1-2 weeks of UI regression hunt |
| Stability risk | **High** — frankentui status is "WIP" (yellow badge), version 0.4.0, requires Rust nightly |
| Performance gain? | Possible but unverified for jcode workload (frankentui claims 16-byte cells + cache-line alignment, but jcode's rendering is CPU-light vs other agent CLIs) |

## What jcode uses from ratatui

| Metric | Count |
|---|---|
| Files importing ratatui | 66 |
| Total `ratatui::*` references | 275 |
| Crates depending on ratatui | 9 (`jcode`, `jcode-tui-style`, `jcode-tui-render`, `jcode-tui-markdown`, `jcode-tui-messages`, `jcode-tui-mermaid`, `jcode-tui-usage-overlay`, `jcode-tui-workspace`, plus `ratatui-image`) |

Hot types (top 10):

| Type | Sites |
|---|---|
| `Terminal::new` | 52 |
| `TestBackend::new` | 51 |
| `prelude::*` | 17 |
| `Alignment::Left` | 17 |
| `Buffer` | 13 |
| `Modifier::BOLD` | 12 |
| `Rect` | 10 |
| `Line` | 7 |
| `Color` | 6 |
| `DefaultTerminal` | 7 |

## Concrete API mismatches

### 1. `Color` enum is shaped differently

**ratatui**:
```rust
pub enum Color {
    Reset, Black, Red, Green, ..., Indexed(u8), Rgb(u8, u8, u8),
}
```

**frankentui**:
```rust
pub enum Color {
    Rgb(Rgb),                // wrapper struct, not (u8,u8,u8)
    Ansi256(u8),             // similar
    Ansi16(Ansi16),          // nested enum for Black/Red/etc.
    Mono(MonoColor),
}
```

→ Every `Color::Red` becomes `Color::Ansi16(Ansi16::Red)`. ~6 sites direct, more via prelude imports.

### 2. `Widget::render` signature differs

**ratatui**:
```rust
trait Widget {
    fn render(self, area: Rect, buf: &mut Buffer);
}
```

**frankentui**:
```rust
trait Widget {
    fn render(&self, area: Rect, frame: &mut Frame);
}
```

→ Self by reference (not value) + `Frame` parameter (not `Buffer`). Every `impl Widget for Foo` block needs rewrite.

### 3. `Buffer` cell layout differs

**ratatui**: variable-size `Cell` with String symbol, fg/bg Color, modifier, plus skip flag, etc.

**frankentui**: 16-byte aligned `Cell` (`#[repr(C, align(16))]`) with grapheme pool ID, packed `CellAttrs` (4 bytes), no string symbols.

→ All `buf[(x,y)] = Cell::new(...)` patterns need rewrite. The frankentui `set_fast(x, y, cell)` API differs from ratatui's `Index<(u16,u16)>` ergonomics.

### 4. `Rect` types are technically incompatible

Both have `x, y, width, height: u16` but they're declared in different crates — Rust treats them as separate types. Every function signature taking `ratatui::layout::Rect` needs to take `ftui_core::geometry::Rect`.

### 5. `Terminal` lifecycle differs

**ratatui**:
```rust
let mut terminal = ratatui::init();
// ... use ...
ratatui::restore();
```

**frankentui**: `TerminalSession` (RAII guard) + `TerminalWriter` (serialized output gate). Different ownership model. The 52 `Terminal::new` callsites + 5 `restore`/3 `init` need rewrite.

### 6. Test backend is different

**ratatui**: `ratatui::backend::TestBackend::new(width, height)` returns a fake terminal you can drive + assert against.

**frankentui**: No `TestBackend`. Has `ftui_harness::buffer_to_text()` for snapshot testing but the testing pattern is different.

→ 51 test sites use `TestBackend::new(...)`. Each needs a redesign to use frankentui's harness pattern.

### 7. `ratatui-image` (used by jcode-tui-mermaid)

frankentui has no equivalent for `ratatui-image`. Image rendering inside the diagram pane would need a separate solution (either keep `ratatui-image` standalone if its API doesn't depend on ratatui's internals, or write a frankentui-native image protocol).

## Frankentui is WIP

From frankentui's own README:
- ![status](https://img.shields.io/badge/status-WIP-yellow) — explicit WIP banner
- ![rust](https://img.shields.io/badge/rust-nightly-blue) — README claims nightly. **Empirical test (this branch)**: `ftui-style` compiles on stable Rust even though `rust-toolchain.toml` pins nightly. Some features may still need nightly; needs deeper test.
- Version 0.4.0
- 850K+ lines in 20 crates (vs ratatui's lean ~50K) — adopting all of it is a major dependency surface increase
- Has features jcode doesn't need: Bayesian intelligence layer, conformal predictor, alpha investing (statistics)

## Realistic migration paths

### A. Don't migrate (recommended)

ratatui is mature, stable, tested. Migration cost (2-4 weeks dev + UI regression hunt) buys nothing concrete unless we have a measured perf bottleneck pointing at ratatui — and we don't.

### B. Hybrid: replace one subsystem at a time

Start with `jcode-tui-usage-overlay` (1 ratatui ref, 134 lines). Confirm:
1. Whether frankentui builds cleanly on stable Rust (jcode's toolchain)
2. Whether the bundle size + transitive deps are acceptable
3. Whether the swap actually saves anything vs the rewrite cost

If the prototype shows neutral-or-better tradeoff, expand to `jcode-tui-style` (4 refs, 490 lines). Stop after each step and re-evaluate.

### C. Wait for frankentui 1.0 + ratatui-compat shim

If frankentui's authors ever ship a `ratatui-compat` re-export crate (a thin shim that exposes ratatui-shaped types backed by frankentui), the migration becomes mechanical. Worth tracking via:
- `gh search prs --repo Dicklesworthstone/frankentui ratatui-compat`

Currently no such shim exists — `ftui-harness/tests/shadow_ratatui_e2e.rs` runs both libraries side-by-side for output comparison but doesn't provide a compatibility layer.

## Recommendation

**Skip the migration.** ratatui works, jcode renders fine, frankentui is WIP + nightly + non-trivial API differences. The 2-4 weeks of migration time has higher-ROI alternatives (e.g., port more upstream PRs, ship deferred features like Plan Mode integration, sandbox bash anchoring).

**If the user insists on prototyping**, the right next step is on this branch:

```bash
# 1. Try adding ftui-style as a peer dep (don't remove ratatui yet)
# 2. In jcode-tui-usage-overlay, add a feature flag that swaps Color
# 3. Build with frankentui's nightly toolchain to confirm it compiles
# 4. Shadow-render a few screens, compare output via ftui-harness
# 5. Decide based on output diff + binary size + build time
```

Even that prototype is 1-2 days of work. Done as throwaway; no production code changes until the data tells us frankentui is worth it.
