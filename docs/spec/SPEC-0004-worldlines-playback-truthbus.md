<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# SPEC: Worldlines, PlaybackCursors, ViewSessions, and TruthBus

**Status:** Draft — Approved-for-Implementation Target  
**Created:** 2026-01-20  
**Target:** Phase 7 foundation (forking-adjacent), compatible with later collapse/privacy  
**Primary Outcomes:** “Runs normally” + “Debug feels like a debugger” + “Clients are dumb”

---

## 0) Doctrine

- **Worldline is the boundary.** (U0 + per-warp tick patches + expected hashes + recorded outputs)
- **PlaybackCursor is a viewpoint.** (materialize any tick without mutating head)
- **Clients are dumb.** They render authoritative truth frames; no rollback/diff/rebuild logic.
- **Determinism is non-negotiable.** Canonical ordering → canonical bytes → canonical hashes.
- **Global V1 + Local W1:** The global `WarpTickPatchV1` log is preserved for engine-wide determinism/replay. `WorldlineTickPatchV1` is a projected derivative for per-warp cursors and does not replace the global ledger.

---

## 1) Core Concepts

### 1.1 Worldline

A `Worldline` is an append-only boundary encoding sufficient to reconstruct (a) interior state and (b) client-visible truth.

#### WorldlineId

- Stable identity for a worldline (hash or opaque id; `WorldlineId(Hash)` wrapper for clarity).
- **Default**: one worldline per warp (for now), but forkable.

#### Durable boundary

- `U0Ref` (initial checkpoint handle)
- `patch[t] = WorldlineTickPatchV1` (from A: renamed for consistency)
- `expected[t] = HashTriplet { state_root, patch_digest, commit_hash }`
- `outputs[t] = OutputFrameSet` (recorded truth frames per tick) — MVP requires recording

#### Invariants

- **WL-001 (Holography):** Given `U0Ref`, patches, and canonical apply, any tick’s state is reconstructible.
- **WL-002 (Truth):** Given recorded outputs per tick, any tick’s client-visible truth is reconstructible byte-for-byte.

---

### 1.2 PlaybackCursor

A `PlaybackCursor` is an ephemeral viewpoint materialized at a tick of a worldline.

#### Fields

- `cursor_id: CursorId` (`CursorId(Hash)` wrapper)
- `worldline_id: WorldlineId`
- `warp_id: WarpId` (for routing/keys)
- `tick: u64`
- `role: CursorRole` (`Writer` | `Reader`) (added for explicit distinction)
- `mode: PlaybackMode`
- `store: GraphStore` (materialization; never shared with other cursors)
- `pin_max_tick: u64` (stable bound for future live-follow; MVP = `worldline_len-1`)

#### Cursor Invariants

- **CUR-001:** Cursor never mutates worldline unless role is `Writer` and mode requires advance.
- **CUR-002:** Cursor never executes rules when seeking; it applies recorded patches only.
- **CUR-003:** After seek/apply, cursor verifies expected hashes byte-for-byte.

---

### 1.3 PlaybackMode

`PlaybackMode` defines what happens when the engine “steps” the cursor.

```rust
enum PlaybackMode {
  Paused,
  Play,               // Writer: may append; Reader: consumes existing history then stops at frontier
  StepForward,        // advance one tick then Paused
  StepBack,           // seek tick-1 then Paused
  Seek { target: u64, then: SeekThen },
}

enum SeekThen {
  Pause,
  RestorePrevious,
  Play,
}
```

Semantics:

- `Seek` is a one-shot command.
- `StepBack` is seek; no inverse execution is assumed.
- `Reader`+`Play` consumes existing worldline history; at frontier it transitions to `Paused`.
    - e.g. When `Reader` is in `Play` and reaches `tick == pin_max_tick`, it transitions to `Paused`.

**CUR-PLAY-001:** `Reader` in `Play` must not advance beyond `pin_max_tick`; on reaching it, transition to `Paused`.

---

### 1.4 ViewSession

A `ViewSession` couples:

