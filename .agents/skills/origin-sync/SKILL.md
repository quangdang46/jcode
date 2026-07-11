# Origin Sync: Fork Sync Skill for jcode (quangdang46/jcode)

## Problem

This repo is a **fork** of `1jehuang/jcode`. Multiple modules have been extracted into separate repos under `github.com/quangdang46/*`. When syncing from upstream, conflicts inevitably arise where upstream changes collide with code that was replaced by external repo references.

The naive approach (`git merge` + resolve everything by hand) breaks because:
- Upstream may modify code that now lives in an external repo → taking upstream's change **reverts the extraction**
- Upstream may modify adapter code (e.g., `casr_adapter.rs`, `dcg_bridge.rs`) — need to check if the external repo already handles this
- Upstream may add features that **should** be extracted but aren't yet → need to redirect

## Core Principle

> **The fork's code is the decorated state. Upstream changes that touch extracted domains must be redirected to the external repos, not reverted.**

---

## Extracted Module Map

| # | Cargo Dep Name | External Repo | Local Adapter/Bridge Files | Domain |
|---|---|---|---|---|
| 1 | `casr` | `quangdang46/cross_agent_session_resumer` | `crates/jcode-base/src/casr_adapter.rs`, `crates/jcode-base/src/import.rs` | Session import/resume (replaces `jcode-import-core`) |
| 2 | `ffs-search`, `ffs-engine`, `ffs-symbol` | `quangdang46/fast_file_search` | `crates/jcode-tui/src/tui/app/at_picker.rs` (uses `ffs_engine::mention` and `ffs_search::mention`) | File search, @-mention autocomplete, symbols |
| 3 | `dcg-core` | `quangdang46/destructive_command_guard` | `crates/jcode-app-core/src/dcg_bridge.rs` | Permission guard, YOLO classifier |
| 4 | `hashline` | `quangdang46/hashline` | `crates/jcode-app-core/src/tool/hashline_edit.rs` | SHA-256 anchored hashing |
| 5 | `mempalace-core` | `quangdang46/mempalace_rust` | `crates/jcode-mempalace-adapter/` (entire crate) | Memory palace |
| 6 | `dynamic_context_pruning` | `quangdang46/dynamic_context_pruning` | `crates/jcode-app-core/src/dcp_bridge.rs`, `dcp_plugin.rs` | Context pruning |
| 7 | `rtco-core` | `quangdang46/rust_token_cost_optimizer` | `crates/jcode-app-core/src/rtco_filter.rs` | Token cost optimization |

### Additional git dependencies

| Dep | Owner | Domain |
|---|---|---|
| `agentgrep` | `1jehuang/agentgrep` | Code search (not extracted, from upstream contributor) |
| `beads_rust` | `quangdang46/beads_rust` | Issue/bead tracking integration (preserve on `Cargo.toml` conflicts) |

### Not-yet-extracted domains that commonly conflict

- `crates/jcode-tui/src/tui/app/inline_interactive.rs` — session picker, resume logic (uses `casr` through `import.rs`)
- `crates/jcode-session-types/src/lib.rs` — `ResumeTarget` enum (has `ForeignSession` variant added locally)
- `src/cli/tui_launch.rs` — terminal launch (uses `casr_adapter` heavily)
- `crates/jcode-tui/src/tui/session_picker/` — session picker UI (has `ForeignSession` arms added locally)
- `crates/jcode-app-core/src/yolo_classifier.rs` — DCG integration
- `crates/jcode-tui/src/tui/app/state_ui_input_helpers.rs` — slash commands (`/permissions`, `/models`), `$<skill>`, `@<file>` autocomplete, FFS tool rename
- `crates/jcode-tui/src/tui/app/state_ui.rs` — `/skills` report, `/permissions` handler
- `crates/jcode-tui/src/tui/ui_overlays.rs` — help overlay entries
- `crates/jcode-base/src/safety.rs` — AUTO_ALLOWED list with `ffs *` entries
- `crates/jcode-base/src/skill.rs` — `parse_invocation` with `$<name>` instead of `/<name>`
- `Cargo.toml` — `mempalace-backend` feature, `jcode-app-core` dep
- `crates/jcode-app-core/src/tool/mod.rs` — tool registration names + module declarations
- `crates/jcode-app-core/src/dcg_bridge.rs` — `READ_ONLY_ACTIONS` with FFS tools, `mode_to_str`
- `crates/jcode-tui/src/tui/app/at_picker.rs` — `@` mention picker
- `crates/jcode-base/src/prompt.rs` — system prompt with `$skillname`

---

## Sync Workflow

### Step 0: Prerequisites

```bash
# Add upstream remote if missing
git remote add upstream https://github.com/1jehuang/jcode.git

# Verify upstream
git remote -v
# Should show:
#   origin    https://github.com/quangdang46/jcode.git (fetch/push)
#   upstream  https://github.com/1jehuang/jcode.git (fetch)

# Ensure master is clean
git checkout master
git status  # should be clean
```

### Step 1: Fetch upstream

```bash
git fetch upstream
```

### Step 2: Review upstream changes (before merge)

