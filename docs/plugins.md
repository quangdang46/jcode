# Plugin Author Guide

This guide covers how to create, test, and distribute plugins for jcode.

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Quick Start: Hello World](#quick-start-hello-world)
3. [Plugin Manifest](#plugin-manifest)
4. [Hooks](#hooks)
5. [Configuration](#configuration)
6. [Testing](#testing)
7. [Distribution](#distribution)

## Architecture Overview

jcode's plugin system lives in two crates:

- **[`jcode-plugin-core`](/crates/jcode-plugin-core/)** — Types, traits, and the registry (`PluginReg`, `PluginId`, `PluginManifest`)
- **[`jcode-plugin-runtime`](/crates/jcode-plugin-runtime/)** — Runtime plugin host (loading, lifecycle, sandbox)

A plugin is a Rust crate that implements the [`Plugin`](/crates/jcode-plugin-core/src/plugin.rs) trait and registers itself via `inventory::submit!`. At startup, the plugin host scans for registered plugins, loads them, and calls their lifecycle hooks.

### Key Types

| Type | Location | Description |
|------|----------|-------------|
| `Plugin` trait | `jcode-plugin-core::plugin` | Core lifecycle trait (init, activate, deactivate) |
| `PluginId` | `jcode-plugin-core::types` | Unique identifier (name + version) |
| `PluginManifest` | `jcode-plugin-core::manifest` | Metadata from Cargo.toml |
| `PluginReg` | `jcode-plugin-core::registry` | Global plugin registry |
| `PluginEvent` | `jcode-plugin-runtime::events` | Lifecycle event type |

## Quick Start: Hello World

### Step 1: Create a crate

```bash
cargo new --lib jcode-plugin-hello
cd jcode-plugin-hello
```

### Step 2: Add dependencies

In `Cargo.toml`:

```toml
[package]
name = "jcode-plugin-hello"
version = "0.1.0"
edition = "2021"

[package.metadata.jcode-plugin]
name = "hello"
version = "0.1.0"
description = "A friendly hello world plugin"
hooks = ["init", "shutdown"]

[dependencies]
jcode-plugin-core = { git = "https://github.com/quangdang46/jcode" }
inventory = "0.3"
serde = { version = "1", features = ["derive"] }
```

### Step 3: Implement the Plugin trait

In `src/lib.rs`:

```rust
use jcode_plugin_core::plugin::{Plugin, PluginContext, PluginResult};
use jcode_plugin_core::PluginReg;

pub struct HelloPlugin;

impl Plugin for HelloPlugin {
    fn id(&self) -> &'static str {
        "hello"
    }

    fn init(&mut self, _ctx: &PluginContext) -> PluginResult {
        println!("Hello from plugin 'hello'!");
        Ok(())
    }

    fn shutdown(&mut self) -> PluginResult {
        println!("Goodbye from plugin 'hello'!");
        Ok(())
    }
}

inventory::submit! {
    PluginReg::new("hello", "0.1.0", || Box::new(HelloPlugin))
}
```

### Step 4: Load and verify

```bash
jcode plugin load --path ./jcode-plugin-hello
jcode plugin list
# You should see: hello (0.1.0)
```

## Plugin Manifest

Plugin metadata goes in `[package.metadata.jcode-plugin]` in `Cargo.toml`.

| Key | Required | Description |
|-----|----------|-------------|
| `name` | Yes | Plugin name (used in `jcode plugin list`) |
| `version` | Yes | Semantic version |
| `description` | No | Short description |
| `author` | No | Author string |
| `hooks` | No | Comma-separated hooks this plugin implements |
| `min-jcode-version` | No | Minimum jcode version required |

Example:

```toml
[package.metadata.jcode-plugin]
name = "my-analyzer"
version = "0.2.0"
description = "Code analysis plugin"
author = "jcode team"
hooks = ["init", "pre_tool_call", "shutdown"]
min-jcode-version = "0.26.0"
```

## Hooks

The `Plugin` trait defines lifecycle hooks that jcode calls at specific points:

| Hook | Signature | When Called |
|------|-----------|-------------|
| `init` | `fn init(&mut self, ctx: &PluginContext) -> PluginResult` | At plugin load time |
| `activate` | `fn activate(&mut self) -> PluginResult` | When plugin is enabled |
| `deactivate` | `fn deactivate(&mut self) -> PluginResult` | When plugin is disabled |
| `shutdown` | `fn shutdown(&mut self) -> PluginResult` | At jcode shutdown |
| `pre_tool_call` | `fn pre_tool_call(&self, name: &str, args: &str) -> PluginResult` | Before every tool call |
| `post_tool_call` | `fn post_tool_call(&self, name: &str, result: &str) -> PluginResult` | After every tool call |

### `PluginContext`

```rust
pub struct PluginContext {
    pub jcode_version: String,
    pub data_dir: PathBuf,    // ~/.jcode/plugins/<name>/
    pub config: Value,        // Plugin-specific config parsed from YAML
}
```

## Configuration

Plugins can provide a default configuration:

```rust
use jcode_plugin_core::config::ConfigProvider;

impl ConfigProvider for HelloPlugin {
    fn default_config(&self) -> &'static str {
        r#"greeting: "Hello, world!" language: "en""#
    }
}
```

Users override config in `~/.jcode/plugins/<name>.yaml` or `./.jcode/plugins/<name>.yaml`.

## Testing

### Unit test

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_ok() {
        let mut plugin = HelloPlugin;
        let ctx = PluginContext {
            jcode_version: "0.26.0".into(),
            data_dir: std::env::temp_dir().join("jcode-test-plugin"),
            config: serde_json::json!({}),
        };
        assert!(plugin.init(&ctx).is_ok());
    }
}
```

### Kill-switch test

```rust
#[test]
fn test_kill_switch() {
    std::env::set_var("JCODE_PLUGIN_KILL_HELLO", "1");
    // Plugin should not be loaded by the runtime
    std::env::remove_var("JCODE_PLUGIN_KILL_HELLO");
}
```

## Distribution

Plugins can be distributed three ways:

1. **Workspace crate** — Add to `Cargo.toml` workspace
2. **Local path** — `jcode plugin load --path /path/to/plugin`
3. **Git URL** — `jcode plugin clone <url>`

### Best Practices

- Prefix crate names with `jcode-plugin-` for discoverability
- Use semver
- Keep plugins focused on one concern
- Use the kill-switch naming convention: `JCODE_PLUGIN_KILL_<NAME>` (uppercase, underscores)
