// =====================================================================
// MemoryProvider trait — seam between MemoryManager and alternative backends
// =====================================================================
//
// This trait defines the contract for jcode's memory backend. The existing
// MemoryManager is one implementation (JcodeLocalProvider). A future
// MempalaceProvider backed by mempalace-core implements the same trait,
// allowing jcode to switch backends via config without changing call sites.
//
// Ref: docs/research/03_jcode_memory_internals.md §A "jcode MemoryProvider trait sketch"
// Ref: docs/research/00_UPGRADE_AND_INTEGRATION_PLAN.md phase-3 mp-040

use anyhow::Result;
use async_trait::async_trait;
use std::pin::Pin;
use std::sync::Arc;

use crate::{MemoryEntry, MemoryScope};

/// Aggregate statistics about a memory provider's store and graph.
#[derive(Debug, Clone, Default)]
pub struct GraphStats {
    pub memories: usize,
    pub tags: usize,
    pub edges: usize,
    pub clusters: usize,
}

/// Configuration for a memory provider instance.
#[derive(Debug, Clone)]
pub struct MemoryProviderConfig {
    /// Path to the project directory. `None` means use CWD.
    pub project_dir: Option<std::path::PathBuf>,
    /// Whether to include skills in memory operations.
    pub include_skills: bool,
    /// When true, use isolated test storage instead of real memory.
    pub test_mode: bool,
    /// Minimum similarity threshold for embedding-based search.
    pub embedding_threshold: f32,
    /// Maximum hits to return from embedding search.
    pub embedding_max_hits: usize,
    /// Minimum similarity for storage-level deduplication.
    pub storage_dedup_threshold: f32,
}

impl Default for MemoryProviderConfig {
    fn default() -> Self {
        Self {
            project_dir: None,
            include_skills: true,
            test_mode: false,
            embedding_threshold: 0.5,
            embedding_max_hits: 10,
            storage_dedup_threshold: 0.85,
        }
    }
}

/// Primary trait for jcode memory backends.
///
/// Implementors must be `Send + Sync + 'static`. All methods return
/// `Result` so backends can fail gracefully (e.g., missing storage dirs,
/// corrupt files, embedder unavailable).
///
/// MemoryManager (jcode-local backend) is the canonical impl. A future
/// MempalaceProvider backed by mempalace-core implements the same trait
/// for the mempalace integration path.
#[async_trait]
pub trait MemoryProvider: Send + Sync + 'static {
    /// The configuration this provider was built with.
    fn config(&self) -> &MemoryProviderConfig;

    // ---- Insert / mutate ----

    /// Store a memory entry in the project-scoped memory store.
    fn remember_project(&self, entry: MemoryEntry) -> Result<String>;

    /// Store a memory entry in the global (user-level) memory store.
    fn remember_global(&self, entry: MemoryEntry) -> Result<String>;

    /// Upsert (insert or update) a project-scoped memory entry.
    fn upsert_project_memory(&self, entry: MemoryEntry) -> Result<String>;

    /// Upsert (insert or update) a global-scoped memory entry.
    fn upsert_global_memory(&self, entry: MemoryEntry) -> Result<String>;

    /// Delete a memory entry by id. Returns `true` if the entry was found
    /// and deleted, `false` if it did not exist.
    fn forget(&self, id: &str) -> Result<bool>;

    /// Add a tag to an existing memory entry.
    fn tag_memory(&self, memory_id: &str, tag: &str) -> Result<()>;

    /// Create a directed weighted edge from one memory to another.
    fn link_memories(&self, from_id: &str, to_id: &str, weight: f32) -> Result<()>;

    // ---- Confidence (used by post-retrieval maintenance) ----

    /// Boost a memory's confidence score (called when memory was useful).
    fn boost_confidence(&self, id: &str, amount: f32) -> Result<()>;

    /// Decay a memory's confidence score (called when memory was retrieved
    /// but deemed not relevant).
    fn decay_confidence(&self, id: &str, amount: f32) -> Result<()>;

    // ---- Read / search ----

    /// List all memory entries matching the given scope.
    fn list_all_scoped(&self, scope: MemoryScope) -> Result<Vec<MemoryEntry>>;

    /// Full-text search over memory entries matching the given scope.
    fn search_scoped(&self, query: &str, scope: MemoryScope) -> Result<Vec<MemoryEntry>>;

    /// Find memories similar to the given text using embedding similarity.
    fn find_similar_scoped(
        &self,
        text: &str,
        threshold: f32,
        limit: usize,
        scope: MemoryScope,
    ) -> Result<Vec<(MemoryEntry, f32)>>;

    /// Find memories similar to a pre-computed embedding vector.
    fn find_similar_with_embedding_scoped(
        &self,
        query_embedding: &[f32],
        threshold: f32,
        limit: usize,
        scope: MemoryScope,
    ) -> Result<Vec<(MemoryEntry, f32)>>;

    /// Cascade search: try embedding, fall back to keyword if below threshold.
    fn find_similar_with_cascade_scoped(
        &self,
        text: &str,
        threshold: f32,
        limit: usize,
        scope: MemoryScope,
    ) -> Result<Vec<(MemoryEntry, f32)>>;

    /// Walk the memory graph from a given node, returning related entries.
    fn get_related(&self, memory_id: &str, depth: usize) -> Result<Vec<MemoryEntry>>;

    // ---- Stats ----

    /// Return aggregate statistics about the memory store and graph.
    fn graph_stats(&self) -> Result<GraphStats>;

    // ---- Lifecycle ----

    /// Extract memories from a conversation transcript and store them.
    /// Returns ids of the newly created memory entries.
    fn extract_from_transcript<'a>(
        &'a self,
        transcript: &'a str,
        session_id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>>> + Send + 'a>>;
}

/// Type alias for a dynamically-dispatched `MemoryProvider`.
///
/// Call sites that need a backend-agnostic handle use `DynMemoryProvider`.
/// jcode's `crate::memory::provider()` factory returns this type.
pub type DynMemoryProvider = Arc<dyn MemoryProvider>;