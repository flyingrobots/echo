<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# ADR-000X: Causality-First API — Ingress + MaterializationPort, No Direct Graph Writes

- **Status:** Implemented
- **Date:** 2026-01-14
- **Implementation:** 2026-01-17 (see `docs/rfc/mat-bus-finish.md`)

## Context

Echo/WARP is deterministic. The authoritative record is the **causality ledger** (intents/events). The **WARP state** is a projection of that ledger. We need:

- Local-in-browser (WASM) runtime mode
- Remote runtime mode (native + WebSocket)
- Generic devtools/inspectors
- High resiliency at the boundary (retries, disconnects)
- Strict determinism and provenance guarantees

We must prevent “god mode” APIs (direct graph mutation) from leaking to tools/JS.

## Decision

### 1) Ingress is the only write path (causality boundary)

All graph mutation is a consequence of ingested causal events.

Forbidden as public API:

- `apply(rule, scope)`
- `insert_node`, `delete_node`, `connect`, `disconnect`
- `tx_begin/commit/abort`

Allowed write API (canonical bytes only):

- `ingest_intent(intent_bytes) -> ACKI | ERR!`

Ingress assigns canonical sequence numbers. Tools do not.

### 2) UI is bus-first via MaterializationBus + MaterializationPort

Runtime emits derived updates on an internal **MaterializationBus** (channels). The boundary exposes a **MaterializationPort**:

- unidirectional (tick → emit → subscribers)
- ephemeral stream (must be subscribed)
- replay(1) per channel for late joiners (cache last finalized batch)

Consumers receive `MaterializationFrame` bytes; the bus stays internal.

#### Confluence-Safe Emission Semantics

**Critical distinction:**

- **Determinism**: Same bytes every run (timing-independent)
- **Confluence**: Same *meaning* regardless of rewrite order

The bus must satisfy both. Picking a "canonical winner" (e.g., max key) is deterministic but violates confluence by silently discarding values that should exist.

**Bus storage model (order-independent):**

```text
pending[channel][emit_key] = bytes
```

Where `EmitKey = (scope_hash, compact_rule_id)` — computable from executor context without scheduler internals.

**At finalize (post-commit):** all values emitted in deterministic order (by EmitKey).

#### Channel Policies

| Policy | Behavior | Use Case |
| ------ | -------- | -------- |
| `Log` (default) | All emissions in EmitKey order | Event streams, traces, multi-writer channels |
| `StrictSingle` | Error if >1 emission | Catch bugs; enforce single-writer semantically |
| `Reduce { join_fn }` | Merge via deterministic join function | Semantic coalescing with explicit merge logic |

**Banned:** Silent "winner picks" (e.g., max-key-wins). This violates confluence by hiding values.

**If you want single-value semantics:**

1. Use `StrictSingle` to catch violations, OR
2. Use footprints (`b_out`) to enforce single-writer at scheduling level, OR
3. Use `Reduce` with an explicit merge function

Footprints are optional and only for hard mutual-exclusion cases. Bus policies are semantic; scheduler conflicts are performance/control. Don't conflate.

### 3) One protocol, two transports

The same bytes-only framed protocol runs over:

- WASM exports returning `Uint8Array`
- WebSocket binary messages (same frames)

No JSON protocol.

### 4) Resiliency at the Port via idempotent ingress

`intent_id = H(intent_bytes)`.

Ingress dedupes `intent_id -> seq`:

- first ingest assigns seq (Accepted)
- retries return Duplicate + original seq

This provides at-least-once delivery with exactly-once causality.

### 5) Optional future: Causal Outbox (Egress) with causal acks

If needed for outbound side effects:

- ticks create outbox messages in state
- delivery agent sends (at-least-once)
- acks return via Ingress as intents
- `msg_id = H(message_bytes)` for idempotency

## Consequences

- Determinism protected by construction (no hidden mutation channels)
- Tools interact safely (intents + observation), not via engine primitives
- Local/remote modes share protocol and decoders
- UI can avoid direct state reads by consuming materializations
- Inspectors may still require gated read APIs (InspectorPort) for browsing

## Notes

“Causality” is the API. “State” is a projection.
