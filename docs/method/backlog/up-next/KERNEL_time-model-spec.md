<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

> **Milestone:** Time Semantics Lock | **Priority:** P1

# TT0 — Time Model Spec Lock

Lock the vocabulary and semantics for HistoryTime vs HostTime, tick-based TTL/deadlines, and the StreamAdmissionDecision digest chain. This is a spec-only feature — no runtime code, only documents and their review artifacts.

**Issues:** #191, #192

---

## T-7-1-1: Spec — HistoryTime vs HostTime field classification (#191)

Status: complete.

Resolution: compressed and superseded by current invariant doctrine. The stale
task check found that the original target path,
`docs/spec-time-streams-and-wormholes.md`, no longer exists in the current spec
architecture. The invariant itself is now carried in
`docs/invariants/FIXED-TIMESTEP.md`: R5 covers tick-denominated TTL/deadline
semantics, R6 forbids HostTime from directly affecting admission, commit
identity, read identity, replay outcome, or causal ordering, and the added
"Time field classification" section names the current obvious HistoryTime and
HostTime fields. Static enforcement is covered by `scripts/ban-nondeterminism.sh`
and the allowlist governance in `docs/determinism/RELEASE_POLICY.md`; runtime
schema comments also state that `WorldlineTick` and `GlobalTick` have no
wall-clock semantics.

The old full-table ritual is intentionally not resurrected. Future protocol
work should extend the current invariant or runtime schema fragments, not create
`docs/spec-time-streams-and-wormholes.md` solely to satisfy this stale card.

Completion evidence:

- `docs/invariants/FIXED-TIMESTEP.md` now contains the compact
  HistoryTime/HostTime classification table and the strengthened decision-record
  rule.
- `scripts/tests/fixed_timestep_invariant_test.sh` now checks the
  HistoryTime/HostTime language and verifies that the static nondeterminism
  guard bans wall-clock APIs.
- `scripts/ban-nondeterminism.sh` bans `SystemTime`, `Instant`, sleep calls, and
  other nondeterministic APIs across determinism-critical crate paths.

**User Story:** As a contributor implementing time-aware adapters, I want a single authoritative document that classifies every session-stream time field as HistoryTime (deterministic, ordering/replay) or HostTime (telemetry only) so that I never accidentally introduce nondeterminism through a time field.

**Requirements:**

- R1: Produce a table in `docs/spec-time-streams-and-wormholes.md` listing every known time field across `StreamAdmissionDecision`, `ClockDecision`, `EventEnvelope`, `InspectorEnvelope`, and session-proto messages.
- R2: Each field is classified as HistoryTime or HostTime with a one-line rationale.
- R3: Add a "decision record" rule: any adapter that consults HostTime must emit a canonical decision record before the simulation consumes the result.
- R4: Cross-reference `docs/spec/merkle-commit.md` for fields that feed into `admission_digest`.

**Acceptance Criteria:**

- [x] AC1: Superseded. The classification table exists in the current invariant
      doc, `docs/invariants/FIXED-TIMESTEP.md`, rather than the obsolete
      `docs/spec-time-streams-and-wormholes.md` path.
- [x] AC2: The compact table classifies every field it names and leaves no TBD
      entries.
- [x] AC3: The decision-record rule is stated as a normative MUST/MUST NOT
      requirement.
- [x] AC4: Superseded. The compressed stale-task audit checked the
      classification against `echo-session-proto`, generated TTD protocol
      surfaces, runtime schema fragments, and the static wall-clock ban script.
      No separate reviewer confirmation was claimed for this closure.

**Definition of Done:**

- [x] Code reviewed locally
- [x] Tests pass locally
- [x] Documentation updated

**Scope:** Classification of existing time fields; normative HistoryTime/HostTime rule.
**Out of Scope:** Runtime enforcement of the rule; new adapter code; changes to `echo-session-proto` wire format.

**Test Plan:**

- **Goldens:** n/a (spec-only)
- **Failures:** n/a
- **Edges:** Verify that `sender_tick` in `EventEnvelope` is correctly classified as HistoryTime (not HostTime), even though it originates from a remote host.
- **Fuzz/Stress:** n/a

**Blocked By:** none
**Blocking:** T-7-1-2, T-7-2-1

**Est. Hours:** 3h
**Expected Complexity:** ~150 LoC (markdown)

---

## T-7-1-2: Spec — TTL/deadline semantics are ticks only (#192)

**User Story:** As a game designer using Echo, I want certainty that all TTL and deadline semantics use deterministic tick/epoch counts so that my game logic replays identically regardless of host performance.

**Requirements:**

- R1: Add a normative section to `docs/spec-time-streams-and-wormholes.md` stating that all TTL and deadline semantics use Chronos ticks or epoch counts, never wall-clock durations.
- R2: Document the "timer as stream" pattern: a system requests a timer via a deterministic event; an adapter fires it; the simulation consumes the recorded firing decision.
- R3: Enumerate known TTL/deadline touch points (session keep-alive, admission budgets, retry policies, wormhole expiry) and confirm each is tick-denominated.
- R4: Add a "violation checklist" — signs that wall-clock time has leaked into semantic state.

**Acceptance Criteria:**

- [ ] AC1: Normative "no wall-clock TTL" rule is present in the spec.
- [ ] AC2: Timer-as-stream pattern is documented with a minimal worked example.
- [ ] AC3: At least 4 known TTL/deadline touch points are enumerated and confirmed tick-only.
- [ ] AC4: Violation checklist has at least 3 items.

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Spec text and worked example for tick-only deadlines.
**Out of Scope:** Runtime linting or compile-time enforcement; changes to adapter implementations.

**Test Plan:**

- **Goldens:** n/a (spec-only)
- **Failures:** n/a
- **Edges:** Clarify what happens when a tick-based TTL expires during a paused view (answer: it does not expire until the view advances).
- **Fuzz/Stress:** n/a

**Blocked By:** T-7-1-1
**Blocking:** T-7-2-1

**Est. Hours:** 3h
**Expected Complexity:** ~120 LoC (markdown)
