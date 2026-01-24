<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Per-Warp Time Sovereignty

**Status:** Draft
**Created:** 2026-01-20
**Target:** Phase 7 (Post-BOAW)
**Authors:** Claude (research agent)

## Overview

This plan defines how different WARPs can exist at different "now" positions within the same Engine step, safely and deterministically. This enables:

- **Warp A** in LIVE mode (advancing tick frontier, ingesting new intents)
- **Warp B** in REPLAY mode (replaying historical commits or applying recorded tick patches)
- **Warp C** in PAUSED mode (no-op, frozen in time)

All executing concurrently within one Engine step call.

---

## 1. Current State

### What Exists Today

| Component                | Location                   | Current Capability                                                                  |
| ------------------------ | -------------------------- | ----------------------------------------------------------------------------------- |
| **WarpState**            | `warp_state.rs:43-46`      | `BTreeMap<WarpId, GraphStore>` - per-warp isolation via separate stores             |
| **WorkUnit**             | `boaw/exec.rs:149-159`     | Carries `warp_id` explicitly - work units are warp-tagged                           |
| **execute_work_queue()** | `boaw/exec.rs:192-282`     | Resolves `GraphView` per-unit from correct store via `resolve_store(&unit.warp_id)` |
| **tick_history**         | `engine_impl.rs:424`       | `Vec<(Snapshot, TickReceipt, WarpTickPatchV1)>` - **engine-global**, not per-warp   |
| **jump_to_tick()**       | `engine_impl.rs:1581-1601` | Replays patches sequentially, but operates on **whole engine**                      |
| **WarpTickPatchV1**      | `tick_patch.rs:324-461`    | `apply_to_state()` applies canonical ops to WarpState                               |
| **Footprint isolation**  | `scheduler.rs:162-222`     | Keys include `warp_id` - cross-warp conflicts impossible by design                  |
| **Commit DAG**           | `engine_impl.rs:1052-1056` | **Single linear chain** - parents from `last_snapshot` (global)                     |

### What's Missing

1. **No `WarpRunMode`** - no enum for LIVE/REPLAY/PAUSED
2. **No per-warp timeline** - tick history is engine-global
3. **No warp-local "now"** - no tracking of each warp's position in its timeline
4. **No mode-aware scheduling** - work queue doesn't filter by mode
5. **No REPLAY intent rejection** - no mechanism to block new intents for replaying warps
6. **No per-warp commit DAG** - single chain, not per-warp branches

---

## 2. Constraints & Invariants

### Non-Negotiable (Compile-Time or Hard Runtime Errors)

| ID                  | Invariant                                                                                          | Enforcement                              |
| ------------------- | -------------------------------------------------------------------------------------------------- | ---------------------------------------- |
| **REPLAY-001**      | REPLAY warps MUST NOT ingest new intents                                                           | Runtime check at `ingest_intent()` entry |
| **REPLAY-002**      | REPLAY warps MUST only apply recorded patches (not execute rules)                                  | Mode branch in step                      |
| **REPLAY-003**      | All hashes (`commit_hash`, `patch_digest`, `state_root`) MUST match recorded history byte-for-byte | Post-apply verification                  |
| **REPLAY-004**      | REPLAY execution MUST NOT depend on wall clock, random, or nondet                                  | ADR-0006 ban list                        |
| **LIVE-001**        | LIVE warps MAY ingest new intents                                                                  | Default behavior                         |
| **LIVE-002**        | LIVE execution deterministic given ingress                                                         | Existing guarantee                       |
| **LIVE-003**        | LIVE warps MUST NOT read/write other warps' state                                                  | WarpId-scoped keys                       |
| **ISOLATION-001**   | Each warp's timeline is independent                                                                | Per-warp `tick_history`                  |
| **ISOLATION-002**   | No cross-warp `GraphView` aliasing during parallel execution                                       | Per-unit resolution                      |
| **DETERMINISM-001** | Mixed-mode execution produces deterministic per-warp commit DAGs                                   | Canonical merge                          |

### Soft Invariants (Debug Assertions, Upgradable)

| ID             | Invariant                                  | Enforcement           |
| -------------- | ------------------------------------------ | --------------------- |
| **PAUSED-001** | PAUSED warps produce zero work units       | Mode filter in build  |
| **MODE-001**   | Mode transitions are explicit and recorded | API enforcement       |
| **REPLAY-005** | REPLAY completion triggers mode transition | Configurable callback |

