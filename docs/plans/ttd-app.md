<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# TTD Application Master Plan

**Status:** Draft
**Created:** 2026-01-25
**Scope:** Complete Time Travel Debugger architecture from protocol to browser UI

---

## Executive Summary

This plan transforms the WARPSITE demo concept into a full **Time Travel Debugger (TTD)** application. The TTD proves determinism by running Echo entirely in-browser via WASM, letting users fork reality, step multiple timelines, and verify hash equality.

The system is built on three pillars:

1. **Wesley** — Schema compiler generating typed protocols, registries, and enforcement tables
2. **TTD Core** — Compliance engine, receipts, obligations, and deterministic verification
3. **TTD App** — Browser-based debugger with 3D provenance visualization

### Key Doctrine

> "Clients are dumb. The browser becomes the prover."

- No protocol logic in UI — only render authoritative truth frames
- Determinism is the product — every byte is canonical and hashable
- Everything is versioned — schema_hash is the universe identity

---

## Part 1: What Exists Today

### Implemented Infrastructure

| Component               | Location                                     | Status                                |
| ----------------------- | -------------------------------------------- | ------------------------------------- |
| MaterializationBus      | `warp-core/src/materialization/bus.rs`       | ✅ Complete                           |
| MBUS Frame V1/V2        | `warp-core/src/materialization/frame*.rs`    | ✅ Complete                           |
| Channel Policies        | `warp-core/src/materialization/channel.rs`   | ✅ Log, StrictSingle, Reduce          |
| Reduce Operations       | `warp-core/src/materialization/reduce_op.rs` | ✅ Sum, Max, Min, First, Last, Concat |
| PlaybackCursor          | `warp-core/src/playback.rs`                  | ✅ Seek, step, verify                 |
| ViewSession + TruthSink | `warp-core/src/playback.rs`                  | ✅ Subscriptions, TruthFrames         |
| ProvenanceStore         | `warp-core/src/provenance_store.rs`          | ✅ Patches, expected, outputs         |
| Footprint Enforcement   | `warp-core/src/footprint_guard.rs`           | ✅ Phase 6B complete                  |
| JS-ABI Wire Protocol    | `echo-session-proto/src/wire.rs`             | ✅ CBOR, OpEnvelope                   |
| WS Gateway              | `echo-session-ws-gateway/`                   | ✅ Tunneling                          |
| warp-viewer (3D)        | `crates/warp-viewer/`                        | ✅ WGPU + egui                        |
| warp-wasm               | `crates/warp-wasm/`                          | ⚠️ Placeholder bindings               |

### Key Types Already Defined

```rust
// Identifiers (warp-core/src/ident.rs)
pub type Hash = [u8; 32];
pub struct NodeId(Hash);
pub struct EdgeId(Hash);
pub struct TypeId(Hash);
pub type ChannelId = TypeId;
pub struct WarpId(Hash);
pub struct WorldlineId(Hash);
pub struct SessionId(Hash);
pub struct CursorId(Hash);

// Playback (warp-core/src/playback.rs)
pub enum PlaybackMode { Paused, Play, StepForward, StepBack, Seek { target, then } }
pub enum CursorRole { Writer, Reader }
pub struct CursorReceipt { session_id, cursor_id, worldline_id, warp_id, tick, commit_hash }
pub struct TruthFrame { cursor: CursorReceipt, channel, value, value_hash }

// Provenance (warp-core/src/provenance_store.rs)
pub struct HashTriplet { state_root, patch_digest, commit_hash }
pub struct WorldlineTickPatchV1 { header, warp_id, ops, in_slots, out_slots, patch_digest }
pub type OutputFrameSet = Vec<(ChannelId, Vec<u8>)>;

// Footprints (warp-core/src/footprint.rs)
pub struct Footprint { n_read, n_write, e_read, e_write, a_read, a_write, b_in, b_out, factor_mask }
pub enum ViolationKind { NodeReadNotDeclared, NodeWriteNotDeclared, CrossWarpEmission, ... }
```

### Gaps Identified

| Gap                      | Impact                                     | Solution                           |
| ------------------------ | ------------------------------------------ | ---------------------------------- |
| Rule-to-atom attribution | Provenance queries incomplete              | Add AtomWrite tracking             |
| Emission recording       | Can't verify observable truth              | Add emissions_digest to receipts   |
| Op registry              | No typed op validation                     | Wesley generates ttd.ops channel   |
| Rule contracts           | Footprints exist, emission contracts don't | Wesley generates ttd.rules channel |
| WASM engine bindings     | Placeholder stubs                          | Wire warp-core to warp-wasm        |
| Browser UI               | None                                       | Build TTD app with Three.js        |

---

## Part 2: Wesley Schema Compiler

Wesley is the schema compiler that generates typed protocols, registries, and enforcement tables from annotated GraphQL SDL.

### 2.1 Directive Vocabulary

```graphql
# ─── Determinism / Canonicalization ───────────────────────────────
directive @canonicalCbor(version: U32 = 1) on OBJECT | FIELD_DEFINITION
directive @noFloat on OBJECT | FIELD_DEFINITION
directive @fixed(kind: String!, scale: I32) on FIELD_DEFINITION
directive @sorted(by: [String!]!) on FIELD_DEFINITION
directive @noUnorderedMap on OBJECT | FIELD_DEFINITION
directive @keyBytes on FIELD_DEFINITION

# ─── Channel Registry ─────────────────────────────────────────────
enum ChannelPolicy {
    LOG
    STRICT_SINGLE
    REDUCE
}
enum ReducerKind {
    LAST
    FIRST
    CONCAT
    SUM
    MAX
    MIN
    CANONICAL_MERGE
}

directive @channel(
    id: ChannelId!
    version: U16!
    policy: ChannelPolicy!
    reducer: ReducerKind
    doc: String
) on OBJECT

directive @emitKey(type: String!) on OBJECT
directive @entryType(name: String!) on OBJECT

# ─── Op Registry ──────────────────────────────────────────────────
enum OpKind {
    COMMAND
    QUERY
    EVENT
}

directive @op(
    opcode: String!
    version: U16!
    kind: OpKind!
    response: String
    doc: String
) on OBJECT

directive @opError(code: String!, severity: String = "ERROR") on OBJECT

# ─── Rule Contracts ───────────────────────────────────────────────
directive @rule(id: RuleId!, version: U16!) on OBJECT
directive @triggerOp(opcode: String!, phase: String) on OBJECT
directive @triggerEvent(eventKind: String!) on OBJECT
directive @footprintRead(kind: String!, argType: String) on OBJECT
directive @footprintWrite(kind: String!, argType: String) on OBJECT

enum EmitCount {
    EXACTLY_ONE
    AT_LEAST_ONE
    ZERO_OR_MORE
}
directive @mustEmit(channel: ChannelId!, count: EmitCount!) on OBJECT
directive @mayEmitOnly(channels: [ChannelId!]!) on OBJECT
directive @ruleDeterminism(kind: String!, detail: String) on OBJECT
directive @noSideEffects(kinds: [String!]!) on OBJECT

# ─── Global Invariants ────────────────────────────────────────────
enum InvariantSeverity {
    INFO
    WARN
    ERROR
    FATAL
}

directive @invariant(
    id: String!
    severity: InvariantSeverity!
    kind: String! # "TICK" | "EVENTUAL" | "SAFETY"
    expr: String!
    doc: String
) on SCHEMA
```

### 2.2 Example Schema Usage

```graphql
schema
  @invariant(id: "TICK_EMITS_STATE", severity: FATAL, kind: "TICK",
    expr: 'tick.mustEmit("ttd.state", EXACTLY_ONE)')
  @invariant(id: "SEEK_PRODUCES_HEAD", severity: ERROR, kind: "EVENTUAL",
    expr: 'op.produces("TTD_SEEK", "ttd.head", EXACTLY_ONE, within 3)')
{
  query: Query
}

# Channel payload type
type TtdStatePayload
  @channel(id: "ttd.state", version: 1, policy: STRICT_SINGLE, reducer: LAST)
  @canonicalCbor
  @noUnorderedMap
{
  header: StateHeader!
  atoms: [AtomRecord!]! @sorted(by: ["atomId"])
}

# Op request type
type CmdSeek
  @op(opcode: "TTD_SEEK", version: 1, kind: COMMAND, response: "Ack")
  @canonicalCbor
{
  cursorId: Bytes! @keyBytes
  tick: Tick!
}

# Rule contract (metadata only)
type RuleContract_MovePlayer
  @rule(id: "rule.move_player", version: 1)
  @triggerOp(opcode: "APP_INPUT", phase: "post")
  @footprintRead(kind: "AtomId", argType: "AtomId")
  @footprintWrite(kind: "AtomId", argType: "AtomId")
  @mustEmit(channel: "ttd.state", count: EXACTLY_ONE)
  @mayEmitOnly(channels: ["ttd.state", "ttd.provenance", "ttd.tick"])
  @noSideEffects(kinds: ["time", "random", "io"])
{
  _manifestOnly: String
}
```

### 2.3 Wesley Compiler Outputs

#### A) Type/Codegen Outputs (what devs use)

| Output          | Location                                   | Contents                       |
| --------------- | ------------------------------------------ | ------------------------------ |
| Rust types      | `crates/ttd-protocol-rs/src/types.rs`      | Structs/enums for ops/channels |
| CBOR codecs     | `crates/ttd-protocol-rs/src/cbor.rs`       | Canonical encode/decode        |
| Rust registries | `crates/ttd-protocol-rs/src/registry.rs`   | Op/channel lookup tables       |
| Hash helpers    | `crates/ttd-protocol-rs/src/hash.rs`       | Digest functions               |
| TS types        | `packages/ttd-protocol-ts/src/types.ts`    | TypeScript types               |
| Zod validators  | `packages/ttd-protocol-ts/src/zod.ts`      | Runtime validation             |
| TS registries   | `packages/ttd-protocol-ts/src/registry.ts` | Op/channel tables              |

#### B) Manifest/Enforcement Outputs (what keeps you honest)

| Output             | Location                                        | Contents                        |
| ------------------ | ----------------------------------------------- | ------------------------------- |
| Channel registry   | `crates/ttd-manifest/src/channel_registry.rs`   | IDs, policies, reducers         |
| Op registry        | `crates/ttd-manifest/src/op_registry.rs`        | Opcodes, versions, types        |
| Rule contracts     | `crates/ttd-manifest/src/rule_contracts.rs`     | Triggers, footprints, emissions |
| Invariants         | `crates/ttd-manifest/src/invariants.rs`         | Compiled invariant programs     |
| Footprint specs    | `crates/ttd-manifest/src/footprint_specs.rs`    | Declared read/write sets        |
| Emission contracts | `crates/ttd-manifest/src/emission_contracts.rs` | mustEmit/mayEmitOnly tables     |

