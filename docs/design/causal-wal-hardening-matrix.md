<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Causal WAL Hardening Matrix

Status: active plan.

This packet is the no-count planning slice for the WAL hardening phase after
the causal WAL foundation and filesystem recovery gates. The goal is not to
expand Echo's WAL scope. The goal is to make the existing WAL truth boundary
harder to fool.

Doctrine:

```text
Echo may only claim what its WAL can recover.
No durable claim without durable evidence.
No visible outcome without committed history.
No external effect without committed authorization.
```

The twenty slices below focus on adversarial recovery witnesses, reusable crash
fixtures, disk-level corruption fixtures, semantic validation negatives, and
operator/debugger evidence surfaces. They intentionally do not implement WSC,
distributed WALs, multi-writer replication, user undo, or application-specific
editing semantics.

## Fixture Surface

The first implementation priority is a reusable fixture surface for hostile WAL
tests. The fixture surface should let tests assemble repeatable histories and
then damage them at exact boundaries.

Required fixture capabilities:

- create deterministic temp WAL roots;
- open a strict filesystem WAL store;
- append committed submission, tick, checkpoint, retained-reading, and outbox
  transactions;
- append uncommitted frames without commit markers;
- truncate the active segment at named byte boundaries;
- corrupt record kind, magic, payload, and digest bytes;
- recover read-only and writable reports;
- run doctor/inspection without mutating storage;
- rebuild submission, receipt, retention, outbox, and commit-evidence indexes;
- compare live state to recovered replay state;
- report crash scenario name, expected posture, actual posture, and first
  mismatch.

The first fixture implementation can live in `warp-core` integration tests.
If the fixture becomes useful outside Rust tests, promote the same shape to a
CLI/BATS crash runner later. The runner should kill or interrupt Echo at named
boundaries only after the Rust fixture semantics are stable.

## Test Style

Hardening tests should be ugly and specific. Prefer names that encode the
failure boundary:

```text
crash_after_submission_commit_before_ack_retry_returns_duplicate_posture
read_only_recovery_reports_uncommitted_tail_without_truncating
corrupt_committed_record_digest_blocks_recovery
```

Each test should prove one claim. If a fixture helper is needed, add the helper
first and then add the narrow witness that justifies it.

## No-Count Planning Slice

- [x] Write this hardening plan.
- [x] Link the plan from `docs/BEARING.md`.
- [x] Define slices 46-65 with user stories, acceptance criteria, and test
      plans.
- [ ] Keep slice checkboxes updated in `docs/BEARING.md` before committing each
      completed slice.

## Slice 46: WAL Hardening Fixture Surface

User story:

As an Echo maintainer, I need a deterministic fixture surface that can construct
valid WAL histories and damage them at exact byte/transaction boundaries.

Acceptance criteria:

- A new hardening test fixture can create deterministic filesystem WAL roots.
- The fixture can append committed submissions and append uncommitted tails.
- The fixture can truncate active segment files without going through recovery.
- The fixture can recover read-only and writable reports.
- The fixture names every crash/corruption scenario in assertion failures.

Test plan:

- `hardening_fixture_recovers_committed_submission`
- `hardening_fixture_appends_uncommitted_tail`
- `hardening_fixture_truncates_segment_for_torn_tail`
- `hardening_fixture_read_only_recovery_does_not_mutate_segment`

## Slice 47: WAL Recovery Golden Corpus

User story:

As Echo, I need a fixed corpus of minimal WAL shapes that proves recovery
posture across clean, partial, and corrupt histories.

Acceptance criteria:

- Golden fixtures cover clean committed segment, empty WAL, uncommitted tail,
  torn record, corrupt digest, bad magic, and unknown record kind.
- Each corpus case asserts transaction count, first/last committed LSN, tail
  posture, and whether recovery blocks or remains inspectable.
- Corpus helpers are deterministic and do not depend on wall clock.

Test plan:

- `wal_recovery_golden_clean_committed_segment`
- `wal_recovery_golden_empty_store`
- `wal_recovery_golden_uncommitted_tail`
- `wal_recovery_golden_torn_record`
- `wal_recovery_golden_corrupt_digest`
- `wal_recovery_golden_bad_magic`
- `wal_recovery_golden_unknown_record_kind`

## Slice 48: Submission ACK Crash Matrix

