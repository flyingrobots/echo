<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

> **Milestone:** Tumble Tower | **Priority:** P2
>
> Status: active cool idea. Task DAG issue #235 is still open and blocks the
> Tumble Tower course track (#238). `docs/guide/tumble-tower.md` defines the
> inputs-only lockstep proof, but no Tumble Tower physics simulation or
> two-peer harness exists yet.

# Lockstep Harness

**Issue:** #235

**User Story:** As a learner, I want a two-peer lockstep harness for Tumble Tower that verifies per-tick physics fingerprints so that I can confirm the physics simulation is deterministic across peers.

## Requirements

- R1: Implement a two-peer lockstep harness for Tumble Tower's input model
  (block placement position + rotation), sharing the same protocol shape as the
  Splash Guy lockstep task once that harness exists.
- R2: Exchange `physics_fingerprint` each tick between peers; mismatch triggers desync alert with the divergent tick number.
- R3: Support replaying a recorded input sequence for regression testing.
- R4: Log per-tick state summaries (body count, sleeping count, total energy) for debugging.

## Acceptance Criteria

- [ ] AC1: Two peers run a 200-tick Tumble Tower game with identical physics fingerprints on every tick.
- [ ] AC2: Replay of a recorded 200-tick input sequence produces identical fingerprints to the original run.
- [ ] AC3: Desync alert correctly identifies the divergent tick when one peer uses a different physics stage.
- [ ] AC4: Per-tick state summary log is emitted and parseable.

## Definition of Done

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Lockstep harness for Tumble Tower, fingerprint exchange, replay, logging.
**Out of Scope:** Real network transport; more than 2 peers; rollback/prediction.

## Test Plan

- **Goldens:** Golden fingerprint sequence for a scripted 200-tick two-peer Tumble Tower game.
- **Failures:** Peer sends block placement outside arena bounds (rejected); replay file is truncated (run until end, report truncation).
- **Edges:** Both peers place blocks at the same position on the same tick; zero-input game (just gravity).
- **Fuzz/Stress:** Property test: random block placements for 2 peers over 300 ticks, verify fingerprint match.

**Blocked By:** stage-0-aabb; operationally blocked until there is at least one
runnable Tumble Tower physics stage to fingerprint.
**Blocking:** desync-breakers

**Est. Hours:** 5h
**Expected Complexity:** ~350 LoC