- `session_id`: `SessionId` (`SessionId(Hash)` wrapper)
- `active_cursor`: `CursorId`
- `subscriptions`: `Set<ChannelId>`

Clients interact with sessions, not the global bus.

### ViewSession Invariants

#### VS-001

- Session cursor switch is opaque to subscribers (no resubscribe required).
- On `session_set_active_cursor`, the system **must enqueue** truth frames for all subscribed channels for the new cursor tick (so the UI snaps immediately).

#### VS-002

Sessions isolate views (two sessions can view different ticks concurrently).

---

### 1.5 TruthBus

`TruthBus` delivers authoritative values for `(session, cursor, channel)`.

#### MBUS v2 Payload

**Header**: "`MBUS`", version `0x0002`, reserved, `payload_len (u32)`

**Payload**:

- `session_id[32]`
- `cursor_id[32]`
- `worldline_id[32]`
- `warp_id[32]`
- `tick[u64 LE]`
- `commit_hash[32]`
- `entry_count[u32 LE]`
- entries:
    - `channel_id[32]`
    - `value_hash[32] = blake3(value)`
    - `value_len[u32 LE]`
    - `value[value_len]`

Packets are concatenated like v1 frames. MBUS v1 remains untouched.

#### Frames

```rust
struct CursorReceipt {
  session_id: SessionId,
  cursor_id: CursorId,
  worldline_id: WorldlineId,
  warp_id: WarpId,
  tick: u64,
  commit_hash: Hash,
}

struct TruthFrame {
  cursor: CursorReceipt,
  channel: ChannelId,
  value: Vec<u8>,     // canonical bytes
  value_hash: Hash,   // blake3(value)
}
```

##### Contract

- Frames are authoritative state for that cursor.
- Clients replace render state; no client rollback, diffs, or replay.
- Every `TruthFrame` must include `CursorReceipt`.

##### Output Invariants

- **OUT-001**: For `(worldline_id, tick, channel)`, value bytes are deterministic and identical across runs/machines.
- **OUT-002**: Playback at tick t reproduces the same `TruthFrames` recorded at tick t.

---

## 2) ProvenanceStore Seam (Wormholes-Ready)

Worldline data is accessed through an interface (local memory today, wormholes later).

```rust
trait ProvenanceStore {
  fn u0(&self, w: WorldlineId) -> Result<U0Ref, HistoryError>;
  fn len(&self, w: WorldlineId) -> Result<u64, HistoryError>;

  fn patch(&self, w: WorldlineId, tick: u64) -> Result<WorldlineTickPatchV1, HistoryError>;
  fn expected(&self, w: WorldlineId, tick: u64) -> Result<HashTriplet, HistoryError>;

  // MVP: recorded truth frames per tick (channel->bytes)
  fn outputs(&self, w: WorldlineId, tick: u64) -> Result<Vec<(ChannelId, Vec<u8>)>, HistoryError>;

  // optional: checkpoints
  fn checkpoint_before(&self, w: WorldlineId, tick: u64) -> Option<CheckpointRef>;
}
```

### HistoryError

- `HistoryUnavailable { tick }` must be deterministic (no retries unless future fetch layer is explicitly implemented).

---

## 3) Engine Step Model

One Engine step is phased:

### Phase A: Plan

- Identify writer cursors that advance (`Play`/`StepForward`).
- Identify cursors that seek (`Seek`/`StepBack`).
- Build global work queue for advancing writers (warp+shard units).

### Phase B: Execute (parallel BOAW)

- Workers claim `WorkUnits`.
- Resolve `GraphView` for writer cursor’s store (read-only).
- Emit `TickDeltas` thread-locally.

### Phase C: Commit

- Commit (global):
    - merge deltas to ops
    - apply to engine state
    - produce global snapshot + global patch V1
    - project ops into per-warp worldline tick patches + expected hashes + outputs

### Phase D: Seek/Playback (cursor materialization)

- For each cursor seek: apply recorded patches (no rules).
- Verify expected hashes.
- Apply `SeekThen` mode transition.

