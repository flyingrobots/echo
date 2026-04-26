<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo WARP Drift

This note captures where Echo currently drifts from the stronger WARP
doctrine now stabilized across the papers and cross-repo design work.

It is not a claim that Echo is "wrong." Echo is ahead in some places.
It does mean some bootstrap implementation choices are in danger of
hardening into ontology if they are not corrected deliberately.

## The current WARP baseline

The relevant baseline is now:

- **There is no canonical materialized graph.** The substrate is
  witnessed causal history. Graph-like structure is an
  observer-relative reading over that history.
- **A strand is a real speculative lane, not a frozen snapshot.**
  Its realized state is resolved against inherited parent history at a
  chosen basis, and bounded reads should materialize only the backward
  causal cone required by the local divergence and optic footprint.
- **Resolution is the same kernel at every scale.** Tick admission,
  braid comparison/collapse, and replica import differ mainly by their
  normalization path, not by being fundamentally different problems.
- **Observation is not just querying state.** The read side should be
  described in terms of observer plans, bounded readings, witness, and
  rights/budget posture.
- **Distributed import is witnessed suffix admission, not state sync.**
  The durable object is a witnessed shell/hologram over transported
  claims, and the important invariant is shell equivalence, not naive
  final-state agreement alone.

## Where Echo is already strong

Echo is not generally behind WARP.

The repo already has meaningful runtime truth in exactly the areas that
matter for the admission kernel:

- `settlement.rs` defines `SettlementDelta`, `SettlementPlan`,
  `SettlementDecision`, `ConflictArtifactDraft`, and
  `SettlementResult`.
- the ABI/kernel boundary already has a generic `dispatch_intent(...)`
  plus observation and settlement publication entrypoints.
- neighborhood and settlement publication are already treated as
  witness-bearing runtime surfaces rather than ad hoc debug helpers.

That means Echo is already closer to the current WARP core than a
surface skim might suggest.

## Where Echo is drifting

### 1. Strands are still defined as bootstrap child-worldline forks

This is the biggest drift.

The current strand contract still treats a strand as:

- an ephemeral speculative execution lane
- derived from a base worldline at a specific tick
- implemented as a child worldline created by fork
- pinned to one immutable base coordinate

That was an honest bootstrap cut, but it is no longer the right target.
If this hardens, Echo will keep teaching frozen-fork semantics after
WARP has moved to live-following holographic strands.

### 2. Strand lifecycle is still "session-scoped and hard-delete"

The repo still says a strand must not outlive the session, and drop
removes the strand, child worldline, heads, and provenance, returning a
`DropReceipt` as the only proof the strand existed.

That is a perfectly understandable bootstrap safety posture. It should
not become the theory of strands.

The stronger model is:

- the strand is a real speculative lane
- the runtime may cache or retain it however it wants
- the important thing is that reads and settlement treat it as a lawful
  speculative history-bearing object

### 3. The observer/read boundary is still under-specified

Echo has the right top-level instinct:

- `dispatch_intent(...)`
- `observe(...)`
- `observe_neighborhood_site(...)`
- settlement publication

But the read side is not yet explicit enough about:

- observer plan vs observer instance
- bounded reading artifact vs raw state snapshot
- budget and rights posture
- witness/shell metadata carried by a reading

If this stays vague, integrators will route around the doctrine and
fall back to "observe = materialize some state."

### 4. Witnessed suffix sync needs the stronger shell model

Echo already has a good design direction for witnessed suffix sync, but
the final semantic cut is not yet locked at the runtime boundary.

The important correction is:

- do not export naked patch streams as the meaning of sync
- do not import by replaying a frontier delta loop and calling that the
  theory
- do export/import witnessed suffix shells whose admission outcome is
  explicit: admitted, staged, plural, conflict, or obstruction

### 5. Public docs still mix the new doctrine with older graph/state-first language

The repo README is much better than it used to be, but it still
teaches some older intuitions:

- "deterministic causal graph-rewrite engine"
- "a new state"
- immutable snapshots as the primary explanatory frame

Those phrases are not fatal. They do pull the reader back toward a
state-first picture just as the rest of the stack is moving away from
it.

## What Echo should do next

The correction path is straightforward.

### First: re-found strands as live holographic lanes

The runtime should move from:

- frozen base coordinate
- copied child-worldline prefix
- hard-delete bootstrap semantics

to:

- parent anchor plus live-following inherited history
- local ownership only over the closed optic footprint required for the
  divergence
- basis-relative realization
- explicit revalidation/conflict when the parent moves inside an owned
  footprint

Implementation note: the runtime now exposes this distinction via
`Strand::live_basis_report(...)` and settlement planning. Disjoint parent drift
settles through a target-local import root. Owned-footprint overlap runs
explicit settlement revalidation: already-satisfied replay imports as `Clean`,
apply failure is `Obstructed`, and state-changing replay remains
`ParentFootprintOverlap` conflict residue. The active decision/implementation
plan is
[0010 — Live-basis settlement correction plan](../design/0010-live-basis-settlement-plan/design.md).

### Second: define observer plans and reading artifacts explicitly

Echo should expose the read side as:

- an authored/configured observer plan
- a runtime observer instance when accumulation is needed
- an emitted reading artifact carrying payload, coordinate, witness,
  budget, and obstruction/plurality posture

This keeps the runtime generic and keeps applications from treating
Echo as an app-local state server in disguise.

### Third: promote suffix exchange into admission shells

Echo should make remote import/export look like ordinary admission at a
distance:

- normalize to a comparable frontier
- carry the transported local situation in a witness-bearing shell
- return explicit import outcomes
- preserve the shell-equivalence story for independent imports

### Fourth: update the docs/invariants to match the corrected runtime

The bootstrap strand contract and README language should be revised only
after the runtime direction is pinned, not before.

## ASAP backlog items

These items now define the reconciliation path:

- [KERNEL_live-holographic-strands](../method/backlog/asap/KERNEL_live-holographic-strands.md)
- [PLATFORM_observer-plan-reading-artifacts](../method/backlog/asap/PLATFORM_observer-plan-reading-artifacts.md)
- [PLATFORM_witnessed-suffix-admission-shells](../method/backlog/asap/PLATFORM_witnessed-suffix-admission-shells.md)

Relevant existing design context:

- [0004 — Strand contract](../design/0004-strand-contract/design.md)
- [0008 — Strand settlement](../design/0008-strand-settlement/design.md)
- [0009 — Witnessed causal suffix sync](../design/0009-witnessed-causal-suffix-sync/design.md)
- [0010 — Live-basis settlement correction plan](../design/0010-live-basis-settlement-plan/design.md)
- [0011 — Optic and observer runtime doctrine](../design/0011-optic-observer-runtime-doctrine/design.md)
- [0006 — Echo Continuum alignment](../design/0006-echo-continuum-alignment/design.md)

## Practical rule

Echo should remain free to use worldlines, heads, caches, checkpoints,
and child-lane machinery internally.

What must change is the semantic story told by the runtime and its
docs:

- strands are not frozen forks
- reads are not just state snapshots
- sync is not packet or patch folklore

Echo is strongest when it treats all three as the same admission story
at different altitudes.
