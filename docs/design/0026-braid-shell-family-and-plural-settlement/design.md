<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# 0026 — Braid Shell Family and Plural Settlement

_Make lawful plurality a retainable, replayable outcome: add the `Plural`
settlement arm and emit braid-scale results as in-graph holographic shells
(θ_braid) of the same family as tick receipts — so braiding becomes a WARP
optic lowering instead of a service-layer function call._

Legend: `PLATFORM`

Status: **proposed — awaiting packet + test-plan review**

## Doctrine

AIΩN Paper VII (DOI 10.5281/zenodo.19751149):

- **Prop 3.5** — WARP is closed over its own witness-bearing outputs: tick
  receipts, braid shells, and import shells are **one shell family** of
  retained holographic boundaries living inside the causal graph. That
  containment is what makes replay cheap at every scale.
- **§4.2** — "irreducible plurality need not be treated as merge failure":
  `Plural` is a first-class arm of the outcome algebra
  `Derived ⊔ Plural ⊔ Conflict ⊔ Obstruction`, not a staging posture.

Tracking issues: flyingrobots/echo#537 (shell-family doctrine, requirements
1–2 of 5), with #538 (three-tier posture) as a field-level rider. Connective
doctrine for #470 / #476 / #483 — whichever braid/settlement work lands
first establishes the shell family; this packet is that work.

## Current state (verified @465cf61e)

- `SettlementDecision` (`crates/warp-core/src/settlement.rs#146`) has no
  plural arm; `ImportCandidate` lowers to `AdmissionOutcomeKind::Derived`
  (`settlement.rs#158`) and conflicts carry `ConflictReason`
  (`settlement.rs#36`). Settlement compares **one strand** against base.
- Braid identities exist without substance: `OpticFocus::Braid`
  (`crates/warp-core/src/optic.rs#161`), `EchoCoordinate::Braid`
  (`optic.rs#297`), `SupportPin` geometry implemented and
  invariant-validated — but no reducer materializes a braid and nothing
  emits a braid-level shell.
- `BoundaryTransitionRecord` (`crates/warp-core/src/provenance_store.rs#626`)
  is the existing retained-shell mechanics at strand boundaries — the
  family to extend, **not** a pattern to duplicate.
- Plurality exists only admission-side: `PluralIntent`,
  `PluralityRequiresExplicitPolicy` (optic.rs admission path). Below
  admission, plurality is destroyed (admit-one/block-one per tick).

## Hill

A settlement comparison over two or more strands sharing a fork basis can
end in a **retained plural outcome**: a θ_braid shell, resident in the
provenance store, carrying the comparison basis, member strand refs, the
outcome arm (`Collapsed`/`Plural`/`Conflict`), and its witness — such that
**the braid outcome can be replayed from the shell alone**, without
rematerializing member strands. That replay test is the definition of done.

## Campaign map (this packet = E1)

| Slice  | Scope                                                                                                           | Status                                                               |
| :----- | :-------------------------------------------------------------------------------------------------------------- | :------------------------------------------------------------------- |
| E0     | Tier posture field (`scratch \| author-only \| shared`) on strand creation (echo#538)                           | optional preface; if not first, E1 carries the field on θ_braid only |
| **E1** | **`SettlementDecision::Plural` + θ_braid shell family + replay-from-shell (this packet)**                       | proposed                                                             |
| E2     | Holographic strand origins — fork via checkpoint-pinned basis ref, empty entry vector (echo#537 comment design) | next                                                                 |
| E3     | Braid reducer/weave over N strands + collapse policies                                                          | after E2 (needs cheap strands)                                       |

## Acceptance criteria

1. `SettlementDecision` gains a `Plural` arm carrying the surviving
   alternatives as refs (not clones), lowering to
   `AdmissionOutcomeKind::Plural`; existing `Derived`/`Conflict` paths are
   byte-identical for single-strand settlement (regression-pinned).
2. A `BraidShell` (θ_braid) record exists in the
   `BoundaryTransitionRecord` family: comparison basis (fork basis ref +
   frontier facts), member strand refs with their `SupportPin` posture,
   outcome arm, witness digest. It is written into the provenance store —
   in-graph, content-addressed, retained.
3. **Replay-from-shell acceptance test**: given only the θ_braid shell and
   the provenance store, replay reproduces the settlement outcome
   (same arm, same member verdicts, same digests) without loading member
   strand histories.
4. Plurality is never silently collapsed: a plural result requires an
   explicit, witnessed collapse act (policy-named) to become `Derived`;
   absent policy, `Plural` is the retained truth.
5. θ_braid carries a revelation-posture field (E0 rider:
   `scratch | author-only | shared`, default `author-only`) so the shell
   family never hardens around implicit shared visibility (echo#538).
6. No new record family parallel to `BoundaryTransitionRecord`; extension
   only.

## Test plan (for review before RED)

1. **Plural arm shape** — settlement over two strands with disjoint lawful
   rewrites of the same footprint region under an explicit plural policy
   yields `SettlementDecision::Plural` with both members referenced;
   ABI lowering maps to `AdmissionOutcomeKind::Plural`.
2. **Single-strand regression** — existing settle paths produce identical
   outcomes and digests before/after (golden fixtures from current tests).
3. **Shell emission** — every settlement that ends `Collapsed`/`Plural`/
   `Conflict` at braid scope writes exactly one θ_braid into provenance;
   shell digest is deterministic across runs.
4. **Replay-from-shell** — drop/forget member strand materializations;
   replay from θ_braid reproduces arm + member verdicts + witness digest.
   (The headline test; failure here fails the hill.)
5. **No silent collapse** — plural result + no collapse policy stays
   `Plural` across ticks; collapse without named policy is an
   `Obstruction` with witness.
6. **Posture default** — θ_braid created from debugger/counterfactual
   strands defaults `author-only`; promotion to `shared` is an explicit
   act that re-witnesses.
7. **Conflict still conflicts** — overlapping rewrites without plural
   policy keep today's `Conflict` + `ConflictReason` behavior exactly.

## Playback questions

1. Can a plural settlement outcome be retained, queried, and replayed from
   its shell without rematerializing member strands?
2. Is the θ_braid shell demonstrably the same record family as the
   existing boundary-transition mechanics (one family, per Prop 3.5)?
3. Does any path silently collapse plurality?

## Non-goals

- No braid reducer/weave over N>2 strands or collapse-policy library (E3).
- No suffix-transport shell θ_rep / import idempotence (later slice).
- No fork-mechanics change (E2 owns holographic origins).
- No session implementation (design 0025 owns that).
- No public ABI breakage beyond the additive `Plural` arm.

## Open questions (for James at packet review)

1. Should E0 (tier posture on strand creation) land first as its own tiny
   slice, or is the θ_braid-only posture field (criterion 5) the right
   E1-scoped compromise?
2. `Plural` member refs: strand ids + basis digests, or full
   `SupportPin` snapshots? (Refs keep the shell holographic; pins make it
   self-contained. Recommendation: refs + pin digests.)
3. Does `AdmissionOutcomeKind` already reserve a `Plural` discriminant in
   the ABI, or does this slice mint it (ABI version note required)?
