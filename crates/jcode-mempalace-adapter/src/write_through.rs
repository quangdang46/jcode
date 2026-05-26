use std::sync::Arc;

use async_trait::async_trait;
use mempalace_core::{
    Drawer, DrawerId, MemoryScope,
    SearchHit, SearchScope,
};

use jcode_memory_types::MemoryProvider as JcodeMemoryProvider;

pub struct WriteThroughProvider {
    local: Arc<dyn JcodeMemoryProvider>,
    palace: Arc<dyn mempalace_core::MemoryProvider>,
}

impl WriteThroughProvider {
    pub fn new(
        local: Arc<dyn JcodeMemoryProvider>,
        palace: Arc<dyn mempalace_core::MemoryProvider>,
    ) -> Self {
        Self { local, palace }
    }
}

#[async_trait]
impl mempalace_core::MemoryProvider for WriteThroughProvider {
    async fn add_drawer(&self, drawer: Drawer) -> anyhow::Result<DrawerId> {
        self.palace.add_drawer(drawer).await
    }

    async fn remember(
        &self,
        content: String,
        scope: MemoryScope,
    ) -> anyhow::Result<DrawerId> {
        self.palace.remember(content, scope).await
    }

    async fn forget(&self, id: &DrawerId) -> anyhow::Result<bool> {
        let r = self.palace.forget(id).await?;
        let _ = self.local.forget(&id.to_string());
        Ok(r)
    }

    async fn search(
        &self,
        query: &str,
        scope: &SearchScope,
    ) -> anyhow::Result<Vec<SearchHit>> {
        self.palace.search(query, scope).await
    }

    async fn search_with_embedding(
        &self,
        query_vec: &[f32],
        scope: &SearchScope,
    ) -> anyhow::Result<Vec<SearchHit>> {
        self.palace.search_with_embedding(query_vec, scope).await
    }

    async fn related(
        &self,
        id: &DrawerId,
        depth: usize,
    ) -> anyhow::Result<Vec<SearchHit>> {
        self.palace.related(id, depth).await
    }

    async fn extract_from_transcript(
        &self,
        transcript: &str,
        session_id: &str,
    ) -> anyhow::Result<Vec<DrawerId>> {
        self.palace.extract_from_transcript(transcript, session_id).await
    }

    async fn graph_stats(&self) -> anyhow::Result<mempalace_core::knowledge_graph::KgStats> {
        self.palace.graph_stats().await
    }

    async fn get_drawers(
        &self,
        scope: Option<&SearchScope>,
        limit: Option<usize>,
    ) -> anyhow::Result<Vec<Drawer>> {
        self.palace.get_drawers(scope, limit).await
    }

    fn fingerprint(&self) -> &str {
        self.palace.fingerprint()
    }

    fn embedder(&self) -> &dyn mempalace_core::Embedder {
        self.palace.embedder()
    }

    fn store(&self) -> &dyn mempalace_core::palace::PalaceStore {
        self.palace.store()
    }
}