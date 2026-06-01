<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# `wesley-gen` test loop speedup

Legend: `PLATFORM`

## What hurts

Round-trip on a single small `echo-wesley-gen` integration change today
looks like this:

- A single integration test (`tests/generation.rs::test_*`) routinely
  takes **60–120s** before it even reports pass/fail.
- A workspace `cargo build` from cold ran **7m41s** in the most recent
  `code-lawyer` review pass.
- A single `git commit` ran the pre-commit hook for **3m23s** before
  prettier triggered an abort-and-restage cycle, doubling the wait.

The compound effect: small mechanical PRs (one-line code review fixes)
take many minutes between intent and confirmation. The verification
loop is the bottleneck, not the code.

## Where the time goes

Profile of a single `tests/generation.rs::test_no_std_id_list_field_…`
run:

1. The test calls `Command::new("cargo").args(["run", "-p",
"echo-wesley-gen", …])` once per case. `cargo run` re-checks
   freshness across the whole dependency graph on every invocation,
   even when the binary is current.
2. `write_basic_generated_crate` writes the generated module to
   `target/echo-wesley-gen-basic-smoke/<PID>/<label>/`. The PID
   segment guarantees the per-test crate cannot share build artifacts
   with any other test, so `echo-wasm-abi`, `echo-registry-api`, and
   `serde` are recompiled from scratch every single time.
3. `assert_generated_crate_checks` runs `cargo check` against that
   per-PID crate. Because the crate has its own `[workspace]` block,
   it cannot reuse the parent workspace's target directory either.

Pre-commit hook profile (most recent run):

1. `verify-local cargo check -p echo-wesley-gen` ran in 4m53s on a
   cold cache. This re-checks the whole transitive dep graph for the
   crate, not the staged subset.
2. `prettier` runs over the full set of staged markdown files. When it
   has any reformatting to do, the hook aborts and the user must
   `git add -A` and re-commit — paying the cargo cost twice.

## Concrete wins (rank-ordered by yield/effort)

### 1. Share a target dir for generated consumer crates

Drop the `<PID>` segment from
`target/echo-wesley-gen-basic-smoke/<PID>/<label>/` and add a
`[workspace]` exclusion that points the generated crate's
`CARGO_TARGET_DIR` at a shared cache (e.g.
`target/echo-wesley-gen-basic-smoke-shared-target`). `echo-wasm-abi`,
`echo-registry-api`, and `serde` then compile **once** across all
integration tests instead of once per test.

Expected delta: 60–120s per test → 5–15s per test on warm cache.

Risk: tests that mutate the same `<label>` directory must be serialized
or use unique labels. The existing labels are already unique by test
name, so this is mostly a matter of dropping the PID segment safely.

### 2. Pre-build the `echo-wesley-gen` binary once per test binary

Replace each test's `Command::new("cargo").args(["run", "-p",
"echo-wesley-gen", …])` with a `OnceLock<PathBuf>` that runs
`cargo build -p echo-wesley-gen --release` (or `--profile dev`) on
first use, captures the binary path, then invokes that binary
directly. Subsequent tests skip the freshness re-check entirely.

Expected delta: ~3–5s per test invocation, multiplied by ~20+
integration tests.

Risk: low. The `OnceLock` ensures the binary is up-to-date with the
current source on the first run; subsequent runs in the same test
binary reuse it. If the source changes between `cargo test` runs the
binary is rebuilt automatically.

### 3. Scope the pre-commit hook to staged crates

The hook currently runs `cargo check -p echo-wesley-gen` regardless of
what's staged. It should:

- Parse `git diff --cached --name-only` for paths under `crates/<X>/`.
- For each unique `<X>`, run `cargo check -p <X>` (not the whole
  workspace) — and only if any source file in that crate is staged.
- Skip cargo entirely if only docs/markdown is staged.

Expected delta: 4m53s pre-commit → 0–60s for typical small PRs.

Risk: low for the obvious cases (single-crate edits, docs-only edits).
Cross-crate change sets still pay the full cost; that's correct.

### 4. cargo-nextest for integration tests

`cargo-nextest` runs each `#[test]` in its own process and parallelizes
them aggressively. The wesley-gen integration suite is structurally
parallel (each test compiles into its own consumer crate, no shared
mutable state if win #1 lands with disciplined labels), so nextest
should drop wall-clock from sequential to roughly `total / cores`.

Expected delta: linear speedup with available cores on the suite that
currently dominates wall-clock time.

Risk: introduces a new dev-tool dependency. CI would also need to learn
about it. Worth doing after wins #1–#3, not before.

### 5. Don't re-run cargo work after a prettier abort

The current hook flow is: cargo verify → markdown lint → prettier
fixup → abort → user restages → cargo verify AGAIN (from scratch).
The prettier abort path should either:

- Skip the cargo verify on the retry if nothing under `crates/`
  changed between the two stagings, or
- Run prettier first (cheap) and only run cargo if markdown formatting
  is already clean.

Expected delta: cuts the abort-cycle cost roughly in half for the
docs-touching commits that hit prettier.

Risk: requires re-ordering the hook script with care; the cargo gate
must still fire on any final pre-commit invocation that includes Rust
changes.

## Why it matters now

This was identified during the PR #382 review-resolution pass, where
the verification loop dominated the actual work. Every additional
review thread or follow-up commit pays the same cost, so the loop
amortizes badly across iterative review work — which is exactly the
high-frequency case we want to optimize for.

Win #1 alone would be a step-change in interactive iteration speed.
Wins #1 + #2 + #3 are mechanical, low-risk, and individually
shippable; they don't depend on each other.

## Suggested cycle shape

One cycle covering wins #1–#3 (target-dir sharing + binary
pre-build + scoped pre-commit) — all three are local to scripts and
test harness code, neither touches generated wire formats nor the
codec contract, so the determinism gates are not at risk. Win #4
(nextest) and win #5 (hook reorder) can ride as follow-ups once the
big rocks land.
