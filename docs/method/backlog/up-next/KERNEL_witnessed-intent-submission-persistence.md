<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Witnessed Intent Submission Persistence

Status: follow-up implementation slice.

Depends on:

- WitnessedIntentSubmission v0.

## Why now

WitnessedIntentSubmission v0 records accepted application ingress as
deterministic Echo-owned submission history, but the first slice intentionally
does not build a durable restart/recovery substrate for accepted but not yet
ticked submissions.

Pending inbox memory alone is not the end state. Echo needs restart replay for
accepted submission history without giving application code tick authority or
making transport arrival semantic history.

## RED

Add a failing replay/recovery test:

- submit canonical application intent bytes;
- assert Echo records a witnessed submission and leaves the intent pending;
- snapshot or persist the runtime using the narrowest existing storage hook;
- reconstruct the runtime;
- assert the same submission id, generation, target head, ingress id, and
  pending ingress membership are restored;
- assert no tick, handler dispatch, scheduler work, or application mutation
  occurs during recovery.

## GREEN

Persist and restore witnessed submission records and pending ingress membership
through the existing runtime/provenance storage boundary, or add the smallest
new storage shell needed to make accepted-but-not-yet-ticked submissions
replayable after restart.

## Acceptance Criteria

- Accepted pending submissions survive restart with stable submission identity.
- Duplicate resubmission after restart returns duplicate posture, not a second
  semantic submission event.
- Submission recovery does not advance `GlobalTick` or `WorldlineTick`.
- Submission recovery does not call `SchedulerCoordinator::super_tick(...)`.
- Submission recovery does not dispatch installed handlers or execute contracts.
- Raw transport arrival order remains outside semantic Echo history.

## Non-goals

- Do not implement scheduler work candidates.
- Do not issue law witnesses or admission tickets.
- Do not add outcome subscriptions.
- Do not add QueryView.
- Do not add automatic retry.
