# jcode Feature PR Backlog — From 13 Reference Repos

> Goal-driven implementation backlog. Each row = 1 PR against `master`.
> For each missing feature, the implementation subagent must:
> 1. Spawn a research subagent to verify the actual code in `/tmp/feature-research/<repo>`
> 2. Compare against jcode implementation
> 3. Produce a plan markdown: research findings, reasoning, alternatives considered, chosen approach
> 4. Implement, test, and open the PR
> 5. Attach the plan markdown to the PR description

## Priority Legend
- **P0** — Critical: Blocks core workflows or closes major user-visible gaps
- **P1** — High: Significant value, matches established patterns in multiple reference repos
- **P2** — Medium: Nice-to-have, ecosystem parity
- **P3** — Low: Experimental, niche use cases

## Effort Legend
- **S** — Small (<1 day)
- **M** — Medium (1-3 days)
- **L** — Large (3-7 days)
- **XL** — Extra Large (>1 week, may need to split)

---

## Section A — Provider System (from opencode, oh-my-pi, pi-agent-rust, crush)

| # | Feature | Source | Status | Pri | Effort | Plan File | Branch |
|---|---------|--------|--------|-----|--------|-----------|--------|
| A1 | Auth trait with combinators (Bearer/Header/Remove/Custom/Optional/Config/OrElse/AndThen/Pipe) | opencode | ✅ PR #466 | P0 | M | docs/pr-plans/A1-auth-trait-combinators.md | feat/A1-auth-trait-combinators |
| A2 | 4-axis Route (Protocol × Endpoint × Auth × Framing) | opencode | ✅ Implemented | P0 | L | — | master |
| A3 | Canonical LlmRequest/LlmEvent/LlmError schema | opencode | ✅ Implemented | P0 | M | — | master |
| A4 | OpenAI Responses protocol | opencode | ✅ Implemented | P0 | M | — | master |
| A5 | Anthropic Messages protocol | opencode | ✅ Implemented | P0 | M | — | master |
| A6 | 13 inband dialect layer (anthropic/deepseek/gemini/glm/harmony/kimi/qwen3/xml/etc) | oh-my-pi | ❌ Stub only | P1 | L | docs/pr-plans/A6-inband-dialects.md | feat/A6-inband-dialects |
| A7 | VCR test infrastructure (recorded-replay cassettes) | pi-agent-rust, opencode | ✅ Implemented | P1 | L | — | master |
| A8 | Reactive failover walker | oh-my-openagent, oh-my-pi | ❌ Missing | P1 | M | docs/pr-plans/A8-failover-walker.md | feat/A8-failover-walker |
| A9 | Catalog service (in-memory Map<ProviderId, ProviderEntry>) | opencode | ✅ Implemented | P1 | M | — | master |
| A10 | Integration/Credential service (OAuth PKCE + device code + API key) | opencode | ⚠️ Partial | P1 | M | docs/pr-plans/A10-integration-credential.md | feat/A10-integration-credential |
| A11 | Provider: Azure OpenAI Responses | codex | 🔜 Pending | P1 | S | docs/pr-plans/A11-provider-azure.md | feat/A11-provider-azure |
| A12 | Provider: Vertex AI (Claude + Gemini) | opencode, pi-agent-rust | 🔜 Pending | P1 | S | docs/pr-plans/A12-provider-vertex.md | feat/A12-provider-vertex |
| A13 | Provider: Groq | opencode | 🔜 Pending | P2 | S | docs/pr-plans/A13-provider-groq.md | feat/A13-provider-groq |
| A14 | Provider: Mistral | opencode | 🔜 Pending | P2 | S | docs/pr-plans/A14-provider-mistral.md | feat/A14-provider-mistral |
| A15 | Provider: Cohere v2 | pi-agent-rust | 🔜 Pending | P2 | S | docs/pr-plans/A15-provider-cohere.md | feat/A15-provider-cohere |
| A16 | TUI /provider command (list/login/logout/set default) | opencode, oh-my-pi | 🔜 Pending | P1 | M | docs/pr-plans/A16-tui-provider.md | feat/A16-tui-provider |
| A17 | TUI /model command (browse/filter/pick model) | opencode | 🔜 Pending | P1 | M | docs/pr-plans/A17-tui-model.md | feat/A17-tui-model |
| A18 | Models.dev auto-bootstrap with cache + fingerprint | opencode | 🔜 Pending | P1 | S | docs/pr-plans/A18-models-dev-bootstrap.md | feat/A18-models-dev-bootstrap |
| A19 | Provider Prometheus metrics | jcode-native | 🔜 Pending | P2 | S | docs/pr-plans/A19-provider-metrics.md | feat/A19-provider-metrics |

