# Shared Memory via Mempalace — Master Plan
> Issue #382 | Extend mempalace coordination module + wire into jcode
> Research: oh-my-claudecode, oh-my-openagent, oh-my-codex, pi-agent-rust, mempalace_rust

## Context

jcode has switched to mempalace as its memory backend. mempalace_rust already has a `coordination/` module with:
- `SignalStore` — inter-agent messaging (SQLite)
- `LeaseStore` — ownership leases with TTL (SQLite)
- `Team` — team shared items (in-memory)
- `Mesh` — P2P mesh sync (in-memory)
- `ActionStore` — action lifecycle (SQLite)
- `CheckpointStore` — blocking conditions (SQLite)
- `RoutineStore` — composable workflows (SQLite)
- `Frontier` — action prioritization (pure functions)
- 20+ MCP tools already exposed

**What's missing (6 features from comparison):**
1. Live delivery to idle agents (from oh-my-openagent)
2. Event sourcing for replay (from oh-my-codex)
3. File reservations (from pi-agent-rust)
4. Saturation signals (from pi-agent-rust)
5. Artifact handoff for large payloads (from oh-my-claudecode)
6. Two-phase claim protocol (from oh-my-openagent)

**Approach:** Extend mempalace's coordination module with these 6 features, then wire into jcode via `jcode-mempalace-adapter`.

---

## Part 1: What mempalace Already Has (No Changes Needed)

### 1.1 SignalStore (signals.rs)
- SQLite-backed inter-agent messaging
- 5 signal types: Info, Request, Response, Alert, Handoff
- Thread support, reply-to, read/unread tracking
- Expiry support
- MCP tools: `mempalace_signal_send`, `mempalace_signal_read`

### 1.2 LeaseStore (leases.rs)
- SQLite-backed ownership leases
- TTL: 10 min default, 60 min max
- acquire/release/renew/cleanup
- Active lease query
- MCP tool: `mempalace_lease`

### 1.3 Team (team.rs)
- In-memory team shared items
- 3 item types: Memory, Pattern, Observation
- Visibility: Shared/Private
- Team profile aggregation (top concepts, files, patterns)
- MCP tools: `mempalace_team_share`, `mempalace_team_feed`

### 1.4 ActionStore (actions.rs)
- SQLite-backed action lifecycle
- Status: Pending/InProgress/Completed/Failed/Cancelled/Blocked
- Dependency edges: Blocks/BlockedBy/DependsOn/RelatesTo/Supersedes
- Auto-propagation: unblocks downstream when dependencies complete
- MCP tools: `mempalace_action_create`, `mempalace_action_update`, `mempalace_frontier`, `mempalace_next`

### 1.5 CheckpointStore (checkpoints.rs)
- SQLite-backed blocking conditions
- Types: CI, Approval, Deploy, Timer, Manual
- Auto-block parent action on create
- Auto-unblock when all checkpoints passed
- Expiry support

### 1.6 RoutineStore (routines.rs)
- SQLite-backed composable workflows
- Sequential step execution with dependency chains
- Run tracking with step results
- MCP tool: `mempalace_routine_run`

### 1.7 Mesh (mesh.rs)
- In-memory P2P mesh network
- Peer registration with URL validation (SSRF protection)
- Sync scopes: memories, actions, semantic, procedural, relations, graph
- Auth token support
- MCP tool: `mempalace_mesh_sync`

### 1.8 Frontier (frontier.rs)
- Pure function: score actions by priority + age + in-progress bonus
- Filter by agent (exclude leased actions)
- MCP tools: `mempalace_frontier`, `mempalace_next`

---

## Part 2: Features to Add to mempalace

### 2.1 Live Delivery (from oh-my-openagent)

**What:** Inject messages directly into idle agent sessions at turn boundaries, bypassing inbox polling.

**Where to add:** `coordination/live_delivery.rs` (new file)

**Design:**