```bash
# See what's new from upstream since our last sync
git log --oneline master..upstream/master

# Check which files changed
git diff --stat master..upstream/master

# *** CRITICAL ***: Check if upstream has changed ANY file we've modified locally.
# Auto-merge can silently overwrite our changes if there's no textual conflict.
# This includes extracted-domain files AND all local customizations.
# Use git log to find all files we've modified on master since fork:
LOCAL_FILES=$(git log --all --since="2026-01-01" --diff-filter=M --format="" --name-only -- \
  '*.rs' '*.toml' '*.json' '*.sh' '*.md' \
  | sort -u | grep -v 'target/' | grep -v '.worktrees/' | head -200)
UPSTREAM_FILES=$(git diff --name-only master..upstream/master)
# Files that changed both locally AND upstream — these need attention
COMMON_FILES=$(comm -12 <(echo "$UPSTREAM_FILES" | sort) <(echo "$LOCAL_FILES" | sort))
if [ -n "$COMMON_FILES" ]; then
  echo "=== Files changed by upstream that we've also modified locally ==="
  echo "$COMMON_FILES"
  echo "=== These risk silent overwrite. Review each after merge. ==="
fi
# Also check the known extracted-domain files explicitly
git diff --stat master..upstream/master -- $UPSTREAM_FILES
```

### Step 2.5: Hunk-level diff analysis (critical)

Before merging, check every COMMON_FILE at the hunk level.
Auto-merge produces no conflict markers when upstream modified
different lines than we did — but our changes can still be
semantically broken or partially overwritten.

```bash
# For each file in COMMON_FILES from Step 2:
echo "$COMMON_FILES" | while IFS= read -r file; do
  echo "=== $file ==="
  # Show our local changes (compared to what we merged last)
  echo "-- Our changes (HEAD):"
  git show HEAD:"$file" | diff - <(git show upstream/master:"$file") 2>/dev/null || true
  
  # Show upstream's changes (compared to merge-base)
  MERGE_BASE=$(git merge-base HEAD upstream/master)
  echo "-- Upstream changes (merge-base..upstream/master):"
  git diff "$MERGE_BASE..upstream/master" -- "$file" | head -80
  
  # Interactive check: does our local addition survive upstream's diff?
  echo "-- Confirm each of OUR hunks still applies cleanly:"
  git log --all -1 --format="%H" -- "$file"  # last commit touching this file
  echo ""
done
```

**What to look for per hunk**:
1. **Our addition vs upstream deletion**: Upstream deleted a function we added to → **needs restore** (Category A/B resolution)
2. **Both added code near each other**: Upstream added code adjacent to ours → may need reordering (Category B)
3. **Upstream refactored around our code**: Upstream renamed symbols/types our code depends on → **our code now references dead names** (Category B — incorporate both)
4. **Our enum variant vs upstream's enum**: Upstream added new variants to the same enum we extended → need to keep both (Category B sub-type)

**Automatic sanity check**: run this before merging to flag likely breaks:

```bash
echo "$COMMON_FILES" | while IFS= read -r file; do
  # Try a dry-run 3-way merge to see what auto-merge would do
  # (this is what `git merge` will do internally)
  MERGE_BASE=$(git merge-base HEAD upstream/master)
  git merge-file -p \
    <(git show HEAD:"$file") \
    <(git show "$MERGE_BASE":"$file") \
    <(git show upstream/master:"$file") \
    2>/dev/null | diff - <(git show HEAD:"$file") | head -30 && \
    echo "  ^ $file: auto-merge preserves HEAD (good)" || \
    echo "  ^ $file: auto-merge may overwrite HEAD (check!)"
done
```

### Step 3: Merge

```bash
git merge upstream/master
```

**Do NOT use `git rebase`** for upstream sync — it rewrites history for the entire fork, making it impossible for collaborators to sync. Use `git merge`.

### Step 4: Conflict Classification & Resolution

For EACH conflicted file, classify the conflict:

#### Category A: Extracted Code Conflict

**Symptom**: Upstream changed code that was replaced by an external repo (casr, ffs, dcg, hashline, mempalace, dcp, rtco).

**Resolution**: **KEEP OUR VERSION. Discard upstream changes entirely.**

```bash
git checkout --ours -- <file>
git add <file>
```

**Reasoning**: The upstream's inline implementation was superseded by the external crate. Taking upstream changes would reintroduce the old inline code and break the dep chain.

**Exception**: If the upstream change also modifies the local adapter/bridge code in a way that's compatible with the external repo, check if the external repo already has equivalent support. If yes → forward-port the change to the external repo as a separate PR. If no → ask user.

#### Category B: Local Extension Conflict

**Symptom**: Conflict in files that have local additions not present upstream (e.g., `ForeignSession` variant in `ResumeTarget`, extra match arms).

**Resolution**: **KEEP OUR VERSION** for the local additions, but **INCORPORATE UPSTREAM'S** changes to shared code where they don't conflict.

```bash
# For each conflict hunk:
# - If the conflict is entirely about our local additions → keep ours
# - If upstream added something non-overlapping → incorporate both
# - If upstream changed the same area → need manual merge
```

**Sub-types**:
- **New enum variants added both sides** → keep both. This requires manual editing.
- **New match arms both sides** → keep both. Manual editing.
- **Upstream refactored the module structure** → compare carefully. If upstream moved code that references extracted deps, our paths need to stay.

#### Auto-Resolution: Scripted 3-way merge for Category B/F

Use this to **automatically resolve** Category B conflicts and Category F silent overwrites.
The strategy: start from OUR HEAD, then incorporate upstream's additions that don't conflict
with ours (trust ours on overlap).

The script AUTO-DISCOVERS which files to resolve — no hardcoded lists.

