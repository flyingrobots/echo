<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# `echo-graph`

Canonical renderable graph format for Echo (nodes/edges + payloads).

## What this crate does

- Defines the graph types used on the session wire and in visualizers:
    - `RenderGraph` with nodes, edges, and payload fields suitable for
      serialization and rendering.
    - `WarpOp`, `WarpSnapshot`, `WarpDiff`, `WarpFrame` for structural WARP updates
      (add/update/remove nodes/edges) and per-epoch snapshots/diffs.
- Provides helpers to hash graphs deterministically (via BLAKE3) for use in
  state verification and write-ahead logging.
- Is the shared graph representation between:
    - `echo-session-proto` (protocol/frame layer),
    - `ttd-browser`,
    - and other browser/host tools that need a renderable WARP view.

## Documentation

- Runtime and graph-carrier context: `docs/architecture/outline.md`.
- Core runtime tour: `docs/spec/warp-core.md`.
- WARP stream schema: `docs/spec/warp-view-protocol.md`.
- JS/CBOR encoding rules: `docs/spec/js-cbor-mapping.md`.
