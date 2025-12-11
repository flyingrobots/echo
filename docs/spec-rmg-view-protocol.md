<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# RMG View Protocol (RVP)

A narrow, deterministic pub/sub protocol for sharing Render Mesh Graphs (RMGs) between clients via the session service. It is an instantiation of the Echo Interaction Pattern (EIP) template; see the template section below for reuse in future services.

## Goals
- Deterministic state sharing for RMG timelines across multiple viewers.
- Publisher authority: only the owner of an RmgId may write; others are read‑only.
- Gapless, hash‑checked stream of snapshots + diffs.
- Bounded, non‑blocking I/O suitable for desktop + wasm targets.

## Transport
- Encoding: canonical CBOR via `ciborium` (same rules as `spec-serialization-protocol`).
- Framing: header + payload + checksum (current session wire framing).
- MAX_PAYLOAD: 8 MiB (hard cap; errors if exceeded).
- Non‑blocking read loop; partial frames preserved until complete.

## Identity & Authority
- `ClientId`: assigned by session service at connect (opaque).
- `RmgId`: publisher-chosen identifier for an RMG timeline.
- `Owner`: the `ClientId` that registered/published the RmgId first; only the owner may send writes on that channel.
- Any client may subscribe to any RmgId; writes from non‑owners are rejected with `Forbidden`.

## Message Pattern
- Channels: one logical channel per `RmgId`.
- Types (direction server↔client):
  - `RegisterRmg { rmg_id, initial_epoch, hash }` (client→server)
  - `Snapshot { rmg_id, epoch, hash, bytes }` (owner→server→subscribers)
  - `Diff { rmg_id, from_epoch, to_epoch, hash, bytes }` (owner→server→subscribers)
  - `Ack { rmg_id, epoch, hash }` (server→owner; optional telemetry)
  - `Error { rmg_id, code, detail }` (server→owner)
  - `ResyncRequest { rmg_id, expected_epoch, expected_hash }` (server→owner, subscriber→server optional)
- Gaplessness: server enforces `to_epoch` == `last_epoch + 1` per RmgId per subscriber.
- Hashing: canonical hash of post-apply graph; server validates owner frames before fan‑out.
- Backpressure: if subscriber lag or hash mismatch, server sends `ResyncRequest`; owner must send fresh `Snapshot`.

## Client Behavior (viewer)
- Maintains dirty flag per RmgId. On local mutation: `dirty = true` with next epoch candidate.
- Net tick: if `dirty` and publish enabled, send `Diff` (or `Snapshot` if no base) and clear `dirty` on success; retry/backoff on error.
- Subscriptions: apply incoming Snapshot/Diff only if gapless and hash matches; otherwise request resync.
- UX toggles: per‑RmgId `publish_enabled`, `receive_enabled`; when receive is re‑enabled, request resync to current epoch.

## Session Service Behavior
- Track owner per RmgId; reject non‑owner publishes with `Error{Forbidden}`.
- Validate gapless epochs and hashes from owner; on mismatch reply `Error{HashMismatch}` and drop frame.
- Fan‑out valid Snapshot/Diff to all subscribers except sender.
- On subscriber hash/epoch mismatch, send `ResyncRequest` to owner and optionally drop or buffer until fixed.

## Error Codes (draft)
- `Forbidden`: non‑owner attempted publish.
- `HashMismatch`: provided hash does not match recomputed state.
- `EpochGap`: `from/to` not contiguous with server state.
- `Oversize`: payload exceeded MAX_PAYLOAD.
- `DecodeError`: CBOR/frame parse failure.

## Echo Interaction Pattern (EIP) Template
Use this skeleton for future services:
- **Roles:** publisher/authority, subscribers/consumers (and optional mediating hub).
- **Identity:** opaque `ClientId`, resource identifier (`RmgId` here).
- **Authority rule:** who may write, who may read; enforcement location.
- **Message set:** register, snapshot, diff/update, ack, error, resync.
- **Validation:** gapless sequencing, content hash, size caps, decode checks.
- **Transport:** canonical encoding, framing, MAX payload, non‑blocking I/O.
- **Backpressure/recovery:** resync requests, retries, drop/flush policy.
- **Toggles:** per‑resource publish/receive enable.
- **Observability:** emit errors + resync events for metrics/logging.

## Open Questions
- Do we need per-subscriber flow control (drop vs queue) or rely on resend-on-resync only?
- Should owner registration time out/expire to allow ownership handoff?
- Encrypt/authenticate transport (TLS/Noise) in this phase or later?

