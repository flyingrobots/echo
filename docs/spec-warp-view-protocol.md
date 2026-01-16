<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# WARP View Protocol (WVP)
> **Background:** For a gentler introduction, see [WARP Primer](/guide/warp-primer).


A narrow, deterministic pub/sub protocol for sharing **WARP streams** (snapshots + diffs over a renderable graph) between tools via the session service.

This is an instantiation of the Echo Interaction Pattern (EIP): a small, boring, verifiable stream contract that tools can implement consistently.

## Goals

- Deterministic state sharing for a WARP stream across multiple tools/viewers.
- Publisher authority: only the owner of a `WarpId` may publish; others are read‑only.
- Gapless, hash‑checked stream of snapshots + diffs.
- Bounded, non‑blocking I/O suitable for desktop + wasm targets.

## Transport

- Encoding: canonical CBOR via `ciborium` (see `crates/echo-session-proto`).
- Framing: JS-ABI v1.0 packet framing (`MAGIC || VERSION || FLAGS || LENGTH || PAYLOAD || CHECKSUM`).
- MAX_PAYLOAD: 8 MiB (hard cap; errors if exceeded).
- Non‑blocking read loop; partial frames preserved until complete.

## Identity & Authority

- `WarpId`: publisher-chosen identifier for a WARP stream.
- `Producer`: the first connection that successfully publishes on a `WarpId`; only the producer may publish further frames for that `WarpId`.
- Any client may subscribe to any `WarpId`; writes from non‑producers are rejected with a protocol `Error`.

## Message Set (wire)

The current wire schema is intentionally small:

- `handshake` / `handshake_ack`
- `subscribe_warp { warp_id }`
- `warp_stream { warp_id, frame }`
  - `frame = Snapshot(WarpSnapshot) | Diff(WarpDiff)`
- `notification`
- `error`

See `crates/echo-session-proto/src/lib.rs` and `crates/echo-session-proto/src/wire.rs` for the canonical Rust types and op strings.

## Stream Semantics

- **Snapshots** are authoritative full-state resets for a `WarpId` at a specific epoch.
- **Diffs** must be gapless for live streams:
  - `from_epoch == last_epoch`
  - `to_epoch == from_epoch + 1`
- **Hashing:** `WarpSnapshot::state_hash` / `WarpDiff::state_hash` are optional. When present, consumers should verify them and treat mismatches as desync.

## Consumer Algorithm (per `WarpId`)

Tools that consume a WARP stream keep a per-`WarpId` state machine:

1. Start in **Idle** (no epoch).
2. On `Snapshot(epoch = e)`: set epoch to `e`, replace local graph, (optionally) verify `state_hash`.
3. On `Diff(from = e, to = e+1)`: apply ops; advance epoch; (optionally) verify `state_hash`.
4. Any gap or hash mismatch is a protocol error:
   - today: disconnect + reconnect (simple, explicit),
   - future: formalize a resync request flow (see backlog).

## Session Service Behavior (hub)

- Track producer per `WarpId`; reject non‑producer publishes with an `Error` response.
- Track the latest epoch and latest snapshot per `WarpId`:
  - when a subscriber subscribes and a snapshot exists, the hub sends the latest snapshot immediately.
- Validate gapless diffs from the producer; if a diff is non-sequential, reject and do not fan out.
- Fan‑out valid `warp_stream` frames and `notification` messages to subscribed clients via per-connection outboxes.

## Error Codes (draft)

- `ForbiddenPublish`: non‑producer attempted publish.
- `SnapshotRequired`: diff received before first snapshot for a stream.
- `EpochGap`: `from/to` not contiguous with server state.
- `Oversize`: payload exceeded MAX_PAYLOAD.
- `DecodeError`: CBOR/frame parse failure.

## Compatibility Notes

- The protocol prefers `subscribe_warp` / `warp_stream` op strings and `warp_id`.
- Decoders accept legacy aliases (`subscribe_rmg` / `rmg_stream` and `rmg_id`) during the WARP rename transition.

## Implementation Checklist (v0)

- [x] Define the WVP package: channel naming, `WarpId` + owner identity, publisher-only writes, snapshot + diff pattern, transport envelope.
- [x] Generalize as an Echo Interaction Pattern (EIP) template (roles, authority, message types, flow styles).
- [x] Enforce authority: session-service rejects non-owner writes; clients surface errors.
- [x] Dirty-flag sync loop in viewer (publish snapshot/diff on net tick when dirty; throttle/batch).
- [x] Publish/subscribe toggles in UI (enable/disable send/receive per `WarpId`).
- [x] Session-service wiring: publish endpoint, validate owner + gapless epochs, rebroadcast to subscribers.
- [x] Client wiring: bidirectional tool connection; surface authority/epoch errors as notifications.
- [x] Demo path: session-service + two viewers (publisher + subscriber) (`docs/guide/wvp-demo.md`).
- [x] Tests: authority rejection, gapless enforcement, dirty-loop behavior, toggle respect; integration test w/ two clients + loopback server. (Tracking: #169)
- [x] Docs sync: update GitHub Issues as slices land.

## Backlog / Open Questions

- Do we need explicit per-subscriber flow control (drop vs queue) or rely on reconnect + snapshot?
- Should producer ownership time out/expire to allow ownership handoff?
- Add a formal resync request message (consumer→hub→producer) once multi-tool workflows land.
- Encrypt/authenticate transport (TLS/Noise) in a later phase.