## Section B — Plugin System (from oh-my-pi, pi-agent-rust, opencode, crush, qwen-code)

| # | Feature | Source | Status | Pri | Effort | Plan File | Branch |
|---|---------|--------|--------|-----|--------|-----------|--------|
| B1 | ToolTier enum (Read/Write/Exec) + ApprovalGate | oh-my-pi | ✅ Implemented | P0 | M | — | master |
| B2 | CapabilityChainV2 (5-layer policy) | pi-agent-rust, oh-my-pi | 🔜 Pending | P1 | M | docs/pr-plans/B2-capability-chain-v2.md | feat/B2-capability-chain-v2 |
| B3 | PluginManager (load/unload/list/enable/disable with 3 source types) | oh-my-pi | ⚠️ Partial | P1 | M | docs/pr-plans/B3-plugin-manager.md | feat/B3-plugin-manager |
| B4 | Workspace crate plugin path (Rust crates via inventory::submit!) | oh-my-pi, pi-agent-rust | 🔜 Pending | P1 | S | docs/pr-plans/B4-workspace-crate-plugin.md | feat/B4-workspace-crate-plugin |
| B5 | Plugin hot-reload via SHA-256 fingerprint | opencode | 🔜 Pending | P2 | S | docs/pr-plans/B5-plugin-hot-reload.md | feat/B5-plugin-hot-reload |
| B6 | Per-extension kill switch (JCODE_PLUGIN_KILL_<NAME>) | pi-agent-rust | 🔜 Pending | P2 | S | docs/pr-plans/B6-plugin-kill-switch.md | feat/B6-plugin-kill-switch |
| B7 | CLI plugin subcommands (load/clone/list/unload/enable/disable/reload/info) | opencode | 🔜 Pending | P1 | S | docs/pr-plans/B7-cli-plugin-cmds.md | feat/B7-cli-plugin-cmds |
| B8 | Plugin author guide (docs/plugins.md) | oh-my-pi | 🔜 Pending | P1 | S | docs/pr-plans/B8-plugin-author-guide.md | feat/B8-plugin-author-guide |
| B9 | Plugin STRIDE threat model | pi-agent-rust | 🔜 Pending | P2 | S | docs/pr-plans/B9-plugin-threat-model.md | feat/B9-plugin-threat-model |

## Section C — Tools (from oh-my-pi, CCB, codebuff, codex, crush)