---

## 3. Design

### 3.1 Warp-Local "Now" Definition

"Now" is not wall-clock time but a **position in the warp's commit DAG**.

```rust
/// The temporal position of a single warp within its own timeline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WarpNow {
    /// The warp this position belongs to.
    pub warp_id: WarpId,
    /// Current tick index (0 = initial state U0, 1 = after first commit, etc.)
    pub tick_index: u64,
    /// The commit hash at this position (None for U0).
    pub commit_hash: Option<Hash>,
    /// Current execution mode.
    pub mode: WarpRunMode,
}
```

**Location**: `crates/warp-core/src/warp_timeline.rs`

For a linear chain: `(warp_id, tick_index)` uniquely identifies the state.
For future branching: `(warp_id, commit_hash)` would be the canonical form.

### 3.2 WarpRunMode Model

```rust
/// Execution mode for a warp within an Engine step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WarpRunMode {
    /// Normal operation: new intents allowed, rules execute, commits advance frontier.
    Live,

    /// Replaying recorded history: no new intents, only apply recorded patches.
    Replay {
        /// Target tick index to replay to (post-apply tick_index; patches 0..target_tick-1).
        target_tick: u64,
        /// Source of recorded patches for verification.
        source: ReplaySource,
    },

    /// No-op: warp is excluded from this step entirely.
    Paused,

    /// (Future) Forking: create a new timeline branch from current position.
    #[non_exhaustive]
    _Reserved,
}

/// Source of recorded patches for REPLAY mode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplaySource {
    /// Replay from engine's own ledger (local tick_history).
    LocalLedger,
    /// Replay from external patches (e.g., received from network peer).
    External(Vec<WarpTickPatchV1>),
}
```

**Design rationale**: Modes are **per-warp, not per-engine**. This allows warp A to advance (LIVE) while warp B replays (REPLAY) in the same `Engine.step()` call.

### 3.3 Per-Warp Timeline Structure

```rust
/// Timeline state for a single warp.
#[derive(Clone, Debug)]
pub struct WarpTimeline {
    /// Warp identifier.
    pub warp_id: WarpId,
    /// Current execution mode.
    pub mode: WarpRunMode,
    /// Complete tick history for this warp.
    pub tick_history: Vec<(Snapshot, TickReceipt, WarpTickPatchV1)>,
    /// Most recent snapshot (tip of the DAG).
    pub last_snapshot: Option<Snapshot>,
    /// Initial state for replay (U0 for this warp).
    pub initial_store: GraphStore,
    /// Current position in timeline.
    pub now: WarpNow,
}

impl WarpTimeline {
    /// Get the current tick index (0 = U0, n = after n commits).
    pub fn tick_index(&self) -> u64 {
        self.tick_history.len() as u64
    }

    /// Check if this warp can accept new intents.
    pub fn can_ingest(&self) -> bool {
        matches!(self.mode, WarpRunMode::Live)
    }

    /// Get recorded patch at index (for REPLAY verification).
    pub fn recorded_patch(&self, index: u64) -> Option<&WarpTickPatchV1> {
        self.tick_history.get(index as usize).map(|(_, _, p)| p)
    }
}
```

### 3.4 REPLAY Invariants Enforcement

