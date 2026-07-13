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

The current runtime WAL evidence says four concrete things.

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

## Topology Intents

The same rule applies to topology. A strand fork, braid creation, member weave,
braid settlement, braid collapse, or replica suffix import should not be a
side-channel runtime call. Each one is a topology intent that changes causal
history or the geometry through which causal history is interpreted.

The current braid and strand implementation already has typed strand
construction, braid event logs, settlement provenance entries, retained braid
shells, and replay/audit optics. The remaining durability gate is to promote
those topology operations to WAL-backed accepted evidence and recoverable
WSC-retained material. Track that remaining implementation work in GitHub.

Until that lands, Echo should not overclaim that braid shells and topology
indexes have the same explicit crash-recovery posture as tick receipts. It can
claim that they are causal/provenance entities with retained replay shapes, and
it can name WAL/WSC topology recovery as the required next hardening step.

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
outbox, topology, and graph/WSC projection posture. Uncommitted tail frames are
reported through tail posture and do not appear in rebuilt indexes.
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

The host surface lives in `crates/warp-core/src/trusted_runtime_host.rs`,
especially `TrustedRuntimeHost`, `TrustedRuntimeApp`, `TrustedRuntimeWal`,
`submit_intent_with_runtime_wal_ack(...)`, and `recover_read_only()`.

Related current authority lives in `docs/topics/RuntimeAuthority.md`,
`docs/architecture/continuum-transport.md`, and
`docs/releases/echo-1.0-contract.md`. Live release-hardening work and status
belong in GitHub.

## Current Caveat

The trusted-runtime host tests use an in-memory runtime WAL adapter to prove ACK
ordering, rollback behavior, receipt publication ordering, and recovery index
shape. That is not the same claim as strict filesystem durability. Filesystem
WAL hardening, WSC export/import shape, retained material availability, and
release-grade recovery gates remain the place to prove crash and portability
claims beyond the current ACK boundary witnesses.

`cargo xtask dind` now carries the `dind_durability_convergence_gate` witness
for the joined durability path. The gate commits one filesystem WAL history,
projects it through read-only recovery, imports the same causal evidence through
WSC, reveals retained reading material, and requires all paths to agree on the
same app-facing receipt and bounded reading. Missing CAS support material and
corrupt embedded retained bytes must surface as typed obstruction evidence
rather than a divergent success.
