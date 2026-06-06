# Shared Memory for Multi-Agent — Master Plan
> Issue #382 | Cross-agent data sharing during coordinated workflows
> Research: oh-my-claudecode, oh-my-openagent, oh-my-codex, jcode existing infra

## Context

jcode has a sophisticated memory system (MemoryGraph with edges/clusters/cascade retrieval, mempalace adapter) but it's **session-scoped** — each agent's memory is isolated. The existing `SharedContext` is ephemeral (in-memory only, no persistence, no search, just string key/value). This plan adds:

1. **Shared memory pool** — swarm-scoped memory graph accessible by all agents in a team
2. **Agent identity** — every memory entry tracks who wrote it
3. **Shared memory tools** — read/write/list/delete/cleanup across agents
4. **Conflict resolution** — concurrent write handling
5. **Persistence** — shared memory survives server restarts
6. **Integration** — with existing SharedContext, channels, and notification system

---

## What jcode Already Has

### Memory System
- `MemoryEntry` with id, category, content, tags, embedding, confidence, trust, source
- `MemoryGraph` with edges (HasTag, RelatesTo, Supersedes, Contradicts, DerivedFrom, InCluster)
- `MemoryManager` — session-scoped, project/global storage
- `MempalaceAdapter` — type-conversion bridge to mempalace Palace
- Cascade retrieval: embedding similarity → BFS graph traversal

### Agent Communication
- `SharedContext` — ephemeral key/value store scoped to swarm_id (in-memory only)
- `NotificationType::SharedContext { key, value }` — fanout to swarm members
- Channel system — bidirectional pub/sub within swarm
- DM/broadcast delivery modes
- File-touch tracking for conflict detection

### Swarm Infrastructure
- `SwarmMemberRecord` with session_id, swarm_id, status, role
- `ChannelIndex` for channel subscriptions
- Team configs on disk (but no runtime coordination)

### What's Missing
1. No cross-agent memory visibility (MemoryManager is session-scoped)
2. SharedContext is ephemeral (no persistence, no search, no structure)
3. No memory sharing protocol (no NotificationType for memory events)
4. No shared memory graph (no ownership/provenance model)
5. No cross-store edges (link_memories rejects cross-store links)
6. Team system is config-only (no shared memory pool)
7. Mempalace has no multi-agent awareness

---

## Architecture

### New Crate: `jcode-shared-memory`

```
crates/jcode-shared-memory/
├── Cargo.toml
└── src/
    ├── lib.rs                  # public API, re-exports
    ├── pool.rs                 # SharedMemoryPool — swarm-scoped memory graph
    ├── entry.rs                # SharedMemoryEntry — extends MemoryEntry with agent identity
    ├── store.rs                # persistence (SQLite or JSON files)
    ├── tools.rs                # shared_memory_write/read/list/delete/cleanup
    ├── conflict.rs             # concurrent write resolution
    ├── notifications.rs        # MemoryUpdate notification types
    ├── sync.rs                 # cross-agent synchronization
    └── tests.rs
```

### Integration Points

```
Agent A writes to shared memory
  → SharedMemoryPool::write(entry with agent_id)
  → Persist to store
  → Broadcast MemoryUpdate notification to swarm
  → Agent B receives notification
  → Agent B can read/query shared memory
```

---

## Part 1: Data Structures

### 1.1 SharedMemoryEntry

```rust
/// Extends MemoryEntry with multi-agent identity and provenance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedMemoryEntry {
    // === Inherited from MemoryEntry ===
    pub id: String,
    pub category: MemoryCategory,
    pub content: String,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub confidence: f64,
    pub trust: TrustLevel,
    pub embedding: Option<Vec<f32>>,

    // === Multi-agent extensions ===
    /// Which agent created this memory
    pub author_agent_id: String,
    /// Which agent last modified this memory
    pub last_modified_by: String,
    /// Which swarm this memory belongs to
    pub swarm_id: String,
    /// Visibility scope
    pub scope: SharedMemoryScope,
    /// Version for conflict detection
    pub version: u64,
    /// Lock info (if currently being edited)
    pub lock: Option<MemoryLock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SharedMemoryScope {
    /// Visible to all agents in the swarm
    Swarm,
    /// Visible only to the author agent
    Private,
    /// Visible to specific agents
    Restricted(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLock {
    pub locked_by: String,
    pub locked_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryCategory {
    Fact,
    Preference,
    Entity,
    Correction,
    Custom(String),
    // === New for shared memory ===
    Decision,       // Architectural/design decisions
    Finding,        // Code review / analysis findings
    Blocker,        // Blocking issues
    Learning,       // Lessons learned during task
    Requirement,    // Confirmed requirements
}
```

