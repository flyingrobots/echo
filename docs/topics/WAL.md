<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# WAL

Echo's write-ahead log is the durable commit boundary for causal history.
Applications submit canonical intents. A trusted Echo host admits those intents,
records the accepted-submission fact, schedules lawful work, records decided
tick evidence, and exposes outcomes only after the relevant WAL fact is
committed.

The short rule is:

```text
Echo may only claim what its WAL can recover.
```

## What We Found

The current runtime WAL evidence says eight concrete things.

First, accepted-submission evidence is not just an in-memory editor event. The
WAL-backed ACK path, `submit_intent_with_runtime_wal_ack(...)`, returns only
after the runtime WAL has committed the submission-acceptance transaction. That
acceptance fact recovers as an `AcceptedPending` submission if the scheduler has
not yet produced a decided outcome.

Second, failed WAL acceptance is not allowed to leave half-visible runtime
state. If no runtime WAL is configured, the ACK path returns an explicit
unavailable error before mutating intake state. If WAL commit fails, Echo rolls
back the in-memory intake mutation before returning the error.

Third, tick receipts are published only after scheduler-tick WAL evidence is
committed. Recovery can rebuild both the submission posture and the receipt
index from committed WAL facts. If tick WAL recording fails, Echo rolls back the
visible receipt/outcome instead of exposing a receipt that recovery cannot
rebuild.

Fourth, read-only recovery rebuilds submission and receipt indexes from the WAL
without scheduler callbacks, application callbacks, wall-clock interpretation,
or external I/O. Recovery also emits a certificate over the committed replay
range and recovered index root.

Fifth, an admitted intent may cite typed causal parent tick receipts. Echo does
not interpret a parent as undo, redo, compensation, or any other application
operation. It commits the citation with the child receipt and rebuilds both
parent lookup and reverse child lookup during read-only recovery. A contract can
therefore define inverse semantics while Echo preserves the provenance chain
across process loss. Recovery rejects explicitly empty, duplicated, or
out-of-order parent receipt coordinates instead of normalizing committed
non-canonical bytes. Before exposing a recovered correlation, it also requires
that correlation's parent set to match the independently retained ingress
envelope.

Sixth, trusted submission intake commits the canonical retained ingress envelope
in the same transaction as acceptance evidence. Reopening a filesystem WAL can
restore witnessed submission identity, generation, route, payload, and causal
parents without scheduler or contract callbacks. Legacy acceptance records that
lack envelope material remain inspectable, but recovery reports them as
obstructed and the trusted host refuses to claim a complete replay.
Transaction construction rejects any retained envelope whose submission or
canonical-envelope identity disagrees with its acceptance record.
Recovery requires exactly one acceptance frame and at most one retained
envelope frame in each intake transaction.

Seventh, a scheduler-tick transaction for ticketed runtime ingress retains the
canonical local-commit provenance entry, its exact typed `TickReceipt`, and any
installed-invocation identity attached to the outcome. That identity is a
tagged proposition: the existing legacy Wesley/GraphQL evidence keeps its
byte-stable tag-1 encoding, while tag 2 retains provider package id and exact
reference, operation id and coordinate, Target IR identity, and scheduler rule
id. Provider decoding rejects empty coordinates, reserved operation ids, and
malformed package or Target IR digests rather than laundering structurally
invalid fields through a valid WAL frame checksum. Provider evidence does not
fabricate a legacy retained-contract coordinate.

Filesystem WAL reopen loads that evidence into a fresh provenance service,
replays the worldline from its deterministic registered boundary, restores
receipt correlations, and only then publishes the reconstructed runtime. The
recovery witness compares global tick, frontier tick, state root, provenance,
receipt, contract evidence, and the app-facing outcome without running the
scheduler or contract handler. A fresh host may then independently reinstall
the same provider package as host configuration; recovered invocation remains
idempotent and does not become duplicate scheduler work. A legacy digest-only
state-delta record is inspectable but obstructs writable startup.
Transaction construction also requires the receipt and correlation to name the
same causal event and the retained state delta to bind that receipt's content
commitment before the transaction can commit.
Each scheduler transaction must also contain exactly one receipt, correlation,
and runtime state-delta frame; recovery rejects duplicate claims rather than
selecting whichever frame appears first.
WAL activation is non-lossy: if the live host contains submissions, staged
ingress, receipt correlations, provenance, pending inbox work, cycle progress,
or worldline state that recovered WAL evidence cannot reproduce, activation
fails instead of treating process memory as durable authority.