#### C) Docs & Golden Tests

| Output       | Location                         | Contents          |
| ------------ | -------------------------------- | ----------------- |
| Channel docs | `docs/generated/ttd_channels.md` | Auto-generated    |
| Op docs      | `docs/generated/ttd_ops.md`      | Auto-generated    |
| Golden tests | `fixtures/ttd/*.cbor`            | Canonical vectors |

---

## Part 3: Receipt & Digest System

### 3.1 EINT v2 — Intent Envelope

Extends the current JS-ABI envelope with schema_hash and checksums.

```
EINT v2 Wire Format (Little-Endian)
─────────────────────────────────────────────────────────────────
offset size  field
0      4     magic = ASCII "EINT"
4      2     envelope_version = u16 LE (2)
6      2     flags = u16 LE
8      32    schema_hash = [u8;32]

40     4     opcode = u32 LE
44     2     op_version = u16 LE
46     2     reserved = u16 LE (0)

48     4     payload_len = u32 LE
52     32    payload_checksum = blake3(payload_bytes)

84     N     payload_bytes (canonical CBOR)
─────────────────────────────────────────────────────────────────
```

**Flags (u16):**

- bit0: HAS_RESPONSE_ID
- bit1: COMPRESSED
- others reserved

### 3.2 TTDR v2 — Tick Receipt Record

Extends the receipt to commit to emissions and enable proof mode.

```
TTDR v2 Wire Format (Little-Endian)
─────────────────────────────────────────────────────────────────
offset size  field
0      4     magic = ASCII "TTDR"
4      2     receipt_version = u16 LE (2)
6      2     flags = u16 LE

8      32    schema_hash = [u8;32]
40     32    worldline_id = [u8;32]
72     8     tick = u64 LE

80     32    commit_hash = [u8;32]
112    32    patch_digest = [u8;32]
144    32    state_root = [u8;32]         (zero if absent)
176    32    emissions_digest = [u8;32]   ← NEW
208    32    op_emission_index_digest     ← NEW (optional, zero if absent)

240    2     parent_count = u16 LE
242    2     channel_count = u16 LE

244    32*P  parent_hashes [P][32]
...    var   channel_digests (per channel)
─────────────────────────────────────────────────────────────────
```

**Flags (u16):**

- bit0: HAS_STATE_ROOT
- bit1: HAS_OP_EMISSION_INDEX_DIGEST
- bit2: HAS_CHANNEL_PAYLOAD_HASH
- bit3: HAS_ENTRY_HASHES
- bit4: RECEIPT_MODE (2 bits, see below)

### 3.2.1 Receipt Compression Modes

To control memory and bandwidth in browser/streaming contexts:

| Mode | Name     | Contents                                          | Use Case                      |
| ---- | -------- | ------------------------------------------------- | ----------------------------- |
| 0    | FULL     | All fields, all channel digests, all entry hashes | Archive, verification         |
| 1    | PROOF    | Hashes + digests only (no payload bodies)         | Proof exchange, sync          |
| 2    | LIGHT    | commit_hash + emissions_digest + state_root only  | Streaming, memory-constrained |
| 3    | RESERVED | —                                                 | Future use                    |

**Mode encoding:** bits 4-5 of flags field.

```rust
pub enum ReceiptMode {
    Full = 0,   // Everything
    Proof = 1,  // Hashes only, no bodies
    Light = 2,  // Minimal: commit + emissions + state
}

impl TtdrV2 {
    pub fn to_mode(&self, mode: ReceiptMode) -> TtdrV2 {
        match mode {
            ReceiptMode::Full => self.clone(),
            ReceiptMode::Proof => TtdrV2 {
                // Strip payload bodies, keep hashes
                channel_digests: self.channel_digests.clone(),
                ..self.header_only()
            },
            ReceiptMode::Light => TtdrV2 {
                // Minimal: just the three hashes
                commit_hash: self.commit_hash,
                emissions_digest: self.emissions_digest,
                state_root: self.state_root,
                ..Default::default()
            },
        }
    }
}
```

**Policy:** Browser clients default to LIGHT mode. Proof verification requires FULL or PROOF.

### 3.3 Digest Definitions (Normative)

#### emissions_digest:v1

```
EmissionsDigestInput bytes =
  for channel in channels_sorted_by(channel_id_bytes):
    write(channel_id_bytes)                  # 32 bytes
    write(u16_le(channel_version))
    write(u32_le(entry_count))
    for entry in entries_sorted_by(emit_key_bytes):
      write(u32_le(key_len))
      write(key_bytes)
      write(hash32(entry_value_bytes))       # MBUS v2 entry hash

emissions_digest = BLAKE3("emissions:v1" || EmissionsDigestInput)
```

#### op_emission_index_digest:v1 (optional)

```
OpIndexDigestInput =
  for op in ops_sorted_by(op_ix):
    write(u32_le(op_ix))
    write(rule_id_bytes?)                    # if present
    write(u32_le(ref_count))
    for ref in refs_sorted_by(channel_id, emit_key, value_hash):
      write(channel_id_bytes)
      write(emit_key_bytes)
      write(value_hash32)

op_emission_index_digest = BLAKE3("op_emissions:v1" || OpIndexDigestInput)
```

#### commit_hash:v2

```
commit_hash = BLAKE3(
  "tick_commit:v2" ||
  schema_hash ||
  worldline_id ||
  u64_le(tick) ||
  parent_hashes_sorted ||
  patch_digest ||
  (state_root if present) ||
  emissions_digest ||
  (op_emission_index_digest if present)
)
```

### 3.4 AtomWrite Tracking

New provenance structure for rule-to-atom attribution:

```rust
// crates/warp-core/src/provenance_store.rs

pub struct AtomWrite {
    pub atom_id: NodeId,
    pub rule_id: RuleId,
    pub tick: u64,
    pub old_value: Option<Vec<u8>>,
    pub new_value: Vec<u8>,
}

// Extend WorldlineHistory
pub struct WorldlineHistory {
    pub u0_ref: WarpId,
    pub patches: Vec<WorldlineTickPatchV1>,
    pub expected: Vec<HashTriplet>,
    pub outputs: Vec<OutputFrameSet>,
    pub checkpoints: Vec<CheckpointRef>,
    pub atom_writes: Vec<Vec<AtomWrite>>,  // NEW: per-tick atom writes
}
```

---

## Part 4: Compliance Engine

The compliance engine validates schema-level law: channel policies, emission contracts, determinism constraints, and receipt verification.

### 4.1 Compliance Model (RenderModel)

```graphql
type ComplianceModel {
    panel: PanelId! @hint(kind: "fixed", data: { panel: "global" })

    appId: String
    schemaHash: Hash32
    strictMode: Boolean!

    summary: ComplianceSummary!
    violations: [ViolationRow!]! # Sorted by (severity desc, worldlineId, tick, code)
    suggestedActions: [SuggestedAction!]!
}

type ComplianceSummary {
    fatalCount: U32!
    errorCount: U32!
    warnCount: U32!
    infoCount: U32!
    maxSeverity: Severity!
    green: Boolean!
}

enum ViolationCode {
    # Receipt / hashing
    RECEIPT_MISSING
    RECEIPT_HASH_MISMATCH
    EMISSIONS_DIGEST_MISMATCH
    ENTRY_HASH_MISMATCH

    # Channel policy
    STRICT_SINGLE_VIOLATION
    REDUCE_CONFLICT
    UNKNOWN_CHANNEL
    CHANNEL_VERSION_MISMATCH

    # Rule contracts
    MUST_EMIT_MISSING
    MUST_EMIT_TOO_MANY
    MAY_EMIT_ONLY_VIOLATION
    UNDECLARED_RULE

    # Determinism constraints
    NON_CANONICAL_ENCODING
    UNSORTED_OUTPUT
    FLOAT_USED
    UNORDERED_MAP_USED
    NON_DETERMINISTIC_FIELD

    # Footprint (ties into FootprintViolation)
    FOOTPRINT_READ_VIOLATION
    FOOTPRINT_WRITE_VIOLATION
    FOOTPRINT_ADJACENCY_VIOLATION
}

type ViolationRow {
    severity: Severity!
    code: ViolationCode!
    worldlineId: WorldlineId
    tick: Tick
    ruleId: RuleId
    opId: OpId
    channelId: ChannelId
    message: String!
    detailCbor: Bytes
    evidence: ViolationEvidence
}
```

### 4.2 Compliance Algorithm

Per verified tick:

1. Verify receipt hash chain (parent → child)
2. Verify emissions_digest matches finalized emissions
3. Check channel policies:
    - StrictSingle: exactly one entry
    - Reduce: reducer constraints satisfied
4. Check rule contracts (if per-op emission index exists):
    - mustEmit: required channels present
    - mayEmitOnly: no other channels emitted
5. Check determinism directives:
    - @sorted fields are in order
    - @fixed fields use fixed-point encoding
    - @noUnorderedMap constraints

### 4.3 Panel Integration

Compliance shows up as badges, not new panels:

| Panel          | Integration                                                         |
| -------------- | ------------------------------------------------------------------- |
| Status bar     | One badge (green/yellow/red) + count                                |
| Worldline tree | Per-worldline badge showing "this fork is invalid"                  |
| Receipts       | Drill-down list with violations                                     |
| Provenance     | If provenance missing due to contract violation, show ViolationRows |
| Diff           | "Comparing invalid truth" warning if either side violates           |

---

## Part 5: Eventual Obligations

Eventual obligations track constraints like "TTD_SEEK must produce ttd.head within 3 ticks."

### 5.1 Obligation Types

```rust
// crates/echo-ttd/src/obligations.rs

pub enum OblKind {
    RequiresChannelsWithin,  // op must see channels X,Y,Z within N ticks
    ProducesWithin,          // op must emit channel X with count C within N ticks
}

pub struct ObligationSpec {
    pub spec_ix: u32,
    pub opcode: String,
    pub kind: OblKind,
    pub channels: Vec<ChannelId>,  // for RequiresChannelsWithin
    pub channel: Option<ChannelId>, // for ProducesWithin
    pub emit_count: EmitCount,
    pub within: u32,  // ticks
}

pub struct ObligationInst {
    pub key: (u32, u32),  // (spec_ix, op_ix)
    pub op_tick: u64,
    pub deadline: u64,
    pub status: OblStatus,
    pub satisfied_tick: Option<u64>,
}

pub enum OblStatus { Pending, Satisfied, Failed }
```