### 1.2 SharedMemoryPool

```rust
/// Swarm-scoped memory pool accessible by all agents in a team
pub struct SharedMemoryPool {
    /// Which swarm this pool belongs to
    swarm_id: String,
    /// Storage backend
    store: SharedMemoryStore,
    /// In-memory graph for fast queries
    graph: SharedMemoryGraph,
    /// Event bus for notifications
    bus: Bus,
    /// Index for semantic search
    embedding_index: EmbeddingIndex,
}

impl SharedMemoryPool {
    /// Open or create a pool for a swarm
    pub async fn open(swarm_id: &str, working_dir: &Path) -> Result<Self>;

    /// Write a new memory entry
    pub async fn write(&self, entry: SharedMemoryEntry) -> Result<String>;

    /// Update an existing memory entry (with conflict detection)
    pub async fn update(&self, entry: SharedMemoryEntry) -> Result<UpdateResult>;

    /// Read a specific memory by ID
    pub async fn read(&self, id: &str) -> Result<Option<SharedMemoryEntry>>;

    /// List memories with filters
    pub async fn list(&self, filter: MemoryFilter) -> Result<Vec<SharedMemoryEntry>>;

    /// Search memories by semantic similarity
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>>;

    /// Delete a memory entry
    pub async fn delete(&self, id: &str, agent_id: &str) -> Result<()>;

    /// Cleanup stale/old memories
    pub async fn cleanup(&self, policy: CleanupPolicy) -> Result<CleanupResult>;

    /// Link two memories
    pub async fn link(&self, from_id: &str, to_id: &str, kind: EdgeKind) -> Result<()>;

    /// Get related memories via graph traversal
    pub async fn related(&self, id: &str, depth: usize) -> Result<Vec<SharedMemoryEntry>>;

    /// Subscribe to memory updates
    pub fn subscribe(&self) -> broadcast::Receiver<MemoryUpdate>;
}

#[derive(Debug, Clone)]
pub struct MemoryFilter {
    pub category: Option<MemoryCategory>,
    pub author: Option<String>,
    pub scope: Option<SharedMemoryScope>,
    pub tags: Vec<String>,
    pub since: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone)]
pub enum UpdateResult {
    Updated(SharedMemoryEntry),
    Conflict {
        local_version: u64,
        remote_version: u64,
        remote_author: String,
    },
}

#[derive(Debug, Clone)]
pub enum CleanupPolicy {
    /// Remove entries older than duration
    OlderThan(Duration),
    /// Remove entries with confidence below threshold
    LowConfidence(f64),
    /// Remove entries superseded by others
    Superseded,
    /// Remove private entries from stopped agents
    StaleAgentEntries,
}
```

### 1.3 SharedMemoryStore (Persistence)

