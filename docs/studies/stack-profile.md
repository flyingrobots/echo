<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Study: stacking the optimizations, and where a tick's time goes

**Status:** probe branches, not for merge as they stand.
`claude/combo-m2-lazy` merges M2 (which includes M1, #918 and #919) with
`claude/lazy-blockers`. `claude/stack-envelope` builds on it.
**Author:** Claude (Anthropic), for James Ross.
**Question (James):** with every optimization on, how fast is Echo, and what
should be stacked next?

## Result

| F5 tier M, application profile | Append | Replay |
| --- | ---: | ---: |
| M2 alone | 285 ms | 101 ms |
| M2 + lazy blockers | 274 ms | 101 ms |
| + one worker runs inline (`43a8ddda`) | **173 ms** | 104 ms |
| Legacy profile, for reference | 5,733 ms | 5,808 ms |

Every round of every run was valid under the fixture's own checks.

1. **Merging changes nothing on the normal path.** F5 rarely conflicts, so
   lazy blockers have nothing to remove there. On contended engine ticks the
   combo gives the same gain as lazy blockers on `main`: 272.8 ms to 8.5 ms
   at 10,000 rewrites and 50% contention.
2. **Profiling found a cost no plan named.** Each tick spawned and joined an
   OS thread to run its one rewrite. Running a single worker on the calling
   thread cut an F5 append by 37%.
3. **The planned envelope work is not a speed change.** The commit guard's
   reassembly, the touch log and the ingress writes were about 0.1% of an F5
   tick; reservation about 0.7%. They may still be worth doing for design
   reasons.

## The combo

`claude/combo-m2-lazy` is the M2 branch, a commit adding the conflict
benchmark without the on-demand query (`2e79511c`, so M2 can be measured
alone), and a merge of `claude/lazy-blockers` (`e9f29548`). The only
conflict was `CHANGELOG.md`, resolved by keeping both entries.

Every `perf/*` branch had already merged into `main` by 2026-10-17, so the
combo carries them. One local branch, `perf/history-independent-rollback`,
is based on the 2026-10-12 `main`, has no remote, and looks superseded by
the touch-log rollback; it is not included.

Full `warp-core` suite on the combo, with `native_rule_bootstrap`,
`trusted_runtime`, `host_test`, `delta_validate`,
`application_root_profile` and `det_fixed`: 2,037 passed, 3 failed. The
three are expected: the two host-contract tests that assert
`Rejected.blocked_by` (see [lazy-blockers](lazy-blockers.md)), and the
temp-path socket test that fails identically on `main`.

`footprint_enforce_release` was measured on M2 alone (about 7 ms more per
10,000-rewrite tick) and then dropped, because Edict checks footprints at
compile time.

## Where the time goes

Sampled with macOS `sample` (1 ms interval) on `scene_fixtures_profile`, a
driver that repeats F5 tier M under the application profile, and on
`scheduler_conflict_blame`. The analysis scripts are in
[`stack-profile/`](stack-profile/).

**F5, inside `super_tick_inner` (4,314 samples, one append per tick):**

| Cost | Share |
| --- | ---: |
| Advancing the application trie | 39.1% |
| `pthread_join` waiting on the worker | 23.0% |
| `pthread_create` | 8.5% |
| The worker's own work, whole run | ~2% |
| `record_receipt_correlations` | 3.8% |
| Reservation and drain | 0.7% |
| Guard reassembly, touch log, ingress writes | ~0.1% |

**blake3 by caller.** In F5 it is 36.5% of the tick. Branch hashes in the
trie (`join`) are 21.5%, and leaves and record keys 5.1%. In a
10,000-rewrite tick it is 30% of the commit: plan and rewrites digests
12.8%, slot encoding for the patch digest 8.5%, receipt digest 8.4%.

`blake3::portable::compress_in_place` dominates, but NEON is already built
on aarch64. blake3 uses it only through `hash_many`, for inputs of several
chunks. These hashes are under a block, or long digests fed a few bytes at a
time, so they never reach it.

## Worker count

`scheduler_workers` times one tick of `n` independent rewrites, each
chaining a chosen number of blake3 hashes and writing the result as an
attachment, across worker counts. Apple M5 Pro, 18 cores.

| Rewrites, work each | 1 worker | 18 workers | 256 workers |
| --- | ---: | ---: | ---: |
| 10,000, 1,000 hashes | 543.7 ms | 66.7 ms | 67.5 ms |
| 10,000, 100 hashes | 81.0 ms | 32.3 ms | 34.6 ms |
| 10,000, no work | 29.1 ms | 29.0 ms | 31.2 ms |
| 100, 1,000 hashes | 5.35 ms | 0.68 ms | 1.26 ms |
| 100, no work | 0.26 ms | 0.34 ms | 0.89 ms |

- Threads pay only with real executor work: 8× at 18 workers.
- The commit's serial part (about 29 ms at 10,000 rewrites: reservation,
  patch, digests, state root) caps the gain at any worker count.
