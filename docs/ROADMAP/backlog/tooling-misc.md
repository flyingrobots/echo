<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Tooling & Misc

> **Milestone:** [Backlog](README.md) | **Priority:** Unscheduled

Housekeeping tasks: documentation, logging, naming consistency, and debugger UX design.

**Issues:** #79, #207, #239

## T-10-8-1: Docs / Logging Improvements (#79)

**User Story:** As a contributor, I want improved documentation and structured logging so that onboarding is faster and runtime behavior is observable.

**Requirements:**

- R1: Audit existing doc comments for completeness — add missing module-level docs
- R2: Standardize log levels across crates (`trace` for hot path, `debug` for internals, `info` for lifecycle, `warn`/`error` for problems)
- R3: Add structured fields to log events (e.g., `tick=`, `entity_id=`, `component=`)
- R4: Document the logging configuration in the contributor guide

**Acceptance Criteria:**

- [ ] AC1: Every public module has a `//!` doc comment
- [ ] AC2: Log events use consistent levels per the standard
- [ ] AC3: At least 10 key log sites use structured fields
- [ ] AC4: Contributor guide includes a "Logging" section

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Documentation, logging standardization, contributor guide update.
**Out of Scope:** Log aggregation infrastructure, metrics, tracing spans.

**Test Plan:**

- **Goldens:** n/a
- **Failures:** n/a
- **Edges:** n/a
- **Fuzz/Stress:** n/a

**Blocked By:** none
**Blocking:** none

**Est. Hours:** 4h
**Expected Complexity:** ~200 LoC (doc comments + logging changes)

---

## T-10-8-2: Naming Consistency Audit (#207)

**User Story:** As a user, I want consistent naming across Echo, WARP, Wesley, and Engram so that there is no confusion about product names in code, docs, and CLI output.

**Requirements:**

- R1: Run a noisy-line test (grep for all naming variants) across all repos
- R2: Catalog every instance of inconsistent naming (old names, abbreviations, typos)
- R3: Produce a migration plan with specific find-and-replace operations
- R4: Apply fixes to the echo repo (other repos tracked separately)

**Acceptance Criteria:**

- [ ] AC1: Audit report listing all inconsistencies is produced
- [ ] AC2: All inconsistencies in the echo repo are fixed
- [ ] AC3: CI includes a grep-based lint to catch future naming regressions
- [ ] AC4: Migration plan for other repos is filed as issues in those repos

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Echo repo naming fixes. Audit report for other repos.
**Out of Scope:** Fixing naming in non-echo repos (tracked separately).

**Test Plan:**

- **Goldens:** n/a
- **Failures:** CI lint fails on intentional introduction of old name
- **Edges:** Names in string literals, names in comments, names in URLs
- **Fuzz/Stress:** n/a

**Blocked By:** none
**Blocking:** none

**Est. Hours:** 3h
**Expected Complexity:** ~50 LoC (lint script + renames)

---

## T-10-8-3: Reliving Debugger UX Design (#239)

**User Story:** As a simulation developer, I want a UX design for the reliving debugger so that the Constraint Lens and Provenance Heatmap features are well-specified before implementation begins.

**Requirements:**

- R1: Design the Constraint Lens view (visualize which constraints are active per-entity per-tick)
- R2: Design the Provenance Heatmap view (color-code state by how recently/frequently it was written)
- R3: Specify the data model backing each view (what queries are needed)
- R4: Produce wireframes or mockups (low-fidelity is fine)
- R5: Identify which runtime hooks/APIs are needed to feed data into the views

**Acceptance Criteria:**

- [ ] AC1: UX design document exists at `docs/designs/RELIVING-DEBUGGER-UX.md`
- [ ] AC2: Both Constraint Lens and Provenance Heatmap are specified
- [ ] AC3: Data model and required runtime APIs are listed
- [ ] AC4: At least two wireframes (one per view)

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** UX design, wireframes, data model specification.
**Out of Scope:** Implementation, frontend framework choice, performance optimization.

**Test Plan:**

- **Goldens:** n/a (design document)
- **Failures:** n/a
- **Edges:** n/a
- **Fuzz/Stress:** n/a

**Blocked By:** none
**Blocking:** none

**Est. Hours:** 4h
**Expected Complexity:** ~300 lines (markdown + diagrams)
