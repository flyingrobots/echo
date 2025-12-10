<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# `echo-graph`

Canonical renderable graph format for Echo (nodes/edges + payloads).

## What this crate does

- Defines the graph types used on the session wire and in visualizers:
  - `RenderGraph` with nodes, edges, and payload fields suitable for
    serialization and rendering.
  - `RmgOp`, `RmgSnapshot`, `RmgDiff`, `RmgFrame` for structural RMG updates
    (add/update/remove nodes/edges) and per-epoch snapshots/diffs.
- Provides helpers to hash graphs deterministically (via BLAKE3) for use in
  state verification and write-ahead logging.
- Is the shared graph representation between:
  - `echo-session-proto` (protocol layer),
  - `echo-session-service` (hub),
  - `rmg-viewer` and other tools that want to render or inspect RMGs.

## Documentation

- Conceptual background for RMG graphs and confluence:
  - `docs/spec-rmg-core.md`, `docs/spec-rmg-confluence.md`,
    and related RMG specs in `docs/`.
- The Core booklet (`docs/book/echo/booklet-02-core.tex`) uses these types in
  the JS-ABI + RMG streaming sections:
  - `13-networking-wire-protocol.tex`,
  - `14-rmg-stream-consumers.tex`.