```rust
impl WarpTimeline {
    /// Apply a replay step: verify and apply recorded patch.
    pub fn replay_step(
        &mut self,
        store: &mut GraphStore,
    ) -> Result<ReplayStepResult, ReplayError> {
        let WarpRunMode::Replay { target_tick, ref source } = self.mode else {
            return Err(ReplayError::NotInReplayMode);
        };

        let current_tick = self.tick_index();
        // target_tick is the desired post-apply tick_index (number of patches applied).
        // tick_index 0 = initial state; tick_index N = state after patches 0..N-1.
        // So when current_tick >= target_tick, patches 0..target_tick-1 have been applied.
        if current_tick >= target_tick {
            return Ok(ReplayStepResult::ReplayComplete);
        }

        // Get recorded patch
        let recorded = match source {
            ReplaySource::LocalLedger => {
                self.recorded_patch(current_tick)
                    .ok_or(ReplayError::MissingRecordedPatch { tick: current_tick })?
                    .clone()
            }
            ReplaySource::External(patches) => {
                patches.get(current_tick as usize)
                    .ok_or(ReplayError::MissingRecordedPatch { tick: current_tick })?
                    .clone()
            }
        };

        // Apply patch (no rule execution!)
        recorded.apply_to_store(store)?;

        // Verify post-state matches recorded (REPLAY-003)
        let post_state_root = compute_state_root_for_warp(store, &self.warp_id);
        let (recorded_snapshot, _, _) = &self.tick_history[current_tick as usize];

        if post_state_root != recorded_snapshot.state_root {
            return Err(ReplayError::StateRootMismatch {
                tick: current_tick,
                expected: recorded_snapshot.state_root,
                actual: post_state_root,
            });
        }

        // Advance timeline position
        self.now.tick_index = current_tick + 1;
        self.now.commit_hash = Some(recorded_snapshot.hash);

        Ok(ReplayStepResult::Advanced { tick: current_tick + 1 })
    }
}

#[derive(Debug)]
pub enum ReplayError {
    NotInReplayMode,
    MissingRecordedPatch { tick: u64 },
    StateRootMismatch { tick: u64, expected: Hash, actual: Hash },
    PatchDigestMismatch { tick: u64, expected: Hash, actual: Hash },
    CommitHashMismatch { tick: u64, expected: Hash, actual: Hash },
}
```

### 3.5 LIVE Invariants Enforcement

```rust
impl Engine {
    /// Ingest intent with mode check (LIVE-001 enforced).
    pub fn ingest_intent_for_warp(
        &mut self,
        warp_id: &WarpId,
        intent_bytes: &[u8],
    ) -> Result<IngestDisposition, EngineError> {
        let timeline = self.timelines.get(warp_id)
            .ok_or(EngineError::UnknownWarp(*warp_id))?;

        // REPLAY-001: Reject intents for non-LIVE warps
        if !timeline.can_ingest() {
            return Err(EngineError::WarpNotAcceptingIntents {
                warp_id: *warp_id,
                mode: timeline.mode.clone(),
            });
        }

        // Proceed with normal ingestion
        self.ingest_intent_impl(warp_id, intent_bytes)
    }
}
```

### 3.6 Concurrency Safety Matrix

| Data                    | Sharing Model    | Rationale                                     |
| ----------------------- | ---------------- | --------------------------------------------- |
| `GraphStore` per warp   | **Isolated**     | Each warp has own store in `WarpState.stores` |
| `WarpTimeline` per warp | **Isolated**     | Each warp has own timeline, mode, history     |
| `WorkUnit`              | **Read-shared**  | Built before execution, immutable during      |
| `TickDelta` per worker  | **Thread-local** | Each worker accumulates own delta             |
| `RewriteRule` registry  | **Read-shared**  | Rules immutable after registration            |
| Atomic work counter     | **Shared**       | `AtomicUsize` for work-stealing               |
| Engine metadata         | **Isolated**     | Only one `&mut Engine` exists                 |

**Key guarantee**: During `execute_work_queue()`, each worker:

1. Claims a `WorkUnit` atomically
2. Resolves `GraphView` from correct warp's store (read-only)
3. Writes to thread-local `TickDelta`
4. Never touches another warp's store

### 3.7 Global Work Queue with Mixed Modes

