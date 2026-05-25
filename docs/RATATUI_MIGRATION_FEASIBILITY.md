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


---

## Phase 1 prototype: `jcode-tui-usage-overlay` (landed)

**Status**: ✅ Prototype merged into `experimental/ratatui-to-frankentui` (commit pending push).

The "If the user insists on prototyping" recipe above was implemented end-to-end. See the actual code in `crates/jcode-tui-usage-overlay/{Cargo.toml,src/lib.rs}`.

### What changed

| File | Change |
|---|---|
| `crates/jcode-tui-usage-overlay/Cargo.toml` | Added optional `ftui-style` git dep pinned to `quangdang46/frankentui@33ad1c57`, plus a `frankentui` feature that activates it. Kept `ratatui = "0.30"` as a regular (non-feature-gated) dep so the existing public API does not break. |
| `crates/jcode-tui-usage-overlay/src/lib.rs` | Introduced `pub const fn rgb(self) -> (u8, u8, u8)` as the single source of truth for status colors. `color()` (ratatui-shaped) and the new `color_ftui()` (frankentui-shaped, feature-gated) both delegate to `rgb()` so the two backends can never drift. Added 3 new tests that lock in this contract. |
| `Cargo.lock` | Bumped 3 lockfile entries inside existing semver bounds: `unicode-width 0.2.0 → 0.2.2`, `bitflags 2.10.0 → 2.11.1`, `bumpalo 3.19.1 → 3.20.3`. No `Cargo.toml` version requirements were widened. |

The git dep was deliberately picked over a path dep (`../../../frankentui/crates/ftui-style`) so CI machines and other contributors do not need a sibling `frankentui` checkout to resolve dependencies.

### What this prototype confirmed

| Question (from §B above) | Answer |
|---|---|
| Builds cleanly on jcode's toolchain? | **Yes**. `rustc 1.98.0-nightly` (jcode default) builds `ftui-core` → `ftui-render` → `ftui-style` without modification. Frankentui's `rust-toolchain.toml` pins nightly but in practice the style sub-tree we exercise stays edition-2024-only. |
| Acceptable transitive dep cost? | **Yes for this slice**. The `frankentui` feature pulls in `ftui-core`, `ftui-render`, `ftui-style`, plus three already-present-in-the-graph crates bumped within semver (`unicode-width`, `bitflags`, `bumpalo`). No new top-level deps when the feature is off (off-state graph is byte-for-byte the same modulo the lockfile bumps that are minor patch-level upgrades). |
| Both backends render identical RGB? | **Yes**. The new `frankentui_color_matches_rgb_source_of_truth` test round-trips through `ftui_style::Color::to_rgb()` and asserts `Color::Rgb(r, g, b)` equality with the ratatui side. |
| Default consumer (`src/tui/usage_overlay.rs`) still works? | **Yes**. `cargo check -p jcode --lib` succeeds with no changes to consumers; the existing `Style::default().fg(item.status.color())` call site keeps the ratatui `Color` type. |

### What this prototype did *not* answer

The prototype is intentionally minimal — it only swaps a `Color` constructor, which is the easiest surface area frankentui has. It does **not** validate any of the harder questions:

1. **Widget render pipeline.** The `Widget::render(&self, area, &mut Frame)` vs ratatui's `render(self, area, &mut Buffer)` signature mismatch is untouched here (this crate has no widgets).
2. **Buffer cell layout.** The 16-byte aligned, grapheme-pool-ID frankentui `Cell` has not been integrated.
3. **Terminal lifecycle.** `TerminalSession` (frankentui RAII) vs `Terminal::new` / `ratatui::init` is untouched.
4. **Test backend.** None of the 51 `TestBackend::new(...)` sites were migrated.
5. **`ratatui-image` replacement.** Mermaid pipeline still depends on `ratatui-image`.

So this prototype is a **necessary but not sufficient** foundation for a full migration. It only proves we *can* run both color models off the same data, which is the cheapest test of compatibility.

### Recommended next migration target

If we keep going, the next slice should be `jcode-tui-style` (4 ratatui refs, ~490 LOC). It is the next-smallest crate that touches color/style primitives and has no widgets or buffers of its own — same shape of change as Phase 1, slightly larger footprint.

After that, the order of escalation should be (from cheapest to hardest):

1. `jcode-tui-style` — pure color/style primitives, no widgets.
2. `jcode-tui-render` — switch from ratatui `Buffer` to frankentui `Buffer`/`Frame` for the lowest-level rendering layer. **This is the first hard step**: signatures change, downstream consumers ripple.
3. `jcode-tui-markdown` / `jcode-tui-messages` / `jcode-tui-tool-display` — widget-heavy crates. Each `impl Widget for Foo { fn render(self, area, buf) }` must become `impl Widget for Foo { fn render(&self, area, frame) }`.
4. `jcode-tui-usage-overlay` (consumer side, not the leaf crate) — the `src/tui/usage_overlay.rs` rewrite to use `color_ftui()` and `ftui_style::Style`.
5. `jcode-tui-mermaid` — needs a `ratatui-image` replacement before it can move.
6. Top-level `src/tui/` — the ~52 `Terminal::new` / `ratatui::init` / `restore` sites and 51 `TestBackend::new` test sites. **This is the longest single chunk** and probably should be split further (e.g., per-screen or per-overlay).

### Stop / continue gates

Don't proceed past **step 2 (`jcode-tui-render`)** without first answering:

- **Binary-size delta**: build a release binary in both states (`--no-default-features` vs `--features frankentui`) and compare `size target/release/jcode`. If the delta is more than +1 MB for a feature off → on flip, that's a signal frankentui's "all 20 crates" surface is leaking in.
- **Build-time delta**: clean `cargo build --release` time in both states. We expect the frankentui side to be slower because `ftui-render` + `ftui-core` are large; budget no more than +30s on a fresh build before pulling the brake.
- **Render parity**: shadow-render at least the chat view, the side panel, and one mermaid diagram through `ftui-harness::buffer_to_text()` and diff against the ratatui output. Any non-whitespace diff is a regression.
- **Frankentui upstream stability**: re-check `quangdang46/frankentui` and `Dicklesworthstone/frankentui` for breaking-change advisories. The current pin (`33ad1c57`) is on the `quangdang46` fork; if the upstream fork merges or diverges, we need to re-pin.

If any of those gates fail, the right answer is still **"stop and revert"** — the original recommendation in this document was to skip the migration, and Phase 1 is the smallest reversible unit (drop the feature, drop the dep, drop 3 lockfile bumps).
