<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# PR #257 Review Issues Tracker

Tracking all CodeRabbit review feedback for systematic resolution.

**Legend:**

- Priority: `P0` Critical, `P1` Major, `P2` Minor, `P3` Trivial/Nitpick
- Status: `OPEN`, `FIXED`, `WONTFIX`, `FALSE_POSITIVE`

---

## Code Issues

| #   | File                                           | Line | Issue                                                          | Priority | Status         | Notes                                                        |
| --- | ---------------------------------------------- | ---- | -------------------------------------------------------------- | -------- | -------------- | ------------------------------------------------------------ |
| C01 | `crates/warp-core/tests/boaw_footprints.rs`    | -    | `is_multiple_of` is nightly-only method                        | P0       | FALSE_POSITIVE | Project uses Rust 1.90 where it's stable; clippy enforces it |
| C02 | `crates/warp-core/src/boaw/exec.rs`            | 35   | Missing `Debug` derive on `ExecItem`                           | P3       | OPEN           |                                                              |
| C03 | `crates/warp-core/src/boaw/exec.rs`            | 82   | Mismatch between docs and runtime check for stride fallback    | P3       | OPEN           |                                                              |
| C04 | `crates/warp-core/src/boaw/exec.rs`            | 118  | Hard-fail on `workers == 0` in public sharded/stride APIs      | P1       | OPEN           |                                                              |
| C05 | `crates/warp-core/src/boaw/merge.rs`           | 52   | Don't silently drop ops if origins are missing                 | P2       | OPEN           |                                                              |
| C06 | `crates/warp-core/src/boaw/shard.rs`           | 54   | Add compile-time guard for power-of-two shard count            | P3       | OPEN           |                                                              |
| C07 | `crates/warp-core/src/snapshot_accum.rs`       | 256  | DeleteNode must also purge incident edges                      | P1       | OPEN           | state_root mismatch risk                                     |
| C08 | `crates/warp-core/src/snapshot_accum.rs`       | 369  | Don't silently drop WSC serialization errors                   | P2       | OPEN           |                                                              |
| C09 | `crates/warp-core/src/snapshot_accum.rs`       | 734  | Potential state_root drift: edge hashing skips zero-edge nodes | P1       | OPEN           |                                                              |
| C10 | `crates/warp-core/src/snapshot_accum.rs`       | 833  | Missing behavioral tests for new accumulator                   | P1       | OPEN           |                                                              |
| C11 | `crates/warp-core/src/tick_delta.rs`           | 124  | Determinism hole when ops share same `sort_key`                | P1       | OPEN           |                                                              |
| C12 | `crates/warp-core/tests/boaw_cow.rs`           | 115  | Ignored `unimplemented!` tests need follow-up plan             | P2       | OPEN           |                                                              |
| C13 | `crates/warp-core/tests/boaw_determinism.rs`   | 81   | Silent no-op when WSC bytes absent is test smell               | P2       | OPEN           |                                                              |
| C14 | `crates/warp-core/tests/boaw_determinism.rs`   | 187  | Test name says "admission" but only validates hashes           | P3       | OPEN           |                                                              |
| C15 | `crates/warp-core/tests/boaw_end_to_end.rs`    | 26   | BOAW harness not yet wired                                     | P0       | OPEN           | Tracking issue needed                                        |
| C16 | `crates/warp-core/tests/boaw_end_to_end.rs`    | 77   | Magic numbers undocumented (20, 42)                            | P3       | OPEN           |                                                              |
| C17 | `crates/warp-core/tests/boaw_end_to_end.rs`    | 140  | Conflict scenario only tests at fixed worker count 8           | P2       | OPEN           |                                                              |
| C18 | `crates/warp-core/tests/boaw_footprints.rs`    | 171  | Shard routing test asserts tautology                           | P3       | OPEN           |                                                              |
| C19 | `crates/warp-core/tests/boaw_parallel_exec.rs` | 78   | Needs investigation (truncated)                                | P0       | OPEN           |                                                              |
| C20 | `crates/warp-core/tests/boaw_parallel_exec.rs` | 362  | Hardcoded worker counts instead of WORKER_COUNTS constant      | P3       | OPEN           |                                                              |
| C21 | `crates/warp-core/tests/boaw_privacy.rs`       | 12   | Unused import: `mod common;`                                   | P2       | OPEN           |                                                              |
| C22 | `crates/warp-core/tests/common/mod.rs`         | 146  | Public helper `hex32` missing rustdoc                          | P2       | OPEN           |                                                              |
| C23 | `crates/warp-core/tests/common/mod.rs`         | 597  | Misleading comment: "NoMatch expected" but any error panics    | P2       | OPEN           |                                                              |
| C24 | `crates/warp-core/tests/common/mod.rs`         | 609  | `state_root` is wrong (commit hash != state root)              | P1       | OPEN           |                                                              |
| C25 | `crates/warp-core/tests/common/mod.rs`         | 658  | Needs investigation (truncated)                                | P2       | OPEN           |                                                              |
| C26 | `crates/warp-core/tests/common/mod.rs`         | -    | TODO returns all-zero hash; hides failures                     | P2       | OPEN           |                                                              |
| C27 | `crates/warp-core/tests/common/mod.rs`         | -    | Silent error discarding in `apply` loop                        | P2       | OPEN           |                                                              |
| C28 | `crates/echo-dind-harness/tests/coverage.rs`   | 75   | `hex::encode(SCHEMA_HASH)` double-encodes hex string           | P0       | OPEN           |                                                              |
| C29 | `crates/echo-dind-harness/tests/coverage.rs`   | -    | Environment variable check overly permissive                   | P2       | OPEN           |                                                              |
| C30 | `crates/echo-dind-harness/tests/coverage.rs`   | -    | Silent success message lacks context for CI                    | P3       | OPEN           |                                                              |
| C31 | `crates/echo-dind-tests/src/rules.rs`          | 137  | Needs investigation (truncated)                                | P1       | OPEN           |                                                              |

