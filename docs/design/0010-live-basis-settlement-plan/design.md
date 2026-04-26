<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# 0010 — Live-basis settlement correction plan

_Record the runtime decisions, consequences, and implementation runway for
moving Echo from frozen-fork strand settlement toward live holographic strand
semantics._

Legend: KERNEL

Depends on:

- [0006 — Echo Continuum alignment](../0006-echo-continuum-alignment/design.md)
- [0008 — Strand settlement and conflict artifacts](../0008-strand-settlement/design.md)
- [0009 — Witnessed causal suffix export and import](../0009-witnessed-causal-suffix-sync/design.md)
- [0011 — Optic and observer runtime doctrine](../0011-optic-observer-runtime-doctrine/design.md)
- [Echo WARP Drift](../../WARP_DRIFT.md)

## Why this packet exists

Echo's first strand and settlement implementation was a bootstrap cut:

- fork a child worldline from a parent coordinate
- append speculative commits to the child
- compare the child suffix against the recorded base
- import child entries into the base when the base has not moved

That cut was honest, but it can teach the wrong ontology if left implicit.
The current target is stronger:

- a strand is a live speculative lane, not a frozen snapshot
- the fork coordinate is an anchor, not the whole parent basis forever
- parent movement outside the strand-owned footprint should flow through
- parent movement inside the strand-owned footprint requires explicit
  revalidation, obstruction, or conflict

This packet records the decisions made while adding the first live-basis seam
and names the follow-up work required to make the semantic target true.

## Decision 1: BaseRef is an anchor, not the whole basis

`BaseRef` remains the coordinate where the strand diverged, but runtime reads
and settlement planning must be allowed to evaluate the strand against the
current parent basis.

Current implementation evidence:

- `crates/warp-core/src/strand.rs`
- `Strand::live_basis_report(...)`
- `StrandBasisReport`
- `StrandRevalidationState`
- `crates/warp-core/tests/strand_contract_tests.rs`

Consequences:

- `base_ref.provenance_ref` is still required as the anchor witness.
- callers must not infer "parent must still equal anchor" from the presence of
  a `BaseRef`
- live-basis consumers need to inspect the revalidation state before claiming a
  realization is clean
- child-worldline machinery may remain an implementation detail, but public
  semantics should not regress to "copied prefix plus isolated branch"

Open work:

- bounded reading/materialization should consume the same live-basis report
  rather than rebuilding a separate parent-drift law
- `docs/invariants/STRAND-CONTRACT.md` should eventually be updated so the
  invariant prose matches live-basis semantics

## Decision 2: Closed footprint is read plus write

The strand-owned footprint is closed over both:

- slots the child suffix read
- slots the child suffix wrote

Parent writes into either side require revalidation. A parent write into a
child read dependency can invalidate the child's local reasoning even if the
child did not write that slot.

Current implementation evidence:

- `StrandDivergenceFootprint`
- `ParentMovementFootprint`
- `live_basis_report_requires_revalidation_when_parent_invades_owned_footprint`

Consequences:

- overlap detection must use patch `in_slots` and `out_slots`, not only writes
- the runtime is intentionally conservative if a patch over-approximates its
  read footprint
- false positives are acceptable for v1 revalidation; false cleanliness is not

Open work:

- later footprint precision can reduce unnecessary revalidation
- reading artifacts should expose whether a result is clean, revalidated, or
  obstructed

## Decision 3: Disjoint parent drift imports through target-local roots

The intended live-strand law says:

```text
parent movement outside owned footprint + child local suffix
=> clean realization over current parent basis
```

Settlement cannot append that clean realization honestly by reusing the source
entry's state root.

The reason is mechanical and important. An importer that uses the source
entry's expected state root as the target entry's expected state root is
correct only when the target parent is exactly the strand anchor.

When the parent has advanced disjointly, the correct target root is:

```text
state_root(current_parent_tip + child_suffix_patch)
```

not:

```text
state_root(anchor_parent + child_suffix_patch)
```

Those roots differ because the current parent contains additional disjoint
state.

Current implementation evidence:

- `ImportCandidate::target_expected_state_root`
- `settlement_imports_child_suffix_when_parent_advanced_disjoint`
- `append_import_candidate(...)`

Consequences:

- disjoint parent drift is not a conflict reason
- the planner must compute target-local import expectations before accepting
  the source entry
- the resulting target provenance records target-local replay truth while
  preserving `source_ref` as evidence

The implemented correction:

- settlement produces a target-local import candidate for disjoint parent
  advance
- the target-local candidate carries the expected state root after
  applying the source patch to the current target frontier
- append computes the target commit hash from target parents, target state
  root, patch digest, and policy id
- the resulting `MergeImport` preserves `source_ref` as evidence while
  committing target-local replay truth

## Decision 4: Parent overlap must become explicit revalidation

When parent movement overlaps the child-owned footprint, Echo must not smooth
that over as clean flow-through.

Current implementation evidence:

- `ConflictReason::ParentFootprintOverlap`
- `StrandRevalidationState::RevalidationRequired`
- `StrandOverlapRevalidation::{Clean, Obstructed, Conflict}`
- `settlement_imports_child_suffix_when_parent_overlap_revalidates_clean`
- `settlement_records_conflict_artifact_when_parent_overlap_changes_target_state`