### Phase E: Publish Truth (per session)

- Publish `CursorReceipt`.
- Publish `TruthFrames` for subscribed channels using `ProvenanceStore.outputs(worldline, tick)`.

### Phase F: Mode transitions

- `StepForward`/`StepBack` always end `Paused`.
- `Seek` ends according to `SeekThen`.

### Barrier Invariant

- **STEP-001**: No store mutation while any `GraphView` borrow exists for that store.
- **STEP-002**: Seeking never touches writer cursor store; only cursor.store.

---

## 4) Outputs and Reducers (Authoritative Values)

Rule **OUT-REDUCE-001**: For reduced channels, per-tick outputs are the final reduced value, not raw emission events.

Rule **OUT-REDUCE-002**: Order-dependent reducers must rely on canonical ordering (EmitKey/OpOrigin) so output frames are deterministic.

Rule **OUT-REDUCE-003**: Recorded frames per tick must be byte-identical upon playback.

### Source of Outputs

- At commit, source from `mat_report = bus.finalize()` (existing).
- **MVP**: Treat outputs as engine/global and store under the root warp’s worldline first; later upgrade to warp-scoped outputs.
- **OUT-SCOPE-001 (MVP):** outputs recorded at engine tick scope; associated with root warp worldline.
- **OUT-SCOPE-002 (Upgrade):** outputs recorded per `(warp_id, tick)` once emissions become warp-scoped.

### Output Record