```bash
# Requires: MERGE_BASE, UPSTREAM_BRANCH (e.g. upstream/master)
# Run this AFTER `git merge` completes (all auto-merge done).

MERGE_BASE=$(git merge-base HEAD "$UPSTREAM_BRANCH")
CAT_A_FILES=$(grep -l 'ddg\|dcp\|hashline\|casr\|rtco\|mempalace\|ffs_' \
  <(git diff --name-only "$MERGE_BASE..HEAD" -- '*.rs' 2>/dev/null) 2>/dev/null || echo "")

echo "=== Auto-resolving Category A (extracted domains) — keeping OURS ==="
for file in $CAT_A_FILES; do
  if git ls-files --unmerged "$file" | grep -q . 2>/dev/null; then
    git checkout --ours -- "$file"
    git add "$file"
    echo "  ✓ $file (conflict → kept ours)"
  fi
done

echo "=== Auto-resolving Category B/F (3-way merge with --ours preference) ==="
# Discover ALL common files: upstream changed AND we also changed
# These are all Category B or F candidates
COMMON_FILES=$(comm -12 \
  <(git diff --name-only "$MERGE_BASE..$UPSTREAM_BRANCH" | sort -u) \
  <(git diff --name-only "$MERGE_BASE..HEAD" | sort -u) \
  2>/dev/null)

for file in $COMMON_FILES; do
  # Skip files we already handled as Category A
  if echo "$CAT_A_FILES" | grep -Fxq "$file"; then
    continue
  fi
  if [ ! -f "$file" ]; then continue; fi
  
  # Has conflict markers from auto-merge?
  if git ls-files --unmerged "$file" | grep -q . 2>/dev/null; then
    echo "  🔄 $file (conflict — 3-way merge with --ours)"
  else
    # Category F silent overwrite — check if HEAD changed
    cp "$file" "${file}.check"
    git show HEAD:"$file" > "${file}.head" 2>/dev/null || continue
    if diff -q "${file}.check" "${file}.head" >/dev/null 2>&1; then
      rm -f "${file}.check" "${file}.head"
      continue  # no change — safe
    fi
    echo "  ⚠ $file (silent overwrite — restoring ours + upstream non-overlap)"
    rm -f "${file}.check" "${file}.head"
  fi
  
  # 3-way merge: ours preferred, upstream non-overlap incorporated
  cp "$file" "${file}.ours"
  git show "$UPSTREAM_BRANCH":"$file" > "${file}.theirs" 2>/dev/null || continue
  git show "$MERGE_BASE":"$file" > "${file}.base" 2>/dev/null || continue
  
  if [ -s "${file}.base" ] && [ -s "${file}.theirs" ]; then
    git merge-file --ours -p \
      "${file}.ours" \
      "${file}.base" \
      "${file}.theirs" > "$file" 2>/dev/null || cp "${file}.ours" "$file"
    
    # Ensure file isn't empty
    if [ ! -s "$file" ]; then
      cp "${file}.ours" "$file"
    fi
    git add "$file"
    echo "    → merged"
  fi
  rm -f "${file}.ours" "${file}.theirs" "${file}.base"
done

echo "=== Verification ==="
cargo check 2>&1 | tail -10
```

This replaces the old hardcoded file list with dynamic discovery via:
- `CAT_A_FILES`: found by grepping diff for extracted domain keywords
- `COMMON_FILES`: `comm -12` of upstream changes vs our changes
- Category B detected by `git ls-files --unmerged` (has conflict markers)
- Category F detected by comparing working tree against HEAD (silently changed)
- Both resolved via `git merge-file --ours`

#### Category C: Upstream-Only Change (no conflict)

**Symptom**: Upstream added/modified code that doesn't touch any extracted domain.

**Resolution**: **ACCEPT UPSTREAM** changes as-is.

```bash
git add <file>  # after the auto-merge stage already handled this
```

#### Category D: Third-Party Dep Change

**Symptom**: Upstream changed `Cargo.toml` or `Cargo.lock` — adding, removing, or updating dependencies.

**Resolution**: **CAREFUL MERGE**. Our `Cargo.toml` has git deps that upstream doesn't have. Preserve all `[dependencies]` entries for extracted repos (casr, ffs-search, ffs-engine, ffs-symbol, dcg-core, hashline, mempalace-core, dynamic_context_pruning, rtco-core, beads_rust). For everything else, accept upstream's version.

**Always run `cargo check` after resolving Cargo.toml conflicts.**

#### Category E: New Upstream Feature That Should Be Extracted

**Symptom**: Upstream added a feature that semantically belongs in one of the extracted repos (e.g., new session import format → should be in casr; new file search feature → should be in fast_file_search).

**Resolution**: **DO NOT implement the feature inline**. Instead:
1. Add a `FIXME`/`TODO` comment in the merge commit marking the gap
2. Create an issue in the corresponding external repo
3. Implement it in the external repo
4. Bump the dependency revision
5. Wire it through the adapter layer

#### Category F: Silent Overwrite Risk

**Symptom**: Upstream changed a file that we've modified locally, and the merge auto-resolved WITHOUT conflict. No conflict markers, but our local changes are gone.

**Why this happens**: Git's auto-merge only produces conflict markers when both sides modified the same region. If upstream modified lines far from our local changes, auto-merge silently accepts both — but on the next `cargo check` our changes may no longer apply correctly, or runtime behavior regresses.

**Resolution**: After merge, BEFORE verification:

