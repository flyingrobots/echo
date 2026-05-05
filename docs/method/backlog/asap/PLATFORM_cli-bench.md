<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

> **Milestone:** Developer CLI | **Priority:** P0

# bench (#49)

Run the warp-benches suite and present results.

Status: complete. `echo-cli bench` invokes `cargo bench -p warp-benches`,
supports `--filter`, parses `target/criterion/**/new/estimates.json`, emits
text/JSON summaries, and supports CLI-level baseline comparison through
`--baseline main` against `perf-baseline.json`. CI regression gating remains
handled by the G3 perf gate; the CLI comparison is a developer reporting
surface, not a duplicate release gate.

## T-6-3-1: Bench subcommand -- criterion invocation and reporting

Status: complete.

Implementation status: complete. `echo-cli bench` shells out through cargo,
collects nested Criterion estimates, formats text/JSON output, supports
Criterion regex filtering, and can annotate current medians with baseline,
delta, and status fields when `--baseline main` finds `perf-baseline.json`.
Missing baselines are reported explicitly while retaining absolute values.

Completion evidence:

- `crates/warp-cli/src/bench.rs` implements Criterion command construction,
  recursive estimate collection, table/JSON formatting, and optional baseline
  comparison.
- `crates/warp-cli/src/cli.rs` parses `--filter` and `--baseline` for the
  `bench` subcommand.
- Unit tests cover filter semantics, nested Criterion layouts, empty result
  handling, duration formatting, baseline deltas, new benchmarks, and missing
  baseline output.

**User Story:** As a developer, I want to run benchmarks from the CLI and see formatted results so that I can track performance without memorizing cargo commands.

**Requirements:**

- R1: `echo-cli bench [--filter <pattern>]` invokes `cargo bench -p warp-benches` as a subprocess.
- R2: Collect criterion JSON output from `target/criterion/`.
- R3: Format results as an ASCII table (bench name, mean, median, stddev) for text output.
- R4: `--format json` outputs a merged summary array from parsed Criterion estimates.
- R5: If CLI baseline comparison remains desired, add `--baseline <name>` and report percentage deltas against saved baseline data without duplicating the CI G3 gate.

**Acceptance Criteria:**

- [x] AC1: `echo-cli bench` runs all benchmarks and prints an ASCII table to stdout.
- [x] AC2: `echo-cli bench --filter snapshot` runs only benchmarks matching "snapshot".
- [x] AC3: `echo-cli bench --format json` outputs valid JSON.
- [x] AC4: `echo-cli bench --baseline main` shows percentage change columns when a baseline exists.

**Definition of Done:**

- [x] Code reviewed locally
- [x] Tests pass locally
- [x] Documentation updated

**Scope:** Subprocess invocation, criterion JSON parsing, table/JSON formatting, baseline comparison.
**Out of Scope:** CI integration (handled by existing GitHub Actions). Custom benchmark definitions. Flamegraph generation.

**Test Plan:**

- **Goldens:** ASCII table output for a mock criterion JSON fixture.
- **Failures:** `cargo bench` not found (clear error: "cargo not in PATH"). No benchmark results found (empty table with message).
- **Edges:** Filter that matches nothing (empty results). Baseline file missing (print "no baseline" and show absolute values only).
- **Fuzz/Stress:** N/A.

**Blocked By:** none (T-6-1-1 is implemented enough for current CLI dispatch)
**Blocking:** none

**Est. Hours:** 5h
**Expected Complexity:** ~250 LoC
