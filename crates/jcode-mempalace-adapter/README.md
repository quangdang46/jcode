# jcode-mempalace-adapter

Type-conversion layer between jcode's `MemoryEntry` and mempalace's `Drawer`.

## What's here

- **`convert` module** — bidirectional 1:1 conversions between:
  - `MemoryCategory` ↔ `DrawerKind` (including `Entity`, `Correction`, `Custom`)
  - `MemoryEntry` ↔ `Drawer` (all fields mapped)
  - `MemoryScope` ↔ `MemoryScope` (Project→Local, Global→Global, All→All)
  - `TrustLevel` ↔ String
  - `Reinforcement` ↔ `MpReinforcement`

- **Mirror types** (`Drawer`, `DrawerKind`, `DrawerId`, `MemoryScope`) — local
  definitions that match mempalace's public surface exactly, exported for
  downstream crates that need to construct mempalace-shaped values without
  pulling in the full `mempalace-core` crate.

## Why no mempalace-core dependency?

mempalace-core depends on `rusqlite 0.32` while jcode uses `rusqlite 0.33`
(via `cross_agent_session_resumer`). Both versions link to the native `sqlite3`
library, which cargo's resolver disallows. The mirror-type approach avoids this
entirely — zero conflict, always compiles.

When rusqlite versions align (either by updating mempalace to 0.33, or by
jcode pinning to 0.32), the mirrors can be replaced with `cfg(feature = "backend")`
gates that pull in the real `mempalace-core` types.

## Issues covered

| Issue | Status |
|-------|--------|
| #355 (MempalaceAdapter) | ✅ Type-conversion layer complete. Runtime adapter deferred until rusqlite aligns. |
| #356 (Data migration) | Deferred — needs Palace runtime |
| #357 (MemoryTool config) | Deferred — needs Palace runtime |
| #358 (Prompt injection) | Deferred — needs Palace runtime |
| #359 (Tests + docs) | ✅ Unit tests (6 pass), this README |

## Usage (once backend lands)

```rust
use jcode_mempalace_adapter::{memory_entry_to_drawer, drawer_to_memory_entry};

// jcode → mempalace
let drawer = memory_entry_to_drawer(&entry, MemoryScope::Project);

// mempalace → jcode
let entry = drawer_to_memory_entry(&drawer);
```

## Next steps

1. **Align rusqlite versions** — update mempalace's `rusqlite` to 0.33 OR pin jcode to 0.32
2. **Add `backend` feature** — pull in `mempalace-core`, expose `MempalaceAdapter` struct
3. **Wire MemoryTool** — add `memory_backend` config, dispatch to adapter
4. **Wire prompt injection** — bypass MemoryAgent when mempalace backend
5. **Data migration tool** — convert `~/.jcode/memory/*.json` to mempalace format
