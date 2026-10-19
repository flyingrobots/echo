<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Study: F5's state-root cost, and staged work across a restart

**Status:** study, on a probe branch that is not for merge. It adds
`host_test`-only instrumentation to `compute_state_root_with` and a field to
the F5 fixture's observation. No behaviour changes in builds without
`host_test`.
**Author:** Claude (Anthropic), for James Ross.
**Follows:** [the E1 study](e1-authenticated-maps.md), which named both
checks as open.

## 1. Does a per-tick whole-state root dominate F5's append cost?

**Yes, for tier M.** Every F5 append computes the whole reachable state root
exactly once. In tier M those computations are 96.4% of the append time.

| Tier | Appends | Root computations | Root time | Append time | Share |
| --- | --- | --- | --- | --- | --- |
| S | 200 | 200 | 15.10 ms | 28.37 ms | 53.2% |
| M | 5,000 | 5,000 | 5,917.96 ms | 6,135.80 ms | 96.4% |

Over ten windows of tier M, the root's mean time grows with the reachable
node count, from 113 µs at 284 nodes to 2,340 µs at 4,784. The cost is 400
to 490 ns per reachable node. The mean append time follows it window by
window, from 150 µs to 2,388 µs. The remainder stays between 36 and 48 µs per
append. That remainder is the rest of submission, admission and tick.

**Method.**
- **Where it is counted.** `compute_state_root_with` (`snapshot.rs`) is the
  function every whole-state root computation passes through: the engine's
  commit path, `WorldlineState::state_root` and
  `compute_state_root_for_warp_state`. The probe records its reachable node
  count and elapsed time on each call.
- **Why there.** The existing `state_root_computation_count_for_test` counts
  only calls through `WorldlineState::state_root`, which the engine's commit
  path does not use.
- **Phase boundary.** The F5 fixture clears the probe before its append clock
  and takes the records when the clock stops, so the replay phase is
  excluded.
- **Build and run.** Release test profile with features
  `native_rule_bootstrap,trusted_runtime,host_test`, seed 7. Output:
  [`f5-probe.txt`](f5-state-root-and-staged-recovery/f5-probe.txt).

**Caveats.**
- **Instrumentation overhead.** The probe takes one `Instant` and one mutex
  push per call. Tier M's append time here was 6.14 s, against 5.51 s in the
  uninstrumented bench on the same host.
- **Tier S.** It is not dominated by the root: half of its smaller per-append
  time is elsewhere.

**Consequence.** Most of the gap between F5-M's 1.08 ms per append and E1's
9.4 µs incremental maintenance is this one computation. E1 measured that
computation's replacement, but only outside Echo. The figure is a ceiling on
what M1 shadow integration could save. It is not a prediction of M1's result.

## 2. Does staged but unticked work survive a WAL restart?

**Only as accepted, unstaged work.** A host recovers from the filesystem
runtime WAL after the submission was acknowledged durable
(`submit_intent_with_runtime_wal_ack`) and staged with a ticket, but before
any tick. Recovery restores the witnessed submission. It does not restore the
staging.

| Point | Witnessed | Ticketed records | Outcome |
| --- | --- | --- | --- |
| Before restart | yes | 1 | `Pending`, with a ticketed-ingress id |
| After recovery | yes | 0 | `Pending`, `ticketed_ingress_id: None` |
| After ticking, before restaging | yes | 0 | still `Pending`; 0 committed steps |
| Redeliver, then restage with the same ticket | duplicate | `Staged` (not `Duplicate`) | |
| After ticking | | | `Applied`; 1 provenance entry |

The work is neither lost nor duplicated, provided the host stages it again.
Nothing restages it automatically: a recovered host that only ticks leaves
it pending indefinitely.

Staging carries no durability claim in its documentation, so this is not a
contradiction of a stated contract. It does settle the question the
consolidation left open: "staged but unticked" is a process-local state.
Comparing admission across a restart must treat it as such. Any durable
staging promise would be new work, not existing behaviour.

**Method:** `study_staged_unticked_submission_across_restart`, added to
`trusted_runtime_host_loop_tests.rs` on this branch, with features
`native_rule_bootstrap,trusted_runtime,host_test`. It asserts only that the
work applies exactly once; the rest is recorded. Output:
[`staged-restart.txt`](f5-state-root-and-staged-recovery/staged-restart.txt),
with long byte arrays elided.
