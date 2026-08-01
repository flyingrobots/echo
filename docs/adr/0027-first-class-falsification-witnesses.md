<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# ADR 0027: First-Class Falsification Witnesses

- **Status:** Proposed
- **Date:** 2026-08-01

## Context

Echo admits meaning. It admits operation packages, invocations, causal anchors,
external-action settlements, and observations, and in each case it separates the
party that proposes something from the authority that admits it. Echo has no
corresponding category for the opposite fact: a durable, replayable record that
one of its own semantic claims was shown to be false.

Today that evidence lives in test logs. A `proptest` failure produces a seed, a
shrunk case, and a nonzero exit code. None of those are admitted Echo facts. The
seed does not survive a strategy change. The shrunk case carries no basis, no
observer, no aperture, and no rights posture, so it cannot say _who_ was entitled
to observe the contradiction or _against what exact meaning_ it holds. The exit
code cannot distinguish a refuted property from an obstruction, a scheduler
rejection, a conflict, or a crash.

The gap is concrete and already named in the repository.
`docs/topics/GeneratedRules.md` states that generated footprints are compile-time
claims, that runtime footprint checking is a generator-correctness oracle, and
that the `footprint_enforce_release` qualification lane is not wired into CI.
Echo asserts footprint honesty and cannot presently produce a durable artifact
demonstrating a violation of it.

Three temptations are available and all three are wrong:

- **A boolean on the claim.** Setting `claim.is_falsified` mutates the target
  worldline to say it had a bug. It destroys the distinction between what Echo
  admitted and what Echo later learned, and it makes the fact unverifiable —
  a flag carries no experiment.
- **A specialized obstruction.** An obstruction says Echo could not complete
  something. A falsification says Echo completed everything and the answer was
  wrong. Collapsing them makes "the evaluator lacked authority" and "the property
  is false" the same fact.
- **A richer test-report format.** Any format authored by the discovering tool
  inherits that tool's trust. A fuzzer, a model checker, a remote Kitten, or a
  human can all be buggy, nondeterministic, or adversarial.

## Decision

Echo gains a new admitted semantic-evidence category: the falsification witness.

> **Anyone may discover and propose a counterexample. Only Echo may admit that
> the counterexample falsifies an exact property instance.**

An admitted witness asserts one narrow proposition:

> Given property **P**, its exact semantic closure and lawpack **L**, evaluation
> basis **B**, observer **O**, optic and aperture **A**, rights and budget
> posture **R**, and a replayable causal experiment **H**, Echo reproduced a
> reading **V** for which the property evaluator returned the typed violation
> class **C**.

Four artifacts carry that proposition, with distinct trust postures:

| Artifact                         | Trust posture               |
| -------------------------------- | --------------------------- |
| `GeneratedPropertyV1`            | Admitted authored meaning   |
| `PropertyInstanceV1`             | Echo-bound                  |
| `CounterexampleProposalV1`       | Untrusted                   |
| `AdmittedFalsificationWitnessV1` | Authoritative Echo evidence |

`docs/topics/FalsificationWitnesses.md` holds the field-level schemas, the
verification pseudocode, the reduction law, the threat table, and the delivery
sequence. This record fixes the boundaries those schemas must respect.

### The trust boundary

Discovery sits outside admission. A discovery engine's generator, seed, search
order, coverage map, heuristic score, and locally shrunk output are explanatory
provenance and nothing more. Echo recomputes every semantic fact. A proposal that
asserts a property failed has exactly the standing of a proposal that asserts
anything else: none, until Echo reproduces it.

This mirrors the executable-operation corridor, where exact package bytes do not
independently confer a coordinate, installation, invocability, or authority, and
where the scheduler — not application code — owns private evaluation. A property
package is likewise not self-authorizing: a predicate-program digest cannot mint
a public property coordinate or grant observation rights. Installation begins
from an admitted package and policy.

The verifier, slicer, reducer, and admission entry point are not methods on an
application-facing handle, for the same reason transitional direct operation
prepare/commit is hidden from `TrustedRuntimeApp`. Application code must not
choose when private evaluation or publication occurs.

### The outcome taxonomy is a closed sum

Property evaluation returns exactly one of `HoldsForCase`, `Violated`,
`Obstructed`, or `RuntimeFault`.

`HoldsForCase` says the concrete case did not falsify the property. It is not a
proof of a universal statement. `Obstructed` says Echo could not complete the
required execution, observation, or interpretation under the bound contract — it
says nothing about whether the property holds. `RuntimeFault` says Echo failed to
maintain its own invariants; a fault is not promoted to a semantic refutation
merely because it occurred during property evaluation. Only `Violated` can lead
to an admitted witness.

Echo does not get to blur these under load. A campaign that cannot obtain its
retained material reports obstruction, not refutation.

### Admission requires fresh-host exact replay

