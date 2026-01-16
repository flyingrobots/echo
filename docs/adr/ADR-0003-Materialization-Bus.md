<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# ADR-000X: Causality-First API — Ingress + MaterializationPort, No Direct Graph Writes

- **Status:** Accepted
- **Date:** 2026-01-14
- **Deciders:** James Ross Ω (and the increasingly judgmental kernel)
- **Context:** Echo / WARP deterministic runtime; web + tooling + inspector ecosystem; strict determinism and provenance requirements.

---

## Context

Echo/WARP is a deterministic system where:

1. **WARP State** is a projection (a derived worldline snapshot).
2. **Causality** is the true API (the event/intent ledger that produces state).

We want:

- Generic web-based devtools/inspectors
- Local-in-browser runtime mode (WASM)
- Remote runtime mode (native process + WebSocket)
- High reliability at the boundary (retries, disconnects)
- **Zero non-deterministic mutation paths**
- Clean separation of concerns:
  - **warp-core stays pure**
  - boundary “weirdness” lives at ports/adapters

We reject exposing raw engine primitives (tx/apply/insert) to tools. Tools should not mutate state “directly”. They should emit causal events.

> Note: “causal” (not casual). The kernel is not wearing Crocs.

---

## Decision

### 1) All writes go through **Ingress** (the causal boundary)

**Rule:** _Literally everything that touches the graph MUST go through the inbox/ingress._

- No public `apply(rule, scope)`
- No public `insert_node(...)`
- No public `tx_begin/commit/abort`
- No direct graph mutations from JS/TS or tools

Instead, all writes are:

- `ingest_intent(intent_bytes)` (canonical bytes only)
- runtime assigns canonical sequence numbers
- kernel applies rewrites during ticks as internal mechanics

This makes the ingress/ledger the **API for causality**.

---

### 2) Reads are bus-first via **MaterializationBus**, crossed by a **MaterializationPort**

We distinguish:

- **MaterializationBus (internal runtime)**: derived updates emitted during ticks
- **MaterializationPort (boundary API)**: subscriptions, batching, replay(1), transport

For application UI, we prefer:
- “no direct reads” from WARP state (when feasible)
- UI driven by **bus materializations** (channels)

Direct state reads exist only for inspectors and are capability-gated.

---

### 3) Two transports share one protocol (bytes-only frames)

The same binary protocol runs over:

- **Local Mode:** WASM exports returning `Uint8Array`
- **Remote Mode:** WebSocket **binary** messages (same frames)

No JSON protocol. No stringly-typed nonsense at the boundary.

---

### 4) Resiliency (“Nine Tails”) lives at the Port via idempotent ingress

Retries must not create duplicate causality.

- Define `intent_id = H(intent_bytes)` (hash of canonical intent bytes)
- Ingress is **idempotent**: `intent_id -> seq_assigned`
- Retries return `DUPLICATE` + original seq (no new ledger entry)

This provides “at-least-once delivery” with “exactly-once causality”.

---

### 5) If/when needed, outbound side effects use a **Causal Outbox (Egress)**

We may later introduce:

- **EgressQueue / CausalOutbox**: outbound messages written into the graph during ticks
- Delivery agent sends messages (transport is at-least-once)
- **Acks are causal**: delivery success writes back via Ingress as an ack intent
- Idempotent key: `msg_id = H(message_bytes)`

This preserves determinism while supporting real-world IO.

---

## System Model

### Core entities

- **Ledger (Causality):** append-only input event stream
- **WARP State (Worldline):** deterministic projection of ledger
- **Ingress:** only way to add causal events
- **MaterializationBus:** ephemeral derived outputs from ticks
- **MaterializationPort:** subscription/bridge for bus delivery
- **InspectorPort:** optional direct state reads (gated)

---

## API Surfaces

### A) WarpIngress (write-only)

- `ingest_intent(intent_bytes) -> ACKI | ERR!`
- `ingest_intents(batch_bytes) -> ACKI* | ERR!` (optional later)

Notes:
- intent must be canonical bytes (e.g. `EINT` envelope v1)
- kernel assigns canonical `seq`
- idempotent by `intent_id = H(intent_bytes)`

---

### B) MaterializationPort (bus-driven reads)

The bus is:

- **Unidirectional:** `tick -> emit -> subscribers`
- **Ephemeral:** you must be subscribed to see the stream
- **Stateless relative to ledger:** not a source of truth
- **Replay(1) per channel:** caches last materialized value for late joiners

Port operations (conceptual):

- `view_subscribe(channels, replay_last) -> sub_id`
- `view_replay_last(sub_id, max_bytes) -> VOPS`
- `view_drain(sub_id, max_bytes) -> VOPS`
- `view_unsubscribe(sub_id)`

Channel identity:
- `channel_id = 32 bytes` (TypeId-derived; no strings in ABI)

View ops are deterministic and coalesced (recommended):
- last-write-wins per channel per tick

---

### C) InspectorPort (direct reads, gated)

Used only for devtools/inspectors; not required for app UI:

- `read_node(node_id) -> NODE | ERR!`
- `read_attachment(node_id) -> ATTM | ERR!`
- `read_edges_from(node_id, limit) -> EDGE | ERR!`
- `read_edges_to(node_id, limit) -> EDGE | ERR!`

All enumerations must be canonical order (sorted).

---

## Protocol: FrameV1 (bytes-only)

All boundary messages use the same framing:

```text
FrameV1:
  magic[4]   ASCII (e.g. 'VOPS', 'HEAD')
  ver_u16    = 1
  kind_u16   (optional if magic is sufficient; reserved for future)
  len_u32    payload byte length
  payload[len]
(all integers little-endian)
