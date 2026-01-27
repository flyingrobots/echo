<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Wesley Integration Phase 2 - Handoff

**Date**: 2026-01-26
**Branch**: ttd-spec
**Status**: IN_PROGRESS
**Previous Work**: Wesley vendoring infrastructure complete

---

## What Was Done (Phase 1)

1. **Vendoring Infrastructure**
    - Created `crates/ttd-manifest/` for Wesley manifest JSONs
    - Created `crates/ttd-protocol-rs/` for generated Rust types
    - Created `packages/ttd-protocol-ts/` for generated TypeScript types
    - Created `docs/wesley/wesley.lock` for provenance tracking

2. **xtask Commands**
    - `cargo xtask wesley sync` - regenerates all outputs from Wesley schema
    - `cargo xtask wesley check` - verifies outputs match Wesley commit (~0.4s)

3. **TTD Protocol Schema**
    - Created `~/git/Wesley/schemas/ttd-protocol.graphql`
    - Defines channels, ops, events, state types for TTD
    - Successfully compiles via Wesley `compile-ttd`

4. **Code Generation**
    - Fixed `echo-ttd-gen` to handle custom scalars (Hash, Timestamp)
    - Fixed identifier sanitization (dots in channel names)
    - Fixed iterator lifetime issues
    - `ttd-protocol-rs` is now a workspace member and compiles clean

5. **Documentation**
    - Added "Wesley Schema-First Development" section to `AGENTS.md`

---

## What Needs To Be Done (Phase 2)

### Task 1: Update Wesley Schema with Missing Types

The schema at `~/git/Wesley/schemas/ttd-protocol.graphql` needs these additions:

```graphql
# Add these types to make the schema complete for ttd-browser

type StepResult @wes_codec(format: "cbor", canonical: true) {
    result: StepResultKind!
    tick: Int!
}

enum StepResultKind {
    NO_OP
    ADVANCED
    SEEKED
    REACHED_FRONTIER
}

type Snapshot @wes_codec(format: "cbor", canonical: true) {
    worldlineId: Hash!
    tick: Int!
}

type ComplianceModel @wes_codec(format: "cbor", canonical: true) {
    isGreen: Boolean!
    violations: [Violation!]!
}
```

After editing, run: `cargo xtask wesley sync`

### Task 2: Wire ttd-browser to Use ttd-protocol-rs

**File**: `crates/ttd-browser/Cargo.toml`

Add dependency:

```toml
[dependencies]
ttd-protocol-rs = { workspace = true }
```

**File**: `crates/ttd-browser/src/lib.rs`

Replace these local types with protocol imports:

| Remove                              | Replace With                       |
| ----------------------------------- | ---------------------------------- |
| `StepResultJs` (lines 882-886)      | `ttd_protocol_rs::StepResult`      |
| `TruthFrameJs` (lines 888-895)      | `ttd_protocol_rs::TruthFrame`      |
| `SnapshotJs` (lines 897-901)        | `ttd_protocol_rs::Snapshot`        |
| `ComplianceModelJs` (lines 903-907) | `ttd_protocol_rs::ComplianceModel` |
| `ViolationJs` (lines 909-914)       | `ttd_protocol_rs::Violation`       |
| `ObligationJs` (lines 917-921)      | `ttd_protocol_rs::Obligation`      |
| `ObligationStateJs` (lines 923-927) | `ttd_protocol_rs::ObligationState` |

**Critical fix** at line 731:

```rust
// BEFORE:
let schema_hash = [0u8; 32];

// AFTER:
let schema_hash = hex_to_bytes(ttd_protocol_rs::SCHEMA_SHA256);
```

**PlaybackMode consistency** (lines 327-331):
Ensure the string values match the protocol enum. Either:

- Update protocol enum to use PascalCase (`Paused` not `PAUSED`), or
- Update ttd-browser to use SCREAMING_CASE

### Task 3: Add wesley check to Pre-Commit Hook

**File**: `.githooks/pre-commit` (or wherever hooks live)

Add:

```bash
# Verify Wesley-generated outputs are up to date
echo "Checking Wesley artifacts..."
cargo xtask wesley check || {
    echo "ERROR: Wesley artifacts are stale. Run 'cargo xtask wesley sync' to update."
    exit 1
}
```

The check runs in ~0.4s so it's fast enough for pre-commit.

### Task 4: Add wesley check to CI/CD

**File**: `.github/workflows/ci.yml` (or equivalent)

Add a job or step:

```yaml
- name: Verify Wesley artifacts
  run: cargo xtask wesley check
```

This should run early in the pipeline (before build/test) since stale artifacts will cause confusing failures downstream.

### Task 5: Create package.json for ttd-protocol-ts

**File**: `packages/ttd-protocol-ts/package.json`

```json
{
    "name": "@echo/ttd-protocol-ts",
    "version": "0.1.0",
    "type": "module",
    "main": "index.ts",
    "types": "index.ts",
    "description": "Generated TTD protocol types from Wesley schema (DO NOT EDIT)",
    "license": "Apache-2.0",
    "private": true,
    "dependencies": {
        "zod": "^3.22.0"
    }
}
```

### Task 6: Wire ttd-app to Use ttd-protocol-ts

**File**: `apps/ttd-app/package.json`

Add workspace dependency:

```json
"dependencies": {
  "@echo/ttd-protocol-ts": "workspace:*"
}
```

**File**: `apps/ttd-app/src/types/ttd.ts`

Replace placeholder types with imports from `@echo/ttd-protocol-ts`. Keep only:

- Utility functions (`hexToBytes`, `bytesToHex`, `truncateHash`)
- Any truly app-specific types not in protocol

---

## Verification Checklist

After completing all tasks:

- [ ] `cargo xtask wesley check` passes
- [ ] `cargo check -p ttd-browser` compiles
- [ ] `cargo check -p ttd-protocol-rs` compiles
- [ ] `ttd-browser` uses `SCHEMA_SHA256` from `ttd-protocol-rs`
- [ ] Pre-commit hook runs `wesley check`
- [ ] CI runs `wesley check`
- [ ] `ttd-app` imports from `@echo/ttd-protocol-ts`
- [ ] No duplicate type definitions remain

---

## Key Principle

**The Wesley schema is the source of truth.** Never edit:

- `crates/ttd-protocol-rs/lib.rs`
- `packages/ttd-protocol-ts/*.ts`
- `crates/ttd-manifest/*.json`

If types need to change, edit `~/git/Wesley/schemas/ttd-protocol.graphql` and run `cargo xtask wesley sync`.

---

## Related Files

- `AGENTS.md` - Updated with Wesley schema-first section
- `docs/wesley/wesley.lock` - Current provenance
- `crates/echo-ttd-gen/` - Rust code generator (consumes ttd-ir.json)
- `xtask/src/main.rs` - wesley sync/check implementation
