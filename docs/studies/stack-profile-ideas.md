<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Ideas: left on the table, and new ones

**Status:** untested ideas, not results. Each names the measurement behind
it, from [stack-profile](stack-profile.md) and its
[ledger](stack-profile/ledger.md), or says that none exists yet.
**Author:** Claude (Anthropic), for James Ross.

Shares quoted as "% of an F5 tick" come from the sampled profile of F5 tier
M under the application profile, inside `super_tick_inner`. Shares quoted as
"% of a 10k tick" come from the profile of a 10,000-write engine tick
(`scheduler_workers`, no work, one worker), taken **before** the commit-path
fixes; that profile needs retaking.

## Left on the table

Discussed in this study, not built or not measured.

| Idea | Evidence so far | Identity change | Next step |
| --- | --- | --- | --- |
| **Envelope dispatch**: dispatch from the envelope; ingress records as ops; apply once, with no guard and no undo | Guard, touch log and ingress writes: ~0.1% of an F5 tick | None | Build for design value only, if wanted |
| **Admission-time reservation** | Reservation and drain: 0.7% of an F5 tick | None | Same |
| **Owner marks** in the reservation sets, so the blocker query is O(footprint × log n) | The query costs ~253–279 ms at 10k rewrites, 50% contended | None | Only if blame is queried often |
| **Single-call hashing for the other long digests** (plan, rewrites, receipt) | In the 10k tick: plan and rewrites 12.8%, receipt 8.4% (before the fixes) | None | Same change as the patch digest |
| **One-block trie hashes**: domain and split folded into a precomputed key, so a branch is 64 bytes | Branch hashes are 21.5% of an F5 tick, at two compressions each | M2 v0 recipe | Probe branch; owner decides the recipe |
| **Homomorphic set hash** (LtHash) updated from the diff | Not measured | New root recipe; no openings | The accumulator benchmark below |
| **Paged (wide) trie** | Not measured | New recipe | Same benchmark |
| **Lazy root**: commits bind `parent + patch_digest`, root sealed every K ticks | The trie is 39% of an F5 tick | Protocol | Write-up first; tied to the M2 full seal |
| **Hash ingress keys at admission** | Record keys and leaves are ~5% of an F5 tick | None | Small; latency, not throughput |
| **Declared deferral** as a `ConflictPolicy` (`Retry` exists but is unimplemented) | None | Receipt records the deferral | Probe after a contended fixture exists |
| **GPU batch hashing** (trie levels or set-hash terms) | Not measured | None | Only after the re-profile shows hashing is still large |
| **GPU executors** for uniform systems | 1,000 hashes × 10k rewrites: 543.7 ms on 1 worker, 66.7 ms on 18 | None | Needs a fixture with real per-entity work (F6) |
| **Accumulator microbenchmark**: trie, LtHash and paged, at F5 size, 10× and 100× | — | None (benchmark only) | Ready to build |
| **Re-profile the 10k tick** after the fixes | 22.4 ms remain | — | Next measurement |
| **Footprint allocation**: freeing footprints | 7.4% of a 10k tick | None | See footprint representation below |
| **Reservation sets** (`GenSet`) | 11.6% of a 10k tick | None | See dense slot ids below |
| **Fixtures F1 and F11** (claims and ledger contention), and an F6 parallel variant | — | None | After F6 runs |
| **ZK and Verkle at seals**: field-native boundary commitment, committed dense index, commitment to the diff stream | Design reading of `zk-wormhole-compression.md` | Proof layer | Only with the M2 full-seal decision |
| **hello-echo lint target**: one shared target directory | 528 MB + 527 MB of duplicate lint output | None | A hello-echo change, for its owner |

## New ideas

Ordered by expected value. Each is a hypothesis.

### Measure what has not been measured

1. **Durability on.** Every figure in this study runs with durability set
   to none. A real host writes its WAL, and an fsync per tick may well
   dominate everything above. Measure F5 with the WAL on, then try
   **group commit**: one fsync for several ticks, with acknowledgements held
   until it lands. The semantics do not change, because acknowledgement
   already waits for durability.
2. **How much of F5's graph is ingress bookkeeping.** Each intent leaves an
   event node, its attachment and an edge to a shared kind node in the
   state, so the graph grows by three records with every admitted intent. That growth makes the trie
   deeper, the legacy root walk longer, replay slower and memory larger. If
   the records live in provenance instead of state, which the envelope
   design does, the state stays small. Counting the share of F5's records
   that are ingress records is a five-minute check, and it would revalue the
   envelope idea: not as a cleanup, but as smaller state forever.
3. **Allocations per tick.** `echo-alloc-witness` already exists. Count
   allocations in a 10k tick and in an F5 tick. The aim is a steady-state
   tick that allocates almost nothing: retain the scratch vectors for
   drained, reserved, receipt entries and ops across ticks, as
   `state_root_scratch` already does.

