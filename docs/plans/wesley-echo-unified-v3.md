<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Wesley + Echo Unified Architecture v3

**Status:** Proposal
**Date:** 2026-01-27
**Supersedes:** wesley-architecture-v2.md, all prior Wesley/Echo integration docs

---

## Executive Summary

This plan resolves the confusion around Wesley's role, codec strategy, and Echo integration.

### Key Decisions

1. **Wesley becomes a Rust library** — No more JS/Node subprocess. Compiles to WASM for browser/JS compatibility.

2. **Raw little-endian is the default codec** — CBOR is overkill for most use cases. TypeId provides type identification.

3. **Schemas live in Echo** — Under `janus/wesley/schema/`. Wesley is a generic compiler, not a schema host.

4. **Wesley generates encoders/decoders** — Not just type definitions, but actual serialization code.

5. **Per-type versioning** — Each type has a stable TypeId, semantic version, and content hash.

### Why v3 Supersedes v2

- v2 assumed CBOR everywhere. Research shows raw_le works fine (flyingrobots.dev proves it).
- v2 didn't address encoder/decoder generation. Wesley currently only generates type definitions.
- v2 didn't clarify when CBOR is actually needed (answer: almost never with WASM).

---

## Part 1: Wesley Rust Core

### Module Structure

```
crates/wesley/
├── Cargo.toml
└── src/
    ├── lib.rs              # Public API
    ├── parser/
    │   ├── mod.rs
    │   ├── lexer.rs        # GraphQL tokenizer
    │   └── ast.rs          # GraphQL AST
    ├── ir/
    │   ├── mod.rs
    │   ├── schema.rs       # Schema IR
    │   ├── types.rs        # Type definitions
    │   └── directives.rs   # @wes_* directive handling
    ├── codegen/
    │   ├── mod.rs
    │   ├── rust.rs         # Rust code generation
    │   ├── typescript.rs   # TypeScript code generation
    │   └── codec.rs        # Encoder/decoder generation
    ├── hash.rs             # TypeId and schema hashing
    └── version.rs          # Per-type versioning
```

### Recommended Crates

| Purpose         | Crate                             |
| --------------- | --------------------------------- |
| GraphQL parsing | `apollo-parser` (error-resilient) |
| Rust codegen    | `quote` + `syn` + `prettyplease`  |
| Hashing         | `blake3` (fast, deterministic)    |
| Error reporting | `miette` (nice diagnostics)       |
| WASM bindings   | `wasm-bindgen`                    |

### Library API

```rust
use wesley::{Schema, CodegenTarget, CodecFormat};

// Parse schema
let schema = wesley::parse_schema(include_str!("schema.graphql"))?;

// Generate Rust code with raw_le codec
let rust_code = wesley::generate(
    &schema,
    CodegenTarget::Rust,
    CodecFormat::RawLe,  // Default!
)?;

// Generate TypeScript
let ts_code = wesley::generate(
    &schema,
    CodegenTarget::TypeScript,
    CodecFormat::RawLe,
)?;
```

### WASM Compilation

```rust
// crates/wesley-wasm/src/lib.rs
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn compile_schema(graphql: &str) -> Result<JsValue, JsError> {
    let schema = wesley::parse_schema(graphql)?;
    let output = wesley::generate(&schema, CodegenTarget::TypeScript, CodecFormat::RawLe)?;
    Ok(JsValue::from_str(&output))
}

#[wasm_bindgen]
pub fn encode(type_id: &[u8], data: &JsValue) -> Result<Vec<u8>, JsError> {
    // Generated encoder - same logic as Rust version
}

#[wasm_bindgen]
pub fn decode(type_id: &[u8], bytes: &[u8]) -> Result<JsValue, JsError> {
    // Generated decoder - same logic as Rust version
}
```

**Key insight:** With Wesley-WASM, JavaScript uses the **exact same encoder/decoder** as Rust. No need for CBOR to bridge languages.

---

## Part 2: Codec Strategy

### The New Default: Raw Little-Endian

**Why raw_le wins:**

