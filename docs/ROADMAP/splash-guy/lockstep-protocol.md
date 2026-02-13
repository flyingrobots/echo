<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

> **Milestone:** [Splash Guy](README.md) | **Priority:** P2
>
> This feature is a skeleton. Tasks will be expanded as the GDD matures.

# Lockstep Protocol

**Issue:** #223

**User Story:** As a learner studying deterministic networking, I want a lockstep protocol that exchanges player inputs between two peers and verifies per-tick fingerprints so that I can see how networked determinism works in practice.

## Requirements

- R1: Implement a lockstep input protocol: each peer sends its input for tick T, waits for the other peer's input, then both advance.
- R2: Each peer computes `state_fingerprint` after advancing and exchanges it; mismatch triggers a desync alert.
- R3: Two-peer harness: a test binary that runs two Echo instances in the same process (or via loopback), exchanging inputs via channels.
- R4: Support configurable simulated latency (delay input delivery by N ticks) to test buffering.
- R5: Log input exchange and fingerprint comparison for debugging.

## Acceptance Criteria

- [ ] AC1: Two-peer harness completes a 100-tick game with identical final state on both peers.
- [ ] AC2: Per-tick fingerprints match on both peers for all 100 ticks.
- [ ] AC3: Harness with 3-tick simulated latency still completes correctly (inputs are buffered and applied at the correct tick).
- [ ] AC4: Desync alert fires when one peer's rules are intentionally mutated (setup for controlled-desync).

## Definition of Done

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Lockstep protocol, two-peer harness, fingerprint exchange, simulated latency.
**Out of Scope:** Real network transport (uses in-process channels); rollback/prediction; more than 2 peers.

## Test Plan

- **Goldens:** Golden per-tick fingerprint sequence for a scripted 100-tick two-peer game.
- **Failures:** One peer sends no input (timeout after N ticks, harness reports stall); peer sends input for wrong tick (rejected with error).
- **Edges:** Both peers send identical inputs (valid, should produce identical state); zero-latency (no buffering needed).
- **Fuzz/Stress:** Property test: random inputs from both peers, random latency 0-10 ticks, verify fingerprint match on every tick.

**Blocked By:** rules-and-state
**Blocking:** controlled-desync

**Est. Hours:** 6h
**Expected Complexity:** ~450 LoC