```rust
/// Live delivery system for inter-agent messages
pub struct LiveDelivery {
    /// Pending deliveries per agent
    pending: Arc<RwLock<HashMap<String, Vec<PendingDelivery>>>>,
    /// Delivery history for ack tracking
    history: Arc<RwLock<Vec<DeliveryRecord>>>,
}

pub struct PendingDelivery {
    pub signal_id: String,
    pub from_agent: String,
    pub to_agent: String,
    pub content: String,
    pub signal_type: SignalType,
    pub created_at: DateTime<Utc>,
    pub status: DeliveryStatus,
}

pub enum DeliveryStatus {
    Queued,
    Injected,
    Acknowledged,
    Failed(String),
}

pub struct DeliveryRecord {
    pub signal_id: String,
    pub injected_at: DateTime<Utc>,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub turn_number: Option<usize>,
}
```

**Integration with jcode:**
- jcode's agent turn loop calls `live_delivery.poll(agent_id)` at turn start
- If pending messages exist, inject as system reminder before user message
- Format as XML `<peer_message>` envelopes (like oh-my-openagent)
- After turn completes, call `live_delivery.ack(signal_id)` to mark delivered
- On agent error, call `live_delivery.requeue(signal_id)` to retry

**MCP tool:** `mempalace_live_deliver` — check and inject pending messages

### 2.2 Event Sourcing (from oh-my-codex)

**What:** Record all coordination mutations as an append-only event log for deterministic replay and crash recovery.

**Where to add:** `coordination/event_log.rs` (new file)

**Design:**

```rust
/// Event-sourced coordination log
pub struct CoordinationEventLog {
    conn: Connection,  // SQLite
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoordinationEvent {
    // Signal events
    SignalSent { signal_id: String, from: String, to: String, signal_type: String },
    SignalRead { signal_id: String, agent_id: String },
    
    // Lease events
    LeaseAcquired { lease_id: String, action_id: String, agent_id: String, ttl_minutes: i64 },
    LeaseReleased { lease_id: String, result: Option<String> },
    LeaseRenewed { lease_id: String, extend_minutes: i64 },
    LeaseExpired { lease_id: String },
    
    // Action events
    ActionCreated { action_id: String, title: String, status: String, priority: u8 },
    ActionStatusChanged { action_id: String, from: String, to: String },
    ActionEdgeAdded { from_id: String, to_id: String, edge_type: String },
    
    // Team events
    TeamItemShared { item_id: String, shared_by: String, item_type: String },
    
    // File reservation events
    FileReserved { path: String, agent_id: String, mode: String },
    FileReleased { path: String, agent_id: String },
    
    // Live delivery events
    DeliveryQueued { signal_id: String, to_agent: String },
    DeliveryInjected { signal_id: String, turn_number: usize },
    DeliveryAcknowledged { signal_id: String },
    DeliveryFailed { signal_id: String, reason: String },
}

impl CoordinationEventLog {
    pub fn open(db_path: &Path) -> Result<Self>;
    pub fn append(&self, event: &CoordinationEvent) -> Result<u64>;  // returns sequence
    pub fn append_batch(&self, events: &[CoordinationEvent]) -> Result<u64>;
    pub fn replay_from(&self, sequence: u64) -> Result<Vec<(u64, CoordinationEvent)>>;
    pub fn replay_all(&self) -> Result<Vec<(u64, CoordinationEvent)>>;
    pub fn compact(&self, before_sequence: u64) -> Result<usize>;
    pub fn latest_sequence(&self) -> Result<u64>;
    pub fn snapshot(&self) -> Result<CoordinationSnapshot>;
}
```

**SQL schema:**
```sql
CREATE TABLE IF NOT EXISTS coordination_events (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type TEXT NOT NULL,
    event_data TEXT NOT NULL,  -- JSON
    created_at TEXT NOT NULL,
    correlation_id TEXT
);
CREATE INDEX IF NOT EXISTS idx_events_type ON coordination_events(event_type);
CREATE INDEX IF NOT EXISTS idx_events_created ON coordination_events(created_at);
CREATE INDEX IF NOT EXISTS idx_events_correlation ON coordination_events(correlation_id);
```

**Integration:**
- Every mutation in SignalStore, LeaseStore, ActionStore, etc. also appends to the event log
- `replay_from()` for crash recovery — rebuild state by replaying events
- `compact()` to remove old events (keep last N or after checkpoint)

### 2.3 File Reservations (from pi-agent-rust)