| Aspect         | raw_le                  | CBOR                      |
| -------------- | ----------------------- | ------------------------- |
| Speed          | Fastest (direct memory) | Slower (parse overhead)   |
| Size           | Minimal                 | ~10-30% larger            |
| Complexity     | Trivial                 | Requires library          |
| Cross-language | Works with shared codec | Self-describing           |
| Determinism    | Trivially deterministic | Requires "canonical" mode |

**flyingrobots.dev proves it works:**

- All game types use raw little-endian
- TypeId identifies the format
- Zero serialization overhead
- Works perfectly across Rust and WASM

### When CBOR is Appropriate

CBOR is only needed when:

1. **No shared codec** — Receiver doesn't have Wesley-generated decoders
2. **Dynamic typing** — Type not known until runtime (rare)
3. **External systems** — Interop with non-Echo systems that expect CBOR

**With Wesley-WASM, this is almost never:**

- JS/browser uses Wesley-WASM for encoding/decoding
- Same generated code as Rust
- TypeId identifies the type
- Raw bytes flow through WASM linear memory

### @wes_codec Directive

```graphql
# Default: raw little-endian (fastest, recommended)
type Motion @wes_codec(format: "raw_le") {
    posX: Int!
    posY: Int!
    posZ: Int!
}

# CBOR: only for external interop
type ExternalMessage @wes_codec(format: "cbor", canonical: true) {
    payload: JSON!
}

# Custom: user provides encoder/decoder
type Legacy
    @wes_codec(
        format: "custom"
        encoder: "encode_legacy"
        decoder: "decode_legacy"
    ) {
    data: Bytes!
}
```

### Generated Encoder/Decoder (raw_le)

```rust
// Generated by Wesley
impl Motion {
    pub const TYPE_ID: TypeId = TypeId::from_hash("Motion/v1");

    pub fn to_bytes(&self) -> [u8; 24] {
        let mut buf = [0u8; 24];
        buf[0..8].copy_from_slice(&self.pos_x.to_le_bytes());
        buf[8..16].copy_from_slice(&self.pos_y.to_le_bytes());
        buf[16..24].copy_from_slice(&self.pos_z.to_le_bytes());
        buf
    }

    pub fn from_bytes(buf: &[u8; 24]) -> Self {
        Self {
            pos_x: i64::from_le_bytes(buf[0..8].try_into().unwrap()),
            pos_y: i64::from_le_bytes(buf[8..16].try_into().unwrap()),
            pos_z: i64::from_le_bytes(buf[16..24].try_into().unwrap()),
        }
    }
}
```

---

## Part 3: Echo Integration (Janus)

### Directory Structure

```
echo/
├── janus/                              # TTD subsystem
│   ├── wesley/
│   │   ├── schema/
│   │   │   ├── ttd-protocol.graphql    # TTD protocol types
│   │   │   └── game-types.graphql      # App-specific types
│   │   │
│   │   ├── generated/
│   │   │   ├── rust/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── ttd_protocol.generated.rs
│   │   │   │   └── game_types.generated.rs
│   │   │   │
│   │   │   ├── typescript/
│   │   │   │   ├── index.ts
│   │   │   │   ├── ttd-protocol.generated.ts
│   │   │   │   └── game-types.generated.ts
│   │   │   │
│   │   │   └── manifest/
│   │   │       └── registry.json
│   │   │
│   │   └── wesley.lock                 # Provenance tracking
│   │
│   ├── compliance/                     # Was echo-ttd
│   └── browser/                        # Was ttd-browser
│
├── crates/
│   ├── wesley/                         # Wesley Rust core (NEW)
│   ├── wesley-wasm/                    # WASM bindings (NEW)
│   ├── janus-types/                    # Was ttd-protocol-rs
│   └── janus-compliance/               # Was echo-ttd
│
└── packages/
    └── janus-types-ts/                 # Was ttd-protocol-ts
```

### File Naming Convention

All generated files use `.generated.` suffix:

- `foo.generated.rs` — Rust
- `foo.generated.ts` — TypeScript
- `foo.generated.json` — Manifests

This makes it **obvious** what should never be hand-edited.

