# PR Plan: B8 — Plugin Author Guide

## Research Summary
- **Source repo**: oh-my-pi (`docs/plugin-author-guide.md`, `crates/pi-plugin/README.md`)
- **Key files inspected**:
  - `crates/pi-plugin/src/lib.rs` — Plugin trait, lifecycle hooks, manifest format
  - `crates/pi-plugin-runtime/src/loader.rs` — Loading, sandboxing, resource limits
  - `docs/plugin-author-guide.md` — Step-by-step guide with examples

## Why This Feature Is Missing in jcode
jcode has `jcode-plugin-core` and `jcode-plugin-runtime` with a working plugin system, but there's no dedicated plugin author guide documentation. Developers must read the source code to understand how to create and register plugins.

## Implementation Plan
Write `docs/plugins.md` covering:
1. Plugin architecture overview (Plugin trait, lifecycle hooks)
2. Quick start: Hello World plugin
3. Plugin manifest format (Cargo.toml metadata)
4. Available hooks and their signatures
5. Configuration serialization
6. Testing: unit tests, integration tests, kill-switch test
7. Distribution: publishing, workspace plugins

## Risk Analysis
- **Docs only** — zero code risk, zero compile risk

## Success Criteria
- [ ] docs/plugins.md written with accurate API references
- [ ] Links verified against actual source files