```rust
/// Build work units respecting per-warp modes.
pub fn build_mixed_mode_work_units(
    timelines: &BTreeMap<WarpId, WarpTimeline>,
    live_rewrites: &BTreeMap<WarpId, Vec<(PendingRewrite, ExecuteFn)>>,
) -> MixedModeWorkPlan {
    let mut live_units = Vec::new();
    let mut replay_warps = Vec::new();
    let mut paused_warps = Vec::new();

    for (warp_id, timeline) in timelines {
        match &timeline.mode {
            WarpRunMode::Live => {
                // Build work units for LIVE warps (normal path)
                if let Some(rewrites) = live_rewrites.get(warp_id) {
                    let items: Vec<ExecItem> = rewrites.iter()
                        .map(|(rw, exec)| ExecItem {
                            exec: *exec,
                            scope: rw.scope.local_id,
                            origin: OpOrigin::default(),
                        })
                        .collect();
                    let sharded = partition_into_shards(&items);
                    for shard in sharded {
                        if !shard.items.is_empty() {
                            live_units.push(WorkUnit {
                                warp_id: *warp_id,
                                items: shard.items,
                            });
                        }
                    }
                }
            }
            WarpRunMode::Replay { .. } => {
                replay_warps.push(*warp_id);
            }
            WarpRunMode::Paused => {
                paused_warps.push(*warp_id);
            }
            WarpRunMode::_Reserved => unreachable!(),
        }
    }

    MixedModeWorkPlan {
        live_units,
        replay_warps,
        paused_warps,
    }
}

pub struct MixedModeWorkPlan {
    /// Work units for LIVE warps (rule execution).
    pub live_units: Vec<WorkUnit>,
    /// Warps in REPLAY mode (will apply recorded patches).
    pub replay_warps: Vec<WarpId>,
    /// Warps in PAUSED mode (no-op).
    pub paused_warps: Vec<WarpId>,
}
```

**Key insight**: REPLAY warps don't produce `ExecItem` work units because they don't execute rules - they apply pre-recorded patches. PAUSED warps produce nothing. Only LIVE warps generate rule execution work.

### 3.8 Preventing Cross-Mode Contamination

```rust
impl Engine {
    pub fn step_mixed_mode(&mut self) -> Result<MixedModeStepResult, EngineError> {
        // 1. Build mode-aware work plan
        let plan = build_mixed_mode_work_units(&self.timelines, &self.pending_by_warp);

        // 2. Execute LIVE warps (parallel rule execution)
        let live_deltas = if !plan.live_units.is_empty() {
            execute_work_queue(&plan.live_units, self.worker_count, |warp_id| {
                self.state.store(warp_id)
            })
        } else {
            Vec::new()
        };

        // 3. Merge and commit LIVE deltas (per-warp)
        let mut live_results = BTreeMap::new();
        for warp_id in plan.live_units.iter().map(|u| u.warp_id).collect::<BTreeSet<_>>() {
            let warp_delta = self.extract_warp_delta(&live_deltas, &warp_id);
            let result = self.commit_warp(&warp_id, warp_delta)?;
            live_results.insert(warp_id, result);
        }

        // 4. Execute REPLAY warps (apply recorded patches, verify hashes)
        let mut replay_results = BTreeMap::new();
        for warp_id in &plan.replay_warps {
            let timeline = self.timelines.get_mut(warp_id).unwrap();
            let store = self.state.store_mut(warp_id).unwrap();
            let result = timeline.replay_step(store)?;
            replay_results.insert(*warp_id, result);
        }

        // 5. PAUSED warps: no-op

        Ok(MixedModeStepResult {
            live_results,
            replay_results,
            paused: plan.paused_warps,
        })
    }
}
```

### 3.9 Per-Warp Commit with Isolated Timeline

```rust
impl Engine {
    fn commit_warp(
        &mut self,
        warp_id: &WarpId,
        delta: TickDelta,
    ) -> Result<WarpCommitResult, EngineError> {
        let timeline = self.timelines.get_mut(warp_id)
            .ok_or(EngineError::UnknownWarp(*warp_id))?;
        let store = self.state.store_mut(warp_id)
            .ok_or(EngineError::UnknownWarp(*warp_id))?;

        // Build patch from delta
        let patch = WarpTickPatchV1::from_delta(delta, self.policy_id)?;

        // Apply patch
        patch.apply_to_store(store)?;

        // Compute hashes
        let state_root = compute_state_root_for_warp(store, warp_id);
        let patch_digest = patch.digest();
        let parents = timeline.last_snapshot
            .as_ref()
            .map(|s| vec![s.hash])
            .unwrap_or_default();
        let commit_hash = compute_commit_hash_v2(
            state_root,
            &parents,
            patch_digest,
            self.policy_id,
        );

        // Build snapshot
        let snapshot = Snapshot {
            warp_id: *warp_id,
            hash: commit_hash,
            state_root,
            patch_digest,
            parents,
            // ... other fields
        };

        // Record in warp's timeline (NOT global!)
        let receipt = self.build_receipt_for_warp(warp_id)?;
        timeline.tick_history.push((snapshot.clone(), receipt, patch));
        timeline.last_snapshot = Some(snapshot.clone());
        timeline.now.tick_index += 1;
        timeline.now.commit_hash = Some(commit_hash);

        Ok(WarpCommitResult {
            snapshot,
            tick_index: timeline.now.tick_index,
        })
    }
}
```