## Documentation Issues

| #   | File                                                | Line | Issue                                                    | Priority | Status         | Notes                             |
| --- | --------------------------------------------------- | ---- | -------------------------------------------------------- | -------- | -------------- | --------------------------------- |
| D01 | `README.md`                                         | -    | "worker-count invariant" undefined jargon                | P1       | OPEN           |                                   |
| D02 | `CHANGELOG.md`                                      | -    | Duplicate "Unreleased" sections                          | P2       | OPEN           |                                   |
| D03 | `docs/adr/ADR-0007-BOAW-Storage.md`                 | 173  | Spelling: "refcounts" -> "ref counts"                    | P3       | OPEN           |                                   |
| D04 | `docs/adr/ADR-0007-BOAW-Storage.md`                 | 282  | Commit-hash formula diverges from compute_commit_hash_v2 | P1       | OPEN           |                                   |
| D05 | `docs/adr/ADR-0007-PART-6-FREE-MONEY.md`            | 285  | Spec/code mismatch on `MergeConflict` payload            | P2       | OPEN           |                                   |
| D06 | `docs/spec-warp-core.md`                            | -    | Needs investigation (truncated)                          | P2       | OPEN           |                                   |
| D07 | `docs/memorials/2026-01-18-phase4-rubicon.md`       | -    | Replace "diffed" with clearer verb                       | P3       | OPEN           |                                   |
| D08 | `docs/study/what-makes-echo-tick.md`                | 536  | BOAW expansion incorrect                                 | P2       | FIXED          | Changed to "Best Of All Worlds"   |
| D09 | `docs/study/what-makes-echo-tick.tex`               | 871  | BOAW expansion incorrect                                 | P2       | FIXED          | Changed to "Best Of All Worlds"   |
| D10 | `docs/study/what-makes-echo-tick-processed.md`      | 536  | BOAW expansion incorrect                                 | P2       | FIXED          | Changed to "Best Of All Worlds"   |
| D11 | `docs/study/what-makes-echo-tick-with-diagrams.tex` | 697  | BOAW expansion incorrect                                 | P2       | FIXED          | Changed to "Best Of All Worlds"   |
| D12 | `docs/study/echo-tour-de-code-directors-cut.tex`    | 819  | BOAW expansion already correct                           | P2       | FALSE_POSITIVE | Already says "Best Of All Worlds" |
| D13 | `docs/study/echo-tour-de-code-directors-cut.tex`    | 851  | Invalid Rust syntax in pseudocode                        | P2       | OPEN           |                                   |
| D14 | `docs/study/echo-tour-de-code-directors-cut.tex`    | 1146 | Incorrect claim: Rust clone() is NOT COW                 | P1       | OPEN           |                                   |
| D15 | `docs/study/echo-tour-de-code-with-commentary.tex`  | 115  | `\ding{46}` used before pifont loaded                    | P1       | OPEN           | LaTeX will explode                |
| D16 | `docs/study/echo-tour-de-code-with-commentary.tex`  | 182  | Remove redundant `\usepackage{pifont}`                   | P2       | OPEN           |                                   |
| D17 | `docs/study/echo-tour-de-code-with-commentary.tex`  | 808  | Radix sort pass math inconsistent                        | P2       | OPEN           |                                   |
| D18 | `docs/study/echo-tour-de-code-with-commentary.tex`  | 963  | BOAW expansion incorrect                                 | P2       | OPEN           | Still says wrong thing            |
| D19 | `docs/study/echo-visual-atlas-with-diagrams.tex`    | 120  | Needs investigation (truncated)                          | P0       | OPEN           |                                   |
| D20 | `docs/study/echo-visual-atlas-with-diagrams.tex`    | -    | Date inconsistency                                       | P2       | OPEN           |                                   |
| D21 | `docs/study/echo-visual-atlas-with-diagrams.tex`    | -    | Vacuous frontmatter/backmatter declarations              | P3       | OPEN           |                                   |