User story:

As an application retrying after a crash, I need Echo to distinguish never
accepted from accepted-before-ACK.

Acceptance criteria:

- Crash points around submission intake are modeled explicitly.
- Recovery never invents accepted evidence before commit.
- Retry after commit-before-ACK returns stable duplicate posture.
- Same submission id with a different envelope is a protocol violation.
- New submission id with the same envelope is a new submission unless an
  explicit policy says otherwise.

Test plan:

- `crash_before_submission_commit_recovers_not_accepted`
- `crash_after_submission_commit_before_ack_recovers_pending`
- `crash_after_submission_commit_before_ack_retry_returns_duplicate_posture`
- `crash_after_submission_commit_before_ack_different_envelope_is_protocol_violation`
- `new_submission_id_same_envelope_is_not_duplicate_without_policy`

## Slice 49: Tick Commit And Publish Crash Matrix

User story:

As Echo, visible tick outcomes must imply committed recoverable history.

Acceptance criteria:

- Tick receipt, runtime delta, receipt correlation, and index publication
  boundaries are tested separately.
- Commit-before-publish recovers receipts and rebuilds indexes.
- Publish-before-commit is impossible or rejected by the fixture.
- Failed/uncommitted tick attempt does not advance frontier or become history.

Test plan:

- `crash_before_tick_commit_recovers_no_receipt`
- `crash_after_tick_commit_before_publish_rebuilds_receipt_indexes`
- `tick_publish_requires_committed_transaction`
- `uncommitted_tick_tail_does_not_advance_frontier`
- `failed_tick_tail_does_not_create_success_receipt`

## Slice 50: Segment Corruption Matrix

User story:

As recovery, segment corruption must produce deterministic posture instead of
panic, silent skip, or partial history.

Acceptance criteria:

- Torn header, torn payload, torn digest, bad magic, corrupt digest, unknown disk
  kind, segment gap, and duplicate segment cases are tested.
- Read-only recovery never mutates files.
- Writable recovery only truncates incomplete tails.
- Corrupt committed records block recovery rather than being skipped.

Test plan:

- `torn_segment_header_reports_tail`
- `torn_segment_payload_reports_tail`
- `torn_segment_digest_reports_tail`
- `bad_segment_magic_blocks_recovery`
- `corrupt_committed_record_digest_blocks_recovery`
- `unknown_disk_record_kind_blocks_recovery`
- `segment_gap_blocks_or_obstructs_recovery`
- `duplicate_segment_id_blocks_recovery`

## Slice 51: Writer Epoch Fencing Matrix

User story:

As Echo, recovery must detect split-writer evidence instead of merging
conflicting histories.

Acceptance criteria:

- Writer epoch metadata includes fencing evidence in all strict fixtures.
- Overlapping epochs block recovery.
- Unknown previous epoch blocks new writer acquisition.
- Epoch closure and next-epoch continuity are validated.
- Fencing token mismatch is a recovery fault, not a warning.

Test plan:

- `overlapping_writer_epochs_block_recovery`
- `unknown_previous_writer_epoch_rejected`
- `writer_epoch_chain_requires_previous_final_commit_digest`
- `writer_epoch_fencing_token_mismatch_blocks_recovery`
- `closed_epoch_allows_next_epoch_with_matching_fence_evidence`

## Slice 52: Transaction Contiguity And Commit Semantics

User story:

As Echo, only complete contiguous committed WAL transactions become history.

Acceptance criteria:

- Transaction frames must be contiguous for the current WAL version.
- Commit marker binds first LSN, last LSN, record count, and records root.
- Per-record names never imply commit.
- Interleaved transactions are rejected.

Test plan:

- `interleaved_transactions_rejected`
- `commit_record_count_mismatch_rejected`
- `commit_lsn_range_gap_rejected`
- `commit_records_root_mismatch_rejected`
- `record_kind_name_does_not_imply_history_before_commit`

## Slice 53: Semantic Validator Negative Cases

User story:

As Echo, byte-valid WAL transactions must still be rejected if they violate
runtime law.

Acceptance criteria:

- Digest-valid but semantically invalid transactions are rejected.
- Submission acceptance cannot include scheduler-owned records.
- Tick transaction requires trusted scheduler authority.
- Runtime-control records require runtime authority.
- Frontier transition kind must match the transaction kind.

