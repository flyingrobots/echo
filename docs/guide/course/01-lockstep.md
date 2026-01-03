<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# 01 — Lockstep, Explained (Inputs‑Only Networking)

This module explains the simplest networking model Echo wants to make easy:
**lockstep**.

Lockstep means:

- every peer runs the same simulation locally,
- peers exchange only player inputs (not full game state),
- and each tick has a clear “we agree / we disagree” signal.

## What You’ll Learn

- The difference between state sync vs input sync.
- Why “inputs‑only” is so sensitive to nondeterminism.
- Why Echo treats determinism + hashing as core, not optional.

## The Lockstep Contract (Human Version)

If two peers:

1) start from the same initial state, and
2) apply the same ordered input stream, and
3) advance time using the same tick rules…

…then they should compute the same results.

So we can add a simple check:

- after each tick, compute a fingerprint of state
- compare fingerprints

If the fingerprints match, we are in sync.
If they don’t, we’ve found a determinism bug.

## How This Changes How You Build Games

In many engines, you can get away with:

- “iterate whatever order the container gives me”
- “use wall clock delta seconds”
- “use platform math + randomness”

In lockstep, those become desync factories.

The rule of thumb:

> If it can vary between machines, it cannot decide the simulation.

## Verify Step (Later, in Code)

We will implement a harness that:

- runs two peers with the same input log
- prints PASS if all per‑tick fingerprints match
- prints FAIL with the first mismatching tick if they diverge

## Failure Demo (Later)

We will intentionally break the contract by introducing:

- unseeded randomness
- unstable iteration order
- wall clock time

Then we’ll fix it “the Echo way”.
