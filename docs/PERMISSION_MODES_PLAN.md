# Permission Modes Implementation Plan

> Synthesized from: 9-repo research, dcg-core analysis, 3 rounds QA interview, discussion
> Date: 2026-05-30
> Branch: experiment/dcg-permission-modes

---

## 0. Decisions Made (from QA + Discussion)

| Topic | Decision | Source |
|-------|----------|--------|
| **Scope** | Match Claude Code — full 6-mode pipeline | Round 1 QA |
| **Modes** | All 6: Default, Plan, AcceptEdits, DontAsk, Auto, BypassPermissions | Round 1 QA |
| **Auto/YOLO classifier** | Yes, LLM-based — **built in jcode only** (NOT dcg-core) | Round 1 QA + Discussion |
| **Denial tracking** | Yes, match Claude Code: 3 consecutive / 20 total → fallback to prompt | Round 2 QA |
| **Dangerous patterns** | Build in dcg-core, not jcode. Reduce jcode code by leveraging dcg | Round 2 QA |
| **Safe command whitelist** | Yes, full whitelist (~50 commands like codex readOnlyCommandValidation) | Round 2 QA |
| **Mode cycling** | Yes, full Shift+Tab cycle in TUI (6 modes) | Round 2 QA |
| **Where to build missing features** | Build in dcg-core (keep clean, reusable, not jcode-specific) | Round 3 QA |
| **OS sandboxing** | No, app-level only for now | Round 3 QA |
| **YOLO: why NOT in dcg-core** | dcg consumers use dcg as CLI hook (exit codes), not Rust library. Only jcode links dcg-core. A Rust trait in dcg-core is useless for TS/Go consumers. YOLO is consumer-specific. | Discussion |
| **YOLO: where to build** | `src/yolo_classifier.rs` in jcode. Uses active LLM provider (Claude/Gemini/OpenAI). When Mode::Auto active, dcg_bridge asks YOLO before engine. | Discussion |
| **Protected paths** | Claude Code defaults: ~/.ssh, ~/.aws, ~/.config/gh, .git, .env | Round 3 QA |
| **Pack rules** | Yes, Phase 2 in dcg-core — integrate dcg-cli's 50+ security packs | Round 3 QA |
| **Config format** | TOML (match dcg ecosystem) | Round 3 QA |
| **MCP permissions** | ✅ **Unified pipeline** — MCP tools map `mcp__server__tool`, go through same `Engine::evaluate()`. Unknown MCP tools default = Prompt (ask user). Per-server config in TOML. Inspired by claude-code 3-level matching. | Research + Discussion |
| **YOLO: model strategy** | ✅ **Reuse active provider** (zero extra cost). 2-stage: Stage 1 fast (64 tokens, block yes/no) → Stage 2 thinking (4096 tokens, CoT) if uncertain. Fail closed on timeout/error. Inspired by claude-code yoloClassifier. | Research + Discussion |
| **YOLO: fail behavior** | ✅ **Fail closed** — block on all errors/timeouts/unavailable. 3 consecutive YOLO denials → fallback to interactive prompt (circuit breaker). | Research + Discussion |
| **Swarm/subagent permissions** | ✅ **Option A: inherit + restrict** (opencode pattern). (1) Inherit parent deny rules (session + agent), (2) Default deny recursive capabilities (subagent spawn, todowrite), (3) Children can do LESS, never MORE. | Research + Discussion |
| **Strict mode** | One-way tightening (from oh-my-claudecode) — cannot be weakened by project config | Discussion |
| **Path-aware escalation** | Edit .env/secrets/ → auto escalate even in AcceptEdits mode | Discussion |
| **Consumer-agnostic** | dcg-core exports generic Mode enum, each consumer maps own CLI flags | Discussion |
| **Bypass safety net** | BypassPermissions should have iteration cap + audit log | Discussion |
| **slb relationship** | SLB = two-person rule (peer review), complementary to dcg. No YOLO, no LLM, no Rust, Go-only, no library export. Not relevant for YOLO implementation. | Discussion |
| **Sandboxing** | ❌ **Not doing** — app-level only, no OS sandbox for now | Discussion |

