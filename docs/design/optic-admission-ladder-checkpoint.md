<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Optic Admission Ladder Checkpoint

Status: current admission ladder checkpoint.
Scope: optic invocation admission through `AdmissionTicket`, plus the boundary
between admission evidence and runtime-owned execution.

## Doctrine

Echo's optic admission ladder now reaches lawful admission evidence. It can
positively resolve the narrow Echo-owned fixture gates through
SchedulerWorkCandidate and LawWitness, then issue an `OpticAdmissionTicket`.

That ticket is not execution.

An `AdmissionTicket` is not a `TickReceipt`, not scheduler work enqueueing, not
handler dispatch, and not application state mutation. It is the evidence that
Echo lawfully admitted an invocation for later runtime handling.

Ticketed runtime ingress is the runtime-phase hinge:

```text
AdmissionTicket + witnessed submission -> ticketed runtime ingress
```

Ticketed runtime ingress still does not give application code tick authority.
The trusted runtime owner decides scheduler cadence and tick boundaries.

Application-authored optics may declare retained receipt obligations. They do
not create ticks, create `TickReceipt` values, or command scheduler cadence.

## Pipeline Context

Before this registry ladder matters, Echo may accept canonical application
intent bytes as witnessed submission ingress. That submission ledger belongs to
the end-to-end intent pipeline; it is not caller-supplied admission evidence and
not a gate inside `OpticArtifactRegistry` invocation admission.

## Current Registry Ladder

The current registry admission path evaluates gates in this order:

1. Resolve the artifact handle internally.
2. Reject an unknown handle.
3. Reject an operation mismatch.
4. Require basis request presence.
5. Require aperture request presence.
6. Require budget request presence.
7. Classify capability presentation posture.
8. Optionally publish grant-validation obstruction evidence.
9. If capability validation returns identity-covered material, resolve the
   narrow BasisResolution fixture or obstruct unsupported basis material.
10. If that basis fixture resolves, resolve the narrow ApertureResolution
    fixture or obstruct unsupported aperture material.
11. If that aperture fixture resolves, resolve the narrow BudgetResolution
    fixture or obstruct unsupported budget material.
12. If that budget fixture resolves, check Echo-owned RuntimeSupport facts for
    the registered requirements or obstruct at `RuntimeSupportUnavailable`.
13. If RuntimeSupport resolves, check Echo-owned InvocationAdmission facts for
    the registered artifact handle or obstruct at
    `InvocationAdmissionUnavailable`.
14. If InvocationAdmission resolves, check Echo-owned SchedulerAdmission facts
    for the registered artifact handle or obstruct at
    `SchedulerAdmissionUnavailable`.
15. If SchedulerAdmission resolves, check Echo-owned SchedulerWorkCandidate
    facts for the registered artifact handle or obstruct at
    `SchedulerWorkUnavailable`.
16. If SchedulerWorkCandidate resolves, check Echo-owned LawWitness facts for
    the registered artifact handle or obstruct at `LawWitnessUnavailable`.
17. If LawWitness resolves, issue an `OpticAdmissionTicket`.
18. Publish either obstruction evidence or `AdmissionTicketIssued` evidence.

Presence checks come before resolution checks. Each resolved gate only advances
to the next gate. No caller-owned field can supply runtime support, invocation
admission, scheduler admission, scheduler work candidate, law witness, or
admission ticket evidence.

## Fixture Labels

The current narrow fixture byte strings are code facts. The conceptual boundary
names do not need version suffixes.

```text
basis-request:resolved-fixture
aperture-request:resolved-fixture
budget-request:resolved-fixture
runtime-support:resolved-fixture
invocation-admission:resolved-fixture
scheduler-admission:resolved-fixture
scheduler-work-candidate:resolved-fixture
law-witness:resolved-fixture
```

## Obstruction Reachability