```bash
# List files that upstream changed that we also modified
# These are the highest risk for silent overwrite
git diff --name-only HEAD..origin/master -- $(comm -12 \
  <(git diff --name-only master..upstream/master | sort) \
  <(git log --all --since="2026-01-01" --diff-filter=M --format="" --name-only -- '*.rs' '*.toml' | sort -u)
)

# Check each one. If a file has unexpected differences (our local
# additions missing), restore from origin/master and re-apply:
git show origin/master:<file> > <file>  # get our version
# Then manually merge in any upstream additions that don't conflict
```

**Common files with silent overwrite risk** (checked 2026-06; freeze lesson 2026-07):
- `crates/jcode-tui/src/tui/app/turn.rs` — **NO `biased;` before `event_stream`** (freeze on alt-tab; Lesson 2)
- `crates/jcode-tui/src/tui/app/local.rs`, `run_shell.rs` — event-loop keep-ours (Lesson 1)
- `crates/jcode-tui/src/tui/app/state_ui_input_helpers.rs` — slash commands, FFS rename, `$`/`@` autocomplete
- `crates/jcode-tui/src/tui/app/state_ui.rs` — `/permissions`, `/skills` report
- `crates/jcode-tui/src/tui/ui_overlays.rs` — help entries
- `crates/jcode-base/src/safety.rs` — AUTO_ALLOWED list (FFS tools)
- `crates/jcode-base/src/config.rs` — tool-profile allow lists
- `crates/jcode-base/src/prompt.rs` — system prompt with `$skillname`
- `crates/jcode-base/src/skill.rs` — `parse_invocation` using `$<name>`
- `Cargo.toml` — mempalace-backend feature, jcode-app-core dep
- `crates/jcode-app-core/src/dcg_bridge.rs` — READ_ONLY_ACTIONS, mode helpers
- `crates/jcode-app-core/src/tool/mod.rs` — tool registrations, module declarations
- `crates/jcode-tui/src/tui/app/at_picker.rs` — `@` mention picker
- `crates/jcode-tui/src/tui/app/input.rs` — lazy-init @ picker
- `crates/jcode-tui/src/tui/ui_tools.rs` — tool summary display arms
- `crates/jcode-tui/src/tui/app/tui_lifecycle.rs` — App constructor field
- `crates/jcode-desktop/src/single_session.rs` — tool name match arms
- `crates/jcode-provider-core/src/anthropic.rs` — tool name mapping
- `crates/jcode-base/src/provider/Codex.rs` — tool name mapping
- `crates/jcode-usage-types/src/lib.rs` — telemetry category arms
- `crates/jcode-tui-tool-display/src/lib.rs` — resolve_display_tool_name
- `crates/jcode-tool-types/src/lib.rs` — resolve_tool_name

### Step 5: Verification

```bash
# After all conflicts resolved:
git diff --cached --stat  # review staged changes

# Build must pass
cargo check 2>&1

# Run tests
cargo test 2>&1 | tail -20

# Format check
cargo fmt --all --check 2>&1

# If CI is critical, also run:
# scripts/test_fast.sh
```

### Step 6: Commit & Push

```bash
# The merge creates a commit automatically after all conflicts resolved
# Verify the merge commit message and push
git log --oneline -3
git push origin master
```

---

## Quick Reference: Common Conflict Patterns

### Pattern 1: `ResumeTarget` enum (jcode-session-types)

**Upstream** has: `CodexSession, PiSession, OpenCodeSession`
**Our fork** also has: `ForeignSession { provider_slug, session_id }`

**Resolution**: Keep ForeignSession variant. Add any new variants upstream added.

### Pattern 2: `resolve_resume_target_to_jcode` / `imported_session_id_for_target`

**Upstream** uses inline import logic. **Our fork** delegates to `casr_adapter`.

**Resolution**: Keep our version entirely (`git checkout --ours`).

### Pattern 3: Session picker match arms

**Upstream** matches known providers. **Our fork** has additional `ForeignSession` arms.

**Resolution**: Keep our version with the extra arms, add any new upstream provider arms.

### Pattern 4: `Cargo.toml` dependency changes

**Resolution**: Keep all `github.com/quangdang46/*` deps. Accept upstream dep changes for everything else.

### Pattern 5: Worktree submodule changes

If `.worktrees/` directories show up in `git status` as modified, do NOT stage them. They are managed separately.

```bash
git checkout -- .worktrees/  # restore worktree submodule pointers
```

---

## Pre-Merge Review Checklist

Before starting the merge, complete this checklist:

- [ ] `git fetch upstream` successful
- [ ] `git log master..upstream/master` reviewed for scope
- [ ] No pending local changes (working tree clean)
- [ ] Upstream changes in extracted-domain files identified (see Step 2)
- [ ] Upstream changes in locally-modified files identified (see Step 2 — dynamic file list)
- [ ] External repos' latest status checked (do they have the feature upstream is modifying?)

---

## Post-Merge Audit

After push, verify:

- [ ] `cargo check` passes
- [ ] Local builds work (`cargo build`)
- [ ] Category F check done: all locally-modified files that upstream touched were audited for silent overwrite
- [ ] **Lesson 2**: `rg -n '^\s*biased\s*;' crates/jcode-tui/src/tui/app/turn.rs` is empty (no freeze-on-alt-tab regression; ignore comments)
- [ ] `ffs`/`$`/`@`/`/permissions` features still working
- [ ] `Cargo.toml` has our feature flags (`mempalace-backend`, `dcp`, `rtco`)
- [ ] Extracted features still functional (session resume, file picker, DCG mode)
- [ ] No files from extracted repos left inline (if upstream added new inline code in extracted domains, flag it)
- [ ] Worktrees unaffected (checkout each worktree and run cargo check there too)