```rust
/// Persistence backend for shared memory
/// Options: SQLite (preferred) or JSON files
pub struct SharedMemoryStore {
    /// Path to storage
    path: PathBuf,
    /// Database connection (if SQLite)
    db: Option<SqlitePool>,
}

impl SharedMemoryStore {
    /// Initialize store at path
    pub async fn open(path: &Path) -> Result<Self>;

    /// Insert a new entry
    pub async fn insert(&self, entry: &SharedMemoryEntry) -> Result<()>;

    /// Update an existing entry (returns false if version mismatch)
    pub async fn update(&self, entry: &SharedMemoryEntry) -> Result<bool>;

    /// Get by ID
    pub async fn get(&self, id: &str) -> Result<Option<SharedMemoryEntry>>;

    /// List with filter
    pub async fn list(&self, filter: &MemoryFilter) -> Result<Vec<SharedMemoryEntry>>;

    /// Delete by ID
    pub async fn delete(&self, id: &str) -> Result<()>;

    /// Get all entries for an agent
    pub async fn by_agent(&self, agent_id: &str) -> Result<Vec<SharedMemoryEntry>>;

    /// Get all entries for a swarm
    pub async fn by_swarm(&self, swarm_id: &str) -> Result<Vec<SharedMemoryEntry>>;

    /// Prune old entries
    pub async fn prune(&self, policy: &CleanupPolicy) -> Result<usize>;
}
```

### Storage Format (SQLite schema)

```sql
CREATE TABLE shared_memories (
    id TEXT PRIMARY KEY,
    swarm_id TEXT NOT NULL,
    author_agent_id TEXT NOT NULL,
    last_modified_by TEXT NOT NULL,
    category TEXT NOT NULL,
    content TEXT NOT NULL,
    tags TEXT,  -- JSON array
    scope TEXT NOT NULL,  -- JSON enum
    confidence REAL NOT NULL,
    trust TEXT NOT NULL,
    embedding BLOB,  -- f32 array as bytes
    version INTEGER NOT NULL DEFAULT 1,
    lock TEXT,  -- JSON lock info or NULL
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE shared_memory_edges (
    from_id TEXT NOT NULL,
    to_id TEXT NOT NULL,
    kind TEXT NOT NULL,  -- JSON EdgeKind
    created_at TEXT NOT NULL,
    PRIMARY KEY (from_id, to_id, kind),
    FOREIGN KEY (from_id) REFERENCES shared_memories(id),
    FOREIGN KEY (to_id) REFERENCES shared_memories(id)
);

CREATE INDEX idx_shared_memories_swarm ON shared_memories(swarm_id);
CREATE INDEX idx_shared_memories_agent ON shared_memories(author_agent_id);
CREATE INDEX idx_shared_memories_category ON shared_memories(category);
CREATE INDEX idx_shared_memories_updated ON shared_memories(updated_at);
```

---

## Part 2: Shared Memory Tools

### 2.1 shared_memory_write

```rust
/// Tool: Write a memory entry to the shared pool
pub struct SharedMemoryWriteTool;

impl SharedMemoryWriteTool {
    pub fn name() -> &'static str { "shared_memory_write" }

    pub fn description() -> &'static str {
        "Write a memory entry to the shared memory pool. \
         All agents in the swarm can read this memory."
    }

    pub fn parameters() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "content": {
                    "type": "string",
                    "description": "The memory content to store"
                },
                "category": {
                    "type": "string",
                    "enum": ["fact", "preference", "entity", "correction",
                             "decision", "finding", "blocker", "learning", "requirement"],
                    "description": "Category of the memory"
                },
                "tags": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Tags for categorization"
                },
                "scope": {
                    "type": "string",
                    "enum": ["swarm", "private", "restricted"],
                    "default": "swarm",
                    "description": "Visibility scope"
                }
            },
            "required": ["content", "category"]
        })
    }

    pub async fn execute(&self, ctx: &ToolContext, args: serde_json::Value) -> ToolResult {
        let pool = ctx.shared_memory_pool()?;
        let agent_id = ctx.session_id.clone();
        let swarm_id = ctx.swarm_id()?;

        let entry = SharedMemoryEntry {
            id: uuid(),
            category: parse_category(args["category"])?,
            content: args["content"].as_str()?.to_string(),
            tags: parse_tags(args["tags"])?,
            scope: parse_scope(args["scope"])?,
            author_agent_id: agent_id.clone(),
            last_modified_by: agent_id,
            swarm_id,
            version: 1,
            lock: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            confidence: 1.0,
            trust: TrustLevel::default(),
            embedding: None, // computed async
        };

        let id = pool.write(entry).await?;
        ToolResult::success(format!("Memory {} written to shared pool", id))
    }
}
```