Consequences:

- settlement no longer blanket-rejects all owned-footprint overlap
- overlap that is already satisfied on the current parent basis imports as
  `Clean`
- overlap that would mutate target state remains explicit
  `ParentFootprintOverlap` residue with `Conflict` revalidation metadata
- apply failure on overlapped replay is represented as `Obstructed`
- no hidden retry loop may silently convert overlap into a clean import; the
  decision carries revalidation metadata

Open work:

- define the revalidation artifact shape consumed by observer/read tooling
- thread the same revalidation posture into bounded reads instead of letting
  reading code invent a parallel law
- add an obstruction-focused fixture once there is a natural patch-level
  obstruction case worth preserving

## Decision 5: Local iteration speed is part of the plan

The runtime work is subtle enough that broad Cargo filters were slowing down
the feedback loop and obscuring stale fixture failures.

Current implementation evidence:

- `cargo xtask test-slice strand`
- `cargo xtask test-slice settlement`
- `cargo xtask test-slice observation`
- `cargo xtask test-slice neighborhood`
- `cargo xtask test-slice warp-core-smoke`
- `make test-slice SLICE=...`
- `docs/workflows.md`

Consequences:

- focused runtime slices are now first-class workflow, not ad hoc shell memory
- broad filters such as `cargo test -p warp-core settlement` should be avoided
  during tight iteration because Cargo still compiles and launches unrelated
  integration-test targets
- broader gates still matter before PR submission; slices are for fast local
  correctness cycles

## Overall correction plan

### Step 1: Live-basis detection and conservative settlement classification

Status: implemented in the current slice.

Scope:

- compute child closed footprint
- compute parent movement after anchor
- report `AtAnchor`, `ParentAdvancedDisjoint`, or `RevalidationRequired`
- thread that report into settlement compare/plan
- expose an ABI conflict reason for parent footprint overlap

What this proves:

- Echo can observe the difference between frozen-fork divergence and
  live-basis parent movement
- the runtime no longer has to collapse all parent drift into
  `BaseDivergence`

What this does not prove:

- observer/read paths do not yet consume the same overlap revalidation posture

### Step 2: Target-local import candidates for disjoint parent drift

Status: implemented in the current slice.

Required behavior:

- when parent movement is disjoint, settlement should apply the source patch to
  the current target frontier simulation
- the plan should produce an import candidate whose target expected root is the
  simulated target root, not the source root
- execution should append `MergeImport` using target-local root and parent
  hashes while preserving source provenance identity
- tests should prove the final target contains both the parent drift and child
  imported change

Likely code surfaces:

- `crates/warp-core/src/settlement.rs`
- `ImportCandidate`
- `append_import_candidate(...)`
- `SettlementPlan::to_abi(...)`

ABI note:

- target-local import detail is currently internal to `warp-core`
- ABI plan shape still exposes the source import candidate, while execution
  commits target-local replay truth

### Step 3: Explicit overlap revalidation

Status: implemented for settlement; observer/read artifact integration remains
planned.

Required behavior:

- overlap must produce an inspectable revalidation state or artifact
- the result must distinguish at least:
    - revalidated clean
    - obstructed
    - explicit conflict
- settlement stores the result on import/conflict decisions
- bounded reads should consume the same parent-drift law in the next slice

Likely code surfaces:

- `crates/warp-core/src/strand.rs`
- `crates/warp-core/src/settlement.rs`
- future observer/reading artifact types

### Step 4: Observer plans and bounded reading artifacts

Status: partially implemented by `ObservationArtifact::basis_posture`; broader
observer plans remain planned by
[PLATFORM_observer-plan-reading-artifacts](../../method/backlog/asap/PLATFORM_observer-plan-reading-artifacts.md).
Doctrine: [0011 — Optic and observer runtime doctrine](../0011-optic-observer-runtime-doctrine/design.md).

Required behavior:

- expose read-side posture as an artifact, not just a materialized state blob
- carry coordinate, basis, witness, budget, and obstruction/plurality state
- consume live-basis/revalidation output instead of inventing a separate read
  law

Current implementation evidence:

- `ObservationBasisPosture`
- `ObservationArtifact::basis_posture`
- `ObservationHashInput::basis_posture`
- ABI version 5

### Step 5: Witnessed suffix admission shells

Status: planned by
[PLATFORM_witnessed-suffix-admission-shells](../../method/backlog/asap/PLATFORM_witnessed-suffix-admission-shells.md)
and [0009](../0009-witnessed-causal-suffix-sync/design.md).

Required behavior:

- export/import witnessed suffix shells, not naked patch streams or state blobs
- normalize peer suffixes through the same admission/settlement law
- return explicit outcomes: admitted, staged, plural, conflict, or obstruction

## Non-goals

- Do not remove child-worldline machinery just to satisfy terminology.
- Do not claim final braid collapse semantics in this packet.
- Do not make disjoint parent drift look clean in settlement until target-local
  import roots are implemented.
- Do not hide overlap revalidation inside an opaque retry loop.

## Practical rule for future work

When a runtime decision feels janky, record it with:

- the semantic target
- the current implementation fact
- the consequence of the gap
- the next correction
- file/test evidence

The goal is not to freeze every intermediate compromise. The goal is to keep
intermediate compromises from becoming invisible doctrine.
