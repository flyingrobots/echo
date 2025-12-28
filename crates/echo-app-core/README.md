<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# `echo-app-core`

Shared application services and ports for Echo tools.

## What this crate does

- Defines core ports and services that UI/front-end crates can depend on without
  pulling in concrete frameworks:
  - `ConfigPort` and `ConfigService` for loading/saving structured settings and
    user preferences.
  - `ToastService` and related types for in-app notifications (info/warn/error).
  - `RenderPort` trait so tools can request redraws without depending directly
    on winit or other windowing APIs.
- Helps keep tool UIs (like `warp-viewer`) hexagonal: domain state and reducers
  talk to abstract ports; adapters live in separate crates (`echo-config-fs`,
  `warp-viewer`, etc.).

## Documentation

- High-level architecture and tool pattern: see `docs/book/echo/booklet-05-tools.tex`
  (Echo Editor Tools), especially Section `Echo Tool Hexagon Pattern`
  (`09-tool-hex-pattern.tex`).