### 5.2 Tracker Algorithm

```rust
pub struct OblTracker {
    specs: Vec<ObligationSpec>,
    pending_by_deadline: BTreeMap<u64, Vec<(u32, u32)>>,
    inst: BTreeMap<(u32, u32), ObligationInst>,
}

impl OblTracker {
    // Step A: ingest new ops at tick t
    pub fn ingest_ops(&mut self, tick: u64, ops: &[OpOccurrence]) {
        for op in ops {
            for spec in &self.specs {
                if spec.opcode == op.opcode {
                    let key = (spec.spec_ix, op.op_ix);
                    let deadline = tick + spec.within as u64;
                    self.inst.insert(key, ObligationInst {
                        key,
                        op_tick: tick,
                        deadline,
                        status: OblStatus::Pending,
                        satisfied_tick: None,
                    });
                    self.pending_by_deadline.entry(deadline).or_default().push(key);
                }
            }
        }
    }

    // Step B: evaluate at tick t
    pub fn evaluate(&mut self, tick: u64, tick_summary: &TickSummary) {
        for (key, inst) in &mut self.inst {
            if inst.status != OblStatus::Pending { continue; }
            if tick < inst.op_tick || tick > inst.deadline { continue; }

            if self.check_condition(&self.specs[key.0 as usize], tick_summary) {
                inst.status = OblStatus::Satisfied;
                inst.satisfied_tick = Some(tick);
            }
        }
    }

    // Step C: expire failures at tick t
    pub fn expire(&mut self, tick: u64) {
        let expired: Vec<_> = self.pending_by_deadline
            .range(..tick)
            .flat_map(|(_, keys)| keys.iter().cloned())
            .collect();

        for key in expired {
            if let Some(inst) = self.inst.get_mut(&key) {
                if inst.status == OblStatus::Pending {
                    inst.status = OblStatus::Failed;
                }
            }
        }
    }
}
```

### 5.3 Output Channel

```graphql
type TtdObligationsPayload
    @channel(id: "ttd.obligations", version: 1, policy: LOG, reducer: CONCAT) {
    deltas: [ObligationDelta!]!
}

type ObligationDelta {
    op_ix: U32!
    opcode: String!
    spec_id: String!
    status: OblStatus!
    op_tick: Tick!
    deadline: Tick!
    satisfied_tick: Tick
    failed_tick: Tick
}
```

---

## Part 6: Version Doctrine

### 6.1 Version Axes

| Axis            | Identity                        | Bump When                          |
| --------------- | ------------------------------- | ---------------------------------- |
| Schema Snapshot | `schema_hash` (32 bytes)        | Any semantic schema change         |
| Envelope        | `envelope_version` (u16)        | Header/layout changes              |
| Op              | `(opcode, op_version)`          | Payload layout/semantics change    |
| Type Layout     | `(type_id, type_version)`       | Binary struct changes              |
| Channel         | `(channel_id, channel_version)` | Entry payload schema changes       |
| Rule Contract   | `(rule_id, rule_version)`       | Trigger/footprint/emission changes |
| Receipt         | `receipt_version` (u16)         | Commit hash input changes          |
| Digest Tag      | `*:vN` string                   | Canonical input bytes change       |

### 6.2 Compatibility Rules

1. **Decoders must be multi-version, encoders output latest**
2. **No silent coercion** — parse, canonicalize, re-encode
3. **Schema hash mismatch is FATAL** — different universe
4. **Proof mode requires identical:** schema_hash, receipt_version, digest tags

### 6.3 Version Matrix in Headers

Every durable artifact must carry:

- `format_magic` (MBUS/EINT/TTDR)
- `format_version`
- `schema_hash` (32 bytes)
- Component versions for contents

---

## Part 7: Platform Crates

### 7.1 ttd-browser (WASM)

The browser platform crate compiles to WebAssembly and exposes the TTD engine to JavaScript.

#### Current State (warp-wasm)

`crates/warp-wasm/src/lib.rs` has 12 placeholder functions. We create `ttd-browser` as a new, properly wired crate rather than retrofitting warp-wasm.

#### Target API

```rust
// crates/ttd-browser/src/lib.rs

#[wasm_bindgen]
pub struct TtdEngine {
    engine: Engine,
    provenance: LocalProvenanceStore,
    compliance: ComplianceIndex,
    obligations: OblTracker,
    cursors: BTreeMap<u32, PlaybackCursor>,
    sessions: BTreeMap<u32, ViewSession>,
}

#[wasm_bindgen]
impl TtdEngine {
    // Construction
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self;

    // Transaction control
    pub fn begin(&mut self) -> u64;
    pub fn commit(&mut self, tx_id: u64) -> Uint8Array;  // Returns TTDR v2 receipt

    // Playback
    pub fn create_cursor(&mut self, worldline_id: &[u8]) -> u32;
    pub fn seek_to(&mut self, cursor_id: u32, tick: u64) -> bool;
    pub fn step(&mut self, cursor_id: u32) -> Uint8Array;  // Returns StepResult
    pub fn get_tick(&self, cursor_id: u32) -> u64;

    // Provenance
    pub fn get_state_root(&self, cursor_id: u32) -> Uint8Array;
    pub fn get_commit_hash(&self, cursor_id: u32) -> Uint8Array;
    pub fn get_emissions_digest(&self, cursor_id: u32) -> Uint8Array;

    // Fork support
    pub fn snapshot(&self, cursor_id: u32) -> Uint8Array;
    pub fn fork_from_snapshot(&mut self, snapshot: &[u8]) -> u32;

    // Truth frames
    pub fn create_session(&mut self) -> u32;
    pub fn subscribe(&mut self, session_id: u32, channel: &[u8]);
    pub fn drain_frames(&mut self, session_id: u32) -> Uint8Array;  // MBUS v2 packets

    // Compliance & obligations
    pub fn get_compliance(&self) -> Uint8Array;  // ComplianceModel CBOR
    pub fn get_obligations(&self) -> Uint8Array; // ObligationState CBOR
}
```

#### Build Configuration

```toml
# crates/ttd-browser/Cargo.toml
[package]
name = "ttd-browser"
version = "0.1.0"

[lib]
crate-type = ["cdylib"]

[dependencies]
warp-core = { path = "../warp-core" }
echo-ttd = { path = "../echo-ttd" }
ttd-protocol-rs = { path = "../ttd-protocol-rs" }
ttd-manifest = { path = "../ttd-manifest" }
wasm-bindgen = "0.2"
js-sys = "0.3"
web-sys = { version = "0.3", features = ["console"] }
```

#### Web Worker Support

For the "split reality" demo with multiple concurrent engines:

```rust
// crates/ttd-browser/src/worker.rs

/// Each fork runs in its own Web Worker for true parallelism
#[wasm_bindgen]
pub struct TtdWorkerEngine {
    engine: TtdEngine,
    worker_id: u32,
}

#[wasm_bindgen]
impl TtdWorkerEngine {
    pub fn step_and_hash(&mut self) -> Uint8Array;  // Returns (tick, commit_hash)
}
```

### 7.2 ttd-native (Desktop)

The native platform crate builds a desktop TTD application using egui + wgpu, reusing patterns from warp-viewer.

#### Architecture

```rust
// crates/ttd-native/src/main.rs

fn main() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Echo TTD",
        options,
        Box::new(|cc| Box::new(TtdApp::new(cc))),
    );
}
```

#### Panel Structure

```rust
// crates/ttd-native/src/app.rs

pub struct TtdApp {
    // Core state (same as browser)
    engine: Engine,
    provenance: LocalProvenanceStore,
    compliance: ComplianceIndex,
    obligations: OblTracker,
    cursors: BTreeMap<CursorId, PlaybackCursor>,
    sessions: BTreeMap<SessionId, ViewSession>,

    // Native-specific
    session_client: Option<SessionClient>,  // Unix socket or embedded
    render_state: RenderState,              // WGPU scene

    // UI state
    active_panel: PanelId,
    worldline_tree: WorldlineTreeState,
    inspector_state: InspectorState,
    provenance_drawer: Option<ProvenanceDrawerState>,
}

impl eframe::App for TtdApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Top bar: time controls + compliance badge
        egui::TopBottomPanel::top("controls").show(ctx, |ui| {
            self.render_time_controls(ui);
        });

        // Left: worldline tree
        egui::SidePanel::left("worldlines").show(ctx, |ui| {
            self.render_worldline_tree(ui);
        });

        // Right: inspector
        egui::SidePanel::right("inspector").show(ctx, |ui| {
            self.render_state_inspector(ui);
        });

        // Center: 3D view or tick inspector
        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_central_view(ui);
        });

        // Bottom: timeline
        egui::TopBottomPanel::bottom("timeline").show(ctx, |ui| {
            self.render_timeline(ui);
        });
    }
}
```

#### Build Configuration

```toml
# crates/ttd-native/Cargo.toml
[package]
name = "ttd-native"
version = "0.1.0"

[[bin]]
name = "ttd"
path = "src/main.rs"

[dependencies]
warp-core = { path = "../warp-core" }
echo-ttd = { path = "../echo-ttd" }
ttd-protocol-rs = { path = "../ttd-protocol-rs" }
ttd-manifest = { path = "../ttd-manifest" }
echo-session-proto = { path = "../echo-session-proto" }

# UI
eframe = "0.29"
egui = "0.29"
egui-wgpu = "0.29"

# Rendering (reuse from warp-viewer)
wgpu = "27"
glam = "0.29"
bytemuck = "1.14"
```

### 7.3 Shared Core (echo-ttd)

Both platform crates depend on `echo-ttd` for shared logic:

```rust
// crates/echo-ttd/src/session.rs

/// Platform-agnostic TTD session manager
pub struct TtdSession {
    pub engine: Engine,
    pub provenance: LocalProvenanceStore,
    pub compliance: ComplianceIndex,
    pub obligations: OblTracker,
    pub cursors: BTreeMap<CursorId, PlaybackCursor>,
    pub sessions: BTreeMap<SessionId, ViewSession>,
}

impl TtdSession {
    pub fn new() -> Self;
    pub fn begin(&mut self) -> TxId;
    pub fn commit(&mut self, tx: TxId) -> Result<TtdrV2, CommitError>;
    pub fn create_cursor(&mut self, worldline: WorldlineId) -> CursorId;
    pub fn seek(&mut self, cursor: CursorId, tick: u64) -> Result<(), SeekError>;
    pub fn step(&mut self, cursor: CursorId) -> Result<StepResult, StepError>;
    pub fn fork(&mut self, cursor: CursorId) -> Result<CursorId, ForkError>;
    pub fn get_compliance(&self) -> ComplianceModel;
    pub fn get_obligations(&self) -> ObligationState;
}
```