## Book/Tour Documentation Issues

| #   | File                                                   | Line | Issue                                           | Priority | Status         | Notes                             |
| --- | ------------------------------------------------------ | ---- | ----------------------------------------------- | -------- | -------------- | --------------------------------- |
| B01 | `docs/book/echo/sections/01-what-is-echo.tex`          | 7    | Double space before "(WARP)"                    | P3       | OPEN           |                                   |
| B02 | `docs/book/echo/sections/01-what-is-echo.tex`          | 12   | Inconsistent capitalization: "State" vs "state" | P3       | OPEN           |                                   |
| B03 | `docs/book/echo/sections/01-what-is-echo.tex`          | 22   | Ambiguous terminology: "commit ledger"          | P2       | OPEN           |                                   |
| B04 | `docs/book/echo/sections/15-boaw-storage.tex`          | 96   | Needs investigation (truncated)                 | P0       | OPEN           |                                   |
| B05 | `docs/book/echo/sections/16-tour-overview.tex`         | 11   | Hardcoded date will rot                         | P3       | OPEN           |                                   |
| B06 | `docs/book/echo/sections/16-tour-overview.tex`         | 70   | Brittle line-number reference in protip         | P3       | OPEN           |                                   |
| B07 | `docs/book/echo/sections/16-tour-overview.tex`         | -    | Needs investigation (truncated)                 | P1       | OPEN           |                                   |
| B08 | `docs/book/echo/sections/17-tour-intent-ingestion.tex` | 73   | Needs investigation (truncated)                 | P2       | OPEN           |                                   |
| B09 | `docs/book/echo/sections/18-tour-boaw-execution.tex`   | 11   | BOAW expansion correct here                     | -        | FALSE_POSITIVE | Already says "Best of All Worlds" |
| B10 | `docs/book/echo/sections/19-tour-transaction.tex`      | 25   | Line 715 reference doesn't match code           | P2       | OPEN           |                                   |
| B11 | `docs/book/echo/sections/20-tour-rule-matching.tex`    | 35   | Needs investigation (truncated)                 | P2       | OPEN           |                                   |
| B12 | `docs/book/echo/sections/20-tour-rule-matching.tex`    | 67   | Needs investigation (truncated)                 | P0       | OPEN           |                                   |
| B13 | `docs/book/echo/sections/20-tour-rule-matching.tex`    | 87   | Needs investigation (truncated)                 | P0       | OPEN           |                                   |
| B14 | `docs/book/echo/sections/20-tour-rule-matching.tex`    | 93   | Bloom-filter analogy needs clarification        | P3       | OPEN           |                                   |
| B15 | `docs/book/echo/sections/20-tour-rule-matching.tex`    | -    | Brittle line-number references                  | P3       | OPEN           |                                   |
| B16 | `docs/book/echo/sections/22-tour-delta-merge.tex`      | 15   | Needs investigation (truncated)                 | P0       | OPEN           |                                   |
| B17 | `docs/book/echo/sections/22-tour-delta-merge.tex`      | 65   | Needs investigation (truncated)                 | P2       | OPEN           |                                   |
| B18 | `docs/book/echo/sections/23-tour-hashing.tex`          | 87   | Needs investigation (truncated)                 | P1       | OPEN           |                                   |
| B19 | `docs/book/echo/sections/23-tour-hashing.tex`          | 115  | Needs investigation (truncated)                 | P0       | OPEN           |                                   |
| B20 | `docs/book/echo/sections/24-tour-commit.tex`           | 54   | Needs investigation (truncated)                 | P2       | OPEN           |                                   |
| B21 | `docs/book/echo/sections/25-tour-call-graph.tex`       | -    | Complexity: Drain is O(n), not O(n log n)       | P2       | OPEN           |                                   |
| B22 | `docs/book/echo/booklet-06-tour-de-code.tex`           | 20   | Put `\appendix` before appendix content         | P2       | OPEN           |                                   |
| B23 | `docs/book/echo/colophon-06-tour.tex`                  | 45   | "Phase 6 planning docs" is vague                | P3       | OPEN           |                                   |
| B24 | `docs/book/echo/colophon-06-tour.tex`                  | 55   | Needs investigation (truncated)                 | P1       | OPEN           |                                   |
| B25 | `docs/book/echo/free-money.tex`                        | -    | Fix build command filename                      | P2       | OPEN           |                                   |
| B26 | `docs/book/echo/parts/part-06-tour-de-code.tex`        | 10   | Out-of-order section sequencing                 | P2       | OPEN           |                                   |