- Workers are capped at the number of shard units, so 256 workers on 100
  rewrites is about 100 threads, and thread start-up triples a small tick.

**Inline single worker (`43a8ddda`).** `execute_work_queue` now runs one
worker on the calling thread. Several workers still each get a thread.

| One tick, one worker | Thread | Inline |
| --- | ---: | ---: |
| 1 rewrite, no work | 0.036 ms | 0.008 ms |
| 1 rewrite, 100 hashes | 0.040 ms | 0.015 ms |
| 100 or 10,000 rewrites | unchanged | unchanged |

On F5 tier M an append went from 274 ms to 173 ms, and tier S from 9.1 ms to
5.1 ms. Full suite on `43a8ddda`: the same three failures, nothing new.

## Commit-path fixes (`f118d7ef`)

A profile of a 10,000-write engine tick (`scheduler_workers`, no work per
rewrite, one worker) put most of the commit in three places, none of them
rule work. `f118d7ef` changes all three, and no digest changes:

- the patch digest gathers its bytes and hashes them in one call, instead of
  about eight `blake3` updates per op;
- reservation collects in and out slots in vectors, which the patch sorts
  and deduplicates once, instead of `BTreeSet`s sorted again by the patch;
- the patch canonicalizes ops with a stable sort and a last-wins pass,
  instead of a `BTreeMap`.

`scheduler_workers`, p50 commit, before (`a0bd72da`) and after (`f118d7ef`),
25 ticks per row:

| Rewrites, hashes each | Workers | Before | After |
| --- | ---: | ---: | ---: |
| 10,000, none | 1 | 28.782 ms | 22.425 ms |
| 10,000, none | 18 | 29.499 ms | 23.022 ms |
| 10,000, 100 | 1 | 80.378 ms | 75.921 ms |
| 10,000, 100 | 18 | 32.760 ms | 26.445 ms |
| 100, none | 1 | 0.192 ms | 0.199 ms |
| 100, 100 | 1 | 0.884 ms | 0.808 ms |

F5 tier M under the application profile did not move: 166.07 ms before and
166.14 ms after. Its ticks carry one op. Run-to-run variation is visible
here: `a0bd72da` has the same engine code as `43a8ddda`, which measured
172.72 ms.

The full suite at `f118d7ef` failed only the three expected tests. The fmt
and clippy runs of that campaign are void (see the ledger). A clean rerun
on the committed tree is still owed.

## hello-echo, the Edict toy