In-process re-evaluation is preflight evidence. It detects immediate
nondeterminism cheaply, and shared process state can mask exactly the
dependencies a witness must prove. It is not the admission boundary.

Admission requires reconstructing the property instance on a fresh verification
host: exact package installation, exact basis reconstruction, submission through
ordinary Action ingress, scheduler-owned evaluation, observation under the bound
observer plan and aperture, and evaluation by the exact property program. This
follows the existing fail-closed pattern in operation recovery, which
re-evaluates the compiler-owned result projection over retained canonical input
and requires byte-for-byte equality before publishing.

Echo must not label same-interpreter replay "independent." Independently
implemented replay is a higher evidence grade and a later goal.

### Reduction preserves a typed violation class

A reducer that only preserves "some failure" can silently walk from one bug to
another and present the result as a minimal example of the first. Every accepted
reduction step must preserve the property-declared violation class under an
explicit `ViolationEquivalencePolicyV1`.

The interestingness predicate is therefore not "the process exited nonzero." A
candidate is interesting only if it is canonically valid, replays from the exact
bound basis, observes successfully under the exact bound observer, aperture, and
rights, returns `Violated`, and returns an equivalent violation.

Aperture is part of the claim, not a reduction target. It may be reduced only
when the property explicitly quantifies over apertures, and the result is a new
`PropertyInstanceV1`.

### Minimality is always qualified

Echo claims `LocallyIrreducible` or `BudgetExhausted`, never an unqualified
global minimum. `ExhaustivelyMinimal` exists but is reserved for explicitly
finite bounded domains with an enumeration certificate.

Reduction budgets are deterministic counters — candidate evaluations, Actions
replayed, scheduler passes, property evaluations, retained bytes loaded, phases,
dependency edges. Wall-clock deadlines may protect an operator but must never
participate in the canonical result, because host speed differs. An interrupted
campaign records `ExternallyInterrupted` or `ReductionObstructed`; it must not
claim the deterministic budget was exhausted.

Minimality includes least sufficient revelation, not only fewest Actions.

### Two identities, not one

`SemanticCounterexampleIdV1` commits the property instance, minimized case,
reduced experiment, violation class, and violating read coordinate. It excludes
fuzzer name, seed, shrinker version, submitter, reduction trace, host identity,
byte placement, and admission coordinate. It answers: _is this the same semantic
counterexample?_

`FalsificationArtifactIdV1` commits the semantic id plus the full evidence
envelope. It answers: _is this the same admitted evidence envelope?_

Two tools finding the same case must converge on one semantic identity while
retaining distinct provenance. Reducer search traces must stay out of the
semantic identity, because reducer algorithms will improve and the semantic
object is the refuting case, not the path that found it.

This extends the existing retained-evidence distinction, where the semantic
coordinate says what question bytes answer while the content hash identifies the
bytes.

### The target worldline is never rewritten

Verification runs on private hosts or strands. The admitted witness is appended
to a separate evidence worldline whose history says Echo admitted evidence
_about_ the target history. The admission transaction affects exactly one
evidence-worldline frontier and zero target-worldline frontiers.

The evidence worldline's state is an append-only catalog. Derived indexes —
witnesses per property, current claim posture, regression sets — are disposable
and rebuildable from the WAL.

An old witness remains permanently valid against its original property instance.
A fix does not retroactively invalidate historical evidence; reapplication
against a successor property produces a _new_ artifact carrying
`StillFalsifies`, `NoLongerFalsifies`, `Inapplicable`, or `Obstructed`. A release
gate asks whether successor outcomes are acceptable, never whether old witnesses
have been deleted.

### Durability precedes publication

The witness commits through one failure-atomic WAL transaction under
`WalAppendAuthority::AdmissionKernel`, which already exists and already governs
causal-anchor admission. The transaction kind is new; so are its record kinds.
Nothing — witness, index, or receipt — is published before the transaction is
committed and flushed.

Recovery revalidates shape and authority, decodes bounded payloads canonically,
recomputes both identities, cross-checks the receipt, and rebuilds indexes. A
missing retained object must not make a witness disappear: recovery surfaces it
as evidence-unavailable or replay-obstructed, preserving the historical fact that
it was admitted while refusing to claim it is presently replayable. This is the
posture retained evidence already takes, where missing material obstructs
explicitly instead of becoming an empty read, a cache hit, or a generic failure.

### Enforcement posture is part of the evidence

The footprint guard is compiled out unless `debug_assertions` or
`footprint_enforce_release` is enabled. A witness produced under a build where
the guard was inert demonstrates nothing. The enforcement posture is recorded in
the replay certificate and checked at admission.

## Consequences

### The first vertical is footprint honesty, and it is blocked on a real gap

`GeneratedFootprintSoundness@1` states `Actual ⊆ Declared` for reads and writes.
Verification established that Echo cannot evaluate this today, for a reason
sharper than expected: `FootprintGuard` holds only the declared sets and has no
accumulator. It compares each access against what was declared and panics on a
miss. Nothing records what an Action actually touched.

