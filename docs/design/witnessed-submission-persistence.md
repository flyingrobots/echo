<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Witnessed Submission Persistence

Status: accepted and implemented local persistence shell.

## Claim

Echo can export and restore accepted witnessed submission material without
running the scheduler. The persistence image binds each witnessed submission
record to its canonical ingress envelope so a trusted host can recover
accepted-but-not-yet-ticked work after restart.

## Boundary

`WitnessedSubmissionPersistenceSnapshot` is a deterministic local image of
accepted submission records and canonical ingress envelopes. Restoring the image
does all of the following:

- restores witnessed submission identity;
- restores Echo-owned submission generation;
- restores canonical envelope material;
- preserves duplicate detection after restart.

Restoring the image does none of the following:

- does not enter envelopes into scheduler-visible head inboxes;
- does not stage ticketed runtime ingress;
- does not advance `GlobalTick` or `WorldlineTick`;
- does not dispatch handlers or execute contracts.

The host still owns durable storage. This slice supplies the smallest core
persistence shell needed for the host to persist accepted submission material.

## Invariants

- The envelope ingress id must match the persisted submission record.
- The envelope route must resolve to the persisted writer head.
- The target inbox policy must still accept the envelope.
- Invalid snapshots fail without partial import.
- Duplicate submission after restore returns duplicate posture, not a new
  semantic submission event.
- Recovery is not scheduler admission and does not grant application tick
  authority.

## Non-Goals

- Do not implement disk storage or a write-ahead log.
- Do not stage scheduler work during restore.
- Do not issue admission tickets, law witnesses, or receipt correlations.
- Do not retry or execute restored submissions automatically.

## Witnesses

- `cargo test -p warp-core --lib witnessed_submission_persistence`
