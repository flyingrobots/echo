<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

> **Milestone:** Time Travel | **Priority:** P2
>
> Status: active cool idea with partial substrate. `PlaybackCursor`,
> checkpoint-backed seek, provenance checkpoints/forks, `ttd-browser`, and the
> TTD UI shell exist. This card tracks the missing pause/buffer/catch-up
> capability semantics and integrated Reliving debugger behavior, not basic
> replay existence.

# TT2 — Time Travel MVP

Core time travel: pause simulation while tools stay live, rewind/fork, and
buffered catch-up via checkpoint-backed replay. Plus the Reliving debugger MVP
for timeline scrubbing and causal slicing.

**Issues:** #171, #205

---

## T-7-3-1: Implement time travel core — pause/rewind/buffer/catch-up (#171)

**User Story:** As a developer, I want to pause the simulation (while inspector/tools remain live), rewind to an earlier tick, fork a new worldline, and catch up via checkpoints so that I can debug temporal bugs without restarting the session.

**Requirements:**

- R1: Define and implement pause-buffer admission policy: freeze
  simulation-view cursors while tool-view cursors remain live; stream events
  accumulate in backlog.
- R2: Implement a capability-gated `Fork(worldline, tick)` operation that
  creates a new worldline fork/branch at the specified tick.
- R3: Implement `Rewind(view, tick)` that switches a view to an existing
  worldline/tick, loading from the nearest materialized checkpoint when
  available.
- R4: Implement `CatchUp(view, target_tick, budget)` that fast-forwards a view
  with checkpoint-backed replay, stopping when the target is reached or the
  compute budget is exhausted.
- R5: All operations emit deterministic decision/provenance records into the
  authoritative worldline history.
- R6: Inspector/stream frames reflect paused/buffered state accurately during
  time travel.

**Acceptance Criteria:**

- [ ] AC1: Unit test: pause a 2-stream simulation at tick 50, verify tool-view cursors advance while sim-view cursors are frozen.
- [ ] AC2: Unit test: fork at tick 20, advance the fork to tick 25 with independent state, verify original worldline is unaffected.
- [ ] AC3: Integration test: rewind from tick 100 to tick 10 using a checkpoint, verify state matches the original tick-10 snapshot hash.
- [ ] AC4: Integration test: catch-up from tick 10 to tick 100 via
      checkpoint-backed replay, verify final state hash matches the original
      tick-100 commit.
- [ ] AC5: Capability test: fork without the required time-travel capability
      returns a deterministic denial error.

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Pause-buffer policy, Fork, Rewind, CatchUp operations with capability
checks.
**Out of Scope:** Merge operations (deferred to TT3); UI for time travel (that is T-7-3-2); distributed catch-up across network peers.

**Test Plan:**

- **Goldens:** Snapshot hash comparison: tick-10 state via rewind must match tick-10 state from original run (bit-exact).
- **Failures:** Fork at tick beyond current head (error); rewind to tick with no
  available checkpoint (graceful fallback to full replay); catch-up with zero
  budget (immediate return, no progress).
- **Edges:** Fork at tick 0 (genesis); rewind to current tick (no-op); catch-up when already at target.
- **Fuzz/Stress:** Property test: fork-then-catchup from random tick pairs produces state hashes matching the original worldline.

**Blocked By:** T-7-2-5, T-7-2-3, T-7-2-4
**Blocking:** T-7-3-2, T-7-4-1

**Est. Hours:** 6h
**Expected Complexity:** ~600 LoC

---

## T-7-3-2: Implement Reliving debugger MVP — scrub timeline + causal slice + fork branch (#205)

**User Story:** As a developer debugging a simulation, I want a timeline scrubber that lets me move to any tick, view the causal slice (which events caused the current state), and fork a new branch from any point so that I can explore "what if" scenarios interactively.

**Requirements:**

- R1: Timeline scrubber UI component: horizontal bar showing tick range with
  checkpoint markers; drag to seek to any tick.
- R2: Causal slice panel: given the current tick and a selected entity/node,
  show the chain of admission decisions and graph rewrites that contributed to
  its current state (walk the provenance/worldline history backward).
- R3: "Fork from here" button: creates a new worldline branch at the scrubber's
  current tick (delegates to T-7-3-1 Fork operation).
- R4: Visual indicator showing which worldline branch the view is currently on (branch name/id + divergence point).
- R5: Scrubber seek uses materialized checkpoints for ticks that are not
  in-memory (delegates to T-7-3-1 Rewind/CatchUp).

**Acceptance Criteria:**

- [ ] AC1: Scrubber renders a 1000-tick timeline with at least 5 checkpoint
      markers.
- [ ] AC2: Dragging the scrubber to tick N loads state within 200ms (using nearest checkpoint).
- [ ] AC3: Causal slice for a selected node shows at least the admission decision and rewrite that last modified it.
- [ ] AC4: "Fork from here" creates a branch and switches the view to it; the branch indicator updates.
- [ ] AC5: Scrubber works correctly when the simulation is paused (no advancing ticks).

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Timeline scrubber, causal slice panel, fork-from-here action, branch indicator.
**Out of Scope:** Full causal graph visualization (DAG rendering); merge UI; multi-branch scrubber overlay.

**Test Plan:**

- **Goldens:** Screenshot golden of scrubber at tick 500 with checkpoint markers and branch indicator.
- **Failures:** Scrub to a tick where the checkpoint is corrupted (show error, do not crash); causal slice on a node with no history (empty state).
- **Edges:** Scrub to tick 0 (genesis); scrub to the head tick; fork from genesis.
- **Fuzz/Stress:** Rapid scrubbing across 10,000 ticks without UI freeze (debounced seek, < 16ms frame time).

**Blocked By:** T-7-3-1, T-7-2-6
**Blocking:** T-7-4-1

**Est. Hours:** 6h
**Expected Complexity:** ~550 LoC
