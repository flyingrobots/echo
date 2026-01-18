<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
<!-- markdownlint-disable MD007 MD029 MD033 MD046 -->
Perfect! Now I have a comprehensive understanding. Let me create a summary document for you.

       Summary: warp-core Module Organization & Architecture

       Based on my thorough exploration of /Users/james/git/echo/crates/warp-core/src, here's a detailed breakdown of
       the module structure, key components, and their responsibilities.

       ---
       1. OVERALL MODULE ORGANIZATION

       The crate is organized into several architectural layers:

       Core Modules (42 Rust files total)

       Public API Modules:
       - math/ - Deterministic math subsystem (Vec3, Mat4, Quat, PRNG, trigonometry)
       - wsc/ - Write-Streaming Columnar snapshot format
       - inbox/ - Canonical inbox management for deterministic intent sequencing
       - materialization/ - Channel-based output subsystem
       - fixed/ - Q32.32 fixed-point helpers

       Private Infrastructure Modules:
       - engine_impl.rs (1806 lines) - Core rewrite engine
       - scheduler.rs (1304 lines) - Deterministic radix-sort scheduler
       - tick_patch.rs (1694 lines) - Delta boundary artifacts (Paper III)
       - graph.rs (568 lines) - In-memory graph store
       - snapshot.rs (350 lines) - State snapshots and hash computation
       - receipt.rs (241 lines) - Tick receipts (accepted/rejected rewrites)
       - footprint.rs (250 lines) - Independence checks and read/write tracking
       - rule.rs (84 lines) - Rewrite rule definitions
       - attachment.rs (403 lines) - Polymorphic payload system
       - payload.rs (304 lines) - Motion atom encoding/decoding
       - warp_state.rs (116 lines) - Multi-instance WARP state (Stage B1)

       Supporting Modules:
       - ident.rs (162 lines) - Identifier types and hashing
       - record.rs - Node/edge record definitions
       - tx.rs - Transaction identifiers
       - telemetry.rs - Observability hooks
       - sandbox.rs - Testing harness
       - serializable.rs - Serde support (feature-gated)
       - constants.rs - Constants and policy IDs

       ---
       2. GRAPHSTORE DEFINITION & USAGE

       Location: /Users/james/git/echo/crates/warp-core/src/graph.rs

       Purpose: In-memory graph storage for the rewrite executor and tests. The production engine will eventually swap
       in a content-addressed store, but this structure keeps the spike self-contained.

       Key Structure:
       pub struct GraphStore {
           pub(crate) warp_id: WarpId,
           pub(crate) nodes: BTreeMap<NodeId, NodeRecord>,
           pub(crate) edges_from: BTreeMap<NodeId, Vec<EdgeRecord>>,  // Source → edges
           pub(crate) edges_to: BTreeMap<NodeId, Vec<EdgeId>>,        // Reverse: target ← edge ids
           pub(crate) node_attachments: BTreeMap<NodeId, AttachmentValue>,
           pub(crate) edge_attachments: BTreeMap<EdgeId, AttachmentValue>,
           pub(crate) edge_index: BTreeMap<EdgeId, NodeId>,           // Reverse lookup: edge → from
           pub(crate) edge_to_index: BTreeMap<EdgeId, NodeId>,        // Reverse lookup: edge → to
       }

       Key API Operations:
       - insert_node(id, record) - Add/replace node
       - insert_edge(from, edge) - Add/replace directed edge with upsert semantics
       - delete_node_cascade(node) - Remove node and all incident edges
       - delete_edge_exact(from, edge_id) - Remove specific edge
       - canonical_state_hash() - Deterministic BLAKE3 digest over full graph (V2 format)
       - Iteration: iter_nodes(), iter_edges(), iter_node_attachments(), iter_edge_attachments()

       Determinism Contract:
       - All maps use BTreeMap for stable iteration order
       - Edge hashing sorts all edges by EdgeId before encoding
       - Attachment hashing includes type_id + u64 LE length + bytes
       - Node count and edge count encoded as u64 LE for WSC alignment

       Usage in Engine:
       - Wrapped in WarpState for multi-instance support (Stage B1)
       - Modified during rewrite execution via rule's ExecuteFn
       - Snapshotted after tick commit to verify determinism
       - Replayed via TickPatch for delta reconstruction

       ---
       3. EXECUTOR/REWRITE PIPELINE CODE

       Main Engine: /Users/james/git/echo/crates/warp-core/src/engine_impl.rs (1806 lines)

       High-Level Pipeline:

       ┌──────────────────────────────────────┐
       │  EngineBuilder (fluent constructor)  │
       └──────────────┬──────────────────────┘
                      │
                      ▼
                ┌─────────────┐
                │   Engine    │  Owns:
                └─────────────┘  - WarpState (graph stores)
                      │         - Registered RewriteRules
                      │         - DeterministicScheduler
                      │         - TickHistory
                      │         - MaterializationBus
                      │
            ┌─────────┼─────────┐
            │         │         │
            ▼         ▼         ▼
       ┌─────────┐ ┌──────────┐ ┌────────────────┐
       │ Ingest  │ │ Dispatch │ │  Tick Commit   │
       │ Intent  │ │  Intent  │ │  + Finalize    │
       └─────────┘ └──────────┘ └────────────────┘
            │         │              │
            └─────────┼──────────────┘
                      │
                      ▼
                ┌─────────────────────┐
                │ Rewrite Execution   │
                │ (Matching & Apply)  │
                └─────────────────────┘

       Core Types:

       RewriteRule (/Users/james/git/echo/crates/warp-core/src/rule.rs):
       pub struct RewriteRule {
           pub id: Hash,                              // Deterministic rule ID
           pub name: &'static str,                    // Human-readable name
           pub left: PatternGraph,                    // Pattern metadata
           pub matcher: MatchFn,                      // fn(&GraphView, &NodeId) -> bool
           pub executor: ExecuteFn,                   // fn(&GraphView, &mut TickDelta, &NodeId) [Phase 5]
           pub compute_footprint: FootprintFn,        // fn(&GraphView, &NodeId) -> Footprint
           pub factor_mask: u64,                      // O(1) spatial prefilter
           pub conflict_policy: ConflictPolicy,       // Abort, Retry, or Join
           pub join_fn: Option<JoinFn>,              // Optional join strategy
       }

       PendingRewrite (scheduler):
       pub(crate) struct PendingRewrite {
           pub rule_id: Hash,
           pub compact_rule: CompactRuleId,           // Hot-path handle
           pub scope_hash: Hash,                      // Deterministic ordering key
           pub scope: NodeKey,                        // Scope node reference
           pub footprint: Footprint,                  // Read/write sets
           pub phase: RewritePhase,                   // Matched → Reserved → Committed/Aborted
       }

       Execution Flow:

       1. Registration (register_rule):
         - Validate rule uniqueness
         - Assign compact rule ID
         - Store in lookup maps
       2. Enqueueing (apply):
         - Match rule against scope
         - Compute footprint
         - Enqueue PendingRewrite with last-wins semantics on (scope_hash, compact_rule)
       3. Scheduling (radix drain):
         - Extract pending rewrites in deterministic order: (scope_hash, rule_id, nonce)
         - Stable LSD radix sort (20 passes: 2 nonce + 2 rule + 16 scope) with 16-bit digits
         - O(n) complexity, zero comparisons
       4. Independence Checks:
         - Check footprint against ActiveFootprints (generation-stamped sets)
         - O(1) conflict detection via GenSet lookups
         - Apply conflict policy: Abort, Retry, or Join
       5. Execution:
         - Invoke rule's executor callback with mutable GraphStore
         - Create ScopedEmitter for materialization
         - Collect emissions in order-independent BTreeMap<EmitKey, Vec<u8>>
       6. Commit:
         - Snapshot graph state
         - Compute state root hash
         - Finalize materialization channels
         - Record (Snapshot, TickReceipt, TickPatch) in tick history

       ---
       4. DELTA/TICK INFRASTRUCTURE

       Location: /Users/james/git/echo/crates/warp-core/src/tick_patch.rs (1694 lines)

       Purpose:

       Tick patches are prescriptive boundary records for one worldline step—sufficient to deterministically replay the
       tick as a pure delta without re-running rule matching or scheduling.

       Key Types:

       TickCommitStatus:
       pub enum TickCommitStatus {
           Committed,  // Successful commit
           Aborted,    // Reserved for future transactional semantics
       }

       SlotId (unversioned slot identifiers):
       pub enum SlotId {
           Node(NodeKey),           // Full node record (instance-scoped skeleton)
           Edge(EdgeKey),           // Full edge record (instance-scoped skeleton)
           Attachment(AttachmentKey), // Node/edge plane payload + Descend links
           Port(PortKey),           // Boundary port value (opaque key)
       }

       WarpOp (canonical delta operations):
       pub enum WarpOp {
           PortalInit { ... },      // Open descended attachment atomically (Stage B1.1)
           UpsertNode { ... },      // Insert/replace node record
           DeleteNode { ... },      // Delete node (cascade edges)
           UpsertEdge { ... },      // Insert/replace edge record
           DeleteEdge { ... },      // Delete specific edge
           SetAttachment { ... },   // Set/clear attachment value
           // ... more operations
       }

       WarpTickPatchV1 (minimal intentional format):
       pub struct WarpTickPatchV1 {
           pub commit_status: TickCommitStatus,
           pub ops: Vec<WarpOp>,           // Canonical graph edits
           pub in_slots: Vec<SlotId>,      // Unversioned input slots
           pub out_slots: Vec<SlotId>,     // Unversioned output slots
           pub patch_digest: Hash,         // BLAKE3 digest of patch bytes
       }

       Determinism Properties:

       - Ops are prescriptive: execute without re-matching
       - in_slots/out_slots are unversioned (Paper III-compatible)
       - Value versioning recovered via (slot_id, tick_index) interpretation
       - Patch digest committed to snapshot for boundary verification

       Usage in Engine:

       - diff_state() computes patch after commit
       - Tick history stores: Vec<(Snapshot, TickReceipt, WarpTickPatchV1)>
       - Used by jump_to_tick(index) for deterministic replay
       - Future: replayed to recover intermediate state without re-running rules

       ---
       5. SCHEDULER & INDEPENDENCE CHECKING

       Location: /Users/james/git/echo/crates/warp-core/src/scheduler.rs (1304 lines)

       RadixScheduler:

       pub(crate) struct RadixScheduler {
           pending: HashMap<TxId, PendingTx<PendingRewrite>>,
           pub(crate) active: HashMap<TxId, ActiveFootprints>,
       }

       ActiveFootprints (generation-stamped conflict detection):

       pub(crate) struct ActiveFootprints {
           nodes_written: GenSet<NodeKey>,
           nodes_read: GenSet<NodeKey>,
           edges_written: GenSet<EdgeKey>,
           edges_read: GenSet<EdgeKey>,
           attachments_written: GenSet<AttachmentKey>,
           attachments_read: GenSet<AttachmentKey>,
           ports: GenSet<PortKey>,
       }

       Key Operations:

       - enqueue(tx, rewrite) - Last-wins on (scope_hash, compact_rule)
       - drain_for_tx(tx) - Returns pending rewrites in deterministic order
       - try_reserve(rewrite) - O(m) footprint independence check via GenSet

       Deterministic Ordering:

       Lexicographic order: (scope_hash, rule_id, nonce)
       - scope_hash: Full 32-byte node hash
       - rule_id: Compact rule identifier
       - nonce: Insertion order tiebreaker

       ---
       6. MATERIALIZATION SUBSYSTEM

       Location: /Users/james/git/echo/crates/warp-core/src/materialization/

       Architecture:

       External World
            │
            ▲ (ViewOps frames)
            │
       ┌────┴──────────────┐
       │ MaterializationPort │  ← Boundary API
       └────┬──────────────┘
            │
       ┌────┴──────────────┐
       │ MaterializationBus  │  ← Internal Runtime
       └────┬──────────────┘
            │
       ┌────┴──────────────┐
       │      Engine        │  commit → finalize
       └───────────────────┘

       Key Components:

       ChannelId (256-bit hash from domain-separated label):
       pub fn make_channel_id(label: &str) -> ChannelId

       ChannelPolicy (finalization strategies):
       pub enum ChannelPolicy {
           Log,              // All emissions in EmitKey order (default, confluence-safe)
           StrictSingle,     // Error if >1 emission (single-writer enforcement)
           Reduce(ReduceOp), // Merge via commutative operation (Sum, First, etc.)
       }

       EmitKey (order-independent emission identifiers):
       pub struct EmitKey {
           pub scope_hash: Hash,      // Hash of scope node
           pub rule_id: u32,          // Compact rule ID
           pub subkey: u32,           // Optional differentiator (default 0)
       }
       Lexicographic ordering ensures deterministic finalization regardless of rule execution order.

       ScopedEmitter (context-aware emission adapter):
       pub struct ScopedEmitter<'a> {
           bus: &'a MaterializationBus,
           scope_hash: Hash,          // Auto-filled from execution context
           rule_id: u32,              // Prevents rule forgery
       }
       - emit(channel, data) - Emit with subkey 0
       - emit_with_subkey(channel, subkey, data) - Multi-emission support

       MaterializationBus (tick-scoped collection):
       pub struct MaterializationBus {
           pending: RefCell<BTreeMap<ChannelId, BTreeMap<EmitKey, Vec<u8>>>>,
           policies: BTreeMap<ChannelId, ChannelPolicy>,
       }
       - Collects emissions during tick execution
       - Finalizes post-commit according to policies
       - Not thread-safe by design (single-threaded tick execution)

       MaterializationPort (external subscription interface):
       pub struct MaterializationPort {
           subscriptions: BTreeSet<ChannelId>,
           replay_cache: BTreeMap<ChannelId, Vec<u8>>,     // Replay(1) semantics
           pending_frames: Vec<MaterializationFrame>,
       }
       - Subscribe to channels
       - Receive cached values (late-joiner support)
       - Drain pending frames for transport

       Confluence Safety:

       - EmitKey ordering derived from scheduler's canonical order
       - All policies preserve confluence (no silent "winner picks")
       - Duplicate emissions rejected (catches non-deterministic iteration)

       ---
       7. TICK RECEIPTS & SNAPSHOTS

       TickReceipt (/Users/james/git/echo/crates/warp-core/src/receipt.rs):
       pub struct TickReceipt {
           tx: TxId,
           entries: Vec<TickReceiptEntry>,    // Per-candidate outcomes
           blocked_by: Vec<Vec<u32>>,         // Blocking causality poset
           digest: Hash,                       // Canonical commitment
       }
       - Records accepted vs rejected rewrites
       - Captures blocking causality (which accepted rewrites blocked rejections)
       - Digest commits to decision outcomes

       Snapshot (/Users/james/git/echo/crates/warp-core/src/snapshot.rs):
       pub struct Snapshot {
           pub root: NodeKey,
           pub hash: Hash,                    // Commit ID (v2)
           pub parents: Vec<Hash>,            // Parent snapshot hashes
           pub plan_digest: Hash,             // Candidate set + ordering
           pub decision_digest: Hash,         // Tick receipt outcomes
           pub rewrites_digest: Hash,         // Applied rewrites
           pub patch_digest: Hash,            // Tick patch boundary
           pub policy_id: u32,                // Aion policy version pin
           pub tx: TxId,
       }
       - state_root: Graph-only BLAKE3 hash
       - commit_id (v2): state_root + patch_digest + parents + policy_id
       - Deterministic across node, edge, and attachment traversal

       ---
       8. KEY INVARIANTS & DETERMINISM

       Ordering Guarantees:
       - All maps use BTreeMap for stable iteration
       - Rewrites execute in (scope_hash, rule_id, nonce) order (O(n) radix sort)
       - Channel emissions finalized in (channel, scope_hash, rule_id, subkey) order

       Determinism Boundaries:
       - Wire Format: Canonical CBOR (via echo_wasm_abi)
       - JSON: Forbidden for protocol; allowed only for debug/telemetry
       - Float Math: Optimistic IEEE 754 by default; det_fixed feature for strict consensus
       - Hashing: BLAKE3 with architecture-independent encoding (u64 LE lengths)

       Footprint Independence:
       - O(1) conflict detection via generation-stamped sets (GenSet)
       - Checks: node read/write, edge read/write, attachment, ports
       - O(m) reserve time where m = footprint size

       ---
       9. EXAMPLE WORKFLOW: FULL TICK EXECUTION

       1. ingest_intent(payload) → IngestDisposition
          ↓ (queued in inbox)

       2. dispatch_next_intent() → DispatchDisposition
          ↓ (pending edge consumed)

       3. apply(rule, scope) → ApplyResult
          ├─ matcher(store, scope) → bool
          ├─ compute_footprint(store, scope) → Footprint
          └─ enqueue PendingRewrite
          ↓

       4. commit_with_receipt() → (Snapshot, TickReceipt, Report)
          ├─ drain_for_tx() in (scope_hash, rule_id, nonce) order
          ├─ For each rewrite:
          │  ├─ try_reserve(footprint) → bool
          │  ├─ executor(store, scope)  [calls emit() via ScopedEmitter]
          │  └─ record outcome (Applied/Rejected)
          │
          ├─ Finalize MaterializationBus:
          │  ├─ Group emissions by (channel, EmitKey)
          │  ├─ Apply ChannelPolicy to each channel
          │  └─ Produce FinalizeReport (channels + errors)
          │
          ├─ Snapshot graph:
          │  ├─ canonical_state_hash()
          │  ├─ Compute commit_id (v2)
          │  └─ Store in tick_history
          │
          ├─ Compute tick patch:
          │  ├─ diff_state() against previous
          │  └─ Generate WarpTickPatchV1
          │
          └─ Return to caller
          ↓

       5. receive_finalized(channels) @ MaterializationPort
          ├─ Update replay cache
          └─ Queue frames for subscribed channels

       ---
       10. FILE SUMMARY TABLE

       ┌──────────────────┬───────┬───────────────────────────────────────────────────┐
       │       File       │ Lines │                      Purpose                      │
       ├──────────────────┼───────┼───────────────────────────────────────────────────┤
       │ engine_impl.rs   │ 1806  │ Core rewrite engine, builder, tick execution      │
       ├──────────────────┼───────┼───────────────────────────────────────────────────┤
       │ tick_patch.rs    │ 1694  │ Delta operations and patch types (Paper III)      │
       ├──────────────────┼───────┼───────────────────────────────────────────────────┤
       │ scheduler.rs     │ 1304  │ Radix scheduler + footprint independence checks   │
       ├──────────────────┼───────┼───────────────────────────────────────────────────┤
       │ graph.rs         │ 568   │ In-memory graph store (nodes, edges, attachments) │
       ├──────────────────┼───────┼───────────────────────────────────────────────────┤
       │ snapshot.rs      │ 350   │ State snapshots and hash computation              │
       ├──────────────────┼───────┼───────────────────────────────────────────────────┤
       │ attachment.rs    │ 403   │ Polymorphic payload system (Atom, Descend)        │
       ├──────────────────┼───────┼───────────────────────────────────────────────────┤
       │ payload.rs       │ 304   │ Motion atom encoding/decoding                     │
       ├──────────────────┼───────┼───────────────────────────────────────────────────┤
       │ receipt.rs       │ 241   │ Tick receipts (accepted/rejected outcomes)        │
       ├──────────────────┼───────┼───────────────────────────────────────────────────┤
       │ footprint.rs     │ 250   │ Footprints and independence tracking              │
       ├──────────────────┼───────┼───────────────────────────────────────────────────┤
       │ inbox.rs         │ 210   │ Inbox management and intent queuing               │
       ├──────────────────┼───────┼───────────────────────────────────────────────────┤
       │ materialization/ │ ~60   │ Channel emissions, ports, frames                  │
       ├──────────────────┼───────┼───────────────────────────────────────────────────┤
       │ warp_state.rs    │ 116   │ Multi-instance WARP state (Stage B1)              │
       ├──────────────────┼───────┼───────────────────────────────────────────────────┤
       │ rule.rs          │ 84    │ Rewrite rule definitions                          │
       ├──────────────────┼───────┼───────────────────────────────────────────────────┤
       │ ident.rs         │ 162   │ Identifier types and hashing                      │
       └──────────────────┴───────┴───────────────────────────────────────────────────┘
       ---
       This is a sophisticated, determinism-first engine architecture with clear separation between execution (rules +
       scheduler), storage (GraphStore), and output (materialization). The key innovation is the order-independent
       materialization system backed by deterministic ordering of emissions.

  ⎿  Done (23 tool uses · 85.5k tokens · 1m 21s)

