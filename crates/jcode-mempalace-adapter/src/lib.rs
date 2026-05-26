// =====================================================================
// jcode-mempalace-adapter — MempalaceProvider impl of jcode MemoryProvider
// =====================================================================
//
// Implements jcode's `MemoryProvider` trait backed by mempalace-core's
// `Palace`. Core of Phase 3 integration (mp-044).
//
// Translation:
//   jcode MemoryEntry    →  mempalace Drawer (tags/source in metadata)
//   jcode String ID      →  mempalace DrawerId
//   jcode MemoryScope    →  mempalace MemoryScope (Local/Global/Auto)
//   jcode GraphStats     ←  mempalace KgStats
//
// All Palace methods are async; jcode's trait is sync so we use
// tokio::runtime::Handle::current().block_on(...) at each call site.
// This is correct because jcode already runs in a tokio runtime.
//
// Ref: docs/research/00_UPGRADE_AND_INTEGRATION_PLAN.md phase-3 mp-044

use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use jcode_memory_types::{
    MemoryEntry, MemoryScope, MemoryProvider as JcodeMemoryProvider,
    MemoryProviderConfig, GraphStats,
};
use mempalace_core::{
    Drawer, DrawerId, DrawerKind, MemoryScope as MpScope,
    Palace, PalaceBuilder, PalaceConfig,
    SearchHit, SearchScope,
    embedder_from_env, MemoryProvider as MpMemoryProvider,
};

/// jcode MemoryCategory → mempalace DrawerKind.
fn jcode_cat_to_kind(cat: &jcode_memory_types::MemoryCategory) -> DrawerKind {
    use jcode_memory_types::MemoryCategory as Jc;
    use mempalace_core::DrawerKind as MpDk;
    match cat {
        Jc::Fact => MpDk::Fact,
        Jc::Preference => MpDk::Preference,
        Jc::Entity => MpDk::Discovery,
        Jc::Correction => MpDk::Advice,
        Jc::Custom(_) => MpDk::Advice,
    }
}

/// A `MemoryProvider` backed by mempalace-core's `Palace`.
///
/// Construct with [`MempalaceProvider::new`]:
/// ```ignore
/// let provider = MempalaceProvider::new(config, ".jcode/palace".into()).await?;
/// ```
///
/// Then hand to jcode's `provider()` factory as `Arc::new(provider)`.
pub struct MempalaceProvider {
    palace: Palace,
    config: MemoryProviderConfig,
}

impl MempalaceProvider {
    /// Build a new MempalaceProvider.
    ///
    /// `palace_path` — directory where mempalace stores its SQLite + vectors.
    /// Convention: `<project_dir>/.jcode/palace`.
    pub async fn new(
        config: MemoryProviderConfig,
        palace_path: PathBuf,
    ) -> anyhow::Result<Self> {
        // embedder_from_env returns Box<dyn Embedder>; PalaceBuilder needs Arc.
        let embedder: Arc<dyn mempalace_core::Embedder> =
            Arc::from(embedder_from_env()?);

        let mut mp_config = PalaceConfig::default();
        mp_config.palace_path = palace_path;

        let palace = PalaceBuilder::new()
            .config(mp_config)
            .embedder(embedder)
            .open()
            .await?;

        Ok(Self { palace, config })
    }

    /// Build SearchScope from jcode MemoryScope + pagination params.
    fn search_scope(_scope: MemoryScope, limit: usize) -> SearchScope {
        SearchScope {
            limit,
            ..Default::default()
        }
    }

    /// Map jcode MemoryScope to mempalace MemoryScope for remember/forget.
    fn mp_scope(scope: MemoryScope) -> MpScope {
        match scope {
            MemoryScope::Project | MemoryScope::All => MpScope::Local,
            MemoryScope::Global => MpScope::Global,
        }
    }

    /// Build a Drawer from a jcode MemoryEntry.
    /// Tags and source are preserved in metadata since palace Drawer has no
    /// first-class fields for those.
    fn entry_to_drawer(entry: &MemoryEntry) -> Drawer {
        let mut drawer = Drawer::new(entry.content.clone())
            .kind(jcode_cat_to_kind(&entry.category));

        if !entry.tags.is_empty() {
            drawer.metadata
                .insert("tags".to_string(), serde_json::to_value(&entry.tags).unwrap());
        }
        if let Some(ref src) = entry.source {
            drawer.metadata
                .insert("source".to_string(), serde_json::json!(src));
        }
        // Pre-assigned IDs enable idempotent upserts.
        if !entry.id.is_empty() {
            drawer.id = Some(DrawerId::new(entry.id.clone()));
        }
        drawer
    }

    /// Map a mempalace SearchHit to a jcode MemoryEntry.
    /// palace SearchHit has no id/metadata — use content + similarity as defaults.
    fn hit_to_entry(hit: SearchHit) -> MemoryEntry {
        let now = chrono::Utc::now();
        MemoryEntry {
            id: Default::default(),   // palace SearchHit has no id
            category: jcode_memory_types::MemoryCategory::Fact,
            content: hit.text,
            tags: Vec::new(),
            search_text: String::new(),
            created_at: now,
            updated_at: now,
            access_count: 0,
            source: Some(hit.source_file).filter(|s| !s.is_empty()),
            trust: jcode_memory_types::TrustLevel::Medium,
            strength: 0,
            active: true,
            superseded_by: None,
            reinforcements: vec![],
            embedding: None,
            confidence: hit.similarity as f32,
        }
    }

    /// Synchronous block_on wrapper around Palace async methods.
    fn block_on<R, F: std::future::Future<Output = R>>(fut: F) -> R {
        tokio::runtime::Handle::current()
            .block_on(fut)
    }
}

