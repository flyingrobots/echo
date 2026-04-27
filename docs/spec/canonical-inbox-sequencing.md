<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Canonical Inbox Sequencing

_Define intent identity and tick-boundary ordering so arrival order does not become causality._

Legend: KERNEL

Depends on:

- [WARP Rewrite Scheduler](scheduler-warp-core.md)
- [WARP Tick Patch](warp-tick-patch.md)
- [Merkle Commit](merkle-commit.md)

## Why this packet exists

Echo admits intents from outside the kernel. Arrival order is an observer phenomenon; it must not silently become causal order inside a deterministic tick. The inbox contract turns submitted bytes into content-addressed ingress entries and settles pending work in canonical order.

## Human users / jobs / hills

Human users need retries and shuffled submissions to converge.

The hill: submitting the same set of intent bytes in any order produces the same pending set, same canonical tick order, and same committed state when tick membership is held fixed.

## Agent users / jobs / hills

Agent users need idempotent intake for automation.

The hill: an agent can retry a submitted envelope and receive the same identity without duplicating ledger state.

## Decision 1: Intent identity is content identity

Two intents are the same when their canonical bytes are byte-identical: `intent_id = H(intent_bytes)`. Sequence numbers, arrival time, and transport metadata are not identity.

## Decision 2: Ingress is idempotent per resolved writer head

The pending inbox is keyed by `ingress_id` for the resolved writer head. Re-ingesting an already pending or already committed intent must not create a second live entry.

## Decision 3: Tick membership is a causality boundary

Permutation-invariance claims require the same set of intents to belong to the same tick. If tick membership differs, the causal history differs.

## Decision 4: Pending work is ordered at the tick boundary

For a pending set: deduplicate by content identity, sort by ingress/id byte order, and derive audit sequence numbers from that sorted order. The runtime must not use arrival order, hash-map iteration, thread scheduling, or platform timing.

## Decision 5: Ledgers are append-only; queues are maintenance state

Ingress event records are immutable once committed. Processing an event removes or updates pending membership; it does not delete the historical event.