This allows `ttd-browser` and `ttd-native` to share 100% of the core logic.

---

## Part 7.5: TTD Controller WARP Schema (Meta-WARP Architecture)

### Key Insight: TTD as a WARP Graph

The TTD is not merely an observer of WARP graphs — **it IS a WARP graph**. This creates a recursive structure:

1. **Observed App WARP** — The application being debugged
2. **TTD Controller WARP** — Manages debugging state, observes the app
3. **TTD Client** — Renders TTD Controller's MBUS emissions (dumb client)

**Bonus:** You can time-travel the time-travel debugger itself.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           TWO-WARP ARCHITECTURE                             │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    OBSERVED APP WARP                                 │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐                          │   │
│  │  │ App      │  │ App      │  │ App      │   (Your application       │   │
│  │  │ Atoms    │  │ Rules    │  │ Channels │    being debugged)        │   │
│  │  └──────────┘  └──────────┘  └──────────┘                          │   │
│  │                      │                                              │   │
│  │              ┌───────▼───────┐                                      │   │
│  │              │ Provenance    │◀────────────────────────┐           │   │
│  │              │ Store         │                         │           │   │
│  │              └───────────────┘                         │           │   │
│  └────────────────────────────────────────────────────────────────────┘   │
│                         │                                 │                 │
│                         │ reads patches/state             │                 │
│                         ▼                                 │                 │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    TTD CONTROLLER WARP                               │   │
│  │                                                                      │   │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌────────────┐    │   │
│  │  │ ttd/cursor │  │ttd/session │  │ttd/worldline│ │ttd/compliance│   │   │
│  │  │ atoms      │  │ atoms      │  │ atoms      │  │ atoms      │    │   │
│  │  └────────────┘  └────────────┘  └────────────┘  └────────────┘    │   │
│  │                                                                      │   │
│  │  ┌───────────────────────────────────────────────────────────────┐  │   │
│  │  │                      TTD RULES                                 │  │   │
│  │  │  HandleSeek │ HandleStep │ HandleFork │ ComputeCompliance     │  │   │
│  │  └───────────────────────────────────────────────────────────────┘  │   │
│  │                                │                                     │   │
│  │                        ┌───────▼───────┐                            │   │
│  │                        │   TTD MBUS    │                            │   │
│  │                        └───────┬───────┘                            │   │
│  └────────────────────────────────┼────────────────────────────────────┘   │
│                                   │                                         │
│                                   │ MBUS emissions (TruthFrames)            │
│                                   ▼                                         │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                         TTD CLIENTS                                  │   │
│  │  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐        │   │
│  │  │  ttd-browser   │  │   ttd-native   │  │  ttd-remote    │        │   │
│  │  │  (WASM)        │  │  (egui+wgpu)   │  │  (WebSocket)   │        │   │
│  │  │ EINT↑    ↓TF   │  │ EINT↑    ↓TF   │  │ EINT↑    ↓TF   │        │   │
│  │  └────────────────┘  └────────────────┘  └────────────────┘        │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  Legend: EINT = Intent, TF = TruthFrame                                    │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 7.5.1 TTD Atoms (Graph Nodes)

| Atom Type           | Fields                                                    | Purpose                           |
| ------------------- | --------------------------------------------------------- | --------------------------------- |
| `ttd/cursor`        | cursor_id, worldline_id, tick, mode, role, pin_max_tick   | Playback cursor position          |
| `ttd/session`       | session_id, active_cursor_id                              | Client session state              |
| `ttd/worldline`     | worldline_id, head_tick, parent_worldline_id?, fork_tick? | Worldline metadata                |
| `ttd/subscription`  | session_id, channel_id                                    | Session→Channel subscription edge |
| `ttd/compliance`    | worldline_id, summary, violation_count                    | Per-worldline compliance          |
| `ttd/obligation`    | spec_ix, op_ix, deadline, status                          | Tracked eventual obligation       |
| `ttd/observed_warp` | warp_id, schema_hash, tick_frontier                       | Reference to observed app         |

### 7.5.2 TTD Rules (Intent Handlers)

| Rule ID                    | Trigger                | Effect                               | Emits To                          |
| -------------------------- | ---------------------- | ------------------------------------ | --------------------------------- |
| `ttd.handle_seek`          | SeekIntent on inbox    | Updates cursor.tick via seek         | `ttd.head`, `ttd.state_inspector` |
| `ttd.handle_step`          | StepIntent on inbox    | Advances/retreats cursor by 1        | `ttd.head`, `ttd.time_controls`   |
| `ttd.handle_fork`          | ForkIntent on inbox    | Creates new worldline, clones cursor | `ttd.worldline_tree`, `ttd.head`  |
| `ttd.handle_subscribe`     | SubscribeChannelIntent | Creates subscription edge            | `ttd.subscriptions`               |
| `ttd.compute_compliance`   | Post-seek/step hook    | Validates receipts, contracts        | `ttd.compliance`                  |
| `ttd.evaluate_obligations` | Every tick             | Checks pending obligations           | `ttd.obligations`                 |
| `ttd.sync_observed_state`  | After cursor move      | Copies observed app state            | `ttd.state_inspector`             |

### 7.5.3 TTD Channels (MBUS Outputs)

| Channel ID            | Policy        | Reducer | Purpose                  | Error-bearing?  |
| --------------------- | ------------- | ------- | ------------------------ | --------------- |
| `ttd.head`            | STRICT_SINGLE | LAST    | Cursor position + status | ✅ Status enum  |
| `ttd.errors`          | LOG           | CONCAT  | Structured error records | ✅ Primary sink |
| `ttd.time_controls`   | STRICT_SINGLE | LAST    | Playback mode, range     | ❌              |
| `ttd.worldline_tree`  | STRICT_SINGLE | LAST    | Fork hierarchy + badges  | ❌              |
| `ttd.state_inspector` | STRICT_SINGLE | LAST    | Atom table at tick       | ❌              |
| `ttd.compliance`      | STRICT_SINGLE | LAST    | Compliance summary       | ❌              |
| `ttd.obligations`     | LOG           | CONCAT  | Obligation deltas        | ❌              |
| `ttd.provenance`      | LOG           | CONCAT  | AtomWrite records        | ❌              |
| `ttd.diff`            | STRICT_SINGLE | LAST    | State diff for render    | ❌              |
| `ttd.subscriptions`   | STRICT_SINGLE | LAST    | Active subscriptions     | ❌              |

### 7.5.4 ObservedWorldAPI Abstraction

TTD Controller must not depend on `ProvenanceStore` directly. Instead, it depends on an explicit trait:

```rust
/// Abstraction boundary between TTD Controller and observed app.
/// Enables mocking, replay, simulation, remote debugging.
pub trait ObservedWorldAPI: Send + Sync {
    /// Get patch at specific tick
    fn get_patch(
        &self,
        worldline: WorldlineId,
        tick: u64,
    ) -> Result<WorldlineTickPatchV1, ObservedWorldError>;

    /// Get graph snapshot at specific tick
    fn get_state(
        &self,
        worldline: WorldlineId,
        tick: u64,
    ) -> Result<GraphSnapshot, ObservedWorldError>;

    /// Get receipt at specific tick
    fn get_receipt(
        &self,
        worldline: WorldlineId,
        tick: u64,
    ) -> Result<TtdrV2, ObservedWorldError>;

    /// Get atom write history for provenance queries
    fn get_atom_writes(
        &self,
        worldline: WorldlineId,
        tick: u64,
    ) -> Result<Vec<AtomWrite>, ObservedWorldError>;

    /// Get current tick frontier
    fn get_frontier(&self, worldline: WorldlineId) -> u64;

    /// List available worldlines
    fn list_worldlines(&self) -> Vec<WorldlineId>;
}

pub enum ObservedWorldError {
    WorldlineNotFound,
    TickOutOfRange,
    HistoryUnavailable,
    SnapshotCorrupt,
}
```

**Why this matters:**

- **Mocking:** Test TTD Controller without real engine
- **Replay:** Feed recorded provenance for regression tests
- **Simulation:** Synthetic observed worlds for demos
- **Remote debugging:** Observed app on different machine
- **Isolation:** TTD bugs cannot corrupt observed app

**Implementation:**

```rust
// Live implementation wraps actual provenance store
pub struct LiveObservedWorld {
    provenance: Arc<dyn ProvenanceStore>,
    engine: Arc<Mutex<Engine>>,
}

impl ObservedWorldAPI for LiveObservedWorld { ... }

// Mock for testing
pub struct MockObservedWorld {
    patches: BTreeMap<(WorldlineId, u64), WorldlineTickPatchV1>,
    states: BTreeMap<(WorldlineId, u64), GraphSnapshot>,
}

impl ObservedWorldAPI for MockObservedWorld { ... }
```

### 7.5.5 Engine Separation: Trade-offs

**Option A: Same Engine Instance**

- Single tick coordination
- Shared graph store
- Simpler memory model
- **Con:** Can't time-travel debugger independently

**Option B: Separate Engine Instances (RECOMMENDED)**

- Clean separation of concerns
- TTD can be at different tick than observed app
- **Can time-travel the debugger itself**
- TTD bugs can't corrupt observed app
- Easier testing (mock observed engine)

---

## Part 7.6: TTD Intent Protocol

### 7.6.1 Protocol Boundary

**Hard rule:** Keep session wire and intent wire separate.

```
┌─────────────────────────────────────────────────────────────────────┐
│                     SESSION WIRE (JIT!/OpEnvelope)                  │
│                                                                     │
│  Message::Handshake       → Session establishment                   │
│  Message::HandshakeAck    → Session confirmed                       │
│  Message::SubscribeWarp   → Channel subscription control            │
│  Message::WarpStream      → MBUS frames (TruthFrames)              │
│  Message::Notification    → Session-level alerts (disconnect)      │
│  Message::TtdIntent       → **Container** for EINT packet          │
│                             (session layer doesn't parse contents)  │
│  Message::Error           → Session-level errors only               │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ eint_packet bytes passed through
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     INTENT WIRE (EINT v2)                           │
│                                                                     │
│  - schema_hash bound (universe identity)                            │
│  - opcode + op_version (typed dispatch)                             │
│  - payload checksum (integrity)                                     │
│  - receipt-able (appears in commit chain)                           │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ processed by TTD Controller WARP
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     TRUTH WIRE (MBUS v2)                            │
│                                                                     │
│  - channel_id (32 bytes)                                            │
│  - emit_key ordering (deterministic)                                │
│  - value + value_hash (verifiable)                                  │
│  - emissions_digest (canonical)                                     │
│                                                                     │
│  All domain outcomes (success AND failure) are truth.               │
└─────────────────────────────────────────────────────────────────────┘
```

