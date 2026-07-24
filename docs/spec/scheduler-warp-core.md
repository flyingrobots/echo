<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# WARP Rewrite Scheduler

_Define the implemented `warp-core` scheduler: deterministic drain order plus footprint reservation._

Depends on:

- [Canonical Inbox Sequencing](canonical-inbox-sequencing.md)
- [SPEC-0003 - DPO Concurrency Litmus v0](SPEC-0003-dpo-concurrency-litmus-v0.md)
- [WARP Tick Patch](warp-tick-patch.md)
- [ADR 0025 - Scheduler-Owned Executable-Operation Actions](../adr/0025-scheduler-owned-executable-operation-actions.md)

## Purpose

The scheduler is Echo's current settlement law for competing rewrites. It is where candidate work becomes an admitted set, rejected candidates receive a witness, and the tick stays deterministic across runs.

Scheduler claims must be tied to code and tests. When a candidate is rejected,
a reviewer can identify the declared resource conflict and reproduce the
admission decision.

The reservation protocol is predictable: a consumer can construct footprints,
sort pending rewrites as the runtime does, and determine whether `reserve()`
should admit a candidate.

## Decision 1: Pending rewrites carry explicit footprints

A pending rewrite carries scope hash, rule id, scope, footprint, and phase. The footprint is a conservative resource contract across nodes, edges, attachments, boundary ports, and factor mask.

## Decision 2: `reserve()` is check-then-mark

Reservation checks all candidate resources for conflict and marks resources only if the check succeeds. A rejected candidate must leave no partial marks.

## Decision 3: Conflicts are resource conflicts

A candidate conflicts when it writes a resource another admitted candidate reads or writes, or when its boundary port claim overlaps another admitted boundary port claim. Reads may overlap reads.

The broad WARP outcome algebra is `Derived | Plural | Conflict |
Obstruction`. Echo's local `TickReceipt` entries realize the narrower
tick-scale shape `Applied / Rejected(FootprintConflict |
ExecutableOperationObstruction)`. Footprint conflicts name earlier applied
blockers. An executable-operation obstruction has no blockers and contributes
no mutation. `Plural` belongs to broader braid/replica-scale work until an
executable local claim requires it.

Conflict rejection is final for that tick attempt. Retry is a new explicit
causal act, not a hidden retry queue.

Admission obstructions happen before ticketed scheduler work. Internal runtime
faults are not normal receipt dispositions; they roll back the failed scheduler
attempt and enter runtime-local quarantine posture outside the `TickReceipt`
path.

## Decision 4: Drain order is canonical

Pending rewrites drain in lexicographic order derived from `scope_hash`, stable rule id, and nonce tie-breaker. The scheduler does not depend on hash-table iteration order.

## Decision 5: Radix reservation is the default implementation

The default scheduler uses generation-stamped active sets for membership checks. Expected reservation cost is proportional to candidate footprint size.

## Decision 6: Executable Actions are evaluated only inside Tick construction

A canonical executable-operation Action enters the same durable submission and
head-inbox lifecycle as other work. The runtime resolves the exact installed
package and admission policy, but neither submission nor admission evaluates
application semantics.

The scheduler partitions executable and legacy/native ingress deterministically
at the lowest canonical ingress id so the two evaluator categories never share
one candidate batch. Inside an executable batch, Action ingress order is
canonical. Private bounded evaluation yields either a complete prepared
candidate or a typed no-mutation obstruction. The scheduler reserves successful
candidate footprints, constructs one composite consequence, and emits one Tick
receipt entry per Action.

The successor state remains private until the complete Tick transaction is
durable. Construction failure discards it. WAL failure restores the
accepted-pending posture. The direct executable-operation prepare/commit methods
are transitional host/test seams, not a second application execution path.