---

## Troubleshooting

### "Upstream added a new provider to session picker"

This is the most common conflict. Upstream adds a new session provider → our `ResumeTarget`, `import.rs`, `casr_adapter.rs`, `inline_interactive.rs`, `tui_launch.rs`, `session_picker.rs` all need new arms.

**Process**:
1. Note which new provider upstream added
2. Check if `casr` (cross_agent_session_resumer) already supports this provider
   - If YES → add the provider to `ResumeTarget` and wire it through `import.rs` using `casr_adapter::*`
   - If NO → implement support in `casr` repo first, then bump dep rev, then wire it here
3. Add match arms in all relevant files mirroring upstream's pattern but using our adapter functions

### "Both sides added code to the same function"

Manual intervention needed. Open the file and:
1. Identify which parts are ours (adapter calls, ForeignSession, etc.)
2. Identify what upstream added (new features, refactors)
3. Merge the two, keeping our extraction logic, accepting non-overlapping upstream features

### "Cargo.lock conflict"

This is normal for any dep change. Accept ours for git deps, accept theirs for crates.io deps. If unsure, resolve by:
```bash
git checkout --ours Cargo.lock
# This is usually safe since cargo update will fix stale entries
```
Then run `cargo generate-lockfile` or just `cargo check`.

---

## Repo Status Snapshot (as of last update)

- **Fork**: quangdang46/jcode
- **Upstream**: 1jehuang/jcode
- **Status**: diverged (regenerate counts with `git rev-list --left-right --count master...upstream/master`)
- **Extracted repos**: 7 (casr, ffs, dcg, hashline, mempalace, dcp, rtco)
- **Adapter code**: ~2687 lines across 4+ bridge files

### Category G: Upstream Struct Field Addition (Silent Dependency Break)

**Symptom**: Upstream added a field to a struct that both sides reference. Our fork's `git checkout --ours` keeps the struct *definition* without the new field, but other files that auto-merged from upstream (or `--theirs` resolved) already reference that field. Build fails with `missing field` or `no field named X` even though the merge had **no textual conflict**.

**Root cause**: The conflict was in a *different* file. Upstream's struct change auto-merged cleanly into our struct definition file because that file had no merge conflict — but the `--ours` resolution for a *different conflict* reverted the struct change.

OR: The struct is in a Category A (extracted) or Category B (fork-modified) file that we kept ours, while dependent code auto-merged from upstream uses the new field.

**Detection**: Run `cargo check` after merge. If `no field named` errors point to a struct whose definition you kept ours, this is Category G.

**Resolution: Accept upstream's struct changes, keep our struct initializers updated.**

1. **Check what upstream added**: `git diff HEAD..upstream/master -- path/to/struct.rs`
2. **Apply struct field addition ONLY** — NOT all upstream changes, just the field(s):
   - Add the field definition (with `#[serde(default)]` if present)
3. **Update all struct initializers** in the fork's code to include the new field:
   - Category A/B files: add `field: default_value,`
   - Search: `grep -rn 'StructName {' --include='*.rs' crates/`
   - Add default value to every initializer

**Example**: Upstream adds `embedding_model: Option<String>` to `MemoryEntry`. Our fork kept the old struct. Dependent code auto-merged uses it. Fix:
```bash
# 1. Add field definition
sed -i '/pub confidence: f32/i\    pub embedding_model: Option<String>,' struct.rs
# 2. Find & fix every initializer
grep -rn 'MemoryEntry {' --include='*.rs' | grep -v 'fn\|pub struct'
# Add embedding_model: None, to each
```

**Why `--ours` is wrong here**: The struct field is a *schema change*, not a code customization. It's safe to accept upstream's change because:
- It doesn't override any of our custom logic
- It's a data-carrying field (serialized with `#[serde(default)]`)
- Without it, dependent code breaks at compile time

**Prevention**: After merge, always run `cargo check` and grep for `missing field` / `no field named` errors before declaring merge complete. These are Category G signals.

---

## Sync Log

### 2026-06-16 — v0.29.0 upstream sync (commit ff54caec)
**Files**: 201 changed (+14848 -2850). **123 commits**. **Conflicts resolved**: 31 files.

| Category | Area | Description |
|----------|------|-------------|
| 🔧 Refactor | **Anthropic 404 fallback** | `auth/lifecycle.rs` +644 -21, `provider/anthropic.rs` +347 -67 |
| ✨ Feature | **Memory hybrid retrieval** | `find_similar_hybrid` (BM25 + RRF), `memory_rerank.rs` (+475), `embedding_backend.rs` (+120) |
| ✨ Feature | **Sidecar all providers** | `SidecarBackend::Provider`, `ACTIVE_PROVIDER` static + `active_provider_fork()` |
| 🐛 Bugfix | **Skill sort** | `skill.rs` `list()` sorts by name for deterministic KV cache — cherry-picked into ours |
| 🐛 Bugfix | **Memory model tagging** | `MemoryEntry.embedding_model` field |
| 🐛 Bugfix | **Swarm Inline** | `comm_session.rs` — Inline = Headless, `output_tail` field |
| 🏗️ Build | **CI improvements** | Pre-install rust-src, pin RUSTUP_TOOLCHAIN=stable |
| 🏗️ Build | **Version bump** | v0.29.0 |

### 2026-06-15 — v0.28.0 upstream sync (commit 5bc6cf6b)
**Files**: 85 changed (+5415 -278). **42 commits**. **Conflicts resolved**: 26 files.