Test plan:

- `byte_valid_submission_with_tick_record_rejected`
- `byte_valid_tick_without_scheduler_authority_rejected`
- `runtime_control_record_without_runtime_authority_rejected`
- `frontier_transition_kind_mismatch_rejected`
- `submission_transaction_with_runtime_posture_record_rejected`

## Slice 54: Checkpoint Crash Matrix

User story:

As recovery, checkpoints must accelerate replay without creating or erasing
history.

Acceptance criteria:

- Crash before checkpoint rename leaves no usable checkpoint.
- Crash after checkpoint rename before publication can use the checkpoint if it
  validates against committed WAL.
- Published checkpoint with missing material obstructs precisely.
- Corrupt latest checkpoint falls back or blocks according to documented scope.

Test plan:

- `crash_before_checkpoint_rename_uses_prior_checkpoint_or_full_replay`
- `valid_checkpoint_without_publication_record_can_be_used`
- `published_checkpoint_missing_material_obstructs`
- `corrupt_latest_checkpoint_falls_back_to_prior_valid_checkpoint`
- `checkpoint_ahead_of_wal_chain_is_rejected`

## Slice 55: Retained Material Before Reference Matrix

User story:

As Echo, committed references must not point at unavailable retained material
without typed obstruction.

Acceptance criteria:

- Missing submission payload, receipt material, state delta, reading envelope,
  and checkpoint material map to documented scope.
- Missing diagnostic-only material does not block causal recovery.
- Missing state-delta material blocks global recovery if needed to reconstruct
  frontier.
- Missing reading material returns obstruction, not empty success.

Test plan:

- `missing_submission_payload_recovers_submission_obstruction`
- `missing_tick_receipt_material_recovers_receipt_obstruction`
- `missing_tick_state_delta_blocks_frontier_recovery`
- `missing_reading_material_returns_obstruction`
- `missing_diagnostic_material_does_not_block_recovery`

## Slice 56: Side-Effect Outbox Crash Matrix

User story:

As Echo, external effects must never escape before committed authorization and
must be idempotent after crash.

Acceptance criteria:

- External effect cannot run without committed outbox intent.
- Existing artifact is detected before retry.
- Existing mismatched artifact obstructs.
- Observation commit records already-performed effect.
- Crash after effect before observation recovers as existing artifact match, not
  blind retry.

Test plan:

- `external_effect_requires_committed_outbox_authorization`
- `crash_after_effect_before_observation_detects_existing_artifact`
- `existing_artifact_digest_mismatch_obstructs`
- `materialization_observation_marks_effect_already_observed`
- `outbox_replay_uses_idempotency_token`

## Slice 57: Recovery Reducer Determinism

User story:

As Echo, replay must apply committed facts without scheduler callbacks, wall
clock, random, network, or app code.

Acceptance criteria:

- Recovery reducer is tested as a pure function.
- Replaying the same committed transactions twice yields identical roots.
- Different transaction order rejects or yields a different validated chain; it
  never silently normalizes.
- Recovery does not require scheduler or app callback surfaces.

Test plan:

- `pure_replay_same_transactions_same_roots`
- `pure_replay_order_is_commit_chain_order`
- `pure_replay_rejects_frontier_mismatch`
- `recovery_reducer_does_not_require_scheduler`
- `recovery_reducer_does_not_require_app_callbacks`

## Slice 58: Shadow Replay Harness

User story:

As a maintainer, every mutating WAL path should prove live state equals
recovered state.

Acceptance criteria:

- Add a reusable helper that runs a live scenario, recovers from WAL, and
  compares roots/indexes/receipts/readings.
- Integrate the helper into selected WAL integration tests.
- Divergence reports enough context to diagnose the first mismatch.
- Harness remains deterministic and service-free.

Test plan:

- `shadow_replay_submission_path_matches_live`
- `shadow_replay_tick_path_matches_live`
- `shadow_replay_retention_path_matches_live`
- `shadow_replay_outbox_path_matches_live`
- `shadow_replay_reports_first_mismatch`

## Slice 59: Causal Commit Evidence Projection Matrix

User story:

As [warp-ttd] or an operator, I need commit evidence posture without raw WAL
ownership.

Acceptance criteria:

