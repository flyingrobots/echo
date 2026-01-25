<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Wesley Phase 1 Handoff

**Date:** 2026-01-25
**From:** TTD Architecture Session
**To:** Implementation Agent
**Status:** Ready for execution

---

## Executive Summary

Implement **Wesley v0 (Foundation)** — the first stage of the TTD schema compiler. This stage produces JSON manifests only. No codegen. No bytecode.

**Doctrine:** Wesley is not "codegen" — it's **law compilation**. The schema defines the universe.

---

## What Was Decided

### Protocol Architecture (Approved)

```
JIT!/OpEnvelope  →  Session transport (subscriptions, streaming)
EINT v2          →  Causality inputs (intents, versioned, schema-bound)
MBUS v2          →  Truth output (TruthFrames, emissions_digest)
```

### TTD as Meta-WARP (Approved)

The TTD Controller is itself a WARP graph:

- Atoms: `ttd/cursor`, `ttd/session`, `ttd/worldline`, etc.
- Rules: `ttd.handle_seek`, `ttd.handle_step`, `ttd.handle_fork`, etc.
- Channels: `ttd.head`, `ttd.errors`, `ttd.state_inspector`, etc.

### Wesley Staging (Approved)

| Stage             | Scope                     | This Handoff   |
| ----------------- | ------------------------- | -------------- |
| **v0 Foundation** | Parser, AST, manifests    | ✅ **DO THIS** |
| v1 Codegen        | Rust/TS types, registries | Later          |
| v2 Law Compiler   | Invariants, bytecode      | Later          |

---

## Phase 1a: Wesley v0 Scope

### Deliverables

1. **GraphQL SDL Parser**
    - Parse schema with directives
    - Extract `@channel`, `@op`, `@rule`, `@invariant`, etc.
    - Handle all directives from Part 2.1 of ttd-app.md

2. **AST Model**
    - `SchemaAST` with channels, ops, rules, invariants
    - Preserve directive arguments
    - Canonical field ordering for hashing

3. **Schema Hashing**
    - Deterministic `schema_hash` from canonical SDL representation
    - BLAKE3 hash of normalized schema

4. **JSON Manifest Output**

    ```
    schema.json     - Schema metadata + hash
    manifest.json   - Channel/op/rule registries
    contracts.json  - Emission contracts, footprint specs
    ```

### NOT In Scope (Explicitly Forbidden)

- ❌ Rust code generation
- ❌ TypeScript code generation
- ❌ CBOR codecs
- ❌ Invariant bytecode compilation
- ❌ Registry lookup code

These come in v1 and v2.

---

## File Structure

```
crates/wesley/
├── Cargo.toml
├── src/
│   ├── lib.rs           # Public API
│   ├── parser.rs        # GraphQL SDL parsing
│   ├── ast.rs           # AST types
│   ├── directives.rs    # Directive extraction
│   ├── hash.rs          # Schema hashing
│   ├── manifest.rs      # JSON output generation
│   └── error.rs         # Error types
└── tests/
    ├── parse_basic.rs
    ├── parse_directives.rs
    ├── hash_stability.rs
    └── fixtures/
        ├── minimal.graphql
        ├── ttd_channels.graphql
        └── full_ttd.graphql
```

---

## Key Reference Files

| File                                                | Purpose                             |
| --------------------------------------------------- | ----------------------------------- |
| `docs/plans/ttd-app.md`                             | Master plan (Part 2 for directives) |
| `crates/warp-core/src/materialization/channel.rs`   | Existing ChannelPolicy enum         |
| `crates/warp-core/src/materialization/reduce_op.rs` | Existing ReduceOp enum              |
| `crates/echo-session-proto/src/wire.rs`             | Existing wire protocol patterns     |

---

## Directive Vocabulary (From Part 2.1)

### Must Parse

```graphql
# Determinism
@canonicalCbor(version: U32 = 1)
@noFloat
@fixed(kind: String!, scale: I32)
@sorted(by: [String!]!)
@noUnorderedMap
@keyBytes

# Channels
@channel(id: ChannelId!, version: U16!, policy: ChannelPolicy!, reducer: ReducerKind, doc: String)
@emitKey(type: String!)
@entryType(name: String!)

# Ops
@op(opcode: String!, version: U16!, kind: OpKind!, response: String, doc: String)
@opError(code: String!, severity: String = "ERROR")

# Rules
@rule(id: RuleId!, version: U16!)
@triggerOp(opcode: String!, phase: String)
@triggerEvent(eventKind: String!)
@footprintRead(kind: String!, argType: String)
@footprintWrite(kind: String!, argType: String)
@mustEmit(channel: ChannelId!, count: EmitCount!)
@mayEmitOnly(channels: [ChannelId!]!)
@ruleDeterminism(kind: String!, detail: String)
@noSideEffects(kinds: [String!]!)

# Invariants
@invariant(id: String!, severity: InvariantSeverity!, kind: String!, expr: String!, doc: String)
```

---

## Output Format

### schema.json