---

## 4. Implementation Plan

### Phase 1: Core Types (1 commit)

**Files to create/modify:**

| Action     | File                                    | Changes                                                                 |
| ---------- | --------------------------------------- | ----------------------------------------------------------------------- |
| **NEW**    | `crates/warp-core/src/warp_timeline.rs` | `WarpNow`, `WarpRunMode`, `ReplaySource`, `WarpTimeline`, `ReplayError` |
| **MODIFY** | `crates/warp-core/src/lib.rs`           | Export new module                                                       |

**Tests**: Unit tests for `WarpRunMode` transitions, `WarpTimeline` basic ops.

### Phase 2: Per-Warp Timeline Storage (1 commit)

**Files to modify:**

| Action     | File                                  | Changes                                                                                   |
| ---------- | ------------------------------------- | ----------------------------------------------------------------------------------------- |
| **MODIFY** | `crates/warp-core/src/engine_impl.rs` | Add `timelines: BTreeMap<WarpId, WarpTimeline>` field; migrate `tick_history` to per-warp |
| **MODIFY** | `crates/warp-core/src/warp_state.rs`  | Add `timeline()` accessor                                                                 |

**Tests**: Verify existing tests pass with new storage layout.

### Phase 3: Mode-Aware Intent Ingestion (1 commit)

**Files to modify:**

| Action     | File                                  | Changes                                                                                        |
| ---------- | ------------------------------------- | ---------------------------------------------------------------------------------------------- |
| **MODIFY** | `crates/warp-core/src/engine_impl.rs` | Check `timeline.can_ingest()` in `ingest_intent()`; add `EngineError::WarpNotAcceptingIntents` |

**Tests**:

- `test_replay_warp_rejects_new_intents`
- `test_paused_warp_rejects_new_intents`
- `test_live_warp_accepts_intents`

### Phase 4: Mode-Aware Work Queue (1 commit)

**Files to modify:**

| Action     | File                                  | Changes                                                  |
| ---------- | ------------------------------------- | -------------------------------------------------------- |
| **MODIFY** | `crates/warp-core/src/boaw/exec.rs`   | Add `MixedModeWorkPlan`, `build_mixed_mode_work_units()` |
| **MODIFY** | `crates/warp-core/src/engine_impl.rs` | Implement `step_mixed_mode()`                            |

**Tests**:

- `test_live_warp_generates_work_units`
- `test_replay_warp_no_work_units`
- `test_paused_warp_no_work_units`

### Phase 5: REPLAY Patch Application (1 commit)

**Files to modify:**

| Action     | File                                    | Changes                                                    |
| ---------- | --------------------------------------- | ---------------------------------------------------------- |
| **MODIFY** | `crates/warp-core/src/warp_timeline.rs` | Implement `WarpTimeline::replay_step()`, hash verification |
| **MODIFY** | `crates/warp-core/src/tick_patch.rs`    | Add `apply_to_store()` (single-warp variant)               |

**Tests**:

- `test_replay_applies_recorded_patches`
- `test_replay_detects_state_root_mismatch`
- `test_replay_detects_commit_hash_mismatch`

### Phase 6: Per-Warp Commit (1 commit)

**Files to modify:**

| Action     | File                                  | Changes                                               |
| ---------- | ------------------------------------- | ----------------------------------------------------- |
| **MODIFY** | `crates/warp-core/src/engine_impl.rs` | Implement `commit_warp()`, per-warp snapshot creation |
| **MODIFY** | `crates/warp-core/src/snapshot.rs`    | Add `warp_id` to `Snapshot` (or make warp-scoped)     |

**Tests**:

- `test_commit_advances_warp_timeline`
- `test_commit_hash_deterministic_per_warp`

### Phase 7: Engine API Surface (1 commit)

**Files to modify:**

| Action     | File                                  | Changes                                                   |
| ---------- | ------------------------------------- | --------------------------------------------------------- |
| **MODIFY** | `crates/warp-core/src/engine_impl.rs` | Add `set_warp_mode()`, `get_warp_now()`, `start_replay()` |