#[async_trait]
impl JcodeMemoryProvider for MempalaceProvider {
    fn config(&self) -> &MemoryProviderConfig {
        &self.config
    }

    fn remember_project(&self, entry: MemoryEntry) -> Result<String> {
        let content = entry.content;
        let id = Self::block_on(
            self.palace.remember(content, MpScope::Local)
        )?;
        Ok(id.to_string())
    }

    fn remember_global(&self, entry: MemoryEntry) -> Result<String> {
        let content = entry.content;
        let id = Self::block_on(
            self.palace.remember(content, MpScope::Global)
        )?;
        Ok(id.to_string())
    }

    fn upsert_project_memory(&self, entry: MemoryEntry) -> Result<String> {
        let drawer = Self::entry_to_drawer(&entry);
        let id = Self::block_on(
            self.palace.add_drawer(drawer)
        )?;
        Ok(id.to_string())
    }

    fn upsert_global_memory(&self, entry: MemoryEntry) -> Result<String> {
        // mempalace scope doesn't distinguish project/global in add_drawer;
        // wing/room on the drawer would. For now, use Local.
        let drawer = Self::entry_to_drawer(&entry);
        let id = Self::block_on(
            self.palace.add_drawer(drawer)
        )?;
        Ok(id.to_string())
    }

    fn forget(&self, id: &str) -> Result<bool> {
        let did = DrawerId::new(id.to_string());
        Self::block_on(self.palace.forget(&did))
    }

    fn tag_memory(&self, _memory_id: &str, _tag: &str) -> Result<()> {
        // TODO(mp-044): palace Drawer has metadata; need palace.patch_drawer() to update it
        Ok(())
    }

    fn link_memories(&self, _from_id: &str, _to_id: &str, _weight: f32) -> Result<()> {
        // TODO(mp-061): wire to KnowledgeGraph::add_triple via palace
        Ok(())
    }

    fn boost_confidence(&self, _id: &str, _amount: f32) -> Result<()> {
        // mempalace has helpfulness_score but not exposed on Palace
        Ok(())
    }

    fn decay_confidence(&self, _id: &str, _amount: f32) -> Result<()> {
        Ok(())
    }

    fn list_all_scoped(&self, _scope: MemoryScope) -> Result<Vec<MemoryEntry>> {
        // TODO(mp-044): palace doesn't expose list_all yet
        Ok(vec![])
    }

    fn search_scoped(&self, query: &str, scope: MemoryScope) -> Result<Vec<MemoryEntry>> {
        let search_scope = Self::search_scope(scope, 10);
        let hits = Self::block_on(
            self.palace.search(query, &search_scope)
        )?;
        Ok(hits.into_iter().map(Self::hit_to_entry).collect())
    }

    fn find_similar_scoped(
        &self,
        text: &str,
        threshold: f32,
        limit: usize,
        scope: MemoryScope,
    ) -> Result<Vec<(MemoryEntry, f32)>> {
        let search_scope = Self::search_scope(scope, limit);
        // Store doesn't support min_similarity filter — filter post-query
        let hits = Self::block_on(
            self.palace.search(text, &search_scope)
        )?;
        Ok(hits
            .into_iter()
            .filter(|h| h.similarity >= threshold as f64)
            .map(|h| (Self::hit_to_entry(h.clone()), h.similarity as f32))
            .collect())
    }

    fn find_similar_with_embedding_scoped(
        &self,
        query_embedding: &[f32],
        threshold: f32,
        limit: usize,
        scope: MemoryScope,
    ) -> Result<Vec<(MemoryEntry, f32)>> {
        let search_scope = Self::search_scope(scope, limit);
        let hits = Self::block_on(
            self.palace.search_with_embedding(query_embedding, &search_scope)
        )?;
        Ok(hits
            .into_iter()
            .filter(|h| h.similarity >= threshold as f64)
            .map(|h| (Self::hit_to_entry(h.clone()), h.similarity as f32))
            .collect())
    }

    fn find_similar_with_cascade_scoped(
        &self,
        text: &str,
        threshold: f32,
        limit: usize,
        scope: MemoryScope,
    ) -> Result<Vec<(MemoryEntry, f32)>> {
        let hits = self.find_similar_scoped(text, threshold, limit, scope.clone())?;
        if !hits.is_empty() {
            return Ok(hits);
        }
        // Fall back to keyword (text search without embeddings)
        Ok(self.search_scoped(text, scope)?
            .into_iter()
            .map(|e| (e, 0.0_f32))
            .take(limit)
            .collect())
    }

    fn get_related(&self, memory_id: &str, depth: usize) -> Result<Vec<MemoryEntry>> {
        let did = DrawerId::new(memory_id.to_string());
        let hits = Self::block_on(self.palace.related(&did, depth))?;
        Ok(hits.into_iter().map(Self::hit_to_entry).collect())
    }

    fn graph_stats(&self) -> Result<GraphStats> {
        // palace.graph_stats() is async but this is a simple query
        let mp_stats = Self::block_on(self.palace.graph_stats())?;
        Ok(GraphStats {
            memories: mp_stats.total_entities,
            tags: mp_stats.total_entities, // palace has no tag count
            edges: mp_stats.total_triples,
            clusters: 0,
        })
    }

    fn extract_from_transcript<'a>(
        &'a self,
        transcript: &'a str,
        session_id: &'a str,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<Vec<String>>> + Send + 'a>> {
        let palace = self.palace.clone();
        Box::pin(async move {
            let ids = palace.extract_from_transcript(transcript, session_id).await?;
            Ok(ids.into_iter().map(|d| d.to_string()).collect())
        })
    }
}