`FootprintViolation` does not close the gap. It is a `std::panic::panic_any`
payload naming the single access that tripped the guard, so catching the unwind
yields one violating access rather than an actual footprint.

The guard must therefore grow an opt-in accumulating sink, additive to the
existing checker, leaving the panic path unchanged. The panic is the correct
response in ordinary execution, where an undeclared access is a programmer error
rather than a recoverable condition, and this record does not authorize removing
it.

Because the guard is constructed once per rule execution and pre-filtered to one
warp, an accumulator hung off that instance is per-Action by construction. No new
observation projection is required: actual footprints reach the read-only
evaluator on the execution-evidence channel, which keeps the bound observation
aperture untouched.

### Fixtures must not be manufactured by weakening admission

The production-shaped fixture is a _false property_ over a _lawful_ operation: a
valid operation reads `A` and `B` while a deliberately false property claims its
reads are contained in `{A}`. A second fixture exercises the existing guard
through a provider or Wesley callback under `footprint_enforce_release`, and its
evidence grade must state that provider-native callback replay depends on
reinstalling the exact ambient callback implementation.

Forcing an invalid footprint into an executable-operation package to exercise the
witness system is rejected. The operation corridor's package/program/footprint
closure is a security property, not a test inconvenience.

### WAL rollout is reader-first

The WAL decoder rejects unknown transaction and record codes rather than skipping
them. New writers must not emit falsification records until every reader capable
of opening that WAL is upgraded: ship decoders and recovery logic first, advertise
capability, then activate writing behind an upgraded writer epoch, preferably at
a segment boundary. An older binary meeting the new epoch must refuse read-write
activation rather than truncate.

Retained evidence gains new explicit roles with append-only stable tags rather
than overloading `RetainedEvidenceRole::Witness`. Existing identities are
unchanged because existing tags do not move.

### Costs accepted

- **Fresh-host replay is expensive.** Reconstructing a host per reduction
  candidate may dominate campaign cost. Snapshot-plus-suffix optimization is
  permitted only after proving equivalence to fresh reconstruction.
- **Campaigns are not Ticks.** A reduction campaign spans hundreds or thousands
  of bounded replays and cannot occupy one atomic scheduler unit. A campaign
  coordinator is required, and no attempt gains authority from being requested by
  it.
- **A shared evaluator can share a defect.** Property evaluation and replay
  verification run on the same implementation, so a common bug is invisible.
  Evidence grades must say so honestly.
- **More surface to keep honest.** Four artifacts, two identity laws, a reduction
  law, and a new WAL transaction kind are a significant addition to a runtime
  that already carries a large operation surface.

### Alternatives rejected

| Alternative                                                        | Why rejected                                                                                                     |
| ------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------- |
| Store only a property-test seed                                    | Strategy and RNG changes silently destroy the case; nondeterministic strategies break seed persistence outright. |
| Treat a test runner's shrink result as authoritative               | Imports the discovering tool's trust into Echo's evidence.                                                       |
| Mutate the target worldline with a bug flag                        | Destroys the admitted/learned distinction and carries no experiment.                                             |
| Model falsification as an obstruction                              | Conflates "could not complete" with "completed and was wrong."                                                   |
| Call every failed replay a falsification                           | Promotes obstructions and runtime faults into semantic refutations.                                              |
| Claim global minimality from a local reducer                       | Unprovable and unstable across reducer revisions.                                                                |
| Allow silent rebasing of a witness                                 | A changed basis is a different property instance.                                                                |
| Allow aperture widening during replay                              | Manufactures contradictions by observing more than the claim entitled.                                           |
| Reuse `CausalSuffixBundle` as a replay package                     | It is a shape-only witnessed shell, not a materialized, executable replay bundle.                                |
| Treat same-interpreter replay as independent evidence              | Overstates the evidence grade.                                                                                   |
| Weaken executable-operation admission to build a dishonest fixture | Trades a security property for test convenience.                                                                 |
| Admit a witness under inert footprint enforcement                  | The experiment proves nothing if the guard was compiled out.                                                     |

## References

- `docs/topics/FalsificationWitnesses.md` — schemas, verification pseudocode,
  reduction law, threat model, test matrix, and delivery roadmap.
- [ADR 0014](0014-generated-rule-authorship-and-footprints.md) — generated rule
  authorship and footprint honesty.
- [ADR 0021](0021-public-optic-observation-boundary.md) — public WARP optic over
  internal observation.
- [ADR 0023](0023-admitted-executable-operation-packages.md) — admitted
  executable operation packages.
- [ADR 0025](0025-scheduler-owned-executable-operation-actions.md) —
  scheduler-owned executable-operation Actions.
- `docs/topics/GeneratedRules.md` — the stated absence of a false-footprint
  negative oracle.
- `docs/topics/WAL.md` — WAL truth boundary.
