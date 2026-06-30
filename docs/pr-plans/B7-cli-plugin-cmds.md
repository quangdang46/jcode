# B7 — CLI Plugin Subcommands

**Status:** Implemented ✓  
**Branch:** `feat/B7-cli-plugin-cmds`  
**PR:** [B7 CLI plugin subcommands](https://github.com/quangdang46/jcode/pull/new/feat/B7-cli-plugin-cmds)

## Summary

Add `jcode plugin` subcommands for managing plugin lifecycle: load, clone, list,
unload, enable, disable, reload, and info. These commands operate on the
`PluginManager` from `jcode-plugin-core`, which maintains persistent state in
`~/.jcode/plugins/installed.json`.

## Changes

### `src/cli/args.rs`
- Added `Plugin(PluginSubcommand)` variant to `Command` enum.
- Added `PluginSubcommand` enum with 8 subcommands: `Load`, `Clone`, `List`,
  `Unload`, `Enable`, `Disable`, `Reload`, `Info`.

### `src/cli/commands.rs`
- Added `run_plugin_command()` — dispatches each subcommand to the
  corresponding `PluginManager` method:
  - **Load** — load a plugin from a local path.
  - **Clone** — `git clone` from a URL (with optional `--rev`), then load.
  - **List** — list installed plugins with optional `--kind` filter
    (workspace/local/git).
  - **Unload** — remove a plugin from the registry (preserves files on disk).
  - **Enable** / **Disable** — toggle the `enabled` flag.
  - **Reload** — unload + re-load using the stored source; also attempts a
    hot-reload via `PluginLoader::reload()` if the plugin system is active.
  - **Info** — print debug representation of `InstalledPlugin`.

### `src/cli/dispatch.rs`
- Added `Command::Plugin(subcmd)` match arm → `commands::run_plugin_command()`.

### `crates/jcode-plugin-core/src/manager.rs` (existing)
- `PluginManager`: manages persistent plugin state (installed.json).
- `PluginSource`: enum with `Local`, `Git`, `WorkspaceCrate` variants.
- Git URL validation rejects shell-injection characters and non-https/non-git@
  schemes.

## Commands

```
jcode plugin load ./my-plugin
jcode plugin clone https://github.com/user/plugin.git
jcode plugin clone https://github.com/user/plugin.git --rev v1.0
jcode plugin list
jcode plugin list --kind git
jcode plugin unload my-plugin
jcode plugin enable my-plugin
jcode plugin disable my-plugin
jcode plugin reload my-plugin
jcode plugin info my-plugin
```

## Testing

- `cargo test -p jcode-plugin-core` — 139 tests + 3 manager tests (load/list,
  enable/disable, unload idempotent) pass.
- `cargo check -p jcode` — clean build.

## Review

- [x] All 8 plugin subcommands defined in args.rs
- [x] All 8 dispatch handlers in commands.rs
- [x] Dispatch wired in dispatch.rs
- [x] PluginManager persists state to `~/.jcode/plugins/installed.json`
- [x] Git clone with optional rev/branch checking
- [x] Git URL sanitization prevents shell injection
- [x] `jcode plugin reload` also hot-reloads via PluginLoader::reload()
- [x] Existing test suite passes