---

## 1. Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        jcode (consumer)                         │
│                                                                 │
│  CLI flags: --permission-mode, --dangerously-skip-permissions   │
│  Config: .jcode/config.toml (TOML → maps to dcg types)         │
│  TUI: mode cycling (Shift+Tab), permission dialogs              │
│                                                                 │
│  ┌───────────── yolo_classifier.rs (jcode-only) ──────────┐    │
│  │  Uses active LLM provider (Claude/Gemini/OpenAI)        │    │
│  │  Feeds transcript + command → get allow/deny            │    │
│  │  Denial limits: 3 consecutive → fallback to prompt     │    │
│  │  Only active when Mode::Auto is set                     │    │
│  └─────────────────────────────────────────────────────────┘    │
│                          │                                      │
│  ┌─────────────────── dcg_bridge.rs ───────────────────────┐    │
│  │  action_to_tool_call() → ToolCall + Effects             │    │
│  │  classify():                                            │    │
│  │    if Mode::Auto → ask YOLO first                       │    │
│  │    else → Engine::evaluate() directly                   │    │
│  │    → BridgeDecision (Allow/Prompt/Deny)                 │    │
│  │  set_mode() / current_mode()                            │    │
│  └─────────────────────────────────────────────────────────┘    │
│                          │                                      │
└──────────────────────────┼──────────────────────────────────────┘
                           │ depends on (git URL)
┌──────────────────────────┼──────────────────────────────────────┐
│                    dcg-core (library)                            │
│                          ▼                                      │
│  Engine::evaluate(session, tool_call, mode, effects)            │
│    │                                                            │
│    ├─► Mode::pre_check() → AllowImmediately / Deny / Continue  │
│    ├─► ProtectedPaths check (path-aware escalation)             │
│    ├─► [Phase 2] Pack rule evaluation (50+ security packs)     │
│    ├─► Dangerous command patterns (26-50 regex)                 │
│    ├─► Safe command whitelist (~50 read-only commands)          │
│    ├─► Denial escalation (3 consecutive / 20 total)            │
│    └─► Decision: Allow / Prompt{reason,alternatives} / Deny   │
│                                                                 │
│  ❌ NO YOLO — YOLO is consumer-specific, built in jcode only    │
│  ❌ NO LLM — dcg-core is pure rule-based engine                 │
│  ❌ NO provider knowledge — consumer maps own CLI flags          │
│                                                                 │
│  Already has (v0.6.0-rc.1):                                     │
│    Mode (6 variants + pre_check)                                │
│    Effect (7 variants + is_read_only + is_subset)               │
│    ToolCall (5 variants: Bash/Edit/Write/Read/Network)          │
│    Decision (Allow/Prompt/Deny with reasons + alternatives)     │
│    Session (allow-once codes + per-command deny counter)        │
│    ProtectedPaths (prefix matcher + ~ expansion)                │
│    EngineConfig builder (working_dir + protected_paths)         │
│                                                                 │
│  Phase 2 additions:                                             │
│    DangerousCommandRegistry (26-50 patterns + severity)         │
│    SafeCommandWhitelist (~50 read-only commands)                │
│    DenialEscalation (consecutive + total tracking)              │
│    PathAwareEscalation (.env/secrets → auto Prompt)             │
│    StrictMode (one-way tightening, cannot weaken)               │
│    PackRuleEngine (from dcg-cli, Aho-Corasick matching)        │
│    PerToolOverrides (TOML: allow/deny/prompt per tool pattern)  │
│    NetworkPolicy (host allowlist/denylist)                      │
└─────────────────────────────────────────────────────────────────┘
```

---

## 2. Phase Breakdown

### Phase 1 — Current State (DONE ✅)

Already implemented in branch `experiment/dcg-permission-modes`:

| Item | Status | File |
|------|--------|------|
| dcg-core git dep (not local path) | ✅ Done | `Cargo.toml` |
| `--permission-mode` CLI flag (6 modes) | ✅ Done | `src/cli/args.rs` |
| `--dangerously-skip-permissions` CLI flag | ✅ Done | `src/cli/args.rs` |
| `JCODE_PERMISSION_MODE` env var support | ✅ Done | `src/cli/startup.rs` |
| dcg_bridge adapter module | ✅ Done | `src/dcg_bridge.rs` |
| BridgeDecision → ActionTier mapping | ✅ Done | `src/safety.rs` |
| Engine + Session + ProtectedPaths integration | ✅ Done | `src/dcg_bridge.rs` |
| Legacy AUTO_ALLOWED compatibility (Default/Auto modes) | ✅ Done | `src/dcg_bridge.rs` |
| Tests for Default, Plan, Bypass modes | ✅ Done | `src/dcg_bridge.rs` |
| 9-repo research document | ✅ Done | `docs/PERMISSION_MODES_RESEARCH.md` |

### Phase 2 — dcg-core Enhancements (in dcg repo)

Build these in `/data/projects/destructive_command_guard/crates/dcg-core/`:

#### 2.1 Dangerous Command Patterns [P0]

**What:** 26-50 regex patterns classifying commands by danger level, inspired by claude-code, oh-my-pi, pi-agent-rust.

**New types:**
```rust
pub struct DangerousPattern {
    pub pattern: Regex,
    pub severity: DangerSeverity,
    pub category: DangerCategory,
    pub reason: String,
    pub alternatives: Vec<String>,
}

