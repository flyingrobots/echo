<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Goalpost 6: Topology Intents And WAL Recovery

Status: implemented.

Tracking issue: [#604](https://github.com/flyingrobots/echo/issues/604)

## Goal

Make topology-changing operations admitted causal history.

Strand forks, braid lifecycle events, braid settlements, retained braid shells,
and replica suffix import must cross the same intent, receipt, WAL, and retained
evidence boundary that tick receipts already use. They are not side-channel
runtime state, service calls, or Git-shaped branch metadata.

The target shape is:

```text
topology intent
-> WAL-backed accepted topology evidence
-> receipt or shell
-> recoverable topology index
-> replay/audit reading
```

## Doctrine

AION Paper VII treats tick execution, braid lowering, and replica suffix import
as one recurring WARP optic shape:

```text
Lower(F, P) -> (R, W, theta)
```

At tick scale, `P` is a rewrite bundle and `theta` is a tick receipt. At braid
scale, `P` is a strand braid and `theta` is a braid shell. At replica scale,
`P` is a transported remote suffix family and `theta` is an import shell.

Echo already has typed strand construction, braid event logs, settlement
provenance entries, retained braid shells, and braid replay/audit optics. This
goalpost promotes the remaining topology operations to WAL-backed causal
history so recovery can rebuild them after restart.

## Implementation Anchors

- `crates/warp-core/src/causal_wal.rs` defines the topology WAL transaction and
  record family, topology intent records, deterministic payload codecs,
  recovered topology indexes, recovered topology roots, and readiness posture.
- `crates/warp-core/src/wsc/store.rs` serializes topology records into WSC
  envelopes and recovers them from committed WSC store entries.
- `crates/warp-core/tests/causal_wal_tests.rs` proves topology WAL recovery,
  uncommitted half-fork exclusion, duplicate idempotence, divergent duplicate
  obstruction, and recovery-certificate index rooting.
- `crates/warp-core/tests/wsc_store_tests.rs` proves topology WSC round-trip,
  committed-store recovery, uncommitted staged-envelope exclusion, and
  conflicting duplicate obstruction.
- `crates/warp-core/tests/causal_wal_hardening_tests.rs` keeps topology
  recovery in the WAL readiness gate.

## Required Boundaries

### Strand Forks

Forking a strand creates ancestry. It must become an admitted topology intent
whose receipt binds:

- strand identity;
- source worldline;
- fork tick;
- source commit and boundary hash;
- child worldline;
- writer heads;
- retention posture;
- issuer or session evidence.

The fork ACK must not be returned until the accepted topology evidence is
committed to WAL, or Echo returns an explicit obstruction.

### Braid Events

Creating a braid, weaving a member, finalizing settlement, and collapsing a
plural braid are braid history events. Their event log must be recoverable from
durable evidence, not only from process-local folded state.

The existing `Braid::apply(...)` transition checks remain valuable, but the
accepted event stream must have WAL/WSC recovery posture.

### Braid Shell Retention

`BraidShell` is the retained hologram for braid-scale lowering. Recovery must
be able to recover retained shell identity and material by digest. Re-appending
the same shell remains idempotent; divergent duplicate content remains an
obstruction.

### Replica Suffix Import

Network suffix exchange is not a second merge regime. Transport constructs a
comparable basis and re-expresses remote suffix claims as a weave. Echo then
lowers that weave through the same optic law and retains an import shell.

Duplicate delivery is idempotent re-introduction of the same witnessed
transport object, not a new authored intent.

## Slices

| Slice  | Issue                                                   | Work                                                    |
| ------ | ------------------------------------------------------- | ------------------------------------------------------- |
| GP6-S1 | [#605](https://github.com/flyingrobots/echo/issues/605) | WAL-backed strand fork and drop intent receipts         |
| GP6-S2 | [#606](https://github.com/flyingrobots/echo/issues/606) | WAL/WSC-backed braid event logs and retained shells     |
| GP6-S3 | [#607](https://github.com/flyingrobots/echo/issues/607) | Replica suffix import as a witnessed WARP optic intent  |
| GP6-S4 | [#608](https://github.com/flyingrobots/echo/issues/608) | Recovery indexes for topology state and retained shells |

## Acceptance Criteria

- Strand topology changes are acknowledged only after WAL-backed accepted
  topology evidence commits.
- Braid event logs recover from durable evidence.
- Retained braid shells recover by digest after restart.
- Replica suffix import has explicit authorship, basis, witness, idempotence,
  and retention posture.
- Recovery rebuilds strand registry, child worldline topology, braid event logs,
  retained shell indexes, and import idempotence state from WAL/WSC evidence.
- Missing or corrupt retained topology material returns typed obstruction
  evidence instead of partial silent recovery.

## Non-Goals

- Do not add jedit, file, editor, buffer, or dirty-state nouns to Echo core.
- Do not make Git branches or worktrees topology authority.
- Do not treat WSC graph facts as WAL bootstrap authority.
- Do not collapse braid geometry, settlement, and admission into one ambiguous
  merge operation.