| Category | Area | Description |
|----------|------|-------------|
| ✨ Feature | **Swarm Inline Gallery** | `SwarmSpawnMode::Inline`, `output_tail`, live viewport |
| 🐛 Bugfix | **Reload signal dedup** | Stale signal ignore on startup |
| 🐛 Bugfix | **Browser focus** | `#[cfg()]` simplification |
| 🐛 Bugfix | **CI race** | Replace sccache with rust-cache |
| 🏗️ Build | **Version bump** | v0.28.0 |

### 2026-06-14 — v0.27.0 upstream sync (commit 9a3faf37)
**Files**: 146 changed (+6249 -193). **Conflicts resolved**: 26 files.

| Category | Area | Description |
|----------|------|-------------|
| 🔧 Refactor | **Auth lifecycle** | `lifecycle.rs` rewrite (+287 -14) |
| ✨ Feature | **Swarm Inline Gallery** | `SwarmSpawnMode::Inline`, live viewport tiles |
| ✨ Feature | **Anthropic 404 Fallback** | Retry/fallback logic |
| ✨ Feature | **CrossEncoder reranker** | `jcode-embedding` crate, memory refactor |
| 🐛 Bugfix | **CI** | Pin `RUSTUP_TOOLCHAIN=stable`, pre-install `rust-src` |
| 🏗️ Build | **Version bump** | v0.27.0 |

---

## Category Definitions (Summary)

### Category H: Blind Keep Ours Trap (NEW)

**Symptom**: After resolving all conflicts with `--ours`, the fork works — but upstream's fix for a real bug was silently discarded.

**Root cause**: The conflict was in a file where BOTH sides made meaningful changes. `git checkout --ours` discards the ENTIRE upstream change, including bugfixes mixed in with conflicting lines.

**Rule**: When a file conflicts, NEVER blindly keep ours. Instead:

1. **Identify what upstream changed**: `git diff HEAD..upstream/master -- conflicted/file.rs`
2. **Categorize** each hunk:
   - "Upstream refactored around our code" → keep ours, manually apply bugfix hunks
   - "Upstream added feature we don't have" → evaluate: useful? Then cherry-pick
   - "Upstream fixed a bug" → merge the fix into our code, preserving our logic
   - "Upstream reverted our improvement" → keep ours (our improvement is correct)
3. **Apply with 3-way merge**: 
   ```bash
   # Instead of checkout --ours:
   git merge-file --ours -p ours.rs base.rs theirs.rs > merged.rs
   # Then manually review and re-add any upstream bugfix hunks
   ```
4. **Verify**: `cargo check` must pass. If Category G (struct field) errors appear, fix them.

**Key insight**: An upstream hunk that differs from ours is NOT automatically wrong. Read it. If it fixes a bug we also have, the fix belongs in our code regardless of who wrote it.

**Example**: Upstream fixes `openrouter.rs` to handle a null-pointer crash. Our file conflicts because we also modified the same function. Using `--ours` keeps our code crash-free, but drops upstream's fix for the OTHER crash path that we also have. The correct action: keep our logic, apply upstream's null-check manually.

### Lesson 3: Server model persist dropped by upstream merge (Category H)

**Recorded**: 2026-07-11. **Symptom**: `/model` / model picker switch works in-session, but **restart loses the model** (always back to provider default).

**Root cause**: Fork fixes `1b71a6cd7` / `f84b9e823` / `a20aca1bf` wrote `Config::set_default_model` inside server `apply_set_model` / `apply_set_route` (`provider_control.rs`). Remote/daemon switches go through the **server** protocol, so client-only persist never runs.

Merge **`27731574a`** (`sync-final-merge` ← upstream) auto-merged `provider_control.rs` and **deleted** the `[SERVER-SAVE]` / `[ROUTE-SAVE]` blocks (upstream never had them). No conflict markers — pure Category H silent drop of a fork bugfix.

**Detection**:
```bash
rg -n 'SERVER-SAVE|ROUTE-SAVE|set_default_model' crates/jcode-app-core/src/server/provider_control.rs
# expect: set_default_model calls after successful set_model / set_route_selection
```

**Resolution**: Restore both persist blocks with provider_key (see HEAD `provider_control.rs`). On every sync that touches that file, re-check the SAVE logs still exist.

**Related keep paths** (still present — do not drop):
- Client `/model`: `model_context.rs` → `Config::set_default_model` + `[SAVE]` log
- Picker local apply: `inline_interactive.rs` → `Config::set_default_model` + `[PICKER-SAVE]`
- Ctrl+D default: `inline_interactive.rs` → `set_default_model`
- Restart restore: `Agent::new` + `tui_lifecycle` read `config.provider.default_model`

### Lesson 2: `biased;` in TUI `select!` freezes the screen (alt-tab / unfocus)

**Recorded**: 2026-07 (sync-nov-commits). **Severity**: user-visible freeze.

**Bug**: In `crates/jcode-tui/src/tui/app/turn.rs`, a `tokio::select!` over the live turn loop had:

```rust
tokio::select! {
    biased;
    // ready tool/API futures first …
    event = event_stream.next() => { /* keys, focus, resize */ }
}
```

With `biased;`, branches are polled in order. When a tool future or stream is continuously ready (or re-ready every poll), `event_stream.next()` is **starved**. Result: TUI stops handling keyboard / focus / redraw during long turns — classic freeze on **alt-tab** or unfocus/refocus while a turn is running.

