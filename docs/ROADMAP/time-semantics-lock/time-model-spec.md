<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

> **Milestone:** [Time Semantics Lock](README.md) | **Priority:** P1

# TT0 — Time Model Spec Lock

Lock the vocabulary and semantics for HistoryTime vs HostTime, tick-based TTL/deadlines, and the StreamAdmissionDecision digest chain. This is a spec-only feature — no runtime code, only documents and their review artifacts.

**Issues:** #191, #192

---

## T-7-1-1: Spec — HistoryTime vs HostTime field classification (#191)

**User Story:** As a contributor implementing time-aware adapters, I want a single authoritative document that classifies every session-stream time field as HistoryTime (deterministic, ordering/replay) or HostTime (telemetry only) so that I never accidentally introduce nondeterminism through a time field.

**Requirements:**

- R1: Produce a table in `docs/spec-time-streams-and-wormholes.md` listing every known time field across `StreamAdmissionDecision`, `ClockDecision`, `EventEnvelope`, `InspectorEnvelope`, and session-proto messages.
- R2: Each field is classified as HistoryTime or HostTime with a one-line rationale.
- R3: Add a "decision record" rule: any adapter that consults HostTime must emit a canonical decision record before the simulation consumes the result.
- R4: Cross-reference `docs/spec-merkle-commit.md` for fields that feed into `admission_digest`.

**Acceptance Criteria:**

- [ ] AC1: Classification table exists in `docs/spec-time-streams-and-wormholes.md` with at least 10 fields classified.
- [ ] AC2: No field is left unclassified ("TBD" entries are explicit open questions with tracking issues).
- [ ] AC3: The decision-record rule is stated as a normative requirement, not advisory text.
- [ ] AC4: At least one reviewer has confirmed the classifications against the existing `echo-session-proto` message definitions.

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

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
