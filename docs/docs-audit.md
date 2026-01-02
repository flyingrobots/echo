<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Docs Audit — Purge / Merge / Splurge

This is a lightweight “docs hygiene” memo: which documents look stale, overlapping, or underspecified, and what we should do about them.

**Audit date:** 2026-01-02

---

## Rubric

### Purge
Candidate for removal or archival when the doc is:
- clearly wrong / misleading (claims features that don’t exist),
- unreferenced and not historically valuable,
- replaced by a newer canonical doc (and the old one causes confusion).

*Note:* prefer “archive + redirect note” over hard-deleting, unless we’re sure the content is junk.

### Merge
Candidate for consolidation when two docs:
- describe the same invariant or workflow,
- are both expected to stay accurate,
- and drift risk is higher than the value of having multiple entry points.

### Splurge (Enhance)
Candidate for investment when the doc is:
- canonical (people should read it),
- frequently referenced,
- or is a high-leverage “on-ramp” document (index, primer, workflow).

---

## What We Did In This Pass

Splurged:
- `docs/math-validation-plan.md` — updated to match the current `warp-core` deterministic math implementation and CI lanes (float lane + `det_fixed` lane), and to list concrete tests/commands instead of JS/browser plans.
- `docs/index.md` — updated the docs landing page to point at real, current documents (it previously linked to a missing collision spec).

De-risked (clarified “what is canonical”):
- `docs/spec-deterministic-math.md` — marked as a legacy TS-oriented Phase 0 design sketch; points readers to:
  - `docs/SPEC_DETERMINISTIC_MATH.md` (normative policy)
  - `docs/math-validation-plan.md` (how we test)

---

## Candidates (Next)

### Merge candidates
- Deterministic math docs:
  - `docs/DETERMINISTIC_MATH.md` (hazards) and `docs/SPEC_DETERMINISTIC_MATH.md` (policy) are both useful, but they should cross-link explicitly and avoid duplicating the same “why” text.
  - Action: add a short “Docs map” section to both files so readers know which is normative vs explanatory.

- Scheduler documentation:
  - Multiple reserve/scheduler docs exist (`docs/scheduler-benchmarks.md`, `docs/scheduler-reserve-*.md`, `docs/spec-scheduler.md`).
  - Action: decide which is canonical for “how it works” vs “how we benchmark it”, and add a single landing doc (or update `docs/spec-scheduler.md`) that links the rest.

### Splurge candidates
- `docs/docs-index.md`:
  - It’s already a great index, but could include a short “If you’re changing X, read Y” map (e.g., determinism policy, docs guard, PR policy).
  - Action: add a “Common contributor paths” section.

- `docs/code-map.md`:
  - High leverage for onboarding; should stay aligned with canonical specs.
  - Action: keep concept→spec links accurate as we demote legacy docs.

### Purge / archive candidates (defer until verified)
- “One-off” review burn-down notes under `docs/notes/`:
  - These are historically useful, but may not belong in the default browsing path forever.
  - Action: consider a `docs/notes/archive/` folder and move docs that are purely PR-specific retrospectives once they’ve served their purpose.

---

## Open Questions

- Do we want a formal “doc tier” tag?
  - Example: **Spec (normative)** vs **Guide (how-to)** vs **Notes (historical)** vs **ADR (decisions)**.
- Should VitePress navigation be driven by `docs/docs-index.md` (as the canonical index), rather than having multiple “landing pages”?
