<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Docs Audit — Purge / Merge / Splurge

This is a lightweight “docs hygiene” memo: which documents look stale, overlapping, or underspecified, and what we should do about them.

**Initial audit:** 2026-01-02

---

## Rubric

### Purge

Candidate for removal or archival when the doc is:

- clearly wrong / misleading (claims features that don’t exist),
- unreferenced and not historically valuable,
- replaced by a newer canonical doc (and the old one causes confusion).

_Note:_ prefer “archive + redirect note” over hard-deleting, unless we’re sure the content is junk.

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
- Collision tour docs hygiene:
    - moved the tour source to `docs/public/collision-dpo-tour.html` so VitePress emits `/collision-dpo-tour.html`,
    - added a `docs/spec-geom-collision.md` stub so the tour’s “Spec” link is non-broken.

De-risked (clarified “what is canonical”):

- `docs/spec-deterministic-math.md` — marked as a legacy TS-oriented Phase 0 design sketch; points readers to:
    - `docs/SPEC_DETERMINISTIC_MATH.md` (normative policy)
    - `docs/math-validation-plan.md` (how we test)

---

## What We Did (2026-03-07)

### Archived

Moved 6 superseded documents to `docs/archive/` with redirect stubs:

- `spec-deterministic-math.md` -- legacy Phase 0 TS-oriented draft
- `spec-geom-collision.md` -- stub with no normative content
- `notes/scheduler-radix-optimization.md` -- superseded by `-2` version
- `notes/xtask-wizard.md` -- concept note, never implemented
- `plans/cross-warp-parallelism.md` -- feature already implemented
- `plans/BOAW-tech-debt.md` -- content already in `adr/TECH-DEBT-BOAW.md`

### Consolidated

- Replaced "Related Docs" sections in `SPEC_DETERMINISTIC_MATH.md` and
  `DETERMINISTIC_MATH.md` with structured "Docs Map" tables linking
  all 5 documents in the deterministic math cluster.
- Updated `scheduler.md` Quick Map with status-labeled table.
- Added "(not yet implemented)" to `spec-scheduler.md` title.

### Fixed

- `ROADMAP/backlog/editor-hot-reload.md`: `docs/specs/` -> `docs/spec/`
- `ROADMAP/backlog/plugin-abi.md`: `docs/specs/` -> `docs/spec/`
- `meta/docs-index.md`: `memorial.md` -> `memorials/2026-01-18-phase4-rubicon.md`
- `ROADMAP/ISSUE-INDEX.md`: 6 references to `streams-inspector-frame.md`
  -> `streams-inspector.md` (file never had the `-frame` suffix)
- `architecture-outline.md`: `docs/spec/SPEC-0004...` -> `spec/SPEC-0004...`
  (stale `docs/` prefix), nonexistent `echo-scene-port/README.md` link
- `archive/plans/BOAW-tech-debt.md`: `../adr/` -> `../../adr/` (depth
  changed by archival)
- `archive/notes/scheduler-radix-optimization.md`: image paths updated
  to point back to `../../notes/` after archival
- `meta/docs-index.md`: `public/assets/...` -> `../public/assets/...`

### Added

- `docs/guide/configuration-reference.md` -- engine parameters, protocol
  constants, environment variables, channel policies
- `docs/guide/cargo-features.md` -- all Cargo feature flags across the
  workspace (11 crates, 19 unique flags)

### Added (tooling)

- `cargo xtask lint-dead-refs` -- scans `docs/` for broken markdown
  cross-references. Use `--all` to also check image/HTML references.
- `cargo xtask markdown-fix` -- auto-fixes SPDX headers, runs prettier,
  and applies markdownlint `--fix` across `docs/`. Flags: `--no-prettier`,
  `--no-lint`.
- `cargo xtask docs-lint` -- combined pipeline: `markdown-fix` then
  `lint-dead-refs`. One command for full docs hygiene.

### Formatted

- Ran `cargo xtask markdown-fix` across all 205 docs files: prettier
  formatting, SPDX header normalization, markdownlint auto-fixes applied.
  66 unfixable lint warnings remain (missing code fence languages in
  `THEORY.md`, structural issues in `warp-math-claims.md`).

### Updated

- `README.md` -- added determinism claims link, reference docs section
- `meta/docs-index.md` -- new entries, archive note, updated descriptions
  for redirected docs
- `CHANGELOG.md` -- docs polish entries

---

## Candidates (Next)

### Merge candidates

- ~~Deterministic math docs:~~ **Done (2026-03-07).** Both files now have
  structured “Docs Map” tables cross-linking the full math cluster.

- Scheduler documentation:
    - Multiple reserve/scheduler docs exist (`docs/scheduler-benchmarks.md`, `docs/scheduler-reserve-*.md`, `docs/spec-scheduler.md`).
    - Action: decide which is canonical for “how it works” vs “how we benchmark it”, and add a single landing doc (or update `docs/spec-scheduler.md`) that links the rest.

### Splurge candidates

- `docs/meta/docs-index.md`:
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
- Should VitePress navigation be driven by `docs/meta/docs-index.md` (as the canonical index), rather than having multiple “landing pages”?
