<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# WARP Stream Wire Protocol

> **Background:** For a gentler introduction, see [WARP Primer](/guide/warp-primer).
>
> **Status:** Current wire-schema reference for the WARP stream message types
> implemented by `crates/echo-session-proto`. This document is scoped to
> packet/message encoding and does not specify a session hub, viewer binary, or
> ownership service.

A narrow, deterministic packet schema for sharing **WARP streams**: snapshots
and diffs over the renderable graph types re-exported from `echo-graph`.

## Goals

- Stable message names for WARP stream snapshots/diffs.
- Canonical CBOR payloads with deterministic packet framing.
- Shared Rust types for browser/session-facing tools that consume
  `echo-session-proto`.

## Transport

- Encoding: canonical CBOR via `ciborium` (see `crates/echo-session-proto`).
- Framing: JS-ABI v1.0 packet framing (`MAGIC || VERSION || FLAGS || LENGTH || PAYLOAD || CHECKSUM`).
- Payload length: the JS-ABI packet uses a `u32` payload length and a checksum.
- `Packet::decode_envelope` validates magic, version, payload completeness, and checksum.

## Message Set

The current wire schema is intentionally small:

- `handshake` / `handshake_ack`
- `subscribe_warp { warp_id }`
- `warp_stream { warp_id, frame }`
    - `frame = Snapshot(WarpSnapshot) | Diff(WarpDiff)`
- `notification`
- `error`

See `crates/echo-session-proto/src/lib.rs` and `crates/echo-session-proto/src/wire.rs` for the canonical Rust types and op strings.

## Frame Semantics

- `WarpId` identifies the WARP stream.
- `WarpFrame` is either `Snapshot(WarpSnapshot)` or `Diff(WarpDiff)`.
- `WarpSnapshot` is a full renderable graph snapshot for an epoch.
- `WarpDiff` carries graph operations from one epoch to another.
- `state_hash` fields are optional integrity hints carried by `echo-graph`
  snapshot/diff types.

## Decoder Compatibility

- Encoders emit `subscribe_warp`, `warp_stream`, and `warp_id`.
- Decoders also accept `subscribe_rmg`, `rmg_stream`, and `rmg_id` because the
  Rust types intentionally carry those aliases.