**What:** Explicit exclusive/shared file-level locks for workspace coordination between agents.

**Where to add:** `coordination/file_reservations.rs` (new file)

**Design:**

```rust
/// File reservation system for workspace coordination
pub struct FileReservationStore {
    conn: Connection,  // SQLite
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileReservation {
    pub id: String,
    pub path_pattern: String,  // glob pattern (e.g., "src/auth/*.rs")
    pub agent_id: String,
    pub mode: ReservationMode,
    pub reason: Option<String>,
    pub acquired_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub released_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReservationMode {
    Exclusive,  // only one agent can hold
    Shared,     // multiple agents can read
}

impl FileReservationStore {
    pub fn open(db_path: &Path) -> Result<Self>;
    
    /// Acquire a file reservation
    pub fn acquire(&self, path_pattern: &str, agent_id: &str, mode: ReservationMode, reason: Option<&str>, ttl_minutes: i64) -> Result<FileReservation>;
    
    /// Release a reservation
    pub fn release(&self, reservation_id: &str) -> Result<()>;
    
    /// Check if a path conflicts with existing reservations
    pub fn check_conflict(&self, path_pattern: &str, agent_id: &str, mode: ReservationMode) -> Result<ReservationConflict>;
    
    /// List all active reservations
    pub fn list_active(&self) -> Result<Vec<FileReservation>>;
    
    /// List reservations for a specific agent
    pub fn by_agent(&self, agent_id: &str) -> Result<Vec<FileReservation>>;
    
    /// Cleanup expired reservations
    pub fn cleanup(&self) -> Result<usize>;
    
    /// Get reservation heatmap (for conflict prediction)
    pub fn heatmap(&self) -> Result<Vec<ReservationHeatmapEntry>>;
}

pub enum ReservationConflict {
    None,
    ExclusiveConflict { holder: String, expires_at: DateTime<Utc> },
    SharedWithExclusive { holder: String },
    SameAgent,
}

pub struct ReservationHeatmapEntry {
    pub path_pattern: String,
    pub active_count: usize,
    pub exclusive_count: usize,
    pub shared_count: usize,
    pub agents: Vec<String>,
}
```

**SQL schema:**
```sql
CREATE TABLE IF NOT EXISTS file_reservations (
    id TEXT PRIMARY KEY,
    path_pattern TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    mode TEXT NOT NULL,  -- 'exclusive' or 'shared'
    reason TEXT,
    acquired_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    released_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_reservations_path ON file_reservations(path_pattern);
CREATE INDEX IF NOT EXISTS idx_reservations_agent ON file_reservations(agent_id);
CREATE INDEX IF NOT EXISTS idx_reservations_expires ON file_reservations(expires_at);
```

**Conflict detection:** Glob pattern overlap + mode check. Exclusive conflicts block; shared-with-exclusive warns.

**MCP tools:** `mempalace_file_reserve`, `mempalace_file_release`, `mempalace_file_conflicts`

### 2.4 Saturation Signals (from pi-agent-rust)

**What:** Typed signals that detect coordination problems (duplicate work, stale threads, repeated blockers).

**Where to add:** `coordination/saturation.rs` (new file)

**Design:**

```rust
/// Saturation signal detector
pub struct SaturationDetector {
    config: SaturationConfig,
}

#[derive(Debug, Clone)]
pub struct SaturationConfig {
    pub saturation_window_ms: u64,           // default: 3_600_000 (1 hour)
    pub stale_thread_after_ms: u64,          // default: 1_800_000 (30 min)
    pub min_new_actions_per_window: usize,   // default: 1
    pub repeated_blocker_threshold: usize,   // default: 2
    pub duplicate_work_threshold: usize,     // default: 2
    pub coordination_chatter_threshold: usize, // default: 5
    pub low_throughput_event_threshold: usize, // default: 1
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SaturationSignal {
    FewNewActions,
    RepeatedBlockers,
    DuplicateWork,
    StaleIntroductionsWithoutClaims,
    HighChatterLowThroughput,
    StaleThreads,
}

#[derive(Debug, Clone)]
pub struct SaturationReport {
    pub signals: Vec<(SaturationSignal, SignalEvidence)>,
    pub saturated: bool,
    pub reasons: Vec<String>,
    pub recommendations: Vec<SkillSwitchRecommendation>,
}

pub struct SkillSwitchRecommendation {
    pub signal: SaturationSignal,
    pub recommended_skill: String,
    pub confidence: String,  // "high", "medium", "low"
}
```

