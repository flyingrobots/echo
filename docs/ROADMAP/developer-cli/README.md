<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Developer CLI

> **Priority:** P0 | **Status:** Verified (2026-03-06) | **Est:** ~30h
> **Evidence:** PR [#288](https://github.com/flyingrobots/echo/pull/288), PR [#290](https://github.com/flyingrobots/echo/pull/290)

Ship stable `echo-cli` developer workflows (`verify`, `bench`, `inspect`) with docs and man pages. The CLI provides the primary developer interface for validating simulation determinism, running benchmarks, and inspecting snapshot state from the terminal.

**Blocked By:** Lock the Hashes

## Exit Criteria

- [x] `echo verify` validates simulation determinism from CLI
- [x] `echo bench` runs benchmarks with JSON + human-readable output
- [x] `echo inspect` dumps simulation state for debugging
- [x] Man pages and usage examples committed
- [x] CLI contract documented (stable subcommands, exit codes)

## Features

| Feature        | File                                   | Est. | Status   |
| -------------- | -------------------------------------- | ---- | -------- |
| CLI Scaffold   | [cli-scaffold.md](cli-scaffold.md)     | ~6h  | Verified |
| verify         | [verify.md](verify.md)                 | ~5h  | Verified |
| bench          | [bench.md](bench.md)                   | ~5h  | Verified |
| inspect        | [inspect.md](inspect.md)               | ~9h  | Verified |
| Docs/man pages | [docs-man-pages.md](docs-man-pages.md) | ~5h  | Verified |
