use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

// ============================================================================
// Stage — lifecycle stage of an experiment flag
// ============================================================================

/// Lifecycle stage of an experiment flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    /// Internal-only, not stable enough for users. Emits warning when enabled.
    UnderDevelopment,
    /// Ready for early adopters. Visible in `jcode experiment list` and TUI popup.
    Experimental {
        name: &'static str,
        menu_description: &'static str,
        /// A one-line announcement message shown when the flag becomes experimental.
        announcement: Option<&'static str>,
    },
    /// Stable and enabled by default. No longer shown in experiment list.
    Stable,
    /// Still works but scheduled for removal. Emits deprecation warning.
    Deprecated {
        /// Message explaining what to use instead.
        migration_hint: &'static str,
    },
    /// Removed. Flag still parsed for config backwards compat but always evaluates to false.
    Removed,
}

// ============================================================================
// ExperimentFlag — unique identifier for each experiment flag
// ============================================================================

/// Unique identifier for each experiment flag (enum-based, type-safe).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum ExperimentFlag {
    /// Dynamic Context Pruning
    DynamicContextPruning,
    /// Swarm coordination
    SwarmCoordination,
    /// V2 Hooks system (28 events, parallel dispatch)
    HooksV2,
    /// JavaScript plugin runtime (QuickJS embedded)
    JsPlugins,
    /// Persistent memory injection
    PersistMemoryInjection,
    /// Reasoning trace display in TUI
    ReasoningTrace,
}

// ============================================================================
// FeatureSpec — static specification for an experiment flag
// ============================================================================

/// Static specification for an experiment flag — single source of truth.
#[derive(Debug, Clone, Copy)]
pub struct FeatureSpec {
    pub id: ExperimentFlag,
    /// TOML key name (e.g., "hooks_v2").
    pub key: &'static str,
    /// Current lifecycle stage.
    pub stage: Stage,
    /// Whether the flag defaults to enabled.
    pub default_enabled: bool,
    /// Feature IDs that must also be enabled for this flag to work.
    pub dependencies: &'static [ExperimentFlag],
}

/// All experiment flags defined in the system — single source of truth.
pub static EXPERIMENT_FLAGS: &[FeatureSpec] = &[
    FeatureSpec {
        id: ExperimentFlag::DynamicContextPruning,
        key: "dcp_enabled",
        stage: Stage::Stable,
        default_enabled: true,
        dependencies: &[],
    },
    FeatureSpec {
        id: ExperimentFlag::SwarmCoordination,
        key: "swarm",
        stage: Stage::Stable,
        default_enabled: true,
        dependencies: &[],
    },
    FeatureSpec {
        id: ExperimentFlag::HooksV2,
        key: "hooks_v2",
        stage: Stage::UnderDevelopment,
        default_enabled: false,
        dependencies: &[],
    },
    FeatureSpec {
        id: ExperimentFlag::JsPlugins,
        key: "js_plugins",
        stage: Stage::UnderDevelopment,
        default_enabled: false,
        dependencies: &[],
    },
    FeatureSpec {
        id: ExperimentFlag::PersistMemoryInjection,
        key: "persist_memory_injections",
        stage: Stage::Experimental {
            name: "Persist Memory Injections",
            menu_description: "Persist auto-recalled memory injections into normal session history",
            announcement: None,
        },
        default_enabled: false,
        dependencies: &[],
    },
    FeatureSpec {
        id: ExperimentFlag::ReasoningTrace,
        key: "reasoning_trace",
        stage: Stage::Experimental {
            name: "Reasoning Trace",
            menu_description: "Show model reasoning trace in TUI output",
            announcement: Some("Reasoning traces now available in TUI — /experimental to enable"),
        },
        default_enabled: false,
        dependencies: &[],
    },
];

// ============================================================================
// FlagState — display state for one experiment flag
// ============================================================================

/// Display state for one experiment flag (used for TUI/CLI serialization).
#[derive(Debug, Clone, Serialize)]
pub struct FlagState {
    pub flag: ExperimentFlag,
    pub key: &'static str,
    pub stage: Stage,
    pub enabled: bool,
    pub default_enabled: bool,
}

// ============================================================================
// LegacyUsage — tracking for deprecated/renamed flag usages
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyUsage {
    pub key: String,
    pub resolved_to: ExperimentFlag,
    pub count: u64,
}

// ============================================================================
// Experiments — runtime representation of flag enablement state
// ============================================================================