### 2.2 shared_memory_read

```rust
/// Tool: Read a specific memory or search the shared pool
pub struct SharedMemoryReadTool;

impl SharedMemoryReadTool {
    pub fn name() -> &'static str { "shared_memory_read" }

    pub fn parameters() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "Specific memory ID to read"
                },
                "query": {
                    "type": "string",
                    "description": "Semantic search query"
                },
                "limit": {
                    "type": "integer",
                    "default": 10,
                    "description": "Max results for search"
                }
            }
        })
    }

    pub async fn execute(&self, ctx: &ToolContext, args: serde_json::Value) -> ToolResult {
        let pool = ctx.shared_memory_pool()?;

        if let Some(id) = args["id"].as_str() {
            // Direct read by ID
            match pool.read(id).await? {
                Some(entry) => ToolResult::success(format_memory_entry(&entry)),
                None => ToolResult::error("Memory not found"),
            }
        } else if let Some(query) = args["query"].as_str() {
            // Semantic search
            let limit = args["limit"].as_u64().unwrap_or(10) as usize;
            let results = pool.search(query, limit).await?;
            ToolResult::success(format_search_results(&results))
        } else {
            ToolResult::error("Provide either 'id' or 'query'")
        }
    }
}
```

### 2.3 shared_memory_list

```rust
/// Tool: List memories with optional filters
pub struct SharedMemoryListTool;

impl SharedMemoryListTool {
    pub fn name() -> &'static str { "shared_memory_list" }

    pub fn parameters() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "category": {"type": "string"},
                "author": {"type": "string", "description": "Filter by agent ID"},
                "tags": {"type": "array", "items": {"type": "string"}},
                "since": {"type": "string", "description": "ISO timestamp"},
                "limit": {"type": "integer", "default": 50}
            }
        })
    }

    pub async fn execute(&self, ctx: &ToolContext, args: serde_json::Value) -> ToolResult {
        let pool = ctx.shared_memory_pool()?;
        let filter = MemoryFilter {
            category: parse_optional_category(args["category"])?,
            author: args["author"].as_str().map(String::from),
            scope: None,
            tags: parse_tags(args["tags"])?,
            since: parse_optional_datetime(args["since"])?,
            limit: args["limit"].as_u64().map(|n| n as usize),
        };
        let entries = pool.list(filter).await?;
        ToolResult::success(format_memory_list(&entries))
    }
}
```

### 2.4 shared_memory_delete

```rust
/// Tool: Delete a memory entry (only by author or coordinator)
pub struct SharedMemoryDeleteTool;

impl SharedMemoryDeleteTool {
    pub fn name() -> &'static str { "shared_memory_delete" }

    pub async fn execute(&self, ctx: &ToolContext, args: serde_json::Value) -> ToolResult {
        let pool = ctx.shared_memory_pool()?;
        let id = args["id"].as_str()?;
        let agent_id = ctx.session_id.clone();

        // Check permission: only author or coordinator can delete
        let entry = pool.read(id).await?;
        if let Some(entry) = entry {
            if entry.author_agent_id != agent_id && ctx.role()? != SwarmRole::Coordinator {
                return ToolResult::error("Only the author or coordinator can delete this memory");
            }
        }

        pool.delete(id, &agent_id).await?;
        ToolResult::success(format!("Memory {} deleted", id))
    }
}
```

### 2.5 shared_memory_cleanup