```json
{
    "schema_hash": "abc123...",
    "version": 1,
    "generated_at": "2026-01-25T...",
    "source_files": ["ttd.graphql"],
    "channels": [
        {
            "id": "ttd.head",
            "version": 1,
            "policy": "STRICT_SINGLE",
            "reducer": "LAST",
            "payload_type": "TtdHeadPayload"
        }
    ],
    "ops": [
        {
            "opcode": "TTD_SEEK",
            "version": 1,
            "kind": "COMMAND",
            "payload_type": "CmdSeek",
            "response_type": "Ack"
        }
    ],
    "rules": [
        {
            "id": "ttd.handle_seek",
            "version": 1,
            "triggers": [{ "op": "TTD_SEEK", "phase": "post" }]
        }
    ]
}
```

### manifest.json

```json
{
  "channel_registry": {
    "ttd.head": {"id_bytes": "...", "policy": "STRICT_SINGLE", "reducer": "LAST"},
    "ttd.errors": {"id_bytes": "...", "policy": "LOG", "reducer": "CONCAT"}
  },
  "op_registry": {
    "TTD_SEEK": {"opcode": 1, "version": 1, "kind": "COMMAND"},
    "TTD_STEP": {"opcode": 2, "version": 1, "kind": "COMMAND"}
  },
  "rule_registry": {
    "ttd.handle_seek": {"triggers": [...], "footprints": {...}}
  }
}
```

### contracts.json

```json
{
    "emission_contracts": [
        {
            "rule_id": "ttd.handle_seek",
            "must_emit": [{ "channel": "ttd.head", "count": "EXACTLY_ONE" }],
            "may_emit_only": ["ttd.head", "ttd.errors", "ttd.state_inspector"]
        }
    ],
    "footprint_specs": [
        {
            "rule_id": "ttd.handle_seek",
            "reads": [{ "kind": "AtomId", "arg_type": "CursorId" }],
            "writes": [{ "kind": "AtomId", "arg_type": "CursorId" }]
        }
    ],
    "invariants": [
        {
            "id": "SEEK_PRODUCES_HEAD",
            "severity": "ERROR",
            "kind": "EVENTUAL",
            "expr": "op.produces(\"TTD_SEEK\", \"ttd.head\", EXACTLY_ONE, within 3)"
        }
    ]
}
```

---

## Success Criteria

### Must Pass

1. **Parse test schemas without error**
    - `fixtures/minimal.graphql`
    - `fixtures/ttd_channels.graphql`
    - `fixtures/full_ttd.graphql`

2. **Schema hash stability**
    - Same schema → same hash (deterministic)
    - Whitespace/comment changes → same hash
    - Semantic changes → different hash

3. **Directive extraction complete**
    - All 25+ directives parsed
    - Arguments preserved with correct types
    - Unknown directives produce warnings (not errors)

4. **JSON output valid**
    - Parseable by `serde_json`
    - Matches documented schema above
    - Canonical ordering (sorted keys)

### Nice to Have

- CLI tool: `wesley parse schema.graphql -o output/`
- Watch mode for development
- Diff mode for schema comparison

---

## Anti-Patterns to Avoid

1. **Don't generate code** — That's v1
2. **Don't compile invariants** — That's v2
3. **Don't parse CBOR** — That's v1 codecs
4. **Don't create lookup tables** — That's v1 registries
5. **Don't optimize for speed** — Correctness first

---

## Suggested Approach

1. **Start with parser** using `graphql-parser` crate
2. **Build AST types** that mirror directive structure
3. **Extract directives** into typed structs
4. **Implement hash** using canonical serialization
5. **Generate JSON** with serde

### Recommended Crates

```toml
[dependencies]
graphql-parser = "0.4"      # SDL parsing
serde = { version = "1", features = ["derive"] }
serde_json = "1"
blake3 = "1"                # Schema hashing
thiserror = "1"             # Error types
```

---

## Test Fixtures

### minimal.graphql

```graphql
type Query {
    hello: String
}

type TtdHeadPayload
    @channel(id: "ttd.head", version: 1, policy: STRICT_SINGLE, reducer: LAST) {
    tick: Int!
}
```

### ttd_channels.graphql

```graphql
# All TTD channels from Appendix A
type TtdHeadPayload @channel(id: "ttd.head", version: 1, policy: STRICT_SINGLE, reducer: LAST) { ... }
type TtdErrorsPayload @channel(id: "ttd.errors", version: 1, policy: LOG, reducer: CONCAT) { ... }
# ... etc
```

---

## Knowledge Graph Entities

The following entities are available in the knowledge graph for context:

- `TtdController_WARP` — TTD as meta-WARP architecture
- `TTD_Protocol_Boundary` — JIT!/EINT/MBUS separation
- `Wesley_Staging_Model` — v0/v1/v2 breakdown
- `MaterializationBus` — Existing MBUS implementation
- `MBUS_FrameV2` — Frame protocol

---

## Questions for Implementer

If blocked, these are acceptable to ask:

1. Should unknown directives be errors or warnings?
2. What's the canonical ordering for schema hash input?
3. Should we preserve source locations in AST for error messages?

---

**End of Handoff**

_Execute Wesley v0. No more, no less._
