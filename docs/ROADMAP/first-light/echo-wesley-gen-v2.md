<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# echo-wesley-gen v2 Update

> **Milestone:** [First Light](README.md) | **Priority:** P1 | **Repo:** Echo

Echo-repo work. The `crates/echo-wesley-gen` crate currently consumes `echo-ir/v1` JSON. Update it to handle the `echo-ir/v2` format that Wesley will emit after QIR Phase C, including new fields for query operations and migration metadata.

## T-2-4-1: Update echo-wesley-gen IR deserializer for v2 format

**User Story:** As an Echo developer, I want echo-wesley-gen to consume the v2 IR format so that new Wesley features (QIR operations, migration metadata) are available in generated Rust types.

**Requirements:**

- R1: Extend `WesleyIR` struct in `crates/echo-wesley-gen/src/ir.rs` with v2 fields: `queries` (QIR operation catalog), `migrations` (migration plan references), `blake3_schema_hash` (optional, for future BLAKE3 migration).
- R2: Maintain backward compatibility: v1 IR files (missing v2 fields) must still deserialize successfully via serde defaults.
- R3: Code generation must produce Rust types for QIR operation argument structs.
- R4: Add integration test with a v2 IR fixture file.

**Acceptance Criteria:**

- [ ] AC1: A v2 IR JSON with `queries` field deserializes into `WesleyIR` with populated query catalog.
- [ ] AC2: A v1 IR JSON (no `queries` field) still deserializes without error (backward compat).
- [ ] AC3: Generated Rust code for a query operation compiles and includes argument types.
- [ ] AC4: Integration test in `crates/echo-wesley-gen/tests/generation.rs` covers v2 IR.

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

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
