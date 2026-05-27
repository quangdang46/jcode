# Configuration files reference

This document covers the **on-disk configuration files** jcode reads, with a
focus on **offline** / **air-gapped** / **self-hosted** setups (issue
[#48](https://github.com/quangdang46/jcode/issues/48)). All paths are
relative to your jcode home, which is `~/.jcode/` by default (or
`$JCODE_HOME` if set, or `$XDG_DATA_HOME/jcode` when
[`JCODE_USE_XDG=1`](#xdg-mode) is enabled).

## tl;dr

For a fully offline setup against a local OpenAI-compatible endpoint
(vLLM, llama.cpp, LM Studio, Ollama, etc.):

```toml
# ~/.jcode/config.toml
[provider]
default_provider = "local-vllm"
default_model    = "Qwen/Qwen3-Coder-30B-A3B-Instruct"

[providers.local-vllm]
type        = "openai-compatible"
base_url    = "http://localhost:8000/v1"
default_model = "Qwen/Qwen3-Coder-30B-A3B-Instruct"

[[providers.local-vllm.models]]
id            = "Qwen/Qwen3-Coder-30B-A3B-Instruct"
context_window = 128000
```

Plus run with `--offline` (or `JCODE_OFFLINE=1`) to disable the update
check + telemetry:

```bash
jcode --offline --provider-profile local-vllm
```

## Locations

| Path | Purpose | Format |
|---|---|---|
| `~/.jcode/config.toml` | Main config (providers, defaults, features) | TOML |
| `~/.jcode/auth.json` | Anthropic / Claude OAuth credentials | JSON |
| `~/.jcode/openai-auth.json` | OpenAI / Codex OAuth credentials | JSON |
| `~/.jcode/gemini_oauth.json` | Gemini OAuth credentials | JSON |
| `~/.jcode/mcp.json` | Global MCP server registry | JSON |
| `.jcode/mcp.json` (project) | Project-local MCP servers | JSON |
| `~/.jcode/hooks.toml` | Hook configuration | TOML |
| `.jcode/hooks.toml` (project) | Project-level hooks | TOML |
| `~/.jcode/prompts/*.md` | User-level prompt templates | Markdown |
| `.jcode/prompts/*.md` (project) | Project-level prompt templates | Markdown |
| `~/.jcode/SYSTEM.md` | Global system-prompt override | Markdown |
| `~/.jcode/APPEND_SYSTEM.md` | Global system-prompt append | Markdown |
| `.jcode/SYSTEM.md` (project) | Project system-prompt override | Markdown |
| `.jcode/APPEND_SYSTEM.md` (project) | Project system-prompt append | Markdown |
| `~/.jcode/sessions/` | Persisted session state (autosaved) | JSON per file |
| `~/.jcode/logs/jcode-YYYY-MM-DD.log` | Daily log output | text |
| `~/.config/jcode/<provider>.env` | Per-provider env-file overrides | dotenv |

> Project-level files always win over user-level files of the same name.

## `~/.jcode/config.toml` — main config

The main config has the following top-level tables (all optional):

```toml
[provider]
default_provider = "anthropic"     # provider key (oauth or compat profile)
default_model    = "claude-sonnet-4-5"

[providers.<name>]                 # OpenAI-compatible profile
type          = "openai-compatible"
base_url      = "https://...:port/v1"
default_model = "..."
api_key_env   = "MY_API_KEY"       # optional; reads env var
env_file      = "my-provider.env"  # optional; reads ~/.config/jcode/<file>
no_api_key    = false              # set true for local servers without auth

[[providers.<name>.models]]
id            = "model-id"
context_window = 128000

[features]
memory   = true   # enable embedding-based memory (requires local model)
swarm    = true   # multi-agent collaboration in same repo

[ambient]
enabled = false   # OpenClaw-style ambient mode
```

### Setting up an OpenAI-compatible provider via CLI

You usually don't need to edit this file by hand. The
[`jcode provider add`](../README.md#config-file-setup-for-self-hosted-endpoints-and-mcp)
command writes the profile for you, including secret-safe API key storage:

```bash
# Hosted OpenAI-compatible API (with API key from stdin):
printf '%s' "$MY_API_KEY" | jcode provider add my-api \
  --base-url https://llm.example.com/v1 \
  --model my-model-id \
  --api-key-stdin \
  --set-default

# Local server with no auth:
jcode provider add local-vllm \
  --base-url http://localhost:8000/v1 \
  --model Qwen/Qwen3-Coder-30B-A3B-Instruct \
  --no-api-key \
  --set-default
```

After adding, smoke-test it:

```bash
jcode --provider-profile local-vllm auth-test \
  --prompt 'Reply exactly JCODE_PROVIDER_SETUP_OK'
```

## `~/.jcode/mcp.json` — MCP server registry

```json
{
  "servers": {
    "filesystem": {
      "command": "/path/to/mcp-server",
      "args": ["--root", "/workspace"],
      "env": {},
      "shared": true
    }
  }
}
```

- Project-local equivalents live at `.jcode/mcp.json` (relative to cwd) and
  override entries with the same name.
- Compatibility fallback: if `~/.jcode/mcp.json` doesn't exist on first run,
  jcode imports from `~/.claude/mcp.json` and `~/.codex/config.toml`.
- Run [`jcode mcp trust <path>`](../docs/SAFE_EVALUATION.md) to mark a config
  as trusted when `--require-mcp-trust` is in effect.

## `~/.jcode/SYSTEM.md` and `APPEND_SYSTEM.md` — system-prompt overrides

- `SYSTEM.md` **replaces** jcode's built-in system prompt entirely.
- `APPEND_SYSTEM.md` **appends** to the built-in system prompt without
  removing it. Use this for "always remember to ..."-style additions.

Project-level `.jcode/SYSTEM.md` and `.jcode/APPEND_SYSTEM.md` (relative to
cwd) override the user-level versions.

## `~/.jcode/prompts/<name>.md` — slash-command templates

Discoverable via `jcode prompts list` and invokable as `/<name>` inside the
TUI. Scaffold one with:

```bash
jcode prompts new <name>           # → ./.jcode/prompts/<name>.md
jcode prompts new <name> --user    # → ~/.jcode/prompts/<name>.md
```

See [PR #207 + #217](https://github.com/quangdang46/jcode/pulls?q=prompts) for
the full template format.

## XDG mode

If you set `JCODE_USE_XDG=1`, the home moves to:

```
$XDG_DATA_HOME/jcode               (when XDG_DATA_HOME is set)
~/.local/share/jcode               (default fallback)
```

All file names above are unchanged — only the parent directory moves.
See [PR #225](https://github.com/quangdang46/jcode/pull/225) for the
toggle.

## Offline / air-gapped checklist

For machines that cannot reach the public internet:

1. **Disable the update check + telemetry**: `--offline` or
   `JCODE_OFFLINE=1`.
2. **Configure a local provider**: see the tl;dr above for a vLLM-style
   `config.toml`. Public OAuth flows (Claude, OpenAI, Gemini) won't work
   without internet.
3. **MCP servers**: install + register them in `~/.jcode/mcp.json` ahead of
   time. Project-local `.jcode/mcp.json` is fine for repo-pinned
   integrations.
4. **No memory embeddings**: the default memory backend uses local
   `tract-onnx` weights downloaded once. If your machine never had
   internet, copy `~/.jcode/embeddings/` from another machine, or set
   `[features] memory = false`.
5. **`jcode doctor`**: runs without network access; use it to verify the
   above before depending on jcode in production.

## Hooks

### Overview

Hooks allow you to intercept and react to events during jcode's execution lifecycle. They enable custom logic for logging, filtering, modifying tool inputs/outputs, enforcing policies, and integrating with external systems.

Hooks work by executing external commands (scripts, binaries, HTTP calls) that receive JSON context about the current event and return a response that can continue or block execution.

### Configuration

Hooks are configured in `hooks.toml` files at two levels:

| Path | Purpose | Priority |
|---|---|---|
| `~/.jcode/hooks.toml` | User-level hooks | Lower |
| `.jcode/hooks.toml` (project) | Project-level hooks | Higher (overrides user-level) |

The file format is TOML:

```toml
# Example: ~/.jcode/hooks.toml

[events.pre_tool_use]
command = "/usr/local/bin/my-hook-script.sh"
args = ["--verbose"]
env = { "HOOK_ENV" = "value" }
cwd = "/optional/working/dir"
timeout_secs = 30
pass_input_via_stdin = true

[events.post_tool_use]
command = "echo 'tool completed'"

[events.error]
command = "/usr/local/bin/error-handler.sh"

[events.custom:my_event]
command = "echo 'custom event triggered'"
```

### Events

| Event | Aliases | Description | Blocking |
|---|---|---|---|
| `PreToolUse` | `pretooluse`, `pre_tool_use` | Before a tool is executed | Yes |
| `PostToolUse` | `posttooluse`, `post_tool_use` | After a tool completes | No |
| `PreSession` | `presession`, `pre_session` | Before a session starts | Yes |
| `PostSession` | `postsession`, `post_session` | After a session ends | No |
| `Error` | `error` | On any error | No |
| `Custom:<name>` | — | Custom event (user-defined) | Depends |

**Event name parsing is case-insensitive.** Use any of the listed aliases in your config.

### Hook Input (JSON passed to hooks)

When a hook executes, it receives a JSON payload via stdin with the current context:

```json
{
  "session_id": "sess_abc123",
  "transcript_path": "/home/user/.jcode/sessions/sess_abc123.json",
  "cwd": "/data/projects/myproject",
  "hook_event_name": "PreToolUse",
  "agent_id": null,
  "agent_type": null,
  "tool_name": "Bash",
  "tool_input": { "command": "git status" },
  "tool_use_id": "toolu_xyz789",
  "permission_mode": null
}
```

**Available fields:**

| Field | Type | Description |
|---|---|---|
| `session_id` | String | Unique session identifier |
| `transcript_path` | String | Path to session transcript file |
| `cwd` | String | Current working directory |
| `hook_event_name` | String | Event that triggered this hook |
| `agent_id` | String? | Optional agent identifier |
| `agent_type` | String? | Optional agent type |
| `tool_name` | String? | Tool being executed (for tool events) |
| `tool_input` | JSON? | Tool input parameters |
| `tool_use_id` | String? | Unique tool use identifier |
| `permission_mode` | String? | Permission mode (if applicable) |

### Hook Output (JSON expected from hooks)

Hooks return a JSON response via stdout:

```json
{
  "continue_": true,
  "suppress_output": null,
  "stop_reason": null,
  "decision": null,
  "reason": null,
  "system_message": null,
  "hook_specific_output": null
}
```

**Output fields:**

| Field | Type | Default | Description |
|---|---|---|---|
| `continue_` | bool | `true` | Whether to continue execution |
| `suppress_output` | bool? | null | Suppress tool output display |
| `stop_reason` | String? | null | Reason for stopping (if blocked) |
| `decision` | String? | null | Decision made by hook |
| `reason` | String? | null | Human-readable reason |
| `system_message` | String? | null | Message to inject into system |
| `hook_specific_output` | Object? | null | Event-specific fields (see below) |

**`hook_specific_output` fields:**

| Field | Type | Description |
|---|---|---|
| `hook_event_name` | String | Event name |
| `permission_decision` | String? | Allow/deny decision |
| `permission_decision_reason` | String? | Reason for permission decision |
| `updated_input` | JSON? | Modified tool input |
| `additional_context` | String? | Extra context to include |

**Blocking behavior:**
- Return `continue_: false` to block execution
- Exit code 2 also signals a block

### Handler Configuration

Each hook event in the config maps to a `HookHandlerConfig`:

```toml
[events.<event_name>]
command = "/path/to/handler"      # Required: command to execute
args = ["arg1", "arg2"]          # Optional: arguments (default: [])
env = { "KEY" = "value" }        # Optional: environment variables
cwd = "/working/dir"             # Optional: working directory
timeout_secs = 30                # Optional: execution timeout (default: 30s)
pass_input_via_stdin = true      # Optional: send JSON input via stdin
```

### Matcher Types

Matchers control when a hook fires based on the tool name or context. Four matcher types are supported:

**1. Exact Match** — Matches a single tool name exactly:

```toml
[events.pre_tool_use]
# Handler only fires for Bash tool
command = "/hooks/bash-only.sh"
# (No matcher = matches all)
```

**2. Multi Match** — Matches any of several tools (pipe-separated):

```toml
[events.pre_tool_use]
command = "/hooks/write-edit.sh"
# Handler fires for Write OR Edit tools
```

**3. Regex Match** — Matches tool name via regex pattern:

```toml
[events.pre_tool_use]
command = "/hooks/bash-git.sh"
# Handler fires for Bash tools with git commands
# Context includes the full command for regex matching
```

**4. Wildcard** — Matches all events of this type:

```toml
[events.pre_tool_use]
command = "/hooks/log-all-tools.sh"
# Fires for every tool before execution
```

### Handler Types

Currently, only **Command** handlers are implemented.

**Command Handler** — Executes a shell command:

```toml
[events.pre_tool_use]
command = "/usr/local/bin/my-hook.sh"
args = ["--verbose", "--tool"]
env = { "SESSION_ID" = "123" }
cwd = "/tmp"
timeout_secs = 30
pass_input_via_stdin = true
```

The command receives the hook input as JSON via stdin and should output a `HookOutput` JSON response via stdout.

### Real-World Examples

**1. Log all tool executions:**

```toml
[events.pre_tool_use]
command = "logger"
args = ["tool_executed"]
env = { "LEVEL" = "INFO" }

[events.post_tool_use]
command = "logger"
args = ["tool_completed"]
env = { "LEVEL" = "INFO" }
```

**2. Block dangerous commands:**

```bash
#!/bin/bash
# /hooks/block-rm-rf.sh
read -r input
echo "$input" | jq -e '.tool_input.command | test("rm\\s+-rf\\s+/")' > /dev/null 2>&1
if [ $? -eq 0 ]; then
  echo '{"continue_": false, "stop_reason": "Dangerous rm -rf detected", "decision": "block"}'
  exit 2
fi
echo '{"continue_": true}'
```

```toml
[events.pre_tool_use]
command = "/hooks/block-rm-rf.sh"
```

**3. Audit trail to file:**

```toml
[events.post_tool_use]
command = "tee"
args = ["-a", "/var/log/jcode-audit.jsonl"]
pass_input_via_stdin = true
```

**4. HTTP webhook notification:**

```bash
#!/bin/bash
# /hooks/webhook.sh
read -r input
curl -s -X POST "https://hooks.example.com/jcode" \
  -H "Content-Type: application/json" \
  -d "$input" > /dev/null
echo '{"continue_": true}'
```

```toml
[events.post_tool_use]
command = "/hooks/webhook.sh"
```

**5. Custom event for testing:**

```toml
[events.custom:test_event]
command = "echo 'test event fired'"
```

Trigger via the jcode API or internal events system that dispatches custom events.

### Conditionals

Hooks support simple `if_` conditions to filter when they execute:

```toml
[events.pre_tool_use.if_bash_destructive]
command = "/hooks/confirm-destructive.sh"
# Handler condition: tool_name=Bash
# Also checks tool_input.command for destructive patterns
```

Conditions are shell-like expressions: `field=value` or `field!=value`

Supported fields: `tool_name`, `agent_type`, `permission_mode`

## See also

- [Z.AI Coding Plan quickstart](ZAI_CODING_PLAN.md)
- [Safe evaluation mode](SAFE_EVALUATION.md)
- [`jcode --help`](../README.md#further-reading) for the full CLI surface.
