<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# echo-cli

Developer CLI for the Echo deterministic simulation engine.

## Installation

```sh
cargo install --path crates/warp-cli
```

The binary is named `echo-cli`.

## Subcommands

### `echo-cli verify <snapshot.wsc>`

Validate WSC snapshot integrity. Loads the file, validates structure, reconstructs the graph, and computes state root hashes.

```sh
# Verify a snapshot
echo-cli verify state.wsc

# Verify against a known hash (warp 0 only; additional warps report "unchecked")
echo-cli verify state.wsc --expected abcd1234...

# JSON output
echo-cli --format json verify state.wsc
```

### `echo-cli bench [--filter <pattern>] [--baseline <name>]`

Run Criterion benchmarks, parse JSON results, and format as an ASCII table.

```sh
# Run all benchmarks
echo-cli bench

# Filter by name
echo-cli bench --filter hotpath

# Compare current medians against perf-baseline.json
echo-cli bench --baseline main

# JSON output for CI
echo-cli --format json bench
```

### `echo-cli inspect <snapshot.wsc> [--tree]`

Display WSC snapshot metadata and graph statistics.

```sh
# Show metadata and stats
echo-cli inspect state.wsc

# Include ASCII tree of graph structure
echo-cli inspect state.wsc --tree

# JSON output
echo-cli --format json inspect state.wsc
```

## Global Flags

- `--format text|json` — Output format (default: `text`). Can appear before or after the subcommand.
- `--help` — Show help.
- `--version` — Show version.

## Man Pages

Generate man pages via xtask:

```sh
cargo xtask man-pages
# Output: docs/man/echo-cli.1, echo-cli-verify.1, etc.
```

## Documentation

See the root `README.md` and `docs/spec/` for architecture context.
