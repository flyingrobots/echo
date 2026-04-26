<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

> **Milestone:** Time Travel | **Priority:** P2

# TT1 — Streams Inspector Frame

Status: active but stale cool idea. Task DAG issues #170, #203, and
the #244-#246 set remain open, while #243 now has the fixed-timestep
invariant in `docs/invariants/FIXED-TIMESTEP.md`. No `StreamsFrame`,
inspector stream frame, or Constraint Lens UI exists yet. This card
remains operational as the handle for turning current
playback/provenance/checkpoint substrate into a stream/admission inspection
surface; it is not implemented protocol truth.

Define the stream/admission inspector surface and resolve the remaining TT1
design questions (#244, #245, #246) required before the time-travel MVP.

**Issues:** #170, #203, #243, #244, #245, #246

---

## T-7-2-1: Spec — dt policy: fixed timestep vs admitted dt stream (#243)

**User Story:** As an engine architect, I want a locked design decision on whether Echo uses a fixed timestep or variable dt admitted as a stream so that all downstream code (physics, animation, admission budgets) can commit to one model.

**Requirements:**

- R1: Treat `docs/invariants/FIXED-TIMESTEP.md` as the current decision
  artifact.
- R2: Confirm downstream TT1 work follows the locked decision: fixed
  timestep is default; `dt` is not an admitted stream fact.
- R3: Document any remaining catch-up/checkpoint implications in the
  time-model or retention follow-up docs rather than reopening dt policy.

**Acceptance Criteria:**

- [ ] AC1: The fixed-timestep invariant remains the normative decision.
- [ ] AC2: TT1 follow-up docs do not reintroduce per-tick or admitted
      variable `dt`.
- [ ] AC3: Catch-up/checkpoint implications are covered by the current
      time-model or retention docs.

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Verify downstream TT1 alignment with the fixed-timestep invariant.
**Out of Scope:** Reopening variable-dt support; changes to the scheduler.

**Test Plan:**

- **Goldens:** n/a (spec-only)
- **Failures:** n/a
- **Edges:** What happens when variable-dt admission is disabled mid-session (answer: revert to fixed timestep, document the transition).
- **Fuzz/Stress:** n/a

**Blocked By:** T-7-1-1, T-7-1-2
**Blocking:** T-7-2-4

**Est. Hours:** 3h
**Expected Complexity:** ~100 LoC (markdown)

---

## T-7-2-2: Spec — TimeStream retention, spool compaction, wormhole density (#244)

**User Story:** As an operator deploying Echo sessions, I want documented policies for how long TimeStream spools are retained, when compaction occurs, and how wormhole density is managed so that I can size storage and predict seek latency.

**Requirements:**

- R1: Define retention tiers: hot (in-memory ring buffer), warm (on-disk WAL), cold (CAS archive).
- R2: Specify compaction triggers: tick count threshold, byte budget, or explicit GC request.
- R3: Define wormhole density policy: minimum one wormhole checkpoint per N ticks (configurable), plus mandatory checkpoints at branch/merge points.
- R4: Document the relationship between retention and replay cost (seek latency formula).
- R5: Align with `docs/method/backlog/up-next/KERNEL_timestream-retention.md`
  and the existing `ProvenanceStore::checkpoint_before()` seam.

**Acceptance Criteria:**

- [ ] AC1: Retention tiers are defined with default thresholds.
- [ ] AC2: Compaction triggers are enumerable and configurable.
- [ ] AC3: Wormhole density policy includes a default N value and explains the tradeoff.
- [ ] AC4: A "replay cost" formula or heuristic is documented.

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Policy spec for retention, compaction, and wormhole density.
**Out of Scope:** Implementation of tiered storage (that is echo-cas work); GC runtime code.

**Test Plan:**

- **Goldens:** n/a (spec-only)
- **Failures:** n/a
- **Edges:** What happens when compaction runs during an active rewind (answer: compaction must not remove ticks reachable from any active view cursor).
- **Fuzz/Stress:** n/a

**Blocked By:** T-7-1-1, T-7-1-2
**Blocking:** T-7-2-4

**Est. Hours:** 4h
**Expected Complexity:** ~200 LoC (markdown)

---

## T-7-2-3: Spec — Merge semantics for admitted stream facts across worldlines (#245)

**User Story:** As a multiplayer game developer, I want clear merge semantics for when worldlines rejoin so that buffered "future" events are handled deterministically and I can reason about conflict resolution.

**Requirements:**

- R1: Define three merge strategies: discard-and-reattach, replay-and-revalidate, authority-wins.
- R2: Specify what happens to stream facts admitted on a diverged branch when merging back to canonical: revalidation rules, conflict detection, paradox quarantine.
- R3: Document the interaction between merge semantics and `admission_digest` (merged branch must produce a valid digest chain).
- R4: Provide a worked example: two peers diverge, one admits events the other did not, they merge.

**Acceptance Criteria:**

- [ ] AC1: Three merge strategies are defined with tradeoff analysis.
- [ ] AC2: Revalidation rules are specified for at least two stream types (NetworkRx, GameInput).
- [ ] AC3: Worked example covers diverge, independent admission, and merge with conflict.
- [ ] AC4: Conflict/quarantine behavior is cross-referenced to current
      worldline/provenance docs or a live follow-up card, not a retired spec
      path.

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Merge semantics spec for stream facts across worldlines.
**Out of Scope:** Runtime merge implementation; UI for conflict resolution.

**Test Plan:**

- **Goldens:** n/a (spec-only)
- **Failures:** n/a
- **Edges:** What if a merged branch contains an observation fact referencing a stream seq that the canonical branch also admitted at a different tick (answer: seq collision detection, documented).
- **Fuzz/Stress:** n/a

**Blocked By:** T-7-1-1, T-7-1-2
**Blocking:** T-7-3-1

**Est. Hours:** 4h
**Expected Complexity:** ~250 LoC (markdown)

---

## T-7-2-4: Spec — Security/capabilities for fork/rewind/merge in multiplayer (#246)

**User Story:** As a session host, I want a capability model that controls who can fork, rewind, and merge worldlines so that time-travel operations cannot be abused in multiplayer.

**Requirements:**

- R1: Define the capability names or rights model for fork, rewind, and
  merge, aligned with
  `docs/method/backlog/up-next/KERNEL_time-travel-capabilities.md`.
- R2: Specify per-session and per-player capability grants (host can restrict rewind to observers only, etc.).
- R3: Document provenance sovereignty: a player's forked branch carries their signer identity; merging requires authority from the branch owner or session host.
- R4: Define fault codes for unauthorized time-travel operations.

**Acceptance Criteria:**

- [ ] AC1: Time-travel capability names and denial faults are documented in
      the live capability follow-up.
- [ ] AC2: Per-session capability grant model is documented with example configurations.
- [ ] AC3: Provenance sovereignty rules are stated as normative requirements.
- [ ] AC4: At least 2 new fault codes are defined (e.g., `ERR_FORK_DENIED`, `ERR_MERGE_UNAUTHORIZED`).

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Capability model spec for time-travel operations.
**Out of Scope:** Runtime enforcement implementation; key management infrastructure.

**Test Plan:**

- **Goldens:** n/a (spec-only)
- **Failures:** n/a
- **Edges:** What happens when a player's capability is revoked while they have an active forked branch (answer: branch is quarantined, not silently destroyed).
- **Fuzz/Stress:** n/a

**Blocked By:** T-7-2-1, T-7-2-2
**Blocking:** T-7-3-1

**Est. Hours:** 4h
**Expected Complexity:** ~180 LoC (markdown)

---

## T-7-2-5: Implement StreamsFrame inspector support (#170)

**User Story:** As a developer debugging a live Echo session, I want an inspector frame that shows per-stream backlog, per-view cursor positions, and recent admission decisions so that I can understand why events are or are not entering the simulation.

**Requirements:**

- R1: Define the stream/admission frame shape in the inspector or observer
  protocol, covering `stream_id`, backlog metrics, cursor positions,
  recent admission-decision summaries, and `admission_digest`.
- R2: Add a streams/admission frame kind to the selected inspector envelope
  once that envelope exists.
- R3: Emit the frame at a deterministic tick boundary consistent with the
  current scheduler/playback emission order.
- R4: Serialize to JSONL for offline analysis; expose via WebSocket transport.
- R5: Add subscription/filter support for the streams/admission frame in the
  selected inspector command surface.

**Acceptance Criteria:**

- [ ] AC1: The streams/admission frame type compiles and is included in the
      selected inspector or observer module.
- [ ] AC2: A unit test constructs the frame with mock data and serializes it
      to JSON matching a golden snapshot.
- [ ] AC3: Integration test: run a 10-tick simulation with at least 2 streams, verify `StreamsFrame` is emitted each tick with correct backlog and cursor values.
- [ ] AC4: The selected inspector command surface accepts a streams/admission
      frame subscription or filter.

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Streams/admission frame shape, serialization, emission, and
subscription.
**Out of Scope:** UI rendering of streams data (that is T-7-2-6); checkpoint
density metrics (deferred to TT2).

**Test Plan:**

- **Goldens:** Golden JSON snapshot for streams/admission frame serialization
  (at least 2 streams, 3 recent decisions).
- **Failures:** Verify graceful handling when a stream has zero backlog; when a view has no cursor for a stream.
- **Edges:** Stream with exactly one event admitted at the current tick (boundary between empty and non-empty backlog).
- **Fuzz/Stress:** Property test: random stream/cursor configurations produce valid serialized frames.

**Blocked By:** T-7-2-1, T-7-2-2, T-7-2-3, T-7-2-4
**Blocking:** T-7-2-6, T-7-3-1

**Est. Hours:** 6h
**Expected Complexity:** ~400 LoC

---

## T-7-2-6: Implement Constraint Lens panel — admission explain-why + counterfactual sliders (#203)

**User Story:** As a designer tuning admission policies, I want a UI panel that explains why each event was admitted or rejected and lets me adjust policy parameters with counterfactual sliders so that I can iterate on admission budgets without modifying code.

**Requirements:**

- R1: Render recent stream/admission decision records from the inspector
  frame in a scrollable list with admit/reject status and reason summary.
- R2: Display the policy parameters (budget, fairness order) that were active for each decision.
- R3: Provide counterfactual sliders for `max_events`, `max_bytes`, and `max_work_units` that re-evaluate the most recent tick's admission decisions locally (read-only "what-if", no mutation of the simulation).
- R4: Highlight decisions that would change under the adjusted parameters.

**Acceptance Criteria:**

- [ ] AC1: Panel renders in the inspector UI with at least the last 10 admission decisions.
- [ ] AC2: Each decision shows: stream_id, admitted range, policy_hash, budget values, and admit/reject.
- [ ] AC3: Moving a counterfactual slider recomputes and highlights changed decisions within 100ms.
- [ ] AC4: Panel degrades gracefully when no streams/admission frame data is
      available (shows "no streams data" message).

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Constraint Lens panel UI; counterfactual re-evaluation of admission decisions.
**Out of Scope:** Persisting counterfactual parameter changes; applying adjusted parameters to future ticks; multi-tick counterfactual replay.

**Test Plan:**

- **Goldens:** Screenshot golden of panel with 3 streams, mixed admit/reject decisions.
- **Failures:** Panel with zero decisions; panel with a decision referencing a stream that no longer exists.
- **Edges:** Slider set to 0 (reject all); slider set to max u64 (admit all).
- **Fuzz/Stress:** Render 1000 decisions without UI freeze (< 16ms frame time).

**Blocked By:** T-7-2-5
**Blocking:** T-7-3-2

**Est. Hours:** 6h
**Expected Complexity:** ~500 LoC