**Tests**:

- `test_set_warp_mode_live_to_replay`
- `test_set_warp_mode_replay_to_paused`
- `test_start_replay_from_tick_zero`

### Phase 8: Integration Tests (1 commit)

**Files to create:**

| Action  | File                                              | Changes                     |
| ------- | ------------------------------------------------- | --------------------------- |
| **NEW** | `crates/warp-core/tests/warp_time_sovereignty.rs` | Full integration test suite |

---

## 5. Test Plan

### File: `crates/warp-core/tests/warp_time_sovereignty.rs`

| Test ID | Name                                        | Description                                                                              |
| ------- | ------------------------------------------- | ---------------------------------------------------------------------------------------- |
| **T1**  | `test_live_and_replay_concurrent_isolation` | LIVE warp advances while REPLAY warp replays; neither affects the other                  |
| **T2**  | `test_replay_hash_chain_identity`           | REPLAY produces identical `commit_hash` chains to recorded history (100 ticks, 10 seeds) |
| **T3**  | `test_live_worker_invariance_with_replay`   | LIVE worker-count invariance holds during concurrent REPLAY                              |
| **T4**  | `test_replay_rejects_intents`               | REPLAY warp returns `Err(WarpNotAcceptingIntents)` on intent ingestion                   |
| **T5**  | `test_mixed_mode_work_queue_determinism`    | Mixed-mode execution deterministic across 50 shuffled ingress orderings                  |
| **T6**  | `test_replay_tripwire_nondet_injection`     | **Tripwire**: fails if any nondet input leaks into REPLAY mode                           |
| **T7**  | `test_replay_completion_mode_transition`    | REPLAY completion transitions mode to PAUSED (or LIVE if configured)                     |
| **T8**  | `test_multiple_replay_warps_isolation`      | Multiple REPLAY warps don't interfere with each other                                    |
| **T9**  | `test_cross_mode_commit_dag_independence`   | LIVE and REPLAY warps have completely independent commit DAGs                            |
| **T10** | `test_paused_warp_state_immutable`          | PAUSED warp state completely unchanged across 10 engine steps                            |

### Test Implementation Details

```rust
/// T1: LIVE warp advances while REPLAY warp replays - mutual isolation.
#[test]
fn test_live_and_replay_concurrent_isolation() {
    // Setup: Engine with warp_a (LIVE) and warp_b (REPLAY)
    // 1. Record 10 ticks for warp_b
    // 2. Rewind warp_b to tick 0, set REPLAY mode targeting tick 5
    // 3. Set warp_a to LIVE
    // 4. Execute 5 mixed-mode steps
    // Assert: warp_a advanced 5 ticks, warp_b replayed to tick 5
    // Assert: warp_a's commit hashes are new, warp_b's match recorded history
    // Assert: Neither warp's state was corrupted by the other
}

/// T6: Tripwire - nondet leak into REPLAY fails.
#[test]
fn test_replay_tripwire_nondet_injection() {
    // Setup: Custom rule that attempts to inject nondet (e.g., thread::current().id())
    // Record with clean rule, then replay
    // Assert: If any nondet leaks, state_root mismatch detected
    // This test FAILS if nondet enters replay path - that's the tripwire
}
```

### Additional Test Files

| File                                           | Purpose                                                       |
| ---------------------------------------------- | ------------------------------------------------------------- |
| `crates/warp-core/tests/replay_determinism.rs` | Permutation tests (100+ seeds), cross-platform verification   |
| `crates/warp-core/tests/mode_transitions.rs`   | Valid/invalid mode transitions, transition during active step |

---

## 6. Engine API Surface

### New Methods

