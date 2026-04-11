<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# `echo-config-fs`

Filesystem-backed configuration adapter for Echo tools.

## What this crate does

- Implements `echo-app-core`'s `ConfigStore` using platform-specific
  configuration directories (via the `directories` crate) and JSON files.
- Provides a concrete `FsConfigStore` type that browser/host tools can use to
  persist:
    - local TTD/browser preferences,
    - host/runtime settings (for example browser bridge configuration).
- Keeps persistence details (paths, JSON encoding) out of UI and domain logic,
  which talk only to the abstract `ConfigStore` / `ConfigService` from
  `echo-app-core`.

## Documentation

- See the tool hexagon pattern and crate map in
  `docs/book/echo/sections/09-tool-hex-pattern.tex` (Echo Editor Tools,
  "Echo Tool Hexagon Pattern").