| Obstruction                       | Reachability    | Meaning                                                                                       |
| :-------------------------------- | :-------------- | :-------------------------------------------------------------------------------------------- |
| `UnknownHandle`                   | Reachable today | Echo cannot resolve the runtime-local artifact handle.                                        |
| `OperationMismatch`               | Reachable today | The invocation operation does not match registered artifact metadata.                         |
| `MissingBasisRequest`             | Reachable today | The caller did not provide basis request material.                                            |
| `MissingApertureRequest`          | Reachable today | Basis material is present, but aperture request material is absent.                           |
| `MissingBudgetRequest`            | Reachable today | Basis and aperture material are present, but budget request material is absent.               |
| `MissingCapability`               | Reachable today | Required invocation context is present, but no capability presentation was supplied.          |
| `MalformedCapabilityPresentation` | Reachable today | Capability presentation material is present but not structurally usable.                      |
| `UnboundCapabilityPresentation`   | Reachable today | Capability presentation material is structurally usable but not bound to the invocation.      |
| `CapabilityValidationUnavailable` | Reachable today | A bound presentation exists, but no successful validation or admission has occurred yet.      |
| `UnsupportedBasisResolution`      | Reachable today | Identity-covered material reaches the basis boundary, but the basis shape is unsupported.     |
| `UnsupportedApertureResolution`   | Reachable today | Basis resolution succeeded, but the aperture shape is unsupported.                            |
| `UnsupportedBudgetResolution`     | Reachable today | Aperture resolution succeeded, but the budget shape is unsupported.                           |
| `RuntimeSupportUnavailable`       | Reachable today | Budget resolution succeeded, but Echo has no runtime support fact for the requirements.       |
| `InvocationAdmissionUnavailable`  | Reachable today | Runtime support succeeded, but Echo has no invocation admission fact for the artifact handle. |
| `SchedulerAdmissionUnavailable`   | Reachable today | Invocation admission succeeded, but Echo has no scheduler admission fact for the handle.      |
| `SchedulerWorkUnavailable`        | Reachable today | Scheduler admission succeeded, but Echo has no scheduler work candidate fact for the handle.  |
| `LawWitnessUnavailable`           | Reachable today | Scheduler work candidate succeeded, but Echo has no law witness fact for the handle.          |
| `AdmissionTicketUnavailable`      | Defensive       | Law witness evidence was expected but not available while issuing the ticket.                 |

`AdmissionTicketUnavailable` is defensive. The normal current path either
obstructs at `LawWitnessUnavailable` or issues a ticket once a law witness fact
is present.

## Admission Ticket Boundary

A successful `OpticAdmissionTicket` binds:

- the Echo-owned artifact handle;
- the registered artifact hash;
- the requested operation id;
- the registered requirements digest;
- canonical invocation variable bytes by digest;
- basis, aperture, and budget request digests;
- the Echo-owned law witness digest;
- the deterministic ticket digest.

The ticket proves lawful admission. It does not prove that a scheduler tick ran.
It does not prove that a handler dispatched. It does not prove an application
state effect.

## Runtime Phase Boundary

After admission, ticketed runtime ingress may stage a witnessed submission into
runtime ingress through explicit runtime-owner authority.

That stage is still pre-tick:

```text
AdmissionTicket
-> ticketed runtime ingress
-> scheduler-owned tick later
-> TickReceipt later
-> observable intent outcome later
```

Application dispatch remains ingress evidence. It is not a synchronous domain
RPC and it does not run the scheduler.

## Current Non-Behavior

This checkpoint does not introduce:

- scheduler work enqueueing as a completed execution unit;
- generated or installed handler dispatch;
- contract execution;
- application-controlled ticks;
- hidden retry;
- wall-clock scheduler authority;
- caller-supplied runtime support testimony;
- caller-supplied invocation admission testimony;
- caller-supplied scheduler admission testimony;
- caller-supplied scheduler work testimony;
- caller-supplied law witness testimony;
- caller-issued admission tickets;
- budget reservation.

## Tripwires

If a future slice treats `AdmissionTicket` as execution, the ladder is wrong.

If a future slice lets application dispatch choose tick boundaries, the ladder
is wrong.

If a future slice makes scheduler work or law witness evidence caller-supplied,
the ladder is wrong.

If a future slice makes `TickReceipt` interchangeable with `AdmissionTicket`,
the ladder is wrong.

If a future slice bypasses ticketed runtime ingress for installed contract
execution, the ladder is wrong.

## Next Architectural Slice

The next package-level boundary is an installed contract registry surface that
binds schema hash, artifact hash, codec identity, supported operation ids,
mutation handler rules, and query observers before contract operations become
runtime-visible work or reads.