### Build Integration

```rust
// xtask/src/main.rs

/// Regenerate all Wesley outputs from schema
fn janus_sync() -> Result<()> {
    let schemas = glob("janus/wesley/schema/*.graphql")?;

    for schema_path in schemas {
        let schema = wesley::parse_schema(&fs::read_to_string(&schema_path)?)?;

        // Generate Rust
        let rust = wesley::generate(&schema, CodegenTarget::Rust, CodecFormat::RawLe)?;
        let rust_path = format!(
            "janus/wesley/generated/rust/{}.generated.rs",
            schema_path.file_stem()
        );
        fs::write(&rust_path, rust)?;

        // Generate TypeScript
        let ts = wesley::generate(&schema, CodegenTarget::TypeScript, CodecFormat::RawLe)?;
        let ts_path = format!(
            "janus/wesley/generated/typescript/{}.generated.ts",
            schema_path.file_stem()
        );
        fs::write(&ts_path, ts)?;
    }

    // Update lock file
    update_wesley_lock()?;

    Ok(())
}
```

---

## Part 4: Per-Type Versioning

### TypeId: Stable Identity

```rust
/// TypeId is a 32-byte hash derived from type name + version
/// It NEVER changes once assigned (even if type evolves)
pub struct TypeId([u8; 32]);

impl TypeId {
    /// Create from type name and major version
    pub fn new(name: &str, major: u8) -> Self {
        let input = format!("{}/v{}", name, major);
        Self(blake3::hash(input.as_bytes()).into())
    }
}
```

**Key rule:** TypeId only changes on **major version bump** (breaking change).

### Semantic Versioning

```graphql
type Motion @wes_version(major: 1, minor: 2) @wes_codec(format: "raw_le") {
    posX: Int!
    posY: Int!
    posZ: Int!
    # Added in v1.1:
    velX: Int
    velY: Int
    velZ: Int
}
```

| Change             | Version Bump | TypeId Changes? |
| ------------------ | ------------ | --------------- |
| Add optional field | minor        | No              |
| Add required field | major        | Yes             |
| Remove field       | major        | Yes             |
| Rename field       | major        | Yes             |
| Change field type  | major        | Yes             |

### Generated Metadata

```rust
// Generated by Wesley
impl Motion {
    pub const TYPE_ID: TypeId = TypeId([0x7c, 0x4a, ...]); // Stable
    pub const VERSION: (u8, u8) = (1, 2);                   // major.minor
    pub const CONTENT_HASH: [u8; 32] = [...];               // Exact definition hash

    /// Check wire compatibility
    pub fn is_compatible_with(other_type_id: &TypeId) -> bool {
        // Same TypeId = same major version = compatible
        Self::TYPE_ID == *other_type_id
    }
}
```

---

## Part 5: Migration Path

### Phase 1: Directory Reorganization (Week 1)

1. Create `janus/` directory structure
2. Move `schemas/ttd-protocol.graphql` → `janus/wesley/schema/`
3. Move generated outputs to `janus/wesley/generated/`
4. Update all imports and paths
5. Rename crates: `ttd-*` → `janus-*`

### Phase 2: Wesley Rust Core (Weeks 2-3)

1. Create `crates/wesley/` with parser module
2. Port GraphQL parsing from JS (use `apollo-parser`)
3. Implement IR and directive handling
4. Implement Rust codegen with raw_le
5. Implement TypeScript codegen

### Phase 3: Codec Generation (Week 4)

1. Generate actual `to_bytes()` / `from_bytes()` methods
2. Support `@wes_codec(format: "raw_le")` (default)
3. Support `@wes_codec(format: "cbor")` (legacy/interop)
4. Remove manual encode/decode boilerplate from flyingrobots.dev

### Phase 4: WASM + Integration (Week 5)

1. Create `crates/wesley-wasm/` with wasm-bindgen
2. Update xtask to use Wesley-as-library (not subprocess)
3. Update pre-commit and CI
4. Deprecate JS Wesley

### flyingrobots.dev Migration

The `ir.json` approach becomes obsolete:

