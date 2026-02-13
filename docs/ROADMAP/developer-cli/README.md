<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Developer CLI

> **Priority:** P0 | **Status:** Not Started | **Est:** ~30h

Ship stable `echo-cli` developer workflows (`verify`, `bench`, `inspect`) with docs and man pages. The CLI provides the primary developer interface for validating simulation determinism, running benchmarks, and inspecting snapshot state from the terminal.

**Blocked By:** Lock the Hashes

## Exit Criteria

- [ ] `echo verify` validates simulation determinism from CLI
- [ ] `echo bench` runs benchmarks with JSON + human-readable output
- [ ] `echo inspect` dumps simulation state for debugging
- [ ] Man pages and usage examples committed
- [ ] CLI contract documented (stable subcommands, exit codes)

## Features

| Feature        | File                                   | Est. | Status      |
| -------------- | -------------------------------------- | ---- | ----------- |
| CLI Scaffold   | [cli-scaffold.md](cli-scaffold.md)     | ~6h  | Not Started |
| verify         | [verify.md](verify.md)                 | ~5h  | Not Started |
| bench          | [bench.md](bench.md)                   | ~5h  | Not Started |
| inspect        | [inspect.md](inspect.md)               | ~9h  | Not Started |
| Docs/man pages | [docs-man-pages.md](docs-man-pages.md) | ~5h  | Not Started |
