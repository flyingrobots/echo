# Telemetry: Graph Snapshot for Repro/Replay (Design Note)

Status: Draft • Scope: rmg-core (dev-only feature)

## Problem

When a conflict or unexpected outcome occurs during a transaction, logs with counts are helpful but insufficient for reproduction. We want the option to capture a minimal, deterministic snapshot of the reachable subgraph from `root` at key points (e.g., pre-commit or on conflict) so we can replay locally and bisect.

## Approach

- Add a feature-gated telemetry event `graph_snapshot` that emits the canonical, stable serialization of the reachable subgraph.
- Trigger points (feature-controlled):
  - On first conflict within a tx (sampled or rate-limited)
  - On commit (debug builds only)
- Consumers can store the JSONL stream and later reconstruct the exact state to reproduce behavior.

## Constraints

- Deterministic ordering and bytes: leverage the existing snapshot hash traversal and encoding rules. Do NOT invent a second ordering.
- Size control:
  - Emit only the reachable subgraph from `root`.
  - Optionally redact payloads or cap payload size via a `telemetry_max_payload_bytes` knob.
  - Allow sampling (e.g., `N` per minute) to keep overhead bounded.
- Security: feature must be off by default; never ship in production. Payloads may contain domain data.

## Event Shape (JSONL)

```
{
  "timestamp_micros": 1234567890,
  "tx_id": 42,
  "event": "graph_snapshot",
  "root": "<hex NodeId>",
  "snapshot_hash": "<hex blake3>",
  "nodes": [
    { "id": "<hex>", "ty": "<hex>", "payload": "<base64 or omitted>" }
  ],
  "edges": [
    { "id": "<hex>", "from": "<hex>", "to": "<hex>", "ty": "<hex>", "payload": "<base64 or omitted>" }
  ]
}
```

- Ordering: nodes ascending by `NodeId`, edges grouped by `from` with each group ascending by `EdgeId`.
- Payload encoding: identical to runtime wire format (length-prefixed little-endian), then base64 for JSON safety.

## API Sketch

- `telemetry::graph_snapshot(tx, &GraphStore, &root, redact_payloads: bool)`
- Compiles behind `feature = "telemetry"` only.
- Reuses internal snapshot traversal to ensure identical reachability set and order.

## Replay

- CLI helper (`rmg-cli`) to read JSONL and reconstruct an in-memory `GraphStore` for any `graph_snapshot` event.
- Verify by recomputing the `snapshot_hash` and comparing with the logged value.

## Next Steps

- [ ] Add serialization helper that walks the same reachable set as `compute_snapshot_hash`.
- [ ] Feature-gate emitting on conflict (first per tx) and on commit (debug only).
- [ ] CLI command: `rmg-cli replay --from telemetry.jsonl --tx 42`.
- [ ] Document redaction policy and sampling knobs.