**Detection logic:**
- `FewNewActions`: count actions created in window < threshold
- `RepeatedBlockers`: FNV-1a hash blocker evidence, count duplicates
- `DuplicateWork`: detect "already claimed", "duplicate" in action descriptions
- `StaleThreads`: signals with no activity for > stale_thread_after_ms
- `HighChatterLowThroughput`: many signals but few completions
- `StaleIntroductionsWithoutClaims`: intro signals without follow-up claims

**Blocker fingerprinting (FNV-1a):**
```rust
fn blocker_fingerprint(evidence: &str) -> String {
    let normalized = normalize_evidence(evidence);  // paths→<path>, UUIDs→<uuid>, etc.
    let hash = fnv1a_64(&normalized);
    format!("blocker:{:016x}", hash)
}
```

**MCP tool:** `mempalace_saturation_check` — run saturation analysis

### 2.5 Artifact Handoff (from oh-my-claudecode)

**What:** For large payloads (>2KB), write to separate artifact files with retention levels.

**Where to add:** `coordination/artifacts.rs` (new file)

**Design:**

```rust
/// Artifact store for large payloads
pub struct ArtifactStore {
    base_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: String,
    pub category: String,       // "signal_body", "action_result", "shared_item"
    pub entity_id: String,      // the signal/action/item this artifact belongs to
    pub content: String,        // the large payload
    pub size_bytes: usize,
    pub retention: Retention,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Retention {
    Ephemeral,          // delete after session
    Session,            // delete after swarm ends
    UntilCompletion,    // delete when parent action completes
    Persistent,         // keep forever
}

impl ArtifactStore {
    pub fn new(base_dir: PathBuf) -> Self;
    
    /// Store an artifact, returns artifact ID
    pub fn store(&self, category: &str, entity_id: &str, content: &str, retention: Retention) -> Result<String>;
    
    /// Read an artifact
    pub fn read(&self, artifact_id: &str) -> Result<Option<Artifact>>;
    
    /// Delete an artifact
    pub fn delete(&self, artifact_id: &str) -> Result<()>;
    
    /// Cleanup expired artifacts
    pub fn cleanup(&self) -> Result<usize>;
    
    /// List artifacts for an entity
    pub fn by_entity(&self, entity_id: &str) -> Result<Vec<Artifact>>;
}
```

**Storage:** Files at `{base_dir}/artifacts/{category}/{artifact_id}.json`

**Integration with SignalStore:**
```rust
// When sending a signal with large content:
if content.len() > 2048 {
    let artifact_id = artifact_store.store("signal_body", &signal_id, content, Retention::Session)?;
    signal.metadata.insert("artifact_id".to_string(), json!(artifact_id));
    signal.content = format!("[Artifact: {} bytes, see artifact_id]", content.len());
}
```

### 2.6 Two-Phase Claim (from oh-my-openagent)

**What:** Claim → re-check → write pattern to prevent race conditions in action claiming.

**Where to add:** `coordination/actions.rs` (extend existing)

**Design:**