```rust
type OutputFrameSet = Vec<(ChannelId, Vec<u8>)>; // finalized bytes only`
```

---

## 5) RetentionPolicy (Checkpoints / Wormholes)

```rust
enum RetentionPolicy {
  KeepAll,
  CheckpointEvery { k: u64 },
  KeepRecent { window: u64, checkpoint_every: u64 },
  ArchiveToWormhole { after: u64, checkpoint_every: u64 }, // seam only (future)
}
```

**RET-001**: Retention cannot break verification. If patches are archived, checkpoints + commitments must remain sufficient to verify ticks ≥ checkpoint.

**MVP**: implement `CheckpointEvery { k }` locally.

---

## 6) Forking

Forking creates a new worldline from `(worldline_id, tick)`.

**MVP**: prefix-copy patches/expected/outputs up to tick.

**Future**: checkpoint-at-fork optimization.

---

## 7) Required Public APIs

### Engine / runtime

- `create_worldline_for_warp(warp_id) -> WorldlineId` (or implicit)
- `create_cursor(worldline_id, start_tick, role) -> CursorId`
- `drop_cursor(cursor_id)`
- `set_cursor_mode(cursor_id, PlaybackMode)`
- `create_session(active_cursor) -> SessionId`
- `session_subscribe(session_id, channel_id)`
- `session_set_active_cursor(session_id, cursor_id)`
- `engine_step() -> StepReport` (includes per-session publish results)

### Cursor

- `seek_to(target_tick)` (using `ProvenanceStore`)
- `advance_one_tick()` (writer-only; uses BOAW pipeline)

### TruthBus

- `publish_cursor_receipt(session_id, receipt)`
- `publish_truth_frame(session_id, TruthFrame)`

---

## 8) Acceptance Test Suite

All tests live under crates/warp-core/tests/ unless noted.

### Test harness requirements (common)

Provide a deterministic harness with:

- fixed PRNG (no rand)
- ability to:
    - create worldline + writer cursor
    - run N steps to generate provenance+outputs
    - create reader cursor and seek
    - create sessions and collect published `TruthFrames`

### Common helpers (required)

- `run_steps(writer_cursor, n, workers)`
- `seek(cursor, tick)`
- `collect_frames(session_id) -> Vec<TruthFrame>`
- `last_receipt(session_id) -> CursorReceipt`

#### A) Normal run (“it runs normally”)

##### T1: `writer_play_advances_and_records_outputs`

- **Arrange:** writer cursor at tick 0, mode=`Play`, channel sim/x defined (x increments each tick)
- **Act:** run 10 engine steps
- **Assert:**
    - `worldline.len == 10`
    - expected hashes exist for 1..10
    - `outputs[t]` contains `(sim/x -> bytes_of(t))` for each tick t (or correct encoding)

##### T2: `step_forward_advances_one_then_pauses`

- **Arrange:** writer cursor mode=`StepForward`
- **Act:** `engine_step()`
- **Assert:** tick += 1, mode == `Paused`

##### T3: paused_noop_even_with_pending_intents

- **Arrange:** writer cursor mode=`Paused`, but ingress queue has intents
- **Act:** `engine_step()`
- **Assert:** tick unchanged, no new patches, no outputs appended

#### B) Debug semantics (“debug feels like a debugger”)

##### T4: `seek_moves_cursor_without_mutating_writer_store`

- **Arrange:** run writer to tick 20; create reader cursor at tick 20; snapshot writer `state_root`
- **Act:** reader `Seek(target=5, then=Pause)` and `engine_step()`
- **Assert:**
    - writer `state_root` unchanged
    - reader tick == 5
    - reader store hashes match `expected[5]`

##### T5: `step_back_is_seek_minus_one_then_pause`

- **Arrange:** reader cursor at tick 10, mode=`StepBack`
- **Act:** `engine_step()`
- **Assert:** tick == 9, mode == `Paused`

##### T6: `reader_play_consumes_existing_then_pauses_at_frontier`

- **Arrange:** worldline length 7; reader cursor at tick 0, mode=`Play`
- **Act:** step until tick reaches 7
- **Assert:** at tick 7 mode transitions to `Paused`; no new patches appended

#### C) TruthBus correctness (the “gap” is closed)

##### T7: `truth_frames_are_cursor_addressed_and_authoritative`

- **Arrange:** session subscribes to sim/x; writer runs to tick 10
- **Act:** set session active cursor to tick 3 (reader seek), then to tick 7
- **Assert:**
    - frames include `CursorReceipt` with correct tick/`commit_hash`
    - values are exactly x=3 then x=7 (bytes)
    - **client required behavior**: replace-only (assert in test by ensuring frames are full-state, not deltas)

##### T8: `outputs_match_recorded_bytes_for_same_tick`

- **Arrange:** writer runs to tick 12 recording outputs; create reader cursor; session subscribes to channels
- **Act:** seek to tick k and publish truth
- **Assert:**
    - `TruthFrames` published at tick k are byte-identical to `ProvenanceStore.outputs(worldline,k)`
    - include same channel set and same bytes

##### T9: `two_sessions_same_channel_different_cursors_receive_different_truth`

- **Arrange:** two sessions subscribe sim/x, with cursors at ticks 2 and 9
- **Act:** publish for both sessions
- **Assert:** `session1` receives `x=2`, `session2` receives `x=9`, no cross-talk

##### T10: `session_cursor_switch_is_opaque_to_subscribers`

- **Arrange:** session subscribes to sim/x, then switch active cursor 3→4
- **Act:** publish
- **Assert:** subscriber did not resubscribe; frames simply update with new `CursorReceipt`

#### D) Reducer edge cases

##### T11: `reducer_commutative_is_permutation_invariant_and_replayable`

- **Arrange:** reducer channel sum with multiple emissions per tick, commutative (Sum)
- **Act:** run with permuted ingress ordering across seeds; record outputs; replay tick
- **Assert:**
    - final sum value identical across permutations
    - playback outputs match recorded outputs bytes

##### T12: `reducer_order_dependent_is_canonically_deterministic_and_replayable`

- **Arrange:** reducer channel concat, order-dependent, relies on canonical key ordering
- **Act:** run across permutations
- **Assert:**
    - output identical across permutations (because canonicalization)
    - playback equals recorded

##### T13: `reduced_channel_emits_single_authoritative_value_per_tick`

- **Arrange:** reducer channel produces many emissions
- **Act:** run one tick
- **Assert:**
    - `outputs[t]` contains exactly one value for that channel (or defined fixed cardinality)
    - ensures we’re not leaking raw event streams

#### E) Determinism + tripwires

##### T14: `cursor_seek_fails_on_corrupt_patch_or_hash_mismatch`

- **Arrange:** corrupt patch bytes at tick 6
- **Act:** reader seek across tick 6
- **Assert:** deterministic error `StateRootMismatch` (or equivalent)

##### T15: `seek_past_available_history_returns_history_unavailable`

- **Arrange:** worldline length 10
- **Act:** seek to tick 50
- **Assert:** deterministic error `HistoryUnavailable { tick: 50 }`

##### T16: `worker_count_invariance_for_writer_advance`

- **Arrange:** same ingress, run 1 tick with `workers {1,2,8,32}`
- **Assert:** `commit_hash` identical

#### F) Checkpoints / wormholes seam

##### T17: `checkpoint_replay_equals_full_replay`

- **Arrange:** retention policy `CheckpointEvery{k=5}`; run to tick 25
- **Act:** seek to tick 23 using checkpoint path and U0-only path
- **Assert:** identical `state_root` + outputs at tick 23

#### G) Forking door-open

##### T18: `fork_worldline_diverges_after_fork_tick_without_affecting_original`

- **Arrange:** original runs to tick 20; fork at tick 7; advance fork for 3 ticks
- **Assert:**
    - original expected hashes unchanged
    - fork expected hashes diverge after tick 7
    - truth frames for original and fork sessions are independent

#### H) MBUS v2 Specific

##### T19: `mbus_v2_roundtrip_single_packet`

**Arrange:**

- Construct `CursorReceipt` with fixed 32-byte hashes:
    - `session_id = [0x01;32]`, `cursor_id=[0x02;32]`, `worldline_id=[0x03;32]`, `warp_id=[0x04;32]`
    - tick = 42
    - `commit_hash=[0x05;32]`
- Construct entries:
    - `channel A = make_channel_id(“a”)`, `value = [1,2,3]`
    - `channel B = make_channel_id(“b”)`, `value = [9]`
    - `value_hash = blake3(value)`

    **Act:**

- `encode_v2_packet(receipt, entries) → bytes`
- `decode_v2_packet(bytes) → packet`

**Assert:**

- decoded receipt fields equal original
- decoded entries `count==2`
- each entry channel/value equals original
- each entry `value_hash` equals blake3(value)

##### T20: `mbus_v1_rejects_v2`

**Arrange:**

- Build a valid v2 packet bytes (from T19)

**Act:**

- Call existing v1 `MaterializationFrame::decode(&packet_bytes[..])`

**Assert:**

- reject due to version mismatch
- `MaterializationFrame::decode(&v2_bytes).is_none()`
- `decode_v2_packet(&v1_bytes).is_none()`

##### T21: `mbus_v2_rejects_v1`

**Arrange:**

- Build a valid v1 frame bytes:
    - `MaterializationFrame::new(channel, data).encode()`

**Act:**

- Call v2 decoder on v1 bytes

**Assert:**

- reject due to version mismatch
- `MaterializationFrame::decode(&v2_bytes).is_none()`
- `decode_v2_packet(&v1_bytes).is_none()`

##### T22: `mbus_v2_multi_packet_roundtrip`

**Arrange:**

- Build packet `P1` (tick=1, entries=`[(chA,[1])]`)
- Build packet `P2` (tick=2, entries=`[(chA,[2]), (chB,[7,7])]`)
- Concatenate bytes = `encode(P1) || encode(P2)`

**Act:**

- `decode_v2_packets(concat_bytes)`

**Assert:**

- returns `Vec` `len=2`
- `packet[0]` matches `P1` exactly
- `packet[1]` matches `P2` exactly

---

## 9) Required Commands

- `cargo test -p warp-core --features delta_validate`
- `cargo test -p echo-dind-harness`
- (bench later) `cargo bench -p warp-core --bench boaw_baseline`

---

## 10) Non-Goals (MVP)

- External provenance fetch and trust verification (signatures)
- `LiveFrontier`-follow cursors (writer advancing while reader follows)
- Cross-worldline merge/collapse (Phase 8)
- Per-worldline privacy policy enforcement (Phase 9)

---

## 11) Global Tick vs Per-Warp Worldlines (from A)

### 11.1 GlobalTick

Every `Engine.commit_with_receipt(tx)` produces one **`GlobalTick`** index (ledger position):

- `global_tick := tick_history.len()` before push

This tick may touch multiple warps.

### 11.2 Per-Warp Worldline Projection

At commit time, project the global tick into per-warp records:

For each `warp_id` touched (or all known warps, storing no-ops):

- `warp_ops = ops.filter(op.key.warp_id == warp_id)` in canonical order
- build `WorldlineTickPatchV1` for that warp at this global tick
- compute per-warp `state_root` after patch applied
- compute per-warp `commit_hash` (warp-local chain)

This gives each warp its own time axis and “now”.

---

## 12) Patch-Centric Per-Warp Tick Artifact

### 12.1 Shared Tick Header (One Per Global Tick)

```rust
struct WorldlineTickHeaderV1 {
  global_tick: u64,
  policy_id: u32,
  rule_pack_id: Hash,
  plan_digest: Hash,
  decision_digest: Hash,   // receipt digest
  rewrites_digest: Hash,
}
```

### 12.2 Per-Warp Patch Record (The Worldline Unit)

```rust
struct WorldlineTickPatchV1 {
  header: WorldlineTickHeaderV1,
  warp_id: WarpId,

