/// Stub: plugin system is not yet wired up.

pub struct PluginSystem {
    pub registry: PluginRegistry,
    pub loader: PluginLoader,
}

pub struct PluginRegistry;
pub struct PluginLoader;

impl PluginRegistry {
    pub async fn list(&self) -> Vec<(PluginId, ())> {
        vec![]
    }
}

impl PluginLoader {
    pub async fn reload(&self, _id: &PluginId) -> anyhow::Result<()> {
        Ok(())
    }
}

pub struct PluginId;

impl PluginId {
    pub fn short_name(&self) -> &str {
        ""
    }
}

pub fn plugin_system() -> Option<PluginSystem> {
    None
}
