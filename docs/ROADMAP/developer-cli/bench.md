<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

> **Milestone:** [Developer CLI](README.md) | **Priority:** P0

# bench (#49)

Run the warp-benches suite and present results.

## T-6-3-1: Bench subcommand -- criterion invocation and reporting

**User Story:** As a developer, I want to run benchmarks from the CLI and see formatted results so that I can track performance without memorizing cargo commands.

**Requirements:**

- R1: `echo-cli bench [--filter <pattern>]` invokes `cargo bench -p warp-benches` as a subprocess.
- R2: Collect criterion JSON output from `target/criterion/`.
- R3: Format results as an ASCII table (bench name, samples, mean, median, stddev) for text output.
- R4: `--format json` outputs the raw criterion JSON merged into a single array.
- R5: `--baseline <name>` compares against a saved baseline and reports deltas (percentage change).

**Acceptance Criteria:**

- [ ] AC1: `echo-cli bench` runs all benchmarks and prints an ASCII table to stdout.
- [ ] AC2: `echo-cli bench --filter snapshot` runs only benchmarks matching "snapshot".
- [ ] AC3: `echo-cli bench --format json` outputs valid JSON.
- [ ] AC4: `echo-cli bench --baseline main` shows percentage change columns when a baseline exists.

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Subprocess invocation, criterion JSON parsing, table/JSON formatting, baseline comparison.
**Out of Scope:** CI integration (handled by existing GitHub Actions). Custom benchmark definitions. Flamegraph generation.

**Test Plan:**

- **Goldens:** ASCII table output for a mock criterion JSON fixture.
- **Failures:** `cargo bench` not found (clear error: "cargo not in PATH"). No benchmark results found (empty table with message).
- **Edges:** Filter that matches nothing (empty results). Baseline file missing (print "no baseline" and show absolute values only).
- **Fuzz/Stress:** N/A.

**Blocked By:** T-6-1-1
**Blocking:** none

**Est. Hours:** 5h
**Expected Complexity:** ~250 LoC