  // Canonical ops for this warp only (already sorted by sort_key and validated)
  ops: Vec<WarpOp>,

  // Optional but recommended: conservative slots for this warp (from A)
  in_slots: Vec<SlotId>,
  out_slots: Vec<SlotId>,

  // Deterministic digest of ops (+ header fields that are part of patch identity)
  patch_digest: Hash,
}
```

> [!note]
> _Do not deprecate `WarpTickPatchV1`. The global patch log must persist for whole-engine replay and existing determinism proofs_

#### Patch Digest Definition (Frozen v1)

```rust
patch_digest = blake3( "worldline_tick_patch:v1" || policy_id || rule_pack_id || warp_id || encode_ops(ops) )
```

(Use domain prefix; include `warp_id`; `encode_ops` is your existing canonical op encoding.)

### 12.3 Apply Target

Add a warp-local apply entry point:

- `WorldlineTickPatchV1::apply_to_store(&mut GraphStore)` **or**
- `WorldlineTickPatchV1::apply_to_state(&mut WarpState)` but only mutating the target warp

**MVP recommendation**: `apply_to_store` is cleaner and makes cursor materialization warp-local.

---

## 13) Per-Warp Expected Hashes

Per `(warp_id, tick)` store:

```rust
struct HashTriplet {
  state_root: Hash,    // warp-local state root
  patch_digest: Hash,  // from WorldlineTickPatchV1
  commit_hash: Hash,   // warp-local commit hash v2 shape
}
```

### Commit Hash Definition

```rust
commit_hash = compute_commit_hash_v2( state_root, parents=[prev_commit_hash], patch_digest, policy_id )
```

Parents chain is per warp. No coupling across warps.

---

## 14) Seek Algorithm

- **If target > tick**: apply patches (tick+1..=target) to `cursor.store`.
- **If target < tick**: rebuild `cursor.store` from warp U0 (checkpoint seam later), then apply 0..=target.

### Verification Required After Seek

- `state_root(cursor.store)` equals recorded `expected[warp_id][target].state_root`
- `commit_hash` equals recorded `expected[warp_id][target].commit_hash`
- failure is deterministic error.

---

## 15) What Ships First

### You Will Implement

1. **Per-warp worldline ledger** derived at commit time:
    - `WorldlineTickPatchV1` per `(warp_id, global_tick)`
    - per-warp expected hashes per tick
    - per-warp recorded outputs per tick (from `MaterializationBus` finalize)

2. **`PlaybackCursor`** per warp:
    - `cursor` has its own materialization store
    - can **`Seek`** to any available tick using per-warp patches
    - verifies `state_root` + `commit_hash` identity vs recorded expected hashes

3. **`ViewSession`** per consumer:
    - binds `active_cursor` + channel subscriptions
    - cursor switches are opaque to subscribers (no resubscribe)

4. **`TruthBus` MBUS v2**:
    - cursor-stamped packets carrying (receipt + channel values)
    - v1 remains untouched

### You Explicitly Defer

- fetching archived history (“wormholes”) beyond local availability (but you add the seam)
- running a head warp “Play while cursors follow” (LiveFrontier follow)
- forking semantics beyond a prefix fork stub
- privacy/type registry enforcement

---

## 16) Frozen IDs and Wire Shapes

You already have:

- Hash = `[u8; 32]`
- `WarpId`, `TypeId`, `NodeId`, etc all wrap Hash
- `ChannelId = TypeId` (domain-separated “channel:”)

Add wrappers (transparent) for plan clarity:

- `WorldlineId`(Hash)
- `CursorId`(Hash)
- `SessionId`(Hash)

Ordering is stable (bytewise).

---

## 17) OPORD: 6 Commits + Green Gates

```markdown:disable-run
<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# OPORD: Implement Worldlines + PlaybackCursors + ViewSessions + TruthBus

