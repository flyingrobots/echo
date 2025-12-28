<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# `warp-viewer`

Interactive 3D viewer for Echo WARP streams.

## What this crate does

- Connects to the Echo session hub via `echo-session-client` and subscribes to
  one or more WARP streams (per-`WarpId`).
- Reconstructs the WARP graph using canonical `echo-graph` types and applies
  snapshots/diffs using the gapless, hash-checked semantics defined in
  `echo-session-proto`.
- Projects one or more WARPs into a 3D scene (a ring of WARP “spheres”) and
  renders them with WGPU (nodes, edges, debug overlays).
- Provides a minimal HUD and UI flow:
  - Title / Connecting / Error / Viewer screens,
  - settings and menu overlays,
  - toast notifications, perf stats, and controls legend.
- Acts as the reference implementation of the Echo tool hexagon pattern:
  - domain state and reducers in `UiState` / `ViewerState`,
  - side effects via ports:
    - config through `echo-app-core::ConfigPort` + `echo-config-fs`,
    - session through `echo-session-client::tool::SessionPort`,
    - redraws through `echo-app-core::render_port::RenderPort`.

## Documentation

- The Viewer’s state machine, UI, and WARP ring layout are described in:
  - `docs/book/echo/booklet-05-tools.tex`, Section
    `Echo WARP Viewer: State Machine & UI` (`08-warp-viewer-spec.tex`),
  - `docs/book/echo/booklet-05-tools.tex`, Section
    `Echo Tool Hexagon Pattern` (`09-tool-hex-pattern.tex`).
- The underlying session protocol and WARP streaming semantics are covered in
  the Core booklet (`docs/book/echo/booklet-02-core.tex`), Sections
  `13-networking-wire-protocol.tex` and `14-warp-stream-consumers.tex`.
