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
- Scheduler-owned tick receipts can be correlated back to ticketed runtime
  ingress records, admission ticket digests, and witnessed submission ids.
- Core can observe a witnessed submission as unknown, pending, or decided by a
  scheduler-owned tick receipt.
- Footprint conflicts are explicit receipt rejections, not hidden retries.
- Failed `SuperTick` attempts are failure-atomic: uncommitted runtime,
  provenance, and receipt-correlation writes are rolled back before any fault
  posture is recorded.
- Scoped internal scheduler faults quarantine the culprit writer head. Healthy
  unrelated heads remain eligible for later scheduler-owned ticks.
- Unscoped scheduler faults quarantine the runtime until trusted recovery.
- The optic admission ladder resolves through AdmissionTicket and currently
  can stage ticketed runtime ingress through an explicit runtime-owner authority
  token without ticking.

## What Is Not Yet True

- Accepted submissions are not yet complete witnessed ingress history.
- Clients cannot yet observe per-intent applied/rejected application semantics
  by id.
- Installed Wesley handler dispatch is not wired into scheduler-owned execution.
- Generic QueryView remains unsupported in core.

## Doctrine

Echo accepts intent submissions as witnessed ingress history.

Echo does not execute submissions synchronously.

Echo's trusted runtime owner controls tick boundaries.

A tick receipt witnesses the scheduler-owned decision.

A rejected candidate remains witnessed history.

Rollback is tick-local cleanup of an uncommitted failed scheduler transaction.

Quarantine is durable runtime posture after an internal fault.

Lawful rejection is not a fault.

Fault recovery is trusted runtime control, not application behavior.

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

1. WitnessedIntentSubmission.
2. SchedulerWorkCandidate.
3. LawWitness.
4. AdmissionTicket.
5. TicketedRuntimeIngress.
6. ReceiptCorrelation.
7. IntentOutcomeObservation.
8. InstalledContractHostDispatch.
9. ConflictPolicy / ExplicitRetry.
10. QueryViewObserverBridge.
11. Replay/DIND proof.

## Immediate Next Slice

InstalledContractHostDispatch should connect installed Wesley contract handlers
to scheduler-owned runtime execution without letting application dispatch call
handlers synchronously.

This slice must not implement QueryView, streaming subscriptions, automatic
retry, execution outside scheduler-owned ticks, or wall-clock cadence semantics.