hello-echo pins Edict `df80f92` and Echo `490134c0`, 641 commits behind
Echo `main` `4e801ef2`, and refuses other producers by design. Run with its
lock edited to each target in a separate worktree (Rust 1.96 throughout;
Echo's pin asks for 1.90):

| Edict | Echo | Result |
| --- | --- | --- |
| `df80f92` (pin) | `490134c0` (pin) | passes: 1 golden, 1 deterministic rerun, 2 refusals, 1 boundary, 3 fixed-seed property, 8 stress |
| `df80f92` | `main` `4e801ef2` | builds; runtime refuses before any case |
| `df80f92` | stack `f118d7ef` | builds; runtime refuses before any case |
| `main` `0c8cf37` | `main` and stack | build refuses: vendored `causal-cell/manifest.cbor` differs from Edict's fixture |

The runtime refusal is hello-echo's pinned verification-report hash. The
built package is identical at all three Echo revisions (`3665d692…`), but
the verification report is `2541c826…` at the pin and `e3e3f37c…` at both
`main` and the stack. So Echo changed the report somewhere between the pin
and `main`, and the stack produces exactly what `main` produces. hello-echo's
runtime cases have therefore not run against `main` or the stack. Running
them needs hello-echo's pins advanced and its artifacts re-vendored, which is
its owner's process.

## The game fixture

`claude/game-fixture` (`e5645148`) holds fixture F6, real-time game scenes.
It is committed so it is not lost. It has not yet been built, tested or
measured, and nothing in this study comes from it.

## Process errors, and what they cost

- **Editing a worktree a campaign was using.** The game fixture was written
  into the stack worktree while a campaign ran there. The suite had already
  compiled without it, but the fmt and clippy runs saw the uncommitted files
  and are void. The fixture moved to its own worktree.
- **A hidden refusal.** A runner hid the lock wrapper's JSON output, and the
  wrapper refused because its temporary directory had been removed. Two
  hello-echo runs then ran at the pin while labelled `main` and stack. They
  were caught from their headers, renamed and voided, and the runner now
  checks the checkout before running.

All figures are in the [benchmark ledger](stack-profile/ledger.md), which
`stack-profile/ledger.py` builds from the campaign outputs and logs.

## Options considered, not yet built

| Option | Expected effect | Identity change |
| --- | --- | --- |
| Hash long digests in one call | NEON and `update_rayon` for big ticks; little on F5 | None: same bytes |
| One-block trie hashes (domain and split in a precomputed key) | About half of the trie's 21.5% | Changes the unfrozen M2 v0 recipe |
| Homomorphic set hash (LtHash) updated from the diff | Cost depends only on the diff, not graph size; order-free, so workers and ticks can sum separately | New root recipe; loses openings |
| Lazy root: commits bind `parent + patch_digest`, root sealed every K ticks | Most of the trie's 39% | Protocol change; tied to the M2 full seal |
| Envelope dispatch (no guard, no undo) | ~0.1% of an F5 tick | None |
| Admission-time reservation | ~0.7% of an F5 tick | None |

**Proof systems.** ADR-0020 already makes commitment and opening proofs an
optional layer above storage, preferring Verkle-style commitments with
IPA openings when bounded openings are needed. A per-tick root needs speed
and state equality. Openings need a tree. Verkle updates cost
elliptic-curve work, about 100× a blake3 hash. ZK circuits favour
field-friendly hashes over blake3. So openings, Verkle and ZK belong on
sealed checkpoints, not on the per-tick root. That makes the M2 full-seal
decision the one that shapes the proof side.

## Decisions for the owner

1. Whether to land the inline single-worker change. It changes no result
   and is independent of M1, M2 and lazy blockers.
2. Whether the per-tick root should become a set hash over the diff, with
   trees and proofs moving to sealed checkpoints.
3. Whether the envelope and admission-time reservation are still wanted, for
   design rather than speed.

## Reproduce

```sh
cargo bench -p warp-core --features native_rule_bootstrap,trusted_runtime,application_root_profile \
  --bench scene_fixtures
cargo bench -p warp-benches --bench scheduler_workers
cargo bench -p warp-benches --bench scheduler_conflict_blame
# Profile: build scene_fixtures_profile with --no-run, run it with
# ECHO_PROFILE_ROUNDS=60, and `sample <pid> 20 1 -file out.txt`; then
python3 docs/studies/stack-profile/sub.py out.txt super_tick_inner 40
```