```rust
/// Tool: Cleanup stale/old memories
pub struct SharedMemoryCleanupTool;

impl SharedMemoryCleanupTool {
    pub fn name() -> &'static str { "shared_memory_cleanup" }

    pub fn parameters() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "policy": {
                    "type": "string",
                    "enum": ["old", "low_confidence", "superseded", "stale_agents"],
                    "description": "Cleanup policy"
                },
                "older_than_hours": {
                    "type": "integer",
                    "default": 24,
                    "description": "For 'old' policy: remove entries older than N hours"
                },
                "min_confidence": {
                    "type": "number",
                    "default": 0.3,
                    "description": "For 'low_confidence' policy: remove entries below threshold"
                }
            },
            "required": ["policy"]
        })
    }

    pub async fn execute(&self, ctx: &ToolContext, args: serde_json::Value) -> ToolResult {
        let pool = ctx.shared_memory_pool()?;
        let policy = match args["policy"].as_str()? {
            "old" => CleanupPolicy::OlderThan(Duration::hours(
                args["older_than_hours"].as_i64().unwrap_or(24)
            )),
            "low_confidence" => CleanupPolicy::LowConfidence(
                args["min_confidence"].as_f64().unwrap_or(0.3)
            ),
            "superseded" => CleanupPolicy::Superseded,
            "stale_agents" => CleanupPolicy::StaleAgentEntries,
            _ => return ToolResult::error("Unknown cleanup policy"),
        };
        let result = pool.cleanup(policy).await?;
        ToolResult::success(format!("Cleaned up {} entries", result.removed))
    }
}
```

---

## Part 3: Conflict Resolution

### 3.1 Optimistic Locking with Version Check

```rust
/// Conflict resolution strategy: optimistic locking
pub struct ConflictResolver;

impl ConflictResolver {
    /// Attempt to update with conflict detection
    pub async fn update(
        store: &SharedMemoryStore,
        entry: &SharedMemoryEntry,
    ) -> Result<UpdateResult> {
        // 1. Read current version
        let current = store.get(&entry.id).await?;
        let current = match current {
            Some(e) => e,
            None => return Ok(UpdateResult::Updated(entry.clone())),
        };

        // 2. Version check
        if entry.version != current.version {
            return Ok(UpdateResult::Conflict {
                local_version: entry.version,
                remote_version: current.version,
                remote_author: current.last_modified_by.clone(),
            });
        }

        // 3. Lock check
        if let Some(ref lock) = current.lock {
            if lock.expires_at > Utc::now() && lock.locked_by != entry.last_modified_by {
                return Ok(UpdateResult::Conflict {
                    local_version: entry.version,
                    remote_version: current.version,
                    remote_author: lock.locked_by.clone(),
                });
            }
        }

        // 4. Increment version and update
        let mut updated = entry.clone();
        updated.version = current.version + 1;
        updated.updated_at = Utc::now();
        store.update(&updated).await?;

        Ok(UpdateResult::Updated(updated))
    }
}
```

### 3.2 Merge Strategy for Concurrent Writes

```rust
/// When two agents write to the same logical memory, merge instead of overwrite
pub struct MergeStrategy;

impl MergeStrategy {
    /// Try to merge two conflicting entries
    pub fn try_merge(
        local: &SharedMemoryEntry,
        remote: &SharedMemoryEntry,
    ) -> Option<SharedMemoryEntry> {
        // Only merge if same category and similar content
        if local.category != remote.category {
            return None;
        }

        // Check content similarity
        let similarity = cosine_similarity(
            local.embedding.as_ref()?,
            remote.embedding.as_ref()?,
        );

        if similarity < 0.8 {
            return None; // Too different, real conflict
        }

        // Merge: keep both contents, combine tags
        let mut merged = local.clone();
        merged.content = format!("{}\n\n---\n\n{}", local.content, remote.content);
        merged.tags = merge_tags(&local.tags, &remote.tags);
        merged.version = local.version.max(remote.version) + 1;
        merged.updated_at = Utc::now();
        merged.last_modified_by = "system:merge".to_string();

        Some(merged)
    }
}
```

### 3.3 Lock Management

