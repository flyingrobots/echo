<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Spec: Canonical Inbox Sequencing + Deterministic Scheduler Tie-Break

## 0) Purpose

Guarantee that for a given tick, the full WARP graph is bit-identical across runs
that ingest the same set of intents in any order.

This requires:
- intent identity is content-based, not sequence-based
- within-tick ordering is canonical (derived), not arrival-based
- conflict resolution is deterministic and order-independent
- hashing/enumeration is canonical (sorted)
- ledger entries are append-only (no event node deletion)

This spec aligns with ADR-0003: ingress accepts intent_bytes, runtime assigns
canonical sequence numbers, idempotency is keyed by intent_id = H(intent_bytes),
and enumerations that influence state are canonical.

## 1) Terms

- intent_bytes: canonical bytes submitted via ingress.
- intent_id: H(intent_bytes) (content hash).
- seq: canonical sequence number assigned by runtime/kernel.
- tick: a kernel step where rewrites apply and materializations emit.
- footprint: the read/write set (or conflict domain) used by the scheduler to detect conflicts.

## 2) Invariants

I1 - Content identity

Two intents are the "same intent" iff intent_bytes are byte-identical, therefore
intent_id matches.

I2 - Idempotent ingress

Ingress must be idempotent by intent_id -> seq_assigned. Retries return the
original seq.

I3 - Arrival order is non-semantic (within a tick)

Within a tick, the relative ordering of intents MUST NOT depend on arrival
order, insertion order, hash-map iteration order, or thread interleavings.

I4 - Canonical enumeration

Any enumeration that influences state, hashing, serialization, or replay MUST
be in canonical sorted order.

I5 - Ledger is append-only

Inbox/ledger **event nodes** are immutable and MUST NOT be deleted.

Processing an event is modeled as **queue maintenance**: remove a `pending`
marker (edge or flag) so the event is no longer considered pending, without
removing the ledger entry itself.

## 3) Data model requirements

### 3.1 Ledger / Inbox entries

Each pending inbox entry MUST carry:
- intent_id: Hash32
- intent_bytes: Bytes (canonical)
- optional: seq: u64 (canonical rank / audit field; see §4)
- optional: tick_id (if your model persists tick partitioning)
- optional: priority_class (stable scheduler priority input)

Rule: seq is NOT part of identity. Identity is intent_id.

Minimal implementation model (recommended for determinism):
- Ledger entry = immutable event node keyed by `intent_id` (or derived from it).
- Pending membership = `edge:pending` from `sim/inbox` → `event`.
- Applied/consumed = delete the pending edge (queue maintenance), keeping the
  event node forever.

### 3.2 Tick membership (important boundary)

To get bit-identical results under permutations, the set of intents in each tick
must be the same across runs.

For tests like DIND "permute and converge," enforce one of:
- ingest all intents before starting ticks (single-tick bucket), or
- include an explicit tick tag/hint inside intent_bytes so membership is
  deterministic from content.

If membership differs, you changed causality, and bit-identical full graphs are
not expected.

## 4) Canonical ordering (and optional sequence assignment) algorithm

### 4.1 When ordering/seq is assigned

Canonical order is derived at the tick boundary for the set of pending intents
for that tick (or for the ledger segment being committed).

If you persist `seq` for auditing/debugging, it is assigned at the same boundary
and MUST be a deterministic function of the pending set.

### 4.2 Canonical ranking

Given a tick's pending set P:
1) Deduplicate by intent_id (idempotency).
2) Sort intents by intent_id ascending (bytewise).
3) (Optional) Assign seq = 1..|P| in that sorted order.

That is the canonical order.

Consequence: ingesting intents in any order yields the same seq assignments, the
same node/edge insertion schedule, and the same full hash.

### 4.3 Idempotency interaction

If an intent is re-ingested:
- compute intent_id
- if already present (committed or pending), return DUPLICATE + seq_assigned and
  DO NOT create a new inbox entry.

## 5) Scheduler: deterministic conflict resolution

### 5.1 Conflict detection

Two intents conflict if their computed footprints overlap per existing rules.

### 5.2 Deterministic tie-break key

When choosing a winner among conflicting candidates, the scheduler MUST use a
stable tie-break key independent of evaluation order.

Define:

priority_key(intent) = (
  priority_class,     // stable, explicit (e.g., system > user > background)
  intent_id           // stable content hash
)

Then:
- The winner is min(priority_key) (or max - pick one and freeze it).
- Losers are deferred to the next tick (or rejected) deterministically.

If you need multi-phase scheduling, extend the tuple, but every field must be
stable and derived from content/state in a canonical way.

### 5.3 Evaluation order must not leak

Even if you compute footprints in parallel, the final chosen schedule must be
equivalent to:
- consider all pending intents
- compute (or cache) footprints
- select winners using priority_key ordering only

No "first one we happened to see" logic.

## 6) Graph construction + hashing canonicalization

To make the whole WARP graph bit-identical, ensure these are canonical:
- node IDs for inbox entries: derive from intent_id (or from (tick_id, intent_id)),
  not from seq counters
- pending edge IDs: derive from (inbox_root, intent_id) or an equivalent stable function
- edge insertion order: if edges are stored in vectors/lists, insert in canonical
  sorted order
- attachment ordering: canonical sort
- any map iteration: never used directly; materialize keys, sort, then emit

This is the same "sorted enumeration" rule ADR-0003 already states for reads;
apply it everywhere hashing touches.

## 7) Required tests (prove it suite)

T1 - Permutation invariance (full hash)
- Take a fixed set S of canonical intent bytes.
- Run N seeds; each seed shuffles ingestion order of S.
- Enforce same tick membership (e.g., ingest all before first tick).
- Assert:
  - full graph hash identical across all seeds
  - ledger/inbox node IDs identical
  - seq assignments identical

T2 - Conflict invariance
- Construct S where at least two intents conflict (overlapping footprints).
- Shuffle ingestion order.
- Assert the same winner intent_id is chosen (and same losers deferred), across
  seeds.

T3 - Idempotency invariance
- Ingest same intent twice (different arrival times, different threads).
- Assert no duplicate ledger entry; same seq returned.

## 8) Implementation summary

Move seq assignment from "ingest arrival" to "tick commit canonicalization" and
make the scheduler's winner selection purely a function of stable keys
(priority_class + intent_id).
