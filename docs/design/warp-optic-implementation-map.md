<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# WARP Optic Implementation Map

Status: current doctrine map.
Scope: map the WARP optic paper vocabulary onto Echo's current implementation
without weakening Echo's scheduler authority boundary.

## Core Correction

Application-authored optics do not create ticks.

Application dispatch does not command ticks.

Authored surfaces may declare retained consequence obligations that the runtime
must satisfy, including receipt obligations. Only Echo's trusted
runtime/scheduler owns tick boundaries and `TickReceipt` emission.

The WARP paper's hot-text examples are useful for explaining retained
consequence obligations. They are not an Echo authority model. In Echo, an
authored contract can declare the receipt family and retention obligations that
must be satisfied when scheduler-owned execution admits and decides work. It
cannot create, schedule, or emit ticks.

## Paper-to-Echo Map

The paper's witness/shell vocabulary is intentionally abstract. Echo implements
the local contract-host seam with typed evidence objects. Those objects are not
interchangeable.

| WARP paper noun               | Echo implementation noun                                                                                                          |
| :---------------------------- | :-------------------------------------------------------------------------------------------------------------------------------- |
| `Ψ = (Ω, χ, ρ, Π, Λ)`         | Installed contract/admission/observation boundary.                                                                                |
| Observer plan `Ω`             | `AuthoredObserverPlan`, `ContractQueryObserver`, and `ReadingEnvelope` observer identity.                                         |
| Optic slice `χ`               | Basis, aperture, budget, footprint, and resolved query basis.                                                                     |
| Lowering `LowerΨ`             | Admission lowering for evidence phase; scheduler-owned tick lowering for runtime phase.                                           |
| Witness `W`                   | `LawWitness`, tick receipt evidence, reading-envelope evidence, and graph facts.                                                  |
| Retained shell `θ` / hologram | `AdmissionTicket`, `TickReceipt`, `ReadingEnvelope`, graph facts, retained readings, retained artifacts, and provenance payloads. |
| Commitment                    | Lawful admission or scheduler-owned tick decision, depending phase.                                                               |
| Revelation                    | `ObservationService`, `QueryView` observer routing, and `ReadingEnvelope` emission.                                               |
| Transport/import shell        | Future replica/settlement shell work; not the local contract-host seam.                                                           |

Do not search for one generic `Hologram` type and collapse these families into
it. The typed split is an implementation improvement: it makes the evidence
phase, runtime phase, and read/revelation phase auditable with different
objects.

## Echo Pipeline

The WARP paper uses commitment and lowering language broadly. Echo implements a
stricter local runtime pipeline:

```text
submission
-> admission evidence
-> ticketed runtime ingress
-> scheduler-owned tick
-> TickReceipt
```

`AdmissionTicket` is lawful admission evidence.

`TickReceipt` is scheduler-owned execution outcome evidence.

`ReadingEnvelope` is observer-relative read evidence.

The current evidence ladder is:

```text
Witnessed submission
-> Capability presentation classification / identity coverage
-> BasisResolution
-> ApertureResolution
-> BudgetResolution
-> RuntimeSupport
-> InvocationAdmission
-> SchedulerAdmission
-> SchedulerWorkCandidate
-> LawWitness
-> AdmissionTicket
```

`LawWitness` precedes and is bound by `AdmissionTicket`. A ticket without the
witness is not lawful admission evidence.

The hinge from evidence into runtime is:

```text
AdmissionTicket + witnessed submission -> ticketed runtime ingress
```

Ticketed runtime ingress is not a tick. It stages admitted work for the
trusted runtime owner.

## Compiler Seam

The WARP paper's application boundary is now partially implemented in Echo:
Wesley emits application request helpers, mutation host helpers, and query
observer host helpers. Echo core remains generic.

Separate the surfaces:

| Surface               | Responsibility                                                             |
| :-------------------- | :------------------------------------------------------------------------- |
| Application helpers   | Build canonical intent and query requests from authored contract nouns.    |
| Contract-host helpers | Install generated mutation handlers and read-only query observers.         |
| Echo core runtime     | Admit, schedule, observe, and witness without importing application nouns. |