- Echo projects causal commit evidence for accepted pending, decided applied,
  decided rejected, obstructed, and recovery-faulted cases.
- Projection exposes commit anchor, durability mode, LSN, transaction id, writer
  epoch, and digest.
- Projection does not require raw segment paths or recovery authority.
- Absent durability evidence is explicit.

Test plan:

- `commit_evidence_projects_accepted_pending`
- `commit_evidence_projects_decided_applied`
- `commit_evidence_projects_decided_rejected`
- `commit_evidence_projects_obstructed`
- `commit_evidence_absent_is_explicit`

## Slice 60: WAL Doctor And Inspector Contract Tests

User story:

As an operator, I need inspection commands/read models to report truth without
mutating storage.

Acceptance criteria:

- Read-only doctor reports clean, would-truncate, obstructed, corrupt, and
  missing-material postures.
- Doctor never truncates or rewrites files.
- JSON/report shape has stable field names for agents.
- Recovery certificate fields are present and deterministic.

Test plan:

- `wal_doctor_clean_report_is_stable`
- `wal_doctor_would_truncate_does_not_mutate`
- `wal_doctor_corrupt_committed_record_reports_obstructed`
- `wal_doctor_missing_material_reports_obstruction`
- `recovery_certificate_has_stable_json_shape`

## Slice 61: Crashpoint Runner Contract

User story:

As Echo, I need a future CLI/BATS crash runner contract that mirrors the Rust
fixture semantics before it shells out to real processes.

Acceptance criteria:

- Rust crash fixtures define canonical crashpoint names.
- A test-visible crashpoint manifest lists supported boundaries.
- The manifest distinguishes simulated in-process cuts from future process-kill
  cuts.
- No CLI runner claims more than the Rust fixture proves.

Test plan:

- `crashpoint_manifest_lists_submission_boundaries`
- `crashpoint_manifest_lists_tick_boundaries`
- `crashpoint_manifest_lists_checkpoint_boundaries`
- `crashpoint_manifest_marks_process_kill_as_future_until_runner_exists`

## Slice 62: Filesystem Strict Sync Evidence

User story:

As Echo, strict filesystem mode must make sync boundaries inspectable enough for
tests to prove ACK ordering.

Acceptance criteria:

- Tests prove commit flushing syncs the WAL file before the caller can observe
  accepted evidence.
- Segment creation and manifest/checkpoint rename sync the containing
  directory.
- The filesystem adapter does not claim strict durability if any required sync
  step is bypassed.

Test plan:

- `filesystem_commit_flush_is_ack_boundary`
- `filesystem_segment_creation_syncs_directory`
- `filesystem_manifest_rename_syncs_directory`
- `filesystem_checkpoint_rename_syncs_directory`
- `filesystem_strict_mode_rejects_missing_sync_evidence`

## Slice 63: Object-Store Manifest Negative Matrix

User story:

As Echo, strict object-store mode must reject adapters that cannot prove
conditional manifest semantics.

Acceptance criteria:

- Every missing capability has a distinct validation error.
- Read-after-write uncertainty blocks strict mode.
- Conditional manifest commit is modeled as compare-and-swap, not overwrite.
- Object-store strict validation remains mechanism-neutral and app-noun-free.

Test plan:

- `strict_object_store_requires_content_addressed_objects`
- `strict_object_store_requires_object_version_verification`
- `strict_object_store_requires_conditional_manifest_commit`
- `strict_object_store_requires_verified_read_after_write`
- `strict_object_store_rejects_unconditional_manifest_overwrite`

## Slice 64: Security And Redaction Posture Matrix

User story:

As Echo, recovery and inspection must distinguish missing material from
policy-hidden or encrypted material.

Acceptance criteria:

- Postures include present, redacted by policy, encrypted-key-unavailable,
  missing, corrupt, and obstructed.
- Inspector output distinguishes unavailable-by-policy from missing-by-corrupt
  storage.
- Redaction posture does not become success, empty payload, or silent skip.

Test plan:

- `redacted_material_is_policy_posture_not_missing`
- `encrypted_key_unavailable_is_policy_posture_not_corruption`
- `missing_material_is_not_redaction`
- `corrupt_material_is_not_redaction`
- `inspector_reports_redaction_posture_explicitly`

## Slice 65: WAL Hardening Release Gate

User story:

As Echo, I need one gate that tells us whether the WAL is trustworthy enough for
the next real-app persistence push.

Acceptance criteria:

- A readiness check aggregates all hardening witnesses.
- Gate reports blocked and passed categories.
- Gate includes app-noun guard, shadow replay, crash matrix, doctor, outbox,
  semantic validator, filesystem, object-store, and projection coverage.
- `docs/BEARING.md` marks exactly what is complete and what remains future.

Test plan:

- `wal_hardening_gate_reports_blocked_categories`
- `wal_hardening_gate_passes_when_all_witnesses_green`
- `wal_hardening_gate_includes_app_noun_guard`
- `wal_hardening_gate_includes_crashpoint_fixture_surface`
- targeted WAL hardening suite plus app-noun guard and doc checks.

## Slice 66: Canonical Segment Namespace

User story:

As Echo, WAL segment files need a deterministic logical namespace that is not
derived from wall-clock time.

Acceptance criteria:

- New filesystem WAL roots write segments under a `segments/` directory.
- Creating the segment namespace syncs the WAL root directory in strict
  filesystem mode.
- Segment file names are derived only from logical `WalSegmentId`.
- Recovery treats logical segment id as the ordering authority.
- Date or hour partitions are not part of causal recovery truth.

Test plan:

- `canonical_segment_path_uses_logical_segments_directory`
- `filesystem_segment_namespace_creation_syncs_root_directory`
- `recovery_scans_canonical_segments_directory`

## Slice 67: Segment Placement Policy Guard

User story:

As Echo, operators may want wall-clock folders for operational convenience, but
those folders must never become authoritative causal ordering.

Acceptance criteria:

- Authoritative wall-clock placement is rejected.
- Non-authoritative wall-clock placement may be represented as an adapter-local
  layout hint.
- Segment recovery remains manifest/id based, not path-time based.

Test plan:

- `wall_clock_segment_placement_cannot_be_authoritative`
- `wall_clock_segment_placement_may_be_non_authoritative`

## Slice 68: Legacy Flat Segment Compatibility

User story:

As Echo, existing first-cut WAL roots using flat segment files should remain
recoverable during migration.

Acceptance criteria:

- Recovery reads flat root-level segment files when no canonical namespace
  exists.
- Canonical `segments/` layout is preferred for new writes.
- Compatibility does not let duplicate segment ids silently win.

Test plan:

- `legacy_flat_segment_scan_remains_readable`
- `duplicate_segment_id_across_layouts_blocks_recovery`

## Slice 69: Canonical Gap And Rewrite Behavior

User story:

As Echo, segment gap detection and writable recovery rewrites must use the
canonical namespace consistently.

Acceptance criteria:

- Segment gaps in the canonical namespace block recovery.
- Writable tail rewrite emits canonical segment paths.
- Rewrites do not flatten canonical segments back into the root.

Test plan:

- `segment_gap_in_canonical_directory_blocks_recovery`
- `writable_recovery_rewrite_preserves_canonical_segments_directory`

## Slice 70: Segment Id Rotation Guard

User story:

As Echo, segment rotation must fail explicitly rather than wrapping logical ids.

Acceptance criteria:

- `next_segment_id(None)` starts at logical segment id 1.
- Incrementing a normal id returns the next logical id.
- Incrementing `u64::MAX` reports overflow and does not wrap to zero.

Test plan:

- `next_segment_id_overflow_blocks_rotation`

## Slice 71: Segment Manifest Entry Shape

User story:

As Echo, segment manifests should bind logical ids and frame ranges without
making wall-clock paths authoritative.

Acceptance criteria:

- Manifest entries include segment id, canonical relative path, digest, and LSN
  bounds.
- The relative path uses the logical `segments/segment-*.ecwal` shape.
- Empty segments report no first/last LSN rather than fake zero bounds.

Test plan:

- `segment_manifest_entry_binds_logical_id_not_wall_clock_path`

## Slice 72: Segment Layout Release Gate

User story:

As Echo, release readiness must fail if the WAL segment layout rules are not
covered.

Acceptance criteria:

- `WalReleaseReadinessGates` includes segment layout coverage.
- Audit output names the missing `segment_layout_policy` gate.
- A fully covered hardening gate passes only when this category is green.

Test plan:

