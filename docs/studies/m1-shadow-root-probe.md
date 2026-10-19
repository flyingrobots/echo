<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Study: an M1 shadow root inside the runtime, and an incomplete write set

**Status:** study, on a probe branch that is not for merge. It adds the
`shadow_root_probe` feature. With that feature, an incremental application
root is kept beside the legacy root after every runtime commit. Without it,
nothing changes. This is a feasibility probe for M1's shadow mode, not M1
itself.
**Author:** Claude (Anthropic), for James Ross.
**Follows:** [E1](e1-authenticated-maps.md), which selected `radix-v0` as the
M1 experimental profile, and
[the F5 state-root study](f5-state-root-and-staged-recovery.md).

## Findings

1. **A tick patch's `out_slots` is not its write set.** Each F5 commit writes
   six slots, according to its own patch ops. Only three of them appear in
   `out_slots`. The three missing ones are written by runtime-ingress
   materialization (`materialize_runtime_ingress_event`): the event node, its
   intent attachment, and the event-to-kind edge. Across tier M the counts
   are:
   - 29,994 slots written by ops;
   - 19,996 slots declared in `out_slots`;
   - 14,997 slots written but not declared.

   `out_slots` is built from footprints, and materialization writes the store
   outside any footprint. The declared set also names the bucket node, which
   the ops never write.
2. **Built from the ops instead, the shadow root is exact.** The incremental
   `radix-v0` root equals its independent rebuild after every F5 commit, in
   tiers S and M. It covers every record in every warp: 30,066 records at the
   end of tier M. The legacy root covers the 4,784 reachable nodes.
3. **The shadow costs about 3% of its legacy neighbour.** Over tier M the
   shadow update costs 168 ms in all, 33.6 µs per commit, while the legacy
   root costs 6,087 ms. In the last tenth of the run, the shadow costs 41 µs
   per commit and the legacy root about 2,340 µs.

## Why finding 1 matters

`echo_operation_written_slots_v1` collects `patch.out_slots`. Interval
corroboration, the evidence behind the scoped stale-basis exception (#837),
uses it to decide that an Action's target was not written between its origin
and its parent. Any write missing from `out_slots` is invisible to that
check.

I could not construct an Action that exploits this. The undeclared writes
happen once per newly materialized ingress, which deduplication prevents
from repeating. Corroboration matters only when a value changes and is
restored, and a single write cannot do both. Value comparison at the parent
also catches a single change. So this is a gap in the evidence, not a
demonstrated wrong commit.

The safe repairs are:
- derive interval writes from the patch ops, which are complete for these
  commits;
- or declare materialization writes in `out_slots`.

**For M1:** "derive record updates from the existing transaction/touch
boundary" must mean the ops or the touch log, not `out_slots`.

## Method

**Hook.** After each runtime commit in `SchedulerCoordinator`
(`coordinator.rs`), `shadow_root_probe::after_commit` maps each `WarpOp` to
the slots it can change:

| Op | Slots |
| --- | --- |
| Node upsert | the node |
| Node delete | the node and its alpha attachment |
| Edge upsert | the edge |
| Edge delete | the edge and its beta attachment |
| Attachment set | the attachment |
| Portal open | the key and the child root |

Warp-instance ops would trigger a full rebuild; none occurred.

**Update.** For each slot, the hook reads the committed record, maps it to
`H(m1.record.v0 ‖ kind ‖ ids)` and its canonical bytes, and applies the batch
to a persistent `radix-v0` map. The first commit bootstraps from every
record.

**Parity.** At the end of each F5 run, `rebuild_root` hashes every record in
the live state into a fresh map. A record-level diff against a mirror of the
shadow found the `out_slots` gap. The first version of the probe used
`out_slots`, and parity failed with 597 missing records in tier S.

**Run.** Release test profile, features
`native_rule_bootstrap,trusted_runtime,host_test,shadow_root_probe`, F5 seed
7. Every run held `host/heavy-work` under the resource guard. Output:
[`shadow-probe.txt`](m1-shadow-root-probe/shadow-probe.txt).

| Tier M window | Shadow hashed bytes per commit | Shadow µs per commit | Legacy root µs per commit (F5 study) |
| --- | --- | --- | --- |
| 0 | 4,927 | 16.5 | 113 |
| 4 | 6,651 | 31.6 | 1,028 |
| 9 | 7,190 | 41.3 | 2,340 |

The shadow's hashed bytes grow with log₂ of the record count, as E1 found.
Its time also includes a global mutex, the op-to-slot mapping and record
reads.

## Not covered

- Boundary ports.
- Warp-instance headers.
- Commits rolled back by the state journal.
- Action-commit (`material`) paths: they pass through the same hook, but F5
  does not exercise them.
- Concurrency.
- The legacy root is still computed. Removing it is not M1's job.

## After the fix

Finding 1 is fixed on branch `claude/corroboration-write-set` (`3d32a2a8`, from `origin/main` `25308042`). `extend_echo_operation_written_slots_v1` now collects each patch's `out_slots` plus every slot its ops write, through `WarpOp::extend_written_slots`. A warp-instance upsert or delete makes the interval's writes unknown, so the whole-frontier rule applies across it.

| Check | Result |
| --- | --- |
| New unit test `interval_written_slots_include_writes_outside_declared_footprints` | Without the fix it fails: `assertion failed: written.contains(&…SlotId::Node(event))` ([RED](m1-shadow-root-probe/fix-red.txt)). With the fix it passes ([GREEN](m1-shadow-root-probe/fix-green.txt)). |
| Full `warp-core` suite, `native_rule_bootstrap,trusted_runtime,host_test`, on the fix branch | 1,942 passed. One failed: `special_files_are_rejected_before_a_read_attempt`, with "path must be shorter than SUN_LEN". It fails identically without the fix: the scratch `TMPDIR` path is too long for a Unix socket. |
| Clippy `-D warnings` (with and without features) and fmt | Pass |

This branch (`claude/m1-shadow-probe-v2`) cherry-picks the fix as `61b1a2b5`. The probe now takes its slots from the same write set corroboration uses, rather than its own mapping, and was run again ([output](m1-shadow-root-probe/shadow-probe-after-fix.txt)):

| Tier | Parity | Records | Write-set slots per commit | Of which outside `out_slots` | Shadow / legacy root |
| --- | --- | --- | --- | --- | --- |
| S | holds | 1,218 | 7 | 3 | 8.0 / 18.3 ms |
| M | holds | 30,066 | 7 | 3 | 184 / 5,923 ms |

The write set now has seven slots per commit: the six the ops write plus the bucket node, which `out_slots` declares but the ops never write. With the fix, corroboration and the shadow use one definition of "written".

On this branch, the library unit tests and the `executable_operation_pipeline_tests`, `trusted_runtime_host_loop_tests`, `scene_fixture_tests` and `f5_state_root_probe` targets all pass: 1,009 tests, none failed.