1. Convert `ir.json` → `game-types.graphql`
2. Run `cargo xtask janus sync`
3. Delete `ir.json` and `generate_binary_codecs.mjs`
4. Update imports to use `janus-types`

---

## Appendix: Codec Comparison

| Aspect              | raw_le               | CBOR              | Custom                 |
| ------------------- | -------------------- | ----------------- | ---------------------- |
| **Speed**           | ⚡ Fastest           | 🐢 Slower         | Varies                 |
| **Size**            | Minimal              | +10-30%           | Varies                 |
| **Determinism**     | ✅ Trivial           | ✅ With canonical | ⚠️ Your responsibility |
| **Cross-language**  | ✅ With Wesley-WASM  | ✅ Native         | ⚠️ Manual              |
| **Self-describing** | ❌ Needs TypeId      | ✅ Built-in       | ❌ Needs TypeId        |
| **Nested types**    | ⚠️ Flat only         | ✅ Native         | Varies                 |
| **Use case**        | Game data, hot paths | External interop  | Legacy systems         |

### Recommendation

```graphql
# 95% of types: use raw_le (default)
type Motion @wes_codec(format: "raw_le") { ... }
type GameState @wes_codec(format: "raw_le") { ... }

# Only for external systems without Wesley
type ExternalApiMessage @wes_codec(format: "cbor") { ... }
```

---

## Summary

| Before                          | After                                    |
| ------------------------------- | ---------------------------------------- |
| Wesley is JavaScript            | Wesley is Rust library + WASM            |
| CBOR everywhere                 | raw_le default, CBOR optional            |
| Schemas in Wesley repo          | Schemas in Echo (`janus/wesley/schema/`) |
| Types only, manual codecs       | Wesley generates encoders/decoders       |
| Scattered artifacts             | Unified `janus/` subsystem               |
| Global schema hash              | Per-type versioning                      |
| Rules in Rust (error-prone)     | Rules in Rhai (sandboxed determinism)    |
| Footprint violations at runtime | Footprint violations at build time       |

### The Complete Developer Experience

```
1. Define types     →  janus/wesley/schema/game.graphql
2. Generate code    →  cargo xtask janus sync
3. Write rules      →  rules/movement.rhai (NOT Rust!)
4. Test & iterate   →  Hot-reload Rhai, replay for determinism
```

**Wesley + Rhai together** solve the two hardest problems:

- **Wesley**: Type-safe serialization without boilerplate
- **Rhai**: Deterministic game logic without footguns

**The end result:** Developers define types in GraphQL, run `cargo xtask janus sync`, and get fully-functional Rust + TypeScript code with blazing-fast raw binary serialization. No manual encode/decode. No CBOR overhead. No confusion.

---

## Part 6: Rhai Scripting Layer

### Why Rhai?

Rhai provides **sandboxed determinism** for game rules. Developers write gameplay logic in Rhai, not Rust:

| Concern             | Rhai Approach                          |
| ------------------- | -------------------------------------- |
| **Non-determinism** | No `HostTime`, no IO, no threads       |
| **Side effects**    | All mutations through `warp.apply()`   |
| **Concurrency**     | Single-threaded per branch             |
| **Budget**          | Engine tick-budgeted deterministically |

### Developer Workflow

1. **Define types** in Wesley GraphQL schema
2. **Run** `cargo xtask janus sync` to generate types
3. **Write rules** in Rhai scripts (not Rust!)
4. **Test** with deterministic replay

### Example: Movement Rule

```rhai
// rules/movement.rhai

fn match_moving_entities(entity) {
    // Return true if entity has Motion component
    warp.has_component(entity, Motion::TYPE_ID)
}

fn apply_movement(entity) {
    let motion = warp.get(entity, Motion::TYPE_ID);

    // Update position (deterministic fixed-point math)
    motion.pos_x += motion.vel_x;
    motion.pos_y += motion.vel_y;
    motion.pos_z += motion.vel_z;

    // Apply gravity
    if motion.pos_y > 0 {
        motion.vel_y -= GRAVITY;
    }

    // Emit the update through warp (never mutate directly)
    warp.apply("update_motion", entity, motion);
}

// Register the rule
warp.register_rule("movement", #{
    matcher: match_moving_entities,
    executor: apply_movement,
    footprint: #{
        reads: [Motion::TYPE_ID],
        writes: [Motion::TYPE_ID],
    }
});
```