- `segment_layout_gate_is_part_of_wal_release_readiness`
- `wal_hardening_gate_passes_when_all_categories_are_green`

## Slice 73: Manifest-Addressed Placement Doctrine

User story:

As Echo, docs and tests must make the manifest/logical-id model the durable
truth boundary for segment placement.

Acceptance criteria:

- Segment manifest entries remain logical-id based.
- Wall-clock partitioning is documented as non-authoritative adapter policy.
- No test asserts causal meaning from date-derived paths.

Test plan:

- Segment manifest and placement-policy hardening fixtures.
- `git diff --check`

## Slice 74: Canonical Layout Migration Witness

User story:

As Echo, migration from flat root segment files to `segments/` should be
test-visible and non-destructive.

Acceptance criteria:

- Flat legacy roots remain readable.
- Writable recovery rewrites into canonical layout.
- Duplicate legacy/canonical ids are treated as corruption or obstruction, not
  as a merge policy.

Test plan:

- `legacy_flat_segment_scan_remains_readable`
- `writable_recovery_rewrite_preserves_canonical_segments_directory`
- `duplicate_segment_id_across_layouts_blocks_recovery`

## Slice 75: Segment Layout Drift Gate

User story:

As Echo, future changes must not turn operational path layout into causal
history.

Acceptance criteria:

- Canonical path tests reject date partitions in durable relative paths.
- Placement policy validation rejects authoritative wall-clock placement.
- Release gate coverage fails when segment layout witnesses are missing.

Test plan:

- `canonical_segment_path_uses_logical_segments_directory`
- `wall_clock_segment_placement_cannot_be_authoritative`
- `segment_layout_gate_is_part_of_wal_release_readiness`

## Slice 76: Active Segment Id Enforcement

User story:

As Echo, frames must be appended only to the active logical segment they claim.

Acceptance criteria:

- Filesystem append checks the frame header segment id against the active store
  segment id.
- A mismatch returns typed store error instead of writing misleading bytes.
- Segment id enforcement remains storage-generic and contains no app nouns.

Test plan:

- `filesystem_append_frame_rejects_inactive_segment_id`

## Slice 77: Canonical Segment Rotation

User story:

As Echo, segment rotation should seal the current segment and create the next
canonical segment under `segments/`.

Acceptance criteria:

- Rotation returns a `WalSegmentSeal` for the previous segment.
- The active segment id advances monotonically.
- The new canonical segment file is created under `segments/`.
- Rotation rejects an existing next segment instead of truncating it.
- Strict filesystem sync evidence records the new segment file and containing
  directory sync.

Test plan:

- `filesystem_rotate_segment_creates_next_canonical_segment`
- `filesystem_rotate_segment_does_not_overwrite_existing_next_segment`

## Slice 78: Rotation Tail Safety

User story:

As Echo, rotation must not seal a segment containing uncommitted frames or a
torn tail.

Acceptance criteria:

- Rotation inspects the current segment before sealing.
- Segments with uncommitted frames reject with typed store error.
- Torn final records reject through the same tail-safety posture.

Test plan:

- `filesystem_rotate_segment_rejects_uncommitted_tail`

## Slice 79: Multi-Segment Recovery

User story:

As recovery, committed transactions split across rotated segments must replay as
one logical WAL stream.

Acceptance criteria:

- Recovery scans multiple canonical segment files in logical segment-id order.
- Transactions in later segments recover cleanly.
- Last committed LSN reflects the logical WAL stream, not one file.

Test plan:

- `filesystem_recovery_reads_transactions_across_rotated_segments`

## Slice 80: Rotation Authority Guard

User story:

As Echo, only the active writer epoch may rotate WAL segments.

Acceptance criteria:

- Rotation requires an active writer epoch.
- Epoch mismatch rejects before creating the next segment file.
- The guard preserves the single-writer WAL authority boundary.

Test plan:

- `filesystem_rotate_segment_rejects_epoch_mismatch`

## Slice 81: Manifest Read Roundtrip

User story:

As Echo, published filesystem manifests must be readable as structured WAL
evidence.

Acceptance criteria:

- Manifest files decode back into `WalManifest`.
- Decoding uses the same canonical field encoding as publication.
- Missing manifest remains explicit `None`.

Test plan:

- `filesystem_manifest_read_roundtrips_published_manifest`

## Slice 82: Manifest Segment-Count Validation