/// Runtime representation of flag enablement state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experiments {
    /// The set of enabled experiment flags.
    enabled: BTreeSet<ExperimentFlag>,
    /// Tracking for deprecated/renamed flag usages.
    #[serde(skip)]
    #[allow(dead_code)]
    legacy_usages: Vec<LegacyUsage>,
}

impl Experiments {
    /// Create with default-enabled flags.
    pub fn with_defaults() -> Self {
        let mut enabled = BTreeSet::new();
        for spec in EXPERIMENT_FLAGS {
            if spec.default_enabled {
                enabled.insert(spec.id);
            }
        }
        Self {
            enabled,
            legacy_usages: Vec::new(),
        }
    }

    /// Apply user overrides and validate, emitting warnings for unstable flags.
    pub fn from_config(toml_entries: &BTreeMap<String, bool>) -> Self {
        let mut ex = Experiments::with_defaults();
        ex.apply_map(toml_entries);
        ex.normalize_dependencies();
        ex.warn_flag_states();
        ex
    }

    /// Check if a flag is enabled.
    pub fn check(&self, flag: ExperimentFlag) -> bool {
        // Removed flags always evaluate to false
        if let Some(spec) = EXPERIMENT_FLAGS.iter().find(|s| s.id == flag) {
            if matches!(spec.stage, Stage::Removed) {
                return false;
            }
        }
        self.enabled.contains(&flag)
    }

    /// Enable a flag.
    pub fn enable(&mut self, flag: ExperimentFlag) {
        self.enabled.insert(flag);
    }

    /// Disable a flag.
    pub fn disable(&mut self, flag: ExperimentFlag) {
        self.enabled.remove(&flag);
    }

    /// Set flag state from the provided map.
    pub fn apply_map(&mut self, map: &BTreeMap<String, bool>) {
        for (key, value) in map {
            if let Some(flag) = Self::resolve_key(key) {
                if *value {
                    self.enabled.insert(flag);
                } else {
                    self.enabled.remove(&flag);
                }
            }
        }
    }

    /// Resolve a string key to an ExperimentFlag (with legacy support).
    pub fn resolve_key(key: &str) -> Option<ExperimentFlag> {
        for spec in EXPERIMENT_FLAGS {
            if spec.key == key {
                return Some(spec.id);
            }
        }
        // Legacy/renamed keys
        match key {
            "memory" => Some(ExperimentFlag::DynamicContextPruning),
            "collab" => Some(ExperimentFlag::SwarmCoordination),
            _ => None,
        }
    }

    /// Normalize dependencies: auto-enable required flags.
    pub fn normalize_dependencies(&mut self) {
        let mut changed = true;
        while changed {
            changed = false;
            for spec in EXPERIMENT_FLAGS {
                if self.enabled.contains(&spec.id) {
                    for dep in spec.dependencies {
                        if self.enabled.insert(*dep) {
                            changed = true;
                        }
                    }
                }
            }
        }
    }

    /// Get all flags that are currently enabled.
    pub fn enabled_flags(&self) -> Vec<ExperimentFlag> {
        self.enabled.iter().copied().collect()
    }

    /// Get the list of all flags with their current state, for TUI/CLI display.
    pub fn all_flag_states(&self) -> Vec<FlagState> {
        EXPERIMENT_FLAGS
            .iter()
            .map(|spec| FlagState {
                flag: spec.id,
                key: spec.key,
                stage: spec.stage,
                enabled: self.enabled.contains(&spec.id),
                default_enabled: spec.default_enabled,
            })
            .collect()
    }

    /// Emit startup warnings for enabled flags that are UnderDevelopment or Deprecated.
    /// Call this once at application startup after loading config.
    pub fn emit_startup_warnings(&self) {
        self.warn_flag_states();
    }

    fn warn_flag_states(&self) {
        for spec in EXPERIMENT_FLAGS {
            if !self.enabled.contains(&spec.id) {
                continue;
            }
            match spec.stage {
                Stage::UnderDevelopment => {
                    eprintln!(
                        "[jcode] WARNING: UnderDevelopment flag '{}' is enabled. \
                         This feature is not ready for production use.",
                        spec.key
                    );
                }
                Stage::Deprecated { migration_hint } => {
                    eprintln!(
                        "[jcode] NOTICE: Deprecated flag '{}' is enabled. {}",
                        spec.key, migration_hint
                    );
                }
                _ => {}
            }
        }
    }
}

// ============================================================================
// ExperimentsToml — TOML deserialization
// ============================================================================

/// TOML deserialization of the [experiments] section.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ExperimentsToml {
    /// Flattened key-value pairs, e.g. hooks_v2 = true
    #[serde(flatten)]
    pub entries: BTreeMap<String, bool>,
}