## Diagram Issues

| #   | File                              | Line | Issue                                          | Priority | Status         | Notes               |
| --- | --------------------------------- | ---- | ---------------------------------------------- | -------- | -------------- | ------------------- |
| M01 | `docs/study/diagrams/tour-02.mmd` | -    | Clarify apply+hash are commit internals        | P2       | OPEN           |                     |
| M02 | `docs/study/diagrams/tour-03.mmd` | -    | Missing trailing newline                       | P3       | FALSE_POSITIVE | Already has newline |
| M03 | `docs/study/diagrams/tour-04.mmd` | -    | Missing trailing newline                       | P3       | FALSE_POSITIVE | Already has newline |
| M04 | `docs/study/diagrams/tour-10.mmd` | 6    | `S2[...]` is placeholder masquerading as shard | P2       | OPEN           |                     |
| M05 | `docs/study/diagrams/tour-10.mmd` | 15   | Define Shard 3 explicitly                      | P2       | OPEN           |                     |
| M06 | `docs/study/diagrams/tour-10.mmd` | 29   | Missing trailing newline                       | P3       | FALSE_POSITIVE | Already has newline |
| M07 | `docs/study/diagrams/tour-11.mmd` | -    | Missing trailing newline                       | P3       | FALSE_POSITIVE | Already has newline |
| M08 | `docs/study/diagrams/tour-12.mmd` | -    | Missing trailing newline                       | P3       | FALSE_POSITIVE | Already has newline |
| M09 | `docs/study/diagrams/tour-13.mmd` | 7    | Missing trailing newline                       | P3       | FALSE_POSITIVE | Already has newline |
| M10 | `docs/study/diagrams/tour-13.mmd` | -    | BLAKE3 label vague                             | P3       | OPEN           |                     |

## Script Issues

