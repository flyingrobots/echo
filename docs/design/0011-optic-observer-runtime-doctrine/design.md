<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# 0011 — Optic and observer runtime doctrine

_Formalize the runtime subset of WARP optics and Observer Geometry so Echo can
implement live strands, settlement, observation, and witnessed shells with one
shared noun stack._

Legend: KERNEL / PLATFORM

Depends on:

- [0005 — Echo TTD witness surface](../0005-echo-ttd-witness-surface/design.md)
- [0006 — Echo Continuum alignment](../0006-echo-continuum-alignment/design.md)
- [0008 — Strand settlement and conflict artifacts](../0008-strand-settlement/design.md)
- [0009 — Witnessed causal suffix export and import](../0009-witnessed-causal-suffix-sync/design.md)
- [0010 — Live-basis settlement correction plan](../0010-live-basis-settlement-plan/design.md)
- [Continuum Foundations](../../architecture/continuum-foundations.md)
- [Observer plans and reading artifacts](../../method/backlog/asap/PLATFORM_observer-plan-reading-artifacts.md)

Source theory inputs:

- WARP paper 07, "Optics, Holograms, and ..." draft
- Observer Geometry I, "Projection, Basis, Accumulation, and Observer
  Signatures" draft

## Why this packet exists

Echo already uses optic language, but the implementation still spreads the
concept across local runtime terms:

- footprint enforcement
- tick admission
- strand live-basis reports
- settlement plans
- conflict artifacts
- observation artifacts
- TTD witness shells

The risk is semantic drift. If settlement, observation, import, and debugging
each invent a local version of "clean", "conflict", "obstruction", or
"reading", Echo will eventually have multiple incompatible truth laws.

This packet pins the shared doctrine:

- an **optic** is the runtime shape for bounded lowering, admission, witness,
  and retention
- an **observer** is the revelation-side structure that determines what a read
  can project, preserve, accumulate, and emit
- a **reading artifact** is not a raw state blob; it is an observer-relative
  emission with coordinate and witness posture

## Decision 1: Optic is the runtime boundary object

Echo should formalize the WARP optic as the shared boundary object across tick
execution, strand settlement, braid comparison, suffix import, and observation.

Runtime subset:

```text
Optic = (ObserverPlan, OpticSlice, LoweringSurface, AdmissionLaw, RetentionContract)

Lower(frontier, weave) = (Outcome, Witness, Shell)
```

Where:

- `ObserverPlan` fixes the revelation-side discipline available to this optic
- `OpticSlice` is the bounded site where judgement is lawful
- `LoweringSurface` constructs comparable claims over that slice
- `AdmissionLaw` classifies the claims
- `RetentionContract` specifies what must be preserved for replay, audit,
  transport, reliance, or revelation
- `Outcome` is the admitted result
- `Witness` is the evidence that makes the outcome reviewable
- `Shell` is the retained carrier that satisfies the retention contract

The outcome algebra should stay broader than binary success/failure:

```text
Outcome(X) = Derived(X) | Plural(X) | Conflict | Obstruction
```

Current Echo-specific names may remain narrow, but new runtime concepts should
not collapse this algebra into `Ok` versus `Err`.

## Decision 2: Observer is revelation-side, not the whole optic

The Observer Geometry model should be formalized as Echo's read-side structure,
but it must not absorb the whole optic.

Runtime subset:

```text
StructuralObserver = (Projection, ObserverBasis, ObserverState, UpdateLaw, EmissionLaw)
```

Where:

- `Projection` is the raw view over state, provenance, shell, or history
- `ObserverBasis` names the native distinctions the observer can express
- `ObserverState` holds accumulated observer memory when the observer is
  stateful
- `UpdateLaw` accumulates projected traces
- `EmissionLaw` produces the externally visible reading

Echo should distinguish:

- `ObserverPlan`: authored or compiled revelation discipline
- `ObserverInstance`: runtime observer plus accumulated state
- `ReadingArtifact`: emitted result with witness and posture metadata

The observer is therefore the revelation-side face of an optic. It decides what
can be seen and emitted; it does not by itself decide what becomes admitted
kernel truth.

## Decision 3: Parent basis and observer basis are different nouns

Echo now needs two basis concepts, and conflating them would make the code hard
to reason about.

`ParentBasis` means the parent lane coordinate against which a strand or braid
is realized.

`ObserverBasis` means the native distinctions an observer can express,
preserve, or accumulate.

`BasisPosture` should be reserved for runtime claims about a realization:

