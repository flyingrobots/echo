<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Developer Tooling

Priority: P0  
Status: Not Started  
Blocked By: Lock the Hashes

Objective: ship stable `echo-cli` developer workflows (`verify`, `bench`, `inspect`) with docs and man pages.

## Features

- [F6.1 CLI Scaffold](./F6.1-cli-scaffold.md) (Repo: Echo)
- [F6.2 Verify](./F6.2-verify.md) (Repo: Echo)
- [F6.3 Bench](./F6.3-bench.md) (Repo: Echo)
- [F6.4 Inspect](./F6.4-inspect.md) (Repo: Echo)
- [F6.5 Docs / Man Pages](./F6.5-docs-man-pages.md) (Repo: Echo)

## Exit Criteria

- `echo-cli verify|bench|inspect` CLI contract is stable.
- JSON and human-readable output modes are available for all subcommands.
- Man pages and README usage examples are generated and freshness-gated in CI.