```rust
impl Engine {
    /// Set the execution mode for a warp.
    ///
    /// # Errors
    /// - `UnknownWarp` if warp_id not found
    /// - `InvalidModeTransition` if transition not allowed (e.g., during active step)
    pub fn set_warp_mode(
        &mut self,
        warp_id: WarpId,
        mode: WarpRunMode
    ) -> Result<(), EngineError>;

    /// Get the current temporal position of a warp.
    pub fn get_warp_now(&self, warp_id: &WarpId) -> Option<&WarpNow>;

    /// Start replay for a warp from its current position to target_tick.
    ///
    /// This is a convenience wrapper that:
    /// 1. Validates target_tick is in recorded history
    /// 2. Sets mode to Replay { target_tick, source: LocalLedger }
    pub fn start_replay(
        &mut self,
        warp_id: WarpId,
        target_tick: u64
    ) -> Result<(), EngineError>;

    /// Start replay from external patches (e.g., received from network).
    pub fn start_replay_external(
        &mut self,
        warp_id: WarpId,
        patches: Vec<WarpTickPatchV1>,
    ) -> Result<(), EngineError>;

    /// Execute one engine step with mixed-mode support.
    pub fn step_mixed_mode(&mut self) -> Result<MixedModeStepResult, EngineError>;

    /// Get all warps and their current modes.
    pub fn warp_modes(&self) -> impl Iterator<Item = (&WarpId, &WarpRunMode)>;
}
```

### Result Types

```rust
pub struct MixedModeStepResult {
    /// Results for warps that were in LIVE mode.
    pub live_results: BTreeMap<WarpId, WarpCommitResult>,
    /// Results for warps that were in REPLAY mode.
    pub replay_results: BTreeMap<WarpId, ReplayStepResult>,
    /// Warps that were PAUSED (no-op).
    pub paused: Vec<WarpId>,
}

pub struct WarpCommitResult {
    pub snapshot: Snapshot,
    pub tick_index: u64,
}

pub enum ReplayStepResult {
    /// Advanced to the next tick.
    Advanced { tick: u64 },
    /// Reached target_tick, replay complete.
    ReplayComplete,
}
```

---

## 7. Scheduling Rules

### Which Warps Run

1. **LIVE warps**: Generate work units if they have pending rewrites
2. **REPLAY warps**: Apply one recorded patch per step (no work units)
3. **PAUSED warps**: Skipped entirely (no state change)

### Mode Determination Per Step

```text
For each warp in canonical order (BTreeMap iteration):
  match warp.mode:
    Live →
      if has_pending_rewrites(warp):
        generate WorkUnits for this warp
      else:
        no-op this step

    Replay { target_tick, source } →
      if warp.tick_index < target_tick:
        schedule replay_step for this warp
      else:
        replay complete, transition to PAUSED (or callback)

    Paused →
      no-op
```

### Work Queue Execution Order

1. All LIVE work units execute in parallel (existing `execute_work_queue`)
2. LIVE deltas merged and committed per-warp
3. REPLAY warps apply patches sequentially (per-warp, can be parallelized across warps)
4. PAUSED warps skipped

---

## 8. Time Travel / Rewind / Replay Selection

### Required Inputs for REPLAY

| Input              | Source                                            | Required |
| ------------------ | ------------------------------------------------- | -------- |
| `warp_id`          | Caller                                            | Yes      |
| `target_tick`      | Caller                                            | Yes      |
| `ReplaySource`     | Caller chooses                                    | Yes      |
| Recorded patches   | `LocalLedger` or `External(Vec<WarpTickPatchV1>)` | Yes      |
| Initial state (U0) | Stored in `WarpTimeline.initial_store`            | Auto     |

### Rewind Mechanism

```rust
impl WarpTimeline {
    /// Rewind warp to tick 0 (U0 state) for replay.
    pub fn rewind_to_origin(&mut self, store: &mut GraphStore) {
        // Clone initial state back to active store
        *store = self.initial_store.clone();
        self.now.tick_index = 0;
        self.now.commit_hash = None;
        // Note: tick_history preserved for replay source
    }

    /// Rewind to specific tick (requires re-applying patches 0..tick).
    pub fn rewind_to_tick(
        &mut self,
        store: &mut GraphStore,
        tick: u64
    ) -> Result<(), ReplayError> {
        self.rewind_to_origin(store);
        for i in 0..tick {
            let patch = self.recorded_patch(i)
                .ok_or(ReplayError::MissingRecordedPatch { tick: i })?;
            patch.apply_to_store(store)?;
        }
        self.now.tick_index = tick;
        self.now.commit_hash = if tick == 0 {
            None
        } else {
            self.tick_history.get(tick as usize - 1)
                .map(|(s, _, _)| s.hash)
        };
        Ok(())
    }
}
```

---

## 9. Risks & Mitigations

