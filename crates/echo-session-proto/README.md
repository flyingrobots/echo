<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# `echo-session-proto`

Wire schema and deterministic encoding for Echo browser/session-facing
protocol surfaces.

## What this crate does

- Defines the logical protocol types used by Echo browser/session-facing tools:
    - `Message` enum (handshake, handshake_ack, error, subscribe_warp,
      warp_stream, notification).
    - `OpEnvelope` wrapper used as the CBOR payload for JS-ABI v1.0 packets.
    - Notification and WARP stream payload types (`Notification`,
      `WarpStreamPayload`).
    - `EINT v2` and `TTDR v2` frame types used by current TTD/browser-host
      surfaces.
    - Re-exports canonical WARP graph types from `echo-graph`.
- Implements deterministic JS-ABI v1.0 encoding for these messages:
    - Canonical CBOR value encoding with strict rules (no tags, definite
      lengths, preferred integer encodings, canonical map ordering).
    - Packet framing helpers (`wire` module) that build and verify
      `MAGIC || VERSION || FLAGS || LENGTH || PAYLOAD || CHECKSUM` packets.
- Serves as the retained protocol contract between:
    - `ttd-browser`,
    - current Echo-side browser-host bridge work,
    - and any remaining tooling that still needs these deterministic encodings
      during the `warp-ttd` cutover.

## Documentation

- The Echo book’s Core booklet (`docs/book/echo/booklet-02-core.tex`) covers
  these decisions in:
    - Section `Low-Level Networking: JS-ABI Wire Protocol`
      (`13-networking-wire-protocol.tex`),
    - Section `Consuming WARP Streams: Snapshots and Diffs`
      (`14-warp-stream-consumers.tex`).
- JS-ABI v1.0 encoding rules: `docs/js-cbor-mapping.md`.
