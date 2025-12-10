<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# `echo-session-proto`

Wire schema and deterministic encoding for the Echo session hub.

## What this crate does

- Defines the logical protocol types used by the session hub and tools:
  - `Message` enum (handshake, handshake\_ack, error, subscribe\_rmg,
    rmg\_stream, notification).
  - `OpEnvelope` wrapper used as the CBOR payload for JS-ABI v1.0 packets.
  - Notification and RMG stream payload types (`Notification`,
    `RmgStreamPayload`).
  - Re-exports canonical RMG graph types from `echo-graph`.
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

- Canonical JS-ABI v1.0 encoding and packet framing are specified in:
  - `docs/tex/sources/adr/adr-0013-*.tex` (deterministic encoding),
  - `docs/tex/sources/arch/arch-0013-*.tex` (wire protocol framing).
- The Echo book’s Core booklet (`docs/book/echo/booklet-02-core.tex`) mirrors
  these decisions in:
  - Section `Low-Level Networking: JS-ABI Wire Protocol`
    (`13-networking-wire-protocol.tex`),
  - Section `Consuming RMG Streams: Snapshots and Diffs`
    (`14-rmg-stream-consumers.tex`).