| Risk                                                  | Severity     | Mitigation                                                                            |
| ----------------------------------------------------- | ------------ | ------------------------------------------------------------------------------------- |
| **Per-warp timeline storage increases memory**        | Medium       | Use structural sharing for `GraphStore` snapshots; only store deltas                  |
| **REPLAY hash mismatch debugging is hard**            | Medium       | Include tick index, expected vs actual hashes, and delta dump in `ReplayError`        |
| **Mode transition race conditions**                   | Low          | Mode changes only allowed between steps; enforce via `&mut Engine`                    |
| **Future forking complicates commit DAG**             | Low (future) | Design `parents: Vec<Hash>` now; collapse/merge is separate feature                   |
| **Cross-warp portal operations during mixed modes**   | Medium       | Portal creation in LIVE warp that targets REPLAY warp must be blocked; add validation |
| **Global `policy_id` shared across warps**            | Low          | Acceptable for now; future per-warp policy is out of scope                            |
| **REPLAY from external source has no chain of trust** | Medium       | External `ReplaySource` should require signature verification (future enhancement)    |

---

## 10. Out of Scope (Future Work)

1. **Per-warp forking/branching** - Commit DAG supports multiple parents but implementation deferred
2. **Collapse/merge across warps** - ADR-0007 Layer 7 specified but not implemented here
3. **Privacy mode per-warp** - Mind vs Diagnostics modes are engine-global currently
4. **Network-sourced REPLAY verification** - Signature verification for `ReplaySource::External`
5. **Per-warp policy_id** - All warps share engine's `policy_id`

---

## 11. Summary

Per-warp time sovereignty is achievable with **minimal, composable changes** because the existing architecture already enforces per-warp isolation via:

- `WarpId`-scoped keys in footprints
- Per-unit `GraphView` resolution in `execute_work_queue()`
- Separate `GraphStore` per warp in `WarpState`

The main additions are:

1. **WarpRunMode enum** - explicit mode tracking per warp
2. **Per-warp timeline storage** - migrate global `tick_history` to per-warp
3. **Mode-aware scheduling** - filter work queue by mode
4. **REPLAY enforcement** - apply recorded patches instead of executing rules

The design **preserves all existing determinism guarantees** and **does not block future forking/collapse features**.

---

## Appendix A: File Change Summary

| File                                              | Action  | LOC Estimate |
| ------------------------------------------------- | ------- | ------------ |
| `crates/warp-core/src/warp_timeline.rs`           | **NEW** | ~300         |
| `crates/warp-core/src/lib.rs`                     | MODIFY  | +5           |
| `crates/warp-core/src/engine_impl.rs`             | MODIFY  | +200         |
| `crates/warp-core/src/boaw/exec.rs`               | MODIFY  | +50          |
| `crates/warp-core/src/tick_patch.rs`              | MODIFY  | +30          |
| `crates/warp-core/src/snapshot.rs`                | MODIFY  | +10          |
| `crates/warp-core/src/warp_state.rs`              | MODIFY  | +20          |
| `crates/warp-core/tests/warp_time_sovereignty.rs` | **NEW** | ~400         |
| `crates/warp-core/tests/replay_determinism.rs`    | **NEW** | ~200         |
| `crates/warp-core/tests/mode_transitions.rs`      | **NEW** | ~150         |
| **Total**                                         |         | ~1365        |

---

## Appendix B: Compile-Time vs Runtime Enforcement

| Invariant                   | Enforcement      | Mechanism                                   |
| --------------------------- | ---------------- | ------------------------------------------- |
| REPLAY-001 (no intents)     | **Runtime**      | Check in `ingest_intent()`                  |
| REPLAY-002 (patches only)   | **Runtime**      | Mode branch in `step_mixed_mode()`          |
| REPLAY-003 (hash match)     | **Runtime**      | Post-apply verification                     |
| REPLAY-004 (no nondet)      | **Compile-time** | ADR-0006 ban list + `ban-nondeterminism.sh` |
| LIVE-003 (no cross-warp)    | **Compile-time** | `WarpId` in all key types                   |
| ISOLATION-002 (no aliasing) | **Runtime**      | Per-unit `GraphView` resolution             |
| DETERMINISM-001             | **Runtime**      | Canonical merge in `merge_deltas()`         |