```rust
/// Advisory lock for exclusive write access
pub struct LockManager;

impl LockManager {
    /// Acquire a lock on a memory entry
    pub async fn acquire(
        store: &SharedMemoryStore,
        entry_id: &str,
        agent_id: &str,
        duration: Duration,
    ) -> Result<LockResult> {
        let entry = store.get(entry_id).await?;
        let entry = match entry {
            Some(e) => e,
            None => return Ok(LockResult::NotFound),
        };

        // Check existing lock
        if let Some(ref lock) = entry.lock {
            if lock.expires_at > Utc::now() {
                if lock.locked_by == agent_id {
                    return Ok(LockResult::AlreadyHeld);
                }
                return Ok(LockResult::HeldBy(lock.locked_by.clone()));
            }
        }

        // Acquire lock
        let mut updated = entry.clone();
        updated.lock = Some(MemoryLock {
            locked_by: agent_id.to_string(),
            locked_at: Utc::now(),
            expires_at: Utc::now() + duration,
        });
        store.update(&updated).await?;

        Ok(LockResult::Acquired)
    }

    /// Release a lock
    pub async fn release(
        store: &SharedMemoryStore,
        entry_id: &str,
        agent_id: &str,
    ) -> Result<()> {
        let entry = store.get(entry_id).await?;
        if let Some(mut entry) = entry {
            if let Some(ref lock) = entry.lock {
                if lock.locked_by == agent_id {
                    entry.lock = None;
                    store.update(&entry).await?;
                }
            }
        }
        Ok(())
    }
}

pub enum LockResult {
    Acquired,
    AlreadyHeld,
    HeldBy(String),
    NotFound,
}
```

---

## Part 4: Notifications

### 4.1 MemoryUpdate Notification

```rust
/// Notification broadcast when shared memory changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryUpdate {
    /// New memory written
    Created {
        entry_id: String,
        author: String,
        category: String,
        swarm_id: String,
    },
    /// Existing memory updated
    Updated {
        entry_id: String,
        author: String,
        version: u64,
        swarm_id: String,
    },
    /// Memory deleted
    Deleted {
        entry_id: String,
        author: String,
        swarm_id: String,
    },
    /// Memory linked to another
    Linked {
        from_id: String,
        to_id: String,
        kind: String,
        swarm_id: String,
    },
}

/// Integration with existing notification system
impl From<MemoryUpdate> for NotificationType {
    fn from(update: MemoryUpdate) -> Self {
        NotificationType::MemoryUpdate {
            payload: serde_json::to_string(&update).unwrap_or_default(),
        }
    }
}
```

### 4.2 NotificationType Extension

```rust
// Add to existing NotificationType enum in jcode-protocol
pub enum NotificationType {
    // ... existing variants ...
    FileConflict { ... },
    SharedContext { key: String, value: String },
    Message { ... },
    // NEW: Memory update notification
    MemoryUpdate { payload: String },
}
```

### 4.3 Channel Integration

```rust
/// Subscribe to memory updates for a swarm
pub fn subscribe_memory_channel(
    channel_index: &ChannelIndex,
    swarm_id: &str,
    session_id: &str,
) {
    let channel = format!("memory:{}", swarm_id);
    channel_index.subscribe(session_id, swarm_id, &channel);
}

/// Broadcast memory update to swarm
pub fn broadcast_memory_update(
    bus: &Bus,
    swarm_id: &str,
    update: &MemoryUpdate,
) {
    let channel = format!("memory:{}", swarm_id);
    let notification = NotificationType::from(update.clone());
    bus.send(BusEvent::Notification {
        channel,
        notification,
    });
}
```

---

## Part 5: Integration with Existing Systems

### 5.1 SharedContext Bridge

```rust
/// Bridge between SharedContext (ephemeral key/value) and SharedMemory (persistent graph)
pub struct SharedContextBridge;

impl SharedContextBridge {
    /// Promote a SharedContext entry to shared memory
    pub async fn promote_to_memory(
        pool: &SharedMemoryPool,
        context: &SharedContext,
    ) -> Result<String> {
        let entry = SharedMemoryEntry {
            id: uuid(),
            category: MemoryCategory::Fact,
            content: format!("{}: {}", context.key, context.value),
            tags: vec!["from-shared-context".to_string()],
            author_agent_id: context.from_session.clone(),
            last_modified_by: context.from_session.clone(),
            swarm_id: String::new(), // filled from context
            scope: SharedMemoryScope::Swarm,
            version: 1,
            lock: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            confidence: 0.8,
            trust: TrustLevel::default(),
            embedding: None,
        };
        pool.write(entry).await
    }

    /// Sync shared memory changes back to SharedContext
    pub async fn sync_to_context(
        pool: &SharedMemoryPool,
        context_store: &SharedContextStore,
    ) -> Result<()> {
        // Periodically sync recent shared memory entries
        // to SharedContext for backward compatibility
        let recent = pool.list(MemoryFilter {
            since: Some(Utc::now() - Duration::minutes(5)),
            limit: Some(100),
            ..Default::default()
        }).await?;

        for entry in recent {
            context_store.set(
                &entry.swarm_id,
                &format!("memory:{}", entry.id),
                &entry.content,
            ).await?;
        }

        Ok(())
    }
}
```

