<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# RMG MWMR Concurrency Spec (Footprints, Ports, Factor Masks)

Status: Draft • Date: 2025-10-27 • Owner: rmg-core

## Why

We want lock-free multi-writer/multi-reader (MWMR) deterministic rewriting. Under DPOI semantics, if matches are pairwise independent and the no-delete-under-descent invariant holds, a batch’s result is unique up to typed open-graph isomorphism independent of order. This doc fixes the runtime model, data structures, and perf plan.

## Runtime Model

State ⟨G, epoch_att, epoch_skel, P⟩
- G: working graph (skeleton + attachments)
- epoch_att / epoch_skel: monotonically increasing u64 counters (attachments, skeleton)
- P: pending rewrites ⟨rule, match, footprint, stamp, phase⟩

Phases
- MATCH: compute monic match m: L ↪ G; gluing tests; compute footprint F; enqueue Matched
- RESERVE (lock-free OCC): allowed iff independent(F, Y.F) for all Y with phase∈{Reserved,Committed}; then phase := Reserved
- COMMIT (bounded CAS):
  - (a) skeleton edits (N/E) with release-stores
  - (b) port occupancy (B) with release-stores
  - publish journals; if any P_write ⇒ epoch_att++; if any N/E_write ⇒ epoch_skel++
- ABORT/RETRY/JOIN on independence failure or validation error

Reader isolation
- Readers acquire both epochs at entry and never see torn state; flips happen only after publication. Reclamation after a grace period.

## Footprints & Independence

Footprint F = (N_read, N_write, E_read, E_write, B_in, B_out; factor_mask)
- N_*: node bitmaps; E_*: edge bitmaps
- B_in/B_out: boundary port occupancy bitmaps; port key = `(node_id << 32) | (port_id << 2) | dir_bits`
- factor_mask: u64 coarse partition (room/shard/system factor)

Independence(F1,F2) iff
- (F1.N_write ∪ F1.E_write ∪ F1.B_in ∪ F1.B_out) is disjoint from all read/write sets of F2, and symmetrically; and
- (F1.factor_mask & F2.factor_mask) == 0

Ordering & determinism
- Physical execution is parallel; planning/logs use a stable key `(scope_hash, rule_id, stamp)`; results are order-independent by Theorem A.

## Scheduler & Batching

- Build maximal independent sets (MIS) from Matched.
- Reserve MIS entries; commit them in parallel.
- Conflicts ⇒ RETRY or JOIN (precomputed join) per rule policy.
- Priorities: physics > gameplay > cosmetic (configurable); fairness via randomized backoff.

## Data Structures

- Bitmaps: block-sparse (Roaring-style) with SIMD kernels for AND-isZero/OR (AVX2/NEON); scalar fallback.
- Ports: two bitmaps B_in/B_out keyed by packed port id; hot path for interface conflicts.
- Factor masks: O(1) precheck before bitmaps.
- Compact ids: internal `CompactRuleId(u32)`; wire/disk keeps canonical `Hash256`.
- Node/Edge indices: `NodeIx/EdgeIx`; hash ids for global identity.

## Two-Plane Publish

- Enforce no-delete-under-descent: attachment positions touched cannot be deleted by concurrent skeleton rewrites.
- Publish attachments, then skeleton; epochs per plane; pointer swaps/double-buffered sections; readers pinned by epoch.
- Lazy flips: new readers bind to new epochs immediately; old readers finish on old epochs; reclamation after grace period.

## Zero-Copy Storage Alignment

- Snapshot = page-aligned slabs: headers, NodeEntry[], EdgeEntry[], payload arena.
- Load via mmap; base+offset arithmetic; zero decode.
- Snapshot hash = BLAKE3 over canonical slabs; optional Merkle overlays for partial verify.

## Rule Identity & Hot-Load

- Family ID (stable): `blake3("rule-family:v1" || fully_qualified_name)` — compile-time const in Rust; computed once on load in Lua.
- Revision ID (dynamic): `blake3("rule-rev:<lang>:canon-ast-v1" || canonical AST graph bytes)` — flips on semantic changes; used for hot‑reload/peer compatibility; not in scheduling keys.

## Performance Targets

Baseline demo (Phase 1):
- 1k nodes; 10 concurrent rewrites/tick @ 60 FPS
- Independence + commit ≤ 2 ms; matching ≤ 8 ms (typed, local, incremental optional)

Stretch demo (Phase 2):
- 10k nodes; 100 concurrent rewrites/tick; SIMD bitmaps + factor masks + incremental caches

## Telemetry (JSONL)

- `conflict_rate`, `retry_count`, `join_success`, `reservation_latency_ms`, `commit_latency_ms`
- `epoch_flip_latency_ms`, `reader_epoch_lifetime_ms_p50/p95/p99`
- `bitmap_and_checked`, `bitmap_and_short_circuits`, `factor_mask_elided`
- `matches_found`, `matches_invalidated`, `match_time_ms`

## Risks & Mitigations

- Matching cost: constrain |L| ≤ 5–10; typed seeds; local neighborhoods; incremental rematch near diffs; only add incremental when matching > 50% frame time.
- Conflict storms: finer factor masks (per-room/per-type/per-port); join catalog; priority scheduling.
- Epoch stalls: double-buffer planes; lazy flips; grace period reclamation.
- Port bottleneck: versioned ports; batch reservations; separate factor masks for input/output/internal ports.

## Roadmap & Deliverables

Phase 0 (Tick determinism)
- Footprint + independence (ports/nodes/edges/factor)
- MIS batch planner; permutation test for isomorphic results
- Two-plane commutation harness under no-delete-under-descent

Phase 1 (Baseline performance)
- SIMD bitmaps; factor masks; CompactRuleId(u32); basic telemetry
- Bench 1k×10 @ 60 FPS; independence+commit ≤ 2 ms

Phase 2 (Optimization)
- Spatial indexing/sharding; incremental matching; join catalog; Merkle overlays
- Bench 10k×100; independence ≤ 2 ms; matching ≤ 8 ms

Phase 3 (Real demo)
- Multiplayer confluence demo (zero desync), time‑travel fork/merge, inspector visualization of footprints/conflicts

References: confluence skeleton v5, RMG math confluence, offset-graph arena notes