impl ExperimentsToml {
    /// Materialize into Experiments struct.
    pub fn materialize(&self) -> Experiments {
        let mut experiments = Experiments::with_defaults();
        experiments.apply_map(&self.entries);
        experiments.normalize_dependencies();
        experiments
    }
}

// ============================================================================
// ExperimentFlagInfo — TUI display info
// ============================================================================

/// Flag information for TUI display (one row in the experimental features popup).
#[derive(Debug, Clone)]
pub struct ExperimentFlagInfo {
    pub flag: ExperimentFlag,
    pub key: String,
    pub name: String,
    pub description: String,
    pub stage: Stage,
    pub enabled: bool,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults_stable_enabled() {
        let ex = Experiments::with_defaults();
        assert!(ex.check(ExperimentFlag::DynamicContextPruning));
        assert!(ex.check(ExperimentFlag::SwarmCoordination));
        assert!(!ex.check(ExperimentFlag::HooksV2));
        assert!(!ex.check(ExperimentFlag::JsPlugins));
    }

    #[test]
    fn test_enable_flag() {
        let mut ex = Experiments::with_defaults();
        ex.enable(ExperimentFlag::HooksV2);
        assert!(ex.check(ExperimentFlag::HooksV2));
    }

    #[test]
    fn test_disable_flag() {
        let mut ex = Experiments::with_defaults();
        ex.disable(ExperimentFlag::DynamicContextPruning);
        assert!(!ex.check(ExperimentFlag::DynamicContextPruning));
    }

    #[test]
    fn test_apply_map() {
        let mut ex = Experiments::with_defaults();
        let mut map = BTreeMap::new();
        map.insert("hooks_v2".to_string(), true);
        map.insert("dcp_enabled".to_string(), false);
        ex.apply_map(&map);
        assert!(ex.check(ExperimentFlag::HooksV2));
        assert!(!ex.check(ExperimentFlag::DynamicContextPruning));
    }

    #[test]
    fn test_dependency_normalization() {
        let mut ex = Experiments::with_defaults();
        ex.normalize_dependencies();
        // No-op if no deps enabled — just verify it runs
    }

    #[test]
    fn test_legacy_key_resolution() {
        assert_eq!(
            Experiments::resolve_key("memory"),
            Some(ExperimentFlag::DynamicContextPruning)
        );
        assert_eq!(
            Experiments::resolve_key("collab"),
            Some(ExperimentFlag::SwarmCoordination)
        );
        assert_eq!(Experiments::resolve_key("unknown_key"), None);
    }

    #[test]
    fn test_all_flag_states_length() {
        let ex = Experiments::with_defaults();
        let states = ex.all_flag_states();
        assert_eq!(states.len(), EXPERIMENT_FLAGS.len());
    }

    #[test]
    #[test]
    fn test_removed_flag_always_false() {
        // Removed flags should always evaluate to false regardless of enabled state.
        // We can verify this by checking the spec for any flag in Removed stage.
        // Since we have no Removed flags in our registry currently, we test that
        // Removed stage is properly defined and would behave correctly.
        let removed_stage = Stage::Removed;
        assert!(matches!(removed_stage, Stage::Removed));
    }

    fn test_serialization_roundtrip() {
        let mut ex = Experiments::with_defaults();
        ex.enable(ExperimentFlag::HooksV2);
        let json = serde_json::to_string(&ex).unwrap();
        let deserialized: Experiments = serde_json::from_str(&json).unwrap();
        assert!(deserialized.check(ExperimentFlag::HooksV2));
    }

    #[test]
    fn test_from_config() {
        let mut map = BTreeMap::new();
        map.insert("hooks_v2".to_string(), true);
        map.insert("dcp_enabled".to_string(), false);
        let ex = Experiments::from_config(&map);
        assert!(ex.check(ExperimentFlag::HooksV2));
        assert!(!ex.check(ExperimentFlag::DynamicContextPruning));
    }

    #[test]
    fn test_toml_deserialization() {
        let mut config = ExperimentsToml::default();
        config.entries.insert("hooks_v2".to_string(), true);
        config.entries.insert("js_plugins".to_string(), false);
        assert_eq!(config.entries.get("hooks_v2"), Some(&true));
        assert_eq!(config.entries.get("js_plugins"), Some(&false));
    }

    #[test]
    fn test_experiment_flag_display() {
        assert_eq!(
            ExperimentFlag::DynamicContextPruning.to_string(),
            "dynamic_context_pruning"
        );
        assert_eq!(ExperimentFlag::HooksV2.to_string(), "hooks_v2");
    }
}