### 5.2 Mempalace Integration

```rust
/// Extend MempalaceAdapter with swarm-scoped wings
impl MempalaceAdapter {
    /// Open a swarm-scoped Palace
    pub async fn open_swarm(swarm_id: &str, base_path: &Path) -> Result<Self> {
        let palace_path = base_path.join("swarms").join(swarm_id);
        let palace = Palace::open(&palace_path).await?;
        Ok(Self { palace })
    }

    /// Write to swarm-scoped wing
    pub async fn write_swarm(&self, entry: SharedMemoryEntry) -> Result<()> {
        let drawer: Drawer = entry.into();  // reuse existing conversion
        self.palace.add_drawer(drawer).await
    }
}
```

### 5.3 Team System Integration

```rust
/// Extend TeamConfig with shared memory pool reference
impl TeamConfig {
    /// Get or create the shared memory pool for this team
    pub async fn shared_memory_pool(&self, working_dir: &Path) -> Result<SharedMemoryPool> {
        SharedMemoryPool::open(&self.name, working_dir).await
    }
}
```

### 5.4 Bus Integration

```rust
// Add to BusEvent enum
pub enum BusEvent {
    // ... existing variants ...
    /// Shared memory update (in-process)
    MemoryShared {
        swarm_id: String,
        update: MemoryUpdate,
    },
}
```

---

## Part 6: Tool Registration

### Register in tool/mod.rs

```rust
// In crates/jcode-app-core/src/tool/mod.rs
pub fn register_shared_memory_tools(registry: &mut ToolRegistry, ctx: &ToolContext) {
    if ctx.swarm_id().is_ok() {
        registry.register(SharedMemoryWriteTool, ctx);
        registry.register(SharedMemoryReadTool, ctx);
        registry.register(SharedMemoryListTool, ctx);
        registry.register(SharedMemoryDeleteTool, ctx);
        registry.register(SharedMemoryCleanupTool, ctx);
    }
}
```

### Tool Descriptions for LLM

```rust
pub const SHARED_MEMORY_TOOLS_DESC: &str = r#"## Shared Memory Tools

When working in a team/swarm, you have access to shared memory:

- `shared_memory_write` — Store information that other agents can read
  Use for: decisions, findings, blockers, learnings, requirements
- `shared_memory_read` — Read specific memory or search by query
- `shared_memory_list` — List memories with filters (category, author, tags)
- `shared_memory_delete` — Remove a memory (only your own or coordinator)
- `shared_memory_cleanup` — Clean up stale/old memories

Shared memory persists across sessions and is visible to all swarm members.
"#;
```

---

## Part 7: Edge Cases

### 7.1 Agent Crashes While Holding Lock

**Resolution:** Lock has `expires_at` field. After expiry, lock is automatically released. Default lock duration: 5 minutes.

### 7.2 Concurrent Writes to Same Entry

**Resolution:** Optimistic locking with version check. If version mismatch, return Conflict with both versions. Caller decides: retry, merge, or abort.

### 7.3 Agent Reads Memory Written by Stopped Agent

**Resolution:** Memories persist after agent stops. `CleanupPolicy::StaleAgentEntries` can optionally clean up entries from stopped agents.

### 7.4 Swarm Dissolves While Memory Exists

