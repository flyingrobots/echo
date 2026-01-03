<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Echo Course: Networking‑First (Build “Splash Guy”)

This course teaches Echo by building a small game‑simulation demo that proves:

- two peers can run the same simulation from the same inputs,
- produce matching per‑tick fingerprints,
- and diagnose/fix desyncs when they occur.

Scenario: **Splash Guy** — a grid arena game with water balloons (timed hazards) and chain reactions.

Start with the scenario spec:

- [/guide/splash-guy](/guide/splash-guy)

Glossary (recommended while reading):

- [/guide/course/glossary](/guide/course/glossary)

---

## Two Tracks (Same Demo)

You can follow either track, or both:

### Track A: Concept (no code required)

Goal: understand what Echo is, why it’s different, and how the networking-first mental model works.

### Track B: Builder (hands-on)

Goal: implement the demo and make it pass deterministic replay + lockstep sync checks.

Both tracks share the same steps and vocabulary; Track B adds “do the thing” instructions.

---

## Course Modules (Outline)

Each module has:

- a learning goal,
- a “verify” step (fingerprint/replay check),
- and a controlled “make it fail” desync demo (so you learn to diagnose).

### 00 — Orientation: shared truth, not vibes

- Goal: understand Echo’s core promise and what we’re trying to prove.
- Verify: run a short replay and see stable per‑tick fingerprints.
- Page: [/guide/course/00-orientation](/guide/course/00-orientation)

### 01 — Lockstep in plain language

- Goal: understand inputs‑only networking, and why it changes how you author gameplay.
- Verify: same input log => same fingerprints.
- Page: [/guide/course/01-lockstep](/guide/course/01-lockstep)

### 02 — Model the world (grid + players + balloons)

- Goal: learn how to model gameplay state so it remains deterministic and debuggable.
- Verify: state hash matches after N ticks with a fixed input script.

### 03 — Rules: movement + placement, deterministically

- Goal: implement basic rules with explicit conflict resolution.
- Verify: collisions resolve identically across runs.
- Failure demo: unstable iteration order changes who “wins”.

### 04 — Time as ticks (fuses, chain reactions)

- Goal: model timers deterministically and safely.
- Verify: chain reactions occur on the same tick across peers.
- Failure demo: wall‑clock time causes drift/desync.

### 05 — Randomness (optional), done safely

- Goal: add pickups without breaking determinism.
- Verify: seeded PRNG makes drops identical.
- Failure demo: unseeded randomness causes divergence.

### 06 — Replay as your debugger

- Goal: treat replay as the default debugging tool.
- Verify: bisect to the first mismatching tick.

### 07 — Minimal transport (inputs + fingerprints)

- Goal: wire a simple two‑peer harness that proves sync.
- Verify: one command produces PASS/FAIL.

### 08 — Watchability (rendering as a derived view)

- Goal: make the demo visible without letting rendering influence simulation.
- Verify: “render-only” components do not affect fingerprints.

### 09 — Packaging the proof (“show the world it works”)

- Goal: produce a shareable demo script + short video path + clear README.
- Verify: fresh clone → run → PASS.

---

## Next Step (Implementation Work)

This outline is the “map”.
As we implement the demo, each module will become a real page under `docs/guide/course/`.
