<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Editor Hot-Reload

> **Milestone:** [Backlog](README.md) | **Priority:** Unscheduled

File-watching and hot-reload infrastructure for the editor/dev-server workflow. Enables rapid iteration on simulation schemas and scripts.

**Issues:** #24, #75, #76
**Chain:** #75 → #76 → T-10-4-3

## T-10-4-1: Draft Hot-Reload Spec (#75)

**User Story:** As a simulation developer, I want a hot-reload specification so that the reload behavior is well-defined and predictable (what reloads, what resets, what persists).

**Requirements:**

- R1: Define reloadable units (schema files, script files, config files)
- R2: Define reload semantics (full restart vs. incremental patch)
- R3: Define state preservation rules (which simulation state survives a reload)
- R4: Define error handling (what happens when a reloaded file has errors)
- R5: Specify the reload protocol between file watcher and runtime

**Acceptance Criteria:**

- [ ] AC1: Spec document exists at `docs/specs/SPEC-HOT-RELOAD.md`
- [ ] AC2: All five requirements are addressed
- [ ] AC3: At least two scenarios are worked through (schema change, script change)
- [ ] AC4: Spec reviewed by at least one contributor

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Specification only.
**Out of Scope:** Implementation, UI/UX design for reload indicators.

**Test Plan:**

- **Goldens:** n/a (spec document)
- **Failures:** n/a
- **Edges:** n/a
- **Fuzz/Stress:** n/a

**Blocked By:** none
**Blocking:** T-10-4-2

**Est. Hours:** 3h
**Expected Complexity:** ~250 lines (markdown)

---

## T-10-4-2: File Watcher / Debounce (#76)

**User Story:** As a simulation developer, I want a file watcher with debounce logic so that rapid saves don't trigger redundant reloads.

**Requirements:**

- R1: Watch a configurable set of directories/globs for changes
- R2: Debounce events with a configurable window (default 200ms)
- R3: Coalesce multiple file changes within the debounce window into a single reload event
- R4: Emit a `ReloadEvent` with the list of changed paths
- R5: Handle platform differences (fsevents on macOS, inotify on Linux)

**Acceptance Criteria:**

- [ ] AC1: `FileWatcher` struct with `watch(paths, debounce_ms)` API
- [ ] AC2: Debounce coalesces rapid saves into a single event
- [ ] AC3: Works on macOS and Linux (CI tests on both)
- [ ] AC4: Unit tests cover single change, rapid changes, and directory deletion

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** File watcher library, debounce logic, `ReloadEvent` type.
**Out of Scope:** Actual reload logic, editor integration, UI feedback.

**Test Plan:**

- **Goldens:** n/a
- **Failures:** Watched directory deleted, permission denied on file
- **Edges:** File changed exactly at debounce boundary, symlinked files, zero-length files
- **Fuzz/Stress:** Rapid-fire file writes (1000 changes in 1 second) to verify debounce

**Blocked By:** T-10-4-1
**Blocking:** T-10-4-3

**Est. Hours:** 5h
**Expected Complexity:** ~300 LoC

---

## T-10-4-3: Hot-Reload Implementation (#24)

**User Story:** As a simulation developer, I want the editor to automatically reload when I save a file so that I see changes reflected immediately without manual restart.

**Requirements:**

- R1: Integrate `FileWatcher` (T-10-4-2) with the runtime reload path
- R2: Implement state preservation as defined in the spec (T-10-4-1)
- R3: Display reload status (success/failure) via the CLI or dev-server
- R4: On reload error, keep the previous working state and display the error
- R5: Log all reload events with timing information

**Acceptance Criteria:**

- [ ] AC1: Saving a schema or script file triggers an automatic reload
- [ ] AC2: Reload preserves simulation state as defined in the spec
- [ ] AC3: A syntax error in a reloaded file shows an error but does not crash
- [ ] AC4: Integration test: modify file → assert reload event → assert new behavior

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Runtime reload integration, error recovery, status display.
**Out of Scope:** Editor GUI, browser-based hot-module-replacement, multi-file atomic reload.

**Test Plan:**

- **Goldens:** State snapshot before and after reload of a known schema change
- **Failures:** Reload with syntax error, reload with type error, reload with missing import
- **Edges:** Reload during active tick, reload with no actual changes (no-op)
- **Fuzz/Stress:** Continuous rapid reloads while simulation is running

**Blocked By:** T-10-4-2
**Blocking:** none

**Est. Hours:** 6h
**Expected Complexity:** ~400 LoC