### 7.6.2 TTD Intent Opcodes

| Opcode | Name                 | Payload                            | Description          |
| ------ | -------------------- | ---------------------------------- | -------------------- |
| 0x0001 | TTD_SEEK             | `{ cursor_id, target_tick, then }` | Seek cursor to tick  |
| 0x0002 | TTD_STEP             | `{ cursor_id, direction }`         | Step forward/back    |
| 0x0003 | TTD_FORK             | `{ source_cursor_id, label? }`     | Fork reality         |
| 0x0004 | TTD_SUBSCRIBE        | `{ session_id, channel_id }`       | Subscribe to channel |
| 0x0005 | TTD_QUERY_PROVENANCE | `{ cursor_id, atom_id, depth }`    | Query atom history   |
| 0x0006 | TTD_SET_MODE         | `{ cursor_id, mode }`              | Set playback mode    |

### 7.6.3 Intent Flow

```
┌─────────────┐      ┌──────────────────────────────────────────────┐
│ TTD Client  │      │           TTD Controller WARP                 │
│             │      │                                               │
│  User clicks│      │  ┌─────────┐   ┌─────────────┐               │
│  "Step →"   │──────▶  │ EINT    │──▶│ ttd/inbox   │               │
│             │      │  │ Decoder │   │ event node  │               │
│             │      │  └─────────┘   └──────┬──────┘               │
│             │      │                       │                       │
│             │      │               ┌───────▼───────┐              │
│             │      │               │ ttd.handle_   │              │
│             │      │               │ step rule     │              │
│             │      │               └───────┬───────┘              │
│             │      │                       │                       │
│             │      │  ┌────────────────────▼────────────────────┐ │
│             │      │  │           MBUS Emissions                │ │
│             │      │  │  ttd.head  │  ttd.time_controls        │ │
│             │      │  └─────────────────────────────────────────┘ │
│             │      └──────────────────────────────────────────────┘
│             │                       │
│  ◀──────────┼───────────────────────┘
│  TruthFrame │      (subscribed channels)
└─────────────┘
```

### 7.6.4 Error Reporting: Errors as Truth

**Principle:** Domain errors are truth, not session messages.

```rust
// ttd.head payload includes operation status
pub struct TtdHeadPayload {
    pub cursor_id: CursorId,
    pub worldline_id: WorldlineId,
    pub tick: u64,
    pub commit_hash: Hash,
    pub status: OpStatus,           // Ok | Failed | Pending
    pub error_ref: Option<Hash>,    // If Failed, hash into ttd.errors
}

pub enum OpStatus {
    Ok = 0,
    Failed = 1,
    Pending = 2,
}

// ttd.errors payload for detailed error info
pub struct TtdErrorRecord {
    pub error_id: Hash,
    pub source_intent_hash: Hash,
    pub code: TtdErrorCode,
    pub message: String,
    pub detail_cbor: Option<Vec<u8>>,
}
```

**Why errors as truth:**

1. Clients are dumb — just render `ttd.head` status
2. Testing is clean — assert on channel contents
3. Provenance — errors are receipted, verifiable
4. Obligations work — `op.produces("TTD_SEEK", "ttd.head", EXACTLY_ONE)` satisfied even on failure

---

## Part 7.7: Platform Crates as MBUS Clients

### Architectural Shift

The key insight: **ttd-browser and ttd-native are pure MBUS clients**. They:

1. Send intents via EINT → TTD Controller inbox
2. Receive truth via MBUS channel subscriptions
3. Render TruthFrames — **zero protocol logic in UI**

### 7.7.1 TtdController (replaces TtdSession)

```rust
// crates/echo-ttd/src/controller.rs

/// The TTD Controller runs as a WARP graph
pub struct TtdController {
    /// The TTD's own WARP engine (runs TTD schema)
    engine: Engine,

    /// Materialization bus for TTD channels
    bus: MaterializationBus,

    /// Abstracted interface to observed app (see 7.5.4)
    observed: Arc<dyn ObservedWorldAPI>,

    /// Retention configuration
    retention: RetentionConfig,

    /// Fork manager with eviction
    forks: ForkManager,
}

impl TtdController {
    /// Process an EINT intent packet
    pub fn ingest_intent(&mut self, eint_bytes: &[u8]) -> Result<(), IntentError> {
        // 1. Decode EINT packet, validate schema_hash
        // 2. Create inbox event node in TTD graph
        // 3. Begin transaction
        // 4. Let rules fire (HandleSeek, HandleStep, etc.)
        // 5. Commit → produces TTD MBUS emissions
    }

    /// Drain finalized channels for a session
    pub fn drain_for_session(&self, session_id: SessionId) -> Vec<MaterializationFrame> {
        // Return frames for subscribed channels
    }

    /// Tick the controller (for background obligations, etc.)
    pub fn tick(&mut self) -> Result<(), TickError> {
        // Evaluate obligations, emit deltas
    }
}
```

### 7.7.2 Platform Crate Pattern

**ttd-browser (WASM):**

```rust
#[wasm_bindgen]
pub struct TtdClient {
    controller: TtdController,
    session_id: SessionId,
    port: MaterializationPort,
}

#[wasm_bindgen]
impl TtdClient {
    pub fn send_intent(&mut self, eint_bytes: &[u8]) {
        self.controller.ingest_intent(eint_bytes).unwrap();
    }

    pub fn tick_and_drain(&mut self) -> Uint8Array {
        self.controller.tick().ok();
        let frames = self.controller.drain_for_session(self.session_id);
        encode_frames(&frames).into()
    }

    pub fn subscribe(&mut self, channel_label: &str) {
        let channel_id = make_channel_id(channel_label);
        self.port.subscribe(channel_id);
    }
}
```

**ttd-native:** Same pattern with egui render loop calling `tick_and_drain()`.

### 7.7.3 Rendering Stack Separation Rule

**STRICT RULE:** One rendering stack per platform. No cross-platform rendering abstractions.

| Platform    | Rendering Stack       | UI Framework |
| ----------- | --------------------- | ------------ |
| **Browser** | Three.js + WebGL      | React        |
| **Native**  | wgpu + custom shaders | egui         |

**Explicitly forbidden:**

- ❌ Shared shader code between platforms
- ❌ Shared scene graph abstractions
- ❌ Cross-platform rendering utilities
- ❌ Unified 3D primitive libraries

**Explicitly allowed:**

- ✅ Shared data models (atoms, channels, receipts)
- ✅ Shared protocol types (EINT, MBUS, TTDR)
- ✅ Shared business logic (compliance, obligations)
- ✅ Shared test fixtures and golden vectors

**Rationale:**

1. **Toolchain entropy:** Mixing WebGL/Three.js with wgpu creates maintenance burden
2. **Performance:** Each platform has different optimization paths
3. **Expertise:** Teams can specialize without cross-contamination
4. **Testing:** Platform-specific rendering tested in isolation

```rust
// ✅ CORRECT: Shared data model
pub struct AtomRenderData {
    pub atom_id: NodeId,
    pub position: [f32; 3],
    pub color: [f32; 4],
    pub value_history: Vec<(u64, Vec<u8>)>,
}

// ❌ WRONG: Shared rendering abstraction
pub trait CrossPlatformRenderer {
    fn draw_atom(&self, atom: &AtomRenderData);  // NO!
}

// ✅ CORRECT: Platform-specific renderers
// Browser: src/three/atomPillar.ts
// Native: src/render/atom_pillar.rs
```

---

## Part 7.8: Memory & Retention Model

### Critical Constraint

WASM + multiple forks + provenance + receipts + 4D visualization = memory pressure.

Without explicit retention policies, the demo works but real apps die.

### 7.8.1 Retention Policies

| Resource           | Policy                 | Default             | Configurable |
| ------------------ | ---------------------- | ------------------- | ------------ |
| Provenance patches | Rolling window         | Last 1000 ticks     | ✅           |
| Receipts (TTDR)    | LRU cache + LIGHT mode | 100 receipts, LIGHT | ✅           |
| Graph snapshots    | Checkpoint interval    | Every 100 ticks     | ✅           |
| Truth frames       | Per-channel LRU        | 50 frames/channel   | ✅           |
| Forks              | Max active + eviction  | 5 active, LRU evict | ✅           |
| Atom write history | Sliding window         | Last 500 ticks      | ✅           |

### 7.8.2 Retention Strategies

```rust
pub struct RetentionConfig {
    /// Maximum ticks of provenance to retain
    pub provenance_window: u64,

    /// Maximum receipts in LRU cache
    pub receipt_cache_size: usize,

    /// Default receipt mode for caching
    pub receipt_mode: ReceiptMode,

    /// Checkpoint interval for graph snapshots
    pub checkpoint_interval: u64,

    /// Max truth frames per channel
    pub frames_per_channel: usize,

    /// Maximum active forks
    pub max_active_forks: usize,

    /// Atom write history window
    pub atom_write_window: u64,
}

impl Default for RetentionConfig {
    fn default() -> Self {
        Self {
            provenance_window: 1000,
            receipt_cache_size: 100,
            receipt_mode: ReceiptMode::Light,
            checkpoint_interval: 100,
            frames_per_channel: 50,
            max_active_forks: 5,
            atom_write_window: 500,
        }
    }
}
```

### 7.8.3 Fork Eviction

When `max_active_forks` exceeded:

1. **LRU eviction:** Least-recently-accessed fork evicted first
2. **Preservation rule:** Fork with unsaved changes prompts user
3. **Snapshot-on-evict:** Evicted fork serialized to IndexedDB (browser) or disk (native)
4. **Restore-on-demand:** User can restore evicted fork from storage

```rust
pub struct ForkManager {
    active: LruCache<WorldlineId, ForkState>,
    evicted: BTreeMap<WorldlineId, ForkSnapshot>,
    config: RetentionConfig,
}

impl ForkManager {
    pub fn activate(&mut self, worldline: WorldlineId) -> Result<(), ForkError> {
        if self.active.len() >= self.config.max_active_forks {
            let victim = self.active.pop_lru()?;
            self.evicted.insert(victim.0, victim.1.snapshot());
        }
        self.active.push(worldline, ForkState::new());
        Ok(())
    }
}
```

