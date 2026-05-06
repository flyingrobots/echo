<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# echo-wesley-gen v2 Update

Status: complete.

Resolution: superseded. Do not implement this old JSON-deserializer update as
written. Wesley is moving to a Rust library boundary, and Echo should consume
Wesley-owned Rust APIs or generated Rust artifacts in-process rather than
building a new Echo-owned dependency on a transient `echo-ir/v2` JSON shape.

The durable requirement survives at the host boundary: Echo-owned code must keep
using deterministic canonical bytes for Intents, observations, receipts, and
causal history. Any adapter/import boundary that receives non-canonical data
must canonicalize once before it can affect Echo history.

> **Milestone:** First Light | **Priority:** P1 | **Repo:** Echo

Echo-repo work. The `crates/echo-wesley-gen` crate currently consumes `echo-ir/v1` JSON. Update it to handle the `echo-ir/v2` format that Wesley will emit after QIR Phase C, including new fields for query operations and migration metadata.

## T-2-4-1: Update echo-wesley-gen IR deserializer for v2 format

Status: complete.

**User Story:** As an Echo developer, I want echo-wesley-gen to consume the v2 IR format so that new Wesley features (QIR operations, migration metadata) are available in generated Rust types.

**Requirements:**

- R1: Extend `WesleyIR` struct in `crates/echo-wesley-gen/src/ir.rs` with v2 fields: `queries` (QIR operation catalog), `migrations` (migration plan references), `blake3_schema_hash` (optional, for future BLAKE3 migration).
- R2: Maintain backward compatibility: v1 IR files (missing v2 fields) must still deserialize successfully via serde defaults.
- R3: Code generation must produce Rust types for QIR operation argument structs.
- R4: Add integration test with a v2 IR fixture file.

**Acceptance Criteria:**

- [x] AC1: Superseded. Echo should not add a new implementation dependency on
      `echo-ir/v2` JSON for this path.
- [x] AC2: Superseded. Backward compatibility for old JSON fixtures is not the
      live contract boundary.
- [x] AC3: Superseded by the Wesley Rust library/generated Rust artifact
      direction.
- [x] AC4: Superseded. Future tests should target the Rust boundary and Echo's
      canonical Intent/observation bytes, not this old JSON fixture.

**Definition of Done:**

- [x] Code reviewed locally
- [x] Tests pass locally
- [x] Documentation updated

**Scope:** `crates/echo-wesley-gen/src/ir.rs` (v2 fields), `crates/echo-wesley-gen/src/main.rs` (codegen for queries), test fixtures.
**Out of Scope:** Runtime query execution in Echo. Migration execution. BLAKE3 hash computation (planning only, see T-2-5-1).

**Test Plan:**

- **Goldens:** Snapshot test of generated Rust code from a v2 IR fixture.
- **Failures:** Malformed v2 IR (missing required v2 sub-fields), invalid query operation shapes.
- **Edges:** v2 IR with empty `queries` array, v2 IR with zero types but non-empty queries.
- **Fuzz/Stress:** N/A.

**Blocked By:** none (can be developed against a draft v2 IR spec before Wesley ships QIR)
**Blocking:** none

**Est. Hours:** 5h
**Expected Complexity:** ~200 LoC (IR struct extensions ~50, codegen ~100, tests ~50)

---
