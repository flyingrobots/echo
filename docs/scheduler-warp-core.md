<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# WARP Rewrite Scheduler (warp-core)

This document covers the **implemented** rewrite scheduler in Rust `warp-core`.

It is **not** the planned ECS/system scheduler described in `docs/spec-scheduler.md`.
For a “which scheduler doc should I read?” landing page, see `docs/scheduler.md`.

---

## Scope

This doc exists to keep a single, up-to-date source of truth for:
- what `reserve()` means and guarantees,
- what determinism properties we require for rewrite scheduling, and
- how we validate and benchmark those claims.

**Code:** `crates/warp-core/src/scheduler.rs`

---

## Mental Model

### What is being scheduled?

During a transaction, rules produce **pending rewrites**. Each pending rewrite carries a **footprint**:
the set of nodes/edges/ports it reads/writes.

### What does the scheduler enforce?

The rewrite scheduler enforces **footprint independence**:
two rewrites may both be accepted only if they do not have conflicting read/write resource access.

This is the core mechanism behind “independent rewrites commute” and therefore tick determinism.

---

## `reserve()` — Semantics & Guarantees

`reserve()` decides whether a pending rewrite can be admitted into the “accepted set” for a given tx.

### Conflicts (informal)

A conflict exists if either rewrite:
- writes a resource the other reads, or
- writes a resource the other writes.

Resources are tracked in (at least) these categories:
- node reads/writes
- edge reads/writes
- port/boundary claims (in/out)

### Atomicity (check-then-mark)

The implementation uses a **two-phase protocol**:
1) **check phase**: detect any conflicts without mutating the active set
2) **mark phase**: only if the check phase succeeds, mark all resources

**Guarantee:** if a rewrite is rejected, it must not partially mark state.

### Determinism

Given the same sequence of rewrites and the same footprints, the accept/reject decisions must be the same.

This implies:
- deterministic iteration order over tracked resources,
- deterministic tie-break behavior when draining.

---

## Evidence (Tests)

Most of the “reserve is correct” evidence is in `crates/warp-core/src/scheduler.rs` unit tests.
Some integration-level behavior is also exercised via `crates/warp-core/tests/*`.

Key test themes:
- **Atomic reservation:** failing reserves do not partially mark resources.
- **Determinism:** the same reserve sequence yields the same accept/reject decisions repeatedly.
- **Conflict detection:** node/edge/port conflicts are detected correctly.
- **Independence:** independent rewrites are admitted concurrently.

Concrete tests to look for in `crates/warp-core/src/scheduler.rs`:
- `reserve_is_atomic_no_partial_marking_on_conflict`
- `reserve_determinism_same_sequence_same_results`
- `reserve_scaling_is_linear_in_footprint_size` *(timing is noisy; treat as a sanity check, not a benchmark)*

For a concrete “how to run” section, see below.

---

## Complexity (Why `reserve()` is O(m), not O(k×m))

Let:
- `m` = total footprint size of the candidate rewrite (sum of read/write sets across resource kinds)
- `k` = number of rewrites already admitted to the active set

The current approach is GenSet-based: it checks and marks membership in active sets.

### Expected complexity

- **Best case:** O(1) (early exit on the first conflict)
- **Worst case (no conflict):** O(m) checks + O(m) marks ⇒ O(m)
- **Independent of k:** membership checks do not iterate over previous rewrites

### “Count the loops” (check + mark)

In the worst case (no early conflict), `reserve()` does:
- a pass over each resource category to **check** conflicts, then
- a pass over each resource category to **mark** resources

This is the origin of the “O(m) checks + O(m) marks ⇒ O(m)” claim.

### Hashing caveat

The active sets rely on hash tables.
Average-case behavior is O(1) per lookup/insert, but pathological collisions can degrade.

See also the adversarial hashing bench:
- `crates/warp-benches/benches/scheduler_adversarial.rs`

---

## Benchmarks

Benchmarks are intentionally separated from unit-test timing:

- Drain throughput / apply+commit: `crates/warp-benches/benches/scheduler_drain.rs`
- Hash-table collision behavior: `crates/warp-benches/benches/scheduler_adversarial.rs`

Run benches:

```sh
cargo bench -p warp-benches
```

---

## Notes on Performance Claims

Some older docs used strong language like “empirical proof” or quoted “10–100× faster” speedups.
The safest, reviewable stance is:
- algorithmically, the GenSet approach avoids the old “compare against every previous footprint” pattern (so it should not scale like `k×m`), and
- meaningful performance claims should come from Criterion benches, not single-run timings inside unit tests.

---

## How To Run Locally

Most of the scheduler evidence runs as part of normal `warp-core` tests:

```sh
cargo test -p warp-core
```

If you need to target an individual integration test:

```sh
cargo test -p warp-core --test reserve_gate_tests
```

---

## Docs Maintenance Notes

When changing `crates/warp-core/src/scheduler.rs` behavior (especially around `reserve()`):
- update this doc,
- keep `docs/scheduler.md`’s mapping accurate,
- and prefer encoding invariants in tests over prose-only claims.
