<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Witnessed Intent Submission Persistence

Status: implemented local persistence shell; durable host storage remains.

Depends on:

- Local witnessed submission replay shape from the installed intent pipeline.

## Why now

WitnessedIntentSubmission v0 records accepted application ingress as
deterministic Echo-owned submission history, but the first slice intentionally
does not build a durable restart/recovery substrate for accepted but not yet
ticked submissions.

Pending inbox memory alone is not the end state. Echo needs restart replay for
accepted submission history without giving application code tick authority or
making transport arrival semantic history.

Crash/restart behavior must recover to one of these states:

- submission was not accepted;
- submission was accepted and remains pending;
- submission was accepted and later decided by a tick receipt.

It must not recover to a half-accepted, uncorrelatable state.

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

Persist and restore witnessed submission records plus canonical ingress envelope
material through the smallest core storage shell needed to make accepted-but-
not-yet-ticked submissions recoverable after restart. The host still owns the
durable storage medium.

Implemented local shell:

- `WitnessedSubmissionPersistenceSnapshot` exports accepted submission records
  in deterministic replay order.
- Each persistence record carries the canonical ingress envelope.
- Restore validates ingress id, resolved head, and inbox policy before import.
- Restore records semantic submission history and envelope material without
  entering scheduler-visible inboxes.

## Acceptance Criteria

- Accepted pending submissions survive restart with stable submission identity.
- Duplicate resubmission after restart returns duplicate posture, not a second
  semantic submission event.
- Submission recovery does not advance `GlobalTick` or `WorldlineTick`.
- Submission recovery does not call `SchedulerCoordinator::super_tick(...)`.
- Submission recovery does not dispatch installed handlers or execute contracts.
- Raw transport arrival order remains outside semantic Echo history.
- Invalid persistence images fail without partial import.

## Non-goals

- Do not implement scheduler work candidates.
- Do not issue law witnesses or admission tickets.
- Do not add outcome subscriptions.
- Do not add QueryView.
- Do not add automatic retry.