**Status:** Ready
**Created:** 2026-01-20
**Rule:** No big-bang PR. Every commit must compile. Every gate must be green.

---

## Mission

Implement the SPEC “Worldlines, PlaybackCursors, ViewSessions, TruthBus” such that:

- `Writer` cursor runs normally and appends deterministic provenance + outputs
- `Reader` cursors seek/step back and reproduce hashes + outputs byte-identically
- Clients are dumb: `TruthFrames` replace render state with cursor stamp
- `ProvenanceStore` seam exists for future wormholes

---

## Commit 1 — MBUS v2 Encoder/Decoder + Tests

**Goal:**

Front-load publishing correctness.

**Add**

- new file: `materialization/frame_v2.rs`
- MBUS v2 encoder/decoder
- tests: roundtrip, version rejection, multi-packet

**Gate**

- `cargo test -p warp-core --features delta_validate`

---

## Commit 2 — Types + IDs + ProvenanceStore Seam + Per-Warp Worldline Store (Merged)

**Goal:**

Compile-only scaffold; in-memory per-warp store.

**Add**

- `crates/warp-core/src/worldline.rs`
  - `WorldlineId`, `HashTriplet`, `ProvenancePayload`
- `crates/warp-core/src/playback.rs`
  - `CursorId`, `SessionId`, `CursorRole`, `PlaybackMode`, `SeekThen`
  - `CursorReceipt`, `TruthFrame`