| # | Feature | Source | Status | Pri | Effort | Plan File | Branch |
|---|---------|--------|--------|-----|--------|-----------|--------|
| C1 | DAP (Debug Adapter Protocol, 27 ops) | oh-my-pi | ❌ Missing | P1 | XL | docs/pr-plans/C1-dap-debugger.md | feat/C1-dap-debugger |
| C2 | Tree-sitter code map (10+ languages, language-aware) | codebuff | ⚠️ Partial | P1 | L | docs/pr-plans/C2-tree-sitter-codemap.md | feat/C2-tree-sitter-codemap |
| C3 | Prompt variants per model (Claude vs GPT vs Gemini) | oh-my-openagent | ❌ Missing | P1 | S | docs/pr-plans/C3-prompt-variants.md | feat/C3-prompt-variants |
| C4 | Tmux session management (multi-pane) | oh-my-openagent | ⚠️ Partial | P2 | M | docs/pr-plans/C4-tmux-management.md | feat/C4-tmux-management |
| C5 | Voice Mode (speech-to-text + TTS) | CCB | ❌ Missing | P3 | L | docs/pr-plans/C5-voice-mode.md | feat/C5-voice-mode |
| C6 | Chrome Use (browser automation via Chrome DevTools) | CCB | ⚠️ Partial | P2 | M | docs/pr-plans/C6-chrome-use.md | feat/C6-chrome-use |
| C7 | Computer Use (cross-platform screen capture + vision) | CCB | ⚠️ Partial (macOS only) | P3 | XL | docs/pr-plans/C7-computer-use.md | feat/C7-computer-use |
| C8 | Langfuse monitoring integration | CCB | ❌ Missing | P2 | M | docs/pr-plans/C8-langfuse.md | feat/C8-langfuse |
| C9 | Sentry error tracking | CCB | ❌ Missing | P3 | M | docs/pr-plans/C9-sentry.md | feat/C9-sentry |
| C10 | GrowthBook feature flag integration | CCB | ❌ Missing | P3 | S | docs/pr-plans/C10-growthbook.md | feat/C10-growthbook |
| C11 | Pipe IPC multi-instance orchestration | CCB | ❌ Missing | P3 | XL | docs/pr-plans/C11-pipe-ipc.md | feat/C11-pipe-ipc |
| C12 | Remote Control Docker UI (phone-accessible) | CCB | ❌ Missing | P3 | XL | docs/pr-plans/C12-remote-control.md | feat/C12-remote-control |
| C13 | ACP Protocol (Zed/Cursor IDE integration) | CCB | ❌ Missing | P3 | XL | docs/pr-plans/C13-acp-protocol.md | feat/C13-acp-protocol |
| C14 | RTK Token Optimization (compress bash output 60-90%) | kimchi | ❌ Missing | P1 | M | docs/pr-plans/C14-rtk-token-opt.md | feat/C14-rtk-token-opt |
| C15 | Network Tool (port scan + host discovery) | crush | ❌ Missing | P2 | S | docs/pr-plans/C15-network-tool.md | feat/C15-network-tool |
| C16 | Webhook tool (receive + forward) | oh-my-openagent | ❌ Missing | P2 | S | docs/pr-plans/C16-webhook-tool.md | feat/C16-webhook-tool |
| C17 | SQLite diagnostic tool | oh-my-openagent | ❌ Missing | P2 | S | docs/pr-plans/C17-sqlite-diagnostic.md | feat/C17-sqlite-diagnostic |
| C18 | Bash script sandbox (read-only / no-network) | codex | ❌ Missing | P2 | S | docs/pr-plans/C18-bash-sandbox.md | feat/C18-bash-sandbox |
| C19 | Auto-reply tool (suggest + confirm) | oh-my-openagent | ❌ Missing | P3 | S | docs/pr-plans/C19-auto-reply.md | feat/C19-auto-reply |
| C20 | Infrastructure diagram MCP tool | pi-agent-rust | ❌ Missing | P3 | S | docs/pr-plans/C20-infra-diagram-mcp.md | feat/C20-infra-diagram-mcp |

## Section D — UI / Display (from CCB, codebuff, crush, kimchi)

| # | Feature | Source | Status | Pri | Effort | Plan File | Branch |
|---|---------|--------|--------|-----|--------|-----------|--------|
| D1 | Running items list (subagent + tool status) | CCB | ✅ Implemented | P1 | — | — | master |
| D2 | Agent detail overlay + live transcript | CCB | ✅ Implemented | P1 | — | — | master |
| D3 | Agent session attachment (Enter on running item) | CCB | ✅ Implemented | P1 | — | — | master |
| D4 | Agent definitions + registry | CCB | ✅ Implemented | P1 | — | — | master |
| D5 | Live token saver displays (RTK/Headroom/Caveman) | kimchi | ❌ Missing | P2 | M | docs/pr-plans/D5-token-saver-display.md | feat/D5-token-saver-display |
| D6 | /cost command (token+spend breakdown) | CCB | ❌ Missing | P1 | M | docs/pr-plans/D6-cost-command.md | feat/D6-cost-command |
| D7 | Web UI (full TypeScript SPA) | CCB | ❌ Missing | P3 | XL | docs/pr-plans/D7-web-ui.md | feat/D7-web-ui |
| D8 | Inline image rendering in TUI | codebuff | ❌ Missing | P3 | S | docs/pr-plans/D8-inline-image.md | feat/D8-inline-image |
| D9 | Custom color themes (CSS/TOML) | CCB | ✅ Implemented | P2 | — | — | master |
| D10 | Panel-based TUI layout | opencode | ✅ Implemented | P2 | — | — | master |
| D11 | Agent-specific color and theme | crush | ✅ Implemented | P2 | — | — | master |
| D12 | Custom splash screen on startup | crush | ✅ Implemented | P2 | — | — | master |
| D13 | Tooltip detail for each tool | ccrus | ✅ Implemented | P2 | — | — | master |
| D14 | Command completions | CCB | ✅ Implemented | P1 | — | — | master |
| D15 | Subagent session transcript management | crushed | ✅ Implemented | P2 | — | — | master |