```rust
impl ActionStore {
    /// Two-phase claim: check → lock → re-check → update
    pub fn claim_action(&self, action_id: &str, agent_id: &str) -> Result<ClaimResult> {
        // Phase 1: Check if claimable
        let action = self.get_action(action_id)?;
        let action = match action {
            Some(a) => a,
            None => return Ok(ClaimResult::NotFound),
        };
        
        if action.status != ActionStatus::Pending && action.status != ActionStatus::Failed {
            return Ok(ClaimResult::NotClaimable { current_status: action.status });
        }
        
        // Check dependencies are met
        let deps = self.get_dependencies(action_id)?;
        for dep_id in deps {
            let dep = self.get_action(&dep_id)?;
            if let Some(dep) = dep {
                if dep.status != ActionStatus::Completed && dep.status != ActionStatus::Cancelled {
                    return Ok(ClaimResult::Blocked { by: dep_id });
                }
            }
        }
        
        // Phase 2: Atomic update with status check
        // Use SQLite transaction for atomicity
        let tx = self.conn.unchecked_transaction()?;
        let current_status: String = tx.query_row(
            "SELECT status FROM actions WHERE id = ?1",
            [action_id],
            |row| row.get(0),
        )?;
        
        if current_status != "pending" && current_status != "failed" {
            tx.rollback()?;
            return Ok(ClaimResult::RaceCondition { current_status });
        }
        
        tx.execute(
            "UPDATE actions SET status = 'in_progress', assigned_to = ?1, updated_at = ?2 WHERE id = ?3",
            params![agent_id, Utc::now().to_rfc3339(), action_id],
        )?;
        tx.commit()?;
        
        Ok(ClaimResult::Claimed)
    }
}

pub enum ClaimResult {
    Claimed,
    NotFound,
    NotClaimable { current_status: ActionStatus },
    Blocked { by: String },
    RaceCondition { current_status: String },
}
```

**MCP tool:** `mempalace_action_claim` — two-phase claim with conflict detection

---

## Part 3: Integration with jcode

### 3.1 Extend jcode-mempalace-adapter

**File:** `crates/jcode-mempalace-adapter/src/coordination.rs` (new)

```rust
/// Coordination adapter for jcode
pub struct CoordinationAdapter {
    signals: SignalStore,
    leases: LeaseStore,
    actions: ActionStore,
    reservations: FileReservationStore,
    event_log: CoordinationEventLog,
    live_delivery: LiveDelivery,
    saturation: SaturationDetector,
    artifacts: ArtifactStore,
}

impl CoordinationAdapter {
    pub fn open(base_path: &Path) -> Result<Self>;
    
    // Signal operations
    pub fn send_signal(&self, from: &str, to: &str, content: &str, signal_type: SignalType) -> Result<String>;
    pub fn read_signals(&self, agent_id: &str, unread_only: bool) -> Result<Vec<Signal>>;
    
    // Live delivery
    pub fn poll_delivery(&self, agent_id: &str) -> Result<Vec<PendingDelivery>>;
    pub fn ack_delivery(&self, signal_id: &str) -> Result<()>;
    
    // File reservations
    pub fn reserve_file(&self, path: &str, agent_id: &str, mode: ReservationMode) -> Result<String>;
    pub fn check_file_conflict(&self, path: &str, agent_id: &str) -> Result<ReservationConflict>;
    
    // Action claiming
    pub fn claim_action(&self, action_id: &str, agent_id: &str) -> Result<ClaimResult>;
    
    // Saturation
    pub fn check_saturation(&self) -> Result<SaturationReport>;
    
    // Event log
    pub fn replay_events(&self, from_sequence: u64) -> Result<Vec<CoordinationEvent>>;
}
```

### 3.2 Register as jcode Tools

**File:** `crates/jcode-app-core/src/tool/coordination.rs` (new)

Register 8 new tools:
1. `shared_memory_write` → `team.share()`
2. `shared_memory_read` → `signals.read_signals()` + `actions.get_action()`
3. `shared_memory_list` → `team.feed()` + `actions.list_actions()`
4. `shared_memory_delete` → custom delete with permission check
5. `shared_memory_cleanup` → `leases.cleanup()` + `reservations.cleanup()`
6. `file_reserve` → `reservations.acquire()`
7. `file_conflicts` → `reservations.check_conflict()`
8. `saturation_check` → `saturation.check_saturation()`

### 3.3 Agent Turn Loop Integration

**File:** `crates/jcode-app-core/src/agent/turn_loops.rs`

At turn start:
```rust
// 1. Poll live delivery
let pending = coordination.poll_delivery(&agent_id)?;
if !pending.is_empty() {
    let envelope = format_peer_messages(&pending);
    inject_as_system_reminder(&mut messages, &envelope);
    for msg in &pending {
        coordination.ack_delivery(&msg.signal_id)?;
    }
}

// 2. Check saturation
let saturation = coordination.check_saturation()?;
if saturation.saturated {
    inject_saturation_warning(&mut messages, &saturation);
}
```

