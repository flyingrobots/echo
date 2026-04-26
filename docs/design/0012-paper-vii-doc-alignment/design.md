<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# 0012 — Paper VII doc alignment and consolidation

_Align Echo's documentation with Paper VII's optic/causal-history framing,
eliminate redundant architecture docs, and produce a single vocabulary
authority._

Legend: `DOCS`

Depends on:

- nothing (this is a docs-only cycle)

## Why this cycle exists

Paper VII (WARP: Optics, Holograms, and Worldlines over Shared Causal
History) stabilizes the WARP noun map and reframes the architecture around
witnessed causal history, optics, and holographic shells. Echo's README
already speaks this language. The rest of the docs do not.

Echo currently has six overlapping documents doing variations of the same
job (bridging WARP theory to Echo's implementation):

| Doc                                     | Lines | Problem                                                                                                    |
| --------------------------------------- | ----- | ---------------------------------------------------------------------------------------------------------- |
| `theory/THEORY.md`                      | ~1060 | Paper I-IV paraphrase. Teaches through a DPO/graph-rewriting lens that is no longer the conceptual center. |
| `architecture/WARP_DRIFT.md`            | ~219  | Drift memo. Good voice, but orphaned as a standalone correction note.                                      |
| `architecture/continuum-foundations.md` | ~408  | Multi-repo bridge. Uses older optic framing (projection/footprint/rewrite/witness/reintegration).          |
| `architecture/outline.md`               | ~320  | Scored 2/5 in audit. Mostly aspirational ECS material that doesn't exist.                                  |
| `architecture/TERMS_...WORMHOLES.md`    | ~179  | State-plane vocabulary only. Clean but incomplete for the optic layer.                                     |
| `guide/course/glossary.md`              | ~42   | Teaching glossary. Small, good, but disconnected from TERMS doc.                                           |

The README was rewritten to speak Paper VII. Now the docs behind it must
follow, or the README becomes a false storefront.

This is also the right moment to consolidate. Fewer docs means fewer places
for vocabulary drift to hide.

## Human users / jobs / hills

### Primary human users

- Contributors reading Echo docs to understand the architecture.
- James (author) maintaining conceptual alignment across Echo and the paper
  series.

### Human jobs

1. Read one glossary and know every WARP term Echo uses.
2. Read one bridge doc and understand how each AIΩN paper maps to Echo's
   implementation and where Echo currently drifts.
3. Stop encountering stale graph-rewriting-first framing in non-spec docs.

### Human hill

A contributor can understand Echo's relationship to the WARP papers without
reading six overlapping documents.

## Agent users / jobs / hills

### Primary agent users

- Claude (agent) working in the Echo repo.

### Agent jobs

1. Look up a WARP term in one canonical glossary.
2. Determine whether Echo implements a Paper VII concept by checking one
   bridge doc.
3. Use the correct framing (causal history, not graph-first) when writing
   new docs or design cycles.

### Agent hill

An agent can find the canonical definition of any WARP term and
programmatically determine whether Echo's docs use it correctly by checking
two files.

## Human playback

1. The human opens `docs/architecture/glossary.md`.
2. Every Paper VII glossary term (Appendix B) appears, organized into
   progressive tiers.
3. The human opens `docs/architecture/warp-bridge.md`.
4. Each AIΩN paper (I through VII) has a compact mapping to Echo's
   implementation plus an honest drift note.
5. The human searches for `theory/THEORY.md` — it no longer exists.
6. The human searches for `architecture/outline.md` — it no longer exists.
7. `pnpm docs:build` passes.

## Agent playback

1. The agent runs `ls docs/architecture/glossary.md docs/architecture/warp-bridge.md`.
2. Both files exist.
3. The agent runs `ls docs/theory/THEORY.md docs/architecture/outline.md` — both are gone.
4. The agent greps for "DPO" in non-spec docs — zero hits outside
   `spec/` and `design/` directories.
5. `pnpm docs:build` passes.

## Implementation outline

### Phase 1 — Concordance (research, no edits)

1. Grep every Paper VII glossary term (Appendix B) across all Echo docs.
2. Record which docs use each term, whether usage matches Paper VII, and
   whether the doc uses older/conflicting framing.
3. Classify each term: Tier A (rename/refine in Echo), Tier B (introduce
   to Echo), Tier C (paper-only, not needed in implementation docs).

### Phase 2 — Consolidate and reframe

1. **Delete** `docs/architecture/outline.md` (2/5, mostly aspirational).
2. **Create** `docs/architecture/warp-bridge.md`:
    - One section per AIΩN paper (I through VII).
    - Each section: what the paper establishes, how Echo implements it, where
      Echo currently drifts.
    - Absorbs the useful content from THEORY.md, WARP_DRIFT.md, and
      continuum-foundations.md.
    - Uses Paper VII framing: causal history, optics, outcome algebra,
      shells, commitment/folding/revelation.
3. **Create** `docs/architecture/glossary.md`:
    - Three tiers: Friendly, Implementation, WARP/Optic.
    - Absorbs TERMS doc (state-plane vocabulary) and course glossary content.
    - Adds all Paper VII Appendix B terms at appropriate tiers.
4. **Delete** `docs/theory/THEORY.md`.
5. **Delete** `docs/architecture/WARP_DRIFT.md`.
6. **Delete** `docs/architecture/continuum-foundations.md`.
7. **Delete** `docs/architecture/TERMS_WARP_STATE_INSTANCES_PORTALS_WORMHOLES.md`.
8. **Redirect** `docs/guide/course/glossary.md` to link to the unified
   glossary.

### Phase 3 — Sweep remaining docs

1. Sweep non-spec, non-design docs for graph-rewriting-first framing that
   should be reframed around causal history (e.g., invariants, guides,
   BEARING). Light-touch: change the framing, not the technical content.
2. Update `docs/index.md` and `README.md` links for moved/deleted docs.
3. Run `pnpm docs:build` and fix any dead links.

### Phase 4 — Verify

1. Run full playback questions.
2. Commit.

## Tests to write first

- `pnpm docs:build` must pass after all changes (existing gate).
- Agent grep: "DPO" appears only in `spec/` and `design/` directories.
- Agent grep: "graph-rewrite engine" or "graph rewrite engine" appears
  zero times outside `design/` directories.
- `docs/architecture/glossary.md` contains every Paper VII Appendix B
  heading term.
- `docs/architecture/warp-bridge.md` contains sections for Papers I
  through VII.
- Deleted files are gone: `theory/THEORY.md`, `architecture/outline.md`,
  `architecture/WARP_DRIFT.md`, `architecture/continuum-foundations.md`,
  `architecture/TERMS_WARP_STATE_INSTANCES_PORTALS_WORMHOLES.md`.

## Risks / unknowns

- **Scope creep into specs/designs**: The sweep phase must stay light-touch.
  Specs describe implementation details where graph-rewriting language is
  correct. Design docs are historical records. Neither should be rewritten
  for cosmetic alignment.
- **THEORY.md has institutional value**: It's the only place that explains
  Papers I-IV at length. The bridge doc must cover the same ground more
  concisely without losing the "Echo does this differently" mapping.
- **Dead links**: Deleting 5 files will create dead links. The build gate
  catches these, but fixing them is a known cost.

## Postures

- **Accessibility:** Not applicable (docs content, no UI).
- **Localization:** Not applicable.
- **Agent inspectability:** Improved. Two canonical files instead of six
  overlapping ones.

## Non-goals

- Rewriting spec docs (`docs/spec/*`) beyond fixing dead links.
- Rewriting design docs (`docs/design/*`) beyond fixing dead links.
- Implementing glossary enforcement tooling (that's the existing cool-idea
  backlog item).
- Updating code comments or Rust doc comments for vocabulary alignment.
- Adding Paper VII formal notation (Ψ, Ω, χ, etc.) to implementation docs.
  The glossary should use plain English names; the formal symbols stay in
  the paper.
