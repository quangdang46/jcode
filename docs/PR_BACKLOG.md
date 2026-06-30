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
| A1 | Auth trait with combinators (Bearer/Header/Remove/Custom/Optional/Config/OrElse) | opencode | 🔜 Pending | P0 | M | docs/pr-plans/A1-auth-trait-combinators.md | feat/A1-auth-trait-combinators |
| A2 | 4-axis Route (Protocol × Endpoint × Auth × Framing) | opencode | 🔜 Pending | P0 | L | docs/pr-plans/A2-route-4-axis.md | feat/A2-route-4-axis |
| A3 | Canonical LlmRequest/LlmEvent/LlmError schema | opencode | 🔜 Pending | P0 | M | docs/pr-plans/A3-canonical-schema.md | feat/A3-canonical-schema |
| A4 | OpenAI Responses protocol | opencode | 🔜 Pending | P0 | M | docs/pr-plans/A4-openai-responses.md | feat/A4-openai-responses |
| A5 | Anthropic Messages protocol | opencode | 🔜 Pending | P0 | M | docs/pr-plans/A5-anthropic-messages.md | feat/A5-anthropic-messages |
| A6 | 13 inband dialect layer (anthropic/deepseek/gemini/glm/harmony/kimi/qwen3/xml/etc) | oh-my-pi | 🟡 In Progress (3/12) | P1 | L | docs/pr-plans/A6-inband-dialects.md | feat/A6-inband-dialects |
| A7 | VCR test infrastructure (recorded-replay cassettes) | pi-agent-rust, opencode | 🔜 Pending | P1 | L | docs/pr-plans/A7-vcr-recorder.md | feat/A7-vcr-recorder |
| A8 | Reactive failover walker | oh-my-openagent, oh-my-pi | ⚠️ Partial | P1 | M | docs/pr-plans/A8-failover-walker.md | feat/A8-failover-walker |
| A9 | Catalog service (in-memory Map<ProviderId, ProviderEntry>) | opencode | 🔜 Pending | P1 | M | docs/pr-plans/A9-catalog-service.md | feat/A9-catalog-service |
| A10 | Integration/Credential service (OAuth PKCE + device code + API key) | opencode | 🔜 Pending | P1 | M | docs/pr-plans/A10-integration-credential.md | feat/A10-integration-credential |
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
| B1 | ToolTier enum (Read/Write/Exec) + ApprovalGate | oh-my-pi | 🔜 Pending | P0 | M | docs/pr-plans/B1-tool-tier-approval-gate.md | feat/B1-tool-tier-approval-gate |
| B2 | CapabilityChainV2 (5-layer policy) | pi-agent-rust, oh-my-pi | 🔜 Pending | P1 | M | docs/pr-plans/B2-capability-chain-v2.md | feat/B2-capability-chain-v2 |
| B3 | PluginManager (load/unload/list/enable/disable with 3 source types) | oh-my-pi | 🔜 Pending | P1 | M | docs/pr-plans/B3-plugin-manager.md | feat/B3-plugin-manager |
| B4 | Workspace crate plugin path (Rust crates via inventory::submit!) | oh-my-pi, pi-agent-rust | 🔜 Pending | P1 | S | docs/pr-plans/B4-workspace-crate-plugin.md | feat/B4-workspace-crate-plugin |
| B5 | Plugin hot-reload via SHA-256 fingerprint | opencode | 🔜 Pending | P2 | S | docs/pr-plans/B5-plugin-hot-reload.md | feat/B5-plugin-hot-reload |
| B6 | Per-extension kill switch (JCODE_PLUGIN_KILL_<NAME>) | pi-agent-rust | 🔜 Pending | P2 | S | docs/pr-plans/B6-plugin-kill-switch.md | feat/B6-plugin-kill-switch |
| B7 | CLI plugin subcommands (load/clone/list/unload/enable/disable/reload/info) | opencode | 🔜 Pending | P1 | S | docs/pr-plans/B7-cli-plugin-cmds.md | feat/B7-cli-plugin-cmds |
| B8 | Plugin author guide (docs/plugins.md) | oh-my-pi | ✅ Done (PR #471) | P1 | S | docs/pr-plans/B8-plugin-author-guide.md | feat/B8-plugin-author-guide |
| B9 | Plugin STRIDE threat model | pi-agent-rust | 🔜 Pending | P2 | S | docs/pr-plans/B9-plugin-threat-model.md | feat/B9-plugin-threat-model |

## Section C — Tools (from oh-my-pi, CCB, codebuff, codex, crush)

| # | Feature | Source | Status | Pri | Effort | Plan File | Branch |
|---|---------|--------|--------|-----|--------|-----------|--------|
| C1 | DAP (Debug Adapter Protocol, 27 ops) | oh-my-pi | ❌ Missing | P1 | XL | docs/pr-plans/C1-dap-debugger.md | feat/C1-dap-debugger |
| C2 | Tree-sitter code map (10+ languages, language-aware) | codebuff | ⚠️ Partial | P1 | L | docs/pr-plans/C2-tree-sitter-codemap.md | feat/C2-tree-sitter-codemap |
| C3 | Prompt variants per model (Claude vs GPT vs Gemini) | oh-my-openagent | ✅ Done (PR #470) | P1 | S | docs/pr-plans/C3-prompt-variants.md | feat/C3-prompt-variants |
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
| C15 | Agent Skills standard (AGENTS.md/.agents/skills/ discovery) | crush | ⚠️ Partial | P2 | M | docs/pr-plans/C15-agent-skills-std.md | feat/C15-agent-skills-std |
| C16 | crushignore (extend .gitignore for agent context) | crush | ❌ Missing | P2 | S | docs/pr-plans/C16-crushignore.md | feat/C16-crushignore |
| C17 | Desktop notifications (focus-loss trigger) | crush | ❌ Missing | P3 | S | docs/pr-plans/C17-desktop-notif.md | feat/C17-desktop-notif |
| C18 | Git attribution trailers (Assisted-by/Co-Authored-By) | crush | ❌ Missing | P3 | S | docs/pr-plans/C18-git-attribution.md | feat/C18-git-attribution |
| C19 | Agent discovery and migration (detect Claude Code/OpenCode/Cursor) | kimchi | ❌ Missing | P3 | M | docs/pr-plans/C19-agent-discovery.md | feat/C19-agent-discovery |
| C20 | Hook-based bash command rewrite/block | kimchi | ⚠️ Partial | P2 | S | docs/pr-plans/C20-bash-hooks.md | feat/C20-bash-hooks |

## Section D — Multi-Agent Orchestration (from oh-my-openagent, codebuff, kimchi, qwen-code)

| # | Feature | Source | Status | Pri | Effort | Plan File | Branch |
|---|---------|--------|--------|-----|--------|-----------|--------|
| D1 | Agent Arena (multi-model competition, side-by-side) | qwen-code | ❌ Missing | P2 | L | docs/pr-plans/D1-agent-arena.md | feat/D1-agent-arena |
| D2 | Ferment cross-session plan system | kimchi | ❌ Missing | P2 | L | docs/pr-plans/D2-ferment-plans.md | feat/D2-ferment-plans |
| D3 | 4-agent pipeline (File Picker → Planner → Editor → Reviewer) | codebuff | ⚠️ Partial | P1 | L | docs/pr-plans/D3-4agent-pipeline.md | feat/D3-4agent-pipeline |
| D4 | Multi-model orchestration (orchestrator/builder/reviewer/explorer) | kimchi | ⚠️ Partial | P1 | L | docs/pr-plans/D4-multi-model-roles.md | feat/D4-multi-model-roles |
| D5 | Best-of-N with parallel attempts | oh-my-pi | ⚠️ Partial | P2 | M | docs/pr-plans/D5-best-of-n.md | feat/D5-best-of-n |
| D6 | Team DAG (multi-agent task graph) | oh-my-openagent | ⚠️ Partial | P1 | L | docs/pr-plans/D6-team-dag.md | feat/D6-team-dag |

## Section E — Session/Persistence (from pi-agent-rust, kimchi, crush)

| # | Feature | Source | Status | Pri | Effort | Plan File | Branch |
|---|---------|--------|--------|-----|--------|-----------|--------|
| E1 | SQLite session store (segmented log + offset index) | pi-agent-rust | ⚠️ Partial (JSONL) | P1 | L | docs/pr-plans/E1-sqlite-sessions.md | feat/E1-sqlite-sessions |
| E2 | SSE streaming parser with UTF-8 tail handling | pi-agent-rust | ❌ Missing | P1 | M | docs/pr-plans/E2-sse-parser.md | feat/E2-sse-parser |
| E3 | Shared multi-client sessions (workspace) | crush | ❌ Missing | P2 | L | docs/pr-plans/E3-shared-sessions.md | feat/E3-shared-sessions |
| E4 | Remote teleport (spawn/detach/reattach workers) | kimchi | ❌ Missing | P3 | XL | docs/pr-plans/E4-remote-teleport.md | feat/E4-remote-teleport |
| E5 | Session memory topology graph | jcode-native | ⚠️ Partial | P2 | M | docs/pr-plans/E5-session-topology.md | feat/E5-session-topology |

## Section F — Workflow Pipeline (from gajae-code, kimchi)

| # | Feature | Source | Status | Pri | Effort | Plan File | Branch |
|---|---------|--------|--------|-----|--------|-----------|--------|
| F1 | Workflow pipeline: deep-interview → ralplan → ultragoal | gajae-code | ⚠️ Partial | P1 | L | docs/pr-plans/F1-workflow-pipeline.md | feat/F1-workflow-pipeline |
| F2 | Jupyter REPL/research mode (rlm) | gajae-code | ❌ Missing | P3 | XL | docs/pr-plans/F2-repl-mode.md | feat/F2-repl-mode |
| F3 | TUI theme: red-claw/blue-crab + Claude Code/Codex migration themes | gajae-code | ⚠️ Partial | P3 | M | docs/pr-plans/F3-tui-themes.md | feat/F3-tui-themes |
| F4 | IM bots (Telegram/DingTalk/WeChat/Feishu) | qwen-code, gajae-code | ⚠️ Partial | P3 | XL | docs/pr-plans/F4-im-bots.md | feat/F4-im-bots |

## Section G — TUI (from opencode, crush, kimchi)

| # | Feature | Source | Status | Pri | Effort | Plan File | Branch |
|---|---------|--------|--------|-----|--------|-----------|--------|
| G1 | File browser sidebar (workspace navigator) | opencode | ⚠️ Partial | P2 | L | docs/pr-plans/G1-file-browser.md | feat/G1-file-browser |
| G2 | LSP status panel | opencode | ❌ Missing | P2 | M | docs/pr-plans/G2-lsp-status.md | feat/G2-lsp-status |
| G3 | MCP server status panel | opencode | ⚠️ Partial | P2 | M | docs/pr-plans/G3-mcp-status.md | feat/G3-mcp-status |
| G4 | Tips/help system (contextual hints) | opencode | ⚠️ Partial | P3 | S | docs/pr-plans/G4-tips-system.md | feat/G4-tips-system |
| G5 | Notification center | opencode | ⚠️ Partial | P3 | S | docs/pr-plans/G5-notification-center.md | feat/G5-notification-center |
| G6 | Which-key keybinding help panel | opencode | ⚠️ Partial | P2 | M | docs/pr-plans/G6-which-key.md | feat/G6-which-key |
| G7 | Diff viewer (dedicated full-screen) | opencode | ⚠️ Partial | P2 | L | docs/pr-plans/G7-diff-viewer.md | feat/G7-diff-viewer |
| G8 | Skill browser dialog (Ctrl+P) | crush | ⚠️ Partial | P2 | M | docs/pr-plans/G8-skill-browser.md | feat/G8-skill-browser |

## Section H — Security (from pi-agent-rust, codex, CCB)

| # | Feature | Source | Status | Pri | Effort | Plan File | Branch |
|---|---------|--------|--------|-----|--------|-----------|--------|
| H1 | WASM extension runtime with capability gates | pi-agent-rust | ❌ Missing | P3 | XL | docs/pr-plans/H1-wasm-runtime.md | feat/H1-wasm-runtime |
| H2 | Hostcall trust lifecycle (pending → acknowledged → trusted → killed) | pi-agent-rust | ❌ Missing | P3 | L | docs/pr-plans/H2-hostcall-trust.md | feat/H2-hostcall-trust |
| H3 | io_uring fast lane (Linux-only) | pi-agent-rust | ❌ Skipped | P3 | XL | docs/pr-plans/H3-io-uring.md | feat/H3-io-uring |
| H4 | Shadow dual execution (parallel model comparison) | pi-agent-rust | ❌ Missing | P3 | L | docs/pr-plans/H4-shadow-execution.md | feat/H4-shadow-execution |

## Section I — Benchmarking/Eval (from oh-my-pi, codebuff, pi-agent-rust)

| # | Feature | Source | Status | Pri | Effort | Plan File | Branch |
|---|---------|--------|--------|-----|--------|-----------|--------|
| I1 | JBench eval framework (commit reconstruction) | codebuff | ⚠️ Partial | P2 | L | docs/pr-plans/I1-jbench-eval.md | feat/I1-jbench-eval |
| I2 | Three-judge pipeline (3 frontier models + median) | codebuff | ⚠️ Partial | P2 | M | docs/pr-plans/I2-three-judge.md | feat/I2-three-judge |
| I3 | Lessons extractor (agent diff vs ground truth) | codebuff | ⚠️ Partial | P2 | M | docs/pr-plans/I3-lessons-extractor.md | feat/I3-lessons-extractor |

## Section J — Polish & Ecosystem (from CCB, crush, kimchi)

| # | Feature | Source | Status | Pri | Effort | Plan File | Branch |
|---|---------|--------|--------|-----|--------|-----------|--------|
| J1 | First-wins flag policy (shared workspaces) | crush | ❌ Missing | P3 | S | docs/pr-plans/J1-first-wins-flag.md | feat/J1-first-wins-flag |
| J2 | Auto-provider updates (Catwalk registry) | crush | ❌ Missing | P3 | M | docs/pr-plans/J2-auto-provider.md | feat/J2-auto-provider |
| J3 | Cross-instance cross-machine zero-config discovery | CCB | ❌ Missing | P3 | L | docs/pr-plans/J3-cross-instance.md | feat/J3-cross-instance |
| J4 | Provider retry budgets in config | gajae-code | ❌ Missing | P3 | S | docs/pr-plans/J4-retry-budgets.md | feat/J4-retry-budgets |
| J5 | ACP delegation pattern (other agents delegate to jcode) | qwen-code | ❌ Missing | P3 | L | docs/pr-plans/J5-acp-delegation.md | feat/J5-acp-delegation |

---

## Backlog Statistics

- **Total features identified**: ~80 across 10 sections
- **P0 (critical)**: ~7 features
- **P1 (high)**: ~25 features
- **P2 (medium)**: ~30 features
- **P3 (low/niche)**: ~18 features

## Execution Order (suggested by dependency + priority)

**Phase 1 — Foundation (P0, weeks 1-2)**:
A1 → A2 → A3 → A4 → A5 → B1

**Phase 2 — Core Ecosystem (P1, weeks 3-6)**:
A6 → A7 → A8 → A9 → A10 → B2 → B3 → C2 → C3 → C14 → D3 → D4 → D6 → E1 → E2 → F1

**Phase 3 — Polish (P1-P2, weeks 7-10)**:
A11 → A12 → A16 → A17 → B4 → B7 → C4 → C6 → C15 → C16 → C20 → D5 → G1 → G2 → G3 → G6 → G7 → G8

**Phase 4 — Long Tail (P2-P3, weeks 11+)**:
Remaining P2/P3 items, prioritized by user demand.

---

## Per-PR Plan File Template

Each `docs/pr-plans/<ID>-<name>.md` must contain:

```markdown
# PR Plan: <Feature Name>

## Research Summary
- Source repo(s): <list>
- Key files inspected: <paths in /tmp/feature-research/...>
- Direct code links: <URLs to source repo files>

## Why This Feature Is Missing in jcode
- Gap analysis from PARITY.md §XIV
- Code path that should exist but doesn't

## Alternatives Considered
| Approach | Source Repo | Pros | Cons | Decision |
|----------|-------------|------|------|----------|
| ... | ... | ... | ... | ... |

## Chosen Approach
- Rationale
- Architectural alignment with jcode

## Implementation Plan
- File-by-file changes
- New types/structs
- Test cases
- Migration path (if applicable)

## Risk Analysis
- Performance impact
- Backwards compatibility
- Security implications

## Success Criteria
- [ ] Tests pass
- [ ] PARITY.md updated
- [ ] Docs updated
- [ ] Manual verification command listed
```