### Take hashing off the critical path

4. **Pipeline the tick.** Hashing never changes state. After tick N's ops
   are applied, tick N+1 can reserve and execute against that state while
   N's patch digest, state root and commit hash finish on another core.
   This is deterministic, because hashing's inputs are fixed once N is
   applied. A game frame then pays for execution and apply. Hashing overlaps
   the next frame and is ready before the WAL acknowledgement needs it.
5. **One serialization, several digests.** The plan, rewrites and receipt
   digests walk the same entries. Serialize them once into one buffer and
   hash the three ranges. That changes no format.
6. **Skip the touched diff when executor ops are already canonical.** The
   patch is rebuilt by diffing touched slots after apply (2.5% of a 10k
   tick). The merged executor ops are already the delta after filtering
   no-ops. `delta_validate` already asserts that the two agree, which is
   ready-made evidence for removing the diff.

### Make footprints cheap

7. **Sorted vectors instead of `BTreeSet`s in footprints.** Building,
   checking and freeing footprints is a large share of a big tick (7.4% for
   freeing alone). Small sorted vectors allocate once, free once, and check
   conflicts by merge. If interned per tick, they compare as integers.
8. **Dense slot ids for reservation.** Reservation is greedy, in canonical
   order, against hashed sets (11.6% of a 10k tick). Sort and deduplicate a
   batch's claimed slots once, giving each a dense index, then run the same
   greedy pass against flat arrays of owner and reader marks. A candidate is
   accepted only when none of its slots conflicts with an accepted
   candidate, as now. A single sweep where "the first claimant wins" would
   **not** match: that claimant may itself be rejected over another slot.
   The owner array also gives blockers directly, from a rejected candidate
   to the accepted owner it hit, which makes owner marks unnecessary.
9. **Range write claims.** F6's footprint must run the whole physics step,
   because a footprint must name every edge it writes, and contact edges
   depend on the step. `OutgoingEdges` read claims exist; a write claim over
   `(source, edge type)` would let a rule say "I may add or remove contact
   edges from these bodies" without simulating. That halves F6's work per
   frame. Edict could check it at compile time, as it does other
   footprints.
10. **Prepared execution.** Where a footprint must compute what the
   executor will compute, let the footprint hand its result to the executor
   instead of computing twice. This is the general form of 9.

### Use more cores without breaking determinism

11. **A persistent worker pool.** With 18 workers, a 100-rewrite tick costs
    0.379 ms, against 0.192 ms with one, because threads are spawned and
    joined per tick. Parked workers that live across ticks remove that, and
    an adaptive count (one worker below a work threshold) removes the rest.
12. **Commit worldlines in parallel.** A super-tick commits writer heads one
    after another in canonical order. Distinct worldlines share no state, so
    their commits could run in parallel, with results recorded in canonical
    order. A host with many tenants or many strands gains as many cores as
    it has worldlines.
13. **Staged ticks (phases).** Games want integrate, then collide, then
    resolve, each phase partitioned by cell with no conflicts inside a
    phase. Echo has one reservation per tick. A tick with declared phases,
    each with its own reservation and a barrier between, would let F6 run
    per-cell rewrites on all cores without the rejection that kills that
    design today. It is a scheduler feature, not a rule hack.

### Shape data for the workload

14. **Slot granularity as a knob.** F6 stores one body per node: about 49
    bytes of state carrying a node, an attachment and an edge, each a slot
    with its own trie leaf and hashes. Storing a cell's bodies as one
    attachment (a component array) cuts slots, ops and hashes by the bodies
    per cell, at the price of coarser conflicts. Run F6 both ways.
15. **Persistent state for concurrent readers.** The render read walks the
    live state between ticks. If the store were persistent, like the M1
    trie, a renderer could read frame N while frame N+1 executes, with no
    copy and no lock.
16. **Tick coalescing.** F5 commits one intent per tick by construction.
    Under load, a host could tick when N intents are pending or T has
    passed, spreading the fixed per-tick cost (about 35 µs on F5 after the
    inline change) across N intents. It is a latency and throughput knob:
    measure F5 with 1, 8 and 64 intents per tick.

## Suggested order

1. Re-profile the 10k tick, count F5's ingress share, and measure durability
   on. All three are measurements, and each can reorder the rest.
2. Build and run F6 (`claude/game-fixture`), so the remaining ideas have a
   game to answer to.
3. Then the cheapest identity-free wins: the persistent pool, single-call
   digests, sorted-vector footprints and dense slot ids for reservation.
4. Then the larger designs: pipelined hashing, staged ticks, range write
   claims, and the accumulator benchmark that informs the root decision.