**Fix (fork)**: **Remove `biased;`** from TUI event-loop `select!`s that also wait on `event_stream`. Fair polling lets input/focus land.

**Upstream still has `biased;`** in `turn.rs` (as of v0.41). Future merges that touch `turn.rs` will try to reintroduce it.

**Resolution on every sync**:
1. Treat `crates/jcode-tui/src/tui/app/turn.rs` as **Lesson 1 + Lesson 2** keep-ours for the event loop.
2. After merge, run:
   ```bash
   # Match the statement only (comments may mention biased intentionally)
   rg -n '^\s*biased\s*;' crates/jcode-tui/src/tui/app/turn.rs
   # expect: no matches
   ```
3. If a conflict or auto-merge brings `biased;` back before `event_stream.next()`, **delete it again**. Do not "accept upstream" on that hunk.
4. Optional: also prefer redraw/timer branches ahead of pure stream arms if freezes return (see commit `6862310fa` history: redraw tick ordering).

**Not the same as every `biased;` in the repo** — server/client bus prioritization and tool Alt+B selects may keep `biased;` when they intentionally prioritize one non-UI arm. Only the **TUI event_stream loop** is the freeze bug.

---

## Sync Log

### 2026-07-05 — v0.35.0..v0.36.0 upstream sync (commit 5a2437c7c)
**Branch**: `sync-final-merge` → PR #478
**Upstream commits**: 164 (v0.35.0..v0.36.0). **Files merged**: 261 (+30,369 / -15,264).

**Status**: ✅ Build clean. `cargo check -p jcode-tui` = 0 errors.

| Category | What |
|----------|------|
| ⚡ Swarm | PTY debug, turn cancel registry, per-spawn model/effort, salvage stranded, double-assignment reject, TLDR collapse, run_plan fixes, agent sorting |
| 🖥️ Server | Corrupt journal recovery, history scoping, power inhibition |
| 🏎️ Performance | Memory profiling, glibc MMAP, heap release |
| 🧪 CI | Wildcard re-export, security preflight, memory probe |
| 📝 Mermaid | Layout-tier cache, aspect ratio, resize probes |
| 🐛 Bugfixes | Copy badge, auto-poke, History dedup, /login guide |

**Kept OURS** (Lesson 1 — status bar/event loop):
- `crates/jcode-tui/src/tui/ui_input.rs`, `ui.rs`, `mod.rs`
- `crates/jcode-tui/src/tui/app/turn.rs`, `local.rs`, `run_shell.rs`, `state_ui.rs`

**Kept OURS** (Category A — extracted modules):
- `crates/jcode-base/src/import.rs`, `casr_adapter.rs`
- `crates/jcode-app-core/src/dcg_bridge.rs`, `hashline_edit.rs`
- `crates/jcode-app-core/src/tool/mod.rs` (ffs replacement)

**Kept OURS** (fork-specific crates — user maintains):
- `jcode-hooks/`, `jcode-plugin-core/`, `jcode-plugin-runtime/`
- `jcode-best-of-n/`, `jcode-llm-core/`, `jcode-keywords/`
- `jcode-provider-service/`, `evals/`, `examples/`
- `.beads/`, `.jcode/agents/`

**Category G fixes applied**:
1. `SetPermissionMode` — added variant to `jcode-protocol/src/wire.rs`
2. `is_auto_poke_message` — added to `todo.rs`
3. `DcpCompressTool` / `BestOfNTool` — replaced with `InvalidTool` stubs
4. `task_label` field — added `task_label: None` to all `SwarmMemberStatus` initializers
5. `tldr` field — added `..` patterns to `NotificationType::Message` matches
6. `poll_todo_pipeline` — added stub returning `Ok(())`
7. `same_instance` — added to `InterruptSignal`
8. `set_temperature` — added to Provider trait (default `Ok(())`)
9. `has_credentials` — added free function wrapper in openrouter.rs
10. `fetch_catalog_snapshot` / `persist_catalog` — made `pub` in antigravity.rs
11. `MERMAID_PENDING_PLACEHOLDER_TEXT` — added constant + helper function

### 2026-07-11 — v0.40.0..v0.41.0 upstream sync (commits 7dbc480ae + 82ea601a9 + 0399554e1)

**Branch**: `master` (via `sync-nov-commits` merge).
**Upstream range**: v0.40.0..v0.41.0 (~39 commits). **~107 files** in the merge (+4k lines).

**Status**: ✅ Binary builds. `cargo check -p jcode` / `jcode-tui` / `jcode-base --tests` / `jcode-tui --tests` clean. Installed local `current` channel as `v0.32.1408-dev (0399554e1)`.

| Category | What |
|----------|------|
| 🔀 Merge | Conflicts: Category A keep-ours (casr/dcg/dcp/hashline/ffs adapters), Category B local extensions |
| 🏗️ Category G | App fields/methods (auth catalog refresh, todo goals, spinner cadence, pinned diffs API, signature cache, session title) |
| 🐛 Fork bugfix | **`biased;` freeze** in `turn.rs` — MUST NOT reintroduce (see Lesson 2 below) |
| 🐛 Keyword highlight | Short-alias match extend to token boundary; skip subsumed highlights |
| 🔧 CASR | Removed inline `casr_adapter`; CursorSession path; `import_cursor_session` restored for tests |
| 🖥️ Remote auth | `AvailableModelsUpdated` → `finish_auth_catalog_refresh()` + force redraw |
| 📝 Prompt | `PromptCapabilities` / `MERMAID_PROMPT` module gating |