## Section E — Git / Version Control (from CCB, codebuff, crush)

| # | Feature | Source | Status | Pri | Effort | Plan File | Branch |
|---|---------|--------|--------|-----|--------|-----------|--------|
| E1 | git-auto-commit with diff analysis | CCB | ✅ Implemented | P1 | — | — | master |
| E2 | Branch/status awareness in prompts | CCB | ✅ Implemented | P1 | S | — | master |
| E3 | git history viewer in TUI | codebuff | 🔜 Pending | P2 | M | docs/pr-plans/E3-git-history-viewer.md | feat/E3-git-history-viewer |
| E4 | git blame inline annotation | codebuff | ❌ Missing | P2 | S | docs/pr-plans/E4-git-blame-inline.md | feat/E4-git-blame-inline |
| E5 | Merge conflict resolution assistant | CCB | ❌ Missing | P2 | M | docs/pr-plans/E5-merge-conflict.md | feat/E5-merge-conflict |

## Section F — CLI / Control (from CCB, codex, crush)

| # | Feature | Source | Status | Pri | Effort | Plan File | Branch |
|---|---------|--------|--------|-----|--------|-----------|--------|
| F1 | /help with command groups | CCB | ✅ Implemented | P1 | — | — | master |
| F2 | /context command (view/trim/cache) | CCB | ✅ Implemented | P1 | — | — | master |
| F3 | /reset or /new to start fresh | CCB | ✅ Implemented | P1 | — | — | master |
| F4 | /cost breakdown command | CCB | ❌ Missing | P1 | M | docs/pr-plans/F4-cost-command.md | feat/F4-cost-command |
| F5 | /telemetry on/off | CCB | ✅ Implemented | P1 | — | — | master |
| F6 | /config to inspect/change settings | CCB | ⚠️ Partial | P2 | S | docs/pr-plans/F6-config-command.md | feat/F6-config-command |
| F7 | /delegate subagent spawning | oh-my-openagent | ✅ Implemented | P1 | — | — | master |
| F8 | /reasoning effort control | crush | ✅ Implemented | P2 | — | — | master |
| F9 | Shell injection detection | CCB | ✅ Implemented | P1 | — | — | master |
| F10 | Permission bypass mode | crush | ✅ Implemented | P2 | — | — | master |
| F11 | XML output wrap mode | CCB | ✅ Implemented | P1 | — | — | master |

## Section G — MCP / Integration (from CCB, codex, pi-agent-rust)

| # | Feature | Source | Status | Pri | Effort | Plan File | Branch |
|---|---------|--------|--------|-----|--------|-----------|--------|
| G1 | MCP server for external agents | CCB | ✅ Implemented | P1 | — | — | master |
| G2 | Memory palace (MemPalace) | CCB | ✅ Implemented | P2 | — | — | master |
| G3 | File system MCP tools | codex | ✅ Implemented | P2 | — | — | master |
| G4 | Computer Use MCP | CCB | ✅ Implemented | P2 | — | — | master |
| G5 | Web search/read MCP | CCB | ✅ Implemented | P2 | — | — | master |
| G6 | External MCP client connections | pi-agent-rust | ✅ Implemented | P2 | — | — | master |
| G7 | Stdio MCP for local tools | pi-agent-rust | ✅ Implemented | P2 | — | — | master |
| G8 | Prompt caching via MCP | pi-agent-rust | ❌ Missing | P3 | S | docs/pr-plans/G8-prompt-cache-mcp.md | feat/G8-prompt-cache-mcp |

