//! Hooks module - lifecycle hooks for jcode events

pub mod config;
pub mod execute;
pub mod matcher;
pub mod registry;
pub mod types;

pub use config::{load_hooks_config, HookEvent, HookHandlerConfig, HooksConfig};
pub use execute::{execute_hook, execute_command_hook, HookResult};
pub use matcher::{matches, MatcherContext, parse_multi_pattern};
pub use registry::{HookContext, HookRegistry};
pub use types::*;
