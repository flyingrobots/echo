<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Study: the stack on main, a game fixture, and durability

**Status:** measurement branch `claude/measure-stack`, not for merge.
Candidates for landing are separate branches, listed below.
**Author:** Claude (Anthropic), for James Ross.
**Follows:** [stack-profile](https://github.com/flyingrobots/echo/blob/claude/stack-envelope/docs/studies/stack-profile.md)
on `claude/stack-envelope`, written against a `main` that has since moved.
**Figures:** every number here is copied from
[`stack-on-main/summary.md`](stack-on-main/summary.md) or
[`stack-on-main/ledger.md`](stack-on-main/ledger.md), which `ledger.py`
generates from the run logs in `stack-on-main/runs/`. One machine (this
workstation), Rust 1.96, release benches, `CARGO_INCREMENTAL=0`.

## Why this study exists

By 2026-10-20, #918 to #923 had all merged, so Echo `main` (`0faaf58a`)
already holds M1, M2, #918 and the scene fixtures. The review of the
previous study asked for the inline worker and the commit-path fixes as
**isolated changes, with clean checks on fixed commits**, and for
durability-on figures and a compiled F6. This study does that on the new
`main`.

## Branches

| Branch                    | Commit            | What it is                                                                                                                                               |
| ------------------------- | ----------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `claude/inline-worker`    | `d578e157`        | `main` + one worker runs on the calling thread. Candidate.                                                                                               |
| `claude/commit-path-once` | `747f320c`        | `main` + patch digest hashed in one call, slots and ops sorted once, and a test that the new op canonicalization matches the map it replaced. Candidate. |
| `claude/f6-game-scenes`   | `9a0c9bd3`        | `main` + fixture F6. Candidate (bench and test only).                                                                                                    |
| `claude/measure-stack`    | tip of this study | `main` + bench drivers + F5 durability bench + F6 + both candidates + the allocation census + this study. Measurement only.                              |

The two candidates do not depend on each other or on anything unmerged.
The commit-path change was cherry-picked from `f118d7ef`; its only conflict
was lazy-blockers context, and `main`'s eager `blocked_by` is kept as it is.

## Checks

On each candidate (`checks2.sh`): `cargo fmt --check`, both clippy runs from
the handoff, the SPDX check, the feature-gated test check, and
`scripts/ban-globals.sh`. All pass on `d578e157`, `747f320c` and
`9a0c9bd3` (campaign 4).

Full `warp-core` suite with `native_rule_bootstrap`, `trusted_runtime`,
`host_test`, `delta_validate`, `application_root_profile` and `det_fixed`
(campaign 1): `main` and the inline worker each sum to 2,055 passed and 1
failed; the commit path sums to 2,056 passed and 1 failed, `main` plus its
new canonicalization test. The one failure is the same on all three and on
the stack tip: `special_files_are_rejected_before_a_read_attempt`, which
cannot bind a Unix socket because the scratch path is longer than
`SUN_LEN`. A failure shared with the baseline is evidence against a
regression, not a green suite; each candidate's branch reruns that test from
a short path.

An earlier version of this study reported 2,054 for `main` and the inline
worker. In each of those logs, parallel test output split or interleaved one
`test result:` line, and `ledger.py` counted only lines that began with it.
It now matches the result counts anywhere in a line. The same fix raises the
tip's (`c8d8fd25`) total from 2,056 to 2,057.

The stack tip `b0cef149` passes the same checks (campaign 5), and its
previous commit `c8d8fd25` ran the full suite with the same single
environmental failure (campaign 4).

**hello-echo** (campaign 5): its whole CI sequence (syntax, producer lock,
runtime, effect and patch suites, clean build, writer-epoch assertions,
host fmt and clippy) passes with Edict `0c8cf37` and Echo `main`
`0faaf58a`, with no golden changed. Branch `claude/echo-main-0faaf58a`
(`4596202`) on hello-echo carries the pin bump on top of
`claude/advance-producers`.

The old stack's checks were void, and it shows: F6 needed one clippy fix
(`9a0c9bd3`), two benches carried from that stack failed clippy
(fixed in this branch), and one carried bench was not rustfmt-clean.

## Findings

### 1. The commit-path change is worth landing; the inline worker too, for F5

From the summary tables, stack tip (`both`) against `main` plus benches
(`base`):

- **10,000-write tick, one worker:** 30.378 ms → 21.819 ms. The inline
  worker alone moved it to 29.396 ms; the commit path did the rest.
- **F5 tier M, application profile, append:** 260.08 ms → 167.86 ms with the
  inline worker, 167.46 ms with both. This is where the inline worker
  matters: each F5 tick has one small rewrite, and spawning a thread for it
  was the cost.
- **F6 pile, application profile, frame p50:** 98.832 ms → 80.627 ms, almost
  all from the commit path.
- **F6 bullets, legacy profile:** frames over 16.7 ms went 63 → 18 → 0.

Below a millisecond (100-rewrite ticks) the runs disagree with themselves:
the inline revision is slower even with 18 workers, a path it does not
touch. Treat sub-millisecond rows as noise at this sample size.

### 2. F6 runs, and the pile scene is far over a frame budget

F6 is valid in every run: the graph holds exactly the reference world, and
replaying retained patches reproduces the live root.

- **Arena** (256 bodies, few contacts): about 2 ms a frame, never over budget.
- **Bullets** (projectile churn, ~2,000 live bodies): at the budget on
  `main`; under it at the stack tip, with two spikes per run near 26–28 ms
  on the application profile.
- **Pile** (2,048 bodies, up to 15,335 contacts): about 80 ms a frame at
  the tip, every frame but a few over budget. The game's own step is
  about 2.3 ms.

**The `echo_model` column overstates Echo.** It subtracts twice the
reference step time (the rule runs the step in its footprint and its
executor). The pile profile (`c4-prof-pile-sub.txt`) shows the rule's own
code, which reads the whole world from the graph and diffs it, is a
large share of the tick: `game::footprint` and `game::execute` together
are 2,496 + 1,830 of 14,555 tick samples, against 811 for the step itself.
That is game-side work done through Echo's read API, and a real game would
not read its whole world back every frame. A per-cell or incremental rule
would change this; Echo rejects conflicting rewrites outright, which is why
F6 does not split the frame yet.

Inside Echo, the same profile shows the application trie advance at 23.9%
of the tick, even under the legacy profile: the bench is built with
`application_root_profile`, which turns on the M1 shadow trie for every
worldline. **The legacy-profile F5 and F6 figures in this study and the
previous one include the shadow trie's cost.**

### 3. Durability dominates F5

F5 tier M at the stack tip, application profile, append p50:

| Durability     |       Append | Per append |
| -------------- | -----------: | ---------: |
| none           |    171.81 ms |  0.0342 ms |
| in-memory WAL  |    653.35 ms |  0.1298 ms |
| filesystem WAL | 19,768.71 ms |  3.9968 ms |

The filesystem WAL writes 20,760,256 bytes for 5,000 appends, about 4 KB
per append. On this machine the filesystem WAL makes an append more than a
hundred times dearer than the same append without it.
Rust's `File::sync_all` on macOS issues `F_FULLFSYNC`, which is the right
durability boundary and an expensive one.

The store already has a deferred-commit path
(`append_transaction_deferring_commit_sync`, `sync_deferred_commits`), and
the host uses it on one path. Next: count the synced WAL transactions per
append (submission, admission, tick), and measure group commit with
acknowledgements held until the shared sync lands.

### 4. Half of F5's state is ingress bookkeeping

After tier M, F5's warp holds 5,033 fixture nodes and 5,001 others; 5,032
fixture edges and 5,000 others; 5,000 attachments on fixture nodes and
5,000 on others. One record of each kind per admitted intent, kept in the
state forever. This revalues the envelope idea from the previous study:
records kept in provenance instead of state would halve F5's state, its
trie and its replay.

### 5. A 10,000-rewrite tick allocates 17 KB per rewrite

The allocation census (`c4-alloc.txt`, stack tip) counts calls and bytes
during `commit_with_receipt`. In steady state, 136,861 allocations and
173,537,776 bytes for 10,000 rewrites (13.69 and 17,353.8 per rewrite);
1,592 allocations and 1,620,400 bytes for 100. Each rewrite writes one
32-byte attachment. Something in the commit allocates kilobytes per
rewrite; the 10k profile points at footprints (dropping them is 9.3% of
the commit) and the per-tick vectors.

### 6. Where the 10k tick goes now

From `c4-prof-10k-sub.txt`, under `commit_with_receipt`: building the
patch 24.6% (mostly sorting slots and ops), reservation 14.6%,
`blake3::Hasher::update` 12.9% (the plan, rewrites and receipt digests
still hash field by field), dropping footprints 9.3%, applying ops 8.4%.
The single-call hashing already done for the patch digest applies to the
other three digests, with no identity change.

## Next, in order

1. **Group commit** for the filesystem WAL, after counting syncs per append.
2. **The allocation hot spots**: footprint representation and retained
   per-tick scratch vectors, with a witness in `echo-alloc-witness` that
   fails on today's count.
3. **Single-call hashing** for the plan, rewrites and receipt digests.
4. **An F6 rule that does not read the world back**: the step's input
   state carried incrementally, so F6 measures Echo, not the fixture.
5. The envelope design (ingress records out of state), now with a measured
   size argument.

## Decisions for James

- **Land `claude/inline-worker` and `claude/commit-path-once`?** Both pass
  every check on fixed commits, change no digest (golden vectors GV-001 to
  GV-004 pass), and are independent.
- **Land F6 (`claude/f6-game-scenes`)?** Bench and test only.
- **The `echo_model` column:** keep it as a stated model, or replace it
  once the rule stops reading the world back.
- **Open a hello-echo PR** from `claude/echo-main-0faaf58a`? It passes
  every CI step at Echo `main`.
- Still open from the handoff: a declared deferral
  `ConflictPolicy`, `Rejected.blocked_by` under lazy blockers, and the
  set-hash root.
