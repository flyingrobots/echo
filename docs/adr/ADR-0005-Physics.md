<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# ADR-0005: Physics as Deterministic Scheduled Rewrites (Footprints + Phases)

- **Status:** Accepted
- **Date:** 2026-01-14

## Context

Echo runs deterministic ticks over a WARP graph. Some subsystems (physics, constraints, layout) require multi-pass updates and must remain:

- deterministic across platforms
- schedulable using independence/footprints (confluence-friendly)
- isolated from the public causality API (Inbox/Ingress remains sacred)

Physics introduces:
- broadphase candidate generation
- narrowphase contact computation (possibly swept / TOI)
- iterative constraint resolution
- multi-body coupling (piles) that emerges from pairwise contacts

We must avoid designs that serialize derived work into an “inbox-like” ordered stream, which destroys independence batching and confluence benefits.

## Decision

### 1) Physics runs as an internal tick phase, not as causal ingress

Physics is a system phase executed during `step()`. It does not emit causal events into the ledger.
Causal inputs remain: ingested intents (and optionally a causal `dt` parameter).

### 2) Contacts are modeled as rewrite candidates with explicit footprints

Physics resolution is expressed as rewrite candidates operating over multiple bodies.

- Each candidate represents a contact constraint between bodies A and B.
- Footprint writeset includes `{A, B}` (and any shared state they mutate).
- Candidates commute iff their footprints are disjoint.

This maps directly onto Echo’s scheduler: deterministic conflict resolution and maximal independence batching.

### 3) Candidate selection is set-based, not queue-based

Broadphase produces a *set* of candidates. The scheduler:
- canonicalizes ordering
- selects a maximal independent subset (disjoint footprints)
- applies them in deterministic order
- repeats for a fixed number of solver iterations

We do **not** re-inject derived candidates into a “micro-inbox” stream, because ordered queues erase concurrency benefits.

### 4) Deterministic ordering rules are mandatory

All physics candidate generation and application must be canonical:

- Bodies enumerated in sorted `NodeId` order
- Candidate pair key: `(min(A,B), max(A,B))`
- If swept/TOI is used, include `toi_q` (quantized) in the sort key
- If a manifold produces multiple contacts, include a stable `feature_id` in the key

Recommended canonical key:
`(toi_q, min_id, max_id, feature_id)`

### 5) Two-pass / multi-pass is implemented as phases + bounded iterations

Physics tick phase executes as:

1. **Integrate (predict)** into working state (or `next`) deterministically.
2. **Generate contact candidates** deterministically (broadphase + narrowphase).
3. **Solve** using K fixed iterations:
   - each iteration selects a maximal independent subset via footprints
   - apply in canonical order
4. **Finalize (commit)**: swap/commit working state for the tick.

Iteration budgets are fixed:
- `K_SOLVER_ITERS` (e.g., 4–10)
- Optional `MAX_CCD_STEPS` if swept collisions are enabled

Optional early exit is allowed only if the quiescence check is deterministic (e.g., no writes occurred or hash unchanged).

### 6) Swept collisions (CCD) are supported via earliest-TOI banding (optional)

If CCD is enabled:

- Compute TOI per candidate pair in `[0, dt]`
- Quantize TOI to an integer bucket `toi_q`
- Choose the minimum bucket `toi_q*`
- Collect all candidates with `toi_q <= toi_q* + ε_bucket` into the solve set
- Solve as above (K iters)
- Advance remaining time and repeat up to `MAX_CCD_STEPS`

N-body “simultaneous collisions” are treated as connected components in the footprint graph (collision islands), not as N-ary collision events.

### 7) MaterializationBus emits only post-phase, post-commit outputs

The UI-facing MaterializationBus (channels) emits after physics stabilization for the tick.
No half-updated physics state is observable via materializations.

### 8) Inspector visibility is via trace channels, not causal ledger

Optional debug-only materializations may be emitted:
- `trace/physics/candidates`
- `trace/physics/selected`
- `trace/physics/islands`
- `trace/physics/iters`

These are outputs, not inputs.

## Consequences

### Positive

- Physics reuses existing determinism machinery (scheduler + footprints)
- Preserves maximal independence batching (confluence-friendly)
- Multi-pass behavior is explicit, bounded, and reproducible
- Works in both wasm and native runtimes with the same semantics

### Negative / Tradeoffs

- Requires careful canonical ordering in broadphase/narrowphase
- Requires fixed iteration budgets for convergence (predictable but not “perfect”)
- CCD adds complexity; may be deferred until needed

## Notes

Physics is not a public API. It is an internal deterministic phase driven by causal inputs.
Candidate sets + footprint scheduling preserve concurrency benefits; queueing derived work does not.