User story:

As recovery, a published manifest must not lie about segment count.

Acceptance criteria:

- Validation compares manifest segment count against scanned segment files.
- Count mismatch returns typed store error.
- The validation is independent of wall-clock path layout.

Test plan:

- `filesystem_manifest_validation_rejects_segment_count_mismatch`
- `filesystem_manifest_validation_accepts_matching_segment_summary`

## Slice 83: Manifest Commit-Anchor Validation

User story:

As recovery, a published manifest must match the last committed LSN and commit
digest recovered from segment contents.

Acceptance criteria:

- Last committed LSN mismatch returns typed store error.
- Last commit digest mismatch returns typed store error.
- A matching manifest returns a structured validation report.

Test plan:

- `filesystem_manifest_validation_rejects_last_lsn_mismatch`
- `filesystem_manifest_validation_rejects_last_digest_mismatch`

## Slice 84: Manifest Tail Safety

User story:

As recovery, a manifest cannot validate while segments contain an uncommitted
tail.

Acceptance criteria:

- Manifest validation rejects full uncommitted frames after the last commit.
- Manifest validation rejects torn tails.
- A manifest never turns incomplete history into accepted disk truth.

Test plan:

- `filesystem_manifest_validation_rejects_uncommitted_tail`

## Slice 85: Manifest Validation Release Gate

User story:

As a maintainer, release readiness must require manifest validation coverage.

Acceptance criteria:

- `WalReleaseReadinessGates` includes `segment_manifest_validation`.
- Audit output names the missing gate.
- A fully green readiness report requires manifest validation coverage.

Test plan:

- `segment_manifest_validation_gate_is_part_of_wal_release_readiness`
- `wal_hardening_gate_passes_when_all_categories_are_green`

## Slice 86: Runtime WAL Adapter Port

User story:

As a trusted runtime host, I need a local WAL adapter at the app-facing ACK
boundary without giving the application append authority.

Acceptance criteria:

- `TrustedRuntimeHost` owns the runtime WAL adapter.
- `TrustedRuntimeHost` configures the adapter through `TrustedRuntimeWalConfig`.
- The adapter can be backed by a host-owned filesystem WAL root.
- Reopened filesystem adapters continue the committed LSN and commit-digest
  chain.
- Filesystem read-only recovery preserves torn/corrupt segment posture.
- Filesystem-backed scheduler batches reject unsafe multi-transaction rollback
  until an atomic filesystem batch transaction exists.
- `TrustedRuntimeApp` exposes no WAL append or tick authority.
- The adapter can be inspected by host tests as read-only evidence.

Test plan:

- `runtime_wal_ack_adapter_is_configured_by_trusted_host_boundary`
- `runtime_wal_ack_submit_commits_acceptance_before_returning_handle`
- `filesystem_runtime_wal_ack_reconstructs_submission_and_tick_from_root`
- `filesystem_runtime_wal_ack_reconstructed_host_appends_after_recovery`
- `filesystem_runtime_wal_ack_recovery_reports_uncommitted_tail_from_root`
- `filesystem_runtime_wal_ack_recovery_rejects_corrupt_root`
- `filesystem_runtime_wal_ack_multi_head_tick_rejects_before_partial_filesystem_append`

## Slice 87: Submission Acceptance Transaction Wiring

User story:

As a caller, returned accepted submission evidence must be backed by a committed
submission-intake WAL transaction when using the WAL-backed ACK path.

Acceptance criteria:

- `submit_intent_with_runtime_wal_ack(...)` requires a configured runtime WAL.
- Accepted submissions record `SubmissionAcceptedRecorded` and acceptance
  evidence before the handle is returned.
- The recovered submission index can rebuild the pending submission from WAL.

Test plan:

- `runtime_wal_ack_submit_commits_acceptance_before_returning_handle`
- Recovered submission index assertion for `AcceptedPending`.

## Slice 88: Duplicate Submit ACK Posture

User story:

As a retrying client, resubmitting the same accepted envelope must not spray
duplicate WAL acceptance transactions.

Acceptance criteria:

- Duplicate submit returns the original submission id.
- If WAL evidence already exists, the submission-acceptance transaction count
  remains unchanged.
- If the duplicate came from a legacy non-WAL intake path, the WAL ACK path
  backfills exactly one acceptance transaction before returning.

