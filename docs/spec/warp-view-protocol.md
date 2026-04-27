<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# WARP View Protocol

_Define the current `echo-session-proto` packet schema for sharing renderable WARP readings._

Legend: PLATFORM

Depends on:

- [JS to Canonical CBOR Mapping](js-cbor-mapping.md)
- [SPEC-0009 - WASM ABI Contract](SPEC-0009-wasm-abi.md)

## Why this packet exists

Session tools need a small wire schema for snapshots and diffs over renderable graph types. This packet documents the implemented message set in `echo-session-proto`; it does not define a session hub, ownership service, or kernel replay protocol.

## Human users / jobs / hills

Human users need viewer tools that can reconnect to a stream and understand the message shape.

The hill: a tool can decode a packet, identify the WARP stream, and apply a snapshot or diff to its renderable view.

## Agent users / jobs / hills

Agent users need stable message names for automation and tests.

The hill: an agent can generate a handshake, subscribe to a WARP stream, and decode `warp_stream` frames using shared Rust types.

## Decision 1: This protocol carries readings, not kernel commits

The protocol transports renderable snapshots and diffs from `echo-graph`. Optional state hashes are integrity hints carried by those view types. Kernel replay and commit verification use tick patches and Merkle commits, not view-protocol frames.

## Decision 2: Packet framing is deterministic

Encoding uses canonical CBOR payloads inside the JS-ABI packet frame: `MAGIC || VERSION || FLAGS || LENGTH || PAYLOAD || CHECKSUM`.

## Decision 3: The message set is intentionally small

Current messages are `handshake`, `handshake_ack`, `subscribe_warp`, `warp_stream`, `notification`, and `error`. A `warp_stream` frame is either `Snapshot(WarpSnapshot)` or `Diff(WarpDiff)`.

## Decision 4: Compatibility aliases are decoder-only history

Encoders emit WARP names. Decoders may accept older RMG aliases where the Rust types intentionally support them. New documents and examples should use WARP names only.