### 7.8.4 Channel Retention Policies

Per-channel configuration for truth frame retention:

| Channel               | Retention        | Reason            |
| --------------------- | ---------------- | ----------------- |
| `ttd.head`            | Keep latest only | Single value      |
| `ttd.errors`          | Rolling 100      | Error history     |
| `ttd.state_inspector` | Keep latest only | Large payload     |
| `ttd.provenance`      | Window match     | Sync with patches |
| `ttd.worldline_tree`  | Keep latest only | Single value      |
| `ttd.compliance`      | Keep latest only | Single value      |
| `ttd.obligations`     | Rolling 50       | Recent deltas     |

### 7.8.5 Browser-Specific Constraints

| Constraint   | Limit             | Mitigation                             |
| ------------ | ----------------- | -------------------------------------- |
| WASM heap    | ~2GB max          | Aggressive GC, streaming               |
| IndexedDB    | ~50% disk         | Quota management                       |
| Web Workers  | Memory per worker | Share provenance via SharedArrayBuffer |
| Canvas/WebGL | GPU memory        | LOD for 4D view, frustum culling       |

**Memory budget example:**

```
Base WASM heap:        50 MB
Per fork overhead:     20 MB × 5 forks = 100 MB
Provenance window:     ~50 MB (1000 ticks × 50KB avg)
Truth frame cache:     ~10 MB
4D visualization:      ~30 MB
────────────────────────────────
Total target:          ~240 MB (well under 2GB limit)
```

---

## Part 8: Browser TTD Application

### 8.1 Architecture

```
Browser
├── WASM Instance A (Fork 1, 1 worker)
├── WASM Instance B (Fork 2, 16 workers)
├── WASM Instance C (Fork 3, shuffled)
├── JS Glue (fork, step_all, compare hashes)
├── Three.js (4D Provenance View)
└── React/Svelte UI Shell
```

### 8.2 UI Panels

| Panel                 | Purpose                                         |
| --------------------- | ----------------------------------------------- |
| **Time Controls**     | Play/Pause/Step/Seek, speed control             |
| **Timeline**          | Visual markers for intents, forks, rule fires   |
| **Worldline Tree**    | Fork hierarchy with compliance badges           |
| **State Inspector**   | Atom table with values, types, provenance links |
| **Tick Inspector**    | What rules fired, what patches applied          |
| **Provenance Drawer** | Slide-out showing atom history + causal chain   |
| **Fork Manager**      | Side-by-side comparison, hash verification      |
| **4D View**           | Spacetime visualization (Three.js)              |

### 8.3 4D Provenance Visualization (Three.js)

| Element       | Three.js Implementation                 |
| ------------- | --------------------------------------- |
| Mouse worm    | `TubeGeometry` from `CatmullRomCurve3`  |
| Atom pillar   | Stacked `BoxGeometry`, colored by value |
| Causal arrows | `ArrowHelper`                           |
| Time plane    | `PlaneGeometry` at current tick Z       |
| Scrubber      | Raycaster → jump cursor                 |

### 8.4 Demo Script (Theater)

**Scene: Cliff Tick** — Physics sim paused at Tick 47, right before collision.

**Act 1: Split Reality**

1. User clicks **"Fork"** twice → 3 lanes appear
2. Lane A: "Baseline (1 worker)"
3. Lane B: "Max Cores (16 workers)"
4. Lane C: "Chaos Mode (shuffled)"
5. User clicks **"Step All"**
6. All three advance, hashes computed
7. **Reveal:** `✅ VERIFIED: A = B = C`

**Act 2: Show Me Why**

1. User clicks an atom (e.g., `counter_1`)
2. Provenance drawer slides in
3. Shows: value 47, last write at T45, rule `update_counter`, intent `ClickIntent`
4. User clicks **"View in 4D"**
5. 4D view opens: mouse worm, atom pillar, causal arrow

**Act 3: Counterfactual**

1. Fork again at T47
2. In Fork C, nudge one input
3. Step both
4. **Reveal:** A = B ✅, C diverged ✅ (but still deterministic)

---

## Part 9: Test Plan

### 9.1 Test Taxonomy

| Category                | Purpose                                       |
| ----------------------- | --------------------------------------------- |
| Codec tests             | Roundtrip encode/decode, canonical validation |
| Canonicalization tests  | @sorted, permutation invariance               |
| Digest/receipt tests    | Known inputs → exact hashes                   |
| Manifest/compiler tests | Wesley output stability                       |
| Obligation tests        | Creation, satisfaction, failure, pruning      |
| Integration tests       | Scenario runs with channel verification       |
| Cross-platform tests    | Identical hashes across platforms/workers     |
| UI behavior tests       | Render model snapshots                        |

### 9.2 Concrete Test Cases

| ID         | Category       | Description                             |
| ---------- | -------------- | --------------------------------------- |
| T-EINT-01  | Codec          | EINT v2 roundtrip with schema_hash      |
| T-EINT-02  | Codec          | EINT v2 checksum mismatch rejection     |
| T-MBUS-01  | Codec          | MBUS v2 entry hash correctness          |
| T-MBUS-02  | Canon          | MBUS v2 unsorted rejection              |
| T-DIG-01   | Digest         | emissions_digest permutation invariance |
| T-DIG-02   | Digest         | tick_commit:v2 domain tag enforcement   |
| T-INV-01   | Compiler       | Invariant program hash stability        |
| T-INV-02   | Compiler       | Unknown function error stability        |
| T-OBL-01   | Obligation     | producesWithin satisfied in window      |
| T-OBL-02   | Obligation     | requiresChannels failed after deadline  |
| T-OBL-03   | Obligation     | Worldline isolation                     |
| T-INT-01   | Integration    | TTD_SEEK → ttd.head within 3 ticks      |
| T-PROOF-01 | Cross-platform | Concurrency profiles same hashes        |

### 9.3 Golden Vector Format

```rust
// fixtures/ttd/goldens.cbor
struct GoldensV1 {
    vectors: Vec<GoldenVector>,
}

struct GoldenVector {
    id: String,
    kind: VectorKind,  // Roundtrip, Digest, Compile, etc.
    input: Vec<u8>,
    expected: Vec<u8>,  // Hash or canonical output
    meta: VectorMeta,
}

struct VectorMeta {
    receipt_version: u16,
    digest_tag: String,
    schema_hash: [u8; 32],
}
```

---

## Part 10: File Structure

### 10.1 New Crates

```
crates/
├── wesley/                        # Schema compiler
│   ├── src/
│   │   ├── lib.rs
│   │   ├── parser.rs             # GraphQL SDL parser
│   │   ├── directives.rs         # Directive handling
│   │   ├── codegen_rust.rs       # Rust code generation
│   │   ├── codegen_ts.rs         # TypeScript generation
│   │   └── invariants.rs         # Invariant bytecode compiler
│   └── Cargo.toml
│
├── ttd-protocol-rs/               # Generated protocol types
│   ├── src/
│   │   ├── lib.rs
│   │   ├── types.rs              # Op/channel payloads
│   │   ├── cbor.rs               # Canonical codecs
│   │   ├── registry.rs           # Lookup tables
│   │   └── hash.rs               # Digest helpers
│   └── Cargo.toml
│
├── ttd-manifest/                  # Generated enforcement tables
│   ├── src/
│   │   ├── lib.rs
│   │   ├── channel_registry.rs
│   │   ├── op_registry.rs
│   │   ├── rule_contracts.rs
│   │   ├── invariants.rs
│   │   └── emission_contracts.rs
│   └── Cargo.toml
│
├── echo-ttd/                      # TTD core logic (platform-agnostic)
│   ├── src/
│   │   ├── lib.rs
│   │   ├── compliance.rs         # Compliance engine
│   │   ├── obligations.rs        # Eventual obligations
│   │   ├── receipts.rs           # TTDR v2 encode/decode
│   │   ├── render_models.rs      # UI render models
│   │   └── session.rs            # TTD session management
│   └── Cargo.toml
│
├── ttd-browser/                   # WASM target for browser TTD
│   ├── src/
│   │   ├── lib.rs                # wasm_bindgen exports
│   │   ├── engine.rs             # WasmEngine wrapper
│   │   ├── glue.rs               # JS interop helpers
│   │   └── worker.rs             # Web Worker support
│   ├── pkg/                      # wasm-pack output
│   └── Cargo.toml
│
└── ttd-native/                    # Native desktop TTD (egui + wgpu)
    ├── src/
    │   ├── main.rs               # Entry point
    │   ├── app.rs                # Application state
    │   ├── panels/               # UI panels (reuse warp-viewer patterns)
    │   │   ├── mod.rs
    │   │   ├── time_controls.rs
    │   │   ├── worldline_tree.rs
    │   │   ├── state_inspector.rs
    │   │   ├── provenance.rs
    │   │   └── compliance.rs
    │   ├── render/               # 3D rendering (WGPU)
    │   │   ├── mod.rs
    │   │   ├── scene.rs
    │   │   └── shaders/
    │   └── session.rs            # Native session client
    └── Cargo.toml
```

### 10.2 TypeScript Packages

```
packages/
├── ttd-protocol-ts/               # Generated TS types
│   ├── src/
│   │   ├── types.ts
│   │   ├── zod.ts
│   │   ├── cbor.ts
│   │   └── registry.ts
│   └── package.json
│
├── ttd-manifest-ts/               # Generated TS manifests
│   ├── src/
│   │   ├── channel_registry.ts
│   │   ├── op_registry.ts
│   │   └── invariants.ts
│   └── package.json
│
└── ttd-app/                       # Browser application
    ├── src/
    │   ├── main.tsx
    │   ├── components/
    │   │   ├── TimeControls.tsx
    │   │   ├── Timeline.tsx
    │   │   ├── WorldlineTree.tsx
    │   │   ├── StateInspector.tsx
    │   │   ├── ProvenanceDrawer.tsx
    │   │   ├── ForkManager.tsx
    │   │   └── View4D.tsx
    │   ├── three/
    │   │   ├── scene.ts
    │   │   ├── atomPillar.ts
    │   │   ├── mouseWorm.ts
    │   │   └── causalArrow.ts
    │   ├── wasm/
    │   │   ├── loader.ts             # WASM initialization
    │   │   ├── bridge.ts             # TtdEngine wrapper
    │   │   └── workers.ts            # Web Worker pool for forks
    │   └── store/
    │       ├── cursors.ts
    │       ├── compliance.ts
    │       └── forks.ts              # Fork comparison state
    ├── public/
    │   └── ttd_browser_bg.wasm       # Built from ttd-browser crate
    └── package.json
```

