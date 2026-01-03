<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Demo 3 Scenario: “Tumble Tower” (Deterministic Physics)

This document defines **Demo 3**: a staged, increasingly “real” physics proof that Echo can run a
rigid-body style simulation deterministically across peers.

If Demo 2 (“Splash Guy”) proves *lockstep gameplay determinism* without continuous physics, Demo 3
proves the harder claim: **deterministic physics** that stays stable over time and across machines.

---

## Proof Claim (What We’ll Show the World)

For a fixed tick rate:

1) All peers start from the same initial state (world + parameters + seed).
2) All peers consume the same ordered input log (per tick).
3) All peers compute a per‑tick fingerprint (hash).
4) Fingerprints match on every tick.

If any mismatch occurs, it’s a hard failure:

- record the first mismatching tick,
- produce a minimal mismatch report,
- replay + bisect to find the cause.

---

## Why “Tumble Tower”?

We want a demo that is:

- **immediately watchable** (stacking blocks looks like “physics” to everyone),
- **determinism-stressing** (contacts, stacking, ordering sensitivity),
- **teachable** (we can intentionally break it and show how to diagnose/fix),
- **stageable** (we can start simple and deliberately sign up for the hard parts later).

The core fantasy:

> A tower of blocks under gravity, where players can spawn blocks, poke blocks, and remove supports.

---

## Stages (Physics Ladder)

Demo 3 is explicitly staged so each step has a crisp “done” definition and a crisp verify step.

### Stage 0 — 2D AABB blocks (no rotation)

- Bodies are **axis-aligned rectangles** (AABB).
- Fixed time step tick.
- Gravity + velocity integration.
- Deterministic collision/contact resolution ordering.

Why first:

- still looks like physics,
- keeps the “hard problem” focused on determinism/ordering,
- avoids early rabbit holes (rotation + manifold generation).

### Stage 1 — Rotation + angular dynamics (OBB contacts)

- Oriented boxes (OBB).
- Angular velocity, torque, rotational inertia.
- Deterministic contact generation / manifold ordering.

### Stage 2 — Friction + restitution

- Bounce (restitution).
- Static + kinetic friction.
- Deterministic solver behavior (ordering + math).

### Stage 3 — Sleeping + stack stability

- Sleep/wake transitions are deterministic.
- Stable behavior over long runs (thousands of ticks).
- Deterministic island building and solver ordering.

### Stage 4+ (Optional) — 3D extension

- Only after 2D proves itself.
- Same proof claim, larger surface area.

---

## World Model (What Exists)

### Environment

- A static floor (and optionally static walls).
- A bounded camera/view (non-authoritative; derived from state).

### Bodies (Blocks)

Each block has:

- a unique ID (stable, deterministic ordering key)
- position (2D)
- velocity (2D)
- size (width/height)
- material parameters (stage-dependent: restitution, friction)
- sleep state (stage-dependent)

Stage-dependent additions:

- rotation angle, angular velocity, inertia (Stage 1+)

---

## Inputs (Network Payload)

We keep the networking model **inputs-only** (lockstep), and treat physics as part of the
deterministic simulation.

Example input actions (per tick):

- `SpawnBlock { x, y, w, h }` (or spawn from a fixed catalog + deterministic spawn IDs)
- `Poke { block_id, impulse }` (or nudge direction + magnitude)
- `RemoveBlock { block_id }`

We may start even simpler:

- scripted spawns at fixed ticks
- a single “poke the tower” input

The key is that inputs are canonical, ordered, and replayable.

---

## Determinism Hazards (What This Demo Is For)

Tumble Tower is designed to surface the classic determinism traps in physics:

- **unordered iteration** over contacts/bodies changes outcomes
- **different contact ordering** changes solver results
- **floating point edge cases** accumulate and drift
- **sleeping/island grouping** can be nondeterministic if built from unordered sets
- “almost resting” situations are sensitive to tiny numeric differences

This demo only counts as “done” if it stays stable under those stresses.

---

## Verify Steps (Every Stage)

Every stage must ship with:

- a deterministic replay test (same seed + same inputs => same fingerprints), and
- at least one long-run stability test (the “does it drift after 10k ticks?” question).

We also want teaching-oriented “breakers”:

- toggles that intentionally introduce nondeterminism (unstable ordering, wall-clock dt, etc.)
- a harness that reports the first mismatch tick

---

## Next: Course + Implementation

This doc defines the scenario and the staged ladder.
Follow-up work will:

- implement the stage 0 simulation,
- add the two-peer lockstep harness,
- build visualization and debug overlays,
- write the staged “physics ladder” course docs.