Generated helper names may be GraphQL-shaped because they belong to the
authored contract. `warp-core` must not grow those application nouns.

## QueryView Status

`QueryView`/`Query` routes to installed contract query observers when a
matching observer is registered. `echo-wesley-gen --contract-host` emits query
observer host helpers. `UNSUPPORTED_QUERY` means no installed observer supports
the requested query id.

This does not implement the full observer-rights or revelation lattice from
the paper. Current observers are bounded, read-only contract-host seams.

Generated query observer helpers receive read-only observer context:

- no mutable runtime;
- no scheduler control;
- no `TickDelta`;
- no state mutation authority;
- no tick authority.

## Outcome Algebra

The paper's broad lawful outcome algebra is:

```text
Derived | Plural | Conflict | Obstruction
```

Echo's local `TickReceipt` entries currently realize a narrower tick-scale
shape:

```text
Applied / Rejected(FootprintConflict)
```

Conflict rejection is final for that tick attempt. Retry is a new explicit
causal act. There is no hidden retry queue.

Admission obstructions happen before ticketed scheduler work. Internal runtime
faults are not normal receipt dispositions; they roll back the failed scheduler
attempt and enter runtime-local quarantine posture outside the `TickReceipt`
path.

Do not implement broad `Plural` support merely to mirror the paper. Treat it as
broader WARP algebra and future braid/replica-scale work until an executable
claim requires it.

## Time And Tick Authority

Echo has fixed logical ticks and trusted runtime-owned scheduler control. A
host may run Echo on a fixed wall-clock cadence, but wall-clock frequency is
host/runtime-owner policy, not application authority and not semantic history.

Application-authored optics may declare retained tick/receipt obligations.
They do not own tick cadence, tick membership, or receipt emission.

## Transport And Replica Scope

Current Echo implements the local contract-host read/write seam. It does not
yet implement full replica transport/import optics, settlement shells,
adversarial transport, or idempotent import of already-adjudicated outcomes.

Local ticketed runtime ingress is not replica transport. Replica transport must
carry authorship evidence, comparable basis, witness material, idempotence
keys, and settlement/import law before it can be treated as admitted local
history.

## Fault Quarantine

Current scheduler fault quarantine is runtime-local posture. Durable
control-plane/provenance fault evidence remains future work.

Do not call runtime-local quarantine durable fault evidence until that boundary
actually exists.

## Continuum And Runtime Position

Continuum is the protocol-shaped causal medium.

Echo is a concrete deterministic WARP runtime implementation for that medium.

Avoid saying Echo is the primary runtime of Continuum. Also avoid weakening
Echo into an application framework. Echo owns local deterministic runtime
responsibilities while speaking the broader Continuum causal vocabulary.

## Future Policy Scope

Ephemeral Scratch, Author-Only Speculative Lane, and Shared/Admitted Lane are
paper-level privacy/runtime policy concepts. Echo has worldlines, heads,
admission, quarantine, and observation, but the current local contract-host
pipeline does not implement the full social lane model.

Do not force speculative or social lane policy into the installed contract
registry boundary.

## Guardrails

Implementation improvements over the paper examples that must be preserved:

- Application optics do not create ticks.
- Application dispatch does not execute synchronously.
- Application dispatch does not command ticks.
- Authored optics declare retained consequence obligations; Echo satisfies
  them through trusted runtime execution.
- `AdmissionTicket` is distinct from `TickReceipt`.
- `AdmissionTicket` is not execution.
- `LawWitness` precedes and is bound by `AdmissionTicket`.
- Query observers are read-only.
- `QueryView` bridge and Wesley query observer helpers exist.
- Fault quarantine is runtime-local unless durable evidence is explicitly
  added.
- Conflict rejection is final for that tick attempt; retry is a new causal act.

## Next Code Slice

The next code slice is the Installed Contract Registry Boundary:

```text
schema identity
+ artifact identity
+ codec identity
+ supported operation ids
+ mutation handlers
+ query observers
+ observer plan identities
+ contract package/version identity
-> one installed contract package boundary
```

Unsupported operations should be rejected at this package boundary before they
become runtime-visible work or accepted reads.