### 10.3 Fixtures & Tests

```
fixtures/
└── ttd/
    ├── goldens.cbor               # Golden vectors
    ├── invariants_compile.cbor    # Compiler test cases
    ├── obligations_cases.cbor     # Obligation scenarios
    ├── eint/
    │   ├── basic_seek.bin
    │   └── bad_checksum.bin
    ├── mbus/
    │   ├── one_channel.bin
    │   └── multi_entry.bin
    └── ttdr/
        ├── basic_receipt.bin
        └── with_emissions.bin
```

---

## Part 11: Implementation Phases

### Phase 1: Wesley Schema Compiler (Staged)

Wesley is split into three maturity layers to prevent scope creep:

#### Phase 1a: Wesley v0 — Foundation (~1 week)

**Scope:** Parser + AST + manifests only. No codegen. No bytecode.

**Deliverables:**

- [ ] GraphQL SDL parser with directive extraction
- [ ] AST model for schema, channels, ops, rules
- [ ] Schema hashing (canonical field ordering)
- [ ] Channel/op/rule model extraction
- [ ] JSON manifest output: `schema.json`, `manifest.json`, `contracts.json`

**Files:**

- `crates/wesley/src/parser.rs`
- `crates/wesley/src/ast.rs`
- `crates/wesley/src/manifest.rs`

**Output format:**

```json
// schema.json
{ "schema_hash": "...", "version": 1, "channels": [...], "ops": [...], "rules": [...] }

// manifest.json
{ "channel_registry": {...}, "op_registry": {...}, "rule_contracts": {...} }

// contracts.json
{ "emission_contracts": [...], "footprint_specs": [...] }
```

#### Phase 1b: Wesley v1 — Codegen (~1 week)

**Scope:** Consumes manifests → generates types and registries.

**Deliverables:**

- [ ] Rust types generation (structs/enums for ops/channels)
- [ ] Rust CBOR codecs (canonical encode/decode)
- [ ] Rust registries (lookup tables)
- [ ] TypeScript types generation
- [ ] TypeScript Zod validators
- [ ] TypeScript registries

**Files:**

- `crates/wesley/src/codegen_rust.rs`
- `crates/wesley/src/codegen_ts.rs`
- `crates/ttd-protocol-rs/` (generated)
- `packages/ttd-protocol-ts/` (generated)

**Invariant:** Codegen is deterministic — same manifest → same output bytes.

#### Phase 1c: Wesley v2 — Law Compiler (~1 week)

**Scope:** Invariant expression compiler + enforcement bytecode.

**Deliverables:**

- [ ] Invariant expression parser (EBNF grammar)
- [ ] Obligation spec compilation
- [ ] Enforcement bytecode generation
- [ ] Verification program output
- [ ] Golden test framework

**Files:**

- `crates/wesley/src/invariants.rs`
- `crates/wesley/src/bytecode.rs`
- `crates/ttd-manifest/` (enforcement tables)

### Phase 2: Receipt & Digest System (~1 week)

**Deliverables:**

- [ ] EINT v2 encoder/decoder
- [ ] TTDR v2 encoder/decoder
- [ ] emissions_digest computation
- [ ] op_emission_index_digest computation
- [ ] commit_hash:v2 computation
- [ ] AtomWrite tracking in provenance

**Files:**

- `crates/echo-session-proto/src/eint_v2.rs`
- `crates/echo-ttd/src/receipts.rs`
- `crates/warp-core/src/provenance_store.rs` (extend)

### Phase 3: Compliance Engine (~1 week)

**Deliverables:**

- [ ] ComplianceIndex computation
- [ ] Channel policy checks
- [ ] Rule emission contract checks
- [ ] Determinism directive checks
- [ ] ViolationRow generation
- [ ] Render model integration

**Files:**

- `crates/echo-ttd/src/compliance.rs`
- `crates/echo-ttd/src/render_models.rs`

### Phase 4: Eventual Obligations (~3 days)

**Deliverables:**

- [ ] ObligationSpec compilation from invariants
- [ ] OblTracker state machine
- [ ] Tick-by-tick evaluation
- [ ] ttd.obligations channel

**Files:**

- `crates/echo-ttd/src/obligations.rs`

### Phase 5: Platform Crates (~1.5 weeks)

#### Phase 5a: ttd-browser (WASM)

**Deliverables:**

- [ ] Full WasmEngine implementation
- [ ] Transaction control (begin/commit)
- [ ] Cursor management (create/seek/step)
- [ ] Fork support (snapshot/fork_from_snapshot)
- [ ] Session & truth frame API
- [ ] Compliance API
- [ ] Web Worker multi-instance support

**Files:**

- `crates/ttd-browser/`

#### Phase 5b: ttd-native (Desktop)

**Deliverables:**

- [ ] egui + wgpu application shell
- [ ] Native session client (Unix socket or embedded)
- [ ] Panel implementations (reuse warp-viewer patterns)
- [ ] 3D scene rendering (port from warp-viewer)
- [ ] Compliance badge integration

**Files:**

- `crates/ttd-native/`

### Phase 6: Browser TTD App (~2 weeks)

**Deliverables:**

- [ ] React/Svelte UI shell
- [ ] Time controls + timeline
- [ ] Worldline tree with badges
- [ ] State inspector table
- [ ] Provenance drawer
- [ ] Fork manager with hash comparison
- [ ] Three.js 4D view (mouse worm, atom pillar, causal arrows)
- [ ] Integration with ttd-browser WASM

**Files:**

- `packages/ttd-app/`

### Phase 7: Demo Polish (~3 days)

**Deliverables:**

- [ ] "Cliff tick" physics scenario
- [ ] Demo script walkthrough
- [ ] Static hosting setup (Vercel/Cloudflare)
- [ ] README + docs

---

## Part 12: Dependencies & Order

### 12.1 Two-WARP Runtime Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              RUNTIME VIEW                                   │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │                      OBSERVED APP WARP                                 │ │
│  │  ┌─────────────┐                                                      │ │
│  │  │ App Engine  │──▶ Provenance Store ◀──────────────┐                │ │
│  │  │ (separate)  │                                     │                │ │
│  │  └─────────────┘                                     │                │ │
│  └──────────────────────────────────────────────────────│────────────────┘ │
│                                                          │ reads            │
│  ┌───────────────────────────────────────────────────────│───────────────┐ │
│  │                      TTD CONTROLLER WARP              │               │ │
│  │                                                       ▼               │ │
│  │  ┌─────────────┐   ┌─────────────┐   ┌─────────────────────────────┐ │ │
│  │  │ TTD Engine  │──▶│ TTD MBUS    │──▶│ ttd.head | ttd.errors |    │ │ │
│  │  │ (separate)  │   │ (channels)  │   │ ttd.state_inspector | ...  │ │ │
│  │  └──────▲──────┘   └─────────────┘   └──────────────┬──────────────┘ │ │
│  │         │                                           │                │ │
│  │         │ ingest_intent(EINT)                       │ TruthFrames   │ │
│  └─────────│───────────────────────────────────────────│────────────────┘ │
│            │                                           │                   │
│            │                                           ▼                   │
│  ┌─────────│───────────────────────────────────────────────────────────┐  │
│  │         │              TTD CLIENTS (MBUS Subscribers)               │  │
│  │         │                                                           │  │
│  │  ┌──────┴────────┐  ┌────────────────┐  ┌────────────────┐         │  │
│  │  │ ttd-browser   │  │  ttd-native    │  │  ttd-remote    │         │  │
│  │  │ send: EINT    │  │  send: EINT    │  │  send: EINT    │         │  │
│  │  │ recv: MBUS    │  │  recv: MBUS    │  │  recv: MBUS    │         │  │
│  │  │ render: dumb  │  │  render: dumb  │  │  render: dumb  │         │  │
│  │  └───────────────┘  └────────────────┘  └────────────────┘         │  │
│  └─────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 12.2 Build Dependency Order

```
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│  Phase 1: Wesley                                                │
│  ┌──────────────┐                                              │
│  │ SDL Parser   │                                              │
│  │ + Directives │                                              │
│  └──────┬───────┘                                              │
│         │                                                       │
│         ▼                                                       │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐     │
│  │ ttd-protocol │    │ ttd-manifest │    │ ttd-protocol │     │
│  │ -rs          │    │              │    │ -ts          │     │
│  └──────┬───────┘    └──────┬───────┘    └──────────────┘     │
│         │                   │                                   │
└─────────┼───────────────────┼───────────────────────────────────┘
          │                   │
          ▼                   ▼
┌─────────────────────────────────────────────────────────────────┐
│  Phase 2-4: TTD Core (with TtdController WARP)                  │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐     │
│  │ EINT v2      │    │ TTDR v2      │    │ Digests      │     │
│  │ (session-    │    │ (echo-ttd)   │    │ (warp-core)  │     │
│  │  proto)      │    │              │    │              │     │
│  └──────┬───────┘    └──────┬───────┘    └──────┬───────┘     │
│         │                   │                   │              │
│         └───────────────────┼───────────────────┘              │
│                             ▼                                   │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐     │
│  │ Compliance   │    │ Obligations  │    │ TtdController│     │
│  │ (echo-ttd)   │    │ (echo-ttd)   │    │ WARP Schema  │     │
│  └──────┬───────┘    └──────┬───────┘    └──────┬───────┘     │
│         │                   │                   │              │
└─────────┼───────────────────┼───────────────────┼──────────────┘
          │                   │                   │
          ▼                   ▼                   ▼
┌─────────────────────────────────────────────────────────────────┐
│  Phase 5: Platform Crates (MBUS Clients)                        │
│  ┌─────────────────────────┐    ┌─────────────────────────┐   │
│  │ ttd-browser             │    │ ttd-native              │   │
│  │ (WASM + wasm-bindgen)   │    │ (egui + wgpu)           │   │
│  │                         │    │                         │   │
│  │ Pattern: EINT in,       │    │ Pattern: EINT in,       │   │
│  │          MBUS out,      │    │          MBUS out,      │   │
│  │          render dumb    │    │          render dumb    │   │
│  └───────────┬─────────────┘    └─────────────────────────┘   │
│              │                                                  │
└──────────────┼──────────────────────────────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────────────────────────────┐
│  Phase 6: Browser App                                           │
│  ┌───────────────────────────────────────────────────────┐     │
│  │ ttd-app (React/Svelte + Three.js)                     │     │
│  │ deps: ttd-browser.wasm, ttd-protocol-ts               │     │
│  │                                                       │     │
│  │ Pattern: Subscribe channels, render TruthFrames       │     │
│  └───────────────────────────────────────────────────────┘     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Appendix A: TTD Channels Registry

| Channel ID        | Policy        | Reducer | Purpose                      |
| ----------------- | ------------- | ------- | ---------------------------- |
| `ttd.state`       | STRICT_SINGLE | LAST    | Authoritative state snapshot |
| `ttd.head`        | STRICT_SINGLE | LAST    | Current head position        |
| `ttd.receipt`     | LOG           | CONCAT  | Tick receipts                |
| `ttd.diff`        | STRICT_SINGLE | LAST    | State diff for rendering     |
| `ttd.provenance`  | LOG           | CONCAT  | AtomWrite records            |
| `ttd.ops`         | LOG           | CONCAT  | Ops registry manifest        |
| `ttd.rules`       | STRICT_SINGLE | LAST    | Rule contracts manifest      |
| `ttd.obligations` | LOG           | CONCAT  | Obligation status deltas     |
| `ttd.compliance`  | STRICT_SINGLE | LAST    | Compliance summary           |

---

## Appendix B: Invariant Expression Grammar

```ebnf
program     = statement*
statement   = call ";"

