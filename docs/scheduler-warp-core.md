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

## Terminology (warp-core / Rust)

This document is about the **implemented** scheduler in Rust `warp-core`. Terms below are Rust-native
and are intended to be directly checkable against code:

- **Pending rewrite:** `PendingRewrite` in `crates/warp-core/src/scheduler.rs` — a scheduled unit of work
  carrying `(scope_hash, compact_rule, scope, footprint, phase)`.
- **Footprint:** `crate::footprint::Footprint` in `crates/warp-core/src/footprint.rs` — the declared resource
  access sets for a rewrite:
  - node reads/writes: `n_read` / `n_write`
  - edge reads/writes: `e_read` / `e_write`
  - attachment reads/writes: `a_read` / `a_write`
  - boundary port claims: `b_in` / `b_out` (treated as a single “claimed ports” set for conflict purposes)
  - coarse prefilter: `factor_mask` (a superset/partition hint used as an O(1) independence fast-path)
- **Resource keys:** concrete key types used by the scheduler’s active sets:
  - nodes: `NodeKey` (warp id + local node id)
  - edges: `EdgeKey` (warp id + local edge id)
  - attachments: `AttachmentKey`
  - boundary ports: `PortKey`

The scheduler’s correctness assumes **footprints are a sound over-approximation** of the rewrite’s effects:
rewrites must not mutate resources they did not declare, or determinism can be violated.

---

## Mental Model

### What is being scheduled?

During a transaction, rules produce **pending rewrites**. Each pending rewrite carries a **footprint**:
the set of nodes/edges/attachments/ports it reads/writes/claims.

### What does the scheduler enforce?

The rewrite scheduler enforces **footprint independence**:
two rewrites may both be accepted only if they do not have conflicting read/write resource access.

This is the core mechanism behind “independent rewrites commute” and therefore tick determinism:
if two rewrites have disjoint footprints (per the conflict rules below), then applying them in either order
must yield the same final state (because neither rewrite reads or writes data the other could influence).

---

## `reserve()` — Semantics & Guarantees

`reserve()` decides whether a pending rewrite can be admitted into the “accepted set” for a given tx.

### Conflicts (informal)

A conflict exists if either rewrite:
- writes a resource the other reads, or
- writes a resource the other writes.

Resources are tracked in (at least) these categories:
- node reads/writes (`Footprint::{n_read,n_write}`)
- edge reads/writes (`Footprint::{e_read,e_write}`)
- attachment reads/writes (`Footprint::{a_read,a_write}`)
- boundary port claims (`Footprint::{b_in,b_out}`; any intersection conflicts regardless of direction)

### Static vs dynamic conflicts

The scheduler enforces **static conflicts** computed from declared footprints at `reserve()` time.
If a rewrite has side effects that are not covered by its footprint (a “dynamic conflict”), that is treated as
an implementation bug: the footprint must be expanded until it fully describes the rewrite’s effects.

### Atomicity (check-then-mark)

The implementation uses a **two-phase protocol**:
1) **check phase**: detect any conflicts without mutating the active set
2) **mark phase**: only if the check phase succeeds, mark all resources

**Guarantee:** if a rewrite is rejected, it must not partially mark state.

### Determinism

Given the same sequence of rewrites and the same footprints, the accept/reject decisions must be the same.

Concretely, this means:

- `reserve()` iterates over footprint sets in a deterministic order (the underlying footprint sets are ordered),
  and performs membership lookups/inserts in `GenSet` active sets. The active sets are hash-backed for O(1)
  lookup/insert, but determinism does **not** rely on hash-table iteration order (we never iterate the hash tables).
- `drain_for_tx()` drains rewrites in a deterministic lexicographic order derived from:
  - `scope_hash` (full 32 bytes),
  - rule id (stable compact id),
  - `nonce` (an insertion-order tie-break).

See the ordering invariant comment at the top of `crates/warp-core/src/scheduler.rs`.

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

This section describes the **default Radix scheduler** (`SchedulerKind::Radix`) which uses generation-stamped
sets (`GenSet`) for independence checks. The legacy `Vec<Footprint>` implementation exists for comparisons and
scales like `O(k×m)` because it checks a candidate footprint against every previously-reserved footprint.

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

The active sets rely on hash tables (`FxHashMap` under the hood).
Average-case behavior is O(1) per lookup/insert, but pathological collisions can degrade.

Status / mitigation notes:
- `FxHashMap` is deterministic (not cryptographically seeded); collision resistance is not its goal.
- We currently treat scheduler keys as internal engine identifiers (not attacker-controlled inputs).
- We maintain an adversarial-collision benchmark to detect regressions and quantify worst-case behavior:
  `crates/warp-benches/benches/scheduler_adversarial.rs`.
- Longer-term hasher hardening discussion lives in `docs/notes/scheduler-optimization-followups.md`.

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

Concrete guidance (so reviews can be consistent):
- **Minor claims (≈10% level):** require a Criterion benchmark showing ≥10% median change with a 95% CI that does not overlap 0.
- **Major claims (≥2× / “order of magnitude”):** require stable benchmark inputs and a Criterion result showing ≥2× median change,
  plus a narrative describing the benchmark scenario, inputs, and why the result should generalize.
- Always include the exact bench (`scheduler_drain` vs `scheduler_adversarial`) and what aspect it measures.

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

Additional maintenance expectations:
- **Versioning:** this doc targets the default implementation (`SchedulerKind::Radix`). If the default changes, update the Scope banner
  and the “Complexity” section to match.
- **Drift detection:** treat changes to `reserve()`/footprint structures as “docs must change” in code review; prefer adding/adjusting tests
  (in `crates/warp-core/src/scheduler.rs` and `crates/warp-core/tests/*`) that encode any new invariants.
- **Ownership:** changes to `crates/warp-core/src/scheduler.rs` should request review from the warp-core maintainers / determinism owners.
- **Deprecation flow:** if this scheduler is replaced, leave this doc as a redirect stub (like other scheduler satellites) and update `docs/scheduler.md`
  so older links remain stable.
