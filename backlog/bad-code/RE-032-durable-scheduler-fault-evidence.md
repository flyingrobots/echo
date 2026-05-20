<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# RE-032: Publish Durable Scheduler Fault Evidence

## Problem

`WorldlineRuntime` now quarantines scoped scheduler head faults and runtime-wide
faults through runtime-local control posture. That prevents automatic retry
loops after rollback, but the fault records are not yet published into durable
control-plane/provenance history.

## Why It Matters

Fault quarantine changes future scheduling behavior. If Echo reconstructs a
runtime from committed history without replaying fault evidence, a previously
faulted head could become runnable again without trusted recovery. That would
violate the scheduler safety doctrine.

## Desired Shape

- Publish `SchedulerFaultRecorded` and `SchedulerFaultResolved` evidence through
  an Echo-owned durable control-plane or provenance fact boundary.
- Replay fault posture when reconstructing runtime scheduler state.
- Keep fault evidence out of application history and tick receipts.
- Preserve the current rule that application-facing code cannot resolve or
  clear scheduler faults.

## Acceptance Tests

- `scheduler_fault_record_replays_head_quarantine`
- `scheduler_fault_resolution_replays_trusted_recovery`
- `runtime_fault_record_replays_global_quarantine`
- `fault_evidence_is_control_plane_not_application_history`
