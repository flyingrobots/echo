<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Wesley Migration Planning Phase B

> **Milestone:** [First Light](README.md) | **Priority:** P1 | **Repo:** Wesley

Wesley-repo work. Extend Wesley's migration system to handle schema evolution with backfill scripts, switch-over plans, and contract-based validation.

## T-2-2-1: Backfill script generation for schema migrations

**User Story:** As a Wesley user, I want automatic backfill script generation when a schema migration adds or transforms fields so that I can safely evolve my schema without data loss.

**Requirements:**

- R1: Detect additive field changes (new required fields, type widening) between schema versions.
- R2: Generate a backfill script (TypeScript) that reads existing rows and populates new fields with default or computed values.
- R3: Backfill scripts must be idempotent (safe to re-run).
- R4: Scripts reference the Wesley type catalog for type-safe field access.

**Acceptance Criteria:**

- [ ] AC1: Adding a required `email` field to `User` produces a backfill script that sets a default value.
- [ ] AC2: Running the backfill script twice produces the same result (idempotency test).
- [ ] AC3: Type-narrowing changes (e.g., `String` to `Int`) produce an error, not a backfill.
- [ ] AC4: Generated scripts pass TypeScript type checking.

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Repo: Wesley. `migration/backfill.ts` module. Integration with schema diff engine.
**Out of Scope:** Actual database execution runtime. Switch-over orchestration (T-2-2-2). Rollback scripts.

**Test Plan:**

- **Goldens:** Snapshot tests for generated backfill scripts against 3+ migration scenarios.
- **Failures:** Incompatible type changes, removed required fields, circular dependencies.
- **Edges:** Empty migration (no changes), migration with only removals, migration with rename detection.
- **Fuzz/Stress:** N/A (code generation, not runtime).

**Blocked By:** none
**Blocking:** T-2-2-2

**Est. Hours:** 5h
**Expected Complexity:** ~350 LoC

---

## T-2-2-2: Switch-over plan and contract validation

**User Story:** As a Wesley user, I want a switch-over plan that coordinates the migration sequence (backfill, schema swap, validation) so that I can execute migrations with confidence.

**Requirements:**

- R1: Generate a migration plan document (JSON) listing steps: pre-check, backfill, schema swap, post-check.
- R2: Each step references the relevant generated artifact (backfill script, new schema, validation contract).
- R3: Validation contracts assert post-migration invariants (field presence, type correctness, referential integrity).
- R4: Plan document includes rollback hints (which steps are reversible).

**Acceptance Criteria:**

- [ ] AC1: A migration adding a field produces a 4-step plan (pre-check, backfill, swap, post-check).
- [ ] AC2: Validation contract catches a missing required field in post-check.
- [ ] AC3: Plan JSON is human-readable and machine-parseable.
- [ ] AC4: Rollback hints correctly identify irreversible steps (e.g., column drop).

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Repo: Wesley. `migration/planner.ts` and `migration/contract.ts` modules.
**Out of Scope:** Actual migration execution runtime. Database-specific DDL generation. Backfill script generation (T-2-2-1).

**Test Plan:**

- **Goldens:** Snapshot tests for plan JSON across 3+ migration scenarios.
- **Failures:** Circular dependency in migration steps, incompatible schema versions.
- **Edges:** No-op migration (identical schemas), destructive-only migration (all removals).
- **Fuzz/Stress:** N/A.

**Blocked By:** T-2-2-1
**Blocking:** none

**Est. Hours:** 5h
**Expected Complexity:** ~300 LoC

---