pub enum DangerSeverity {
    Low,      // Unusual but not destructive (e.g., curl without pipe)
    Medium,   // Potentially harmful (e.g., git push --force)
    High,     // Destructive (e.g., rm -rf, sudo)
    Critical, // Irreversible/system-level (e.g., dd, mkfs, fork bomb)
}

pub enum DangerCategory {
    RecursiveDelete,
    DiskDestruction,
    ForkBomb,
    RemoteFetchAndExecute,
    PermissionEscalation,
    SystemShutdown,
    CredentialModification,
    ReverseShell,
    NetworkExfiltration,
    ForcePush,
    // extensible
}
```

**Patterns to include (from research):**
- `rm -rf /`, `rm -rf *`, `rm -rf ~`
- `sudo` chains, `chmod 777`, `chown`
- `curl | sh`, `wget | bash`, `pip install | python`
- `:(){:|:&};:` (fork bomb), `bomb()`
- `dd if=`, `mkfs`, `shred`
- `git push --force`, `git clean -fdx`
- `nc -l`, `ncat`, reverse shell patterns
- `aws s3 rm --recursive`, `gcloud compute instances delete`
- `kubectl delete namespace`, `docker system prune -a`
- `DROP TABLE`, `TRUNCATE`, `DELETE FROM` (SQL)

**Integration point:** `Engine::evaluate()` calls `DangerousPatternRegistry::check(&tool_call)` before mode-based evaluation. High/Critical severity overrides mode decision → Prompt or Deny.

#### 2.2 Safe Command Whitelist [P0]

**What:** Explicit allowlist of known-safe read-only commands that auto-approve in all modes (except DontAsk deny-listed ones).

**New types:**
```rust
pub struct SafeCommandEntry {
    pub command: &'static str,        // e.g., "git"
    pub allowed_subcommands: &'static [&'static str],  // e.g., ["status", "log", "diff", "show", "branch"]
    pub safe_flags: &'static [&'static str],           // e.g., ["--oneline", "--color"]
}

pub fn is_known_safe_command(cmd: &str) -> bool;
```

**Commands to whitelist (from codex + claude-code research):**
- `cat`, `head`, `tail`, `less`, `more`
- `ls`, `find` (safe flags only), `stat`, `pwd`, `whoami`, `which`
- `grep`, `rg`, `ag` (safe flags only)
- `git status`, `git log`, `git diff`, `git show`, `git branch` (read-only subcommands)
- `gh issue view`, `gh pr list`, `gh pr status` (read-only)
- `npm run lint`, `npm run check`, `npm run typecheck`
- `cargo check`, `cargo clippy`, `cargo test` (no --release)
- `tsc`, `eslint`, `prettier --check`
- `base64` (safe opts only), `wc`, `tr`, `cut`, `sort`, `uniq`

#### 2.3 Denial Escalation [P1]

**What:** Use existing `Session::deny_counter` to escalate after N denials. Add session-wide tracking.

**New types:**
```rust
pub struct DenialConfig {
    pub max_consecutive: u32,  // default: 3
    pub max_total: u32,        // default: 20
}

