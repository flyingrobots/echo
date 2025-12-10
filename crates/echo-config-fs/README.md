<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# `echo-config-fs`

Filesystem-backed configuration adapter for Echo tools.

## What this crate does

- Implements `echo-app-core`'s `ConfigStore` using platform-specific
  configuration directories (via the `directories` crate) and JSON files.
- Provides a concrete `FsConfigStore` type that tools like `rmg-viewer` and
  `echo-session-service` use to persist:
  - viewer preferences (camera pose, HUD toggles, vsync options),
  - host/service settings (e.g., socket path).
- Keeps persistence details (paths, JSON encoding) out of UI and domain logic,
  which talk only to the abstract `ConfigPort` / `ConfigService` from
  `echo-app-core`.

## Documentation

- See the tool hexagon pattern and crate map in
  `docs/book/echo/booklet-05-tools.tex` (Echo Editor Tools),
  Section `Echo Tool Hexagon Pattern` (`09-tool-hex-pattern.tex`).
