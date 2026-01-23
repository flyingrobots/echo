<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# SPEC-0004 Self-Review Hit List

**Date:** 2026-01-22
**Branch:** `graph-boaw`
**Status:** Pre-PR review complete

---

## Summary

| Category      | High  | Medium | Low    | Total  |
| ------------- | ----- | ------ | ------ | ------ |
| Source Code   | 0     | 7      | 36     | 43     |
| Test Code     | 1     | 8      | 18     | 27     |
| Documentation | 0     | 3      | 8      | 11     |
| API Surface   | 0     | 0      | 6      | 6      |
| **TOTAL**     | **1** | **18** | **68** | **87** |

---

## HIGH Severity

- [ ] **#53** Cross-file: Massive test helper duplication (~330 lines duplicated across 3 test files). `test_worldline_id`, `test_cursor_id`, `setup_worldline_with_ticks`, `create_add_node_patch`, etc. should be in `tests/common/mod.rs`.

---

## MEDIUM Severity

### Source Code

- [ ] **#1** `playback.rs:314` — Long mid-function comment block contradicts itself ("Actually, let's clarify..."). Clean up or move to module-level docs.
- [ ] **#2** `playback.rs:394` — `StepForward` for writers returns `StepResult::Advanced` but does nothing (misleading stub). Should return `NoOp` or document clearly.
- [ ] **#3** `playback.rs:566` — `publish_truth` hash conversion is fragile (relies on `blake3::Hash` to `[u8;32]` via `into()`). Add explicit type annotation.
- [ ] **#4** `provenance_store.rs:204` — `add_checkpoint` silently no-ops if worldline doesn't exist. Should return error or log.
- [ ] **#5** `provenance_store.rs:189` — `append()` doesn't validate `global_tick` equals current length (gap risk).
- [ ] **#6** `retention.rs:47` — `ArchiveToWormhole` is "not implemented" but no compile-time warning when used.
- [ ] **#7** `frame_v2.rs:111` — `debug_assert!` for payload size check. Release builds silently produce invalid packets if payload exceeds `u32::MAX`.

### Test Code

- [ ] **#8** `view_session_tests.rs:713` — T16 tests conceptually belong in BOAW test file, not "view sessions".
- [ ] **#9** `view_session_tests.rs:726` — `make_touch_rule` closure duplicated between T16 and T16-shuffled (47 lines x 2).
- [ ] **#10** `view_session_tests.rs:873` — `XorShift64` + `shuffle` reimplemented inline (duplicates `common/mod.rs`).
- [ ] **#11** `outputs_playback_tests.rs:92` — `setup_worldline_with_ticks` duplicated verbatim across 3 files.
- [ ] **#12** `outputs_playback_tests.rs:698` — Direct field mutation `cursor.tick = 100` bypasses public API.
- [ ] **#13** `checkpoint_fork_tests.rs:59` — `create_add_node_patch` duplicated verbatim.
- [ ] **#14** `reducer_emission_tests.rs:1254` — `bus_log` is non-mut but calls `emit()`. Misleading if interior mutability.
- [ ] **#15** `view_session_tests.rs:317` — Helper functions block (~110 lines) duplicated across 3 test files.

### Documentation

- [ ] **#16** `architecture-outline.md:125` — Says "`TruthSink` trait" but it's actually a `struct`.
- [ ] **#17** `architecture-outline.md:128` — `RetentionPolicy` variants listed incorrectly (says "Archival", missing `CheckpointEvery`).
- [ ] **#18** `architecture-outline.md:121` — Potentially broken link path (`/spec/` vs relative).

---

## LOW Severity

### Source Code (LOW)

- [ ] **#19** `playback.rs:264` — All `PlaybackCursor` fields are `pub` (risky for `store` field).
- [ ] **#20** `playback.rs:381` — Writer stub TODO not marked with `// TODO:` for grep-ability.
- [ ] **#21** `retention.rs:21` — Missing `#[non_exhaustive]` on `RetentionPolicy` enum.
- [ ] **#22** `worldline.rs:260` — `OutputFrameSet` type alias doesn't show docs in all IDE contexts. Consider newtype.
- [ ] **#23** `frame_v2.rs:149` — `decode_v2_packet` returns `Option` with no failure reason. Consider `Result<_, DecodeError>`.
- [ ] **#24** `frame_v2.rs:174` — Variable named `cursor` confusing given `CursorId` in crate. Rename to `offset`.
- [ ] **#25** `playback.rs:25` — `BTreeMap` imported at top but only used in `TruthSink`. Consider importing at point of use.
- [ ] **#26** `playback.rs:34` — `CursorId` and `SessionId` have identical `as_bytes` implementations. Consider macro/trait.
- [ ] **#27** `playback.rs:633` — `TruthSink::collect_frames` clones the entire Vec. Return `&[TruthFrame]` instead.
- [ ] **#28** `playback.rs:631` — Missing `#[must_use]` on `TruthSink::last_receipt`.
- [ ] **#29** `worldline.rs:145` — `#[allow(clippy::too_many_lines)]` on `apply_warp_op_to_store`. Consider refactoring.
- [ ] **#30** `worldline.rs:97` — Simple accessors (`global_tick()`, `policy_id()`) missing `#[inline]`.
- [ ] **#31** `worldline.rs:284` — `ApplyError::UnsupportedOperation` uses `&'static str`. Consider enum of op names.
- [ ] **#32** `provenance_store.rs:139` — `WorldlineHistory` is private but has doc comment. Consider removing.
- [ ] **#33** `provenance_store.rs:229` — `checkpoint()` does redundant `get_mut` after hash computation.
- [ ] **#34** `provenance_store.rs:254` — `#[allow(clippy::cast_possible_truncation)]` on `fork` needs safety comment.
- [ ] **#35** `provenance_store.rs:277` — Repeated `#[allow(clippy::cast_possible_truncation)]`. Consider module-level allow.
- [ ] **#36** `provenance_store.rs:317` — `checkpoint_before` returns `None` for non-existent worldline. Document behavior.
- [ ] **#37** `retention.rs:56` — `Default` impl could use `#[derive(Default)]` with `#[default]` attribute.
- [ ] **#38** `frame_v2.rs:102` — Multiple `#[allow(clippy::cast_possible_truncation)]` in `encode_v2_packet`.
- [ ] **#39** `frame_v2.rs:225` — `decode_v2_packets` creates subslice then re-checks length inside decode. Minor inefficiency.
- [ ] **#40** `playback.rs:559` — `publish_truth` error doc references `HistoryError` inconsistently.