## Section H — Security (from pi-agent-rust, codex)

| # | Feature | Source | Status | Pri | Effort | Plan File | Branch |
|---|---------|--------|--------|-----|--------|-----------|--------|
| H1 | WASM sandbox for extensions | pi-agent-rust | ❌ Missing | P2 | XL | docs/pr-plans/H1-wasm-sandbox.md | feat/H1-wasm-sandbox |
| H2 | Supply chain SBOM verification | pi-agent-rust | ❌ Missing | P3 | M | docs/pr-plans/H2-sbom-verify.md | feat/H2-sbom-verify |
| H3 | Secret scanning in git | codex | ❌ Missing | P2 | S | docs/pr-plans/H3-secret-scanning.md | feat/H3-secret-scanning |

## Section I — Observability (from CCB, pi-agent-rust)

| # | Feature | Source | Status | Pri | Effort | Plan File | Branch |
|---|---------|--------|--------|-----|--------|-----------|--------|
| I1 | Prometheus metrics exporter | jcode-native | ✅ Implemented | P2 | — | — | master |
| I2 | OpenTelemetry tracing | CCB | ✅ Implemented | P2 | — | — | master |
| I3 | Structured logging (JSON) | CCB | ✅ Implemented | P2 | — | — | master |
| I4 | Langfuse integration | CCB | ❌ Missing | P2 | M | docs/pr-plans/I4-langfuse.md | feat/I4-langfuse |
| I5 | Sentry error tracking | CCB | ❌ Missing | P3 | M | docs/pr-plans/I5-sentry.md | feat/I5-sentry |
| I6 | Per-request cost tracking | CCB | ✅ Implemented | P2 | — | — | master |

## Section J — Desktop / Platform (from CCB, oh-my-pi)

| # | Feature | Source | Status | Pri | Effort | Plan File | Branch |
|---|---------|--------|--------|-----|--------|-----------|--------|
| J1 | macOS background computer use | CCB | ⚠️ Partial (macOS only) | P2 | L | docs/pr-plans/J1-computer-use.md | feat/J1-computer-use |
| J2 | Window management (move/resize/focus) | CCB | ✅ Implemented | P2 | — | — | master |
| J3 | Keyboard + mouse input routing | CCB | ✅ Implemented | P2 | — | — | master |
| J4 | Screenshot capture + vision analysis | CCB | ✅ Implemented | P2 | — | — | master |
| J5 | Accessibility tree parsing (AX) | CCB | ✅ Implemented | P2 | — | — | master |
| J6 | Notifications (macOS native + terminal) | CCB | ✅ Implemented | P2 | — | — | master |
| J7 | App focus detection | CCB | ✅ Implemented | P2 | — | — | master |
| J8 | Multi-monitor support | CCB | ❌ Missing | P3 | M | docs/pr-plans/J8-multi-monitor.md | feat/J8-multi-monitor |
| J9 | Screen recording (ReplayKit) | CCB | ❌ Missing | P3 | L | docs/pr-plans/J9-screen-recording.md | feat/J9-screen-recording |
| J10 | Platform auto-detection (macOS/Windows/Linux) | CCB | ✅ Implemented | P2 | — | — | master |

## Implementation Summary

| Phase | Total | ✅ Done | 🔜 Pending | ❌ Missing | ⚠️ Partial |
|-------|-------|---------|------------|-----------|-----------|
| **P0 (Foundation)** | 6 | 6 | 0 | 0 | 0 |
| **P1 (Core)** | ~25 | 14 | 5 | 5 | 1 |
| **P2+ (Polish)** | ~50 | 18 | 6 | 22+ | 4 |

**Criteria for Success:**
- ✅ P0: 6/6 implemented (A1-A5, B1) — **100% complete**
- ✅ P1: 14/25 done — 56% of P1 implemented. Remaining P1 gaps: A6 (dialects), A8 (failover), A10 (integration/credential polish), A16 (TUI provider), A17 (TUI model), A11 (Azure provider), A12 (Vertex provider), C1 (DAP), C2 (tree-sitter codemap), C3 (prompt variants), C14 (RTK token opt), D6 (cost command), E1-E5 (git tools), F4 (cost command)