| #   | File                            | Line | Issue                                    | Priority | Status | Notes                    |
| --- | ------------------------------- | ---- | ---------------------------------------- | -------- | ------ | ------------------------ |
| S01 | `docs/study/build-tour.py`      | 41   | Escape order corrupts `\textbackslash{}` | P0       | OPEN   |                          |
| S02 | `docs/study/build-tour.py`      | 57   | Needs investigation (truncated)          | P1       | OPEN   |                          |
| S03 | `docs/study/build-tour.py`      | 153  | Needs investigation (truncated)          | P1       | OPEN   |                          |
| S04 | `docs/study/build-tour.py`      | 195  | xelatex return code ignored              | P1       | OPEN   | Stale PDFs mask failures |
| S05 | `docs/study/build-tour.py`      | 202  | main() missing return type annotation    | P3       | OPEN   |                          |
| S06 | `docs/study/build-tour.py`      | 240  | f-string without placeholders            | P3       | OPEN   |                          |
| S07 | `scripts/bench_accumulate.py`   | 91   | Needs investigation (truncated)          | P1       | OPEN   |                          |
| S08 | `scripts/bench_accumulate.py`   | -    | Needs investigation (truncated)          | P0       | OPEN   |                          |
| S09 | `scripts/bench_report_local.py` | -    | Needs investigation (truncated)          | P2       | OPEN   |                          |

## Web/HTML Issues

| #   | File                                   | Line | Issue                            | Priority | Status | Notes |
| --- | -------------------------------------- | ---- | -------------------------------- | -------- | ------ | ----- |
| W01 | `docs/benchmarks/index.html`           | 76   | Toggle buttons need ARIA state   | P2       | OPEN   |       |
| W02 | `docs/benchmarks/index.html`           | 168  | Guard CI formatting to avoid NaN | P2       | OPEN   |       |
| W03 | `docs/benchmarks/report-inline.html`   | 76   | Toggle buttons need ARIA state   | P2       | OPEN   |       |
| W04 | `docs/benchmarks/report-inline.html`   | 172  | Guard CI formatting to avoid NaN | P2       | OPEN   |       |
| W05 | `docs/benchmarks/parallelism-study.md` | 83   | Needs investigation (truncated)  | P0       | OPEN   |       |
| W06 | `docs/benchmarks/parallelism-study.md` | 110  | Needs investigation (truncated)  | P1       | OPEN   |       |

## LaTeX Class Issues

| #   | File                  | Line | Issue                              | Priority | Status | Notes |
| --- | --------------------- | ---- | ---------------------------------- | -------- | ------ | ----- |
| L01 | `docs/study/aion.cls` | 152  | Public DOI metadata is no-op       | P2       | OPEN   |       |
| L02 | `docs/study/aion.cls` | 172  | Empty TOC placeholder is tech debt | P3       | OPEN   |       |
| L03 | `docs/study/aion.cls` | -    | Fragile emptiness check            | P1       | OPEN   |       |
| L04 | `docs/macros.tex`     | -    | Needs investigation (truncated)    | P2       | OPEN   |       |

## Config/Misc Issues

| #   | File                          | Line | Issue                                        | Priority | Status | Notes                    |
| --- | ----------------------------- | ---- | -------------------------------------------- | -------- | ------ | ------------------------ |
| X01 | `.markdownlint.json`          | -    | Disabling MD045 removes alt-text enforcement | P2       | OPEN   | Accessibility regression |
| X02 | `testdata/dind/*.hashes.json` | -    | Missing trailing newlines                    | P3       | OPEN   |                          |

---

## Summary

| Priority    | Total | Open | Fixed | Won't Fix | False Positive |
| ----------- | ----- | ---- | ----- | --------- | -------------- |
| P0 Critical | 12    | 11   | 0     | 0         | 1              |
| P1 Major    | 19    | 19   | 0     | 0         | 0              |
| P2 Minor    | 35    | 31   | 4     | 0         | 0              |
| P3 Trivial  | 24    | 17   | 0     | 0         | 7              |
| Total       | 90    | 78   | 4     | 0         | 8              |

---

Last updated: 2026-01-19