### Test Code (LOW)

- [ ] **#41** `view_session_tests.rs:82` — Magic number `patch_digest: [tick as u8; 32]` wraps at tick > 255.
- [ ] **#42** `view_session_tests.rs:119` — Magic number `+100` offset for `commit_hash` unexplained.
- [ ] **#43** `view_session_tests.rs:145` — Magic number `10` for `pin_max_tick` not named.
- [ ] **#44** `view_session_tests.rs:719` — `WORKER_COUNTS` uses `[1,2,8,32]` vs `common` uses `[1,2,4,8,16,32]`.
- [ ] **#45** `view_session_tests.rs:232` — Loop count `5` is a magic number.
- [ ] **#46** `outputs_playback_tests.rs:3` — `#![allow(clippy::expect_fun_call)]` is file-wide. Scope to specific functions.
- [ ] **#47** `outputs_playback_tests.rs:427` — Magic number `k = 12u64` — why 12?
- [ ] **#48** `playback_cursor_tests.rs:21` — `test_cursor_id()` has different signature than other test files. Prevents extraction.
- [ ] **#49** `playback_cursor_tests.rs:256` — Unused variable `_hash_at_3` computed but never asserted.
- [ ] **#50** `playback_cursor_tests.rs:207` — "Tick 10 is valid" reasoning unclear. Document convention.
- [ ] **#51** `reducer_emission_tests.rs:29` — `key_sub as key` shadows 2-arg `key` function. Confusing.
- [ ] **#52** `reducer_emission_tests.rs:43` — `factorial` overflow guard uses `debug_assert!`. Use `assert!`.
- [ ] **#53** `reducer_emission_tests.rs:176` — Redundant re-assertion after loop (same check inside and after).
- [ ] **#54** `reducer_emission_tests.rs:539` — Double-finalization pattern (wasteful and confusing).
- [ ] **#55** `checkpoint_fork_tests.rs:9` — `#![allow(clippy::unwrap_used)]` is file-wide.
- [ ] **#56** `checkpoint_fork_tests.rs:135` — `cursor_tick = patch_index + 1` convention is fragile.
- [ ] **#57** Missing edge case tests: `pin_max_tick=0`, seek to `u64::MAX`, empty worldline, duplicate WorldlineId registration.
- [ ] **#58** `outputs_playback_tests.rs:623` — `unsubscribed_channel` variable name is redundant with test logic.

### Documentation (LOW)

- [ ] **#59** CHANGELOG claims "T19-T22" but these labels don't appear in test file names.
- [ ] **#60** `code-map.md` says "T1-T10 playback tests" but file has T1,T4,T5,T6,T7,T8 (not T2,T3,T9,T10).
- [ ] **#61** CHANGELOG `checkpoint()` description says "Create checkpoint" but function is `add_checkpoint`.
- [ ] **#62** CHANGELOG claims `WorldlineId` is "content-addressed" but tests use fixed bytes.

### API Surface

- [ ] **#63** `RetentionPolicy` exported but no public function accepts/returns it (dangling export).
- [ ] **#64** `apply_warp_op_to_store` exposes internal mutation without guardrails.
- [ ] **#65** `ApplyError` vs `ApplyResult` naming creates cognitive collision (different contexts).
- [ ] **#66** `compute_state_root_for_warp_store` newly public — low-level, easy to misuse.
- [ ] **#67** `CheckpointRef` exposed publicly but only meaningful in provenance context.
- [ ] **#68** `playback` module exports 11 types in a flat list. Consider sub-grouping in docs.

---

## Recommended Fix Priority

### P0 — Before PR

- [ ] Fix #53 (HIGH): Extract shared test helpers to `tests/common/mod.rs`
- [ ] Fix #16-#18 (MEDIUM): Factual errors in `architecture-outline.md`
- [ ] Fix #9-#10 (MEDIUM): Use existing `common/` XorShift64/shuffle/make_touch_rule

### P1 — Before Merge

- [ ] Fix #1-#2 (MEDIUM): Clean up playback.rs stub and comments
- [ ] Fix #5 (MEDIUM): Add tick gap validation to `append()`
- [ ] Fix #7 (MEDIUM): Promote `debug_assert!` to runtime check in frame_v2

### P2 — Follow-up Issue

- [ ] Fix #4 (MEDIUM): Error handling in `add_checkpoint`
- [ ] Fix #8 (MEDIUM): Move T16 to appropriate test file
- [ ] Fix #21 (LOW): Add `#[non_exhaustive]` to `RetentionPolicy`

### P3 — Tech Debt

- [ ] All remaining LOW severity items