---

## Part 4: Implementation Phases

### Phase 1: Event Sourcing (mempalace)
1. `coordination/event_log.rs` — SQLite event log
2. Integrate with existing stores (append events on mutations)
3. `replay_from()` for crash recovery
4. `compact()` for log cleanup
5. Tests

### Phase 2: File Reservations (mempalace)
6. `coordination/file_reservations.rs` — SQLite store
7. Glob pattern overlap detection
8. Exclusive/shared semantics
9. Heatmap for conflict prediction
10. MCP tools: `mempalace_file_reserve`, `mempalace_file_release`, `mempalace_file_conflicts`
11. Tests

### Phase 3: Saturation Signals (mempalace)
12. `coordination/saturation.rs` — signal detector
13. FNV-1a blocker fingerprinting
14. All 6 signal types with thresholds
15. Skill-switch recommendations
16. MCP tool: `mempalace_saturation_check`
17. Tests

### Phase 4: Live Delivery (mempalace)
18. `coordination/live_delivery.rs` — delivery system
19. Pending queue with ack tracking
20. XML envelope formatting
21. Requeue on error
22. MCP tool: `mempalace_live_deliver`
23. Tests

### Phase 5: Artifact Handoff (mempalace)
24. `coordination/artifacts.rs` — artifact store
25. Large payload detection (>2KB)
26. Retention levels
27. Integration with SignalStore
28. Tests

### Phase 6: Two-Phase Claim (mempalace)
29. Extend `ActionStore::claim_action()` with SQLite transaction
30. Dependency check before claim
31. Race condition detection
32. MCP tool: `mempalace_action_claim`
33. Tests

### Phase 7: jcode Integration
34. `jcode-mempalace-adapter/src/coordination.rs` — adapter
35. Register 8 new tools in jcode
36. Agent turn loop integration (live delivery + saturation)
37. E2E tests

---

## Part 5: Files to Create/Modify

### mempalace_rust (6 new files + 1 modified)

| File | Lines (est.) | Purpose |
|------|-------------|---------|
| `coordination/event_log.rs` | 300 | Event sourcing with SQLite |
| `coordination/file_reservations.rs` | 350 | File reservation system |
| `coordination/saturation.rs` | 400 | Saturation signal detector |
| `coordination/live_delivery.rs` | 250 | Live delivery to idle agents |
| `coordination/artifacts.rs` | 200 | Artifact handoff for large payloads |
| `coordination/mod.rs` | +6 | Register new modules |
| `coordination/actions.rs` | +80 | Two-phase claim extension |
| `mcp_server.rs` | +200 | 6 new MCP tools |
| **Total** | **~1,586** | |

### jcode (2 new files + 2 modified)

| File | Lines (est.) | Purpose |
|------|-------------|---------|
| `jcode-mempalace-adapter/src/coordination.rs` | 200 | Coordination adapter |
| `jcode-app-core/src/tool/coordination.rs` | 300 | 8 new tools |
| `jcode-app-core/src/tool/mod.rs` | +10 | Register tools |
| `jcode-app-core/src/agent/turn_loops.rs` | +30 | Turn loop integration |
| **Total** | **~540** | |

### Grand Total: ~2,126 lines

---

## Part 6: Verification

### mempalace
1. `cargo test -p mempalace-core` — all coordination tests pass
2. `cargo test -p mempalace-core --test coordination` — integration tests
3. Manual: send signal → read signal → mark read
4. Manual: acquire lease → conflict detected → release
5. Manual: create action → two-phase claim → race condition handled
6. Manual: reserve file → conflict detected → release
7. Manual: saturation check → signals detected → recommendations
8. Manual: large payload → artifact created → signal references artifact
9. Manual: event log → replay → state recovered

### jcode
10. `cargo check -p jcode-mempalace-adapter` — compiles
11. `cargo test -p jcode-app-core` — tool tests pass
12. Manual: Agent A writes shared memory → Agent B reads it
13. Manual: Live delivery → messages injected at turn boundary
14. Manual: File reservation → conflict prevented
15. Manual: Saturation warning → injected into agent context
