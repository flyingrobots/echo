<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# WARP Rewrite Scheduler

_Define the implemented `warp-core` scheduler: deterministic drain order plus footprint reservation._

Legend: KERNEL

Depends on:

- [Canonical Inbox Sequencing](canonical-inbox-sequencing.md)
- [SPEC-0003 - DPO Concurrency Litmus v0](SPEC-0003-dpo-concurrency-litmus-v0.md)
- [WARP Tick Patch](warp-tick-patch.md)

## Why this packet exists

The scheduler is Echo's current settlement law for competing rewrites. It is where candidate work becomes an admitted set, rejected candidates receive a witness, and the tick stays deterministic across runs.

## Human users / jobs / hills

Human users need scheduler claims tied to code and tests.

The hill: when a candidate is rejected, a reviewer can identify the declared resource conflict and reproduce the accept/reject decision.

## Agent users / jobs / hills

Agent users need a predictable reservation protocol.

The hill: an agent can construct footprints, sort pending rewrites the same way the runtime does, and predict whether `reserve()` should admit the candidate.

## Decision 1: Pending rewrites carry explicit footprints

A pending rewrite carries scope hash, rule id, scope, footprint, and phase. The footprint is a conservative resource contract across nodes, edges, attachments, boundary ports, and factor mask.

## Decision 2: `reserve()` is check-then-mark

Reservation checks all candidate resources for conflict and marks resources only if the check succeeds. A rejected candidate must leave no partial marks.

## Decision 3: Conflicts are resource conflicts

A candidate conflicts when it writes a resource another admitted candidate reads or writes, or when its boundary port claim overlaps another admitted boundary port claim. Reads may overlap reads.

The broad WARP outcome algebra is `Derived | Plural | Conflict |
Obstruction`. Echo's local `TickReceipt` entries currently realize the narrower
tick-scale shape `Applied / Rejected(FootprintConflict)`, with blocker
attribution for conflicts. `Plural` belongs to broader braid/replica-scale work
until an executable local claim requires it.

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