Test plan:

- `runtime_wal_ack_duplicate_submit_does_not_append_second_acceptance`
- `runtime_wal_ack_duplicate_without_prior_wal_backfills_acceptance`

## Slice 89: Pre-ACK WAL Failure Rollback

User story:

As Echo, if the WAL cannot commit accepted-submission evidence, the in-memory
intake mutation must not remain visible.

Acceptance criteria:

- Missing WAL returns an explicit unavailable posture before mutating runtime
  intake state.
- WAL build failure restores pre-submit runtime state.
- Filesystem append or flush failure restores pre-submit runtime state and
  leaves no committed submission evidence after recovery repair.
- Failed WAL ACK does not create witnessed submission history.

Test plan:

- `runtime_wal_ack_path_requires_configured_runtime_wal`
- `runtime_wal_ack_failure_rolls_back_intake_mutation`
- `filesystem_runtime_wal_failure_submission_append_rolls_back_pre_ack_visibility`
- `filesystem_runtime_wal_failure_submission_flush_rolls_back_pre_ack_visibility`

## Slice 90: Tick Receipt Transaction Wiring

User story:

As Echo, visible tick receipts should eventually be backed by committed
scheduler-tick WAL transactions.

Acceptance criteria:

- Host-owned scheduler runs record receipt and correlation facts before
  publishing product-facing receipt evidence.
- The WAL receipt index can rebuild applied/rejected decisions.

Test plan:

- Trusted-host applied-intent fixture.
- Recovered receipt-index witness.

## Slice 91: Tick Commit-Before-Publish Rollback Guard

User story:

As Echo, a tick WAL failure must not leave a half-visible receipt/outcome.

Acceptance criteria:

- Tick WAL failure either restores runtime/provenance state or blocks receipt
  publication under typed runtime fault posture.
- The app-facing outcome cannot observe a receipt whose WAL transaction failed.
- Filesystem append or flush failure during a scheduler tick leaves recovered
  evidence at accepted-pending submission posture with no receipt index entry.
- Filesystem manifest publish failure surfaces a typed store error without
  publishing manifest material.

Test plan:

- Injected tick-WAL failure fixture.
- App-facing outcome remains pending or runtime-faulted, never half-decided.
- `filesystem_runtime_wal_failure_tick_append_rolls_back_visible_outcome`
- `filesystem_runtime_wal_failure_tick_flush_rolls_back_visible_outcome`
- `filesystem_runtime_wal_failure_manifest_publish_reports_store_error`

## Slice 92: Runtime Index Rebuild Contract

User story:

As recovery, WAL-backed submission and receipt indexes should rebuild without
scheduler callbacks.

Acceptance criteria:

- Recovery applies committed WAL facts through pure reducers.
- No scheduler, wall-clock, app callback, or external I/O participates in index
  rebuild.

Test plan:

- Pure in-memory recovery fixture for submit plus tick records.
- Shadow replay comparison for recovered submission/receipt posture.

## Slice 93: WAL-Backed Recovery Certificate In Runtime

User story:

As an operator, restart should produce inspectable evidence about what committed
history was replayed.

Acceptance criteria:

- Recovery certificate covers checkpoint, LSN range, commit digest, tail
  posture, and recovered counts.
- The certificate is evidence, not a new app-domain mutation.

Test plan:

- Recovery certificate fixture over committed and truncated-tail WAL shapes.

## Slice 94: `jedit` Recovery Fixture Contract

User story:

As a real app consumer, `jedit` should be able to distinguish not-accepted,
accepted-pending, decided, rejected, and obstructed edits from Echo recovery
evidence.

Acceptance criteria:

- Echo exposes only generic submission/receipt posture.
- `jedit` maps posture to editor terms outside Echo.

Test plan:

- Sibling `jedit` fixture consumes generic Echo recovery JSON.

## Slice 95: Runtime ACK Drift Gate

User story:

As a maintainer, docs and tests should fail if Echo claims durable ACK semantics
without a WAL-backed witness.

Acceptance criteria:

- Release readiness names runtime ACK coverage as a distinct gate.
- Stale claims that accepted submissions are restart-proof without WAL backing
  are either removed or marked future.

Test plan:

- Runtime ACK readiness gate fixture.
- Stale-claim grep over docs.