// In Session:
pub fn total_denials(&self) -> u32;
pub fn consecutive_denials(&self) -> u32;
pub fn reset_consecutive(&self);  // called on allow
```

**Behavior:** When `consecutive_denials >= max_consecutive` OR `total_denials >= max_total`, override mode decision to `Prompt` (force user interaction). Matches Claude Code behavior exactly.

#### 2.4 Path-Aware Escalation [P1]

**What:** Even in AcceptEdits mode, writing to sensitive paths triggers Prompt.

**New paths to auto-escalate (from oh-my-claudecode + research):**
```
.env, .env.*, .gitconfig, .bashrc, .zshrc, .profile
.mcp.json, .claude.json, .claude/settings.json
.ssh/, .aws/, .gnupg/
**/secrets/**, **/credentials/**
**/.env*, **/.ssh/**
```

**Integration:** Extend `ProtectedPaths` with severity levels. Some paths always Prompt (even bypass — matches Claude Code Step 1g safety checks), others Prompt only in non-bypass modes.

#### 2.5 Strict Mode / One-Way Tightening [P1]

**What:** A master strict flag that can only tighten, never relax. From oh-my-claudecode.

**New types:**
```rust
pub enum StrictnessLevel {
    Default,  // normal operation
    Strict,   // tightens: no bypass, reduced safe whitelist, lower denial limits
}

// In EngineConfig:
pub fn with_strictness(mut self, level: StrictnessLevel) -> Self;
```

**Strict mode effects:**
- Disable BypassPermissions (cannot be activated)
- Reduce max_denials to 5 (from 20)
- Restrict safe command whitelist to minimal set
- Force Prompt for all network operations
- Disable AcceptEdits auto-allow

#### 2.6 Per-Tool User Overrides (TOML) [P2]

**What:** TOML config for allow/deny/prompt per tool pattern.

**Config schema:**
```toml
[permissions]
default_mode = "default"
strict = false

[permissions.protected_paths]
always_prompt = ["~/.ssh", "~/.aws", ".git", ".env"]
always_prompt_recursive = ["**/secrets/**", "**/.ssh/**"]

[permissions.tools]
bash = "prompt"           # Always prompt for bash
edit = "allow"            # Always allow edits (overrides mode)
read = "allow"            # Always allow reads
webfetch = "prompt"       # Always prompt for network
"bash:git *" = "allow"   # Pattern-specific: allow git commands
"bash:rm *" = "deny"     # Pattern-specific: always deny rm

[permissions.denial]
max_consecutive = 3
max_total = 20

[permissions.safe_commands]
# Override safe whitelist
enabled = true
extra = ["just", "make check"]     # Add custom safe commands
deny = ["git branch -D"]           # Remove from safe list
```

**Config resolution chain:**
```
CLI flag > env var > project .jcode/config.toml > user ~/.jcode/config.toml > Engine defaults
```

#### 2.7 Pack Rule Integration [P2]

**What:** Move dcg-cli's 50+ security packs into dcg-core for direct consumption.

**Source:** `/data/projects/destructive_command_guard/crates/dcg-cli/src/packs/`

**Key components to migrate:**
- `PackRegistry` with Aho-Corasick keyword pre-filter + RegexSet batch matching
- `SafePattern` (34 whitelist regex patterns)
- `DestructivePattern` (blacklist with severity, alternatives, Tier-A effects)
- 20+ pack categories (core, database, cloud, kubernetes, containers, system...)
- Allowlist system (project `.dcg/allowlist.toml`, user, system)

#### 2.8 Network Policy [P3]

**What:** Host allowlist/denylist for network calls.

**Future consideration** — not blocking for permission modes MVP.

---

### Phase 3 — jcode Integration (in jcode repo)

#### 3.1 TUI Mode Cycling (Shift+Tab) [P0]

**What:** Runtime mode switching via keyboard shortcut.

**Cycle order:** `default → acceptEdits → plan → auto → dontAsk → bypassPermissions → default`

**Implementation:**
- Add keybinding handler in TUI event loop
- Call `dcg_bridge::set_mode(next_mode)` on cycle
- Show mode indicator in status bar with color
- Confirm dialog before entering BypassPermissions (match Claude Code)
- Update mode display in real-time

**UI mockup:**
```
┌──────────────────────────────────────────────────────┐
│ jcode v0.13.0  │ 🔒 Plan Mode  │ Claude Opus 4.8    │
├──────────────────────────────────────────────────────┤
│                                                      │
│  [Shift+Tab to change mode]                          │
│                                                      │
└──────────────────────────────────────────────────────┘
```

#### 3.2 TUI Permission Dialogs [P0]

**What:** Interactive approval/deny/always-allow for Prompt decisions.

**Dialog types:**
- Bash command: show command, Approve/Deny/Always-approve
- File edit: show diff, Approve/Deny
- Network: show URL, Approve/Deny
- Protected path: show path + warning, Approve/Deny

**Match dcg-core Decision:**
- `Decision::Allow` → auto-execute, no dialog
- `Decision::Prompt { reason, allow_once_code, alternatives }` → show dialog with reason + alternatives
- `Decision::Deny { reason, alternatives }` → show denied message with alternatives

#### 3.3 Config Loading [P1]

**What:** Load TOML config and pass to dcg-core.

**Resolution:**
```
CLI --permission-mode > JCODE_PERMISSION_MODE env > .jcode/config.toml > ~/.jcode/config.toml > Mode::Default
```

**Implementation:**
- Parse `config.toml` → dcg-core types
- `EngineConfig::builder()` with protected_paths from config
- Per-tool overrides fed to engine evaluation
- Denial config from TOML → `DenialConfig`

#### 3.4 YOLO Classifier Implementation [P2]

**What:** Build YOLO auto-approval classifier in jcode. Uses active LLM provider to classify commands as safe/unsafe.

**Why in jcode, NOT dcg-core:**
- dcg consumers (Claude Code, Codex, Gemini) use dcg as CLI hook (exit codes), not Rust library
- Only jcode links dcg-core as Cargo dependency — a Rust trait would be useless for TS/Go consumers
- YOLO needs LLM integration — consumer-specific (each agent uses different provider)
- Keeps dcg-core clean: pure rule-based engine, no LLM knowledge

**Implementation:**
- Create `src/yolo_classifier.rs` — standalone module, no dcg-core trait dependency
- Use active provider's chat completion API as subagent
- Send transcript + action → get allow/deny decision
- Respect denial limits (3 consecutive YOLO denials → stop calling YOLO, show interactive prompt)
- Two-stage: fast check (small model) → thinking check (large model) if uncertain

**Wiring in dcg_bridge:**
```rust
pub fn classify(action: &str) -> BridgeDecision {
    let mode = current_mode();
    if mode == Mode::Auto {
        // Ask YOLO first (jcode-side)
        match crate::yolo_classifier::classify(action) {
            Some(YoloDecision::Allow) => return BridgeDecision::Allow,
            Some(YoloDecision::Deny) => {
                // Track denial, fallback to prompt if limit hit
                return handle_yolo_deny(action);
            }
            None => {} // Classifier unavailable → fall through to engine
        }
    }
    // All other modes → delegate to dcg-core engine
    classify_via_dcg(action, mode)
}
```

#### 3.5 Subagent Permission Restriction [P2]

**What:** When spawning subagents (swarm/team), derive restricted permissions from parent.

**Pattern (from opencode + oh-my-openagent):**
```rust
fn derive_subagent_permissions(parent: &PermissionContext, subagent: &SubagentConfig) -> PermissionConfig {
    // Inherit parent denies + external_directory rules
    // Default deny todowrite/task for subagents
    // Force subagents to yolo mode (parent = auth boundary)
}
```

---

## 3. Dependency Map

```
Phase 1 (DONE)
    │
    ▼
Phase 2.1 Dangerous Patterns ─────┐
Phase 2.2 Safe Command Whitelist ─┤ (can run in parallel)
Phase 2.3 Denial Escalation ──────┤
Phase 2.4 Path-Aware Escalation ──┤
Phase 2.5 Strict Mode ────────────┘
    │
    ▼
Phase 3.1 TUI Mode Cycling
Phase 3.2 TUI Permission Dialogs
Phase 3.3 Config Loading
    │
    ▼
Phase 2.6 Per-Tool Overrides (TOML) ──┐
Phase 2.7 Pack Rule Integration ──────┤ (dcg-core, can run in parallel)
Phase 3.4 YOLO Classifier (jcode) ────┘ ← jcode-only, NOT dcg-core
    │
    ▼
Phase 3.5 Subagent Permissions
    │
    ▼
Phase 4 — MCP Permissions (future)
```

---

## 4. Open Questions (still need discussion)

| # | Question | Why it matters | Options |
|---|----------|----------------|---------|
| ~~1~~ | ~~YOLO: trait in dcg-core vs jcode?~~ | ~~RESOLVED~~ | ✅ **YOLO in jcode only.** dcg consumers use CLI hooks (exit codes), not Rust library. Only jcode links dcg-core. A Rust trait in dcg-core is useless for TS/Go consumers. YOLO needs LLM — consumer-specific. |
| ~~2~~ | ~~YOLO: which LLM provider?~~ | ~~RESOLVED~~ | ✅ **Reuse active provider** (Claude/Gemini/OpenAI — zero extra cost). 2-stage: fast (64 tokens) + thinking (4096 tokens). Fail closed. Inspired by claude-code `yoloClassifier.ts`. |
| ~~3~~ | ~~MCP permissions: unified or separate?~~ | ~~RESOLVED~~ | ✅ **Unified pipeline**. MCP tools map `mcp__server__tool` → `Engine::evaluate()`. Unknown tools default = Prompt. Per-server config in TOML. Inspired by claude-code 3-level matching (`mcp__server`, `mcp__server__*`, `mcp__server__tool`). |
| ~~4~~ | ~~Sandboxing future?~~ | ~~NOT DOING~~ | ❌ **App-level only**, no OS sandbox. codex proves OS-level is gold standard but too complex for now. |
| ~~5~~ | ~~Multi-agent/swarm permission inheritance?~~ | ~~RESOLVED~~ | ✅ **Option A: inherit + restrict** (opencode pattern). (1) Inherit parent deny rules (session + agent), (2) Default deny recursive capabilities (subagent spawn, todowrite), (3) Children can do LESS, never MORE. claude-code adds static deny-list (no Agent nesting, no AskUser, no vault) + worker→leader escalation. |

---

## 5. Success Criteria

- [ ] All 6 permission modes work end-to-end via CLI flag + env var + config
- [ ] Mode cycling works in TUI with Shift+Tab
- [ ] TUI permission dialogs show for Prompt decisions
- [ ] Dangerous commands detected and blocked/prompted in all modes
- [ ] Safe commands auto-approve in Plan/Default/AcceptEdits modes
- [ ] Denial tracking: 3 consecutive / 20 total → fallback to prompt
- [ ] Protected paths always prompt even in AcceptEdits
- [ ] Strict mode available and one-way tightening
- [ ] TOML config overrides work per-tool and per-pattern
- [ ] YOLO classifier: 2-stage (fast 64t + thinking 4096t), reuse active provider, fail closed, 3-consecutive circuit breaker
- [ ] MCP tools go through unified permission pipeline (`mcp__server__tool` → Engine::evaluate), unknown = Prompt
- [ ] Subagent permissions: inherit parent deny rules + default deny recursive capabilities
- [ ] dcg-core dependency via git URL (not local path, not crates.io)
- [ ] `cargo check` passes with zero errors
- [ ] Test coverage for all modes + edge cases