Eighth, causal-anchor admission has its own admission-kernel authority,
transaction kind, fact record, receipt record, and affected frontier. The
canonical claim contains no receipt identity. Echo derives that identity from
the claim, host support-policy digest, and WAL coordinate, commits the fact and
receipt in one transaction, and recovers only complete, internally consistent
pairs. Uncommitted frames do
not become admitted anchors; malformed payloads, unknown enum codes, trailing
bytes, missing or duplicate required frames, mismatched fact/receipt evidence,
noncanonical frame order, and mismatched WAL coordinates are rejected. The
trusted-host API requires the current logical durable frontier and exact
host-installed root support before invoking this transition, and returns only
after commit.

## Boundaries

The WAL belongs to the trusted runtime host. Application-facing code can submit
intents and observe outcomes, but it cannot append WAL records, tick the
scheduler, install packages, stage ticketed ingress, or perform trusted
recovery.

That split matters because an accepted edit is not durable because an editor
buffer says it is dirty, because a file was written to disk, or because a UI
event happened. It is durable because Echo recorded the accepted submission and
later recorded any decided receipt under host-owned WAL authority.

## Recovery Postures

The useful postures are:

| Posture            | Meaning                                                                                                               |
| ------------------ | --------------------------------------------------------------------------------------------------------------------- |
| `not_accepted`     | The intent never reached WAL-backed accepted submission posture.                                                      |
| `accepted_pending` | Accepted-submission evidence was recovered, but no decided receipt was recovered.                                     |
| `decided_applied`  | A recovered receipt says the work applied under named law.                                                            |
| `decided_rejected` | A recovered receipt says the work was rejected or conflicted.                                                         |
| `obstructed`       | Recovery found accepted or decided evidence, but required material or consistency checks obstruct restoring the work. |
| `recovery_faulted` | Required committed WAL evidence or retained material is missing or corrupt.                                           |

An app such as `jedit` maps these generic postures into product language
outside Echo. Echo should not grow editor, file, buffer, or dirty-state nouns in
order to explain them.

## What This Means For Editors

For Jim/jedit, "dirty" should not mean "at risk of being lost." It should mean
"not currently materialized to the host file projection." If an edit intent has
received WAL-backed accepted-submission evidence, the edit's identity and
posture belong to recoverable causal history even before the host file is
written. Restoring the editor text still requires retained material, a decided
receipt, or a recovered reading that carries the content needed for the view. If
that content is unavailable, the editor must keep the materialization warning and
surface the recovered obstruction instead of claiming the buffer can be rebuilt
from an ACK alone.

The correct safety gate is therefore not a classic dirty-buffer warning on quit
or file switch. The correct gate is whether the edit crossed the Echo
WAL-backed ACK boundary and whether recovery can later classify it as pending,
decided, rejected, or obstructed. If that boundary is unavailable, the editor
must say so honestly instead of pretending a local buffer is causal history.

Restored submission material alone is not runtime-state replay. For WAL-backed
ticketed transitions, the trusted host now also requires replayable provenance
and the exact tick receipt, then restores the materialized worldline and outcome
before allowing writable startup. That is the causal authority Jim needs for
durable edit history. It is still not complete editor-session restoration:
viewport, cursor, mode, open-buffer topology, and other application projections
must be expressed as admitted application intents and recovered readings rather
than inferred from the text worldline.

## Topology Intents

The same rule applies to topology. A strand fork, braid creation, member weave,
braid settlement, braid collapse, or replica suffix import should not be a
side-channel runtime call. Each one is a topology intent that changes causal
history or the geometry through which causal history is interpreted.

The current braid and strand implementation already has typed strand
construction, braid event logs, settlement provenance entries, retained braid
shells, and replay/audit optics. Topology operations do not currently have
WAL-backed accepted evidence or recoverable WSC-retained material. Echo must not
claim that braid shells and topology indexes have the same explicit
crash-recovery posture as tick receipts. They are causal/provenance entities
with retained replay shapes, not evidence of WAL/WSC topology recovery.

## WAL, Graph Facts, And WSC

WAL bytes are the durable commit authority. WARP graph facts can track WAL
segment evidence, and WSC can serialize graph facts plus WAL references or
bundled WAL material, but neither graph facts nor WSC replace the WAL commit
boundary.

A WAL path is a storage locator, not causal identity. Causal identity comes from
the writer epoch, LSN range, segment digest, commit digest chain, and validated
commit anchors.

Recovery planning records the bootstrap source, optional checkpoint posture,
committed replay suffix, tail posture, recovered index roots, retained-material
posture, and projected evidence posture. A recovery plan may start from a
projected WAL root or storage manifest, but it does not require graph WAL nodes
as recovery input.