**Category G (v0.41) fixes applied**:
1. `auth_catalog_refresh_pending` init + `finish_auth_catalog_refresh`
2. `gather_todos_and_goals_for_session` + `InfoWidgetData.todo_goals` / `todos_are_swarm_plan`
3. `load_session_title` / `derive_session_title`
4. `STRIP_SPINNER_FRAME_MS` / `STRIP_SPINNER_FPS`
5. `collect_pinned_diffs_cached` (not the old multi-arg content probe)
6. `side_pane_images_signature` cache shape (invalidate-on-mutation, not version tuple)
7. `client_focused` + set/unfocused redraw helpers
8. Test compile: TodoItem `active_form`, openrouter-runtime dev-dep, prepare helpers, copy-badge helpers

**Post-merge MUST-check (freeze)**:
```bash
# Must print nothing for turn.rs event-loop select! (statement only)
rg -n '^\s*biased\s*;' crates/jcode-tui/src/tui/app/turn.rs && echo "FAIL: biased reintroduced" || echo "OK: no biased in turn.rs"
```

**Note on other `biased;` (OK to keep)**: intentional priority selects that do **not** starve `event_stream`:
- `crates/jcode-tui/src/tui/backend.rs` — cancellation-safety test
- `crates/jcode-app-core/src/agent/turn_streaming_mpsc.rs` — tool completion vs Alt+B
- `crates/jcode-app-core/src/server/client_lifecycle.rs` — client I/O over bus noise

### 2026-07-11 — v0.41.0..v0.42.0 upstream sync (merge upstream/master → master)

**Branch**: `master`.
**Upstream range**: post-v0.41 tip → v0.42.0 + follow-ups (~34 commits, tip `329c1ebf0`).
**Files**: ~122 changed in merge (+7k / -1.5k).

**Status**: ✅ `cargo check -p jcode` clean. Lesson 2 (`biased;` absent in `turn.rs`). Lesson 3 (`SERVER-SAVE` / `ROUTE-SAVE` + `PICKER-SAVE` + `apply_config_default_model`) preserved. Fork version stays **0.32.0** (do not take upstream package version).

| Category | What |
|----------|------|
| ✨ Feature | **LaTeX / terminal math** (`jcode-render-core` math, markdown render path) |
| ✨ Feature | **Typo-resistant fuzzy** (`jcode-fuzzy` crate; TUI picker / slash commands) |
| ✨ Feature | **Swarm agent cards** under spawn calls; strip `icon`/`task`; dock `managed_members` |
| 🖥️ Server | Swarm persistence, lock races, lifecycle ownership, reload recovery, Retry-After |
| 🏎️ Memory | Retained-heap watchdog (`release_retained_heap_if_excessive`, idle TUI trim) |
| 📝 Skills | Upstream trailing-prompt after skill; **fork keeps `$` namespace** + `SkillInvocation` |
| 🐛 Fork protect | Lesson 2 no-`biased;`; Lesson 3 model persist blocks; agent route-aware default_model |

**Kept OURS** (Category A / B / H):
- Extracted deps + adapters (casr/ffs/dcg/hashline/mempalace/dcp/rtco/beads)
- `skill.rs` `$` + trailing prompt (not `/skill`)
- `turn.rs` event loop without `biased;`
- `provider_control.rs` SERVER/ROUTE-SAVE
- Cargo workspace version 0.32.0 + `jcode-fuzzy` member

**Category G / bad-merge repairs**:
1. `GalleryMember.icon` / `.task` + `disp_w` / `CHIP_TASK_*` in `swarm_gallery.rs`
2. `members_to_gallery` + `member_icon` / `age_marker` in info-widget adapter
3. Restored `ui_prepare.rs` from HEAD (duplicate-fn concat), re-applied upstream swarm-card inject + cache signature
4. `SwarmInfo` fields: `managed_members`, `selected`, `focused`, `plan_progress`, `spinner_frame`
5. `process_memory.rs` take upstream retention-trim APIs
6. Deduped `apply_inline_interactive_filter` onto `jcode_fuzzy::fuzzy_score_tokens`
7. Skill invocation tests updated for `SkillInvocation` + trailing prompt

**Post-merge MUST-check**:
```bash
rg -n '^\s*biased\s*;' crates/jcode-tui/src/tui/app/turn.rs && echo FAIL || echo OK
rg -n 'SERVER-SAVE|ROUTE-SAVE' crates/jcode-app-core/src/server/provider_control.rs
rg -n 'parse_invocation|SkillInvocation' crates/jcode-base/src/skill.rs | head
cargo check -p jcode
```

### 2026-07-11 — v0.42.0..v0.43.0 upstream sync (2 commits)

**Branch**: `master`.
**Upstream commits**:
- `a07bd3d96` feat(markdown): support common LaTeX containers
- `649276753` release: v0.43.0

**Status**: ✅ `cargo check -p jcode` clean. Lesson 2/3 OK. Fork package version remains **0.32.0**.

| Category | What |
|----------|------|
| ✨ Feature | `normalize_latex_math` — `\(...\)`, `\[...\]`, display envs, fenced math/latex/tex/katex |
| ✨ Feature | Broader math env delimiters (`gather`, `multline`, `eqnarray`, …) |
| 📝 Release | changelog `v0.43.0.json` + index entry (accepted) |
| 🏗️ Category D | Cargo.toml version conflict → **keep 0.32.0** (not 0.43.0) |

**No extracted-domain / Lesson 2–3 files touched** by the 2 commits. Auto-merge for render/markdown paths (Category C).
