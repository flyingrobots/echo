<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# SPEC-0003: DPO Concurrency Litmus (v0)

Status: Draft • Date: 2026-01-03 • Owner: warp-core

This spec note is a bridge between the **DPO/DPOI concurrency story** (critical pairs, commuting squares, and order-independence) and what `warp-core` enforces **today**:

- a deterministic scheduler order,
- conservative **footprint-based independence checks**, and
- receipts that commit to “accepted vs rejected” outcomes deterministically.

It is intentionally “low ceremony”: the goal is to pin *executable evidence* (litmus tests) that match the theory-shaped intent without claiming we have implemented full categorical DPO.

## What we are (and are not) proving in code

`warp-core` does **not** implement categorical DPO/DPOI rewriting directly yet; rules are matcher/executor functions plus explicit read/write `Footprint`s. The concurrency claim we can actually test right now is:

> Given a fixed starting state and a fixed set of candidate rewrites for a tick, the engine produces a unique deterministic outcome (same terminal digest) independent of the order in which candidates were enqueued.

In other words: we test *order independence under the engine’s admissibility rules*, not the full “unique up to iso” statement from adhesive-category DPO.

## Ambient assumptions (pragmatic “adhesive enough”)

The intended north-star semantics are typed open-graph DPOI in an adhesive (or quasi-adhesive) setting. The implementation relies on a pragmatic subset of those assumptions:

- **Typed graph discipline**: rules and payloads are domain-separated by `TypeId` and stable ids; “same bytes, different meaning” is prevented at the deterministic boundary.
- **No hidden edges**: attachment bytes are opaque; any causal dependency that matters to matching/scheduling must be expressed as explicit skeleton structure (nodes/edges/ports) or explicit attachment-slot reads (Stage B1 descent-chain reads).
- **Gluing/dangling conditions become mechanical invariants**:
  - “Dangling edge” and “delete-under-descent” hazards are prevented by rule design + validation (where present) and, critically, by conservative footprints that force serialization when overlap is possible.
- **Deterministic resolution policy for overlaps**: overlapping candidates do not race; they are either:
  - jointly admitted when independent, or
  - deterministically rejected/serialized when not independent.

## Mapping: DPO concurrency vocabulary → `warp-core` mechanisms

This table is a translation layer for the parts we actually enforce mechanically.

| DPO/DPOI concept | `warp-core` mechanism (today) |
| --- | --- |
| “Independent” parallel rewrites commute | `Footprint` sets are disjoint under the scheduler’s conflict rules; both candidates reserve and commit; result is order-independent. |
| Critical pair (overlapping matches) | Overlap shows up as footprint intersection (node/edge/attachment/port); later candidate is rejected with `FootprintConflict`. |
| Concurrency theorem gives uniqueness of result | Engine drains candidates in a canonical order and applies an admissible subset; receipts + digests pin the outcome deterministically. |
| Gluing condition / dangling prevention | Enforced conservatively via explicit footprint reads/writes, “no hidden edges”, and (where implemented) validation checks. |

## Litmus suite (executable taxonomy)

The litmus suite exists to keep the “DPO concurrency” claim honest by pinning three case families:

1. **Commuting / independent pair**
   - Two candidates are independent (disjoint footprints).
   - Enqueue order varies, but the terminal digest is identical.
2. **Conflicting / critical-pair-style overlap**
   - Two candidates overlap (footprint intersection).
   - Outcome is deterministic: exactly one is admitted (canonical winner), and the other is rejected with `FootprintConflict` in the receipt.
3. **Overlapping scope but still composable**
   - Candidates share a high-level scope notion (e.g., both target the same node),
     but their *resource footprints* remain disjoint (e.g., disjoint boundary ports, read-only overlaps).
   - Both are admitted and the terminal digest is order-independent.

## What “terminal digest” means here

For litmus tests that compare different enqueue orders within the **same tick**, “terminal digest” is the full `Snapshot.hash` produced by `Engine::commit()`. This commits to:

- `state_root` (reachable-only canonical hashing),
- `patch_digest` (delta ops + slot sets),
- parent list (empty for the first commit in the litmus setup), and
- policy id (part of the deterministic boundary).

## References

- `docs/spec-mwmr-concurrency.md` (footprints, ports, and independence model)
- `docs/warp-math-claims.md` (theory framing: DPO/DPOI + determinism claims)
- Litmus tests: `crates/warp-core/tests/dpo_concurrency_litmus.rs`
