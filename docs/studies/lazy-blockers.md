<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Study: derive footprint blockers on demand, not at every tick

**Status:** probe on `claude/lazy-blockers`, built on Echo `main` at
`4e801ef2`. Not for merge as it stands: it changes a public host outcome
field and a receipt validation rule, which are the owner's decisions.
**Author:** Claude (Anthropic), for James Ross.
**Question (James):** a rejected rewrite has been rejected. Why does the tick
work out which rewrites blocked it? Everything needed to say so is
recoverable, so could that be lazy?

## Result

Yes. On the engine path, making blocker attribution lazy removes the tick's
quadratic cost and breaks nothing that hashes, replays or recovers.

| Rewrites | Contended | Rejected | `main` commit p50 | Lazy commit p50 | Speed-up |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 1,000 | 0% | 0 | 1.20 ms | 1.61 ms | none (noise) |
| 1,000 | 50% | 499 | 3.07 ms | 0.68 ms | 4.5× |
| 3,000 | 50% | 1,499 | 25.84 ms | 2.38 ms | 10.9× |
| 10,000 | 0% | 0 | 13.41 ms | 13.60 ms | none |
| 10,000 | 10% | 999 | 110.78 ms | 12.29 ms | 9.0× |
| 10,000 | 50% | 4,999 | 274.24 ms | 8.95 ms | 30.6× |
| 10,000 | 90% | 8,999 | 93.33 ms | 5.50 ms | 17.0× |

- **With no conflicts nothing changes,** because `main` never scanned then.
- **With conflicts, a tick now gets cheaper as more rewrites are rejected,**
  since rejected rewrites don't execute. On `main` it got much more
  expensive.
- **The cost moved; it did not disappear.** Asking for the whole tick's
  blockers afterwards costs about what the tick used to pay: 253 ms at
  10,000 rewrites and 50% contention. That is paid only when someone asks.

Full tables: [`lazy-blockers/bench-main.txt`](lazy-blockers/bench-main.txt)
and [`lazy-blockers/bench-lazy.txt`](lazy-blockers/bench-lazy.txt). Apple M5
Pro, 18 cores, release build, Rust 1.96, 15 ticks per row, under the
`host/heavy-work` lock.

## What the tick did, and why it was quadratic

Each tick reserves candidates in canonical order. `RadixScheduler::reserve`
answers "does this footprint overlap one already accepted?" with set lookups,
which is cheap. The receipt also records, for each rejection, *which*
accepted candidates it overlaps. The scheduler's sets do not know that, so
`reserve_for_receipt` compared each rejected candidate's footprint with every
accepted one: rejected × accepted footprint comparisons per tick. Its own
comment said "consider spatial indexing if tick candidate counts grow large".

## What the probe changes

1. **The tick records only the decision.** `reserve_for_receipt` no longer
   scans; a footprint-conflict rejection carries an empty blocker list.
2. **`Engine::footprint_blockers(state_before, receipt)`** derives the lists
   on demand. It recomputes every entry's footprint from its rule and scope
   against the state the tick's footprints came from, and names each earlier
   applied entry whose footprint conflicts, using the same
   `footprints_conflict` predicate.
3. **Validation accepts a rejection without recorded blockers.**
   `TickReceipt::try_from_retained_parts` and the provenance codec's
   canonical-receipt check no longer require one. Every other blocker rule
   (ascending, earlier, applied) is unchanged.
4. **Benchmark:** `warp-benches/benches/scheduler_conflict_blame.rs`, an
   engine tick in which a chosen share of rewrites write one shared node.

## Why this changes no identity

`compute_tick_receipt_digest` hashes the receipt's entries and attribution,
not `blocked_by`. So the receipt digest, decision digest and commit hash are
unchanged by construction. The suite agrees: no digest, replay, recovery or
golden-vector test failed.

Something worth knowing either way: on `main`, the stored blocker lists are
written into retained provenance, but nothing hashes them. They are a stored
claim that the commit does not authenticate. A list derived on demand from
committed facts can't drift from them.

## Did it break anything?

