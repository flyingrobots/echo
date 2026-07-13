<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Witnessed Submission Persistence

Status: accepted and implemented through the trusted runtime WAL.

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

The host-owned WAL commits the versioned retained envelope in the same
transaction as acceptance evidence. Filesystem reopen rebuilds the persistence
snapshot and restores it before exposing the configured WAL to the host.

## Invariants

- The envelope ingress id must match the persisted submission record.
- The envelope route must resolve to the persisted writer head.
- The target inbox policy must still accept the envelope.
- Snapshot export fails if any witnessed submission lacks canonical envelope
  material; replay-only records must not silently disappear from persisted
  state.
- Invalid snapshots fail without partial import.
- Duplicate submission after restore returns duplicate posture, not a new
  semantic submission event.
- Recovery is not scheduler admission and does not grant application tick
  authority.
- Recovery certificates bind retained envelope material and report missing
  envelope records as obstructions.

## Non-Goals

- Do not stage scheduler work during restore.
- Do not issue admission tickets, law witnesses, or receipt correlations.
- Do not retry or execute restored submissions automatically.
- Do not claim that restoring accepted envelopes also restores decided runtime
  state; replayable tick deltas and provenance remain a separate boundary.

## Witnesses

- `cargo test -p warp-core --lib witnessed_submission_persistence`
- `cargo test -p warp-core --test ingress_retention_codec_tests`
- `cargo test -p warp-core --features native_rule_bootstrap,trusted_runtime,host_test --test trusted_runtime_host_loop_tests filesystem_runtime_wal_restores_witnessed_submission_material_after_restart`