- `crates/warp-core/src/provenance_store.rs`
  - `ProvenanceStore` trait
  - `LocalProvenanceStore` (in-memory Vec-backed)
- WarpWorldlines inside Engine:
    - `BTreeMap<WarpId, Vec<WorldlineTickPatchV1>>`
    - `BTreeMap<WarpId, Vec<HashTriplet>>`
    - `BTreeMap<WarpId, Vec<OutputFrameSet>>`
- Project ops → per-warp patches at commit

**Gate**

- `cargo test -p warp-core --features delta_validate`

---

## Commit 3 — Warp-Local Apply + State Root + Cursor Seek + Verification (Merged)

**Goal:**

`Reader` seek works; verifies expected hashes.

**Add**

- `PlaybackCursor` struct (in `playback.rs` or `cursor.rs`)
- `cursor.seek_to(target)` uses `ProvenanceStore` patches/expected
- canonical `compute_state_root_for_warp(store, warp_id)` shared function (used later by writer commit too)
- add `apply_to_store` for `WarpOp`/`WorldlineTickPatchV1`
- add `compute_state_root_for_warp_store(&GraphStore, root_node)` or define a warp-local root policy

**Add tests**

- T15 `seek_past_available_history_returns_history_unavailable` (uses `LocalProvenanceStore` `len`)
- T14 `cursor_seek_fails_on_corrupt_patch_or_hash_mismatch` (fake expected mismatch)