⏺ Explore(Explore WSC implementation)
  ⎿  Prompt:
       Explore the WSC (WARP Snapshot Container) implementation in the codebase. Look for:
       1. WscFile, WarpView, or similar snapshot-related code
       2. How snapshots are read/written
       3. Any existing zero-copy or mmap infrastructure
       4. Segment or blob arena handling

       Search in crates/warp-core and any related locations. This is research only - do not modify any code.
  ⎿  ❯ Explore the WSC (WARP Snapshot Container) implementation in the codebase. Look for:
       1. WscFile, WarpView, or similar snapshot-related code
       2. How snapshots are read/written
       3. Any existing zero-copy or mmap infrastructure
       4. Segment or blob arena handling

       Search in crates/warp-core and any related locations. This is research only - do not modify any code.
  ⎿  Search(pattern: "**/warp-core/**/*.rs")

  ⎿  Search(pattern: "WscFile|WarpView|snapshot", type: "rs")

  ⎿  Search(pattern: "zero.copy|mmap|MemoryMap", type: "rs")

  ⎿  Search(pattern: "**/warp-core/Cargo.toml")

  ⎿    ⎿  Found 0 files

  ⎿    ⎿  Found 0 files

  ⎿    ⎿  Found 1 file
          /Users/james/git/echo/crates/warp-core/Cargo.toml
  ⎿    ⎿  Found 81 files
          /Users/james/git/echo/crates/warp-core/build.rs
          /Users/james/git/echo/crates/warp-core/src/footprint.rs
          /Users/james/git/echo/crates/warp-core/src/rule.rs
          /Users/james/git/echo/crates/warp-core/src/sandbox.rs
          /Users/james/git/echo/crates/warp-core/tests/atom_payload_digest_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/footprint_independence_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/mat4_mul_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/math_additional_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/math_convenience_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/nan_exhaustive_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/snapshot_reachability_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/vec3_ops_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/math_rotation_tests.rs
          /Users/james/git/echo/crates/warp-core/src/math/mat4.rs
          /Users/james/git/echo/crates/warp-core/src/math/trig_lut.rs
          /Users/james/git/echo/crates/warp-core/src/attachment.rs
          /Users/james/git/echo/crates/warp-core/src/cmd.rs
          /Users/james/git/echo/crates/warp-core/src/record.rs
          /Users/james/git/echo/crates/warp-core/src/serializable.rs
          /Users/james/git/echo/crates/warp-core/src/snapshot.rs
          /Users/james/git/echo/crates/warp-core/src/tx.rs
          /Users/james/git/echo/crates/warp-core/src/warp_state.rs
          /Users/james/git/echo/crates/warp-core/tests/determinism_audit.rs
          /Users/james/git/echo/crates/warp-core/tests/ledger_tests.rs
          /Users/james/git/echo/crates/warp-core/src/bin/gen_sin_qtr_lut.rs
          /Users/james/git/echo/crates/warp-core/src/constants.rs
          /Users/james/git/echo/crates/warp-core/src/inbox.rs
          /Users/james/git/echo/crates/warp-core/src/math/fixed_q32_32.rs
          /Users/james/git/echo/crates/warp-core/src/math/mod.rs
          /Users/james/git/echo/crates/warp-core/src/math/prng.rs
          /Users/james/git/echo/crates/warp-core/src/math/quat.rs
          /Users/james/git/echo/crates/warp-core/src/math/scalar.rs
          /Users/james/git/echo/crates/warp-core/src/math/trig.rs
          /Users/james/git/echo/crates/warp-core/src/math/vec3.rs
          /Users/james/git/echo/crates/warp-core/src/payload.rs
          /Users/james/git/echo/crates/warp-core/src/receipt.rs
          /Users/james/git/echo/crates/warp-core/src/scheduler.rs
          /Users/james/git/echo/crates/warp-core/src/telemetry.rs
          /Users/james/git/echo/crates/warp-core/src/tick_patch.rs
          /Users/james/git/echo/crates/warp-core/tests/determinism_policy_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/deterministic_sin_cos_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/dfix64_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/dispatch_inbox.rs
          /Users/james/git/echo/crates/warp-core/tests/dpo_concurrency_litmus.rs
          /Users/james/git/echo/crates/warp-core/tests/duplicate_rule_registration_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/engine_motion_negative_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/engine_motion_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/inbox.rs
          /Users/james/git/echo/crates/warp-core/tests/math_scalar_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/math_validation.rs
          /Users/james/git/echo/crates/warp-core/tests/permutation_commute_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/property_commute_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/proptest_seed_pinning.rs
          /Users/james/git/echo/crates/warp-core/tests/reserve_gate_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/tick_receipt_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/tx_lifecycle_tests.rs
          /Users/james/git/echo/crates/warp-core/src/fixed.rs
          /Users/james/git/echo/crates/warp-core/src/graph.rs
          /Users/james/git/echo/crates/warp-core/src/ident.rs
          /Users/james/git/echo/crates/warp-core/src/wsc/build.rs
          /Users/james/git/echo/crates/warp-core/src/wsc/mod.rs
          /Users/james/git/echo/crates/warp-core/src/wsc/read.rs
          /Users/james/git/echo/crates/warp-core/src/wsc/types.rs
          /Users/james/git/echo/crates/warp-core/src/wsc/validate.rs
          /Users/james/git/echo/crates/warp-core/src/wsc/view.rs
          /Users/james/git/echo/crates/warp-core/src/wsc/write.rs
          /Users/james/git/echo/crates/warp-core/src/lib.rs
          /Users/james/git/echo/crates/warp-core/src/materialization/emit_key.rs
          /Users/james/git/echo/crates/warp-core/src/materialization/port.rs
          /Users/james/git/echo/crates/warp-core/src/materialization/emission_port.rs
          /Users/james/git/echo/crates/warp-core/src/materialization/channel.rs
          /Users/james/git/echo/crates/warp-core/src/materialization/reduce_op.rs
          /Users/james/git/echo/crates/warp-core/src/materialization/scoped_emitter.rs
          /Users/james/git/echo/crates/warp-core/tests/materialization_engine_integration.rs
          /Users/james/git/echo/crates/warp-core/src/materialization/mod.rs
          /Users/james/git/echo/crates/warp-core/src/engine_impl.rs
                    /Users/james/git/echo/crates/warp-core/src/materialization/bus.rs
          /Users/james/git/echo/crates/warp-core/tests/common/mod.rs
          /Users/james/git/echo/crates/warp-core/tests/materialization_spec_police.rs
          /Users/james/git/echo/crates/warp-core/tests/materialization_determinism.rs
          /Users/james/git/echo/crates/warp-core/src/materialization/frame.rs
  ⎿  Read(/Users/james/git/echo/crates/warp-core/src/wsc)

  ⎿  Read(/Users/james/git/echo/crates/warp-core/src/wsc/mod.rs)

  ⎿  Read(/Users/james/git/echo/crates/warp-core/src/wsc/types.rs)

  ⎿  Read(/Users/james/git/echo/crates/warp-core/src/wsc/view.rs)

  ⎿  Error: EISDIR: illegal operation on a directory, read

  ⎿  Read 63 lines

  ⎿  Read 301 lines

  ⎿  Read 499 lines

  ⎿  Read(/Users/james/git/echo/crates/warp-core/src/wsc/read.rs)

  ⎿  Read(/Users/james/git/echo/crates/warp-core/src/wsc/write.rs)

  ⎿  Read(/Users/james/git/echo/crates/warp-core/src/wsc/build.rs)

  ⎿  Read 312 lines

  ⎿  Read 302 lines

  ⎿  Read 581 lines

  ⎿  Read(/Users/james/git/echo/crates/warp-core/src/wsc/validate.rs)

  ⎿  Read(/Users/james/git/echo/crates/warp-core/tests/snapshot_reachability_tests.rs)

  ⎿  Read(/Users/james/git/echo/crates/warp-core/Cargo.toml)

  ⎿  Read 656 lines

  ⎿  Read 72 lines

  ⎿  Read 53 lines

  ⎿  Search(pattern: "blob|arena|segment", path: "/Users/james/git/echo/crates/warp-core/src", type: "rs")

  ⎿  Read(/Users/james/git/echo/crates/warp-core/src/attachment.rs · lines 1-200)

  ⎿  Search(pattern: "mmap|MemoryMap|memory.map", path: "/Users/james/git/echo/crates", type: "rs")

  ⎿  Read 200 lines

  ⎿    ⎿  Found 0 files

  ⎿    ⎿  Found 0 files

  ⎿  Read(/Users/james/git/echo/crates/warp-core/src/graph.rs · lines 1-250)

  ⎿  Read(/Users/james/git/echo/crates/warp-core/src/snapshot.rs · lines 1-150)

  ⎿  Bash(grep -r "WscFile\|WarpView" /Users/james/git/echo/crates/warp-core --include="*.rs" | head -20)
     Waiting…
  ⎿  Read 250 lines

  ⎿  Read 150 lines

  ⎿  /Users/james/git/echo/crates/warp-core/src/wsc/mod.rs://! use warp_core::wsc::{WscFile, validate_wsc};
     /Users/james/git/echo/crates/warp-core/src/wsc/mod.rs://! let file = WscFile::open("state.wsc")?;
     /Users/james/git/echo/crates/warp-core/src/wsc/mod.rs:pub use view::{AttachmentRef, WarpView, WscFile};
     /Users/james/git/echo/crates/warp-core/src/wsc/validate.rs:use super::view::WscFile;
     /Users/james/git/echo/crates/warp-core/src/wsc/validate.rs:pub fn validate_wsc(file: &WscFile) -> Result<(), ReadError>
      {
     /Users/james/git/echo/crates/warp-core/src/wsc/validate.rs:fn validate_warp_view(view: &super::view::WarpView<'_>) ->
     Result<(), ReadError> {
     /Users/james/git/echo/crates/warp-core/src/wsc/validate.rs:        let file = WscFile::from_bytes(bytes).unwrap();
     /Users/james/git/echo/crates/warp-core/src/wsc/validate.rs:        let file = WscFile::from_bytes(bytes).unwrap();
     /Users/james/git/echo/crates/warp-core/src/wsc/validate.rs:        let file = WscFile::from_bytes(bytes).unwrap();
     /Users/james/git/echo/crates/warp-core/src/wsc/validate.rs:        let file = WscFile::from_bytes(bytes).unwrap();
     /Users/james/git/echo/crates/warp-core/src/wsc/validate.rs:        let file = WscFile::from_bytes(bytes).unwrap();
     /Users/james/git/echo/crates/warp-core/src/wsc/validate.rs:        let file = WscFile::from_bytes(bytes).unwrap();
     /Users/james/git/echo/crates/warp-core/src/wsc/validate.rs:        let file = WscFile::from_bytes(bytes).unwrap();
     /Users/james/git/echo/crates/warp-core/src/wsc/validate.rs:        let file = WscFile::from_bytes(bytes).unwrap();
     /Users/james/git/echo/crates/warp-core/src/wsc/validate.rs:        let file = WscFile::from_bytes(bytes).unwrap();
     /Users/james/git/echo/crates/warp-core/src/wsc/validate.rs:        let file = WscFile::from_bytes(bytes).unwrap();
     /Users/james/git/echo/crates/warp-core/src/wsc/validate.rs:        let file = WscFile::from_bytes(bytes).unwrap();
     /Users/james/git/echo/crates/warp-core/src/wsc/validate.rs:        let file = WscFile::from_bytes(bytes).unwrap();
     /Users/james/git/echo/crates/warp-core/src/wsc/validate.rs:        let file = WscFile::from_bytes(bytes).unwrap();
     /Users/james/git/echo/crates/warp-core/src/wsc/validate.rs:        let file = WscFile::from_bytes(bytes).unwrap();
  ⎿  Read(/Users/james/git/echo/crates/warp-core/src/materialization/frame.rs · lines 1-300)

  ⎿  Bash(head -100 /Users/james/git/echo/crates/warp-core/src/materialization/bus.rs)
     Waiting…
  ⎿  Read 255 lines

  ⎿  // SPDX-License-Identifier: Apache-2.0
     // © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
     //! Internal materialization bus for tick-scoped channel emissions.
     //!
     //! The [`MaterializationBus`] collects emissions from rewrite rules during a tick.
     //! Emissions are stored in an order-independent manner (keyed by [`EmitKey`]),
     //! then finalized post-commit according to each channel's policy.
     //!
     //! # Order Independence
     //!
     //! The bus uses `BTreeMap<ChannelId, BTreeMap<EmitKey, Vec<u8>>>` internally.
     //! This ensures that:
     //! 1. Insertion order doesn't affect the final result
     //! 2. Finalization iterates in deterministic (canonical) order
     //! 3. Confluence is preserved regardless of rewrite execution order
     //!
     //! # Usage
     //!
     //! Rules emit via `bus.emit(channel, emit_key, data)`. After commit, the engine
     //! calls `bus.finalize()` to resolve each channel according to its policy and
     //! produce the final output.

     use std::cell::RefCell;
     use std::collections::btree_map::Entry;
     use std::collections::BTreeMap;

     use super::channel::{ChannelConflict, ChannelId, ChannelPolicy, MaterializationErrorKind};
     use super::emit_key::EmitKey;

     /// Error returned when the same `(channel, EmitKey)` pair is emitted twice.
     ///
     /// This is a structural invariant: even if the payloads are identical, duplicate
     /// emissions indicate a bug in the rule (e.g., iterating a non-deterministic source
     /// like an unordered container without proper subkey differentiation).
     ///
     /// # Why Reject Identical Payloads?
     ///
     /// Allowing "identical payload = OK" encourages sloppy code that emits redundantly.
     /// Then someone changes a field and tests fail mysteriously. Rejecting always forces
     /// rule authors to think: "Am I iterating deterministically? Do I need unique subkeys?"
     #[derive(Debug, Clone, PartialEq, Eq)]
     pub struct DuplicateEmission {
         /// The channel that received the duplicate.
         pub channel: ChannelId,
         /// The key that was duplicated.
         pub key: EmitKey,
     }

     impl core::fmt::Display for DuplicateEmission {
         fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
             write!(
                 f,
                 "duplicate emission for channel {:?} key {:?}",
                 self.channel, self.key
             )
         }
     }

     impl std::error::Error for DuplicateEmission {}

     /// Internal materialization bus for collecting and finalizing channel emissions.
     ///
     /// This is tick-scoped: emissions accumulate during a tick, then `finalize()` is
     /// called post-commit to produce output. The bus is cleared after finalization.
     ///
     /// # Thread Safety
     ///
     /// `MaterializationBus` uses [`RefCell`] for interior mutability and is intentionally
     /// **not thread-safe** (`!Sync`). This is by design: emissions occur within a single
     /// tick's execution context, which is single-threaded. The rewrite engine processes
     /// rules sequentially within a tick, so no synchronization is needed.
     ///
     /// If parallel emission were ever required (e.g., concurrent rule execution), the
     /// design would need to change to use `RwLock<BTreeMap<...>>` or similar.
     #[derive(Debug, Default)]
     pub struct MaterializationBus {
         /// Pending emissions: `channel -> (emit_key -> data)`.
         /// Uses `BTreeMap` for deterministic iteration order.
         pending: RefCell<BTreeMap<ChannelId, BTreeMap<EmitKey, Vec<u8>>>>,

         /// Channel policies (looked up during finalization).
         policies: BTreeMap<ChannelId, ChannelPolicy>,
     }

     /// Result of finalizing a single channel.
     #[derive(Debug, Clone)]
     pub struct FinalizedChannel {
         /// The channel that was finalized.
         pub channel: ChannelId,
         /// The finalized data (format depends on policy).
         pub data: Vec<u8>,
     }

     /// Report from finalizing all channels.
     ///
     /// Unlike a `Result`, this type always succeeds. Channels that finalize
     /// successfully appear in `channels`; channels that fail (e.g., `StrictSingle`
     /// conflicts) appear in `errors`. This design ensures:
     ///
     /// 1. **No data loss**: A failing channel doesn't erase other channels' outputs
  ⎿  Read(/Users/james/git/echo/docs/adr/ADR-0007-BOAW-Storage.md)

  ⎿  Read 1129 lines

  ⎿  Response:
       Perfect! Now I have a comprehensive understanding. Let me create the final research report:

       WSC (WARP Snapshot Container) Implementation Research Report

       Executive Summary

       The codebase contains a mature, production-ready WSC (Write-Streaming Columnar) snapshot format implementation
       located in /Users/james/git/echo/crates/warp-core/src/wsc/. The system is designed for deterministic
       serialization and zero-copy deserialization of immutable WARP graph state with explicit infrastructure for
       memory-mapped file access and columnar data organization.

       ---
       1. WSC Format Architecture

       File Structure (128-byte header + directory + sections)

       Location: /Users/james/git/echo/crates/warp-core/src/wsc/types.rs

       The format is built on fixed-size, repr(C) rows using bytemuck::Pod for zero-copy transmutation:
       ┌──────────────┬───────────┬──────────────────────────────────────────────────────────────────────────────┐
       │  Component   │   Size    │                                   Purpose                                    │
       ├──────────────┼───────────┼──────────────────────────────────────────────────────────────────────────────┤
       │ WscHeader    │ 128 bytes │ Magic (v1), schema hash, tick, warp count, directory offset                  │
       ├──────────────┼───────────┼──────────────────────────────────────────────────────────────────────────────┤
       │ NodeRow      │ 64 bytes  │ Fixed-size node: node_id (32B) + node_type (32B)                             │
       ├──────────────┼───────────┼──────────────────────────────────────────────────────────────────────────────┤
       │ EdgeRow      │ 128 bytes │ Fixed-size edge: edge_id + from_node_id + to_node_id + edge_type             │
       ├──────────────┼───────────┼──────────────────────────────────────────────────────────────────────────────┤
       │ Range        │ 16 bytes  │ Index descriptor: start_le + len_le (u64 each)                               │
       ├──────────────┼───────────┼──────────────────────────────────────────────────────────────────────────────┤
       │ OutEdgeRef   │ 40 bytes  │ Outbound edge reference: edge_ix_le + edge_id                                │
       ├──────────────┼───────────┼──────────────────────────────────────────────────────────────────────────────┤
       │ AttRow       │ 56 bytes  │ Attachment row: tag (1B) + reserved (7B) + type/warp (32B) + blob offset/len │
       ├──────────────┼───────────┼──────────────────────────────────────────────────────────────────────────────┤
       │ WarpDirEntry │ 184 bytes │ Directory entry per WARP instance with all section offsets                   │
       └──────────────┴───────────┴──────────────────────────────────────────────────────────────────────────────┘
       Key design properties:
       - All fields use little-endian byte order for cross-platform determinism
       - All sections are 8-byte aligned for SIMD-friendly access
       - Blobs are stored contiguously in an arena with (offset, length) references
       - Index tables use Range structures to slice into data tables without copying

       Attachment Types (Tagged Union)

       Two attachment types via AttRow.tag:
       - TAG_ATOM (1): Typed payload (type_id + blob location)
       - TAG_DESCEND (2): Child WARP reference (WarpId, blob fields unused)

       ---
       2. Zero-Copy Read/Write Infrastructure

       Writing: write_wsc_one_warp (deterministic serialization)

       Location: /Users/james/git/echo/crates/warp-core/src/wsc/write.rs

       Input: OneWarpInput struct containing:
       - Pre-sorted nodes/edges (canonical order)
       - Index tables (out_index, node_atts_index, edge_atts_index)
       - Blob arena (8-byte aligned)

       Process:
       1. Pre-calculate all section sizes and offsets
       2. Allocate buffer with capacity
       3. Write header with magic + metadata
       4. Write WARP directory entry with all offsets
       5. Sequentially write each section with padding to 8-byte boundaries
       6. Blobs written at end as concatenated arena

       Determinism guarantees:
       - Input ordering is canonical (nodes by NodeId, edges by EdgeId, out_edges sorted per node)
       - Little-endian encoding is architecture-independent
       - Identical input produces identical bytes every run

       Reading: WscFile & WarpView (zero-copy views)

       Location: /Users/james/git/echo/crates/warp-core/src/wsc/view.rs

       pub struct WscFile {
           data: Vec<u8>,  // Entire file in memory (mmap not yet implemented)
       }

       pub struct WarpView<'a> {
           data: &'a [u8],
           entry: &'a WarpDirEntry,
           nodes: &'a [NodeRow],      // Direct slice - no copy
           edges: &'a [EdgeRow],      // Direct slice - no copy
           out_index: &'a [Range],    // Index table
           out_edges: &'a [OutEdgeRef], // Concatenated edge references
           node_atts_index: &'a [Range],
           node_atts: &'a [AttRow],
           edge_atts_index: &'a [Range],
           edge_atts: &'a [AttRow],
           blobs: &'a [u8],           // Blob arena
       }

       Key features:
       - No copying: Uses bytemuck::try_cast_slice() for safe type punning
       - Lifetime-bound: Borrows prevent data invalidation
       - Binary search indexing: node_ix() and edge_ix() use binary search on sorted tables
       - Range-based access: Out-edges and attachments accessed via Range slices

       Public API:
       impl<'a> WarpView<'a> {
           pub fn out_edges_for_node(&self, node_ix: usize) -> &[OutEdgeRef]
           pub fn node_attachments(&self, node_ix: usize) -> &[AttRow]
           pub fn edge_attachments(&self, edge_ix: usize) -> &[AttRow]
           pub fn blob_for_attachment(&self, att: &AttRow) -> Option<&[u8]>
       }

       Reading Infrastructure: read.rs

       Location: /Users/james/git/echo/crates/warp-core/src/wsc/read.rs

       Error types:
       pub enum ReadError {
           Io(io::Error),
           FileTooSmall { size, minimum },
           InvalidMagic { expected, actual },
           SectionOutOfBounds { name, offset, length, file_size },
           WarpIndexOutOfBounds { index, count },
           AlignmentViolation { name, offset, alignment },
           IndexRangeOutOfBounds { ... },  // Critical: catches corrupted index tables
           InvalidAttachmentTag { tag, index },
           BlobOutOfBounds { offset, length, blob_size },
           OrderingViolation { kind, index },  // Nodes/edges must be sorted
           NonZeroReservedBytes { field, index },
           NonAtomHasBlobFields { index, blob_off, blob_len },
           MissingRoot { root },  // Root must exist in nodes array
       }

       Key functions:
       pub fn validate_header(data: &[u8]) -> Result<&WscHeader, ReadError>
       pub fn read_slice<'a, T: Pod>(data: &'a [u8], offset: u64, count: u64, name: &'static str)
           -> Result<&'a [T], ReadError>
       pub fn read_bytes<'a>(data: &'a [u8], offset: u64, length: u64, name: &'static str)
           -> Result<&'a [u8], ReadError>

       ---
       3. Validation & Integrity

       Location: /Users/james/git/echo/crates/warp-core/src/wsc/validate.rs

       The validate_wsc() function performs comprehensive structural validation:

       1. Index bounds checking: All Range references stay within data table boundaries
       2. Ordering validation: Nodes and edges must be sorted by ID
       3. Attachment validation: Tags must be ATOM (1) or DESCEND (2)
       4. Root node validation: root_node_id must exist in nodes array (or be zero for empty)
       5. Blob bounds: All blob references within the arena
       6. Reserved bytes: Ensure all reserved fields are zero
       7. Out-edge references: Verify all edge indices point to valid edges

       Critical safeguard: The validation catches corrupted index tables that accessors would otherwise mask with
       empty slices.

       ---
       4. Build Infrastructure: GraphStore → WSC

       Location: /Users/james/git/echo/crates/warp-core/src/wsc/build.rs

       The build_one_warp_input() function converts in-memory GraphStore to OneWarpInput in canonical order:

       Canonical ordering rules:
       - Nodes: BTreeMap iteration (already sorted by NodeId)
       - Edges: Globally sorted by EdgeId (via explicit sort)
       - Outbound edges: Sorted by EdgeId within each node's bucket
       - Attachments: Follow same ordering as their owners

       Determinism tests included:
       - "Repeated serialization produces identical bytes" (100x runs)
       - "Different insertion orders produce identical bytes" (proves canonicalization)
       - "Attachments serialized deterministically" regardless of build order

       Blob handling:
       - Attachments are collected with 8-byte alignment (align8_vec())
       - Blobs stored as contiguous arena in order of collection
       - Offset/length references stored in AttRow

       ---
       5. Existing Infrastructure for Zero-Copy/mmap

       Current State

       - No explicit mmap implementation yet
       - Code comments state: "For very large files, consider using memory-mapped IO instead (not yet implemented)"
       - File data currently read entirely into Vec<u8>

       Future-Ready Design

       The architecture is explicitly designed for mmap:

       1. Alignment guarantees: All sections are 8-byte aligned
       2. Pod types: All row types are bytemuck::Pod (byte-identical layout)
       3. Lifetime bounds: WarpView<'a> borrows don't hold file handles
       4. No mutation: Snapshot is immutable by design
       5. Path independence: Reader doesn't care if data comes from memory or mmap

       Path to mmap:
       // Pseudo-code: straightforward substitution
       pub fn open_mmap(path: impl AsRef<Path>) -> Result<Self, ReadError> {
           let file = std::fs::File::open(path)?;
           let mmap = unsafe { Mmap::map(&file)? };  // or memmap crate
           Self::from_bytes(mmap.to_vec())  // or bind to mmap lifetime
       }

       ---
       6. Blob Arena Handling

       Design:
       - Single contiguous blob section at end of file
       - Attachments reference via (offset: u64, length: u64) in AttRow
       - Multiple attachments can share blob data if ranges don't overlap
       - Blobs are 8-byte aligned for SIMD

       Access pattern:
       pub fn blob_for_attachment(&self, att: &AttRow) -> Option<&[u8]> {
           if !att.is_atom() { return None; }
           let off = att.blob_off() as usize;
           let len = att.blob_len() as usize;
           self.blobs.get(off..off + len)
       }

       Future enhancement (from ADR): Segment-level chunking for COW/structural sharing:
       - Individual blob segments become content-addressed
       - Commit builder references only changed segments
       - Massive savings for append-only workloads

       ---
       7. Related Snapshot Infrastructure

       Snapshot Hash Computation

       Location: /Users/james/git/echo/crates/warp-core/src/snapshot.rs

       The system computes deterministic state_root hash via BLAKE3:

       1. Performs reachability BFS (following skeleton edges + Descend attachments)
       2. Encodes reachable state in canonical order (by WarpId, then nodes by NodeId)
       3. For each node: encode ID, type, then sorted outbound edges
       4. Include attachment values (presence tag + type + length + bytes)
       5. Hash the canonical byte stream

       Includes: Snapshot struct with commit hash, parents, plan/decision/patch digests

       Materialization Infrastructure

       Location: /Users/james/git/echo/crates/warp-core/src/materialization/frame.rs

       Parallel binary frame format ("MBUS"):
       - Magic: "MBUS" (4 bytes)
       - Version: v1 (2 bytes LE)
       - Payload length (4 bytes LE)
       - Channel ID (32 bytes) + data

       Uses similar design principles: deterministic, binary, length-prefixed.

       ---
       8. BOAW Architecture (ADR-0007)

       Location: /Users/james/git/echo/docs/adr/ADR-0007-BOAW-Storage.md

       The accepted architecture road map includes:

       1. Immutable Base: WSC-backed snapshots remain the canonical format
       2. TickDelta Overlay: Rewrites emit ops into thread-local deltas (no shared mutation)
       3. Deterministic Admission: Only independent rewrites admitted per tick
       4. Lockless Parallel Execute: Workers execute against read-only snapshot
       5. Deterministic Merge: Thread-local deltas canonically merged
       6. Canonical Commit: Apply patch → new snapshot → compute hashes
       7. Collapse/Merge: Multi-parent merges with typed rules
       8. Privacy: Mind mode forbids sensitive atoms in ledger

       Segment-level COW (Section 5.5):
       - Snapshots stored as content-addressed segments (1–4MB blocks)
       - Commit references segment hashes for: nodes, edges, indices, attachments, blobs
       - Only changed segments written; unchanged segments reused

       ---
       9. Test Coverage

       Snapshot & reachability tests:
       - /Users/james/git/echo/crates/warp-core/tests/snapshot_reachability_tests.rs
         - Unreachable nodes don't affect hash
         - Reachable edges do affect hash

       WSC format tests (integrated into modules):
       - Header validation
       - Magic number checks
       - Alignment violations
       - Index range bounds
       - Ordering violations
       - Attachment tag validation
       - Blob reference validation
       - Root node validation
       - Roundtrip (write → read → verify)

       ---
       10. File Manifest
       ┌─────────────────────┬───────┬─────────────────────────────────────┐
       │        File         │ Lines │               Purpose               │
       ├─────────────────────┼───────┼─────────────────────────────────────┤
       │ src/wsc/mod.rs      │ 63    │ Module docs + re-exports            │
       ├─────────────────────┼───────┼─────────────────────────────────────┤
       │ src/wsc/types.rs    │ 301   │ Row type definitions (Pod)          │
       ├─────────────────────┼───────┼─────────────────────────────────────┤
       │ src/wsc/view.rs     │ 499   │ WscFile, WarpView zero-copy readers │
       ├─────────────────────┼───────┼─────────────────────────────────────┤
       │ src/wsc/read.rs     │ 312   │ Low-level read primitives + errors  │
       ├─────────────────────┼───────┼─────────────────────────────────────┤
       │ src/wsc/write.rs    │ 302   │ Deterministic serialization         │
       ├─────────────────────┼───────┼─────────────────────────────────────┤
       │ src/wsc/build.rs    │ 581   │ GraphStore → OneWarpInput builder   │
       ├─────────────────────┼───────┼─────────────────────────────────────┤
       │ src/wsc/validate.rs │ 656   │ Comprehensive file validation       │
       └─────────────────────┴───────┴─────────────────────────────────────┘
       ---
       Key Takeaways

       Strengths

       1. Deterministic by design: Input order doesn't matter; output bytes are identical
       2. Zero-copy ready: Bytemuck Pod types enable safe pointer casting
       3. Validation-first: Catches corruption before use (not in accessors)
       4. Columnar + blob arena: SIMD-friendly, natural for batch operations
       5. Alignment aware: Explicit 8-byte boundaries throughout
       6. Immutable semantics: Snapshot ownership prevents accidental mutation
       7. Well-architected: Clear separation of read/write/validate/build concerns

       Future Work

       1. mmap integration: Straightforward given Pod guarantees
       2. Segment sharing: ADR-0007 envisions content-addressed segments
       3. TickDelta overlay: Complement snapshots with deterministic delta merging
       4. Collapse/merge: Type-driven multi-parent reconciliation
       5. Privacy enforcement: Mind mode gates and claim records

       ---
       Conclusion

       The WSC implementation is production-ready for deterministic snapshot I/O with explicit infrastructure for
       zero-copy access and future memory-mapping. The columnar design with blob arena, combined with comprehensive
       validation and deterministic ordering, makes it ideal for both research (proving determinism) and production
       (high-performance state materialization). ADR-0007 charts a clear path to leveraging this foundation for
       lockless parallel execution via thread-local deltas and canonical commits.
  ⎿  Done (23 tool uses · 109.3k tokens · 1m 11s)

⏺ Explore(Explore footprint infrastructure)
  ⎿  Prompt:
       Search for any existing footprint, independence checking, or scheduling infrastructure in the codebase. Look
       for:
       1. Footprint struct or similar conflict detection code
       2. Any sharding or virtual shard logic
       3. Admission control or dependency checking
       4. Worker queue or parallel execution infrastructure

       Search across all crates. This is research only - do not modify any code.
  ⎿  ❯ Search for any existing footprint, independence checking, or scheduling infrastructure in the codebase. Look for:
       1. Footprint struct or similar conflict detection code
       2. Any sharding or virtual shard logic
       3. Admission control or dependency checking
       4. Worker queue or parallel execution infrastructure

       Search across all crates. This is research only - do not modify any code.
  ⎿  Search(pattern: "footprint|Footprint", glob: "**/*.rs")

  ⎿  Search(pattern: "shard|Shard|virtual.*shard|sharding", glob: "**/*.rs")

  ⎿  Search(pattern: "admission|dependency.*check|conflict.*detect", glob: "**/*.rs")

  ⎿  Search(pattern: "**/src/lib.rs")

  ⎿    ⎿  Found 1 file
          crates/warp-benches/src/bin/sim_sharded_store.rs
  ⎿    ⎿  Found 5 files
          crates/warp-core/src/telemetry.rs
          crates/warp-core/src/scheduler.rs
          crates/warp-core/src/inbox.rs
          crates/warp-core/tests/footprint_independence_tests.rs
          crates/warp-core/src/rule.rs
  ⎿    ⎿  Found 18 files
          crates/warp-core/src/engine_impl.rs
          crates/warp-core/src/materialization/channel.rs
          crates/warp-core/src/lib.rs
          crates/echo-dind-tests/src/rules.rs
          crates/warp-core/tests/tick_receipt_tests.rs
          crates/warp-core/tests/dpo_concurrency_litmus.rs
          crates/warp-core/tests/duplicate_rule_registration_tests.rs
          crates/warp-core/src/tick_patch.rs
          crates/warp-core/src/receipt.rs
          crates/warp-core/src/scheduler.rs
          crates/warp-core/src/inbox.rs
          crates/warp-benches/benches/scheduler_drain.rs
          crates/echo-dry-tests/src/demo_rules.rs
          crates/echo-dry-tests/src/rules.rs
          crates/warp-core/src/attachment.rs
          crates/warp-core/tests/footprint_independence_tests.rs
          crates/warp-core/src/rule.rs
          crates/warp-core/src/footprint.rs
  ⎿    ⎿  Found 16 files
          /Users/james/git/echo/crates/echo-app-core/src/lib.rs
          /Users/james/git/echo/crates/echo-config-fs/src/lib.rs
          /Users/james/git/echo/crates/echo-graph/src/lib.rs
          /Users/james/git/echo/crates/echo-session-client/src/lib.rs
          /Users/james/git/echo/crates/echo-session-proto/src/lib.rs
          /Users/james/git/echo/crates/warp-geom/src/lib.rs
          /Users/james/git/echo/specs/spec-000-rewrite/src/lib.rs
          /Users/james/git/echo/crates/echo-registry-api/src/lib.rs
          /Users/james/git/echo/crates/echo-wasm-bindings/src/lib.rs
          /Users/james/git/echo/crates/echo-dind-harness/src/lib.rs
          /Users/james/git/echo/crates/echo-dry-tests/src/lib.rs
          /Users/james/git/echo/crates/warp-ffi/src/lib.rs
          /Users/james/git/echo/crates/warp-wasm/src/lib.rs
          /Users/james/git/echo/crates/echo-dind-tests/src/lib.rs
          /Users/james/git/echo/crates/echo-wasm-abi/src/lib.rs
          /Users/james/git/echo/crates/warp-core/src/lib.rs
  ⎿  Search(pattern: "worker|Worker|queue|Queue|executor|Executor", glob: "**/*.rs", head_limit: 30)

  ⎿  Read(/Users/james/git/echo/crates/warp-core/src/footprint.rs)

  ⎿  Read(/Users/james/git/echo/crates/warp-core/tests/footprint_independence_tests.rs)

  ⎿  Read(/Users/james/git/echo/crates/warp-core/src/scheduler.rs)

  ⎿  Read 251 lines

  ⎿  Read 99 lines

  ⎿  Read 1305 lines

  ⎿    ⎿  Found 30 files
          crates/warp-benches/src/bin/sim_sharded_store.rs
          crates/warp-benches/src/bin/sim_parallel_executor.rs
          crates/warp-core/tests/materialization_determinism.rs
          crates/warp-core/tests/materialization_spec_police.rs
          crates/warp-core/src/engine_impl.rs
          crates/warp-core/src/materialization/emission_port.rs
          crates/warp-core/src/materialization/port.rs
          crates/warp-core/src/materialization/emit_key.rs
          crates/warp-core/src/lib.rs
          crates/warp-core/src/graph.rs
          crates/echo-dind-tests/src/rules.rs
          crates/echo-dind-harness/src/dind.rs
          crates/warp-wasm/src/lib.rs
          crates/warp-core/tests/reserve_gate_tests.rs
          crates/warp-core/tests/tick_receipt_tests.rs
          crates/warp-core/tests/permutation_commute_tests.rs
          crates/warp-core/tests/property_commute_tests.rs
          crates/warp-core/tests/dpo_concurrency_litmus.rs
          crates/warp-core/tests/duplicate_rule_registration_tests.rs
          crates/warp-core/tests/engine_motion_negative_tests.rs
          crates/warp-core/tests/dispatch_inbox.rs
          crates/warp-core/src/scheduler.rs
          crates/warp-core/src/inbox.rs
          crates/warp-benches/benches/scheduler_drain.rs
          crates/echo-dry-tests/src/demo_rules.rs
          crates/echo-dry-tests/src/lib.rs
          crates/echo-dry-tests/src/rules.rs
          crates/warp-core/src/snapshot.rs
          crates/warp-core/src/attachment.rs
          crates/warp-viewer/src/gpu.rs
  ⎿  Read(/Users/james/git/echo/crates/warp-benches/src/bin/sim_sharded_store.rs)

  ⎿  Read(/Users/james/git/echo/crates/warp-benches/src/bin/sim_parallel_executor.rs)

  ⎿  Search(pattern: "parallel|Parallel|concurrent|Concurrent|thread|Thread", glob: "**/*.rs", head_limit: 20)

  ⎿  Read 103 lines

  ⎿  Read 83 lines

  ⎿    ⎿  Found 12 files
          crates/warp-benches/src/bin/sim_sharded_store.rs
          crates/warp-benches/src/bin/sim_parallel_executor.rs
          crates/warp-core/src/materialization/bus.rs
          crates/warp-core/src/materialization/mod.rs
          crates/warp-core/src/materialization/emit_key.rs
          crates/warp-core/src/wsc/write.rs
          crates/warp-core/src/receipt.rs
          crates/echo-session-client/examples/publish_pulse.rs
          crates/warp-viewer/src/ui_effects.rs
          crates/warp-core/src/sandbox.rs
          crates/warp-benches/benches/scheduler_adversarial.rs
          crates/echo-session-client/src/lib.rs
  ⎿  Read(/Users/james/git/echo/crates/warp-core/src/inbox.rs · lines 1-100)

  ⎿  Search(pattern: "pending.*queue|work.*queue|task.*queue|dispatch", glob: "**/*.rs", head_limit: 15)

  ⎿  Read(/Users/james/git/echo/crates/warp-core/src/lib.rs · lines 1-150)

  ⎿  Read 100 lines

  ⎿  Read 141 lines

  ⎿    ⎿  Found 10 files
          crates/warp-core/src/engine_impl.rs
          crates/warp-core/src/materialization/port.rs
          crates/echo-dind-tests/src/lib.rs
          crates/echo-dind-harness/src/dind.rs
          crates/warp-wasm/src/lib.rs
          crates/warp-core/tests/dispatch_inbox.rs
          crates/warp-core/src/payload.rs
          crates/warp-core/src/scheduler.rs
          crates/warp-core/src/inbox.rs
          crates/echo-dry-tests/src/engine.rs
  ⎿  Read(/Users/james/git/echo/crates/warp-core/src/engine_impl.rs · lines 1-200)

  ⎿  Search(pattern: "GenSet|ActiveFootprint|RadixScheduler|LegacyScheduler", glob: "**/*.rs")

  ⎿  Read 200 lines

  ⎿    ⎿  Found 2 files
          crates/warp-core/src/engine_impl.rs
          crates/warp-core/src/scheduler.rs
  ⎿  Search(pattern: "**/materialization/*.rs")

  ⎿  Read(/Users/james/git/echo/crates/warp-core/src/materialization/mod.rs · lines 1-150)

  ⎿  Read 65 lines

  ⎿    ⎿  Found 9 files
          /Users/james/git/echo/crates/warp-core/src/materialization/emit_key.rs
          /Users/james/git/echo/crates/warp-core/src/materialization/port.rs
          /Users/james/git/echo/crates/warp-core/src/materialization/emission_port.rs
          /Users/james/git/echo/crates/warp-core/src/materialization/channel.rs
          /Users/james/git/echo/crates/warp-core/src/materialization/reduce_op.rs
          /Users/james/git/echo/crates/warp-core/src/materialization/scoped_emitter.rs
          /Users/james/git/echo/crates/warp-core/src/materialization/mod.rs
          /Users/james/git/echo/crates/warp-core/src/materialization/bus.rs
          /Users/james/git/echo/crates/warp-core/src/materialization/frame.rs
  ⎿  Bash(find /Users/james/git/echo -name "*.rs" -type f | xargs grep -l "reserve\|conflict\|dispatch" | head -20)
     Waiting…
  ⎿  Read(/Users/james/git/echo/crates/warp-benches/benches/scheduler_drain.rs · lines 1-100)

  ⎿  Read 100 lines

  ⎿  /Users/james/git/echo/crates/echo-session-ws-gateway/src/main.rs
     /Users/james/git/echo/crates/echo-dry-tests/src/rules.rs
     /Users/james/git/echo/crates/echo-dry-tests/src/demo_rules.rs
     /Users/james/git/echo/crates/echo-dry-tests/src/engine.rs
     /Users/james/git/echo/crates/warp-core/tests/materialization_spec_police.rs
     /Users/james/git/echo/crates/warp-core/tests/reserve_gate_tests.rs
     /Users/james/git/echo/crates/warp-core/tests/tick_receipt_tests.rs
     /Users/james/git/echo/crates/warp-core/tests/duplicate_rule_registration_tests.rs
     /Users/james/git/echo/crates/warp-core/tests/materialization_determinism.rs
     /Users/james/git/echo/crates/warp-core/tests/footprint_independence_tests.rs
     /Users/james/git/echo/crates/warp-core/tests/dispatch_inbox.rs
     /Users/james/git/echo/crates/warp-core/tests/nan_exhaustive_tests.rs
     /Users/james/git/echo/crates/warp-core/tests/dpo_concurrency_litmus.rs
     /Users/james/git/echo/crates/warp-core/tests/materialization_engine_integration.rs
     /Users/james/git/echo/crates/warp-core/src/telemetry.rs
     /Users/james/git/echo/crates/warp-core/src/wsc/types.rs
     /Users/james/git/echo/crates/warp-core/src/wsc/write.rs
     /Users/james/git/echo/crates/warp-core/src/wsc/build.rs
     /Users/james/git/echo/crates/warp-core/src/wsc/read.rs
     /Users/james/git/echo/crates/warp-core/src/wsc/validate.rs
  ⎿  Read(/Users/james/git/echo/crates/warp-core/tests/reserve_gate_tests.rs · lines 1-150)

  ⎿  Bash(grep -r "DeterministicScheduler\|RadixScheduler\|ActiveFootprint" /Users/james/git/echo/crates/warp-core/src
         --include="*.rs" -n | head -30)

     Waiting…
  ⎿  Read 41 lines

  ⎿  /Users/james/git/echo/crates/warp-core/src/scheduler.rs:26:pub(crate) struct ActiveFootprints {
     /Users/james/git/echo/crates/warp-core/src/scheduler.rs:43:impl ActiveFootprints {
     /Users/james/git/echo/crates/warp-core/src/scheduler.rs:59:pub(crate) struct RadixScheduler {
     /Users/james/git/echo/crates/warp-core/src/scheduler.rs:64:    pub(crate) active: HashMap<TxId, ActiveFootprints>,
     /Users/james/git/echo/crates/warp-core/src/scheduler.rs:100:impl RadixScheduler {
     /Users/james/git/echo/crates/warp-core/src/scheduler.rs:135:        let active =
     self.active.entry(tx).or_insert_with(ActiveFootprints::new);
     /Users/james/git/echo/crates/warp-core/src/scheduler.rs:158:    fn has_conflict(active: &ActiveFootprints, pr:
     &PendingRewrite) -> bool {
     /Users/james/git/echo/crates/warp-core/src/scheduler.rs:239:    fn mark_all(active: &mut ActiveFootprints, pr:
     &PendingRewrite) {
     /Users/james/git/echo/crates/warp-core/src/scheduler.rs:610:pub(crate) struct DeterministicScheduler {
     /Users/james/git/echo/crates/warp-core/src/scheduler.rs:617:impl std::fmt::Debug for DeterministicScheduler {
     /Users/james/git/echo/crates/warp-core/src/scheduler.rs:619:        f.debug_struct("DeterministicScheduler")
     /Users/james/git/echo/crates/warp-core/src/scheduler.rs:628:    Radix(RadixScheduler),
     /Users/james/git/echo/crates/warp-core/src/scheduler.rs:632:impl Default for DeterministicScheduler {
     /Users/james/git/echo/crates/warp-core/src/scheduler.rs:641:impl DeterministicScheduler {
     /Users/james/git/echo/crates/warp-core/src/scheduler.rs:644:            SchedulerKind::Radix =>
     SchedulerImpl::Radix(RadixScheduler::default()),
     /Users/james/git/echo/crates/warp-core/src/scheduler.rs:754:        let mut sched = RadixScheduler::default();
     /Users/james/git/echo/crates/warp-core/src/scheduler.rs:785:        let mut sched = RadixScheduler::default();
     /Users/james/git/echo/crates/warp-core/src/scheduler.rs:842:        let mut sched = RadixScheduler::default();
     /Users/james/git/echo/crates/warp-core/src/scheduler.rs:894:        let mut sched = RadixScheduler::default();
     /Users/james/git/echo/crates/warp-core/src/scheduler.rs:946:        let mut sched = RadixScheduler::default();
     /Users/james/git/echo/crates/warp-core/src/scheduler.rs:996:        let mut sched = RadixScheduler::default();
     /Users/james/git/echo/crates/warp-core/src/scheduler.rs:1049:        let mut sched = RadixScheduler::default();
     /Users/james/git/echo/crates/warp-core/src/scheduler.rs:1107:        let mut sched = RadixScheduler::default();
     /Users/james/git/echo/crates/warp-core/src/scheduler.rs:1178:            let mut sched = RadixScheduler::default();
     /Users/james/git/echo/crates/warp-core/src/scheduler.rs:1265:        let mut sched = RadixScheduler::default();
     /Users/james/git/echo/crates/warp-core/src/engine_impl.rs:19:use crate::scheduler::{DeterministicScheduler,
     PendingRewrite, RewritePhase, SchedulerKind};
     /Users/james/git/echo/crates/warp-core/src/engine_impl.rs:316:    scheduler: DeterministicScheduler,
     /Users/james/git/echo/crates/warp-core/src/engine_impl.rs:477:            scheduler: DeterministicScheduler::new(kind,
     telemetry),
     /Users/james/git/echo/crates/warp-core/src/engine_impl.rs:623:            scheduler: DeterministicScheduler::new(kind,
     telemetry),
     /Users/james/git/echo/crates/warp-core/src/engine_impl.rs:1464:    // predicate (`RadixScheduler::has_conflict` in
     `scheduler.rs`). If one

  ⎿  Response:
       Perfect! Now let me create a comprehensive summary based on my research.

       Summary of Existing Footprint, Independence Checking, and Scheduling Infrastructure

       Based on my thorough search of the Echo codebase, here is what I found:

       1. Footprint Struct and Conflict Detection

       Location: /Users/james/git/echo/crates/warp-core/src/footprint.rs

       A comprehensive footprint infrastructure exists:

       - Footprint struct (lines 131-152): Captures read/write sets for rewrites across:
         - Nodes (read: n_read, write: n_write)
         - Edges (read: e_read, write: e_write)
         - Attachment slots (read: a_read, write: a_write)
         - Boundary ports (input: b_in, output: b_out)
         - Coarse factor_mask (u64) for O(1) prefiltering
       - Set data structures:
         - IdSet (lines 31-63): BTreeSet-backed ordered sets for nodes/edges with deterministic iteration
         - PortSet (lines 65-97): BTreeSet-backed set for boundary port keys
         - AttachmentSet (lines 99-129): BTreeSet-backed set for attachment slots
       - Independence checking Footprint::independent() (lines 161-191):
         - Fast path: checks disjoint factor_mask first
         - Then checks boundary port intersections
         - Then checks edge conflicts (write-write and write-read)
         - Finally checks attachment and node conflicts (write-write and write-read)
         - Symmetric conflict detection with early exits
       - Port key packing (lines 206-214): Helper function pack_port_key() to create 64-bit stable port identifiers

       2. Admission Control and Dependency Checking

       Location: /Users/james/git/echo/crates/warp-core/src/scheduler.rs

       Advanced scheduling infrastructure with two implementations:

       RadixScheduler (Primary)

       - PendingRewrite struct (lines 68-82): Represents pending rewrites with:
         - Rule ID, compact rule handle
         - Scope hash and scope node key
         - Footprint for conflict checking
         - RewritePhase state machine
       - ActiveFootprints struct (lines 26-55): Generation-stamped sets for O(1) conflict detection:
         - Separate tracking: nodes_written, nodes_read, edges_written, edges_read
         - attachments_written, attachments_read, ports
         - All use GenSet<K> for O(1) lookups
       - RadixScheduler struct (lines 59-65):
         - pending: HashMap<TxId, PendingTx> for O(1) enqueue, O(n) drain
         - active: HashMap<TxId, ActiveFootprints> for O(m) independence checking
       - Reserve method (lines 134-143):
         - has_conflict() (lines 158-236): Checks full footprint independence
             - Node write conflicts with prior writes OR reads
           - Node read conflicts only with prior writes
           - Same logic for edges and attachments
           - Port conflicts: ANY intersection fails
         - mark_all() (lines 239-278): Atomically marks all resources when reserved
         - Returns bool: true=reserved, false=aborted on conflict
         - No partial marking on conflict (atomic semantics)

       LegacyScheduler (Comparison)

       - Uses BTreeMap for pending queues
       - Uses Vec<Footprint> for active frontier
       - O(k) scan over reserved footprints using Footprint::independent()

       DeterministicScheduler wrapper (lines 610-712)

       - Swappable implementation (Radix vs Legacy via SchedulerKind enum)
       - Telemetry integration with counters for reserved/conflict tracking
       - Methods: enqueue(), drain_for_tx(), reserve(), finalize_tx()

       GenSet (lines 510-535)

       - Generation-stamped set for O(1) conflict detection without clearing
       - Each transaction gets a new generation
       - Tracks which generation last saw each key

       3. Deterministic Ordering/Scheduling

       Location: /Users/james/git/echo/crates/warp-core/src/scheduler.rs (lines 287-480)

       High-performance O(n) radix-based scheduler:

       - PendingTx<P> struct (lines 301-327):
         - index: FxHashMap for O(1) last-wins deduplication
         - thin: Vec with (scope_be32, rule_id, nonce, handle)
         - fat: Vec<Option> for payload storage
         - scratch and counts16: Reusable buffers for radix sort
       - Deterministic ordering (lines 469-479):
         - Lexicographic order: (scope_hash, rule_id, nonce)
         - 20-pass LSD radix sort with 16-bit digits (O(n) complexity)
         - Byte-lexicographic order over full 32-byte scope hash preserved exactly
       - Radix sort (lines 360-413):
         - 2 passes for nonce, 2 for rule_id, 16 for scope pairs
         - Stable sorting preserves determinism
         - Small-batch optimization (comparison sort for <1024 items)

       4. Worker Queue and Parallel Execution Simulation

       Location: /Users/james/git/echo/crates/warp-benches/src/bin/

       Two benchmark simulators (research tools, not production):

       sim_sharded_store.rs

       - Simulates sharded graph store with per-shard mutation
       - Thread spawning for parallel execution (one thread per shard)
       - No locks on shards - spatial partitioning enables parallelism
       - 1M entities distributed round-robin to shards
       - Demonstrates queue-per-CPU architectural idea

       sim_parallel_executor.rs

       - Uses Rayon thread pool for data-parallel execution
       - Simulates motion update kernel across 1M entities
       - Compares serial vs. parallel performance to measure overhead
       - Demonstrates ideal parallel executor baseline

       5. Inbox/Queue Infrastructure

       Location: /Users/james/git/echo/crates/warp-core/src/inbox.rs

       Event-queue ledger system:

       - Deterministic inbox rules:
         - sys/dispatch_inbox: Drains all pending edges from inbox
         - sys/ack_pending: Consumes single pending edge
       - Event nodes treated as immutable ledger (append-only)
       - Pending tracked via edge:pending edges (not node deletion)
       - Queue maintenance = edge deletion (ledger preservation)

       6. Materialization Bus

       Location: /Users/james/git/echo/crates/warp-core/src/materialization/mod.rs

       Deterministic output infrastructure:

       - MaterializationBus: Collects emissions in order-independent manner
       - EmitKey: Derived from (scope_hash, rule_id, nonce) - scheduler's canonical ordering
       - Channel policies: Log (all preserved), StrictSingle (one max), Reduce (merge)
       - Finalized post-commit for confluence-safe parallel rewriting

       7. Integration in Engine

       Location: /Users/james/git/echo/crates/warp-core/src/engine_impl.rs

       - Engine holds DeterministicScheduler instance
       - EngineBuilder allows scheduler selection (Radix/Legacy)
       - Telemetry support for tracking reserved/conflict metrics
       - Test coverage: 30+ tests validating independence checking, determinism, atomicity, and conflict detection

       Key Test Files

       - /Users/james/git/echo/crates/warp-core/tests/footprint_independence_tests.rs - Footprint independence tests
       - /Users/james/git/echo/crates/warp-core/tests/reserve_gate_tests.rs - Reserve gate admission control
       - /Users/james/git/echo/crates/warp-core/tests/scheduler_adversarial.rs - Scheduler stress tests
       - /Users/james/git/echo/crates/warp-benches/benches/scheduler_drain.rs - Performance benchmarks

       This is a sophisticated, production-ready infrastructure for conflict-free, deterministic concurrent rewriting
       with O(1) admission control and O(n) radix-based scheduling.
  ⎿  Done (22 tool uses · 78.4k tokens · 53s)

⏺ Explore(Explore existing test structure)
  ⎿  Prompt:
       Explore the current test structure in the repository, focusing on:
       1. Where tests are located (tests/ directories, inline tests)
       2. Any existing test harness or common utilities
       3. Current testing patterns used in warp-core
       4. Any benchmarking infrastructure (especially in crates/warp-benches)

       This is research only - do not modify any code.
  ⎿  ❯ Explore the current test structure in the repository, focusing on:
       1. Where tests are located (tests/ directories, inline tests)
       2. Any existing test harness or common utilities
       3. Current testing patterns used in warp-core
       4. Any benchmarking infrastructure (especially in crates/warp-benches)

       This is research only - do not modify any code.
  ⎿  Search(pattern: "**/tests/**/*.rs")

  ⎿  Search(pattern: "**/benches/**/*.rs")

  ⎿  Bash(find /Users/james/git/echo -type d -name "tests" -o -type d -name "benches" | head -20)
     Waiting…
  ⎿  Read(/Users/james/git/echo)

  ⎿  Error: EISDIR: illegal operation on a directory, read

  ⎿    ⎿  Found 41 files
          /Users/james/git/echo/crates/echo-wasm-bindings/tests/api_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/atom_payload_digest_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/footprint_independence_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/mat4_mul_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/math_additional_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/math_convenience_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/nan_exhaustive_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/snapshot_reachability_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/vec3_ops_tests.rs
          /Users/james/git/echo/crates/warp-geom/tests/geom_broad_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/math_rotation_tests.rs
          /Users/james/git/echo/crates/echo-wasm-abi/tests/non_canonical_floats.rs
          /Users/james/git/echo/crates/warp-core/tests/determinism_audit.rs
          /Users/james/git/echo/crates/warp-core/tests/ledger_tests.rs
          /Users/james/git/echo/crates/echo-dind-harness/tests/coverage.rs
          /Users/james/git/echo/crates/echo-dind-harness/tests/permutation_invariance.rs
          /Users/james/git/echo/crates/echo-wasm-abi/tests/canonical_vectors.rs
          /Users/james/git/echo/crates/echo-wasm-abi/tests/fuzz_wire.rs
          /Users/james/git/echo/crates/echo-wesley-gen/tests/generation.rs
          /Users/james/git/echo/crates/warp-core/tests/determinism_policy_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/deterministic_sin_cos_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/dfix64_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/dispatch_inbox.rs
          /Users/james/git/echo/crates/warp-core/tests/dpo_concurrency_litmus.rs
          /Users/james/git/echo/crates/warp-core/tests/duplicate_rule_registration_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/engine_motion_negative_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/engine_motion_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/inbox.rs
          /Users/james/git/echo/crates/warp-core/tests/math_scalar_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/math_validation.rs
          /Users/james/git/echo/crates/warp-core/tests/permutation_commute_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/property_commute_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/proptest_seed_pinning.rs
          /Users/james/git/echo/crates/warp-core/tests/reserve_gate_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/tick_receipt_tests.rs
          /Users/james/git/echo/crates/warp-core/tests/tx_lifecycle_tests.rs
          /Users/james/git/echo/crates/echo-wasm-abi/tests/codec.rs
          /Users/james/git/echo/crates/warp-core/tests/materialization_engine_integration.rs
          /Users/james/git/echo/crates/warp-core/tests/common/mod.rs
          /Users/james/git/echo/crates/warp-core/tests/materialization_spec_police.rs
          /Users/james/git/echo/crates/warp-core/tests/materialization_determinism.rs
  ⎿    ⎿  Found 4 files
          /Users/james/git/echo/crates/warp-benches/benches/scheduler_adversarial.rs
          /Users/james/git/echo/crates/warp-benches/benches/snapshot_hash.rs
          /Users/james/git/echo/crates/warp-benches/benches/motion_throughput.rs
          /Users/james/git/echo/crates/warp-benches/benches/scheduler_drain.rs
  ⎿  /Users/james/git/echo/crates/echo-wasm-bindings/tests
     /Users/james/git/echo/crates/warp-core/tests
     /Users/james/git/echo/crates/warp-benches/benches
     /Users/james/git/echo/crates/echo-dind-harness/tests
     /Users/james/git/echo/crates/warp-geom/tests
     /Users/james/git/echo/crates/echo-wesley-gen/tests
     /Users/james/git/echo/crates/echo-wasm-abi/tests
     /Users/james/git/echo/node_modules/domutils/test/tests
     /Users/james/git/echo/tests
     /Users/james/git/echo/.venv/lib/python3.14/site-packages/mpl_toolkits/axes_grid1/tests
     /Users/james/git/echo/.venv/lib/python3.14/site-packages/mpl_toolkits/mplot3d/tests
     /Users/james/git/echo/.venv/lib/python3.14/site-packages/mpl_toolkits/axisartist/tests
     /Users/james/git/echo/.venv/lib/python3.14/site-packages/numpy/linalg/tests
     /Users/james/git/echo/.venv/lib/python3.14/site-packages/numpy/ma/tests
     /Users/james/git/echo/.venv/lib/python3.14/site-packages/numpy/_core/tests
     /Users/james/git/echo/.venv/lib/python3.14/site-packages/numpy/tests
     /Users/james/git/echo/.venv/lib/python3.14/site-packages/numpy/_pyinstaller/tests
     /Users/james/git/echo/.venv/lib/python3.14/site-packages/numpy/typing/tests
     /Users/james/git/echo/.venv/lib/python3.14/site-packages/numpy/f2py/tests
     /Users/james/git/echo/.venv/lib/python3.14/site-packages/numpy/testing/tests
  ⎿  Read(/Users/james/git/echo/crates/warp-core/tests/common/mod.rs)

  ⎿  Read(/Users/james/git/echo/crates/warp-benches/Cargo.toml)

  ⎿  Bash(ls -la /Users/james/git/echo/crates/warp-benches/)
     Waiting…
  ⎿  Bash(ls -la /Users/james/git/echo/crates/warp-core/tests/)
     Waiting…
  ⎿  Read 75 lines

  ⎿  Read 42 lines

  ⎿  drwxr-xr-x@    - james 17 Jan 05:28 benches
     drwxr-xr-x     - james 17 Jan 20:44 src
     .rw-r--r--@ 6.1k james 30 Dec  2025 .DS_Store
     .rw-r--r--@ 1.1k james 17 Jan 20:53 Cargo.toml
     .rw-r--r--@  726 james 30 Dec  2025 README.md
  ⎿  drwxr-xr-x@    - james 17 Jan 15:49 common
     drwxr-xr-x@    - james 17 Jan 05:28 fixtures
     .rw-r--r--@ 3.4k james 30 Dec  2025 atom_payload_digest_tests.rs
     .rw-r--r--@ 4.2k james 16 Jan 16:36 determinism_audit.rs
     .rw-r--r--@ 3.5k james 17 Jan 05:28 determinism_policy_tests.rs
     .rw-r--r--@  11k james 17 Jan 05:28 deterministic_sin_cos_tests.rs
     .rw-r--r--@ 1.8k james 17 Jan 05:28 dfix64_tests.rs
     .rw-r--r--@ 5.1k james 17 Jan 05:28 dispatch_inbox.rs
     .rw-r--r--@  11k james 17 Jan 05:28 dpo_concurrency_litmus.rs
     .rw-r--r--@ 2.3k james 17 Jan 05:28 duplicate_rule_registration_tests.rs
     .rw-r--r--@ 5.5k james 17 Jan 05:28 engine_motion_negative_tests.rs
     .rw-r--r--@ 5.3k james 17 Jan 05:28 engine_motion_tests.rs
     .rw-r--r--@ 2.3k james 30 Dec  2025 footprint_independence_tests.rs
     .rw-r--r--@ 4.9k james 17 Jan 05:28 inbox.rs
     .rw-r--r--@ 1.1k james 16 Jan 16:36 ledger_tests.rs
     .rw-r--r--@ 6.6k james 30 Dec  2025 mat4_mul_tests.rs
     .rw-r--r--@  25k james 17 Jan 15:50 materialization_determinism.rs
     .rw-r--r--@  21k james 17 Jan 14:56 materialization_engine_integration.rs
     .rw-r--r--@  30k james 17 Jan 15:49 materialization_spec_police.rs
     .rw-r--r--@ 1.0k james 30 Dec  2025 math_additional_tests.rs
     .rw-r--r--@ 1.7k james 30 Dec  2025 math_convenience_tests.rs
     .rw-r--r--@ 2.4k james  1 Jan 10:03 math_rotation_tests.rs
     .rw-r--r--@ 2.2k james 17 Jan 05:28 math_scalar_tests.rs
     .rw-r--r--@  14k james 17 Jan 05:28 math_validation.rs
     .rw-r--r--@ 4.4k james 30 Dec  2025 nan_exhaustive_tests.rs
     .rw-r--r--@ 2.6k james 17 Jan 05:28 permutation_commute_tests.rs
     .rw-r--r--@ 2.3k james 17 Jan 05:28 property_commute_tests.rs
     .rw-r--r--@ 4.0k james 17 Jan 05:28 proptest_seed_pinning.rs
     .rw-r--r--@ 1.5k james 17 Jan 05:28 reserve_gate_tests.rs
     .rw-r--r--@ 2.2k james 30 Dec  2025 snapshot_reachability_tests.rs
     .rw-r--r--@ 9.9k james 17 Jan 05:28 tick_receipt_tests.rs
     .rw-r--r--@ 1.3k james 17 Jan 05:28 tx_lifecycle_tests.rs
     .rw-r--r--@  985 james 30 Dec  2025 vec3_ops_tests.rs
  ⎿  Read(/Users/james/git/echo/crates/warp-benches/benches/motion_throughput.rs)

  ⎿  Read(/Users/james/git/echo/crates/warp-benches/benches/snapshot_hash.rs)

  ⎿  Read(/Users/james/git/echo/crates/warp-core/tests/materialization_engine_integration.rs · lines 1-100)

  ⎿  Read 116 lines

  ⎿  Read 89 lines

  ⎿  Read 100 lines

  ⎿  Read(/Users/james/git/echo/crates/warp-core/tests/determinism_audit.rs)

  ⎿  Read(/Users/james/git/echo/crates/warp-core/tests/permutation_commute_tests.rs)

  ⎿  Read(/Users/james/git/echo/crates/warp-core/tests/proptest_seed_pinning.rs)

  ⎿  Read 127 lines

  ⎿  Read 73 lines

  ⎿  Read 93 lines

  ⎿  Read(/Users/james/git/echo/crates/warp-core/tests/materialization_spec_police.rs · lines 1-80)

  ⎿  Bash(grep -l "^\s*#\[test\]" /Users/james/git/echo/crates/warp-core/src/*.rs 2>/dev/null | head -5)
     Waiting…
  ⎿  Bash(grep -l "^\s*#\[cfg(test)\]" /Users/james/git/echo/crates/warp-core/src/*.rs 2>/dev/null | head -5)
     Waiting…
  ⎿  Read 80 lines

  ⎿  /Users/james/git/echo/crates/warp-core/src/attachment.rs
     /Users/james/git/echo/crates/warp-core/src/engine_impl.rs
     /Users/james/git/echo/crates/warp-core/src/footprint.rs
     /Users/james/git/echo/crates/warp-core/src/graph.rs
     /Users/james/git/echo/crates/warp-core/src/ident.rs
  ⎿  /Users/james/git/echo/crates/warp-core/src/attachment.rs
     /Users/james/git/echo/crates/warp-core/src/engine_impl.rs
     /Users/james/git/echo/crates/warp-core/src/footprint.rs
     /Users/james/git/echo/crates/warp-core/src/graph.rs
     /Users/james/git/echo/crates/warp-core/src/ident.rs
  ⎿  Read(/Users/james/git/echo/crates/warp-core/Cargo.toml)

  ⎿  Bash(grep -A 10 "^\s*#\[cfg(test)\]" /Users/james/git/echo/crates/warp-core/src/attachment.rs | head -20)
     Waiting…
  ⎿  Bash(ls -la /Users/james/git/echo/crates/warp-benches/src/)
     Waiting…
  ⎿  Read 53 lines

  ⎿  drwxr-xr-x - james 17 Jan 20:53 bin

  ⎿  #[cfg(test)]
     #[allow(clippy::expect_used, clippy::unwrap_used)]
     mod tests {
         use super::*;

         struct U8Codec;

         impl Codec<u8> for U8Codec {
             const TYPE_ID: TypeId = TypeId([0xA5; 32]);

             fn encode_canon(value: &u8) -> Bytes {
  ⎿  Bash(ls -la /Users/james/git/echo/crates/warp-benches/src/bin/)
     Waiting…
  ⎿  Read(/Users/james/git/echo/crates/warp-benches/benches/scheduler_drain.rs · lines 1-80)

  ⎿  Read(/Users/james/git/echo/crates/warp-core/tests/engine_motion_tests.rs · lines 1-60)

  ⎿  Read 80 lines

  ⎿  .rw-r--r-- 3.0k james 17 Jan 20:49 sim_parallel_executor.rs
     .rw-r--r-- 3.5k james 17 Jan 20:53 sim_sharded_store.rs
     .rw-r--r-- 2.8k james 17 Jan 20:44 stress_throughput.rs
  ⎿  Read 60 lines

  ⎿  Bash(find /Users/james/git/echo/crates/echo-dry-tests -type f -name "*.rs" | head -10)
     Waiting…
  ⎿  Read(/Users/james/git/echo/crates/echo-dry-tests/src/lib.rs · lines 1-100)

  ⎿  Read(/Users/james/git/echo/crates/warp-benches/src/bin/stress_throughput.rs · lines 1-60)

  ⎿  Read 37 lines

  ⎿  Read 60 lines

  ⎿  /Users/james/git/echo/crates/echo-dry-tests/src/rules.rs
     /Users/james/git/echo/crates/echo-dry-tests/src/demo_rules.rs
     /Users/james/git/echo/crates/echo-dry-tests/src/config.rs
     /Users/james/git/echo/crates/echo-dry-tests/src/lib.rs
     /Users/james/git/echo/crates/echo-dry-tests/src/hashes.rs
     /Users/james/git/echo/crates/echo-dry-tests/src/frames.rs
     /Users/james/git/echo/crates/echo-dry-tests/src/engine.rs
     /Users/james/git/echo/crates/echo-dry-tests/src/motion.rs
  ⎿  Read(/Users/james/git/echo/crates/warp-benches/README.md)

  ⎿  Bash(head -50 /Users/james/git/echo/crates/warp-benches/benches/scheduler_adversarial.rs)
     Waiting…
  ⎿  Read(/Users/james/git/echo/crates/warp-core/tests/materialization_determinism.rs · lines 1-80)

  ⎿  Read 22 lines

  ⎿  // SPDX-License-Identifier: Apache-2.0
     // © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
     #![allow(missing_docs)]

     use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
     use rand::Rng;
     use rustc_hash::FxHashMap;

     /// Key type that forces all entries into the same hash bucket (constant hash).
     #[derive(Clone, Copy, PartialEq, Eq, Hash)]
     struct Colliding(u64);

     fn bench_fxhash_collision(c: &mut Criterion) {
         let mut group = c.benchmark_group("scheduler_adversarial/colliding");
         for &n in &[1_000u64, 5_000, 10_000] {
             group.bench_function(format!("insert_and_probe/{n}"), |b| {
                 b.iter_batched(
                     || {
                         let mut map: FxHashMap<Colliding, u64> = FxHashMap::default();
                         // pre-seed with colliding keys
                         for i in 0..n {
                             map.insert(Colliding(i), i);
                         }
                         map
                     },
                     |mut map| {
                         // probe and insert another colliding key
                         let key = Colliding(n + 1);
                         let _ = map.get(&key);
                         map.insert(key, n + 1);
                         black_box(map);
                     },
                     BatchSize::SmallInput,
                 );
             });
         }
         group.finish();
     }

     fn bench_fxhash_random(c: &mut Criterion) {
         let mut group = c.benchmark_group("scheduler_adversarial/random");
         for &n in &[1_000u64, 5_000, 10_000] {
             group.bench_function(format!("insert_and_probe/{n}"), |b| {
                 b.iter_batched(
                     || {
                         let mut rng = rand::thread_rng();
                         let mut map: FxHashMap<u64, u64> = FxHashMap::default();
                         for _ in 0..n {
                             let k = rng.gen::<u64>();
                             map.insert(k, k);
  ⎿  Read 80 lines

  ⎿  Bash(grep -n "^\s*#\[test\]" /Users/james/git/echo/crates/warp-core/src/attachment.rs | head -5)
     Waiting…
  ⎿  Bash(wc -l /Users/james/git/echo/crates/warp-core/src/attachment.rs)
     Waiting…
  ⎿  Bash(sed -n '300,400p' /Users/james/git/echo/crates/warp-core/src/attachment.rs)
     Waiting…
  ⎿       403 /Users/james/git/echo/crates/warp-core/src/attachment.rs

  ⎿  #[derive(Debug, Error)]
     pub enum RegistryError {
         /// Attempted to register two codecs for the same `TypeId`.
         #[error("duplicate codec registration for type id: {0:?}")]
         DuplicateTypeId(TypeId),
     }

     impl CodecRegistry {
         /// Creates an empty registry.
         #[must_use]
         pub fn new() -> Self {
             Self::default()
         }

         /// Registers codec `C` for payload type `T`.
         ///
         /// `type_name` is a human-readable identifier for debugging/tooling.
         ///
         /// # Errors
         /// Returns [`RegistryError::DuplicateTypeId`] if a codec is already registered
         /// for the same `TypeId`.
         pub fn register<T, C>(&mut self, type_name: &'static str) -> Result<(), RegistryError>
         where
             T: Any + Send + Sync,
             C: Codec<T> + Send + Sync + 'static,
         {
             let type_id = C::TYPE_ID;
             if self.codecs.contains_key(&type_id) {
                 return Err(RegistryError::DuplicateTypeId(type_id));
             }
             self.codecs
                 .insert(type_id, Box::new(ErasedCodecImpl::<T, C>::new(type_name)));
             Ok(())
         }

         /// Returns the codec registered for `type_id` (if any).
         #[must_use]
         pub fn get(&self, type_id: &TypeId) -> Option<&dyn ErasedCodec> {
             self.codecs.get(type_id).map(std::convert::AsRef::as_ref)
         }

         /// Decodes a typed atom payload using the codec registered for its `type_id`.
         ///
         /// # Errors
         /// Returns [`DecodeError::TypeMismatch`] if the payload `type_id` is not
         /// registered.
         pub fn decode_atom(&self, payload: &AtomPayload) -> Result<Box<dyn Any>, DecodeError> {
             let Some(codec) = self.get(&payload.type_id) else {
                 return Err(DecodeError::UnknownTypeId(payload.type_id));
             };
             codec.decode_any(&payload.bytes)
         }
     }

     #[cfg(test)]
     #[allow(clippy::expect_used, clippy::unwrap_used)]
     mod tests {
         use super::*;

         struct U8Codec;

         impl Codec<u8> for U8Codec {
             const TYPE_ID: TypeId = TypeId([0xA5; 32]);

             fn encode_canon(value: &u8) -> Bytes {
                 Bytes::from(vec![*value])
             }

             fn decode_strict(bytes: &Bytes) -> Result<u8, DecodeError> {
                 if bytes.len() != 1 {
                     return Err(DecodeError::InvalidBytes);
                 }
                 Ok(*bytes.first().ok_or(DecodeError::InvalidBytes)?)
             }
         }

         #[test]
         fn atom_decode_with_enforces_type_id() {
             let payload = AtomPayload::new(U8Codec::TYPE_ID, U8Codec::encode_canon(&7u8));
             assert_eq!(payload.decode_with::<U8Codec, u8>().unwrap(), 7u8);

             let wrong = AtomPayload::new(TypeId([0x5A; 32]), U8Codec::encode_canon(&7u8));
             assert!(
                 matches!(
                     wrong.decode_with::<U8Codec, u8>(),
                     Err(DecodeError::TypeMismatch { .. })
                 ),
     "type mismatch must be reported deterministically"
             );
         }

         #[test]
         fn atom_decode_for_match_is_deterministic_and_conservative() {
             // v0 semantics: any decode failure -> rule does not apply (None).
             let good = AtomPayload::new(U8Codec::TYPE_ID, U8Codec::encode_canon(&9u8));
             assert_eq!(good.decode_for_match::<U8Codec, u8>(), Some(9u8));

             let wrong_type = AtomPayload::new(TypeId([0x00; 32]), U8Codec::encode_canon(&9u8));
             assert_eq!(wrong_type.decode_for_match::<U8Codec, u8>(), None);

             let bad_bytes = AtomPayload::new(U8Codec::TYPE_ID, Bytes::from_static(&[]));
  ⎿  376:    #[test]
     391:    #[test]
  ⎿  Bash(ls -la /Users/james/git/echo/crates/warp-core/tests/fixtures/ 2>/dev/null | head -20)
     Waiting…
  ⎿  Bash(find /Users/james/git/echo/crates/warp-core/tests -type f | grep -v "\.rs$" | head -10)
     Waiting…
  ⎿  .rw-r--r--@ 1.4k james 17 Jan 05:28 math-fixtures.cbor
     .rw-r--r--@ 3.7k james 30 Dec  2025 math-fixtures.json
  ⎿  /Users/james/git/echo/crates/warp-core/tests/fixtures/math-fixtures.json
     /Users/james/git/echo/crates/warp-core/tests/fixtures/math-fixtures.cbor
  ⎿  Response:
       Excellent! Now I have comprehensive information. Let me create a detailed report of my findings.

       Test Structure Exploration Report

       Based on my thorough exploration of the repository, here's a comprehensive overview of the test infrastructure
       in the Echo project:

       1. TEST LOCATIONS

       Integration Tests (Separate Test Crates)

       Located in /Users/james/git/echo/crates/warp-core/tests/:

       Core Integration Test Files (40+ test files):
       - Materialization Bus Tests: materialization_engine_integration.rs, materialization_spec_police.rs,
       materialization_determinism.rs
       - Physics & Motion: engine_motion_tests.rs, engine_motion_negative_tests.rs, permutation_commute_tests.rs
       - Determinism Audits: determinism_audit.rs, determinism_policy_tests.rs
       - Math & Validation: math_validation.rs, math_scalar_tests.rs, math_rotation_tests.rs,
       math_convenience_tests.rs, math_additional_tests.rs, deterministic_sin_cos_tests.rs, nan_exhaustive_tests.rs,
       dfix64_tests.rs, mat4_mul_tests.rs, vec3_ops_tests.rs
       - Graph Operations: dpo_concurrency_litmus.rs, dispatch_inbox.rs, inbox.rs, snapshot_reachability_tests.rs
       - Engine Lifecycle: tx_lifecycle_tests.rs, tick_receipt_tests.rs, ledger_tests.rs
       - Property-Based Testing: proptest_seed_pinning.rs, property_commute_tests.rs
       - Payload & Encoding: atom_payload_digest_tests.rs, footprint_independence_tests.rs,
       duplicate_rule_registration_tests.rs, reserve_gate_tests.rs

       Test Utilities:
       - Common module at /Users/james/git/echo/crates/warp-core/tests/common/mod.rs providing shared helpers
       - Test fixture data in /Users/james/git/echo/crates/warp-core/tests/fixtures/ (math-fixtures.cbor,
       math-fixtures.json)

       Inline Tests (In Source Code)

       Found in multiple source files, including:
       - /Users/james/git/echo/crates/warp-core/src/attachment.rs (2+ inline test functions)
       - /Users/james/git/echo/crates/warp-core/src/engine_impl.rs
       - /Users/james/git/echo/crates/warp-core/src/footprint.rs
       - /Users/james/git/echo/crates/warp-core/src/graph.rs
       - /Users/james/git/echo/crates/warp-core/src/ident.rs

       Other Test Crates

       - echo-wasm-bindings: /Users/james/git/echo/crates/echo-wasm-bindings/tests/api_tests.rs
       - echo-wasm-abi: Multiple tests including canonical_vectors.rs, codec.rs, fuzz_wire.rs, non_canonical_floats.rs
       - warp-geom: geom_broad_tests.rs
       - echo-wesley-gen: generation.rs
       - echo-dind-harness: coverage.rs, permutation_invariance.rs

       ---
       2. TEST HARNESS & COMMON UTILITIES

       Primary Test Support Crate: echo-dry-tests

       Location: /Users/james/git/echo/crates/echo-dry-tests/src/

       Modules and Exports:
       - demo_rules: Pre-built rule fixtures (motion_rule(), port_rule() with constants MOTION_RULE_NAME,
       PORT_RULE_NAME)
       - engine: Engine builders (build_engine_with_root(), build_engine_with_typed_root(), EngineTestBuilder)
       - motion: Motion payload construction (MotionPayloadBuilder, DEFAULT_MOTION_POSITION, DEFAULT_MOTION_VELOCITY)
       - rules: Synthetic rule builders (NoOpRule, SyntheticRuleBuilder)
       - frames: Snapshot and diff builders (SnapshotBuilder, DiffBuilder)
       - hashes: Hash generation helpers (make_intent_id(), make_rule_id())
       - config: In-memory config store fake for testing

       Common Module in warp-core Tests

       Location: /Users/james/git/echo/crates/warp-core/tests/common/mod.rs

       Key Utilities:
       // Snapshot hashing
       snapshot_hash_of(store: GraphStore, root: NodeId) -> [u8; 32]

       // Materialization test utilities
       h(n: u8) -> [u8; 32]  // Deterministic test hashes
       key(scope: u8, rule: u32) -> EmitKey
       key_sub(scope: u8, rule: u32, subkey: u32) -> EmitKey

       // Permutation testing
       for_each_permutation<T>(items: &mut [T], f: impl FnMut(&[T]))
       // Uses Heap's algorithm for deterministic permutation generation

       ---
       3. TESTING PATTERNS IN WARP-CORE

       Pattern 1: Determinism Testing

       Tests verify byte-identical output regardless of execution order:
       - File: permutation_commute_tests.rs
       - Approach: Apply rules in different permutations, compare snapshot hashes
       - Example: Tests n=3,4 entity permutations with unique velocities

       Pattern 2: Exhaustive Permutation Testing with Byte Diffing

       - File: materialization_spec_police.rs
       - Approach: Heap's algorithm for all N! permutations, byte-for-byte comparison
       - Pattern: Custom assert_bytes_eq() helper showing context around divergence points
       - Scope: Tests for N ≤ 6 to keep runtime reasonable

       Pattern 3: Property-Based Testing with Pinned Seeds

       - File: proptest_seed_pinning.rs
       - Framework: proptest with deterministic TestRng::from_seed()
       - Pattern: Pin SEED_BYTES for CI reproducibility
       - Example:
       const SEED_BYTES: [u8; 32] = [0x42, 0, 0, 0, ...];
       let rng = TestRng::from_seed(RngAlgorithm::ChaCha, &SEED_BYTES);
       let mut runner = TestRunner::new_with_rng(PropConfig::default(), rng);

       Pattern 4: Float Sensitivity Auditing

       - File: determinism_audit.rs
       - Approach: Test that 1 ULP float changes affect hash output
       - Versions: Tests both V0 (raw f32) and V2 (Q32.32 fixed-point) encodings
       - Assertions: Verify sensitivity and repeatability across 100 iterations

       Pattern 5: Engine Lifecycle Testing

       - File: engine_motion_tests.rs
       - Pattern: Create engine, register rule, apply rule, verify state mutations
       - Structure: Uses GraphStore → Engine → begin() → apply() → commit()
       - Verification: Decode payloads and assert expected transformations

       Pattern 6: Integration Testing with Fixtures

       - Files: materialization_engine_integration.rs
       - Pattern: Simulate rule execution context (SimulatedRuleExec), use ScopedEmitter, test finalization
       - Architecture: Tests the full bus path: apply_reserved_rewrites() → finalize() → receive

       Pattern 7: Error and Edge Case Testing

       - Files: engine_motion_negative_tests.rs, duplicate_rule_registration_tests.rs
       - Pattern: Test expected failures and error conditions
       - Example: Verify that registering duplicate rule names fails

       ---
       4. BENCHMARKING INFRASTRUCTURE (warp-benches)

       Framework: Criterion.rs

       - Version: 0.5 with HTML reports feature
       - Location: /Users/james/git/echo/crates/warp-benches/
       - Configuration in Cargo.toml:
       [dev-dependencies]
       criterion = { version = "0.5", default-features = false, features = ["html_reports"] }

       Benchmark Structure

       4 criterion-based benchmarks, each with harness = false:

       1. Snapshot Hash Benchmark (snapshot_hash.rs)

       - Measures: Hash computation over linear chain graphs
       - Graph Shape: Root node → chain of n entities
       - Sizes: 10, 100, 1,000, 3,000, 10,000, 30,000 nodes
       - Pattern: Uses iter_batched() with BatchSize::PerIteration to exclude setup
       - Configuration:
         - Warm-up: 3 seconds
         - Measurement: 10 seconds
         - Sample size: 80
       - Throughput: Elements (nodes in reachable set)

       2. Motion Throughput Benchmark (motion_throughput.rs)

       - Measures: Engine construction + node inserts (build-only) vs apply+commit
       - Two Groups:
         - motion_build_only: Node insertion performance
         - motion_apply_commit: Rule application and commit performance
       - Sizes: 1, 10, 100, 1,000 entities
       - Configuration:
         - Sample size: 50
         - Warm-up: 3 seconds
         - Measurement: 6 seconds
         - Noise threshold: 0.02 (2%)

       3. Scheduler Drain Benchmark (scheduler_drain.rs)

       - Measures: Scheduler overhead with a no-op rule
       - Approach: Measures applying trivial rule to n entities (excludes executor work)
       - Helper: Custom bench_noop_rule() function
       - Purpose: Isolate scheduler overhead from executor performance

       4. Scheduler Adversarial Benchmark (scheduler_adversarial.rs)

       - Measures: Hash table collision performance under worst-case scenarios
       - Benches:
         - scheduler_adversarial/colliding: All keys hash to same bucket
         - scheduler_adversarial/random: Random key distribution
       - Framework: Uses rustc-hash::FxHashMap
       - Sizes: 1,000, 5,000, 10,000 entries

       Binary Tools in /Users/james/git/echo/crates/warp-benches/src/bin/

       - stress_throughput.rs: "Let 'er rip" stress test
         - Configurable duration (default 10s) and entity count (default 10,000)
         - Continuous apply → commit loop measuring throughput
         - Prints ticks/sec and total updates

       Report Generation

       - HTML Reports: Criterion generates HTML reports automatically
       - Location: Reports in target/criterion/
       - Documentation: Referenced in docs/benchmarks/ and repository README

       ---
       5. TEST DEPENDENCIES & FEATURES

       Core Test Dependencies (in warp-core/Cargo.toml)

       [dev-dependencies]
       proptest = { version = "1.5" }
       echo-dry-tests = { workspace = true }
       echo-wasm-abi = { workspace = true }
       serde = { version = "1.0", features = ["derive"] }

       Bench Dependencies (in warp-benches/Cargo.toml)

       [dev-dependencies]
       criterion = { version = "0.5", default-features = false, features = ["html_reports"] }
       blake3 = { version = "~1.8.2", default-features = false, features = ["std"] }
       rustc-hash = "2.1.1"
       rand = "0.8"

       Feature Flags

       - golden_prng: Optional regression check for PRNG sequences (off by default)
       - trig_audit_print: Extra diagnostic printing in deterministic trig tests
       - det_float: Default float32-backed lane with F32Scalar
       - det_fixed: Experimental fixed-point Q32.32 lane (test-only)

       ---
       6. KEY PATTERNS & BEST PRACTICES OBSERVED

       1. Deterministic Test Seeds: All property-based tests use pinned seeds for reproducibility
       2. Permutation Testing: Exhaustive permutation coverage (Heap's algorithm) for small N
       3. Byte-Identical Assertions: Tests use byte comparison for serialized outputs with contextual diffs
       4. Batched Benchmarks: Criterion iter_batched() separates setup from measurement
       5. Float Precision Auditing: Explicit 1 ULP sensitivity testing for floating-point code
       6. Shared Fixtures: echo-dry-tests crate centralizes test doubles (demo rules, builders)
       7. Inline + Integration Mix: Both module-level #[cfg(test)] blocks and separate test files
       8. Custom Helpers: Utility modules like common/mod.rs for test infrastructure

       ---
       Summary Statistics
       ┌────────────────────────────────────┬────────────────────┐
       │              Category              │       Count        │
       ├────────────────────────────────────┼────────────────────┤
       │ Integration test files (warp-core) │ 40+                │
       ├────────────────────────────────────┼────────────────────┤
       │ Test functions (integration)       │ 100+               │
       ├────────────────────────────────────┼────────────────────┤
       │ Inline test modules (source files) │ 5+                 │
       ├────────────────────────────────────┼────────────────────┤
       │ Benchmark suites                   │ 4                  │
       ├────────────────────────────────────┼────────────────────┤
       │ Test utility crates                │ 1 (echo-dry-tests) │
       ├────────────────────────────────────┼────────────────────┤
       │ Test fixture files                 │ 2 (math-fixtures)  │
       ├────────────────────────────────────┼────────────────────┤
       │ Binary stress tools                │ 1                  │
       └────────────────────────────────────┴────────────────────┘
       This comprehensive test infrastructure supports Echo's critical requirement for determinism across the graph
       rewriting engine, materialization bus, and physics simulation components.
  ⎿  Done (34 tool uses · 67.9k tokens · 1m 17s)

⏺ Explore(Explore materialization bus)
  ⎿  Prompt:
       Search for "materialization" or "bus" related code in the codebase. The AGENTS.md mentions a "materialization
       bus" in warp-core. Find:
       1. Any MaterializationBus or similar infrastructure
       2. How graph state is currently materialized
       3. Any commit/hash computation logic
       4. State root or patch digest computation if it exists

       This is research only - do not modify any code.
  ⎿  ❯ Search for "materialization" or "bus" related code in the codebase. The AGENTS.md mentions a "materialization bus" in
       warp-core. Find:
       1. Any MaterializationBus or similar infrastructure
       2. How graph state is currently materialized
       3. Any commit/hash computation logic
       4. State root or patch digest computation if it exists

       This is research only - do not modify any code.
  ⎿  Search(pattern: "materialization|MaterializationBus")

  ⎿  Search(pattern: "bus|Bus", glob: "**/*.rs", head_limit: 30)

  ⎿  Search(pattern: "state.*root|patch.*digest|commit.*hash", glob: "**/*.rs")

  ⎿  Search(pattern: "**/materialization*")

  ⎿  Search(pattern: "**/*bus*")

  ⎿    ⎿  Found 14 files
          crates/warp-core/src/engine_impl.rs
          crates/echo-dind-tests/src/lib.rs
          crates/echo-dind-harness/src/dind.rs
          crates/warp-wasm/src/lib.rs
          crates/warp-core/tests/tick_receipt_tests.rs
          crates/warp-core/tests/dpo_concurrency_litmus.rs
          crates/warp-core/src/tick_patch.rs
          crates/warp-core/src/receipt.rs
          crates/warp-core/src/constants.rs
          crates/warp-core/tests/determinism_audit.rs
          crates/warp-core/src/snapshot.rs
          crates/warp-core/src/serializable.rs
          crates/warp-core/tests/atom_payload_digest_tests.rs
          crates/warp-benches/benches/snapshot_hash.rs
  ⎿    ⎿  Found 31 files
          crates/warp-core/src/materialization/frame.rs
          crates/warp-core/tests/materialization_determinism.rs
          crates/warp-core/tests/materialization_spec_police.rs
          crates/warp-core/tests/common/mod.rs
          crates/warp-core/src/materialization/bus.rs
          .github/workflows/determinism.yml
          crates/warp-core/src/engine_impl.rs
          docs/rfc/mat-bus-finish.md
          docs/architecture-outline.md
          AGENTS.md
          crates/warp-core/src/materialization/mod.rs
          docs/adr/ADR-0003-Materialization-Bus.md
          CHANGELOG.md
          crates/warp-core/tests/materialization_engine_integration.rs
          crates/warp-core/src/materialization/scoped_emitter.rs
          crates/warp-core/src/materialization/channel.rs
          crates/warp-core/src/materialization/emission_port.rs
          crates/warp-core/src/materialization/port.rs
          crates/warp-core/src/lib.rs
          docs/spec-canonical-inbox-sequencing.md
          docs/diagrams.md
          docs/book/echo/sections/02-high-level-architecture.tex
          docs/book/echo/sections/05-game-loop.tex
          docs/adr/ADR-0004-No-Global-State.md
          docs/adr/ADR-0005-Physics.md
          ECHO_ROADMAP.md
          ADR-0003-Materialization-Bus.md
          ADR-0004-No-Global-State.md
          ADR-0005-Physics.md
          specs/spec-000-rewrite/spec.md
          docs/jitos/spec-0000.md
  ⎿    ⎿  Found 14 files
          crates/warp-core/tests/materialization_determinism.rs
          crates/warp-core/tests/materialization_spec_police.rs
          crates/warp-core/src/materialization/bus.rs
          crates/warp-core/src/engine_impl.rs
          crates/warp-core/src/materialization/mod.rs
          crates/warp-core/tests/materialization_engine_integration.rs
          crates/warp-core/src/materialization/reduce_op.rs
          crates/warp-core/src/materialization/scoped_emitter.rs
          crates/warp-core/src/materialization/channel.rs
          crates/warp-core/src/materialization/emission_port.rs
          crates/warp-core/src/materialization/port.rs
          crates/echo-wasm-abi/tests/fuzz_wire.rs
          crates/echo-session-ws-gateway/src/main.rs
          crates/echo-session-proto/src/lib.rs
  ⎿    ⎿  Found 91 files
          /Users/james/git/echo/node_modules/caniuse-lite/data/features/webusb.js
          /Users/james/git/echo/target/debug/deps/tick_receipt_tests-39543b211e4d4255.4busktzzcy63cefdjoi4e2qv1.1sexrj8.rcg
          u.o
          /Users/james/git/echo/target/debug/deps/tick_receipt_tests-39543b211e4d4255.4busktzzcy63cefdjoi4e2qv1.1r817m6.rcg
          u.o
          /Users/james/git/echo/target/debug/deps/tick_receipt_tests-39543b211e4d4255.4busktzzcy63cefdjoi4e2qv1.1angc7c.rcg
          u.o
          /Users/james/git/echo/target/debug/deps/tick_receipt_tests-39543b211e4d4255.4busktzzcy63cefdjoi4e2qv1.0i713ur.rcg
          u.o
          /Users/james/git/echo/target/debug/deps/tick_receipt_tests-39543b211e4d4255.4busktzzcy63cefdjoi4e2qv1.11n0qwo.rcg
          u.o
          /Users/james/git/echo/target/debug/deps/tick_receipt_tests-39543b211e4d4255.4busktzzcy63cefdjoi4e2qv1.0i8sl4a.rcg
          u.o
          /Users/james/git/echo/target/debug/deps/tick_receipt_tests-39543b211e4d4255.4busktzzcy63cefdjoi4e2qv1.1nszx9e.rcg
          u.o
          /Users/james/git/echo/target/debug/deps/tick_receipt_tests-39543b211e4d4255.4busktzzcy63cefdjoi4e2qv1.0c32ufk.rcg
          u.o
          /Users/james/git/echo/target/debug/deps/proptest_seed_pinning-3290e9553f67db96.edopj5bobustybpfmw8fuqp5x.11btdl8.
          rcgu.o
          /Users/james/git/echo/target/debug/deps/proptest_seed_pinning-3290e9553f67db96.edopj5bobustybpfmw8fuqp5x.0vajpoe.
          rcgu.o
          /Users/james/git/echo/target/debug/deps/proptest_seed_pinning-3290e9553f67db96.edopj5bobustybpfmw8fuqp5x.1ns4agz.
          rcgu.o
          /Users/james/git/echo/target/debug/deps/proptest_seed_pinning-3290e9553f67db96.edopj5bobustybpfmw8fuqp5x.1qe4gop.
          rcgu.o
          /Users/james/git/echo/target/debug/deps/xtask-bab4e62ad2822cb6.bnz4eu3milxbusdd79ekxkpk1.1bpnni2.rcgu.o
          /Users/james/git/echo/target/debug/deps/echo_wesley_gen-723da9b778112c31.bop21ayegbusye6lkkyvev5ea.033eybh.rcgu.o
          /Users/james/git/echo/target/debug/deps/warp_viewer-498a336359877566.b7g3r3oo1zbmxsfixz5s0busx.13iivtv.rcgu.o
          /Users/james/git/echo/target/debug/deps/echo_session_service-46acf066f0ffaefb.bus7far318jkmccszitcxc7bh.1bg35s7.r
          cgu.o
          /Users/james/git/echo/target/debug/deps/atom_payload_digest_tests-aa4965e76f0e2858.82u8n586bustdnlggsmyyemf3.1j7x
          36x.rcgu.o
          /Users/james/git/echo/target/debug/deps/atom_payload_digest_tests-aa4965e76f0e2858.82u8n586bustdnlggsmyyemf3.0by0
          oub.rcgu.o
          /Users/james/git/echo/target/debug/deps/atom_payload_digest_tests-aa4965e76f0e2858.82u8n586bustdnlggsmyyemf3.0uqy
          z8y.rcgu.o
          /Users/james/git/echo/target/debug/deps/atom_payload_digest_tests-aa4965e76f0e2858.82u8n586bustdnlggsmyyemf3.0sfy
          xxh.rcgu.o
          /Users/james/git/echo/target/debug/deps/warp_core-d87237a412a2c68f.a87ze4busok9irsb8p959hp34.0ir2rx5.rcgu.o
          /Users/james/git/echo/target/debug/deps/warp_core-d87237a412a2c68f.a87ze4busok9irsb8p959hp34.1a56pzh.rcgu.o
          /Users/james/git/echo/.venv/lib/python3.14/site-packages/pandas/tests/tseries/offsets/test_business_quarter.py
          /Users/james/git/echo/.venv/lib/python3.14/site-packages/pandas/tests/tseries/offsets/test_custom_business_month.
          py
          /Users/james/git/echo/.venv/lib/python3.14/site-packages/pandas/tests/tseries/offsets/test_business_day.py
          /Users/james/git/echo/.venv/lib/python3.14/site-packages/pandas/tests/tseries/offsets/test_custom_business_day.py
          /Users/james/git/echo/.venv/lib/python3.14/site-packages/pandas/tests/tseries/offsets/test_business_hour.py
          /Users/james/git/echo/.venv/lib/python3.14/site-packages/pandas/tests/tseries/offsets/test_custom_business_hour.p
          y
          /Users/james/git/echo/.venv/lib/python3.14/site-packages/pandas/tests/tseries/offsets/test_business_year.py
          /Users/james/git/echo/.venv/lib/python3.14/site-packages/pandas/tests/tseries/offsets/test_business_month.py
          /Users/james/git/echo/.venv/lib/python3.14/site-packages/pandas/tests/tseries/offsets/**pycache**/test_business_d
          ay.cpython-314.pyc
          /Users/james/git/echo/.venv/lib/python3.14/site-packages/pandas/tests/tseries/offsets/**pycache**/test_business_h
          our.cpython-314.pyc
          /Users/james/git/echo/.venv/lib/python3.14/site-packages/pandas/tests/tseries/offsets/**pycache**/test_business_m
          onth.cpython-314.pyc
          /Users/james/git/echo/.venv/lib/python3.14/site-packages/pandas/tests/tseries/offsets/**pycache**/test_business_q
          uarter.cpython-314.pyc
          /Users/james/git/echo/.venv/lib/python3.14/site-packages/pandas/tests/tseries/offsets/**pycache**/test_business_y
          ear.cpython-314.pyc
          /Users/james/git/echo/.venv/lib/python3.14/site-packages/pandas/tests/tseries/offsets/**pycache**/test_custom_bus
          iness_day.cpython-314.pyc
          /Users/james/git/echo/.venv/lib/python3.14/site-packages/pandas/tests/tseries/offsets/**pycache**/test_custom_bus
          iness_hour.cpython-314.pyc
          /Users/james/git/echo/.venv/lib/python3.14/site-packages/pandas/tests/tseries/offsets/**pycache**/test_custom_bus
          iness_month.cpython-314.pyc
          /Users/james/git/echo/target/debug/deps/ledger_tests-6fb4459ad51559d5.cobxgybuskq2v1p708by3jynh.11vuaxi.rcgu.o
          /Users/james/git/echo/target/debug/deps/ledger_tests-6fb4459ad51559d5.cobxgybuskq2v1p708by3jynh.0miia7n.rcgu.o
          /Users/james/git/echo/target/debug/deps/ledger_tests-6fb4459ad51559d5.cobxgybuskq2v1p708by3jynh.0mj36ps.rcgu.o
          /Users/james/git/echo/target/debug/deps/ledger_tests-6fb4459ad51559d5.cobxgybuskq2v1p708by3jynh.1qlllej.rcgu.o
          /Users/james/git/echo/target/debug/deps/ledger_tests-6fb4459ad51559d5.cobxgybuskq2v1p708by3jynh.1cr9dlk.rcgu.o
          /Users/james/git/echo/target/debug/deps/ledger_tests-6fb4459ad51559d5.cobxgybuskq2v1p708by3jynh.09ugs3g.rcgu.o
          /Users/james/git/echo/target/debug/deps/ledger_tests-6fb4459ad51559d5.cobxgybuskq2v1p708by3jynh.0yvnnce.rcgu.o
          /Users/james/git/echo/target/debug/deps/ledger_tests-6fb4459ad51559d5.cobxgybuskq2v1p708by3jynh.1lwe92s.rcgu.o
          /Users/james/git/echo/target-clippy/debug/deps/libusvg-56c09378487b61e1.rmeta
          /Users/james/git/echo/target-test/debug/deps/libusvg-e365497d4e450faa.rmeta
          /Users/james/git/echo/target-test/debug/deps/libusvg-e365497d4e450faa.rlib
          /Users/james/git/echo/target-test/debug/incremental/echo_wasm_abi-16ox3qyzklihq/s-hey5mh83ss-19isaz6-8vcwqxf0q9as
          t9mxtmsjcmgwg/59womft9t83sbuse9a98d7hdj.o
          /Users/james/git/echo/target-test/debug/deps/echo_wasm_abi-3b5a24c1218ff99b.59womft9t83sbuse9a98d7hdj.1xjem33.rcg
          u.o
          /Users/james/git/echo/target-test/debug/deps/echo_wasm_abi-3b5a24c1218ff99b.59womft9t83sbuse9a98d7hdj.0qln340.rcg
          u.o
          /Users/james/git/echo/target-test/debug/deps/echo_wasm_abi-3b5a24c1218ff99b.59womft9t83sbuse9a98d7hdj.0n4b0wc.rcg
          u.o
          /Users/james/git/echo/target-test/debug/incremental/echo_session_proto-15c9hgpzknuyg/s-hexmd0tqpa-148oo1y-cqngx0p
          x6pp90xw8dr60wmpuk/2c76gsb0efj79x2xiczbusbpg.o
          /Users/james/git/echo/target-test/debug/deps/echo_session_proto-4345c03b4b2d3dd2.2c76gsb0efj79x2xiczbusbpg.08ow9f
          l.rcgu.o
          /Users/james/git/echo/target/debug/deps/libusvg-56c09378487b61e1.rmeta
          /Users/james/git/echo/target/debug/deps/libusvg-e365497d4e450faa.rmeta
          /Users/james/git/echo/target/debug/deps/libusvg-e365497d4e450faa.rlib
          /Users/james/git/echo/target/debug/incremental/echo_wasm_abi-16ox3qyzklihq/s-hey0hmxqb3-0tmprn8-4h2squ1s0kdabdqb6
          so6wrbph/59womft9t83sbuse9a98d7hdj.o
          /Users/james/git/echo/target/debug/deps/echo_wasm_abi-3b5a24c1218ff99b.59womft9t83sbuse9a98d7hdj.0jot3y7.rcgu.o
          /Users/james/git/echo/target/debug/deps/echo_wasm_abi-3b5a24c1218ff99b.59womft9t83sbuse9a98d7hdj.0gr8agf.rcgu.o
          /Users/james/git/echo/target/debug/incremental/echo_session_proto-15c9hgpzknuyg/s-hexmowcpzw-00z8cy4-4izq1prj46rq
          qepyfem91ad67/2c76gsb0efj79x2xiczbusbpg.o
          /Users/james/git/echo/target/debug/deps/echo_session_proto-4345c03b4b2d3dd2.2c76gsb0efj79x2xiczbusbpg.145450t.rcg
          u.o
          /Users/james/git/echo/target/debug/incremental/ledger_tests-3cr1zge85yfnj/s-hey58147lo-1kk7u0x-eupozq2hoa3rc89a41
          s9qhw3x/cobxgybuskq2v1p708by3jynh.o
          /Users/james/git/echo/target/debug/deps/ledger_tests-6fb4459ad51559d5.cobxgybuskq2v1p708by3jynh.0ic3fem.rcgu.o
          /Users/james/git/echo/target/debug/deps/ledger_tests-6fb4459ad51559d5.cobxgybuskq2v1p708by3jynh.1k2o66w.rcgu.o
          /Users/james/git/echo/target/debug/deps/ledger_tests-6fb4459ad51559d5.cobxgybuskq2v1p708by3jynh.1q8i4ju.rcgu.o
          /Users/james/git/echo/target/debug/deps/ledger_tests-6fb4459ad51559d5.cobxgybuskq2v1p708by3jynh.1jxkgiw.rcgu.o
          /Users/james/git/echo/target/debug/deps/ledger_tests-6fb4459ad51559d5.cobxgybuskq2v1p708by3jynh.0xvrtpz.rcgu.o
          /Users/james/git/echo/target/debug/deps/ledger_tests-6fb4459ad51559d5.cobxgybuskq2v1p708by3jynh.0ykmq1l.rcgu.o
          /Users/james/git/echo/target/debug/deps/ledger_tests-6fb4459ad51559d5.cobxgybuskq2v1p708by3jynh.1k9k0vl.rcgu.o
          /Users/james/git/echo/target/debug/deps/ledger_tests-6fb4459ad51559d5.cobxgybuskq2v1p708by3jynh.1oil3i5.rcgu.o
          /Users/james/git/echo/target/debug/deps/ledger_tests-6fb4459ad51559d5.cobxgybuskq2v1p708by3jynh.1vkx5co.rcgu.o
          /Users/james/git/echo/target/debug/deps/ledger_tests-6fb4459ad51559d5.cobxgybuskq2v1p708by3jynh.0xnv2vt.rcgu.o
          /Users/james/git/echo/target/debug/deps/ledger_tests-6fb4459ad51559d5.cobxgybuskq2v1p708by3jynh.1y00ozh.rcgu.o
          /Users/james/git/echo/target/debug/deps/ledger_tests-6fb4459ad51559d5.cobxgybuskq2v1p708by3jynh.1x2vx71.rcgu.o
          /Users/james/git/echo/target/debug/deps/ledger_tests-6fb4459ad51559d5.cobxgybuskq2v1p708by3jynh.1n0bcq8.rcgu.o
          /Users/james/git/echo/target/debug/deps/ledger_tests-6fb4459ad51559d5.cobxgybuskq2v1p708by3jynh.1nzunit.rcgu.o
          /Users/james/git/echo/target/debug/deps/ledger_tests-6fb4459ad51559d5.cobxgybuskq2v1p708by3jynh.0ra8wep.rcgu.o
          /Users/james/git/echo/target/debug/deps/ledger_tests-6fb4459ad51559d5.cobxgybuskq2v1p708by3jynh.023ynya.rcgu.o
          /Users/james/git/echo/target-doc/doc/src/warp_core/materialization/bus.rs.html
          /Users/james/git/echo/docs/rfc/mat-bus-finish.md
          /Users/james/git/echo/crates/warp-core/src/materialization/bus.rs
          /Users/james/git/echo/.git/refs/heads/materialization-bus
          /Users/james/git/echo/.git/logs/refs/heads/materialization-bus
          /Users/james/git/echo/target/doc/src/warp_core/materialization/bus.rs.html
          /Users/james/git/echo/.git/refs/remotes/origin/materialization-bus
          /Users/james/git/echo/.git/logs/refs/remotes/origin/materialization-bus
          /Users/james/git/echo/target/release/deps/libusvg-ee7552b6ee3198dd.rmeta
          /Users/james/git/echo/target/release/deps/libusvg-ee7552b6ee3198dd.rlib
  ⎿    ⎿  Found 100 files
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.458o3anq7v5ei1q8z20540fup.0n
          9nxey.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.458o3anq7v5ei1q8z20540fup.06
          x6107.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.458o3anq7v5ei1q8z20540fup.1c
          ut5l8.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.458o3anq7v5ei1q8z20540fup.0p
          bjco0.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.458o3anq7v5ei1q8z20540fup.1e
          raiy7.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.458o3anq7v5ei1q8z20540fup.18
          uyui9.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.458o3anq7v5ei1q8z20540fup.03
          1pzv7.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.458o3anq7v5ei1q8z20540fup.0b
          ce2b9.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.458o3anq7v5ei1q8z20540fup.1g
          d1srp.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.458o3anq7v5ei1q8z20540fup.1r
          1fz76.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.458o3anq7v5ei1q8z20540fup.1u
          xcmjw.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.458o3anq7v5ei1q8z20540fup.0l
          ncwqm.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.458o3anq7v5ei1q8z20540fup.19
          klqm3.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.458o3anq7v5ei1q8z20540fup.1h
          bam4u.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.458o3anq7v5ei1q8z20540fup.0n
          em2mg.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.458o3anq7v5ei1q8z20540fup.1x
          1wyfh.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.9sywwjg0h73tyozwu3lguqdr7.1g
          d1srp.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.9sywwjg0h73tyozwu3lguqdr7.0l
          ncwqm.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.9sywwjg0h73tyozwu3lguqdr7.1u
          xcmjw.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.9sywwjg0h73tyozwu3lguqdr7.1r
          1fz76.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.9sywwjg0h73tyozwu3lguqdr7.0n
          em2mg.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.9sywwjg0h73tyozwu3lguqdr7.19
          klqm3.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.9sywwjg0h73tyozwu3lguqdr7.1h
          bam4u.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.9sywwjg0h73tyozwu3lguqdr7.1x
          1wyfh.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.9sywwjg0h73tyozwu3lguqdr7.06
          x6107.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.9sywwjg0h73tyozwu3lguqdr7.0n
          9nxey.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.9sywwjg0h73tyozwu3lguqdr7.1c
          ut5l8.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.9sywwjg0h73tyozwu3lguqdr7.18
          uyui9.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.9sywwjg0h73tyozwu3lguqdr7.1e
          raiy7.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.9sywwjg0h73tyozwu3lguqdr7.03
          1pzv7.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.9sywwjg0h73tyozwu3lguqdr7.0b
          ce2b9.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.0mwcd2xzxpyz143zpqb1b3ukp.06
          x6107.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.0mwcd2xzxpyz143zpqb1b3ukp.0n
          9nxey.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.0mwcd2xzxpyz143zpqb1b3ukp.1c
          ut5l8.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.0mwcd2xzxpyz143zpqb1b3ukp.1e
          raiy7.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.0mwcd2xzxpyz143zpqb1b3ukp.0p
          bjco0.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.0mwcd2xzxpyz143zpqb1b3ukp.18
          uyui9.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.0mwcd2xzxpyz143zpqb1b3ukp.03
          1pzv7.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.0mwcd2xzxpyz143zpqb1b3ukp.0b
          ce2b9.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.0mwcd2xzxpyz143zpqb1b3ukp.1g
          d1srp.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.0mwcd2xzxpyz143zpqb1b3ukp.1u
          xcmjw.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.0mwcd2xzxpyz143zpqb1b3ukp.1r
          1fz76.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.0mwcd2xzxpyz143zpqb1b3ukp.0l
          ncwqm.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.0mwcd2xzxpyz143zpqb1b3ukp.19
          klqm3.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.0mwcd2xzxpyz143zpqb1b3ukp.1h
          bam4u.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.0mwcd2xzxpyz143zpqb1b3ukp.0n
          em2mg.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.0mwcd2xzxpyz143zpqb1b3ukp.1x
          1wyfh.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.5ljy85n9s1fqeqxazu3at72hx.03
          1pzv7.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.5ljy85n9s1fqeqxazu3at72hx.0n
          9nxey.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.5ljy85n9s1fqeqxazu3at72hx.1x
          1wyfh.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.5ljy85n9s1fqeqxazu3at72hx.1r
          1fz76.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.5g738ui0p7faehn5splq2tldc.1g
          d1srp.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.5g738ui0p7faehn5splq2tldc.1r
          1fz76.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.5g738ui0p7faehn5splq2tldc.1u
          xcmjw.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.5g738ui0p7faehn5splq2tldc.0l
          ncwqm.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.5g738ui0p7faehn5splq2tldc.1h
          bam4u.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.5g738ui0p7faehn5splq2tldc.19
          klqm3.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.5g738ui0p7faehn5splq2tldc.0n
          em2mg.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.5g738ui0p7faehn5splq2tldc.1x
          1wyfh.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.5g738ui0p7faehn5splq2tldc.1c
          ut5l8.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.5g738ui0p7faehn5splq2tldc.0n
          9nxey.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.5g738ui0p7faehn5splq2tldc.06
          x6107.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.5g738ui0p7faehn5splq2tldc.0p
          bjco0.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.5g738ui0p7faehn5splq2tldc.1e
          raiy7.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.5g738ui0p7faehn5splq2tldc.18
          uyui9.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.5g738ui0p7faehn5splq2tldc.03
          1pzv7.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.5g738ui0p7faehn5splq2tldc.0b
          ce2b9.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.2dx9lk4m6lxh7j4to31m8hz4z.03
          1pzv7.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.8a85nivbd6thl615y0udnt1iu.03
          1pzv7.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.f1nwe9ovq4s2jqnwet5x8y051.0n
          em2mg.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.f1nwe9ovq4s2jqnwet5x8y051.1h
          bam4u.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.f1nwe9ovq4s2jqnwet5x8y051.19
          klqm3.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.f1nwe9ovq4s2jqnwet5x8y051.1x
          1wyfh.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.f1nwe9ovq4s2jqnwet5x8y051.1g
          d1srp.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.f1nwe9ovq4s2jqnwet5x8y051.0l
          ncwqm.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.f1nwe9ovq4s2jqnwet5x8y051.1r
          1fz76.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.f1nwe9ovq4s2jqnwet5x8y051.1u
          xcmjw.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.f1nwe9ovq4s2jqnwet5x8y051.03
          1pzv7.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.f1nwe9ovq4s2jqnwet5x8y051.0b
          ce2b9.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.f1nwe9ovq4s2jqnwet5x8y051.1c
          ut5l8.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.f1nwe9ovq4s2jqnwet5x8y051.0n
          9nxey.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.f1nwe9ovq4s2jqnwet5x8y051.06
          x6107.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.f1nwe9ovq4s2jqnwet5x8y051.18
          uyui9.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.f1nwe9ovq4s2jqnwet5x8y051.0p
          bjco0.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.f1nwe9ovq4s2jqnwet5x8y051.1e
          raiy7.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.06mkffwp3ptvvuyqw4m5zi87w.0n
          9nxey.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.06mkffwp3ptvvuyqw4m5zi87w.03
          1pzv7.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.1cxt0lzvzr446desi43238t6m.03
          1pzv7.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.1cxt0lzvzr446desi43238t6m.0b
          ce2b9.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.1cxt0lzvzr446desi43238t6m.1c
          ut5l8.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.1cxt0lzvzr446desi43238t6m.06
          x6107.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.1cxt0lzvzr446desi43238t6m.0n
          9nxey.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.1cxt0lzvzr446desi43238t6m.18
          uyui9.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.1cxt0lzvzr446desi43238t6m.1e
          raiy7.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.1cxt0lzvzr446desi43238t6m.0p
          bjco0.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.1cxt0lzvzr446desi43238t6m.0n
          em2mg.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.1cxt0lzvzr446desi43238t6m.1h
          bam4u.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.1cxt0lzvzr446desi43238t6m.19
          klqm3.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.1cxt0lzvzr446desi43238t6m.1x
          1wyfh.rcgu.o
          /Users/james/git/echo/target/debug/deps/materialization_determinism-001ea16f8c1d82dc.1cxt0lzvzr446desi43238t6m.1g
          d1srp.rcgu.o
  ⎿  Read(/Users/james/git/echo/crates/warp-core/src/materialization/bus.rs)

  ⎿  Read(/Users/james/git/echo/docs/adr/ADR-0003-Materialization-Bus.md)

  ⎿  Read(/Users/james/git/echo/crates/warp-core/src/materialization/mod.rs)

  ⎿  Read(/Users/james/git/echo/crates/warp-core/src/materialization/frame.rs)

  ⎿  Read 488 lines

  ⎿  Read 126 lines

  ⎿  Read 65 lines

  ⎿  Read 255 lines

  ⎿  Search(pattern: "state_root|patch.*digest|hash.*state", glob: "**/*.rs", output_mode: "content", head_limit: 50)

  ⎿  Read(/Users/james/git/echo/crates/warp-core/src/materialization/emit_key.rs)

  ⎿  Read(/Users/james/git/echo/crates/warp-core/src/materialization/channel.rs)

  ⎿  Read 165 lines

  ⎿  Read 177 lines

  ⎿    ⎿  Found 50 lines
          crates/warp-core/src/graph.rs:414:    /// Computes a canonical hash of the entire graph state.
          crates/warp-core/src/constants.rs:8:/// This value is committed into both `patch_digest` (tick patches) and
          crates/warp-core/src/snapshot.rs:7://! definitions of `state_root` (graph-only hash) and `commit hash` (aka
          crates/warp-core/src/snapshot.rs:8://! `commit_id`: `state_root` + metadata + parents).
          crates/warp-core/src/snapshot.rs:11://! - The graph state hash (`state_root`) is a BLAKE3 digest over a canonical
          crates/warp-core/src/snapshot.rs:45:/// `state_root` (graph-only hash) and commit metadata (parents, digests,
          crates/warp-core/src/snapshot.rs:52:    /// Canonical commit hash derived from `state_root` + metadata (see
          below).
          crates/warp-core/src/snapshot.rs:69:    /// commit id commits to `state_root` and `patch_digest` (plus
          parents/policy),
          crates/warp-core/src/snapshot.rs:71:    pub patch_digest: Hash,
          crates/warp-core/src/snapshot.rs:88:pub(crate) fn compute_state_root(state: &WarpState, root: &NodeKey) -> Hash {
          crates/warp-core/src/snapshot.rs:211:/// Computes the final commit hash from the state root and metadata digests.
          crates/warp-core/src/snapshot.rs:216:    state_root: &Hash,
          crates/warp-core/src/snapshot.rs:232:    h.update(state_root);
          crates/warp-core/src/snapshot.rs:242:/// Commit hash v2 commits only to the replay boundary artifact: `state_root`
          crates/warp-core/src/snapshot.rs:243:/// and the tick `patch_digest` (plus explicit parents and policy id).
          crates/warp-core/src/snapshot.rs:245:    state_root: &Hash,
          crates/warp-core/src/snapshot.rs:247:    patch_digest: &Hash,
          crates/warp-core/src/snapshot.rs:258:    // State root + patch digest + policy id.
          crates/warp-core/src/snapshot.rs:259:    h.update(state_root);
          crates/warp-core/src/snapshot.rs:260:    h.update(patch_digest);
          crates/warp-core/src/tick_patch.rs:295:/// The patch digest commits to:
          crates/warp-core/src/tick_patch.rs:345:        let digest = compute_patch_digest_v2(
          crates/warp-core/src/tick_patch.rs:414:        let expected = compute_patch_digest_v2(
          crates/warp-core/src/tick_patch.rs:750:    /// The patch digest did not match its contents.
          crates/warp-core/src/tick_patch.rs:751:    #[error("patch digest mismatch")]
          crates/warp-core/src/tick_patch.rs:755:fn compute_patch_digest_v2(
          crates/warp-core/src/tick_patch.rs:802:/// Encodes ops into the patch digest stream.
          crates/warp-core/src/tick_patch.rs:805:/// stable, versioned encoding for hashing (`patch_digest`). They are
          crates/warp-core/src/tick_patch.rs:949:///    edits suitable for deterministic replay and hashing
          (`patch_digest`).
          crates/warp-core/src/tick_patch.rs:978:///   for both replay ordering and `patch_digest` hashing.
          crates/warp-core/tests/dpo_concurrency_litmus.rs:111:        a.patch_digest, b.patch_digest,
          crates/warp-core/tests/dpo_concurrency_litmus.rs:112:        "patch digest must be order-independent"
          crates/warp-core/src/engine_impl.rs:20:use crate::snapshot::{compute_commit_hash_v2, compute_state_root,
          Snapshot};
          crates/warp-core/src/engine_impl.rs:233:    /// The policy ID is committed into `patch_digest` and `commit_id` v2.
          crates/warp-core/src/engine_impl.rs:317:    /// Policy identifier committed into `patch_digest` (tick patches) and
          crates/warp-core/src/engine_impl.rs:378:    /// `policy_id` is committed into both `patch_digest` (tick patches)
          and
          crates/warp-core/src/engine_impl.rs:395:    /// - `policy_id`: Policy identifier committed into `patch_digest` and
           `commit_id` v2.
          crates/warp-core/src/engine_impl.rs:415:    /// - `policy_id`: Policy identifier committed into `patch_digest` and
           `commit_id` v2.
          crates/warp-core/src/engine_impl.rs:444:    /// - `policy_id`: Policy identifier committed into `patch_digest` and
           `commit_id` v2.
          crates/warp-core/src/engine_impl.rs:511:    /// - `policy_id`: policy identifier committed into `patch_digest` and
           `commit_id` v2.
          crates/warp-core/src/engine_impl.rs:581:    /// - `policy_id`: Policy identifier committed into `patch_digest` and
           `commit_id` v2.
          crates/warp-core/src/engine_impl.rs:843:        let patch_digest = patch.digest();
          crates/warp-core/src/engine_impl.rs:845:        let state_root = crate::snapshot::compute_state_root(&self.state,
          &self.current_root);
          crates/warp-core/src/engine_impl.rs:855:            &state_root,
          crates/warp-core/src/engine_impl.rs:857:            &patch_digest,
          crates/warp-core/src/engine_impl.rs:867:            patch_digest,
          crates/warp-core/src/engine_impl.rs:1000:        // same v2 commit hash shape (parents + state_root +
          patch_digest) but
          crates/warp-core/src/engine_impl.rs:1003:        let state_root = compute_state_root(&self.state,
          &self.current_root);
          crates/warp-core/src/engine_impl.rs:1015:        let patch_digest = WarpTickPatchV1::new(
          crates/warp-core/src/engine_impl.rs:1024:        let hash = compute_commit_hash_v2(&state_root, &parents,
          &patch_digest, policy_id);

  ⎿  Read(/Users/james/git/echo/crates/warp-core/src/snapshot.rs · lines 1-150)

  ⎿  Read(/Users/james/git/echo/crates/warp-core/src/tick_patch.rs · lines 1-150)

  ⎿  Read 150 lines

  ⎿  Read 150 lines

  ⎿  Read(/Users/james/git/echo/crates/warp-core/src/tick_patch.rs · lines 290-439)

  ⎿  Read(/Users/james/git/echo/crates/warp-core/src/engine_impl.rs · lines 820-899)

  ⎿  Read 150 lines

  ⎿  Read 80 lines

  ⎿  Read(/Users/james/git/echo/crates/warp-core/src/tick_patch.rs · lines 750-849)

  ⎿  Search(pattern: "fn compute_patch_digest", glob: "**/*.rs", output_mode: "content", head_limit: 20)

  ⎿  Read 100 lines

  ⎿    ⎿  Found 1 line
          crates/warp-core/src/tick_patch.rs:755:fn compute_patch_digest_v2(
  ⎿  Read(/Users/james/git/echo/docs/rfc/mat-bus-finish.md)

  ⎿  Read(/Users/james/git/echo/crates/warp-core/src/materialization/reduce_op.rs)

  ⎿  Read 661 lines

  ⎿  Read 385 lines

  ⎿  Response:
       Perfect! Now I have comprehensive information. Let me create a detailed research report.

       Research Summary: Materialization Bus Infrastructure in WARP

       Based on my thorough search of the warp-core codebase, here's what I found regarding materialization, the bus
       infrastructure, and hash computation:

       1. MaterializationBus Infrastructure

       Location: /Users/james/git/echo/crates/warp-core/src/materialization/bus.rs

       The MaterializationBus is a tick-scoped internal infrastructure that collects emissions from rewrite rules during
        a tick:

       - Core Data Structure: Uses BTreeMap<ChannelId, BTreeMap<EmitKey, Vec<u8>>> for order-independent storage
       - Thread Safety: Intentionally not thread-safe (!Sync) using RefCell for interior mutability - by design, since
       rewrite engine processes rules sequentially within a tick
       - Key Methods:
         - emit(channel, emit_key, data) - Collects emissions, rejects duplicate (channel, EmitKey) pairs
         - finalize() - Post-commit resolution according to each channel's policy, returns FinalizeReport
         - clear() - Clears pending emissions (for abort path)

       Order Independence (Confluence Safety): The BTreeMap ordering ensures finalization iterates in deterministic
       (canonical) order regardless of rewrite execution order, preserving confluence.

       2. Graph State Materialization

       Location: /Users/james/git/echo/crates/warp-core/src/materialization/mod.rs (architecture diagram)

       The materialization system follows a hexagonal architecture:

       External World (ViewOps frames)
                ▲
                │
           MaterializationPort (Boundary API)
                │
           MaterializationBus (Internal Runtime)
                │
              Engine (commit → finalize)

       Current Implementation:
       - Emissions are stored in channels keyed by EmitKey
       - Materialization frames are encoded as length-prefixed binary ("MBUS" format)
       - Support for three channel policies: Log (default), StrictSingle, and Reduce(op)

       3. EmitKey: Order-Independent Emission Identification

       Location: /Users/james/git/echo/crates/warp-core/src/materialization/emit_key.rs

       The EmitKey ensures deterministic ordering:

       pub struct EmitKey {
           pub scope_hash: Hash,           // Hash of scope node where rewrite applied
           pub rule_id: u32,               // Compact rule identifier
           pub subkey: u32,                // Differentiator for multiple emissions per rule
       }

       Ordering: Lexicographic (scope_hash, rule_id, subkey) matching scheduler's canonical ordering

       Subkey Usage: For rules emitting multiple items (e.g., iterating children), use stable subkey derived from item
       hash via EmitKey::subkey_from_hash(h: &Hash) -> u32

       4. Channel Policies and Reduce Operations

       Location: /Users/james/git/echo/crates/warp-core/src/materialization/channel.rs and
       /Users/james/git/echo/crates/warp-core/src/materialization/reduce_op.rs

       Channel Policies:


       ┌──────────────┬─────────────────────────────────────────────────┐
       │    Policy    │                    Behavior                     │
       ├──────────────┼─────────────────────────────────────────────────┤
       │ Log          │ All emissions in EmitKey order, length-prefixed │
       ├──────────────┼─────────────────────────────────────────────────┤
       │ StrictSingle │ Error if >1 emission (catches bugs)             │
       ├──────────────┼─────────────────────────────────────────────────┤
       │ Reduce(op)   │ Merge via deterministic operation               │
       └──────────────┴─────────────────────────────────────────────────┘
       ReduceOp Variants:

       Commutative Monoids (permutation-invariant):
       - Sum - u64 little-endian addition (empty→[0;8])
       - Max, Min - lexicographic comparison
       - BitOr, BitAnd - bitwise operations with padding/truncation

       Order-Dependent (deterministic via EmitKey order, NOT commutative):
       - First - take first by EmitKey order
       - Last - take last by EmitKey order
       - Concat - concatenate all in order

       All operations include is_commutative() method for runtime classification.

       5. State Root and Commit Hash Computation

       Location: /Users/james/git/echo/crates/warp-core/src/snapshot.rs

       State Root (compute_state_root):
       - BLAKE3 digest over canonical byte stream of entire reachable graph state
       - Deterministic ordering: nodes visited in ascending NodeId order, edges sorted by EdgeId
       - Includes reachability across instances via Descend attachments
       - Fixed-size, architecture-independent encoding:
         - 32-byte ids (NodeId, TypeId, EdgeId)
         - 1 byte presence tag + 32 bytes type_id + 8-byte LE length + payload bytes

       Commit Hash (v2): compute_commit_hash_v2(state_root, parents, patch_digest, policy_id)
       - Commits to: state_root + explicit parents + patch_digest + policy_id
       - Uses BLAKE3 hashing

       6. Tick Patch and Patch Digest Computation

       Location: /Users/james/git/echo/crates/warp-core/src/tick_patch.rs

       WarpTickPatchV1: The boundary artifact for deterministic replay

       Patch Digest (compute_patch_digest_v2): Commits to:
       - Patch format version (v2)
       - policy_id
       - rule_pack_id
       - commit_status (Committed/Aborted)
       - in_slots / out_slots (sorted, deduped)
       - ops (canonical delta operations)

       Digest Encoding: BLAKE3 of structured format:
       - Version as u16 LE
       - Policy ID as u32 LE
       - Rule pack ID (32 bytes)
       - Commit status code (1 byte)
       - Encoded slots (tag + ids for Node/Edge/Attachment/Port)
       - Encoded ops (tag + operation-specific fields)

       Operations (8 types):
       - OpenPortal, UpsertWarpInstance, DeleteWarpInstance
       - UpsertNode, DeleteNode
       - UpsertEdge, DeleteEdge
       - UpsertAttachment, DeleteAttachment

       7. Engine Integration

       Location: /Users/james/git/echo/crates/warp-core/src/engine_impl.rs

       Commit Flow:
       1. engine_impl.rs:829-831 - Bus finalization: let mat_report = self.bus.finalize()
       2. engine_impl.rs:834-843 - Tick patch creation with ops from state diff
       3. engine_impl.rs:843-859 - Parallel computation:
         - patch_digest = patch.digest()
         - state_root = compute_state_root(&self.state, &self.current_root)
         - hash = compute_commit_hash_v2(&state_root, &parents, &patch_digest, policy_id)
       4. Creates Snapshot with all digests and transaction tracking

       Abort Flow: engine_impl.rs:889-891
       - bus.clear() - clears pending emissions
       - last_materialization.clear() - invalidates stale cached state

       8. Determinism Testing & Design Documents

       ADR-0003 (Determinism Philosophy): /Users/james/git/echo/docs/adr/ADR-0003-Materialization-Bus.md
       - Establishes causality-first API (ingress-only writes)
       - Bus-first UI via MaterializationPort
       - Confluence-safe emission semantics with order-independent storage
       - Rejects silent "winner picks" - requires explicit policy

       RFC: MaterializationBus Completion: /Users/james/git/echo/docs/rfc/mat-bus-finish.md
       - Specifies EmissionPort trait (hexagonal boundary)
       - ScopedEmitter adapter (auto-fills EmitKey)
       - Duplicate (channel, EmitKey) rejection policy
       - Comprehensive cross-platform determinism test suite

       9. Frame Encoding Format

       Location: /Users/james/git/echo/crates/warp-core/src/materialization/frame.rs

       Binary frame format ("MBUS"):
       magic[4]    = "MBUS" (0x4D, 0x42, 0x55, 0x53)
       version[2]  = 0x0001 (little-endian)
       reserved[2] = 0x0000
       length[4]   = payload byte length (LE)
       payload[len] = channel_id[32] || data[len-32]

       All multi-byte integers are little-endian. Max payload ≈4GB (u32 limitation).

       ---
       Key Insights:

       1. No Hash Collision Risk at Materialization Level - EmitKey uses scope_hash + rule_id + subkey, making
       collisions astronomically unlikely
       2. Determinism by Construction - BTreeMaps enforce canonical ordering at every level; same emissions always
       produce same bytes
       3. Confluence-First Design - Silent data loss is prohibited; use explicit policies (Log/StrictSingle/Reduce)
       4. Tick-Scoped Lifecycle - Bus is cleared after finalization, preventing cross-tick contamination
       5. Paper III Integration - Tick patches are replayable delta artifacts sufficient for deterministic state
       recovery
  ⎿  Done (20 tool uses · 94.3k tokens · 53s)

❯ please checkout a git branch called graph-boaw for this work and be sure to git -A and git commit before ending each
  of your turns

∴ Thinking…

  The user wants me to:

  1. Create a new git branch called graph-boaw for the BOAW work
  2. Commit any changes before ending each turn

  Let me first create the branch and then continue with my analysis and implementation plan.

⏺ Let me create the branch for this work first.

⏺ Bash(git checkout -b graph-boaw)
  ⎿  Switched to a new branch 'graph-boaw'

∴ Thinking…

  Good, branch created. Now let me compile my analysis into a comprehensive implementation plan. I have a lot of context from
   the subagent explorations.

  Let me summarize what I've learned:

  Current State

  1. GraphStore (graph.rs, 568 lines): In-memory BTreeMap-based store with nodes, edges, attachments. Supports canonical
  hashing via canonical_state_hash().
  2. WSC Format (wsc/): Mature zero-copy snapshot format with:
    - WscFile / WarpView for reading
    - write_wsc_one_warp for writing
    - build_one_warp_input() for GraphStore → WSC conversion
    - All Pod types, 8-byte aligned, ready for mmap
  3. Footprint Infrastructure (footprint.rs, 250 lines): Already has:
    - Footprint struct with read/write sets for nodes, edges, attachments, ports
    - factor_mask for O(1) prefiltering
    - independent() method for conflict detection
  4. Scheduler (scheduler.rs, 1304 lines): RadixScheduler with:
    - PendingRewrite with footprint and phase tracking
    - ActiveFootprints with GenSet for O(1) conflict detection
    - O(n) radix sort for deterministic ordering
    - try_reserve() for admission control
  5. TickPatch (tick_patch.rs, 1694 lines): Has:
    - WarpOp enum (UpsertNode, DeleteNode, UpsertEdge, DeleteEdge, SetAttachment, etc.)
    - WarpTickPatchV1 with ops, in_slots, out_slots, patch_digest
    - **[Phase 5 Complete]**: Now used for execution emissions via TickDelta
  6. Engine (engine_impl.rs, 1806 lines): **[Phase 5 Complete]**: Current flow:
    - Executors receive `&GraphView` (read-only) and emit ops to `&mut TickDelta`
    - No GraphStore mutation during execution
    - State updated post-execution via `apply_to_state()`
  7. Materialization (materialization/): Mature bus system for output emissions with deterministic ordering.

  What ADR-0007 Requires

  The ADR outlines a migration plan (status as of 2026-01-18):

  1. Step 1: Introduce TickDelta + canonical merge alongside existing GraphStore **[COMPLETE]**
  2. Step 2: Change executors to emit ops into TickDelta (stop writing GraphStore) **[COMPLETE - Phase 5]**
  3. Step 3: Implement SnapshotBuilder that applies TickDelta to produce next snapshot **[COMPLETE - Phase 4]**
  4. Step 4: Wire WSC writer as canonical snapshot output **[COMPLETE]**
  5. Step 5: Read-only execution - GraphView instead of &mut GraphStore **[COMPLETE - Phase 5]**
  6. Step 6: Add collapse/merge phase with typed registry **[PENDING]**
  7. Step 7: Add mind/diagnostics enforcement gates **[PENDING]**

  My Implementation Plan

  Based on the ADR's sequencing section (lines 1062-1105), here's what I'll propose:

  Phase 1: ADR + Test Skeletons (First Commit)

- The ADR is already written - it's the file I read
- Add the test skeletons from the ADR (lines 604-1043)
- These tests will fail initially - that's intentional

  Phase 2: Wire TestHarness (Second Commit)

- Implement TestHarness against the test structure, following the ADR's guidance on how tests should be organized and what
  they should validate.

  The next phases build on this foundation: TickDelta collects operations during execution rather than mutating state
  directly, then SnapshotBuilder applies those operations to generate the next snapshot and write WSC bytes. After that, I'll
   add per-worker delta collection and prove the system produces consistent results regardless of worker count through
  targeted tests.
 Now let me synthesize all the exploration results into a comprehensive implementation plan.

  ---
  BOAW Implementation Plan

  ★ Insight ─────────────────────────────────────
  Architecture Context: The codebase already has strong foundations for BOAW:

- GraphStore uses BTreeMaps for deterministic iteration
- Footprint infrastructure already tracks read/write sets with factor_mask prefiltering
- RadixScheduler provides O(n) deterministic ordering and O(1) conflict detection via GenSet
- TickPatch has WarpOp enum for delta operations (now used for execution emissions via TickDelta)
- WSC is production-ready with zero-copy Pod types, ready for mmap

  The key pivot was: move from "mutate GraphStore → diff after" to "emit ops into TickDelta → apply to snapshot" **[COMPLETE - Phase 5]**
  ─────────────────────────────────────────────────

  ---
  Phase 1: Test Skeletons (This Commit)

  Goal: Establish the contract + drill sergeant tests

  Create test files from ADR-0007 Section 14:

- tests/common/mod.rs - XorShift64 RNG, TestHarness trait, Scenario enum
- tests/boaw_end_to_end.rs - The "god test" for permutation invariance
- tests/boaw_determinism.rs - Serial vs parallel equivalence
- tests/boaw_cow.rs - Unlink semantics, overlay precedence
- tests/boaw_footprints.rs - Independence checks, bucket enforcement
- tests/boaw_merge.rs - Collapse/merge determinism
- tests/boaw_privacy.rs - Mind mode enforcement

  These tests will intentionally panic until we wire the BOAW APIs.

  ---
  Phase 2: Wire TestHarness to Current Engine

  Goal: Tests compile and run against current pipeline

  Implement TestHarness to:

- Build base snapshots from current GraphStore
- Execute via current engine (both serial and parallel call serial for now)
- Return real hashes so test runner is alive

  ---
  Phase 3: TickDelta for Execution (The Real Pivot)

  Goal: Executors emit ops instead of mutating shared state

  Key Changes:

  1. Create TickDelta struct in src/tick_delta.rs:
  pub struct TickDelta {
      pub upsert_nodes: Vec<(NodeId, NodeRecord)>,
      pub delete_nodes: Vec<NodeId>,
      pub upsert_edges: Vec<(EdgeId, EdgeRecord)>,
      pub delete_edges: Vec<EdgeId>,
      pub set_attachments: Vec<(AttachmentKey, Option<AttachmentValue>)>,
  }
  2. Modify ExecuteFn signature to take:
    - &GraphView (read-only snapshot view) instead of &mut GraphStore
    - &mut TickDelta (append-only ops collector)
  3. Implement canonical merge - sort ops by key for deterministic ordering
  4. Add DeltaView - overlay that resolves reads from (snapshot + delta)

  **Status: COMPLETE** (2026-01-18)

  ---
  Phase 4: SnapshotBuilder

  Goal: Apply merged TickDelta to produce next WSC snapshot

  1. SnapshotBuilder takes base snapshot + merged ops
  2. Produces next reachable-only snapshot tables
  3. Writes WSC bytes via existing write_wsc_one_warp
  4. Computes state_root from materialized snapshot

  **Status: COMPLETE** (2026-01-18) - See `docs/memorials/2026-01-18-phase4-rubicon.md`

  ---
  Phase 5: Read-Only Execution

  Goal: Pure execution - executors read from snapshot, write to delta only

  1. ExecuteFn signature changed: `fn(&GraphView, &mut TickDelta, &NodeId)`
  2. No GraphStore mutations during execution - emit ops only
  3. State updated after execution via `apply_to_state()`
  4. Legacy `&mut GraphStore` path removed from executor signature

  **Status: COMPLETE** (2026-01-18)

  ---
  Phase 6: Parallel Execution with Per-Worker Deltas

  Goal: Lockless parallelism without compromising determinism

  1. Per-worker TickDelta - each worker has thread-local delta
  2. Canonical merge - concatenate then sort by (op_kind, key, tie_breaker)
  3. Prove worker-count invariance - the "god test" passes for all worker counts

  ---
  Phase 7+ (Future)

- Segment-level structural sharing (content-addressed chunks)
- Collapse/merge with typed registry
- Privacy enforcement gates (mind vs diagnostics mode)

  ---

Files to Create/Modify
  ┌──────────────────────────────────────────┬────────┬───────────────────────────────────┐
  │                   Path                   │ Action │              Purpose              │
  ├──────────────────────────────────────────┼────────┼───────────────────────────────────┤
  │ crates/warp-core/tests/boaw/mod.rs       │ Create │ Test module organization          │
  ├──────────────────────────────────────────┼────────┼───────────────────────────────────┤
  │ crates/warp-core/tests/boaw/common.rs    │ Create │ XorShift64, TestHarness, Scenario │
  ├──────────────────────────────────────────┼────────┼───────────────────────────────────┤
  │ crates/warp-core/tests/boaw_*.rs         │ Create │ 6 test files from ADR             │
  ├──────────────────────────────────────────┼────────┼───────────────────────────────────┤
  │ crates/warp-core/src/tick_delta.rs       │ Create │ TickDelta struct + merge          │
  ├──────────────────────────────────────────┼────────┼───────────────────────────────────┤
  │ crates/warp-core/src/delta_view.rs       │ Create │ Read-through overlay              │
  ├──────────────────────────────────────────┼────────┼───────────────────────────────────┤
  │ crates/warp-core/src/snapshot_builder.rs │ Create │ Apply delta → next WSC            │
  ├──────────────────────────────────────────┼────────┼───────────────────────────────────┤
  │ crates/warp-core/src/lib.rs              │ Modify │ Export new modules                │
  ├──────────────────────────────────────────┼────────┼───────────────────────────────────┤
  │ crates/warp-core/src/engine_impl.rs      │ Modify │ New ExecuteFn signature           │
  ├──────────────────────────────────────────┼────────┼───────────────────────────────────┤
  │ crates/warp-core/src/rule.rs             │ Modify │ Update ExecuteFn type             │
  └──────────────────────────────────────────┴────────┴───────────────────────────────────┘
  ---

  Immediate Next Steps

  1. Create the test skeleton files from ADR-0007 Section 14
  2. Add a PanicHarness that will fail loudly until wired
  3. Commit as "test: add BOAW compliance test skeletons (red)"

  Would you like me to proceed with Phase 1 - adding the test skeletons?

  ---
  HOO RAH! 🎖️

  I'm ready to begin implementation upon your approval. The approach follows the ADR's migration plan exactly: start with failing tests that define the contract, then implement piece by piece until they pass.