- `AtAnchor`
- `ParentAdvancedDisjoint`
- `RevalidationRequired`
- later read-side refinements such as `Clean`, `Obstructed`, or `Conflict`

Avoid introducing an unqualified `Basis` type in shared runtime APIs. If a
short name is unavoidable, it must live in a tightly scoped module.

## Decision 4: Keep commitment, retention, and revelation separate

Echo should preserve three separate moments:

```text
Commit / Lower:
  build the bounded slice, judge claims, and produce a witnessed outcome

Fold / Retain:
  preserve a shell that satisfies replay, audit, transport, or revelation
  obligations

Reveal / Observe:
  emit an observer-relative reading under aperture, basis, budget, and rights
```

This separation prevents a common failure mode:

- commitment truth becoming UI-only metadata
- storage shells pretending to be the admitted outcome
- observer projection pretending to be the whole kernel state

Kernel code should be allowed to optimize carriers, but it must not erase the
semantic boundary between outcome, witness, shell, and reading.

## Decision 5: Strand settlement is the first concrete braid optic

The current live-basis settlement work is already an optic instance.

Mapping:

| Optic role        | Current Echo settlement role                                    |
| ----------------- | --------------------------------------------------------------- |
| `frontier`        | current target parent worldline frontier                        |
| `weave`           | child suffix entries from the strand lane                       |
| `OpticSlice`      | `StrandBasisReport`, owned footprint, parent movement, overlap  |
| `LoweringSurface` | target-local simulation of source patches over the parent basis |
| `AdmissionLaw`    | import, clean overlap, conflict, or obstruction classification  |
| `Outcome`         | `ImportCandidate` or `ConflictArtifactDraft`                    |
| `Witness`         | basis report, source refs, overlap slots, revalidation outcome  |
| `Shell`           | recorded `MergeImport` or `ConflictArtifact` provenance entry   |

Implementation evidence:

- `StrandBasisReport`
- `StrandDivergenceFootprint`
- `StrandRevalidationState`
- `StrandOverlapRevalidation`
- `SettlementPlan`
- `ImportCandidate::target_expected_state_root`
- `ConflictReason::ParentFootprintOverlap`

Consequence:

Settlement should continue to use settlement-local concrete types until there
is a second or third concrete optic instance that justifies a generic runtime
trait. Do not build a universal optic engine prematurely.

## Decision 6: Observation must become revelation over a reading artifact

`ObservationService::observe(...)` is currently the canonical read path, but it
still looks too much like a request for materialized state.

The next correction is to make observation explicitly return a bounded reading
artifact.

Minimum reading artifact posture:

```text
ReadingArtifact {
    resolved_coordinate,
    observer_plan_ref_or_kind,
    observer_basis,
    projection,
    payload,
    witness_refs,
    parent_basis_posture,
    residual_posture,
    budget_posture,
    rights_posture,
}
```

The first implementation slice exposed parent-basis posture. The second slice
wraps observation artifacts in `ReadingEnvelope`, includes it in
`ObservationHashInput`, and makes observer plan, optional hosted observer
instance, observer basis, witness refs, budget posture, rights posture, and
residual posture ABI-visible. `ObservationRequest` also names observer plan,
optional instance, read budget, and rights posture explicitly.
The kernel keeps full overlap slots internally; the ABI carries overlap count
plus a deterministic slot digest until a stable public slot representation
exists.

Required behavior:

- observing a plain worldline coordinate reports ordinary coordinate truth
- observing a live strand reports the parent-basis posture used for the read
- disjoint parent drift is visible as clean live-basis posture
- owned-footprint overlap is visible as revalidation-gated posture
- reads consume `Strand::live_basis_report(...)` or a shared derivative rather
  than rebuilding overlap law locally

## Decision 7: Witness-bearing shells are the transport and audit boundary

Suffix export/import and TTD explanation should speak in shells, not naked
patch streams or state blobs.

For Echo, a shell is any retained carrier that satisfies an explicit
`RetentionContract`.

Examples:

- tick receipt and replay artifacts
- settlement import/conflict provenance entries
- witnessed suffix shells
- reading artifacts that point at retained provenance and witness material

Shell equivalence should be behavioral:

- two shells may differ in storage layout
- they are equivalent only when they satisfy the same replay, audit, transport,
  or revelation obligations

This lets Echo optimize storage without making observers guess whether a folded
carrier still supports their task.

## Naming doctrine

Use names that expose the layer:

