<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# `echo-session-proto`

Wire schema and deterministic encoding for the Echo session hub.

## What this crate does

- Defines the logical protocol types used by the session hub and tools:
    - `Message` enum (handshake, handshake_ack, error, subscribe_warp,
      warp_stream, notification).
    - `OpEnvelope` wrapper used as the CBOR payload for JS-ABI v1.0 packets.
    - Notification and WARP stream payload types (`Notification`,
      `WarpStreamPayload`).
    - Re-exports canonical WARP graph types from `echo-graph`.
- Implements deterministic JS-ABI v1.0 encoding for these messages:
    - Canonical CBOR value encoding with strict rules (no tags, definite
      lengths, preferred integer encodings, canonical map ordering).
    - Packet framing helpers (`wire` module) that build and verify
      `MAGIC || VERSION || FLAGS || LENGTH || PAYLOAD || CHECKSUM` packets.
- Serves as the shared protocol contract between:
    - `echo-session-service` (the hub),
    - `echo-session-client` (Unix socket client),
    - and any tools that want to talk to the session layer directly.

## Documentation

- The Echo book’s Core booklet (`docs/book/echo/booklet-02-core.tex`) covers
  these decisions in:
    - Section `Low-Level Networking: JS-ABI Wire Protocol`
      (`13-networking-wire-protocol.tex`),
    - Section `Consuming WARP Streams: Snapshots and Diffs`
      (`14-warp-stream-consumers.tex`).
- JS-ABI v1.0 encoding rules: `docs/js-cbor-mapping.md`.
