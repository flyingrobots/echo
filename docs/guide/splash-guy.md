<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Demo 2 Scenario: “Splash Guy” (Deterministic Lockstep Proof)

This document defines a small, familiar gameplay scenario we can use to prove:

- Echo can run a **gameplay simulation deterministically** across peers.
- Networking can be **inputs-only lockstep** with per‑tick fingerprint checks.
- Replay/debugging workflows are a first-class part of building the game (not an afterthought).

“Splash Guy” is a grid‑arena action game with water balloons (timed hazards) and chain reactions.
It is mechanically similar to a classic genre, but uses an original name/theme.

---

## Design Constraints (Why This Scenario)

We want a demo that is:

- **Familiar**: people immediately understand movement, walls, timed hazards, chain reactions.
- **Discrete**: easy to reason about tick-by-tick determinism (no floating physics required).
- **Nasty in the right ways**: contains the classic “determinism traps”:
  - simultaneous movement conflicts
  - chain reactions
  - timers
  - optional randomness (pickups)
- **Small**: implementable end‑to‑end with clear invariants + tests.

---

## Core Loop (Player View)

Each tick, each player may:

- move one tile (up/down/left/right), or
- stay, and optionally
- place a water balloon on their current tile (if allowed).

Water balloons have a **fuse** measured in ticks.
When the fuse hits zero, the balloon **bursts**, creating a “splash” that propagates in the 4
cardinal directions with a configurable range.

The splash:

- stops on walls (and optionally on breakable blocks),
- can trigger other balloons (chain reactions),
- can knock out players caught in it.

---

## World Model (What Exists)

### Arena

- A rectangular grid, e.g. 13×11.
- Tile types:
  - `Empty`
  - `Wall` (indestructible)
  - `Block` (optional, destructible)

### Players

Each player has:

- `player_id` (stable, deterministic ordering key)
- `pos = (x, y)`
- `alive: bool`
- `balloon_limit: u8` (how many balloons can exist for this player at once)
- `balloon_range: u8` (splash range)

### Balloons

Each balloon has:

- `owner_player_id`
- `pos = (x, y)`
- `fuse_ticks_remaining: u16`
- `range: u8` (snapshot of owner’s range at placement time)

### Pickups (Optional Early / Likely Later)

We can start with fixed-map pickups (no randomness) and add deterministic spawning later.

Pickup types:

- `MoreBalloons(+1)`
- `MoreRange(+1)`

---

## Deterministic Simulation Contract (What Must Be True)

We want the core “proof” to be simple:

1) All peers start from the same initial state (map + players + seed).
2) All peers consume the same ordered input log (per tick, per player).
3) All peers compute a per‑tick fingerprint (hash).
4) Fingerprints must match on every tick.

If any mismatch occurs, it’s a hard failure:

- the first divergent tick is recorded,
- the demo outputs a clear mismatch report,
- and we use replay + inspection to locate the cause.

---

## Inputs (Network Payload)

Lockstep networking payload should be **inputs only**.

For each tick `t`, each player provides an input:

- `move`: one of `Up|Down|Left|Right|Stay`
- `place_balloon`: `true|false`

Inputs are only accepted for the current tick and are applied in a deterministic order.

---

## Tick Phases (Deterministic Ordering)

To avoid ambiguity, we define a stable tick pipeline.

Recommended (initial) phase order:

1) **Collect inputs** (for tick `t`).
2) **Resolve movement intents**:
   - compute desired target tiles
   - resolve conflicts deterministically (see below)
   - update player positions
3) **Resolve balloon placement intents**:
   - deterministic ordering by `player_id`
   - enforce per-player balloon limits
4) **Advance fuses**:
   - decrement `fuse_ticks_remaining`
5) **Explode balloons whose fuse hit 0**:
   - compute splash tiles deterministically
   - trigger chain reactions deterministically
6) **Apply damage**:
   - players in splash tiles become `alive = false` (or mark “hit”)
7) **Apply pickups / block destruction** (optional, deterministic)

This explicit phase ordering is part of the “show the world it works” story:
Echo is deterministic because it refuses to leave “who wins?” to accident.

---

## Conflict Resolution Rules (Deterministic by Construction)

We need deterministic rules for common conflicts.

### Movement collisions

If multiple players attempt to enter the same tile in the same tick:

- Only one succeeds (chosen deterministically).
- Others remain in place.

Deterministic tie-break recommendation:

- lowest `player_id` wins.

### Swaps (A ↔ B)

If two players attempt to swap tiles in the same tick (A wants B, B wants A):

Pick one policy and keep it stable:

- either allow swaps (both succeed), or
- disallow swaps (both fail), or
- deterministic tie-break (only one succeeds).

For early teaching simplicity, “disallow swaps” tends to be easiest to reason about.

### Balloon placement collisions

If two players attempt to place a balloon on the same tile:

- This can only happen if they share the tile, which our movement rules usually prevent.
- If it does occur (e.g., due to a map spawn bug), resolve by `player_id` order and log it as a test failure.

---

## What We Teach With This Scenario

This scenario is not just a game; it’s a teaching probe.

It gives us a clean place to teach:

- Why **unordered iteration** causes divergence.
- Why **time sources** must be explicit.
- Why **randomness** must be seeded and stable.
- Why “don’t hide structure in blobs” matters (visibility for conflict + replay).
- How hashes + replay turn desyncs into debuggable problems.

---

## Next: Course Outline

The course that builds this demo lives under:

- `docs/guide/course/`
