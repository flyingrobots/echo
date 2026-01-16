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

## Authoring Conventions (for course writers)

- Prefer short pages with a clear outcome and a verify step.
- Introduce jargon only after the reader has used the idea.
- Every module should include at least one of:
  - a “verify” step (fingerprint/replay check)
  - a “failure demo” (controlled desync)
- Rendering must be described as derived output (never authoritative).

---

## Two Tracks (Same Demo)

You can follow either track, or both:

### Track A: Concept (no code required)

Goal: understand what Echo is, why it’s different, and how the networking-first mental model works.

### Track B: Builder (hands-on)

Goal: implement the demo and make it pass deterministic replay + lockstep sync checks.

Both tracks share the same steps and vocabulary; Track B adds “do the thing” instructions.

---

## Course Modules (Available Now)

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

---

## Planned Modules (Not Yet Written)

Modules 02–09 are planned but not yet implemented in this repo. When they land, they will appear under `docs/guide/course/` and be listed here.