call        = target "." fn "(" args ")"
target      = "tick" | "rule" | "op"
fn          = identifier
args        = (arg ("," arg)*)?

arg         = STRING | STRLIST | EMITCOUNT | WITHIN_INT
STRING      = '"' [^"]* '"'
STRLIST     = "[" STRING ("," STRING)* "]"
EMITCOUNT   = "EXACTLY_ONE" | "AT_LEAST_ONE" | "ZERO_OR_MORE"
WITHIN_INT  = "within" INTEGER

identifier  = [a-zA-Z_][a-zA-Z0-9_]*
INTEGER     = [0-9]+
```

**Allowed Calls:**

| Target | Function          | Args                                    | Opcode |
| ------ | ----------------- | --------------------------------------- | ------ |
| tick   | requireVerified   | ()                                      | 0x01   |
| tick   | strictSingle      | (STRING)                                | 0x02   |
| tick   | forbidChannel     | (STRING)                                | 0x03   |
| tick   | mustEmit          | (STRING, EMITCOUNT)                     | 0x04   |
| rule   | mustEmit          | (STRING, STRING, EMITCOUNT)             | 0x10   |
| rule   | mayEmitOnly       | (STRING, STRLIST)                       | 0x11   |
| rule   | forbidSideEffects | (STRING, STRLIST)                       | 0x12   |
| op     | requiresChannels  | (STRING, STRLIST, WITHIN_INT)           | 0x20   |
| op     | produces          | (STRING, STRING, EMITCOUNT, WITHIN_INT) | 0x21   |

---

## Appendix C: Mockup References

All mockups in `docs/mockups/`:

| File                             | Description                                                    |
| -------------------------------- | -------------------------------------------------------------- |
| `warpsite-main-layout.svg`       | Full debugger: stepper, timeline, graph, atoms, tick inspector |
| `warpsite-provenance-drawer.svg` | Slide-out: atom history + causal chain                         |
| `warpsite-fork-comparison.svg`   | Side-by-side fork view with hash verification                  |
| `warpsite-4d-provenance.svg`     | Spacetime: mouse worm, atom pillar, causal arrows              |

---

## Part 13: Test Plan Addendum (Session + EINT + MBUS)

### 13.1 Test Invariants

#### Wire Invariants (JIT!)

1. `Packet.decode(Packet.encode(x)) == x`
2. Header fields validated strictly: magic exact, version supported, length exact, checksum exact
3. Canonical CBOR: definite lengths, sorted keys, no tags, deterministic floats

#### Intent Invariants (EINT v2)

1. `EINT.decode(EINT.encode(x)) == x`
2. schema_hash must match registry (or fail deterministically)
3. payload checksum must match blake3(payload_bytes)
4. opcode + op_version dispatch is stable and exhaustive

#### Truth Invariants (MBUS/TruthFrame)

1. `TruthFrame.value_hash == blake3(value_bytes)` always
2. Wire representation is canonical for given inputs
3. StrictSingle: exactly 1 finalized entry per tick (or deterministic error)

#### Contract Invariants

1. Each TTD intent produces required channels within window
2. TTD_SEEK → ttd.head EXACTLY_ONE within 3
3. TTD_FORK → ttd.worldline_tree + ttd.head within 5

### 13.2 Test Suite Map

| Suite                      | Location                                      | Proves                      |
| -------------------------- | --------------------------------------------- | --------------------------- |
| **A) echo-session-proto**  | `crates/echo-session-proto/tests/`            | JIT! transport correctness  |
| **B) echo-session-client** | `crates/echo-session-client/tests/`           | Streaming/framing stability |
| **C) warp-core MBUS**      | `crates/warp-core/src/materialization/tests/` | Truth bytes and policies    |
| **D) echo-ttd intents**    | `crates/echo-ttd/tests/`                      | EINT path and contracts     |
| **E) Integration**         | `crates/echo-ttd/tests/integration/`          | Full pipeline               |

### 13.3 Concrete Test Cases

#### A) echo-session-proto Tests

- `wire_packet_roundtrip_basic`
- `wire_reject_bad_magic`
- `wire_reject_bad_version`
- `wire_reject_bad_length`
- `wire_reject_bad_checksum`
- `cbor_reject_non_canonical_map_key_order`
- `cbor_reject_indefinite_length`
- `decode_unknown_op_string_is_error`

#### B) echo-session-client Tests

- `poll_message_partial_header_then_payload`
- `poll_message_multiple_frames_in_one_buffer`
- `poll_message_frame_split_across_reads`
- `backpressure_does_not_reorder_ts`

#### C) warp-core MBUS Tests

- `truthframe_value_hash_correct`
- `mbus_finalize_strict_single_ok`
- `mbus_finalize_strict_single_conflict_reports`
- `mbus_reduce_last_is_deterministic`
- `mbus_entry_ordering_is_canonical`
- `mbus_v2_entry_hashes_match_vectors`
- `emissions_digest_perm_invariance`

#### D) echo-ttd Intent Tests

- `eint_v2_roundtrip_vectors`
- `eint_v2_reject_schema_hash_mismatch`
- `eint_v2_reject_bad_payload_checksum`
- `ttd_seek_produces_head_within_3_satisfied`
- `ttd_seek_produces_head_within_3_failed`
- `ttd_step_forward_updates_tick_monotone`
- `ttd_step_back_respects_pin_max_tick`
- `ttd_fork_creates_worldline_and_head_within_5`
- `obligations_overlap_two_seeks_one_head_tick`
- `obligations_prune_deterministic`
- `worldline_isolation`

#### E) Integration Tests

- `handshake_subscribe_seek_receive_head`
- `send_step_receive_head_updated`
- `send_fork_receive_worldline_tree_and_head`
- `permutation_drill_same_emissions_digest`

### 13.4 Extended Golden Vector Format

```rust
struct GoldensV1 {
    metadata: GoldensMeta,
    vectors: Vec<GoldenVector>,
}

struct GoldensMeta {
    generated_at: u64,
    schema_hash: Hash,
    envelope_version: u16,
    receipt_version: u16,
}

struct GoldenVector {
    id: String,
    category: VectorCategory,
    input: Vec<u8>,
    expected: VectorExpected,
    tags: Vec<String>,  // ["jit", "eint", "mbus", "obligation"]
}

enum VectorCategory {
    JitFrame,
    EintPacket,
    MbusFrame,
    ObligationScenario,
    Roundtrip,
    Rejection,
}

enum VectorExpected {
    Bytes(Vec<u8>),
    Hash(Hash),
    ParseError(String),
    ObligationOutcome {
        satisfied: Vec<(u32, u32)>,
        failed: Vec<(u32, u32)>,
    },
}
```

### 13.5 TTD Fixture App (for Testing)

Minimal schema for testing TTD mechanics without real app complexity:

```graphql
# ttd-fixture-app schema (for testing only)

type Counter @node {
    id: NodeId!
    value: U64!
}

type Accumulator @node {
    id: NodeId!
    total: U64!
    last_input: NodeId # Edge to Counter that last modified it
}

# Rules:
# 1. IncrementCounter: on CounterIntent, increments Counter.value
# 2. AccumulateTotal: on Counter.value change, adds to Accumulator.total

# Channels:
# - fixture.state (STRICT_SINGLE): Counter + Accumulator values
# - fixture.events (LOG): all mutations

# Provides:
# - Causal chain (Counter → Accumulator dependency)
# - Provenance (which rule wrote which atom)
# - Deterministic values for golden tests
```

---

## Appendix D: TTD Error Codes

```rust
pub enum TtdErrorCode {
    // EINT validation (0x01xx)
    EintSchemaMismatch       = 0x0100,
    EintPayloadChecksumMismatch = 0x0101,
    EintUnknownOpcode        = 0x0102,
    EintVersionUnsupported   = 0x0103,

    // Seek errors (0x02xx)
    SeekTargetOutOfRange     = 0x0200,
    SeekHistoryUnavailable   = 0x0201,
    SeekStateRootMismatch    = 0x0202,
    SeekPatchDigestMismatch  = 0x0203,
    SeekCommitHashMismatch   = 0x0204,
    SeekApplyError           = 0x0205,

    // Fork errors (0x03xx)
    ForkSourceCursorNotFound = 0x0300,
    ForkSnapshotCorrupt      = 0x0301,
    ForkWorldlineLimitExceeded = 0x0302,

    // Session errors (0x04xx)
    SessionNotFound          = 0x0400,
    CursorNotFound           = 0x0401,
    SubscriptionInvalid      = 0x0402,

    // Compliance errors (0x05xx)
    ComplianceViolation      = 0x0500,
    ObligationFailed         = 0x0501,
}
```

---

## Appendix E: Version Compatibility Matrix

### Version Decode/Encode Policy

| Component     | Decode | Encode |
| ------------- | ------ | ------ |
| JIT! packet   | v1     | v1     |
| EINT envelope | v1, v2 | v2     |
| MBUS frame    | v1, v2 | v2     |
| TTDR receipt  | v1, v2 | v2     |

**Rule:** Decoders accept multiple versions; encoders always output latest.

### Schema Hash Compatibility

- Same opcode across schema_hash A vs B → **distinct meaning**
- Dispatch uses `(schema_hash, opcode, op_version)` not just opcode
- Schema hash mismatch on EINT → `EintSchemaMismatch` error

---

**End of Plan**

_This document is the authoritative specification for the TTD application._
