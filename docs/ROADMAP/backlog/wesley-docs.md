<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Wesley Docs

> **Milestone:** [Backlog](README.md) | **Priority:** Unscheduled

Wesley-repo documentation consolidation. Recorded here for cross-project tracking.

## T-10-10-1: Information Architecture Consolidation

**User Story:** As a Wesley contributor, I want a consolidated documentation structure so that information is discoverable and not duplicated across scattered files.

**Requirements:**

- R1: Audit existing Wesley documentation (README, inline docs, spec files, wiki)
- R2: Propose a unified information architecture (IA) with clear categories
- R3: Migrate/reorganize existing docs into the new IA
- R4: Add a table of contents or navigation index

**Acceptance Criteria:**

- [ ] AC1: IA proposal is reviewed and approved
- [ ] AC2: All existing docs are migrated to the new structure
- [ ] AC3: No orphaned or duplicate documents remain
- [ ] AC4: Navigation index exists at the docs root

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Wesley repo documentation reorganization.
**Out of Scope:** New content creation (tutorials, migration guides), API reference generation.

**Test Plan:**

- **Goldens:** n/a (documentation)
- **Failures:** n/a
- **Edges:** n/a
- **Fuzz/Stress:** n/a

**Blocked By:** none
**Blocking:** T-10-10-2

**Est. Hours:** 4h
**Expected Complexity:** ~0 LoC (reorganization)

---

## T-10-10-2: Tutorial Series + API Reference

**User Story:** As a new Wesley user, I want tutorials and API reference so that I can learn the tool without reading source code.

**Requirements:**

- R1: Write a "Getting Started" tutorial (install, first schema, generate, use)
- R2: Write an "Advanced Patterns" tutorial (unions, directives, custom validators)
- R3: Generate API reference from JSDoc/TSDoc annotations
- R4: Include migration guides for breaking changes between Wesley versions

**Acceptance Criteria:**

- [ ] AC1: "Getting Started" tutorial is complete and tested (reader can follow it end-to-end)
- [ ] AC2: "Advanced Patterns" tutorial covers at least three advanced features
- [ ] AC3: API reference is generated and published alongside docs
- [ ] AC4: At least one migration guide exists (for the most recent breaking change)

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Tutorials, API reference generation, one migration guide.
**Out of Scope:** Video content, interactive playground, translations.

**Test Plan:**

- **Goldens:** n/a (documentation)
- **Failures:** n/a
- **Edges:** n/a
- **Fuzz/Stress:** n/a

**Blocked By:** T-10-10-1
**Blocking:** none

**Est. Hours:** 6h
**Expected Complexity:** ~0 LoC (documentation)