Full `warp-core` suite with `native_rule_bootstrap`, `trusted_runtime`,
`host_test` and `delta_validate`: 2,014 passed, 6 failed
([`lazy-blockers/full-suite.txt`](lazy-blockers/full-suite.txt)).

| Test | Why it failed | Now |
| --- | --- | --- |
| `special_files_are_rejected_before_a_read_attempt` | Scratch temp path longer than a socket path; fails identically on `main` | Unrelated |
| `tick_receipt_tests::retained_receipt_parts_reject_every_noncanonical_blocker_shape` | Expected `RejectedEntryMissingBlockers` | Updated: that shape is now canonical |
| `tick_receipt_tests::commit_with_receipt_records_accept_reject_and_matches_snapshot_digests` | Asserted the stored list `[0]` | Updated to assert `footprint_blockers` gives `[0]` |
| `tick_receipt_tests::commit_with_receipt_records_multi_blocker_causality` | Asserted the stored list `[0, 1]` | Updated to assert `footprint_blockers` gives `[0, 1]` |
| `installed_contract_intent_pipeline_tests::footprint_conflict_is_final_without_hidden_retry` | Host `IntentOutcomeDecision::Rejected.blocked_by` is now empty | **Still failing:** public host contract |
| `external_consumer_contract_fixture_tests::serious_external_consumer_fixture_proves_hosted_contract_path` | Host `IntentOutcome::Rejected.blocked_by` is now empty | **Still failing:** public host contract |

Every real failure asserts the stored blocker list, the thing being changed.
The two still failing are the honest cost: the host tells a submitter
"rejected, blocked by entry 0", and in this probe it says "rejected" only.
Keeping that answer needs an on-demand query at the host, described below.

## How I know the lazy lists are right

- **Differential test against the removed code.** Test builds keep the old
  scan's output (`eager_blockers_for_test`). `on_demand_blockers_equal_the_removed_eager_scan`
  builds 400 random contended ticks (two rules; random write and read edges
  into up to five shared nodes; up to 40 entities) and asserts
  `footprint_blockers` equals the old lists exactly, and that every rejection
  has at least one blocker. The test also asserts that the seeds produce more than
  400 rejections in total, so it can't pass vacuously.
- **Mutants.** Keeping only the first blocker, and counting only
  write–write overlaps, each fail the test.
- **The existing range-claim test** now checks its blocker through
  `footprint_blockers`.

## Limits and open questions

- **Which state to recompute against.** The lazy query must see the state
  the footprints were computed from. For an engine tick that is the state at
  commit entry. For a runtime tick on `main`, ingress is written into the
  store before matching, so it is the parent state *plus* that tick's ingress
  records. Under the envelope design discussed with James (dispatch from the
  envelope, ingress records as ops), it would be the parent state.
- **Descended instances.** A rewrite queued inside a descended instance also
  reads its descent chain, and the receipt does not keep that chain.
  `footprint_blockers` covers rewrites queued with an empty descent stack,
  which is every rewrite queued through `Engine::apply`.
- **The Action path is unchanged.** `commit_scheduler_action_batch_to_state_v1`
  caps a batch at 64 Actions, so its scan is at most 2,016 comparisons, and
  there the scan *is* the conflict decision. Its outcome record
  (`EchoOperationFootprintConflictV1.blocked_by`) is also encoded. Making it
  lazy would need its prepared footprints, which are not retained. Not
  measured here.
- **The query is still quadratic.** If blame is asked for often, owner marks
  in the reservation sets would make it O(footprint × log n) per rejection.

## Decisions for the owner

1. Whether a tick should stop recording blockers. That means changing the
   receipt validation rule and the provenance check, as in this probe.
2. What the host's `Rejected { blocked_by }` becomes: an on-demand query, a
   field filled lazily, or dropped.
3. Whether the stored lists in retained provenance should be dropped from
   the encoding, or kept and authenticated.

## Reproduce

```sh
cargo bench -p warp-benches --bench scheduler_conflict_blame
cargo test -p warp-core --lib -- lazy_blocker range_tests
cargo test --locked -p warp-core \
  --features native_rule_bootstrap,trusted_runtime,host_test,delta_validate \
  --no-fail-fast
```
