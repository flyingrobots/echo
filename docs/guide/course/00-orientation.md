<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# 00 — Orientation: Shared Truth, Not Vibes

This module sets the frame for the whole course.

## What You’ll Learn

- What Echo is trying to be “best in class” at (determinism, replay, sync, verification).
- What we’re proving with Demo 2 (“Splash Guy”).
- The difference between:
  - “the simulation is correct” and
  - “two machines agree on what happened”.

## The Promise (In Plain Language)

Echo tries to make this statement true:

> If two computers start from the same state and consume the same inputs,
> they produce the same results.

Most engines can’t promise this end-to-end without a lot of game-specific work and caveats.
Echo treats it as the foundation.

## What We’re Building

We’re building a small grid arena game:

- players move on a grid
- players place water balloons with fuses
- balloons burst, splashing in lines and triggering chain reactions

The specific scenario spec is here:

- [/guide/splash-guy](/guide/splash-guy)

## A Useful Mental Model (Before Any Jargon)

- **Inputs**: what players choose (move, place balloon).
- **State**: what is currently true (positions, balloons, timers).
- **Tick**: one step of time.
- **Fingerprint**: a compact check that the whole state matches.

In this course, we treat “fingerprint mismatch” as a first-class event:
it means “we lost shared truth”.

## Verify Step (The Whole Course Has This Shape)

Every module includes a verify step like:

- run a short input script for N ticks
- compute per‑tick fingerprints
- confirm they match across runs (and eventually across peers)

Later modules will show how to intentionally break determinism and then fix it.
