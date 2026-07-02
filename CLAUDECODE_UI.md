# CLAUDECODE_UI.md — Claude Code UI Reference & Migration Map

> **Source:** Decompiled `claude-code-best` repo tại `/tmp/feature-research/claude-code/`
> **Stack:** TypeScript / Bun / React-Ink (`@anthropic/ink`)
> **jcode stack:** Rust / ratatui v0.30 / crossterm v0.29
> **Mục đích:** Làm reference cho migration jcode TUI → Claude Code UI/UX

---

## Table of Contents

1. [Overall Layout — FullscreenLayout](#1-overall-layout--fullscreenlayout)
2. [REPL — Root Screen](#2-repl--root-screen)
3. [Status Line](#3-status-line)
4. [Chat Viewport / Messages](#4-chat-viewport--messages)
5. [User Message](#5-user-message)
6. [Assistant Message (Text)](#6-assistant-message-text)
7. [Thinking / Reasoning Block](#7-thinking--reasoning-block)
8. [Tool Call — Bash](#8-tool-call--bash)
9. [Tool Call — Edit](#9-tool-call--edit)
10. [Tool Call — Read](#10-tool-call--read)
11. [Tool Call — Glob/Grep](#11-tool-call--globgrep)
12. [Tool Call — Agent (Sub-agent)](#12-tool-call--agent-sub-agent)
13. [Permission Dialog — Bash](#13-permission-dialog--bash)
14. [Permission Dialog — Edit/Read/Generic](#14-permission-dialog--editreadgeneric)
15. [Chat Composer (Input)](#15-chat-composer-input)
16. [Spinner / Thinking Indicator](#16-spinner--thinking-indicator)
17. [NewMessagesPill — Unseen Divider](#17-newmessagespill--unseen-divider)
18. [Transcript Overlay (Ctrl+O)](#18-transcript-overlay-ctrlo)
19. [Help / Keybinding Viewer](#19-help--keybinding-viewer)
20. [Session Picker / Resume Conversation](#20-session-picker--resume-conversation)
21. [Footer / Hints Bar](#21-footer--hints-bar)
22. [System Messages](#22-system-messages)
23. [Swarm Gallery / Multi-Agent UI](#23-swarm-gallery--multi-agent-ui)
24. [Theme Switching](#24-theme-switching)
25. [Error State](#25-error-state)
26. [Splash / Empty State](#26-splash--empty-state)
27. [Model Picker](#27-model-picker)
28. [Todos / Task Management](#28-todos--task-management)
29. [Background Tasks / Progress Panel](#29-background-tasks--progress-panel)
30. [Usage / Cost Overlay](#30-usage--cost-overlay)
31. [Copy / Selection](#31-copy--selection)
32. [Toast Notifications](#32-toast-notifications)
33. [Settings / Config Dialog](#33-settings--config-dialog)
34. [Plan Mode](#34-plan-mode)
35. [@-Mentions Popup](#35--mentions-popup)
36. [QuickOpen & GlobalSearch Dialogs](#36-quickopen--globalsearch-dialogs)
37. [Tool Call Grouping & Collapse Patterns](#37-tool-call-grouping--collapse-patterns)
38. [Dialog Registry (56 components)](#38-dialog-registry-56-components)
39. [Features NOT in Claude Code](#39-features-not-in-claude-code)
40. [Migration Priority Map](#40-migration-priority-map)
41. [Appendix: jcode ↔ CC File Mapping](#41-appendix-jcode--cc-file-mapping)

---

## 1. Overall Layout — FullscreenLayout

**CC File:** `src/components/FullscreenLayout.tsx` (549 lines)
**jcode:** `crates/jcode-tui/src/tui/ui.rs` (3178 lines)

### Layout Regions

The screen is divided into **3 regions** in fullscreen mode:

```
┌──────────────────────────────────────────────────────────────────────┐
│  ╔══════════════════════════════════════════════════════════════════╗ │
│  ║        UPPER REGION — flexGrow={1}                             ║ │
│  ║                                                                 ║ │
│  ║  [StickyPromptHeader] — optional, 1-row pinned (when scrolled)  ║ │
│  ║                                                                 ║ │
│  ║  ScrollBox — flexGrow={1}, stickyScroll                         ║ │
│  ║    ├── Messages (VirtualMessageList)                            ║ │
│  ║    ├── Spacer (Box flexGrow={1})                                ║ │
│  ║    ├── SpinnerWithVerb (INSIDE scrollable)                      ║ │
│  ║    └── PermissionRequest (overlay slot, inside ScrollBox)       ║ │
│  ║                                                                 ║ │
│  ║  [NewMessagesPill] — position="absolute" bottom={0} overlay     ║ │
│  ║  [BottomFloat] — companion bubble, bottom-right                 ║ │
│  ╚══════════════════════════════════════════════════════════════════╝ │
├──────────────────────────────────────────────────────────────────────┤
│  ╔══════════════════════════════════════════════════════════════════╗ │
│  ║        BOTTOM SLOT — flexShrink={0}, maxHeight="50%"            ║ │
│  ║                                                                 ║ │
│  ║  [SuggestionsOverlay] — absolute, bottom="100%" — floats above  ║ │
│  ║  [DialogOverlay] — absolute, bottom="100%" — floats above       ║ │
│  ║                                                                 ║ │
│  ║  Box overflowY="hidden"                                         ║ │
│  ║    ├── PromptInputQueuedCommands                                ║ │
│  ║    ├── PermissionStickyFooter                                   ║ │
│  ║    ├── TaskListV2 (expanded todos)                              ║ │
│  ║    ├── PromptDialog / ElicitationDialog / UltraplanDialogs      ║ │
│  ║    ├── GlobalSearchDialog / QuickOpenDialog / HistorySearch      ║ │
│  ║    ├── TeamsDialog / BridgeDialog                               ║ │
│  ║    ├── ModelPicker / FastModePicker / ThinkingToggle            ║ │
│  ║    └── PromptInput (the text input area)                        ║ │
│  ║          └── PromptInputFooter                                  ║ │
│  ║                ├── StatusLine                                   ║ │
│  ║                └── PromptInputFooterLeftSide (hints)            ║ │
│  ╚══════════════════════════════════════════════════════════════════╝ │
└──────────────────────────────────────────────────────────────────────┘
│  ╔══════════════════════════════════════════════════════════════════╗ │
│  ║        MODAL SLOT — position="absolute" bottom={0}              ║ │
│  ║        maxHeight={rows - 2} — 2 rows transcript peek visible   ║ │
│  ║        Used for local JSX commands: /model, /mcp, /config       ║ │
│  ╚══════════════════════════════════════════════════════════════════╝ │
```

### CC Source Code Pattern (FullscreenLayout.tsx)

```tsx
<PromptOverlayProvider>
  <Box flexDirection="row" flexGrow={1} overflow="hidden">
    <Box flexDirection="column" flexGrow={1}>
      {/* UPPER REGION */}
      {stickyPromptHeader}
      <ScrollBox flexGrow={1} stickyScroll>
        <ScrollChromeContext.Provider>
          {scrollable}     // messages + spacer + spinner
        </ScrollChromeContext.Provider>
        {overlay}          // PermissionRequest
      </ScrollBox>
      <NewMessagesPill />
      <BottomFloat />

      {/* BOTTOM SLOT — maxHeight=50% */}
      <Box flexDirection="column" flexShrink={0} width="100%" maxHeight="50%">
        <SuggestionsOverlay />
        <DialogOverlay />
        <Box flexDirection="column" width="100%" flexGrow={1} overflowY="hidden">
          {bottom}         // PromptInput + StatusLine + dialogs
        </Box>
      </Box>
    </Box>

    {/* MODAL SLOT */}
    <ModalContext.Provider>
      <Box position="absolute" bottom={0} maxHeight={rows-2}>
        <Text>---</Text>
        <Box paddingX={2}>{modal}</Box>
      </Box>
    </ModalContext.Provider>
  </Box>
</PromptOverlayProvider>
```

### jcode Migration Notes

| Aspect | CC | jcode | Priority |
|--------|----|-------|----------|
| Bottom slot maxHeight | `maxHeight="50%"` | ⚠️ Cần verify | **P0** |
| Permissions overlay | Trong ScrollBox (scroll cùng messages) | Có thể khác | P2 |
| StickyPromptHeader | 1-row pinned khi scroll | ❌ Chưa có | P3 |

---

## 2. REPL — Root Screen

**CC File:** `src/screens/REPL.tsx` (6680 lines)
**jcode:** `crates/jcode-tui/src/tui/app.rs` (~2000 lines)

```
App (providers)
  -> FpsMetricsProvider
    -> StatsProvider
      -> AppStateProvider
        -> ThemeProvider
          -> KeybindingSetup
            -> AnimatedTerminalTitle
            -> GlobalKeybindingHandlers
            -> ScrollKeybindingHandler
            -> CancelRequestHandler
            -> MCPConnectionManager
              -> AlternateScreen (fullscreen mode only)
                -> FullscreenLayout
```

### How REPL populates FullscreenLayout (REPL.tsx lines 5887-6672)

```tsx
// scrollable prop — INSIDE ScrollBox
<>
  <TeammateViewHeader />
  <Messages />                                    // conversation transcript
  <AwsAuthStatusBox />
  <UserTextMessage />                             // placeholder while processing
  {toolJSX.jsx}                                   // non-immediate local JSX commands
  <Box flexGrow={1} />                            // SPACER — pushes spinner to bottom
  <SpinnerWithVerb />                             // spinner INSIDE scrollable
  <BriefIdleStatus />
</>

// overlay prop — INSIDE ScrollBox (after scrollable)
<PermissionRequest />

// bottom prop — ~20+ possible components
<>
  <PromptInputQueuedCommands />
  <PermissionStickyFooter />
  {toolJSX.jsx}                    // immediate commands
  <TaskListV2 />
  <SandboxPermissionRequest />
  <PromptDialog />
  <ElicitationDialog />
  <UltraplanChoiceDialog />
  <CostThresholdDialog />
  <IdleReturnDialog />
  <GlobalSearchDialog />
  <QuickOpenDialog />
  <HistorySearchDialog />
  <TeamsDialog />
  <BridgeDialog />
  <ModelPicker />
  <FastModePicker />
  <ThinkingToggle />
  <PromptInput />
</>
```

---

## 3. Status Line

**CC File:** `src/components/StatusLine.tsx` (587 lines) + `BuiltinStatusLine.tsx` (129 lines)
**jcode:** `crates/jcode-tui/src/tui/ui_input.rs` lines 670-880 (status bar trong header)

### Vị trí QUAN TRỌNG

**StatusLine RENDERS INSIDE PromptInputFooter** — dưới input box, không phải trên cùng.

```tsx
// PromptInputFooter.tsx (409 lines)
<Box flexDirection="row" paddingX={2}>
  <Box flexDirection="column">
    {mode === 'prompt' && !isShort && !exitMessage.show && !isPasting
     && statusLineShouldDisplay(settings) && (
      <StatusLine />   // ← RENDERS HERE: BELOW INPUT
    )}
    <PipeStatusInline />
    <PromptInputFooterLeftSide />
  </Box>
  <Box flexShrink={1}>
    <Notifications />         // non-fullscreen only
    <BridgeStatusIndicator />
  </Box>
</Box>
```

### ASCII

```
┌─────────────────────────────────────────────────────────────────────┐
│ ▌  Fix the bug in auth.rs                                          │ ← Input
├─────────────────────────────────────────────────────────────────────┤
│ sonnet-4  ctx:42%  [📋 3]  $0.12  cache:78%  ▌auto  🎯 Fix       │ ← StatusLine
│ Tab:autocomplete  Ctrl+X:leader  Ctrl+O:transcript  /:commands     │ ← Hints
└─────────────────────────────────────────────────────────────────────┘
```

### BuiltinStatusLine (trái → phải)

```
ModelName | Context NN% (usedK/totalK) | Session NN% (countdown) | Weekly NN% (countdown) | $cost

Example:
claude-sonnet-4  ctx:42%  45k/100k  session:18%  weekly:5%  $0.12
```

### CachePill (StatusLine.tsx lines 71-158)

Displays: `Cache NN% MM:SS` — hit rate with **60-minute countdown timer**
- Normal: green/white
- Last 5 minutes: red
- Last minute: flashing (alternating red/dim red)

### GoalPill

Shows current goal status: `🎯 Fix auth bug (active)`

### Hiding Logic

**StatusLine tự động ẩn khi:**
- Terminal < 24 rows
- `suppressHint` = true (custom status line or Ctrl+R search)
- `settings.statusLineEnabled === false` (user config)
- Exit message showing
- Pasting in progress

---

## 4. Chat Viewport / Messages

**CC Files:**
- `src/components/Messages.tsx` (1114 lines) — orchestrator
- `src/components/Message.tsx` — per-message dispatcher
- `src/components/MessageRow.tsx` — memo wrapper
- `src/components/VirtualMessageList.tsx` — virtualized scroll

### Message Type Dispatcher (Message.tsx)

```
Message
  ├── AssistantToolUseMessage    — tool_use (outgoing tool calls)
  ├── UserToolResultMessage      — tool_result (returned output)
  │     ├── UserToolSuccessMessage
  │     ├── UserToolErrorMessage
  │     ├── UserToolRejectMessage
  │     └── UserToolCanceledMessage
  ├── UserTextMessage            — dispatcher → ~15 subtypes
  ├── AssistantTextMessage       — assistant text (markdown)
  ├── AssistantThinkingMessage   — thinking blocks
  ├── AssistantRedactedThinkingMessage
  ├── SystemTextMessage          — ~12 subtypes
  ├── GroupedToolUseMessage      — grouped consecutive tool uses
  ├── CollapsedReadSearchGroup   — collapsed read/search groups
  └── AttachmentMessage          — file attachments
```

### Messages Pipeine (Messages.tsx)

```typescript
const processed = pipe(
  rawMessages,
  normalizeMessages,
  reorderMessagesInUI,
  applyGrouping,                  // group consecutive same-type tool uses
  collapseReadSearchGroups,       // collapse Read/Grep/Glob/REPL → 1 row
  collapseHookSummaries,
  collapseBackgroundBashNotifications,
  collapseTeammateShutdowns
);
```

### VirtualMessageList

- Fullscreen mode only
- `useVirtualScroll` hook + Yoga height measurement
- Renders visible slice (start-end) + topSpacer/bottomSpacer
- Supports **sticky prompt header** (last user prompt pinned when scrolled above viewport)
- **Click to expand** — clicking a message toggles verbose mode
- **Transcript search** — indexOf-based, n/N navigation, highlight overlay
- **Cursor navigation** (j/k) via `useImperativeHandle`
- Safety cap: 200 messages (non-virtualized)

---

## 5. User Message

**CC File:** `src/components/messages/UserTextMessage.tsx` + `UserPromptMessage.tsx`

```
┌─ User ──────────────────────────────────────────────────────────────┐
│ > Fix the bug in auth.rs                                           │
└─────────────────────────────────────────────────────────────────────┘

  ↑ left border: colored per-agent (7 colors for sub-agents)
  ↑ "User" label: dimmed
  ↑ text: wrapped to terminal width
```

### UserPromptMessage (truncation)

```typescript
// >10,000 chars → truncate
const MAX_USER_PROMPT_LENGTH = 10000;
const HEAD_TAIL_LENGTH = 2500;

if (content.length > MAX_USER_PROMPT_LENGTH) {
  const head = content.slice(0, HEAD_TAIL_LENGTH);
  const tail = content.slice(-HEAD_TAIL_LENGTH);
  const midLines = content.slice(HEAD_TAIL_LENGTH, -HEAD_TAIL_LENGTH).split('\n').length;
  return <>
    <Text>{head}</Text>
    <Text dimColor>… +{midLines} lines …</Text>
    <Text>{tail}</Text>
  </>;
}
```

### With Image Attachment

```
┌─ User ──────────────────────────────────────────────────────────────┐
│ > What's wrong with this code?                                     │
│                                                                    │
│ ┌──────────────────────────────────┐                               │
│ │  [screenshot.png]                │                               │
│ │  ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ │                               │
│ │  ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ │                               │
│ └──────────────────────────────────┘                               │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 6. Assistant Message (Text)

**CC File:** `src/components/messages/AssistantTextMessage.tsx`

```
┌─ Assistant ─────────────────────────────────────────────────────────┐
│ I'll analyze the auth module. Here's what I found:                 │
│                                                                    │
│ The bug is on line 42 — `validate_expiry` is called without the   │
│ current timestamp, so it always uses `None` as the default.        │
│                                                                    │
│ I'll fix this by adding a `now` parameter:                         │
│                                                                    │
│ ```rust                                                             │
│ fn validate_expiry(expiry: i64, now: i64) -> bool {               │
│     expiry > now                                                    │
│ }                                                                   │
│ ```                                                                 │
└─────────────────────────────────────────────────────────────────────┘

  ↑ assistant label: green (theme.ai_message)
  ↑ text: syntax-highlighted markdown
  ↑ code blocks: syntax highlighting
  ↑ tables: formatted text
```

### Assistant Turn Footer (post-message metadata)

```
┌─ Assistant ──────────────────────────────────────────────────────────┐
│ I'll fix the auth bug. The issue is that `validate_expiry` was      │
│ called without the current timestamp.                               │
│                                                                      │
│ ─── sonnet-4 · Anthropic · 3.2s · ~1,240 tokens ─────────────────── │
└─────────────────────────────────────────────────────────────────────┘
```

Footer variants:

| Variant | Display |
|---------|---------|
| Default | model · provider · duration |
| With context | model · provider · duration · tokens |
| Tool-heavy | model · provider · duration · N tools & tokens |
| Minimal (< 60 cols) | model · duration |

---

## 7. Thinking / Reasoning Block

**CC File:** `src/components/messages/AssistantThinkingMessage.tsx` (55 lines)

### Collapsed (default, non-verbose mode)

```
┌─ Assistant ─────────────────────────────────────────────────────────┐
│  ∴ Thinking  <Ctrl+O to expand>                                    │ ← 1-line, dim
│                                                                    │
│ I'll fix the bug in auth.rs...                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### Expanded (verbose/transcript mode)

```
┌─ Assistant ─────────────────────────────────────────────────────────┐
│  ∴ Thinking...                                                      │
│    Let me analyze the auth module. I need to find where             │
│    validate_expiry is called and check if it has the right           │
│    parameters. Looking at line 42...                                │
│                                                                    │
│ I'll fix the bug in auth.rs...                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### Hidden

```
┌─ Assistant ─────────────────────────────────────────────────────────┐
│ I'll fix the bug in auth.rs...                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### CC Implementation

```tsx
// AssistantThinkingMessage.tsx
if (!verbose && !isTranscriptMode) {
  // Non-verbose, non-transcript: show 1-line dim + hint
  return (
    <Box>
      <Text dimColor>∴ Thinking  {ctrlOToExpand}</Text>
    </Box>
  );
}
// Verbose/transcript: show full thinking with 2-space indent
return (
  <Box flexDirection="column">
    <Text dimColor>∴ Thinking...</Text>
    <Box paddingLeft={2}>
      <Markdown>{thinkingContent}</Markdown>
    </Box>
  </Box>
);
```

---

## 8. Tool Call — Bash

**CC File:** `src/components/messages/AssistantToolUseMessage.tsx`

### Running

```
┌─ Bash ──────────────────────────────────────────────────────────────┐
│ ●  Bash (cargo test --lib jcode-tui)                                │ ← ● = grey loader dot
│ ⠋ running... 2.3s                                                  │ ← progress message
└─────────────────────────────────────────────────────────────────────┘
```

### Completed (success)

```
┌─ Bash ──────────────────────────────────────────────────────────────┐
│ ✓  Bash (cargo test --lib jcode-tui)                             ✓ │ ← green dot
│   test result: ok. 42 passed; 0 failed; 0 ignored                  │
│                                                                     │
│   running 42 tests                                                  │
│   test theme::tests::test_cie76 ... ok                             │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### Completed (failure)

```
┌─ Bash ──────────────────────────────────────────────────────────────┐
│ ✗  Bash (cargo build)                                            ✗ │ ← red dot
│   error[E0596]: cannot borrow `buf` as mutable                     │
│     --> src/render.rs:42:5                                         │
└─────────────────────────────────────────────────────────────────────┘
```

### ToolUseLoader Component (33 lines)

```tsx
// ToolUseLoader.tsx — SIMPLE: blinking/green/red dot
function ToolUseLoader({ isUnresolved, shouldAnimate, isError }) {
  const isVisible = useBlink(shouldAnimate);  // 600ms interval

  let dot;
  if (isUnresolved && shouldAnimate) {
    dot = isVisible ? <Text>●</Text> : <Text> </Text>;  // blinking
  } else if (isUnresolved && !shouldAnimate) {
    dot = <Text dimColor>●</Text>;  // static grey
  } else if (isError) {
    dot = <Text color="red">●</Text>;  // red
  } else {
    dot = <Text color="green">●</Text>;  // green success
  }
  return dot;
}
```

---

## 9. Tool Call — Edit

**CC File:** via tool-specific renderer

### Running

```
┌─ Edit ──────────────────────────────────────────────────────────────┐
│ ●  Edit (Update src/auth.rs)                                        │
│ ⠋ applying...                                                       │
└─────────────────────────────────────────────────────────────────────┘
```

### Completed

```
┌─ Edit ──────────────────────────────────────────────────────────────┐
│ ✓  Edit (Update src/auth.rs)                                      ✓ │
│  12 │ fn validate_expiry(expiry: i64) -> bool {                    │ ← red (-)
│  12 │ fn validate_expiry(expiry: i64, now: i64) -> bool {          │ ← green (+)
│  13 │     expiry > 0                                               │ ← red (-)
│  13 │     expiry > now                                             │ ← green (+)
└─────────────────────────────────────────────────────────────────────┘
```

### Create (new file)

```
┌─ Edit ──────────────────────────────────────────────────────────────┐
│ ✓  Edit (Create src/new_module.rs)                               ✓ │
│  + use std::collections::HashMap;                                  │
│  +                                                                 │
│  + pub struct NewModule {                                          │
│  +     data: HashMap<String, String>,                              │
│  + }                                                               │
└─────────────────────────────────────────────────────────────────────┘

  ↑ green for additions (+), red for deletions (-)
  ↑ line numbers shown
```

---

## 10. Tool Call — Read

### Running

```
┌─ Read ──────────────────────────────────────────────────────────────┐
│ ●  Read (src/auth.rs)                                               │
│ ⠋ reading...                                                        │
└─────────────────────────────────────────────────────────────────────┘
```

### Completed (verbose)

```
┌─ Read ──────────────────────────────────────────────────────────────┐
│ ✓  Read (src/auth.rs)                                             ✓ │
│   1  │ use crate::token::validate_token;                            │
│   2  │                                                             │
│   3  │ pub fn validate_expiry(expiry: i64, now: i64) -> bool {     │
└─────────────────────────────────────────────────────────────────────┘
```

### Completed (non-verbose, default: collapsed)

```
┌─ Read ──────────────────────────────────────────────────────────────┐
│ ✓  Read (src/auth.rs — 7 lines)                                  ✓ │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 11. Tool Call — Glob/Grep

**CC File:** `src/components/messages/CollapsedReadSearchContent.tsx`

Glob/Grep/Read/REPL all get **collapsed** into compact rows.

### Glob — compact (default)

```
☆ glob **/*.rs → 42 matches
```

### Glob — expanded (click to expand)

```
┌─ Glob ──────────────────────────────────────────────────────────────┐
│ ☆ glob **/*.rs → 42 matches                                        │
│   src/main.rs                                                      │
│   src/auth.rs                                                      │
│   src/lib.rs                                                       │
│   ... 39 more                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### Grep — compact (default)

```
◆ grep "validate" src/ → 7 matches in 3 files
```

### Grep — expanded

```
┌─ Grep ──────────────────────────────────────────────────────────────┐
│ ◆ grep "validate" src/ → 7 matches in 3 files                      │
│   src/auth.rs:12: fn validate_expiry(...)                           │
│   src/auth.rs:45: if !validate_expiry(...)                         │
│   ... 4 more                                                       │
└─────────────────────────────────────────────────────────────────────┘
```

### CollapsedReadSearchContent — Live Hint

While active, shows **live update** of current file/pattern:

```
Searching for 3 patterns  (Ctrl+O to expand)
├─ src/auth.rs
⎿

Reading 2 files  (Ctrl+O to expand)
├─ src/auth.rs
⎿
```

When finalized:

```
Read 5 files  ✓  (Ctrl+O to expand)
```

---

## 12. Tool Call — Agent (Sub-agent)

### Running

```
┌─ Agent ─────────────────────────────────────────────────────────────┐
│ ●  Agent (research auth patterns)                                   │
│ ⠋ running... 12.3s                                                 │
│   tools: 3 read, 2 grep, 1 bash                                   │
└─────────────────────────────────────────────────────────────────────┘
```

### Completed

```
┌─ Agent ─────────────────────────────────────────────────────────────┐
│ ✓  Agent (research auth patterns — 15.2s)                        ✓ │
│   tools: 5 read, 2 grep, 1 bash                                   │
│                                                                    │
│   Found 3 common auth patterns in the codebase:                    │
│   1. JWT token validation                                          │
│   2. Session-based auth                                            │
│   3. OAuth2 flow                                                   │
└─────────────────────────────────────────────────────────────────────┘
```

### Delegating

```
┌─ Agent ─────────────────────────────────────────────────────────────┐
│ 📤 Delegating to sub-agent...                                      │
│   task: "implement the fix"                                        │
│   model: claude-sonnet-4-20250514                                  │
└─────────────────────────────────────────────────────────────────────┘
```

### Lifecycle States

| State | Icon | Color |
|-------|------|-------|
| Queued | ... | text_subtle |
| Spawning | 🔱 | accent |
| Running | ● animated | accent (blinking) |
| Done | ✓ | success (green) |
| Failed | ✗ | error (red) |
| Timeout | ⚠ | warning |

---

## 13. Permission Dialog — Bash

**CC File:** `src/components/permissions/BashPermissionRequest/BashPermissionRequest.tsx`
**Renders in:** FullscreenLayout `overlay` slot (inside ScrollBox)

```
┌─────────────────────────────────────────────────────────────────────┐
│ 🔐 Permission required                                              │
│                                                                    │
│ Bash wants to run:                                                 │
│                                                                    │
│ $ rm -rf /tmp/test                                                 │
│                                                                    │
│ ┌────────────────────────────────────────────────────────────────┐ │
│ │ ⚠ This command will delete files permanently.                  │ │
│ └────────────────────────────────────────────────────────────────┘ │
│                                                                    │
│  [y] Allow    [Y] Always Allow    [n] Deny    [Esc] Abort         │
│                                                                    │
│  Ctrl+D: debug  Ctrl+E: explanation                                │
└─────────────────────────────────────────────────────────────────────┘
```

### Bash Classifier Shimmer (auto-approval animation)

```
┌─ Bash ──────────────────────────────────────────────────────────────┐
│ ●  Bash (cargo build)                                               │
│ 🔍 Auto-classifier checking...                                      │ ← shimmer text
│                                                                    │
│ Auto-approved ✓                                                     │ ← when approved
└─────────────────────────────────────────────────────────────────────┘
```

The shimmer runs at 50ms intervals (extracted to `ClassifierCheckingSubtitle` to prevent full dialog re-render at 20fps).

### PermissionPrompt (shared selection UI)

```tsx
// Shows "Do you want to proceed?" with <Select> component
// Supports optional feedback input (Tab toggles inline text input)
// Options: accept, accept-with-feedback, reject, reject-with-feedback
```

### PermissionDialog (wrapper)

```tsx
<Box borderStyle="round" borderColor={color}>
  <Box justifyContent="space-between">
    <PermissionRequestTitle />
    {titleRight}
  </Box>
  <Box paddingX={innerPaddingX}>
    {children}
  </Box>
</Box>
```

---

## 14. Permission Dialog — Edit/Read/Generic

### Edit Permission

```
┌─────────────────────────────────────────────────────────────────────┐
│ 🔐 Permission required                                             │
│                                                                    │
│ Edit wants to modify:                                              │
│                                                                    │
│ → src/auth.rs                                                      │
│                                                                    │
│ ┌─ diff ────────────────────────────────────────────────────────┐  │
│ │  12 │ fn validate_expiry(expiry: i64) -> bool {              │  │
│ │  12 │ fn validate_expiry(expiry: i64, now: i64) -> bool {    │  │
│ │  13 │     expiry > 0                                          │  │
│ │  13 │     expiry > now                                        │  │
│ └───────────────────────────────────────────────────────────────┘  │
│                                                                    │
│  [y] Allow    [a] Always Allow    [n] Deny                        │
└─────────────────────────────────────────────────────────────────────┘
```

### Read Permission (simpler)

```
┌─────────────────────────────────────────────────────────────────────┐
│ 🔐 Permission required                                             │
│                                                                    │
│ Read wants to access:                                              │
│                                                                    │
│ → /etc/passwd                                                      │
│                                                                    │
│ [y] Allow    [a] Always Allow    [n] Deny                         │
└─────────────────────────────────────────────────────────────────────┘
```

### Generic/Fallback Permission

```
┌─────────────────────────────────────────────────────────────────────┐
│ 🔐 Permission required                                             │
│                                                                    │
│ Tool wants to run:                                                 │
│ WebFetch("https://api.example.com")                                │
│   description: "Fetch data from example API"                       │
│                                                                    │
│ [y] Yes   [Y] Yes, and don't ask again for [tool] in [dir]   [n] No│
└─────────────────────────────────────────────────────────────────────┘
```

---

## 15. Chat Composer (Input)

**CC File:** `src/components/PromptInput/PromptInput.tsx` (2650 lines)

### Normal state

```
┌─────────────────────────────────────────────────────────────────────┐
│ ▌                                                                  │
└─────────────────────────────────────────────────────────────────────┘
```

### With input

```
┌─────────────────────────────────────────────────────────────────────┐
│ Fix the bug in auth.rs and add tests                               │
│ ▌                                                                  │
└─────────────────────────────────────────────────────────────────────┘
```

### With autocomplete popup

```
┌─────────────────────────────────────────────────────────────────────┐
│ Read src/aut                                                       │
│ ┌──────────────────────┐                                           │
│ │ auth.rs              │                                           │
│ │ auth_test.rs         │                                           │
│ │ auto_complete.rs     │                                           │
│ └──────────────────────┘                                           │
└─────────────────────────────────────────────────────────────────────┘
```

### CC Implementation Details

```typescript
const PROMPT_FOOTER_LINES = 5;
const MIN_INPUT_VIEWPORT_LINES = 3;

// Input area grows with content
// Uses VimTextInput or TextInput depending on config
<>
  <VimTextInput / TextInput />
  <PromptInputFooter />
</>
```

---

## 16. Spinner / Thinking Indicator

**CC File:** `src/components/Spinner.tsx` (551 lines)
**Renders INSIDE ScrollBox**, after messages + `<Box flexGrow={1} />` spacer.

### States (từ CC source + ảnh thực tế)

CC spinner hiển thị ở 2 chế độ:
1. **Dòng spinner** (trong scrollbox, animate) — khi turn đang chạy
2. **Thumbnail mode** (trong scrollbox) — khi turn done, 1 dòng compact, không animate

```
IDLE:
(empty)

=== ACTIVE SPINNER STATES (turn running) ===
Thinking (loading):     Loading model...
Thinking (rephrase):    Working on it... / Thinking harder...
Thinking (processing):  ⠋ Thinking about the question...  ← Shimmer text (glimmer animation)
Infusing:               ● Infusing... (2m 54s · ↓ 3.3k tokens)  ← Red blink dot + spinner
Tool running:           ● Running Bash (cargo build)
Agent spawning:         🔱 Spawning sub-agent...
Network waiting:        ⏳ Waiting for network...
Permission pending:     🔐 Awaiting permission...
Streaming:              ⠋ Generating... / Streaming response...
Hook executing:         ⚡ Running hook...
Agent delegating:       📤 Delegating...
Brief:                  ✨
Speculation:            🔮 Speculating...

=== THUMBNAIL STATES (turn done, 1 line compact) ===
Thumbnail:              ● Thought for 40s
Thumbnail:              ● Thought for 12s, read 1 file
Thumbnail:              ● Searched for 3 patterns, read 2 files,
                          ran 2 shell commands (Ctrl+O to expand)
Thumbnail:              ● Infusing... (done)
```

Note: "Infusing..." là state đặc biệt — xuất hiện với red dot + elapsed time + token count. Không phải lỗi, là CC design cho process merging/fermenting.

### showSpinner condition (REPL.tsx)

```typescript
const showSpinner =
  (!toolJSX || toolJSX.showSpinner === true) &&
  toolUseConfirmQueue.length === 0 &&
  promptQueue.length === 0 &&
  (isLoading || userInputOnProcessing || ...);
```

### Spinner Components

| Component | Purpose |
|-----------|---------|
| `Spinner` | Simple 2-char animated dot |
| `SpinnerWithVerb` | Main: verb + shimmer + tip + budget |
| `BriefSpinner` | Compact for `/brief` mode |
| `BriefIdleStatus` | Idle placeholder |
| `TeammateSpinnerTree` | Expanded multi-agent tree |

### useBlink (animation hook)

All spinner instances share the same animation clock via `useAnimationFrame`. Pauses when terminal loses focus (`useTerminalFocus`).

---

## 17. NewMessagesPill — Unseen Divider

**CC File:** `src/components/FullscreenLayout.tsx` lines 469-481

```
When user scrolls up and new messages arrive:

┌─────────────────────────────────────────────────────────────────────┐
│ > older message                                                    │
│                                                                    │
│                                                                    │
│                                                                    │
│ ┌────────────────── 3 new messages ─────────────────────────────── │
│                                                                    │
│ > newer message (pinned content)                                   │
└─────────────────────────────────────────────────────────────────────┘
                ↑ clickable to jump to bottom

When at bottom: divider is hidden.
```

### CC Implementation

```tsx
// FullscreenLayout.tsx — driven by useSyncExternalStore
<Box position="absolute" bottom={0} left={0} right={0} justifyContent="center">
  <Box onClick={scrollDown}>
    <Text backgroundColor={hover ? 'hover' : 'normal'} dimColor>
      {' '}N new messages / Jump to bottom ↓
    </Text>
  </Box>
</Box>
```

---

## 18. Transcript Overlay (Ctrl+O)

**CC File:** `src/screens/REPL.tsx` (screen state: `'prompt' | 'transcript'`)

```
Toggle with Ctrl+O — full transcript in alternate screen:

┌─────────────────────────────────────────────────────────────────────┐
│ TRANSCRIPT                                          ↑/↓ scroll    │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│ [User]                                                              │
│ > Fix the bug in auth.rs                                            │
│                                                                     │
│ [Assistant]                                                         │
│ I'll analyze the auth module.                                       │
│                                                                     │
│ [Bash] $ grep -n "validate" src/auth.rs                            │
│        ✓ exit: 0                                                    │
│        12: fn validate_token(token: &str) -> bool {                 │
│                                                                     │
│ [Edit] → Update src/auth.rs                                         │
│                                                                     │
│ [Assistant]                                                         │
│ Fixed the bug.                                                      │
│                                                                     │
├─────────────────────────────────────────────────────────────────────┤
│ /search  ↑↓ navigate  q:close  Ctrl+E:show all                     │
└─────────────────────────────────────────────────────────────────────┘
```

### Keybindings (Transcript context)

- `ctrl+e` — toggle show all compacted messages
- `ctrl+c` / `Esc` / `q` — exit transcript
- `/` — search (n/N navigation)
- Arrow keys — scroll

---

## 19. Help / Keybinding Viewer

**CC File:** `src/components/HelpV2/HelpV2.tsx`, `src/commands/help/` 

⚠️ **Claude Code KHÔNG có Which-Key overlay như MASTER_UI.md spec.** Nó chỉ có `/help` command hiển thị text panel.

### jcode status

jcode có help overlay trong `ui_overlays.rs` (1286 lines) + `input_help.rs` (20 KB). Không cần migrate — CC cũng không có.

---

## 20. Session Picker / Resume Conversation

**CC File:** `src/screens/ResumeConversation.tsx` (15 KB)
**jcode:** `crates/jcode-tui/src/tui/session_picker.rs` (2168 lines)

```
╔══════════════════════════════════════════════════════════════════════╗
║ Sessions (12)                              type to search: auth    ║
╠══════════════════════════════════════════════════════════════════════╣
║                                                                     ║
║ ▸ auth bug fix                    2h ago  main  claude-sonnet-4    ║
║   Fix the authentication bug in auth.rs                             ║
║                                                                     ║
║   feature/tui-redesign           1d ago  feat/ claude-opus-4        ║
║   Migrate TUI to Claude Code patterns                               ║
║                                                                     ║
║   add tests for keymap           3d ago  main  claude-sonnet-4      ║
║                                                                     ║
║   (2 more matches hidden)                                           ║
║                                                                     ║
╠══════════════════════════════════════════════════════════════════════╣
║ Enter:resume  d:delete  f:fork  q:close                             ║
╚══════════════════════════════════════════════════════════════════════╝

  ↑ "▸" cursor for selected item
  ↑ shows: session title, age, git branch, model
  ↑ search filters by title
```

CC loads sessions via `loadAllProjectsMessageLogsProgressive()` / `loadSameRepoMessageLogsProgressive()`.

---

## 21. Footer / Hints Bar

**CC File:** `src/components/PromptInput/PromptInputFooterLeftSide.tsx` (682 lines)

```
┌─────────────────────────────────────────────────────────────────────┐
│ ▌ Fix the bug in auth.rs                                          │ ← Input
├─────────────────────────────────────────────────────────────────────┤
│ sonnet-4  ctx:42%  $0.12  cache:78%  ▌auto                        │ ← StatusLine
│ Tab:autocomplete  Ctrl+X:leader  Ctrl+O:transcript  /:commands     │ ← Hints
└─────────────────────────────────────────────────────────────────────┘

Hints (PromptInputFooterLeftSide) shows conditionally:
- Permission mode symbol + title
- Background task pills (BackgroundTaskStatus)
- Team status (TeamStatus)
- PR badge
- RSS memory (e.g. "128 MB . pid:12345")
- Goal elapsed ("goal (1h22min)")
- Tasks pill
- "esc to interrupt"
- "ctrl+t show tasks"
- Voice mode hints
- Selection copy hints
- Remote session indicator
```

---

## 22. System Messages

**CC File:** `src/components/messages/SystemTextMessage.tsx`

```
Notification:
ℹ Starting new session...

Error:
✗ Connection lost. Reconnecting...

Warning:
⚠ Context limit approaching (90%)

Tool progress:
⚙ Running 5 parallel searches...
```

---

## 23. Swarm Gallery / Multi-Agent UI

**CC File:** `src/components/teams/TeamsDialog.tsx` (649 lines)
**jcode:** `crates/jcode-tui/src/tui/info_widget_swarm_gallery.rs` + `info_widget_team.rs`

```
┌─────────────────────────────────────────────────────────────────────┐
│ ... swarm · 4 agents · 2 active                    [+] expand    │
├──────────────────┬──────────────────┬──────────────────────────────┤
│ ★ coordinator    │ ◆ researcher    │ ⚙ worker-1                   │
│ ─────────────── │ ─────────────── │ ─────────────────────────── │
│ Analyzing auth  │ grep "validate" │ $ cargo test                  │
│ module...        │ in src/...      │ ✓ exit: 0                     │
│                 │                 │ 42 passed                      │
│ status: running │ status: done    │ status: done                  │
├──────────────────┼──────────────────┤                              │
│ ⚙ worker-2     │ (+1 more)       │                              │
│ ─────────────── │                  │                              │
│ idle            │                  │                              │
└──────────────────┴──────────────────┴──────────────────────────────┘
```

### Role Glyphs

| Glyph | Role |
|-------|------|
| ★ | Coordinator |
| ◆ | Researcher |
| ⚙ | Worker |
| ☆ | Search specialist |
| ◇ | Reviewer |
| ⊞ | Planner |
| ◎ | Observer |

### TeammateSpinnerTree (inline)

```
🔱 research-auth
├─ ● Read src/auth.rs ✓
├─ ● Grep "validate" → 5 matches ✓
└─ ⠋ Thinking... 3.2s
```

---

## 24. Theme Switching

**CC File:** `src/components/ThemePicker.tsx` (80 lines)
**jcode:** `jcode-tui-style/src/theme.rs` + `src/theme.rs` (524 lines TOML loader)

```
Theme applied:

│ [catppuccin-mocha]  sonnet-4  ctx:42%  $0.12  cache:78%  ▌auto    │

Trigger: /theme

Available themes:
- auto (match terminal)
- dark
- light
- dark-daltonized
- 6 named palette themes

ctrl+t: toggle syntax highlighting
```

---

## 25. Error State

```
Connection error:

┌─────────────────────────────────────────────────────────────────────┐
│ ✗ Connection lost                                                  │
│   Reconnecting in 3s... (attempt 2/5)                             │
│                                                                    │
│ ▌                                                                  │
└─────────────────────────────────────────────────────────────────────┘

API error:

┌─────────────────────────────────────────────────────────────────────┐
│ ✗ API Error: Rate limited                                          │
│   Retry after 30s or switch model with Meta+P                     │
│                                                                    │
│ ▌                                                                  │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 26. Splash / Empty State

```
First launch (no messages):

┌─────────────────────────────────────────────────────────────────────┐
│                                                                     │
│                                                                     │
│                        claude-code                                  │
│                      v1.0.0                                          │
│                                                                     │
│              "What can I help you with?"                            │
│                                                                     │
│                                                                     │
│ ▌                                                                  │
│                                                                    │
├─────────────────────────────────────────────────────────────────────┤
│ Tab:autocomplete  Ctrl+X:leader  Ctrl+O:transcript  /:commands    │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 27. Model Picker

**CC File:** `src/components/ModelPicker.tsx` (376 lines)
**jcode:** `crates/jcode-tui/src/tui/app/model_context.rs` (69 KB)

```
Trigger: Meta+P or /model

┌─ Select Model ──────────────────────────────────────────────────────┐
│                                                                      │
│ ▸ claude-sonnet-4-20250514    (fast, recommended)     ← → effort   │
│   Context: 200K  Cost: $3/M in  $15/M out                           │
│   ▲ currently active · reasoning: medium                            │
│                                                                      │
│   claude-opus-4-20250514      (most capable)                        │
│   Context: 200K  Cost: $15/M in $75/M out                           │
│   ■ max reasoning capability  ▲ last used: 2h ago                   │
│                                                                      │
│   gemini-2.5-pro              (via OpenProxy)                       │
│                                                                      │
│   gpt-4o                      (via OpenProxy)                       │
│   [configure]                                                        │
│                                                                      │
│ ↑/↓ navigate  ← → effort  Space:1M context  Enter:select  q:close  │
└─────────────────────────────────────────────────────────────────────┘
```

Features:
- ← → arrow keys cycle effort level (low/medium/high/xhigh/max) per model
- Space toggles 1M context
- Fast mode indicator
- Plan mode override notice
- Provider connection status ●/○

---

## 28. Todos / Task Management

**CC File:** `src/components/TaskListV2.tsx` (325 lines) — inline, NOT modal
**jcode:** `crates/jcode-tui/src/tui/todos_view.rs`

```
Trigger: Ctrl+T

Status line pill:
│ sonnet-4  ctx:42%  [📋 3]  $0.12  cache:78%  ▌auto                  │
                      ↑ auto-pill: click or Ctrl+T to toggle

Inline task list:
┌─ To-Do ─────────────────────────────────────────────────────────────┐
│  ☑ Fix validate_expiry param                   done  ✓             │
│  ⏳ Add tests for token module                 in-prog  ⠋          │
│  ☐ Refactor auth module                        pending             │
│  ─────────────────────────────────────────────────────────────────  │
│  1/3 tasks complete  ████░░░░░░░░  33%                               │
│                                                                      │
│ [a] add  [x] toggle  [e] edit  [d] delete                           │
└─────────────────────────────────────────────────────────────────────┘
```

CC limits: `maxDisplay = rows <= 10 ? 0 : min(10, max(3, rows-14))`

---

## 29. Background Tasks / Progress Panel

**CC File:** `src/components/tasks/BackgroundTasksDialog.tsx` (852 lines)
**CC File:** `src/components/tasks/pillLabel.ts` — pill rendering
**CC File:** `src/components/tasks/ShellDetailDialog.tsx` — shell detail modal
**jcode:** `crates/jcode-tui/src/tui/ui_running_items.rs` (270 lines)

### Pills on Status Line

```
│ sonnet-4  ctx:42%  $0.12  [1 agent]  [1 shell]  ▌auto    │
                            ^ teal bg    ^ teal bg
```

CC renders pills on status line với màu teal background:
- `[1 agent]` khi có background agent chạy
- `[1 shell]` khi có shell command chạy (teal bg — ảnh 13)
- Click vào pill → mở BackgroundTasksDialog

### Shell Detail Modal (ảnh 14)

Khi click vào shell task trong dialog:

```
┌─ Shell detail ────────────────────────────────────────────────────┐
│                                                                      │
│   $ cargo test                                                       │
│                                                                      │
│   Running... 12:34:56                                                │
│   elapsed: 45.2s                                                     │
│   status: running                                                    │
│                                                                      │
│   [k] kill  [b] back  [q] close                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### Full Modal Dialog

```
┌─ Background Tasks ─────────────────────────────────────────────────┐
│                                                                      │
│ AGENTS                                                               │
│   ◆ explorer              ● running  8.3s                           │
│     Searching codebase for auth patterns                              │
│     ↓ 1.2k tokens                                                    │
│                                                                      │
│ SHELLS (1)                                                            │
│   $ cargo test            ● running  45.2s                          │
│     ═══════════░░░░░░░░░░  42%                                       │
│                                                                      │
│ LEADER                                                               │
│   ★ coordinator           ● running  12.3s                          │
│                                                                      │
│ ↑/↓ navigate  Enter:detail  k:kill  f:foreground  q:close          │
└─────────────────────────────────────────────────────────────────────┘

  ↑ AGENTS section — background sub-agents
  ↑ SHELLS section — background shell commands (có progress bar)
  ↑ LEADER section — coordinator agent
  ↑ Task types: local_bash, remote_agent, local_agent, in_process_teammate, local_workflow, monitor_mcp, dream, leader
```

### Jump to Bottom Pill (ảnh 15)

```
┌─────────────────────────────────────────────────────────────────────┐
│ > older message                                                    │
│                                                                    │
│                                                                    │
│ ┌────────────────── Jump to bottom (ctrl+End) ↓ ────────────────── │ ← Pill
│                                                                    │
│ > newer message (pinned content)                                   │
└─────────────────────────────────────────────────────────────────────┘
```

CC có **2 chế độ cho NewMessagesPill** (theo ảnh thực tế):
- `count > 0` → "N new message(s) (ctrl+End) ↓" (khi có message mới)
- `count === 0` → "Jump to bottom (ctrl+End) ↓" (khi không có message mới, chỉ là scroll lên)

CC Implementation (FullscreenLayout.tsx:469-481):
```typescript
function NewMessagesPill({ count, onClick }) {
  const [hover, setHover] = useState(false);
  const text = count > 0
    ? `${count} new message${count > 1 ? 's' : ''} (ctrl+End) ↓`
    : 'Jump to bottom (ctrl+End) ↓';
  return (
    <Box onClick={onClick}
         onMouseEnter={() => setHover(true)}
         onMouseLeave={() => setHover(false)}>
      <Text backgroundColor={hover ? 'hover' : 'normal'} dimColor>
        {text}
      </Text>
    </Box>
  );
}
```

---

## 30. Usage / Cost Overlay

**CC File:** `src/components/Settings/Usage.tsx` (tab within Settings)
**jcode:** `crates/jcode-tui/src/tui/info_widget_usage.rs` + `jcode-tui-usage-overlay` crate

```
Usage Statistics                      model: claude-sonnet-4
──────────────────────────────────────────────────────────────────────

This Session:
  Input tokens:  42,000  ($6.30)
  Output tokens: 8,500   ($12.75)
  ─────────────────────────────────────
  Total:                 $19.50

Rate Limits:
  Input:   ████████████████░░░░░  2,000 / 4,000 RPM
  Output:  ███████████████░░░░░░  1,500 / 3,000 RPM

5-hour session:  ██████░░░░░░░░  42%
7-day (all):     ██████████░░░░░  58%
7-day (Sonnet):  ████████████░░░  65%

q:close
```

---

## 31. Copy / Selection

**CC File:** `src/hooks/useCopyOnSelect.ts`, `src/components/ScrollKeybindingHandler.tsx`

⚠️ **Claude Code KHÔNG có dedicated "copy mode" toggle.** Nó dùng:
- `ctrl+shift+c` / `cmd+c` — copy selected text
- `ctrl+c` with active selection — copy instead of cancel
- `Esc` — clear selection

---

## 32. Toast Notifications

**CC File:** `src/context/notifications.tsx`, `src/components/PromptInput/Notifications.tsx`

⚠️ **Claude Code KHÔNG có toast system.** Notifications là inline text trong footer area, không phải popup.

---

## 33. Settings / Config Dialog

**CC File:** `src/components/Settings/Settings.tsx` (tabbed pane)

```
Trigger: /config or /settings

┌─ Configuration ────────────────────────────────────────────────────┐
│ [Status]  [Config*]  [Usage]                                       │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│ ▸ General                                                           │
│   Theme:         dark                 ← → cycle                    │
│   Permission:    auto                 ← → cycle                    │
│   Fast Mode:     off                                               │
│                                                                      │
│ ▸ Keys & Shortcuts                                                  │
│   Keybindings:   ~/.claude/keybinds.json                           │
│   Leader Key:    Ctrl+X                                             │
│                                                                      │
│ ↑/↓ navigate  ← → cycle value  Space:toggle  `/search  Enter:save │
└─────────────────────────────────────────────────────────────────────┘
```

3 tabs: Status (diagnostics), Config (key-value editor with cycling), Usage (rate limit progress bars)

---

## 34. Plan Mode

**CC File:** `src/tools.ts` (EnterPlanModeTool + ExitPlanModeV2Tool), `src/utils/planModeV2.ts`

⚠️ **Plan mode là tool-driven workflow, KHÔNG phải UI overlay.**

```
│ ▌auto   ▌plan  ← mode pill in status line
└─────────────────────────────────────────────────────────────────────

In Plan mode:
┌─ Assistant (Plan mode) ─────────────────────────────────────────────┐
│ Here's my plan to fix the auth bug:                                 │
│                                                                      │
│ 1. Modify `validate_expiry` to accept `now: i64` (Edit)             │
│ 2. Update all call sites (Edit × 3)                                  │
│ 3. Run tests to verify (Bash)                                        │
│                                                                      │
│ ┌─ Plan Approval ─────────────────────────────────────────────────┐ │
│ │  [y] Approve & Implement    [n] Reject    [e] Edit plan        │ │
│ └────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
```

5-phase workflow + optional interview phase. Agent count: Max/Team = 3, others = 1.

---

## 35. @-Mentions Popup

**CC File:** `src/hooks/useIdeAtMentioned.ts`, `src/components/PromptInput/PromptInput.tsx:1462`

```
│ Fix the bug using @aut
│ ┌─ @mention ─────────────────────────────────────────────────────┐ │
│ │  auth.rs                                    file              │ │
│ │  auth_test.rs                               file              │ │
│ │  validate_expiry                            symbol            │ │
│ │  auth-patterns                              memory            │ │
│ └────────────────────────────────────────────────────────────────┘ │
```

IDE MCP `at_mentioned` notification → inserts `@path#Lstart-Lend` at cursor.
Autocomplete registers as **non-modal overlay** (`NON_MODAL_OVERLAYS = new Set(['autocomplete'])`) so TextInput retains focus.

---

## 36. QuickOpen & GlobalSearch Dialogs

**CC File:** `src/components/QuickOpenDialog.tsx` (165 lines), `src/components/GlobalSearchDialog.tsx` (283 lines)

### QuickOpen (Ctrl+Shift+P) — Fuzzy file finder

```
┌─ Quick Open ─────────────────────────────────────────────────────────┐
│ [file name: auth]                                                    │
│                                                                      │
│ ▸ src/auth.rs                                                        │
│   src/auth_test.rs                                                   │
│   src/token.rs                                                       │
│                                                                      │
│ ↑/↓ navigate  Enter:open  q:close                                   │
└──────────────────────────────────────────────────────────────────────┘
```

### GlobalSearch (Ctrl+Shift+F) — Ripgrep workspace search

```
┌─ Search ─────────────────────────────────────────────────────────────┐
│ [search: validate]                                                   │
│                                                                      │
│ ▸ src/auth.rs:12: fn validate_expiry(...)                            │
│   src/auth.rs:45: if !validate_expiry(...)                           │
│   src/token.rs:8: pub fn validate_token(...)                         │
│                                                                      │
│   ┌─ preview ─────────────────────────────────────────────────────┐ │
│   │ 12: fn validate_expiry(expiry: i64) -> bool {                │ │
│   │ 13:     expiry > 0                                             │ │
│   └────────────────────────────────────────────────────────────────┘ │
│                                                                      │
│ ↑/↓ navigate  Enter:open  q:close                                   │
└──────────────────────────────────────────────────────────────────────┘
```

---

## 37. Tool Call Grouping & Collapse Patterns

### GroupedToolUse

Consecutive tool calls of the same type from **cùng 1 assistant turn** get **grouped** thành 1 container.

**CC Rule (từ `groupToolUses.ts` lines 67-207):**
```
Điều kiện GROUP:                                                      File:line
  ✅ 2+ tool calls cùng type (toolName), cùng messageId (cùng 1 turn)  groupToolUses.ts:88-104
  ✅ Tool có implement `renderGroupedToolUse()`                         groupToolUses.ts:31  
  ✅ Non-verbose mode (verbose=false)                                   groupToolUses.ts:73

Điều kiện SINGLE (không group):
  ❌ Chỉ có 1 tool call (count < 2)
  ❌ Tool không hỗ trợ grouped rendering
  ❌ Transcript mode (verbose=true → skip grouping)
```

**CollapsedReadSearchGroups (riêng):** Read/Grep/Glob/REPL/MCP → collapse bất kể count (1 cũng collapse), không cần 2+.

```
Grouped (2+ Bash cùng turn):            Single (1 tool hoặc verbose):
┌─ Bash ───────────────────────┐       ┌─ Bash ───────────────────────┐
│ ✓  Bash (3 calls) ✓         │       │ ●  Bash (cargo build)        │
│ ✓ cargo test     ✓ exit: 0  │       │ ✓ exit: 0                    │
│ ✓ cargo build    ✓ exit: 0  │       └──────────────────────────────┘
│ ✓ cargo clippy   ✓ exit: 0  │
└──────────────────────────────┘
```

Những tool thường được group: Bash, Agent. Edit/Read thường single.

### CollapsedReadSearchGroups

Read/Grep/Glob/REPL calls get **collapsed** into a single compact row (bất kể số lượng):

```
Collapsed (default):    Read 5 files  ✓
                        ├─ src/auth.rs
                        ├─ src/token.rs
                        └─ src/main.rs

Collapsed (active):     Searching for 3 patterns  (Ctrl+O to expand)
                        ├─ src/auth.rs
                        ⎿
```

Also supports git operations, memory operations, MCP queries.

---

## 38. Dialog Registry (56 components)

Full list from `claude-code-best` repo:

### Dialogs (Dialog from @anthropic/ink, 37 components)

| # | Component | File | Trigger |
|---|-----------|------|---------|
| 1 | AutoModeOptInDialog | AutoModeOptInDialog.tsx | Shift+Tab |
| 2 | BridgeDialog | BridgeDialog.tsx | Footer pill |
| 3 | BypassPermissionsModeDialog | BypassPermissionsModeDialog.tsx | --dangerously-skip-permissions |
| 4 | ChannelDowngradeDialog | ChannelDowngradeDialog.tsx | Channel switch |
| 5 | ClaudeInChromeOnboarding | ClaudeInChromeOnboarding.tsx | First Chrome |
| 6 | ClaudeMdExternalIncludesDialog | ClaudeMdExternalIncludesDialog.tsx | Setup |
| 7 | CostThresholdDialog | CostThresholdDialog.tsx | $5 spend |
| 8 | DevChannelsDialog | DevChannelsDialog.tsx | Dev flag |
| 9 | ExportDialog | ExportDialog.tsx | /export |
| 10 | HistorySearchDialog | HistorySearchDialog.tsx | Ctrl+R |
| 11 | IdeAutoConnectDialog | IdeAutoConnectDialog.tsx | Setup |
| 12 | IdeOnboardingDialog | IdeOnboardingDialog.tsx | First IDE |
| 13 | IdleReturnDialog | IdleReturnDialog.tsx | Post-idle |
| 14 | InvalidConfigDialog | InvalidConfigDialog.tsx | Startup |
| 15 | InvalidSettingsDialog | InvalidSettingsDialog.tsx | Startup |
| 16 | MCPServerApprovalDialog | MCPServerApprovalDialog.tsx | Startup |
| 17 | MCPServerDesktopImportDialog | MCPServerDesktopImportDialog.tsx | /mcp desktop |
| 18 | MCPServerMultiselectDialog | MCPServerMultiselectDialog.tsx | Setup |
| 19 | RemoteEnvironmentDialog | RemoteEnvironmentDialog.tsx | /remote-env |
| 20 | TeleportRepoMismatchDialog | TeleportRepoMismatchDialog.tsx | Teleport |
| 21 | WorkflowMultiselectDialog | WorkflowMultiselectDialog.tsx | install-github-app |
| 22 | WorktreeExitDialog | WorktreeExitDialog.tsx | exit in worktree |
| 23 | TeamsDialog | teams/TeamsDialog.tsx | Footer pill |
| 24 | BackgroundTasksDialog | tasks/BackgroundTasksDialog.tsx | Footer pill |
| 25 | UltraplanChoiceDialog | ultraplan/UltraplanChoiceDialog.tsx | Ultraplan |
| 26 | ElicitationDialog | mcp/ElicitationDialog.tsx | MCP resource |
| 27 | DiffDialog | diff/DiffDialog.tsx | /diff |
| 28 | SkillsMenu | skills/SkillsMenu.tsx | /skills |
| 29 | GlobalSearchDialog | GlobalSearchDialog.tsx | Ctrl+Shift+F |
| 30 | QuickOpenDialog | QuickOpenDialog.tsx | Ctrl+Shift+P |
| 31 | AssistantSessionChooser | assistant/AssistantSessionChooser.tsx | multi-session |
| 32 | SnapshotUpdateDialog | agents/SnapshotUpdateDialog.tsx | Agent memory |
| 33 | WizardDialogLayout | wizard/WizardDialogLayout.tsx | Wizard install |
| 34 | GroveDialog | grove/Grove.tsx | Setup |
| 35 | TrustDialog | TrustDialog/TrustDialog.tsx | Setup |
| 36 | Onboarding | Onboarding.tsx | First start |
| 37 | ApproveApiKey | ApproveApiKey.tsx | ANTHROPIC_API_KEY |

### Pane/Modal-Slot Components (18 components)

| # | Component | File | Command |
|---|-----------|------|---------|
| 38 | ModelPicker | ModelPicker.tsx | /model or Meta+P |
| 39 | ThinkingToggle | ThinkingToggle.tsx | keybinding |
| 40 | ThemePicker | ThemePicker.tsx | /theme |
| 41 | ConfigTabs | Settings/Config.tsx | /config |
| 42 | SettingsTabs | Settings/Settings.tsx | /settings |
| 43 | StatusTab | Settings/Status.tsx | /status |
| 44 | Stats | Stats.tsx | /stats |
| 45 | SandboxSettings | sandbox/SandboxSettings.tsx | /sandbox |
| 46 | PermissionRuleList | permissions/rules/PermissionRuleList.tsx | permissions |
| 47 | ShowInIDEPrompt | ShowInIDEPrompt.tsx | /ide |
| 48 | Passes | Passes/Passes.tsx | /passes |
| 49 | HelpV2 | HelpV2/HelpV2.tsx | /help |
| 50 | WebTools | commands/web-tools/web-tools.tsx | /web |
| 51 | AutonomyPanel | commands/autonomyPanel.tsx | /autonomy |
| 52 | SkillPanel | commands/skill-learning/skillPanel.tsx | /skill-learning |
| 53 | SkillSearchPanel | commands/skill-search/skillSearchPanel.tsx | /skill-search |
| 54 | MCPListPanel | mcp/MCPListPanel.tsx | /mcp list |
| 55 | WorkflowsPanel | workflow/panel/WorkflowsPanel.tsx | /workflows |

### Overlay IDs

All components use `useRegisterOverlay(id)` for Escape-key coordination:
`assistant-session-chooser`, `autonomy-panel`, `bridge-disconnect-dialog`, `remote-control-server-dialog`, `skill-panel`, `skill-search-panel`, `bridge-dialog`, `multi-select`, `select`, `global-search`, `history-search`, `pipe-selector`, `quick-open`, `diff-dialog`, `elicitation`, `elicitation-url`, `background-tasks-dialog`, `teams-dialog`, `ultraplan-choice`, `autocomplete` (non-modal)

---

## 39. Features NOT in Claude Code

These MASTER_UI.md features are spec'd but **CC actually doesn't have them** → không cần migrate cho jcode:

| Feature | CC Reality | jcode | Action |
|---------|-----------|-------|--------|
| Which-Keys Panel (§19) | ❌ `/help` = text panel | ✅ Help overlay | Giữ nguyên |
| Mermaid Diagram Pane (§23) | ❌ Not found | ✅ Mermaid side panel | Giữ nguyên |
| File Tree Sidebar (§39) | ❌ QuickOpen/GlobalSearch modal | ✅ @-mention picker | Giữ nguyên |
| Toast Notifications (§36) | ❌ Inline footer text only | ✅ System messages | Giữ nguyên |
| Copy Selection Mode (§34) | ❌ Standard terminal | ✅ Viewport copy | Giữ nguyên |
| Terminal Pets (§59) | ❌ Not found | ✅ jcode-tui-anim | Giữ nguyên |
| Changelog Dialog (§43) | ❌ Not found | ✅ Changelog overlay | Giữ nguyên |

---

## 40. Migration Priority Map

### 🔴 Phase 1 — Layout Alignment (P0)

| # | Feature | CC File | jcode File | Miêu tả |
|---|---------|---------|------------|---------|
| 1 | **Move StatusLine xuống dưới input** | `PromptInputFooter.tsx` → `StatusLine.tsx` | `ui_input.rs` (header bar) | StatusLine phải nằm trong cùng container với input, không ở header |
| 2 | **Bottom slot maxHeight=50%** | `FullscreenLayout.tsx:393` | `ui.rs` | Input + footer không chiếm quá 50% terminal |
| 3 | **NewMessagesPill overlay** | `FullscreenLayout.tsx:469-481` | `ui.rs` + `ui_viewport.rs` | "N new messages" pill overlay khi scroll lên |
| 4 | **SuggestionsOverlay absolute** | `FullscreenLayout.tsx:523-536` | `ui_input.rs` | Float trên input, bottom="100%", không chiếm space |

### 🟠 Phase 2 — Message UX (P1)

| # | Feature | CC File | jcode File |
|---|---------|---------|------------|
| 5 | **Thinking block 3-state toggle** | `AssistantThinkingMessage.tsx` | `ui_messages.rs` |
| 6 | **Tool loader dot** (● blinking) | `ToolUseLoader.tsx` | `ui_tools.rs` |
| 7 | **StatusLine hiding logic** | `PromptInputFooter.tsx:112-113` | `ui_input.rs` |
| 8 | **CachePill + countdown** | `StatusLine.tsx:71-158` | `ui_input.rs` + `mod.rs` |

### 🟡 Phase 3 — Tool Call (P2)

| # | Feature | CC File | jcode File |
|---|---------|---------|------------|
| 9 | **CollapsedReadSearchGroups** | `CollapsedReadSearchContent.tsx` | `ui_messages.rs` |
| 10 | **GroupedToolUse** | `GroupedToolUseContent.tsx` | `ui_messages.rs` + `ui_tools.rs` |
| 11 | **ExpandedShellOutputContext** | `ExpandShellOutputContext.tsx` | `ui_tools.rs` |
| 12 | **UserPrompt truncation** (>10K) | `UserPromptMessage.tsx` | `ui_messages.rs` |

### 🟢 Phase 4 — Polish (P3)

| # | Feature | CC File | jcode File |
|---|---------|---------|------------|
| 13 | **StickyPromptHeader** (1-row pinned, onClick scrollTo) | `FullscreenLayout.tsx:495` | `ui.rs` |
| 14 | **VirtualMessageList click-to-expand + hover** | `VirtualMessageList.tsx` | `ui_viewport.rs` |
| 15 | **Turn footer metadata** | Assistant message footer | `ui_messages.rs` |

---

## 41. Appendix: jcode ↔ CC File Mapping

### jcode Core Files

| jcode File | Lines | Purpose | CC Equivalent |
|------------|-------|---------|---------------|
| `crates/jcode-tui/src/tui/ui.rs` | 3178 | Main layout, draw orchestration | `FullscreenLayout.tsx` |
| `crates/jcode-tui/src/tui/ui_input.rs` | 2504 | Composer, status bar, suggestions | `PromptInput/PromptInput.tsx`, `StatusLine.tsx` |
| `crates/jcode-tui/src/tui/ui_messages.rs` | 2114 | Message rendering (10+ roles) | `messages/*.tsx` (11 files) |
| `crates/jcode-tui/src/tui/ui_header.rs` | 1626 | Header bar (model, auth, version) | `BuiltinStatusLine.tsx` (partial) |
| `crates/jcode-tui/src/tui/ui_tools.rs` | 1381 | Tool call display, batch, diff | `AssistantToolUseMessage.tsx` |
| `crates/jcode-tui/src/tui/ui_viewport.rs` | 1314 | Scroll, copy selection, mouse | `VirtualMessageList.tsx` |
| `crates/jcode-tui/src/tui/ui_pinned.rs` | 1993 | Side panel (diff, mermaid, images) | NOT in CC |
| `crates/jcode-tui/src/tui/ui_overlays.rs` | 1286 | Account picker, model, help, changelog | `ModelPicker.tsx`, `HelpV2/` |
| `crates/jcode-tui/src/tui/info_widget.rs` | 2134 | +17 sub-files (todos, git, swarm, etc.) | `TaskListV2.tsx`, `TeamsDialog.tsx` |
| `crates/jcode-tui/src/tui/ui_running_items.rs` | 270 | Background tasks, timers | `BackgroundTasksDialog.tsx` |
| `crates/jcode-tui/src/tui/session_picker.rs` | 2168 | Session list + resume | `ResumeConversation.tsx` |
| `crates/jcode-tui/src/tui/app.rs` | ~2000 | App root, event loop | `REPL.tsx` |
| `crates/jcode-tui/src/tui/mod.rs` | ~600 | TuiState trait, types | `src/types/message.ts` |

### jcode TUI Crates (22 crates)

| Crate | Purpose | CC Equivalent |
|-------|---------|---------------|
| `jcode-tui-markdown` | Syntax highlighting, math, tables | `Markdown` component |
| `jcode-tui-messages` | DisplayMessage types | `src/types/message.ts` |
| `jcode-tui-style` | Theme colors, spinner | `ThemeProvider`, `Spinner.tsx` |
| `jcode-tui-mermaid` | Mermaid diagram rendering | ❌ Not in CC |
| `jcode-tui-permissions` | Permission dialog (867 lines) | `permissions/*.tsx` |
| `jcode-tui-anim` | Idle animations (donut, gyro, etc.) | ❌ Not in CC |
| `jcode-tui-session-picker` | Session list | `ResumeConversation.tsx` |
| `jcode-tui-account-picker` | Account switching | `ApproveApiKey.tsx` |
| `jcode-tui-usage-overlay` | Cost/usage display | `Settings/Usage.tsx` |
| `jcode-tui-workspace` | Workspace map | ❌ Not in CC |
| `jcode-tui-tool-display` | Tool rendering | `tool.renderToolResultMessage()` |
| `jcode-tui-core` | Copy selection, scroll traits | `useCopyOnSelect.ts` |
| `jcode-tui-render` | Render pipeline | VirtualMessageList |

---

---

## 42. Mouse & Click Interactions (Claude Code style)

### 42.1 CC Click/Hover Inventory

Claude Code (Ink/React) hỗ trợ **6 loại mouse interaction** — KHÔNG chỉ keyboard:

| # | Interaction | CC File | Trigger | Behavior |
|---|-------------|---------|---------|----------|
| 1 | **NewMessagesPill click** | `FullscreenLayout.tsx:469` | `onClick` | Scroll down to bottom |
| 2 | **NewMessagesPill hover** | `FullscreenLayout.tsx:470` | `onMouseEnter/Leave` | Highlight pill bg |
| 3 | **StickyPromptHeader click** | `FullscreenLayout.tsx:495` | `onClick` | Scroll to that prompt |
| 4 | **Message click-to-expand** | `VirtualMessageList.tsx:238` | `onClick` | Toggle verbose mode |
| 5 | **Message hover highlight** | `VirtualMessageList.tsx:239-240` | `onMouseEnter/Leave` | Highlight clickable msg |
| 6 | **Input area click focus** | `PromptInput.tsx:2492` | `onClick` | Set cursor position |
| 7 | **System file path click** | `SystemTextMessage.tsx:377` | `onClick` | Open file |
| 8 | **Drag-to-scroll edge** | `ScrollKeybindingHandler.tsx:349` | mouse drag past viewport | Auto-scroll |
| 9 | **Mouse wheel smooth scroll** | `ScrollKeybindingHandler.tsx` | `ScrollUp/Down` | Smooth momentum scroll |
| 10 | **Terminal selection copy** | `useCopyOnSelect.ts` | Mouse drag select | Copy to clipboard |

### CC Implementation Pattern (Ink/React)

```tsx
// === Pattern 1: Click + Hover ===
const [hover, setHover] = useState(false);
<Box onClick={onClick}
     onMouseEnter={() => setHover(true)}
     onMouseLeave={() => setHover(false)}>
  <Text backgroundColor={hover ? 'hoverColor' : 'baseColor'}>
    Content
  </Text>
</Box>

// === Pattern 2: Message click-to-expand ===
// Only clickable when verbose toggle reveals more content
const clickable = isItemClickable?.(msg) ?? true;
<Box onClick={clickable ? e => onClickK(msg, e.cellIsBlank) : undefined}
     onMouseEnter={clickable ? () => onEnterK(k) : undefined}
     onMouseLeave={clickable ? () => onLeaveK(k) : undefined}>
</Box>

// === Pattern 3: Drag-to-scroll ===
// useDragToScroll hook — when mouse drags past viewport edge,
// auto-scroll at AUTOSCROLL_INTERVAL_MS rate
```

### 42.2 jcode Current Mouse Support

jcode đã sử dụng `crossterm::event::MouseEvent` với `EnableMouseCapture`:

| Có sẵn | File | Note |
|--------|------|------|
| ✅ Mouse wheel smooth scroll | `app.rs:1315-1317` | `mouse_scroll_queue` + easing |
| ✅ Copy selection via drag | `navigation.rs:1175` | `handle_copy_selection_mouse()` |
| ✅ Config toggle | `config: mouse_capture: bool` | Mặc định `true` |
| ✅ Session picker scroll | `session_picker.rs:2112` | Wheel = scroll |
| ✅ Scrollbar (chat) | `ui.rs` | runtime-detected native scrollbar |
| ❌ **Click → expand message** | — | Cần implement hit-test |
| ❌ **Hover highlight** | — | Cần message rect tracking |
| ❌ **NewMessagesPill click** | — | Chưa có |
| ❌ **Drag-to-scroll edge** | — | Chưa có |

### 42.3 ratatui Mouse Implementation Approach

ratatui 0.30 **không có onClick/onHover built-in** (immediate-mode rendering). jcode phải tự xử lý:

```rust
// crossterm event loop already dispatches MouseEvent
// Need to add: message rect tracking + hit-test

// === Step 1: Store clickable message rects during render ===
struct ClickTarget {
    msg_id: String,
    rect: Rect,          // (x, y, w, h) in terminal cells
    action: ClickAction, // ToggleVerbose, ScrollTo, etc.
}

enum ClickAction {
    ToggleVerbose(String),   // message_id
    NewMessagesJumpBottom,
    StickyScrollTo(String),
}

// === Step 2: Hit-test on mouse click ===
fn handle_mouse_click(app: &mut App, mouse: MouseEvent) {
    if mouse.kind != MouseEventKind::Down(MouseButton::Left) { return; }
    let col = mouse.column;
    let row = mouse.row;

    for target in &app.click_targets {
        if col >= target.rect.x
            && col < target.rect.x + target.rect.width
            && row >= target.rect.y
            && row < target.rect.y + target.rect.height
        {
            match &target.action {
                ClickAction::ToggleVerbose(msg_id) => {
                    app.toggle_message_verbose(msg_id);
                }
                ClickAction::NewMessagesJumpBottom => {
                    app.scroll_to_bottom();
                }
                ClickAction::StickyScrollTo(msg_id) => {
                    app.scroll_to_message(msg_id);
                }
            }
            return;
        }
    }
}

// === Step 3: crossterm MouseEvent type ===
// MouseEvent {
//   kind: MouseEventKind::Down(MouseButton::Left),
//       | MouseEventKind::Up(MouseButton::Left)
//       | MouseEventKind::Drag(MouseButton::Left)  // during drag-select
//       | MouseEventKind::Moved                      // terminal selection
//       | MouseEventKind::ScrollUp
//       | MouseEventKind::ScrollDown,
//   column: u16,   // X coordinate in terminal cells
//   row: u16,      // Y coordinate
//   modifiers: KeyModifiers,
//   code: Option<MouseButton>,  // Kitty protocol extended info
// }
```

### 42.4 Integration vào jcode navigation.rs

jcode đã có `App::handle_mouse_event` trong `navigation.rs` (phân phối scroll). Cần thêm:

```rust
// navigation.rs — thêm vào handle_mouse_event()
fn handle_mouse_event(&mut self, mouse: MouseEvent) -> bool {
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            // Hit-test against click targets
            if self.handle_message_click(mouse.column, mouse.row) {
                return true;
            }
        }
        MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
            // Existing smooth scroll (already works)
            self.enqueue_mouse_scroll(...);
        }
        _ => {
            // Existing copy selection (already works)
            self.handle_copy_selection_mouse(mouse);
        }
    }
}
```

### 42.5 jcode Click Targets Rendering Pattern

```rust
// Trong ui.rs, mỗi message render xong → lưu Rect để hit-test
pub struct ClickTargets {
    targets: Vec<ClickTarget>,
}

impl ClickTargets {
    pub fn push<T: Into<ClickAction>>(&mut self, action: T, rect: Rect) {
        self.targets.push(ClickTarget {
            msg_id: /* unique ID */,
            rect,
            action: action.into(),
        });
    }

    pub fn hit_test(&self, col: u16, row: u16) -> Option<&ClickAction> {
        self.targets
            .iter()
            .find(|t| col >= t.rect.x && col < t.rect.x + t.rect.width
                   && row >= t.rect.y && row < t.rect.y + t.rect.height)
            .map(|t| &t.action)
    }
}

// Dùng trong draw:
// let mut targets = ClickTargets::new();
// render_message(frame, msg, area, &mut targets);
// app.set_click_targets(targets);
```

### 42.6 Mouse vs Keyboard UX comparison

| UX | Trước (chỉ keyboard) | Sau (mouse + keyboard) | Như CC? |
|----|---------------------|------------------------|---------|
| New messages | Ctrl+G nhảy xuống | Click "N new msgs" pill | ✅ |
| Expand message | Ctrl+E | Click message | ✅ |
| Sticky header | — (chưa có) | Click header → scroll to prompt | ✅ |
| Scroll | PgUp/PgDn/j/k | Mouse wheel (đã có) | ✅ |
| Copy text | Ctrl+C | Mouse drag select (đã có) | ✅ |
| Drag scroll | — | Drag past edge → auto scroll | ⬜ Future |

---

## 43. Appendix: Ratatui Implementation Reference (v0.30.2)

> **Source:** `ratatui-core-0.1.2` + `ratatui-widgets-0.3.2` tại `~/.cargo/registry/`
> **Cấu trúc:** ratatui 0.30.x được modular hóa thành workspace: `ratatui` (re-export), `ratatui-core` (core traits), `ratatui-widgets` (built-in widgets)

### 43.1 ratatui-core API Reference

**Frame** (`ratatui-core/src/terminal/frame.rs`):
```rust
pub struct Frame<'a> {
    pub(crate) cursor_position: Option<Position>,
    pub(crate) viewport_area: Rect,
    pub(crate) buffer: &'a mut Buffer,
    pub(crate) count: usize,
}

impl Frame<'_> {
    pub const fn area(&self) -> Rect { self.viewport_area }
    pub fn render_widget(&mut self, widget: impl Widget, area: Rect) { ... }
    pub fn render_stateful_widget(&mut self, widget: impl StatefulWidget, area: Rect, state: &mut widget::State) { ... }
    pub fn set_cursor(&mut self, position: Position) { ... }
    pub fn buffer(&self) -> &Buffer { self.buffer }
    pub fn count(&self) -> usize { self.count }
}
```

**Rect** (`ratatui-core/src/layout/rect/rect.rs`):
```rust
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    pub const fn new(x: u16, y: u16, width: u16, height: u16) -> Self;
    pub fn area(self) -> u32;
    pub fn is_empty(self) -> bool;
    pub fn left(self) -> u16;
    pub fn right(self) -> u16;
    pub fn top(self) -> u16;
    pub fn bottom(self) -> u16;
    pub fn inner(self, margin: &Margin) -> Rect;
    pub fn offset(self, offset: Offset) -> Rect;
    pub fn union(self, other: Rect) -> Rect;
    pub fn intersection(self, other: Rect) -> Rect;
    pub fn clamp(self, other: Rect) -> Rect;
}
```

**Layout** (`ratatui-core/src/layout/layout.rs`):
```rust
pub struct Layout {
    direction: Direction,
    constraints: Vec<Constraint>,
    flex: Flex,
    spacing: Spacing,
    margin: Margin,
}

impl Layout {
    pub fn new() -> Self;
    pub fn direction(mut self, direction: Direction) -> Self;
    pub fn constraints<C: AsRef<[Constraint]>>(mut self, constraints: C) -> Self;
    pub fn flex(mut self, flex: Flex) -> Self;
    pub fn spacing(mut self, spacing: impl Into<Spacing>) -> Self;
    pub fn margin(mut self, margin: impl Into<Margin>) -> Self;
    pub fn split(self, area: Rect) -> Rc<[Rect]>;     // Uses kasuari constraint solver
    pub fn split_spacers(self, area: Rect) -> (Rc<[Rect]>, Rc<[Rect]>);
}
```

**Constraint** (`ratatui-core/src/layout/constraint.rs`):
```rust
pub enum Constraint {
    Min(u16),        // At least this size
    Max(u16),        // At most this size
    Length(u16),     // Exact length
    Percentage(u16), // Percentage of total area
    Ratio(u32, u32), // Ratio of total area
    Fill(u16),       // Proportional fill
}
```

**Direction** (`ratatui-core/src/layout/direction.rs`):
```rust
pub enum Direction { Horizontal, Vertical }
```

**Alignment** (`ratatui-core/src/layout/alignment.rs`):
```rust
pub enum HorizontalAlignment { Left, Center, Right }
pub enum VerticalAlignment { Top, Center, Bottom }
pub type Alignment = HorizontalAlignment;   // backwards compat
```

**Style** (`ratatui-core/src/style/style.rs`):
```rust
pub struct Style {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub underline_color: Option<Color>,
    pub add_modifier: Modifier,
    pub sub_modifier: Modifier,
}

impl Style {
    pub const fn new() -> Self;
    pub const fn fg(mut self, fg: Color) -> Self;
    pub const fn bg(mut self, bg: Color) -> Self;
    pub const fn add_modifier(mut self, modifier: Modifier) -> Self;
    pub const fn sub_modifier(mut self, modifier: Modifier) -> Self;
    pub const fn underline_color(mut self, underline_color: Color) -> Self;
}
```

**Color** (`ratatui-core/src/style/color.rs`):
```rust
pub enum Color {
    Reset,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    Gray,
    DarkGray,
    LightRed,
    LightGreen,
    LightYellow,
    LightBlue,
    LightMagenta,
    LightCyan,
    White,
    Rgb(u8, u8, u8),
    Indexed(u8),
}
```

**Modifier** (`ratatui-core/src/style/stylize.rs`):
```rust
pub enum Modifier {
    BOLD           = 0b0000_0000_0000_0001,
    DIM            = 0b0000_0000_0000_0010,
    ITALIC         = 0b0000_0000_0000_0100,
    UNDERLINED     = 0b0000_0000_0000_1000,
    SLOW_BLINK     = 0b0000_0000_0001_0000,
    RAPID_BLINK    = 0b0000_0000_0010_0000,
    REVERSED       = 0b0000_0000_0100_0000,
    HIDDEN         = 0b0000_0000_1000_0000,
    CROSSED_OUT    = 0b0000_0001_0000_0000,
}
```

**Buffer** (`ratatui-core/src/buffer/buffer.rs`):
```rust
impl Buffer {
    pub fn area(&self) -> Rect;
    pub fn content(&self) -> &[Cell];
    pub fn cell_count(&self) -> usize;
    pub fn set_string(&mut self, x: u16, y: u16, string: &str, style: Style);
    pub fn set_style(&mut self, area: Rect, style: Style);
    pub fn set_symbol(&mut self, x: u16, y: u16, symbol: &str);
    pub fn fill(&mut self, area: Rect, symbol: &str, style: Style);
}
```

**Widget & StatefulWidget traits** (`ratatui-core/src/widgets/widget.rs`):
```rust
pub trait Widget {
    fn render(self, area: Rect, buf: &mut Buffer);
}

pub trait StatefulWidget {
    type State;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State);
}
```

**Text primitives** (`ratatui-core/src/text/`):
```rust
pub struct Line<'a>(pub Vec<Span<'a>>);
impl<'a> Line<'a> {
    pub fn from<T: Into<Cow<'a, str>>>(text: T) -> Self;
    pub fn styled(span: Span<'a>, style: Style) -> Self;
    pub fn alignment(mut self, alignment: Alignment) -> Self;
    pub fn left_aligned(mut self) -> Self;
    pub fn centered(mut self) -> Self;
    pub fn right_aligned(mut self) -> Self;
    pub fn width(&self) -> usize;
}

pub struct Span<'a> {
    pub content: Cow<'a, str>,
    pub style: Style,
}
impl<'a> Span<'a> {
    pub fn from<T: Into<Cow<'a, str>>>(content: T) -> Self;
    pub fn styled<T: Into<Cow<'a, str>>>(content: T, style: Style) -> Self;
    pub fn width(&self) -> usize;
}

pub struct Text<'a>(pub Vec<Line<'a>>);
impl<'a> Text<'a> {
    pub fn from<T: Into<Cow<'a, str>>>(text: T) -> Self;
    pub fn width(&self) -> usize;
    pub fn height(&self) -> usize;
}
```

### 43.2 ratatui-widgets API Reference

**Block** (`ratatui-widgets/src/block/block.rs`):
```rust
pub struct Block<'a> {
    pub(crate) titles: Vec<(TitlePosition, Line<'a>)>,  // (position, title_line)
    pub(crate) style: Style,
    pub(crate) borders: Borders,
    pub(crate) border_type: BorderType,
    pub(crate) border_style: Style,
    pub(crate) border_set: border::Set,
    pub(crate) padding: Padding,
    pub(crate) shadow: Option<Shadow>,
}

impl<'a> Block<'a> {
    pub fn new() -> Self;
    pub fn bordered() -> Self;                          // all borders enabled
    pub const fn title(mut self, title: impl Into<Line<'a>>) -> Self;  // top, default align
    pub fn title_top(mut self, title: impl Into<Line<'a>>) -> Self;
    pub fn title_bottom(mut self, title: impl Into<Line<'a>>) -> Self;
    pub fn title_alignment(mut self, alignment: Alignment) -> Self;
    pub fn title_style<S: Into<Style>>(mut self, style: S) -> Self;
    pub fn title_position(mut self, position: TitlePosition) -> Self;
    pub fn style(mut self, style: Style) -> Self;
    pub fn borders(mut self, borders: Borders) -> Self;
    pub fn border_style(mut self, style: Style) -> Self;
    pub fn border_type(mut self, border_type: BorderType) -> Self;
    pub fn border_set(mut self, set: border::Set) -> Self;
    pub fn padding(mut self, padding: Padding) -> Self;
    pub fn shadow(mut self, shadow: Shadow) -> Self;
    pub fn inner(&self, area: Rect) -> Rect;            // Calculate content area
}
```

**BorderType:**
```rust
pub enum BorderType { Plain, Rounded, Double, Quadruple, Thick }
```

**Borders:**
```rust
pub enum Borders { NONE, LEFT, RIGHT, TOP, BOTTOM, ALL }
```

**Paragraph** (`ratatui-widgets/src/paragraph/paragraph.rs`):
```rust
pub struct Paragraph<'a> {
    block: Option<Block<'a>>,
    style: Style,
    text: Text<'a>,
    wrap: Option<Wrap>,
    scroll: (u16, u16),                             // (row, col) scroll offset
    alignment: Alignment,
    leftmost_column: u16,                            // column alignment
}

impl<'a> Paragraph<'a> {
    pub fn new<T: Into<Text<'a>>>(text: T) -> Self;
    pub fn block(mut self, block: Block<'a>) -> Self;
    pub fn style<S: Into<Style>>(mut self, style: S) -> Self;
    pub fn wrap(mut self, wrap: Wrap) -> Self;
    pub fn scroll(mut self, offset: (u16, u16)) -> Self;
    pub fn alignment(mut self, alignment: Alignment) -> Self;
    pub fn line_count(&self, width: u16) -> usize;   // Count wrapped lines
    pub fn line_width(&self) -> usize;                // Max line width
}

impl Widget for Paragraph<'_> { ... }
impl Widget for &Paragraph<'_> { ... }

pub struct Wrap { pub trim: bool }
```

**List** (`ratatui-widgets/src/list/list.rs`):
```rust
pub struct List<'a> {
    block: Option<Block<'a>>,
    items: Vec<ListItem<'a>>,
    style: Style,
    highlight_style: Style,
    highlight_symbol: Option<Line<'a>>,
    repeat_highlight_symbol: bool,
    direction: ListDirection,
    len: usize,
}

impl<'a> List<'a> {
    pub fn new<T: Into<ListItem<'a>>>(items: Vec<T>) -> Self;
    pub fn items<T: Into<ListItem<'a>>>(mut self, items: Vec<T>) -> Self;
    pub fn block(mut self, block: Block<'a>) -> Self;
    pub fn style<S: Into<Style>>(mut self, style: S) -> Self;
    pub fn highlight_style<S: Into<Style>>(mut self, style: S) -> Self;
    pub fn highlight_symbol<L: Into<Line<'a>>>(mut self, highlight_symbol: L) -> Self;
    pub fn repeat_highlight_symbol(mut self, repeat: bool) -> Self;
    pub fn direction(mut self, direction: ListDirection) -> Self;
}

impl StatefulWidget for List<'_> { type State = ListState; ... }

pub struct ListItem<'a> {
    content: Text<'a>,
    style: Style,
}
impl<'a> ListItem<'a> {
    pub fn new<T: Into<Text<'a>>>(content: T) -> Self;
    pub fn style<S: Into<Style>>(mut self, style: S) -> Self;
}

pub struct ListState {
    pub offset: usize,     // Scroll offset
    pub selected: Option<usize>, // Selected item index (None = no selection)
}
impl ListState { pub fn new() -> Self; pub fn default() -> Self; pub fn selected(&self) -> Option<usize>; pub fn select(&mut self, index: Option<usize>); }
```

**Table** (`ratatui-widgets/src/table/table.rs`):
```rust
pub struct Table<'a> {
    block: Option<Block<'a>>,
    rows: Vec<Row<'a>>,
    widths: &'a [Constraint],
    column_spacing: u16,
    style: Style,
    highlight_style: Style,
    highlight_symbol: Option<Line<'a>>,
}

impl<'a> Table<'a> {
    pub fn new<T: Into<Vec<Row<'a>>>>(rows: T, widths: &'a [Constraint]) -> Self;
    pub fn block(mut self, block: Block<'a>) -> Self;
    pub fn style<S: Into<Style>>(mut self, style: S) -> Self;
    pub fn widths(mut self, widths: &'a [Constraint]) -> Self;
    pub fn column_spacing(mut self, spacing: u16) -> Self;
    pub fn highlight_style<S: Into<Style>>(mut self, style: S) -> Self;
    pub fn highlight_symbol<L: Into<Line<'a>>>(mut self, highlight_symbol: L) -> Self;
}
impl StatefulWidget for Table<'_> { type State = TableState; ... }

pub struct Row<'a> {
    pub(crate) cells: Vec<Cell<'a>>,
    pub(crate) height: u16,
    pub(crate) style: Style,
    pub(crate) bottom_margin: u16,
}
impl<'a> Row<'a> {
    pub fn new<T: Into<Cell<'a>>>(cells: Vec<T>) -> Self;
    pub fn style<S: Into<Style>>(mut self, style: S) -> Self;
    pub fn height(mut self, height: u16) -> Self;
    pub fn bottom_margin(mut self, margin: u16) -> Self;
}

pub struct Cell<'a> {
    pub(crate) content: Text<'a>,
    pub(crate) style: Style,
    pub(crate) alignment: Option<Alignment>,
}
impl<'a> Cell<'a> {
    pub fn new<T: Into<Text<'a>>>(content: T) -> Self;
    pub fn style<S: Into<Style>>(mut self, style: S) -> Self;
    pub fn alignment(mut self, alignment: Alignment) -> Self;
}
```

**Scrollbar** (`ratatui-widgets/src/scrollbar/scrollbar.rs`):
```rust
pub struct Scrollbar<'a> {
    orientation: ScrollbarOrientation,
    thumb_symbol: &'a str,
    thumb_style: Style,
    track_symbol: Option<&'a str>,
    track_style: Style,
    begin_symbol: Option<&'a str>,
    begin_style: Style,
    end_symbol: Option<&'a str>,
    end_style: Style,
}

impl Scrollbar<'_> {
    pub fn new(orientation: ScrollbarOrientation) -> Self;
    pub const fn orientation(mut self, orientation: ScrollbarOrientation) -> Self;
    pub fn thumb_symbol(mut self, thumb_symbol: &'a str) -> Self;
    pub fn thumb_style<S: Into<Style>>(mut self, thumb_style: S) -> Self;
    pub fn track_symbol(mut self, track_symbol: Option<&'a str>) -> Self;
    pub fn track_style<S: Into<Style>>(mut self, track_style: S) -> Self;
    pub fn begin_symbol(mut self, begin_symbol: Option<&'a str>) -> Self;
    pub fn begin_style<S: Into<Style>>(mut self, begin_style: S) -> Self;
    pub fn end_symbol(mut self, end_symbol: Option<&'a str>) -> Self;
    pub fn end_style<S: Into<Style>>(mut self, end_style: S) -> Self;
}
impl Widget for Scrollbar<'_> { ... }
impl StatefulWidget for &Scrollbar<'_> { type State = ScrollbarState; ... }

pub enum ScrollbarOrientation { VerticalRight, VerticalLeft, HorizontalBottom, HorizontalTop }
pub struct ScrollbarState { content_length: usize, position: usize, viewport_content_length: usize }
impl ScrollbarState { pub fn new(content_length: usize) -> Self;
    pub fn position(mut self, position: usize) -> Self;
    pub fn viewport_content_length(mut self, length: usize) -> Self;
    pub fn scroll(&mut self, direction: ScrollDirection); }
pub enum ScrollDirection { Forward, Backward }
```

**Gauge** (`ratatui-widgets/src/gauge/gauge.rs`):
```rust
pub struct Gauge<'a> {
    block: Option<Block<'a>>,
    ratio: f64,
    label: Option<Line<'a>>,
    style: Style,
    gauge_style: Style,
}
impl Gauge<'_> {
    pub fn new(ratio: f64) -> Self;                          // 0.0 to 1.0
    pub fn block(mut self, block: Block<'a>) -> Self;
    pub fn label<T: Into<Line<'a>>>(mut self, label: T) -> Self;
    pub fn ratio(mut self, ratio: f64) -> Self;
    pub fn style<S: Into<Style>>(mut self, style: S) -> Self;
    pub fn gauge_style<S: Into<Style>>(mut self, gauge_style: S) -> Self;
}

pub struct LineGauge<'a> {
    block: Option<Block<'a>>,
    ratio: f64,
    label: Option<Line<'a>>,
    style: Style,
    filled_style: Style,
    unfilled_style: Style,
    line_set: symbols::line::Set,
}
```

**Tabs** (`ratatui-widgets/src/tabs/tabs.rs`):
```rust
pub struct Tabs<'a> {
    titles: Vec<Line<'a>>,
    block: Option<Block<'a>>,
    style: Style,
    highlight_style: Style,
    divider: Option<Line<'a>>,
}
impl StatefulWidget for Tabs<'_> { type State = TabsState; ... }
pub struct TabsState { pub selected: usize }
impl TabsState { pub const fn new(selected: usize) -> Self; }
```

### 43.3 How Claude Code React/Ink Patterns Map to ratatui

| React/Ink | ratatui equivalent |
|-----------|-------------------|
| `<Box flexDirection="column" flexGrow={1}>` | `Layout::default().direction(Direction::Vertical).constraints([Constraint::Fill(1), ...]).split(area)` |
| `<Box flexShrink={0}>` | `Constraint::Length(n)` |
| `<Box position="absolute">` | Không có absolute positioning — sử dụng Layout riêng hoặc `Rect` calculculation |
| `<Text color="green">` | `Span::styled(text, Style::default().fg(Color::Green))` |
| `<Text dimColor>` | `Style::default().add_modifier(Modifier::DIM)` |
| `<Box borderStyle="round">` | `Block::bordered().border_type(BorderType::Rounded)` |
| `<Box paddingX={2}>` | `Block::bordered().padding(Padding::horizontal(2))` |
| `<Box justifyContent="center">` | `Layout` với spacing + alignment hoặc paragraph alignment |
| `<Select>` items | `List` + `ListState` |
| `useAnimationFrame` callback | Custom frame loop + timer |
| Typed state store | `struct AppState` với `Enum` processing statuses |
| FlexGrow spacers | `Constraint::Fill(1)` hoặc `Constraint::Min(0)` |

### 43.4 CC Patterns → ratatui Implementation Recipes

#### Recipe 1: FullscreenLayout 3-region

```typescript
// CC (React/Ink)
<Box flexDirection="column" flexGrow={1}>
  <Box flexGrow={1}>       // Upper: scrollbox
    {messages}
    <Box flexGrow={1} />   // Spacer
    <SpinnerWithVerb />
  </Box>
  <Box maxHeight="50%">    // Bottom slot
    <SuggestionsOverlay />
    {promptInput}
  </Box>
</Box>
```

```rust
// ratatui equivalent
let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints(vec![
        Constraint::Fill(1),              // Upper region (scrollable)
        Constraint::Max(area.height / 2),  // Bottom slot: max 50%
    ])
    .split(area);

// Upper region: messages + spacer + spinner
let upper = chunks[0];
let upper_chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints(vec![
        Constraint::Min(1),    // Messages
        Constraint::Fill(1),   // Spacer → pushes spinner down
        Constraint::Length(1), // Spinner
    ])
    .split(upper);
render_messages(frame, upper_chunks[0]);
render_spinner(frame, upper_chunks[2]);

// Bottom slot: suggestions (floating) + input
let bottom = chunks[1];
frame.render_widget(input_widget, bottom);
```

#### Recipe 2: StatusLine inside PromptInputFooter

```typescript
// CC
<Box flexDirection="column">
  <TextInput />                         // PromptInput
  <Box flexDirection="column">          // PromptInputFooter
    <StatusLine />                      // BELOW input
    <PromptInputFooterLeftSide />        // Hints
  </Box>
</Box>
```

```rust
// ratatui equivalent
let input_area = chunks[7];  // from main layout
let footer_lines = 2;        // StatusLine (1) + Hints (1)
let input_lines = input_area.height.saturating_sub(footer_lines);

// Split input area: text input + footer
let input_chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints(vec![
        Constraint::Min(MIN_INPUT_VIEWPORT_LINES),  // Text input
        Constraint::Length(1),                        // StatusLine HERE
        Constraint::Length(1),                        // Hints
    ])
    .split(input_area);

// Render: input text
render_input_text(frame, app, input_chunks[0]);
// Render: StatusLine (model, ctx, cost, pills)
render_status_line(frame, app, input_chunks[1]);
// Render: Hints (keybindings)
render_hints(frame, app, input_chunks[2]);
```

#### Recipe 3: Tool Use Loader Dot

```typescript
// CC: ToolUseLoader.tsx (blinking dot)
function ToolUseLoader({ isUnresolved, shouldAnimate, isError }) {
  return {
    isUnresolved && shouldAnimate ? (visible ? "●" : " ") :
    isUnresolved && !shouldAnimate ? dim("●") :
    isError ? red("●") : green("●")
  }(600ms blink interval);
}
```

```rust
// ratatui equivalent
fn tool_loader_dot(is_unresolved: bool, is_error: bool, frame_visible: bool) -> Span<'static> {
    let (symbol, style) = if is_error {
        ("●", Style::default().fg(Color::Red))
    } else if !is_unresolved {
        ("✓", Style::default().fg(Color::Green))  // success
    } else if frame_visible {
        ("●", Style::default().fg(Color::Gray))
    } else {
        (" ", Style::default())  // hidden during blink
    };
    Span::styled(symbol, style)
}
// Animation driven by frame counter:
// let visible = (frame_count / (60 / 600ms_interval)) % 2 == 0;
```

#### Recipe 4: CollapsedReadSearchGroups

```typescript
// CC: CollapsedReadSearchContent.tsx — live hint with ├─ files
let status = "Searching for 3 patterns";
let files = ["src/auth.rs", "src/token.rs"];
let lastFile = "src/auth.rs";

render(
  <Box marginTop={1}>
    <Text dimColor>
      {status} (Ctrl+O to expand)
      {"\n├─ " + (isActive ? lastFile : files.join("\n├─ "))}
      {"\n⎿"}
    </Text>
  </Box>
);
```

```rust
// ratatui equivalent — single Paragraph
fn render_collapsed_read_group(
    frame: &mut Frame,
    group: &CollapsedReadSearchGroup,
    area: Rect,
) {
    let mut lines = Vec::new();
    let summary = format!("{} {}{}",
        if group.is_active { " " } else { "✓ " },
        group.summary_text,
        if group.is_active { " (Ctrl+O to expand)" } else { " ✓" }
    );
    lines.push(Line::from(Span::styled(summary, Style::default().fg(dark_gray()))));

    // Show live/active files indented
    for f in &group.files_shown {
        let prefix = if *f == group.last_active && group.is_active { "⎿" } else { "├─" };
        lines.push(Line::from(Span::styled(
            format!("{} {}", prefix, f),
            Style::default().fg(dark_gray())
        )));
    }

    frame.render_widget(Paragraph::new(Text::from(lines)), area);
}
```

#### Recipe 5: Bash Permission with Classifier Shimmer

```typescript
// CC: BashPermissionRequest.tsx — shimmer text at 50ms
let subtitle = classifierChecking ? <ClassifierCheckingSubtitle /> : null;
function ClassifierCheckingSubtitle() {
  const shimmer = useShimmerAnimation();  // 50ms wave
  return <Text dimColor>{shimmer("Auto-classifier checking...")}</Text>;
}
```

```rust
// ratatui equivalent — simple pulsing text
fn render_classifier_checking(frame: &mut Frame, area: Rect, frame_count: u64) {
    let phase = (frame_count as f64 * 0.1).sin();  // -1..1 oscillation
    let intensity = ((phase + 1.0) * 0.5 * 155.0 + 100.0) as u8;
    let style = Style::default().fg(Color::Rgb(150, intensity, 180));
    frame.render_widget(
        Paragraph::new("🔍 Auto-classifier checking...").style(style),
        area
    );
}
```

#### Recipe 6: NewMessagesPill

```typescript
// CC: FullscreenLayout.tsx — absolute overlay at ScrollBox bottom
<Box position="absolute" bottom={0} justifyContent="center">
  <Box onClick={scrollDown}>
    <Text dimColor>N new messages / Jump to bottom ↓</Text>
  </Box>
</Box>
```

```rust
// ratatui equivalent — render after messages area
fn render_new_messages_pill(frame: &mut Frame, messages_area: Rect, count: usize) {
    if count == 0 { return; }
    let pill_y = messages_area.bottom().saturating_sub(1);
    let text = format!(" {} new message{} / Jump to bottom ↓ ",
        count, if count > 1 { "s" } else { "" }
    );
    let x = messages_area.x + (messages_area.width.saturating_sub(text.len() as u16)) / 2;
    frame.buffer().set_string(x, pill_y, &text, Style::default().fg(dark_gray()));
}
```

#### Recipe 7: Thinking Block 3-State

```typescript
// CC: AssistantThinkingMessage.tsx — 3 states
if (!verbose && !isTranscriptMode) {
  return <Text dimColor>∴ Thinking  <Ctrl+O to expand></Text>;
}
if (hideInTranscript && isTranscriptMode) { return null; }
// expanded: show full content
return <Box><Text dimColor>∴ Thinking...</Text><Markdown>{content}</Markdown></Box>;
```

```rust
// ratatui equivalent
fn render_thinking_block(
    frame: &mut Frame,
    thinking: &ThinkingBlock,
    area: Rect,
    verbose: bool,
) {
    if !verbose {
        // Collapsed: 1-line dim
        let line = Line::from(vec![
            Span::styled("∴", Style::default().fg(dim_color()).add_modifier(Modifier::ITALIC)),
            Span::styled(" Thinking ", Style::default().fg(dim_color())),
            Span::styled("<Ctrl+O to expand>", Style::default().fg(subtle_color())),
        ]);
        frame.render_widget(Paragraph::new(line), area);
    } else {
        // Expanded: label + content
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Fill(1)])
            .split(area);
        frame.render_widget(
            Paragraph::new("∴ Thinking...").style(Style::default().fg(dim_color())),
            chunks[0],
        );
        // Render thinking content (indented 2 spaces)
        let inner = Rect { x: chunks[1].x + 2, width: chunks[1].width.saturating_sub(2), ..chunks[1] };
        frame.render_widget(
            Paragraph::new(thinking.content.clone()).wrap(Wrap { trim: false }),
            inner,
        );
    }
}
```

### 43.5 jcode Current ratatui Usage Patterns

jcode hiện tại dùng `Layout::default().constraints(...)` với `Rc<[Rect]>` chunks pattern:

```rust
// ui.rs lines 2710-2738 — main layout (packed variant)
let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints(if use_packed {
        vec![
            Constraint::Length(content_height.max(1)), // 0 Messages (exact height)
            Constraint::Length(queued_height),          // 1 Queued
            Constraint::Length(swarm_strip_height),     // 2 Swarm strip
            Constraint::Length(1),                      // 3 Status line ← P0: move this!
            Constraint::Length(notification_height),    // 4 Notification
            Constraint::Length(inline_block_height),    // 5 Inline UI
            Constraint::Length(inline_ui_gap_height),   // 6 Gap
            Constraint::Length(input_height),           // 7 Input
            Constraint::Length(overscroll_height),      // 8 Overscroll
            Constraint::Length(donut_height),           // 9 Donut
        ]
    } else {
        vec![
            Constraint::Min(3),                         // 0 Messages (scrollable)
            // ... same rest
        ]
    })
    .split(chat_area);
```

**Key observation — layout ordering:**
```
Current: Messages → Queued → Swarm → STATUS → Notification → Inline → Input → Overscroll → Donut
CC:      Messages → Spacer → Spinner → [StatusLine + Hints + Input trong Bottom Slot 50%]
```

jcode đặt **Status line ở vị trí chunks[3] (trên input)**, trong khi CC đặt **StatusLine trong PromptInputFooter (dưới input)**. Đây là migration gap P0.

### 43.6 CC Palette → Theme Mapping

| CC variable | ratatui Style | Purpose |
|------------|---------------|---------|
| `theme.text` | `Style::default().fg(Color::Rgb(205, 214, 244))` | Normal text |
| `theme.textMuted` | `Style::default().fg(Color::Rgb(166, 173, 200))` | Dim text |
| `theme.textSubtle` | `Style::default().fg(Color::Rgb(88, 91, 112))` | Very dim text |
| `theme.success` | `Style::default().fg(Color::Green)` | Success ✓ |
| `theme.error` | `Style::default().fg(Color::Red)` | Error ✗ |
| `theme.warning` | `Style::default().fg(Color::Rgb(250, 179, 135))` | Warning ⚠ |
| `theme.info` | `Style::default().fg(Color::Cyan)` | Info |
| `theme.accent` | `Style::default().fg(Color::Rgb(203, 166, 247))` | Purple accent |
| `theme.toolBash` | `Style::default().fg(Color::Rgb(137, 180, 250))` | Bash tool |
| `theme.toolEdit` | `Style::default().fg(Color::Green)` | Edit tool |
| `theme.toolRead` | `Style::default().fg(Color::Cyan)` | Read tool |
| `theme.toolGlob` | `Style::default().fg(Color::Rgb(249, 226, 175))` | Glob/Grep |
| `theme.toolAgent` | `Style::default().fg(Color::Rgb(203, 166, 247))` | Agent tool |
| `dimColor()` | `Style::default().add_modifier(Modifier::DIM)` | Dim modifier |
| `Modifier::BOLD` | `Style::default().add_modifier(Modifier::BOLD)` | Bold text |

---

## 44. AskUserQuestion — Structured Multi-Question Dialog (QUAN TRỌNG)

> **CC File:** `packages/builtin-tools/src/tools/AskUserQuestionTool/AskUserQuestionTool.tsx` (313 lines)
> **CC File:** `src/components/permissions/AskUserQuestionPermissionRequest/QuestionView.tsx` (329 lines)
> **CC File:** `src/components/permissions/AskUserQuestionPermissionRequest/SubmitQuestionsView.tsx`

### Đây là gì?

`askUserQuestion` là **tool do agent gọi** — khi agent cần user trả lời câu hỏi dạng structured (chọn option). CC hiện dialog **multi-question với tab navigation** ngay trong TUI.

### Schema (AskUserQuestionTool.tsx lines 41-67)

```typescript
Question {
  question: string    // Câu hỏi đầy đủ, kết thúc bằng "?"
  header: string      // Label ngắn (tối đa 16 chars), hiện trên tab
  options: [2-4] {    // 2-4 option mỗi câu hỏi
    label: string         // Tên option (1-5 từ)
    description: string   // Giải thích chi tiết
    preview?: string      // Optional markdown preview khi focus
  }
  multiSelect?: boolean  // Cho phép chọn nhiều (mặc định false)
}
```

### ASCII UX — Single Question (như ảnh 6)

```
┌─────────────────────────────────────────────────────────────────────┐
│ ● Cần giúp gì?                                                     │
│                                                                    │
│ Bạn cần tôi giúp gì với dự án OpenProxy?                          │
│                                                                    │
│   1. Code review                                                   │
│      Review code hiện tại hoặc diff gần đây                        │
│                                                                    │
│   2. Debug / Sửa lỗi                                               │
│      Tìm và sửa bug trong codebase                                  │
│                                                                    │
│   3. Feature mới                                                   │
│      Phát triển tính năng mới cho OpenProxy                         │
│                                                                    │
│   4. Khám phá codebase                                             │
│      Tìm hiểu cấu trúc project, kiến trúc, flow                     │
│                                                                    │
│   5. Type something.                                               │
│                                                                    │
│   6. Chat about this                                               │
│                                                                    │
│ Enter to select · ↑/↓ to navigate · Esc to cancel                  │
└─────────────────────────────────────────────────────────────────────┘

  ↑ Single question: auto-submit khi chọn (không cần Submit tab)
  ↑ Option 5 (Other): cho phép user nhập text tự do
  ↑ Option 6 (Chat about this): user muốn giải thích thêm thay vì chọn
```

### ASCII UX — Multi-Question với Tab Navigation (như ảnh 8)

```
┌─────────────────────────────────────────────────────────────────────┐
│ ← · 📌 Chọn chủ đề · 📌 Mức ưu tiên · ⏵ Ultracode · ✓ Submit →  │
├─────────────────────────────────────────────────────────────────────┤
│                                                                    │
│ Bạn có muốn dùng multi-agent orchestration (ultracode) cho task    │
│ này không? (Có thể dùng nhiều agent chạy song song để tăng tốc)    │
│                                                                    │
│ ) 1. Có, dùng ultracode                                           │
│      Dùng workflow multi-agent để xử lý nhanh và kỹ lưỡng        │
│                                                                    │
│   2. Không, làm thường                                              │
│      Tự tay làm trực tiếp không qua orchestration                   │
│                                                                    │
│   3. Type something.                                               │
│                                                                    │
│   4. Chat about this                                               │
│                                                                    │
│ Enter to select · Tab/Arrow keys to navigate · Esc to cancel       │
└─────────────────────────────────────────────────────────────────────┘

  ↑ Tab bar: ← prev | 📌 Chọn chủ đề ✓ | 📌 Mức ưu tiên | ⏵ Ultracode | ✓ Submit →
  ↑ Current tab highlighted (⏵)
  ↑ Đã trả lời: ✓ trên tab
  ↑ Chưa trả lời:空白 trên tab
  ↑ ")" = selection cursor
  ↑ Tab/Arrow keys chuyển giữa các câu hỏi
```

### ASCII UX — Submit View (như ảnh 9)

```
┌─────────────────────────────────────────────────────────────────────┐
│ ← · 📌 Chọn chủ đề · 📌 Mức ưu tiên · ⏵ ✓ Submit →              │
├─────────────────────────────────────────────────────────────────────┤
│                                                                    │
│ Review your answers                                                │
│                                                                    │
│ ⚠ You have not answered all questions                              │
│                                                                    │
│ Ready to submit your answers?                                      │
│                                                                    │
│ ) 1. Submit answers                                                │
│   2. Cancel                                                        │
│                                                                    │
└─────────────────────────────────────────────────────────────────────┘

  ↑ Warning: "You have not answered all questions" khi thiếu câu trả lời
  ↑ Submit tab hiển thị summary tất cả answers
```

### CC Source Code Flow

```
Agent calls AskUserQuestionTool({ questions: [...] })
  ↓
PermissionRequest renders AskUserQuestionPermissionRequest
  ↓
useMultipleChoiceState() → manages question navigation + answers
  ↓
Single question (1 câu, !multiSelect):
  → Auto-submit khi chọn (handleQuestionAnswer → submitAnswers)
  → Không hiện Submit tab (hideSubmitTab = true)

Multiple questions (>1 câu):
  → QuestionNavigationBar với tabs
  → Tab/Arrow keys chuyển giữa câu hỏi
  → Submit tab cuối cùng để review + submit

renderToolResultMessage({ answers }):
  → Hiển thị: "User answered Claude's questions:"
  → Mỗi answer: "· question → answer"

renderToolUseRejectedMessage():
  → Hiển thị: "User declined to answer questions"
```

### jcode Implementation

jcode **chưa có** AskUserQuestion tool — cần tạo mới.

**jcode files cần tạo/sửa:**
- `crates/jcode-tui/src/tui/permissions/ask_user_question.rs` — dialog UI component (QuestionView, SubmitQuestionsView, NavigationBar)
- `crates/jcode-tui/src/tui/app/input.rs` — input handling cho question dialog
- `crates/jcode-tui/src/tui/app.rs` — dialog state management
- `crates/jcode-tui/src/tui/ui_overlays.rs` — render question dialog

**Kiểm tra:**
- Agent gọi askUserQuestion → hiện dialog với tabs + numbered options
- Tab/Arrow keys chuyển giữa câu hỏi
- Enter để chọn option
- "Other" option: cho phép nhập text
- Multi-question: hiện Submit tab để review tất cả answers
- Cancel: hiện "User declined to answer questions"
- Warning khi chưa trả lời hết câu hỏi

---

## 45. BackgroundAgentSelector — Sub-agent List Dưới Status Bar

> **CC File:** `src/components/tasks/BackgroundAgentSelector.tsx` (63 lines)
> **CC File:** `src/components/TeammateViewHeader.tsx` (39 lines)
> **CC File:** `src/components/Spinner/TeammateSpinnerTree.tsx`

### Mô tả

Khi có sub-agent chạy background, CC hiển thị **danh sách agent ngay dưới status bar** (trong bottom slot). User có thể:
1. Click/chọn agent → nhảy vào session của agent đó
2. Xem agent đang làm gì (elapsed time + token count + status)
3. Switch giữa main session và agent session

### ASCII UX

```
├─────────────────────────────────────────────────────────────────────┤
│ ▌ Fix the bug in auth.rs                                           │ ← Input
├─────────────────────────────────────────────────────────────────────┤
│ sonnet-4  ctx:42%  $0.12  cache:78%  ▌auto  [🔱 2 active]         │ ← StatusLine
│                                                                     │
│ ● main                          shift+↓ to manage background agents  │ ← BG AGENT LIST
│ ○ explorer                                                          │
│   Searching codebase for auth patterns                              │
│ ○ worker-1                                                          │
│   ● Running bash (cargo test) · 45s · ↓ 1.2k tokens               │
├─────────────────────────────────────────────────────────────────────┤
│ Tab:autocomplete  Ctrl+X:leader  Ctrl+O:transcript  /:commands     │ ← Hints
└─────────────────────────────────────────────────────────────────────┘

  ↑ Dashed line = separator giữa StatusLine và agent list
  ↑ ● main = agent đang focus (session hiện tại)
  ↑ ○ explorer = background agent, clickable
  ↑ Mỗi agent row: icon + name + description + elapsed + tokens
```

### CC Implementation (BackgroundAgentSelector.tsx)

```typescript
function BackgroundAgentSelector() {
  const tasks = useBackgroundAgentTasks(); // lấy tất cả background agents
  const viewingId = useAppState(s => s.viewingAgentTaskId);
  const selectedBgIndex = useAppState(s => s.selectedBgAgentIndex);
  const pillFocused = footerSelection === 'bg_agent';

  if (tasks.length === 0) return null;

  return (
    <Box flexDirection="column" width="100%">
      {/* Main session row */}
      <Box flexDirection="row" justifyContent="space-between">
        <Text bold={mainHighlighted}>{mainHighlighted ? '● ' : '○ '}main</Text>
        <Text dimColor>shift+↓ to manage background agents</Text>
      </Box>
      {/* Agent rows */}
      {tasks.map(task => (
        <AgentRow key={task.agentId} task={task} selected={...} />
      ))}
    </Box>
  );
}

function AgentRow({ task, selected }) {
  const elapsed = useElapsedTime(task.startTime, task.status === 'running');
  const tokens = task.progress?.tokenCount ?? 0;
  const isRunning = task.status === 'running';
  return (
    <Box flexDirection="row" justifyContent="space-between">
      <Text color={isRunning ? 'success' : undefined}>{selected ? '● ' : '○ '}</Text>
      <Text bold={selected} wrap="truncate-end">
        {task.agentType} <Text dimColor>{task.description}</Text>
      </Text>
      <Text dimColor>{elapsed} · ↓ {formatTokens(tokens)} tokens</Text>
    </Box>
  );
}
```

### TeammateViewHeader — Khi đang xem agent session

Khi user chọn 1 agent, CC hiện header:
```
Viewing @explorer · Esc to return
Searching codebase for auth patterns
```

CC source:
```typescript
function TeammateViewHeader() {
  const viewedTeammate = useAppState(s => getViewedTeammateTask(s));
  if (!viewedTeammate) return null;

  return (
    <Box flexDirection="column" marginBottom={1}>
      <Box>
        <Text>Viewing </Text>
        <Text color={nameColor} bold>@{viewedTeammate.identity.agentName}</Text>
        <Text dimColor> · <KeyboardShortcutHint shortcut="esc" action="return" /></Text>
      </Box>
      <Text dimColor>{viewedTeammate.prompt}</Text>
    </Box>
  );
}
```

### Mouse/Keyboard Interaction

| Hành động | CC | jcode |
|-----------|----|-------|
| Mở agent list | `shift+↓` | ❌ |
| Chọn agent | `↑/↓` + `Enter` | ❌ |
| View agent | Click vào agent row | ❌ |
| Return main | `Esc` | ❌ |
| Hint text | Dynamic: "shift+↓ to manage" ↔ "↑/↓ to select" ↔ "x to stop" | ❌ |

### jcode Implementation

**jcode files cần tạo/sửa:**
- `crates/jcode-tui/src/tui/ui_input.rs` — thêm BackgroundAgentSelector rendering dưới StatusLine
- `crates/jcode-tui/src/tui/app/navigation.rs` — xử lý shift+↓, ↑/↓, Enter, Esc cho agent list
- `crates/jcode-tui/src/tui/mod.rs` — thêm footerSelection state, viewingAgentTaskId, selectedBgAgentIndex
- `crates/jcode-tui/src/tui/ui_messages.rs` — thêm TeammateViewHeader khi đang xem agent session

**Kiểm tra:**
- Có background agent: hiện agent list dưới StatusLine
- Không có agent: không hiện gì
- shift+↓: mở agent list focus
- ↑/↓: chọn agent
- Enter: nhảy vào session agent → header "Viewing @agent"
- Esc: return main session
- Agent row hiển thị icon, name, description, elapsed time, token count

---

## 46. Turn Summary Collapse/Expand Pattern (QUAN TRỌNG)

> **CC File:** `src/components/Spinner/SpinnerAnimationRow.tsx` (Spinner → tool summary khi done)
> **CC File:** `src/components/messages/UserToolResultMessage.tsx` (kết quả tool sau khi mở rộng)
>
> **Pattern này HIỆN TẠI CHƯA CÓ trong CLAUDECODE_UI.md** — đây là 1 trong những tính năng UX quan trọng nhất của CC.

### Mô tả trực quan

```
[BEFORE — thu gọn, 1 dòng]:                                              [AFTER — click → expand detail]:
                                                                          
 ┌─ User ───────────────────────────┐     ┌─ User ─────────────────────────────────────────────────┐
 │ Please implement auth fix        │     │ Please implement auth fix                                │
 └──────────────────────────────────┘     └──────────────────────────────────────────────────────────┘
                                          ┌─ Assistant ──────────────────────────────────────────────┐
                                          │ ⬇ Let me analyze the auth module. Here's what I found:  │
                                          │                                                          │
                                          │ ●  Bash (grep -n "validate" src/auth.rs)               │
                                          │ ✓  exit: 0                                               │
                                          │    src/auth.rs:45: if !validate_expiry(expiry) {         │
……………………………………………………           │                                                                  │
 ⏺ Searched for 3 patterns,              │ ●  Edit (Update src/auth.rs)                             │
   read 2 files, ran 2 shell commands    │ ✓  Validate expiry with current time                     │
                                          │                                                          │
  ↑ 1 line, clickable                     │ And here's the fixed code...                             │
  ↑ Shows summary của cả turn             │                                                          │
  ↑ "⏺" icon                             │ ─── sonnet-4 · Anthropic · 12.3s · ~3,400 tokens ──────── │
  ↑ Gộp cả reasoning + tools              └──────────────────────────────────────────────────────────┘
```

### CC Implementation

```typescript
// SpinnerAnimationRow.tsx — when turn completes, spinner changes thành summary line
// Source:
// - `thinkingStatus === 'number'` → "thought for Xs" 
// - `CollapsedReadSearchContent.tsx` → "Searched for N patterns, read N files"
// - `messageActions.tsx` → isCompactSummary = true → không hiện message actions

// Click to expand (VirtualMessageList.tsx):
// onClickK(msg) → toggles verbose for that user message
// → hiện full tool_use + tool_result + assistant text
```

### AI UX Design Pattern

```
Thu gọn (1 dòng)                               Mở rộng (full detail)
╔══════════════════════════════════════╗        ╔══════════════════════════════════════╗
║ ⏺ Thought for 9s, ran 2 commands   ║  click ║ 📄 User prompt                       ║
║   (Ctrl+O to expand)               ║ ─────→ ║ 🤔 Thinking... (full)                ║
╚══════════════════════════════════════╝        ║ ● Bash (cmd1) ✓                     ║
                                                ║ ● Bash (cmd2) ✓                     ║
                                                ║ ● Edit (file) ✓                     ║
                                                ║ 💬 Assistant response               ║
                                                ║ ─── model · time · tokens ───       ║
                                                ╚══════════════════════════════════════╝
```

Trong ảnh bạn chụp, UI đang hiển thị DÒNG THU GỌN:
```
⏺ Searched for 3 patterns, read 1 file, ran 2 shell commands
```

Đây là collapsed state của cả 1 turn (user prompt + tool calls + assistant message). 
Click vào → expand ra full conversation.

### Tại sao đây là tính năng CRITICAL

1. **Tiết kiệm space** — cả 1 turn gồm 4 Bash + 3 Read + 2 Edit + assistant thinking chỉ gọn 1-2 dòng
2. **Reduce clutter** — user thấy summary trước, click vào nếu cần detail
3. **Familiar pattern** — giống collapse/expand trong IDE, browser, chat apps
4. **Auto-collapse** cho turns cũ, chỉ giữ mở rộng cho turn hiện tại

### jcode: Current Status vs CC

| Tính năng | CC (claude-code-best) | jcode hiện tại |
|-----------|----------------------|-----------------|
| Turn summary line | ✅ `CompactSummary` + `SpinnerAnimationRow` | ❌ Chưa có |
| "Thought for Xs" | ✅ `SpinnerAnimationRow.tsx:187` | ⚠️ Có "thought for Xs" nhưng là static |
| "Searched for N patterns" | ✅ `CollapsedReadSearchContent.tsx:353-373` | ❌ Chưa có |
| Click to expand | ✅ `VirtualMessageList.tsx` onClick toggle verbose | ❌ Chưa có |
| Auto-collapse cũ | ✅ `Messages.tsx` collapse pipeline | ❌ Chưa có |

### jcode Implementation Approach

```rust
// Step 1: Sau mỗi turn, collapse tất cả sub-messages thành 1 summary line
enum TurnSummary {
    Collapsed {
        thinking_secs: u32,
        tool_counts: Vec<(ToolType, u32)>,  // Bash(2), Read(3), Edit(1)
        turn_index: usize,
    },
    Expanded,  // Show full detail
}

// Step 2: Render collapsed state
fn render_turn_summary(frame: &mut Frame, summary: &TurnSummary, area: Rect, click_targets: &mut ClickTargets) {
    let text = format!(
        "⏺ Thought for {}s, {}",
        summary.thinking_secs,
        summary.tool_counts.iter()
            .map(|(t, c)| format!("{} {}", c, t.plural_name()))
            .collect::<Vec<_>>()
            .join(", "),
    );
    
    // Store click target
    click_targets.push(ClickAction::ToggleTurnVerbose(summary.turn_index), area);
    
    // Render
    let line = Line::from(vec![
        Span::styled("⏺ ", Style::default().fg(accent_color())),
        Span::styled(text, Style::default().fg(dim_color())),
        Span::styled("  (Ctrl+O to expand)", Style::default().fg(subtle_color())),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

// Step 3: Click handler
// Trong handle_mouse_click → hit-test → ClickAction::ToggleTurnVerbose(index)
// → app.collapsed_turns.flip(index) → re-render với full detail
```

### Thêm vào beads

Cần thêm bead P0 hoặc update bead P0.1 để include turn summary collapse pattern.
Hiện tại đã có `CollapsedReadSearchGroups` (bead `jcode-o4l`, P2) nhưng CHƯA có turn-level collapse.

---

> **Generated:** 2026-07-01 | **Source:** `claude-code-best` @ `/tmp/feature-research/claude-code/` + `ratatui-0.30.2` @ `~/.cargo/registry/` | **jcode:** v0.32.0-dev