### Rhai Bindings (provided by warp-core)

```rhai
// Query
warp.has_component(entity, type_id)     // Check component existence
warp.get(entity, type_id)               // Get component data
warp.query(type_id)                     // Iterate all entities with component

// Mutation (always through apply)
warp.apply(rule_name, scope, params)    // Apply a rewrite rule
warp.spawn(template)                    // Create new entity
warp.despawn(entity)                    // Remove entity

// Time (deterministic)
warp.tick()                             // Current tick number
warp.delay(ticks, callback)             // Schedule future execution

// NO access to:
// - System time (HostTime)
// - File IO
// - Network
// - Random (use warp.prng() seeded deterministically)
```

### Why Not Rust for Rules?

| Aspect        | Rust Rules               | Rhai Rules          |
| ------------- | ------------------------ | ------------------- |
| **Safety**    | Can bypass determinism   | Sandboxed by design |
| **Iteration** | Recompile on change      | Hot-reload          |
| **Audience**  | Engine developers        | Game designers      |
| **Footguns**  | Many (threads, time, IO) | None (sandboxed)    |

Rust rules exist for **internal engine systems** and **performance-critical paths**. Game developers use Rhai.

### Integration with Wesley Types

Wesley-generated types are available in Rhai:

```rhai
// Wesley generates these constants
let motion = Motion {
    pos_x: 100,
    pos_y: 200,
    pos_z: 0,
    vel_x: 10,
    vel_y: 0,
    vel_z: 0,
};

// TypeId is available
print(Motion::TYPE_ID);  // [0x7c, 0x4a, ...]

// Serialization is automatic
let bytes = motion.to_bytes();  // raw_le encoding
let decoded = Motion::from_bytes(bytes);
```

### S1 Milestone: Deterministic Rhai Surface