Read-only recovery can rebuild durability indexes from committed transactions:
submission posture, receipt/correlation, retained material, materialization
outbox, topology, causal-anchor admissions, and graph/WSC projection posture.
Uncommitted tail frames are reported through tail posture and do not appear in
rebuilt indexes.
Materialization outbox recovery reports typed posture for missing artifacts,
artifact or metadata mismatches, committed observation mismatches, and retained
material unavailability so restart logic can retry, repair, or obstruct without
blindly replaying effects.

The process-kill crashpoint runner exercises the filesystem WAL across real
parent/child process boundaries. A killed child that already committed WAL
material recovers as committed history; a killed child with only uncommitted
frames recovers as tail posture and does not enter accepted or decided history.

The durable vocabulary is precise:

- Records are recorded.
- Transactions are committed.
- Segments are sealed.

Graph facts are projected evidence. WSC carries or references that evidence;
it does not become a second commit authority.

Release-grade durability requires all of the following executable claims:

- Recovery is deterministic from the same committed evidence.
- The defined crashpoint matrix passes.
- Duplicate replay is idempotent.
- Corrupt or incomplete evidence is deterministically rejected.
- Retained evidence survives restart or produces a typed obstruction.
- Required recovery artifacts are emitted by CI.

## Evidence

The runtime ACK and recovery witnesses live in
`crates/warp-core/tests/trusted_runtime_host_loop_tests.rs`.

The core test names to read first are:

- `runtime_wal_ack_submit_commits_acceptance_before_returning_handle`
- `runtime_wal_ack_path_requires_configured_runtime_wal`
- `runtime_wal_ack_failure_rolls_back_intake_mutation`
- `runtime_wal_ack_tick_commits_receipt_transaction_before_outcome_is_observed`
- `runtime_wal_ack_tick_failure_rolls_back_visible_outcome`
- `runtime_wal_ack_recover_read_only_rebuilds_submission_and_receipt_indexes`
- `runtime_wal_ack_recover_read_only_exposes_recovery_certificate`
- `filesystem_runtime_wal_recovers_receipt_causal_parents_after_host_restart`
- `filesystem_runtime_wal_restores_witnessed_submission_material_after_restart`
- `filesystem_runtime_wal_recovers_replayable_provenance_after_restart`
- `runtime_wal_activation_rejects_process_only_committed_history`
- `submission_acceptance_rejects_mismatched_retained_material`
- `tick_transaction_rejects_mismatched_receipt_correlation`
- `replayable_tick_transaction_rejects_unrelated_state_delta_receipt`

The canonical retained transition codec witnesses live in
`crates/warp-core/tests/provenance_retention_codec_tests.rs`. They cover every
current patch operation and slot variant, exact receipt and contract-evidence
round-trip, all truncation boundaries, trailing bytes, corrupt commitments,
missing receipts, and non-local events.

The causal-anchor WAL codec and recovery witnesses live in
`crates/warp-core/tests/causal_anchor_wal_tests.rs`. They cover stable persisted
codes, committed round-trip, uncommitted-tail invisibility, required-frame
cardinality, truncation, trailing bytes, malformed enum codes, cross-admission
evidence mismatch, and WAL-coordinate binding.

The host surface lives in `crates/warp-core/src/trusted_runtime_host.rs`,
especially `TrustedRuntimeHost`, `TrustedRuntimeApp`, `TrustedRuntimeWal`,
`submit_intent_with_runtime_wal_ack(...)`, `admit_causal_anchor(...)`,
`causal_anchor_by_id(...)`, and `recover_read_only()`.

Related current authority lives in `docs/topics/RuntimeAuthority.md`,
`docs/architecture/continuum-transport.md`, and
`docs/releases/echo-1.0-contract.md`. Live release-hardening work and status
belong in GitHub.

## Current Caveat

The trusted-runtime suite uses both the fast in-memory adapter and the strict
filesystem adapter. Filesystem witnesses now prove accepted-submission and
ticketed-transition replay across host reconstruction. The replayable
state-delta path is currently emitted from ticketed receipt correlations; work
committed through lower-level, non-ticketed scheduler ingress does not yet have
the same trusted-host WAL reconstruction claim. Jim must therefore use the
WAL-backed application-intent path for user-facing edits and inverses instead
of treating an internal runtime mutation as durable history.

WSC export/import shape, retained reading availability, multi-correlation tick
packing, and complete application-session projection recovery remain separate
gates. None of those gaps permits falling back to process-local undo authority.

`cargo xtask dind` now carries the `dind_durability_convergence_gate` witness
for the joined durability path. The gate commits one filesystem WAL history,
projects it through read-only recovery, imports the same causal evidence through
WSC, reveals retained reading material, and requires all paths to agree on the
same app-facing receipt and bounded reading. Missing CAS support material and
corrupt embedded retained bytes must surface as typed obstruction evidence
rather than a divergent success.