- `OpticSlice` for bounded comparable sites
- `AdmissionOutcome` for derived/plural/conflict/obstruction results
- `RetentionContract` for shell obligations
- `ReadingArtifact` for emitted observer-relative readings
- `ObserverBasis` for native observer distinctions
- `ParentBasis` or `StrandBasis` for realization coordinates
- `BasisPosture` for runtime cleanliness/revalidation posture

Avoid names that hide the layer:

- `Result` for optic admission outcomes
- `Snapshot` for a witness-bearing read
- `Merge` for all settlement outcomes
- bare `Basis`
- bare `Observer` when the value is only a plan or only an emitted reading

## Implementation runway

### Step 1: Doctrine packet

Status: this packet.

Purpose:

- align Echo's current live-strand settlement work with the WARP optic form
- align the next observation slice with Observer Geometry
- prevent read and settlement paths from inventing parallel laws

### Step 2: Observation basis posture

Status: implemented for basis posture; superseded by Step 3's explicit reading
envelope boundary.

Scope:

- add internal read-side posture metadata to `ObservationArtifact`
- expose whether a read was ordinary worldline, anchored strand, disjoint
  live-basis strand, or revalidation-gated strand
- reuse `StrandBasisReport` or a smaller shared posture type
- add tests proving reads do not hide parent-basis overlap

Current implementation evidence:

- `ObservationBasisPosture`
- `ReadingEnvelope::parent_basis_posture`
- `ObservationArtifact::reading`
- `ObservationHashInput::reading`
- `strand_frontier_observation_reports_disjoint_live_basis_posture`
- `strand_frontier_observation_reports_overlap_revalidation_posture`

Likely code surfaces:

- `crates/warp-core/src/observation.rs`
- `crates/warp-core/src/strand.rs`
- `crates/warp-core/src/neighborhood.rs`
- `crates/echo-wasm-abi/src/kernel_port.rs` when ABI exposure is needed

### Step 3: Reading envelope family boundary

Status: implemented for built-in one-shot observations by
[Reading envelope family boundary](../../method/backlog/up-next/PLATFORM_reading-envelope-family-boundary.md).

Scope:

- name the emitted artifact family cleanly
- distinguish authored observer plan, runtime observer instance, and emitted
  reading
- decide which posture fields become stable ABI now versus internal kernel
  metadata

Current implementation evidence:

- `ReadingEnvelope`
- `ReadingObserverPlan`
- `ReadingObserverBasis`
- `ReadingWitnessRef`
- `ReadingBudgetPosture`
- `ReadingRightsPosture`
- `ReadingResidualPosture`
- `ordinary_worldline_observation_reports_worldline_posture`

### Step 4: Witnessed suffix shells

Status: planned by
[Witnessed suffix admission shells](../../method/backlog/asap/PLATFORM_witnessed-suffix-admission-shells.md).

Scope:

- export/import suffixes as retained shells
- normalize transported suffixes through the same optic admission law
- return admitted, plural, conflict, or obstruction outcomes

### Step 5: Generic optic helpers only after repetition

Status: deferred.

Do not introduce a universal `Optic` trait until at least two concrete paths
need the same helper surface.

Candidate future common surface:

- `OpticSlice` hashing
- admission outcome hashing
- witness/ref packing
- retention-contract checking
- shell-equivalence tests

## Non-goals

- Do not rewrite settlement around a generic optic trait in this slice.
- Do not make every observer stateful.
- Do not require the ABI to expose every internal posture immediately.
- Do not collapse observer basis and parent basis into one type.
- Do not treat a final state snapshot as a sufficient reading artifact.
- Do not block current diagnostic materialization helpers; they remain useful
  as low-level projections.

## Acceptance criteria for the next code slice

The next observer implementation slice should prove:

- an observation artifact carries explicit read posture
- ordinary worldline reads remain stable and read-only
- strand reads report live-basis posture
- disjoint parent movement is visible as clean live-basis reading
- parent movement inside the owned footprint is visible as revalidation-gated
  reading
- the read path does not duplicate settlement's parent-overlap law

The test names should make the distinction obvious. If a future reader cannot
tell whether a test is proving state payload, provenance posture, or observer
artifact shape, the boundary is still too muddy.

## Practical rule

When adding a runtime feature, ask which moment it belongs to:

- **Commit:** does this decide what becomes admitted truth?
- **Retain:** does this preserve the shell needed for replay, audit, transport,
  or revelation?
- **Reveal:** does this emit what an observer may lawfully see?

If the answer is "all three", split the types. That is the point of the optic
and observer doctrine.