**Gate**

- `cargo test -p warp-core --features delta_validate`

---

## Commit 4 — ViewSession + TruthBus (Cursor-Addressed Publish) + Writer Cursor Advance

**Goal:**

Sessions bind cursor+subs; publish frames; subscribers don’t resubscribe; writer runs normally.

**Add**

- `ViewSession` type with:
  - `active_cursor`
  - subscriptions set
- `TruthBus` trait (or reuse your existing `MaterializationPort` by wrapping)
  - `publish_cursor_receipt`
  - `publish_truth_frame`
- Minimal in-memory bus for tests
- `Writer` advance pipeline:
  - BOAW work queue executes rules into `TickDelta`
  - `merge_deltas` canonical
  - apply patch to writer cursor store
  - compute expected hashes and append to `ProvenanceStore`
- Implement `PlaybackMode`:
  - `Play`
  - `StepForward` (then `Paused`)
  - `Paused`

**Add tests**

- T9 `two_sessions_same_channel_different_cursors_receive_different_truth`
- T10 `session_cursor_switch_is_opaque_to_subscribers`
- T7 `truth_frames_are_cursor_addressed_and_authoritative` (stub values for now)
- T1 `writer_play_advances_and_records_outputs` (outputs can be placeholder until next commit)
- T2 `step_forward_advances_one_then_pauses`
- T3 `paused_noop_even_with_pending_intents`
- T16 `worker_count_invariance_for_writer_advance`

**Gate**

- `cargo test -p warp-core --features delta_validate`
- `cargo test -p echo-dind-harness`

---

## Commit 5 — Record Outputs Per Tick (Truth Holography MVP) + Seek/Playback

**Goal:**

Outputs are recorded and replayed byte-identically; clients remain dumb.

**Implement**

- On writer commit: record `OutputFrameSet` per tick into `ProvenanceStore`:
  - `outputs(worldline,tick) = Vec<(ChannelId, bytes)>`
- On session publish: source outputs from `ProvenanceStore.outputs` for `(worldline, tick)`

**Add tests**

- T8 `outputs_match_recorded_bytes_for_same_tick` (byte compare)
- T4 `seek_moves_cursor_without_mutating_writer_store` (ensure seek doesn’t touch writer store)
- T5 `step_back_is_seek_minus_one_then_pause`
- T6 `reader_play_consumes_existing_then_pauses_at_frontier`
- T19–T22 MBUS v2 specifics

**Gate**

- `cargo test -p warp-core --features delta_validate`

---

## Commit 6 — Reducer Semantics + Checkpoint Skeleton + Fork Stub

**Goal:**

Cement reducer truth semantics and open Phase 7.5/7.9 doors.

**Implement**

- Reducer output policy enforcement:
  - reduced channels publish one authoritative value per tick
- Checkpoint policy skeleton:
  - `RetentionPolicy::CheckpointEvery{k}`
  - store checkpoint refs (even if naive clone)
- Fork stub:
  - prefix-copy worldline up to tick

**Add tests**

- T11 commutative reducer invariance + replayable
- T12 order-dependent reducer canonically deterministic + replayable
- T13 `reduced_channel_single_authoritative_value_per_tick`
- T17 `checkpoint_replay_equals_full_replay` (naive checkpoint OK)
- T18 `fork_worldline_diverges_after_fork_tick_without_affecting_original`

**Gate**

- `cargo test -p warp-core --features delta_validate`

---

## Post-OPORD: Bench Baseline (Guardrail)

- Add `crates/warp-core/benches/boaw_baseline.rs`
- Document in `docs/notes/boaw-perf-baseline.md`
```

## Commanders’ Intent

Build the boundary artifacts and publishing semantics first. Make it correct. Then make it fast.