**Resolution:** Shared memory pool persists on disk. Can be re-opened if swarm is recreated. `shared_memory_cleanup` with `OlderThan` policy for manual cleanup.

### 7.5 Private Memory Needs to Become Shared

**Resolution:** `shared_memory_write` with `scope: "swarm"` copies from private to shared. Original private entry remains.

### 7.6 Memory Conflicts During Merge

**Resolution:** `MergeStrategy::try_merge` checks content similarity. If >0.8 similar, merge contents. If <0.8, report conflict to user.

### 7.7 Large Memory Pool Performance

**Resolution:** SQLite indexing on swarm_id, author, category, updated_at. Embedding index for semantic search. Pagination on list operations.

---

## Part 8: Implementation Phases

### Phase 1: Core Data Structures
1. Create `jcode-shared-memory` crate
2. `entry.rs` — SharedMemoryEntry with agent identity
3. `pool.rs` — SharedMemoryPool basic CRUD
4. `store.rs` — SQLite persistence
5. Unit tests

### Phase 2: Tools
6. `tools.rs` — shared_memory_write/read/list/delete/cleanup
7. Register tools in tool/mod.rs
8. Integration tests

### Phase 3: Notifications
9. `notifications.rs` — MemoryUpdate type
10. Extend NotificationType in jcode-protocol
11. Channel integration for memory events
12. Bus integration for in-process events

### Phase 4: Conflict Resolution
13. `conflict.rs` — optimistic locking
14. Merge strategy for concurrent writes
15. Lock management

### Phase 5: Integration
16. SharedContext bridge
17. Mempalace integration (swarm-scoped wings)
18. Team system integration
19. E2E tests

---

## Part 9: Files Modified/Created

### New Files (crates/jcode-shared-memory/)

| File | Lines (est.) | Purpose |
|------|-------------|---------|
| `Cargo.toml` | 40 | Crate config |
| `src/lib.rs` | 80 | Public API |
| `src/entry.rs` | 200 | SharedMemoryEntry + types |
| `src/pool.rs` | 400 | SharedMemoryPool CRUD + search |
| `src/store.rs` | 350 | SQLite persistence |
| `src/tools.rs` | 500 | 5 shared memory tools |
| `src/conflict.rs` | 200 | Optimistic locking + merge |
| `src/notifications.rs` | 150 | MemoryUpdate + channel integration |
| `src/sync.rs` | 150 | Cross-agent synchronization |
| `src/tests.rs` | 400 | Comprehensive tests |
| **Total** | **~2,470** | |

### Modified Files

| File | Change | Lines (est.) |
|------|--------|-------------|
| `Cargo.toml` | Add workspace member | 2 |
| `crates/jcode-protocol/src/notifications.rs` | Add MemoryUpdate variant | 10 |
| `crates/jcode-protocol/src/wire.rs` | Add MemoryUpdate wire format | 15 |
| `crates/jcode-app-core/src/tool/mod.rs` | Register shared memory tools | 20 |
| `crates/jcode-app-core/src/server/state.rs` | SharedContext bridge | 30 |
| `crates/jcode-swarm-core/src/lib.rs` | Memory channel support | 15 |
| `crates/jcode-base/src/bus.rs` | MemoryShared event | 10 |
| `crates/jcode-config-types/src/lib.rs` | SharedMemoryConfig | 15 |
| **Total** | | **~117** |

### Grand Total: ~2,587 lines

---

## Part 10: Verification

1. `cargo check -p jcode-shared-memory` — compiles
2. `cargo test -p jcode-shared-memory` — unit tests pass
3. `cargo test -p jcode-app-core` — integration tests pass
4. Manual: Agent A writes memory → Agent B can read it
5. Manual: Two agents write simultaneously → conflict detected
6. Manual: Agent crashes → lock expires → others can proceed
7. Manual: Swarm dissolves → memory persists → can be re-opened
8. Manual: shared_memory_search finds memories by semantic similarity
9. Manual: MemoryUpdate notifications reach all swarm members
10. Manual: Cleanup removes stale entries
