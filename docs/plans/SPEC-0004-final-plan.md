<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# SPEC-0004 Implementation Plan: Worldlines, PlaybackCursors, ViewSessions, TruthBus

**Status:** In Progress
**Created:** 2026-01-20
**Spec:** `/docs/spec/SPEC-0004-worldlines-playback-truthbus.md`

---

## Corrections Applied (from review)

1. **U0Ref = WarpId** — MVP U0Ref is just a handle to `engine.initial_state` for a warp, not a checkpoint blob
2. **One entry per global tick per warp** — Store patches even if empty to maintain index alignment: `warp_patches[warp_id].len() == global_tick_history_len`
3. **Use existing canonical hash scheme** — `compute_state_root_for_warp_store` must use same ordering as `snapshot.rs`
4. **Minimal truth sink** — Just `BTreeMap<SessionId, Vec<TruthFrame>>`, not a full bus layer
5. **Add demo emission for tests** — Need deterministic emission path or outputs are vacuous
6. **Explicit WarpOp coverage** — `apply_warp_op_to_store` must handle all variants or reject with typed error

---

## Commit Status

### ✅ Commit 1 — MBUS v2 Encoder/Decoder + Tests (COMPLETE)

**Files Created:**

- `crates/warp-core/src/materialization/frame_v2.rs` — V2 encoder/decoder with cursor-stamped packets

**Files Modified:**

- `crates/warp-core/src/materialization/mod.rs` — Export frame_v2 types

**Tests Passing (11/11):**

- T19: `mbus_v2_roundtrip_single_packet`
- T20: `mbus_v1_rejects_v2`
- T21: `mbus_v2_rejects_v1`
- T22: `mbus_v2_multi_packet_roundtrip`
- Plus edge case tests (empty entries, bad magic, truncated, etc.)

**Gate:** `cargo test -p warp-core --features delta_validate -- frame_v2` ✅

---

### 🔲 Commit 2 — Types + IDs + ProvenanceStore Seam + Per-Warp Worldline Store

**New Files:**

- `crates/warp-core/src/worldline.rs`
    - `WorldlineId(Hash)` — transparent wrapper
    - `HashTriplet { state_root, patch_digest, commit_hash }`
    - `WorldlineTickPatchV1` — per-warp projection of global tick
    - `WorldlineTickHeaderV1` — shared header across warps
    - `OutputFrameSet = Vec<(ChannelId, Vec<u8>)>`

- `crates/warp-core/src/playback.rs`
    - `CursorId(Hash)`, `SessionId(Hash)` — transparent wrappers
    - `CursorRole { Writer, Reader }`
    - `PlaybackMode { Paused, Play, StepForward, StepBack, Seek { target, then } }`
    - `SeekThen { Pause, RestorePrevious, Play }`
    - `CursorReceipt` — cursor context for truth frames
    - `TruthFrame` — authoritative value with cursor receipt