From the roadmap (issue #173):

> **S1 – Deterministic Rhai Surface** (Target: "law vs physics" sandbox)
>
> - Deterministic Rhai embedding; no HostTime/IO without Views/claims
> - (Optional) fiber model with `ViewClaim` / `EffectClaim` receipts

This ensures Rhai scripts produce **identical results** given identical inputs, enabling:

- Replay/rollback
- Multiplayer determinism
- Time-travel debugging

---

## Part 7: Build-Time Footprint Enforcement

### Current State: Runtime Enforcement

warp-core has a `FootprintGuard` that panics at runtime if a rule accesses undeclared resources:

```rust
// Runtime check - catches violations AFTER they happen
pub(crate) fn check_node_read(&self, id: &NodeId) {
    if !self.nodes_read.contains(id) {
        std::panic::panic_any(FootprintViolation { ... });
    }
}
```

**Problems:**

- Violations discovered at runtime (in production!)
- Requires comprehensive test coverage to catch issues
- False confidence: "tests pass" ≠ "no footprint bugs"

### Wesley Can Enforce at Build Time

If Wesley knows the rule's declared footprint AND generates the Rhai bindings, it can make violations **impossible**:

#### Strategy 1: Type-Scoped Bindings (Strongest)

Generate Rhai bindings that ONLY expose declared types:

```graphql
# Schema declares the footprint
extend type Mutation {
    applyPhysics(entity: ID!): Motion
        @wes_rule(
            reads: [Motion, Position] # Can read these
            writes: [Motion] # Can write this
            # Implicitly CANNOT access: Health, Inventory, etc.
        )
}
```

Wesley generates a **rule-specific module** with only declared types:

```rhai
// GENERATED: rules/physics.scope.generated.rhai
// This module ONLY exposes types declared in the footprint

// ✅ Allowed - declared in reads
fn get_Motion(entity) { warp.get(entity, Motion_TYPE_ID) }
fn get_Position(entity) { warp.get(entity, Position_TYPE_ID) }

// ✅ Allowed - declared in writes
fn set_Motion(entity, data) { warp.set(entity, Motion_TYPE_ID, data) }

// ❌ NOT GENERATED - Health not in footprint
// fn get_Health(entity) { ... }  // DOES NOT EXIST
```

The physics rule imports this scoped module:

```rhai
// rules/physics.rhai
import "rules/physics.scope.generated";  // Only Motion, Position available

fn physics_apply(entity) {
    let m = get_Motion(entity);     // ✅ Works
    let p = get_Position(entity);   // ✅ Works
    let h = get_Health(entity);     // ❌ COMPILE ERROR: undefined function

    // ...
}
```

**Result:** Footprint violations become **compile-time errors**, not runtime panics.

#### Strategy 2: Static Analysis (Complementary)

Wesley can analyze Rhai scripts to detect potential violations:

```
$ cargo xtask janus check

Analyzing rules/physics.rhai...
  ✓ get_Motion - declared in reads
  ✓ set_Motion - declared in writes
  ✗ ERROR: get_Health at line 15 - Health not in footprint

Footprint violation detected. Fix the rule or update the schema.
```

This catches issues even if the developer accidentally imports the wrong module.

#### Strategy 3: Conflict Detection at Build Time

Wesley can analyze ALL rules and detect conflicts:

```
$ cargo xtask janus check --conflicts

Analyzing rule footprints...

physics_rule:
  reads:  [Motion, Position]
  writes: [Motion]

damage_rule:
  reads:  [Health, Motion]
  writes: [Health]

Conflict analysis:
  ✓ physics_rule ∩ damage_rule = {Motion read-read} → SAFE (can parallelize)

collision_rule:
  reads:  [Position]
  writes: [Position, Motion]

  ✗ physics_rule ∩ collision_rule = {Motion write-write} → CONFLICT
    These rules CANNOT run in parallel on the same entity.
    Consider:
      - Ordering constraint (@wes_rule(after: "collision"))
      - Spatial partitioning (different entities)
```

### Schema Extensions for Footprint

```graphql
# Declare component types
type Motion @wes_component { ... }
type Position @wes_component { ... }
type Health @wes_component { ... }

# Declare rule with footprint
extend type Mutation {
  applyPhysics(entity: ID!): Motion
    @wes_rule(
      reads: [Motion, Position],
      writes: [Motion],
      # Optional: ordering constraints
      after: ["input_processing"],
      before: ["collision_detection"],
    )
}
```

### Generated Artifacts for Footprint

```
janus/wesley/generated/
├── rust/
│   ├── game_types.generated.rs      # Types
│   └── footprints.generated.rs      # Static footprint metadata
├── rhai/
│   ├── game_types.generated.rhai    # All type bindings
│   └── scopes/
│       ├── physics.scope.rhai       # Only Motion, Position
│       ├── damage.scope.rhai        # Only Health, Motion
│       └── collision.scope.rhai     # Only Position, Motion
└── manifest/
    └── rule_graph.json              # Conflict/ordering analysis
```

### Why This Matters

| Aspect           | Runtime (Current) | Build-Time (Wesley)         |
| ---------------- | ----------------- | --------------------------- |
| **When caught**  | Production crash  | `cargo build` failure       |
| **Coverage**     | Depends on tests  | 100% guaranteed             |
| **Confidence**   | "Tests pass"      | "Impossible to violate"     |
| **Parallelism**  | Hope for the best | Proven safe at compile time |
| **Developer UX** | Mysterious panics | Clear error messages        |

### Implementation Phases

1. **Phase 7a: Scoped Rhai Bindings** (Week 6)
    - Generate rule-specific modules with only declared types
    - Violations become "undefined function" errors

2. **Phase 7b: Static Analysis** (Week 7)
    - Analyze Rhai AST for undeclared accesses
    - Report violations with line numbers and suggestions

3. **Phase 7c: Conflict Detection** (Week 8)
    - Build rule dependency graph from footprints
    - Detect write-write and read-write conflicts
    - Generate ordering constraints or warn
