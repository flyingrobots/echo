<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# BEARING

Last updated: 2026-05-20.

This signpost summarizes current direction. It does not create commitments or
replace backlog items, design docs, retros, or CLI status. If it disagrees with
code, the code wins and this file should be corrected.

## Current Bearing

Echo already has deterministic execution; it does not yet have a continuous
witnessed intent pipeline into that execution.

The current priority is to finish the path from application ingress to
scheduler-owned tick outcome without giving application code tick authority.

## What Is Already True

- Echo has deterministic execution through `WorldlineRuntime`,
  `SchedulerCoordinator::super_tick(...)`, and `Engine::commit_with_state(...)`.
- Application-facing `dispatch_intent(...)` submits canonical EINT bytes; it does
  not tick the runtime.
- Trusted runtime control owns scheduler runs through the separate
  `TrustedKernelControlPort` boundary.
- Fixed logical timestep doctrine exists. Wall-clock cadence is host/runtime
  owner policy, not semantic Echo history.
- Tick receipts exist and witness scheduler-owned candidate outcomes.
- Footprint conflicts are explicit receipt rejections, not hidden retries.
- The optic admission ladder resolves through SchedulerAdmission v0 and
  currently stops before scheduler work.

## What Is Not Yet True

- Accepted submissions are not yet complete witnessed ingress history.
- Optic admission has no successful outcome.
- Scheduler work candidates do not exist.
- Law witnesses do not exist.
- Admission tickets do not exist.
- Ticketed runtime ingress is not wired.
- Tick receipts are not cleanly correlated to intent, submission, and ticket ids.
- Clients cannot observe intent outcome by id.
- Installed Wesley handler dispatch is not wired into scheduler-owned execution.
- Generic QueryView remains unsupported in core.

## Doctrine

Echo accepts intent submissions as witnessed ingress history.

Echo does not execute submissions synchronously.

Echo's trusted runtime owner controls tick boundaries.

A tick receipt witnesses the scheduler-owned decision.

A rejected candidate remains witnessed history.

Retry is a new explicit causal act.

AdmissionTicket is not execution.

TickReceipt is not AdmissionTicket.

QueryView waits until the write-side pipeline is credible.

Transport arrival is not semantic Echo history. Echo acceptance is semantic
ingress history.

Submission order may be witnessed. Submission order must not decide scheduler
order.

## Pipeline

Evidence phase:

```text
canonical EINT
-> witnessed submission
-> admission gates
-> scheduler work candidate
-> law witness
-> admission ticket
```

Runtime phase:

```text
admission ticket
-> ticketed runtime ingress
-> scheduler-owned tick
-> tick receipt
-> observable intent outcome
```

The hinge is:

```text
AdmissionTicket + witnessed submission -> ticketed runtime ingress
```

## Locked Sequence

1. WitnessedIntentSubmission v0.
2. SchedulerWorkCandidate v0.
3. LawWitness v0.
4. AdmissionTicket v0.
5. TicketedRuntimeIngress v0.
6. ReceiptCorrelation v0.
7. IntentOutcomeObservation v0.
8. InstalledContractHostDispatch v0.
9. ConflictPolicy / ExplicitRetry v0.
10. QueryViewObserverBridge v0.
11. Replay/DIND proof.

## Immediate Next Slice

WitnessedIntentSubmission v0 proves that accepted canonical application intent
bytes become witnessed Echo ingress history without running the scheduler.

This slice must not implement scheduler work candidates, law witnesses,
admission tickets, ticketed runtime ingress, receipt correlation, outcome
observation, installed handler dispatch, QueryView, streaming subscriptions,
automatic retry, or wall-clock cadence semantics.

If full restart persistence is too large for the first implementation, keep the
v0 ledger narrow and deterministic, then add a separate
WitnessedIntentSubmissionPersistence v0 slice. Pending inbox memory alone is not
the end state.