- `crates/warp-core/src/provenance_store.rs`
    - `ProvenanceStore` trait (seam for future wormholes)
    - `LocalProvenanceStore` — in-memory Vec-backed implementation
    - `HistoryError { HistoryUnavailable { tick }, WorldlineNotFound }`
    - `U0Ref = WarpId` (per correction #1)

**Engine Modifications (`engine_impl.rs`):**

- Add fields:

    ```rust
    warp_patches: BTreeMap<WarpId, Vec<WorldlineTickPatchV1>>,
    warp_expected: BTreeMap<WarpId, Vec<HashTriplet>>,
    warp_outputs: BTreeMap<WarpId, Vec<OutputFrameSet>>,
    ```

- Modify `commit_with_receipt` to project global ops → per-warp patches
- **Invariant:** `warp_patches[warp_id].len() == tick_history.len()` (even for no-ops)

**Gate:** `cargo test -p warp-core --features delta_validate`

---

### 🔲 Commit 3 — Warp-Local Apply + State Root + Cursor Seek + Verification

**Add to `playback.rs`:**

- `PlaybackCursor` struct with:
    - `cursor_id`, `worldline_id`, `warp_id`, `tick`, `role`, `mode`
    - `store: GraphStore` (owned, never shared)
    - `pin_max_tick: u64`
- `PlaybackCursor::seek_to(target, provenance)`:
    - If `target < tick`: rebuild from U0 (initial_state for warp)
    - Apply patches `tick..<target` (exclusive upper bound)
    - Verify `state_root`, `patch_digest`, and `commit_hash` match expected per tick
- `SeekError { HistoryUnavailable, StateRootMismatch, CommitHashMismatch }`

**Add to `worldline.rs`:**

- `WorldlineTickPatchV1::apply_to_store(&self, store: &mut GraphStore)`
- `apply_warp_op_to_store(store, op)` — handle ALL WarpOp variants explicitly

**Add to `snapshot.rs`:**

- `compute_state_root_for_warp_store(&GraphStore, WarpId)` — use existing canonical hash scheme

**Tests:**

- T14: `cursor_seek_fails_on_corrupt_patch_or_hash_mismatch`
- T15: `seek_past_available_history_returns_history_unavailable`

**Gate:** `cargo test -p warp-core --features delta_validate`

---

### 🔲 Commit 4 — ViewSession + TruthBus + Writer Cursor Advance

**Add to `playback.rs`:**

- `ViewSession { session_id, active_cursor, subscriptions: BTreeSet<ChannelId> }`
- `ViewSession::subscribe(channel)`, `set_active_cursor(cursor)`

**Truth Sink (minimal, per correction #4):**

- `TruthSink { frames: BTreeMap<SessionId, Vec<TruthFrame>>, receipts: BTreeMap<SessionId, Vec<CursorReceipt>> }`
- Helper: `collect_frames(session_id) -> &[TruthFrame]` — returns frames for a session
- Helper: `last_receipt(session_id) -> Option<&CursorReceipt>` — reads from the receipts map

**PlaybackCursor::step():**

- Implement `PlaybackMode` state machine
- `Paused` → no-op
- `Play` → Writer appends (BOAW), Reader consumes then pauses at frontier
- `StepForward` → advance one then `Paused`
- `StepBack` → seek(tick-1) then `Paused`
- `Seek { target, then }` → seek then apply `SeekThen`

**Tests:**

- T1: `writer_play_advances_and_records_outputs`
- T2: `step_forward_advances_one_then_pauses`
- T3: `paused_noop_even_with_pending_intents`
- T7: `truth_frames_are_cursor_addressed_and_authoritative`
- T9: `two_sessions_same_channel_different_cursors_receive_different_truth`
- T10: `session_cursor_switch_is_opaque_to_subscribers`
- T16: `worker_count_invariance_for_writer_advance`

**Gate:** `cargo test -p warp-core --features delta_validate` + `cargo test -p echo-dind-harness`

---

### 🔲 Commit 5 — Record Outputs Per Tick + Seek/Playback

**Engine Modifications:**

- On `commit_with_receipt`, after `bus.finalize()`:

    ```rust
    let outputs: OutputFrameSet = mat_report.channels
        .iter()
        .map(|fc| (fc.channel, fc.data.clone()))
        .collect();
    self.warp_outputs.entry(root_warp).or_default().push(outputs);
    ```

**Demo Emission (per correction #5):**

- Add deterministic test emission path so T1/T8 aren't vacuous
- Option A: Demo rule that emits to channel based on tick
- Option B: Compute outputs from state deterministically for tests

**ViewSession Publishing:**

- `publish_truth(cursor, provenance, sink)` sources from `provenance.outputs(worldline, tick)`

**Tests:**

- T4: `seek_moves_cursor_without_mutating_writer_store`
- T5: `step_back_is_seek_minus_one_then_pause`
- T6: `reader_play_consumes_existing_then_pauses_at_frontier`
- T8: `outputs_match_recorded_bytes_for_same_tick`
- T19-T22: MBUS v2 integration

**Gate:** `cargo test -p warp-core --features delta_validate`

---

### 🔲 Commit 6 — Reducer Semantics + Checkpoint Skeleton + Fork Stub

**New File: `crates/warp-core/src/retention.rs`**

```rust
pub enum RetentionPolicy {
    KeepAll,
    CheckpointEvery { k: u64 },
    KeepRecent { window: u64, checkpoint_every: u64 },
    ArchiveToWormhole { after: u64, checkpoint_every: u64 }, // seam only
}
```

**Checkpoint Skeleton:**

- `LocalProvenanceStore::checkpoint(warp_id, tick, state)` — naive clone
- `checkpoint_before(worldline, tick)` for fast seek

**Fork Stub:**

- `LocalProvenanceStore::fork(source, fork_tick, new_id)` — prefix-copy

**Tests:**

- T11: `reducer_commutative_is_permutation_invariant_and_replayable`
- T12: `reducer_order_dependent_is_canonically_deterministic_and_replayable`
- T13: `reduced_channel_emits_single_authoritative_value_per_tick`
- T17: `checkpoint_replay_equals_full_replay`
- T18: `fork_worldline_diverges_after_fork_tick_without_affecting_original`

**Gate:** `cargo test -p warp-core --features delta_validate`

---

## Key Files Reference

| File                             | Purpose                                                   |
| -------------------------------- | --------------------------------------------------------- |
| `materialization/frame.rs:1-255` | Pattern for MBUS encoding                                 |
| `engine_impl.rs:967-1085`        | `commit_with_receipt` — hook for per-warp projection      |
| `tick_patch.rs:98-461`           | `WarpOp`, `apply_to_state` — pattern for `apply_to_store` |
| `snapshot.rs:90-265`             | `compute_state_root`, `compute_commit_hash_v2`            |
| `graph.rs:16-486`                | `GraphStore`, `canonical_state_hash`                      |

---

## Invariants (from spec)

- **WL-001 (Holography):** Given U0Ref + patches + canonical apply, any tick's state is reconstructible
- **WL-002 (Truth):** Given recorded outputs per tick, any tick's client-visible truth is reconstructible byte-for-byte
- **CUR-001:** Cursor never mutates worldline unless role is Writer and mode requires advance
- **CUR-002:** Cursor never executes rules when seeking; it applies recorded patches only
- **CUR-003:** After seek/apply, cursor verifies expected hashes byte-for-byte
- **OUT-001:** For `(worldline_id, tick, channel)`, value bytes are deterministic across runs/machines
- **OUT-002:** Playback at tick t reproduces the same TruthFrames recorded at tick t
- **STEP-001:** No store mutation while any GraphView borrow exists for that store
- **STEP-002:** Seeking never touches writer cursor store; only cursor.store
