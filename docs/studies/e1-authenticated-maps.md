<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Study: E1, complete authenticated maps

**Status:** experiment. This page records a protocol and its results. It
does not describe Echo's current behaviour, and it adopts no commitment
profile. Owner adoption of the recommendation it informs is not established.
**Author:** Claude (Anthropic), for James Ross.
**Source:** experiment E1 in *Echo: Causal Commitments and Authenticated WARP
Readings*, revision 2 (ChatGPT, 2026-10-18), and §6 of its consolidated
comparison-contracts recommendation. Both are filed in the Reader vault
rather than in this repository.

## Question

E1 asks which authenticated-map shape Echo should carry into M1 shadow
integration. It compares a full fixed-depth sparse Merkle tree with a
compressed binary radix (Patricia) Merkle trie. It uses complete
implementations: insert, replace, delete, batches, persistent versions,
membership and absence proofs, strict proof codecs and an independent
rebuild oracle. The retained hashing-only benchmark compared neither of
those, and the revision says it "does not satisfy E1".

## Implementations

Both live in [`experiments/echo-authmap-e1`](../../experiments/echo-authmap-e1/README.md),
a separate Cargo workspace outside Echo's default build.

| Profile | Shape | Leaf and node hashes | Proof |
| --- | --- | --- | --- |
| `sparse-v0` | 256 levels; empty subtrees are precomputed and never stored | `H(leaf ‖ k ‖ v)`, `H(node ‖ l ‖ r)` | Kind, value digest, a 256-bit bitmap of non-empty siblings, and those siblings |
| `radix-v0` | Branches only where keys diverge; a key set has exactly one trie | `H(leaf ‖ k ‖ v)`, `H(branch ‖ bit ‖ l ‖ r)` | Kind, `(bit, sibling)` pairs with strictly increasing bits, and the reached leaf |

The profiles are experimental `v0` codecs, fixed for repeatability. Both
maps share one metered hash function and one node-table type, so their
counters are comparable. Payloads are hashed once into a value digest and
counted apart from structural hashing. A radix branch also stores one
representative key, which is not hashed; it is counted in written bytes.

## Gates

A candidate that fails any gate is not eligible for selection.

| Gate | Requirement | Witness |
| --- | --- | --- |
| G1 | Incremental root equals the independent rebuild at every checkpoint. | `tests/conformance.rs`, `parity` in every workload |
| G2 | Batches, single steps and insertion order agree; random histories of inserts, replacements and deletes read and prove correctly. | `tests/conformance.rs` |
| G3 | No false claim verifies. No mutated encoding verifies. | `tests/conformance.rs`, W7 |
| G4 | Every retained old version reads and proves correctly after later writes. | `tests/conformance.rs`, W4 |

## Workloads

All workloads are deterministic, seeded and single-threaded. Values are 64
bytes unless stated. The harness is `src/bin/e1.rs`.

| ID | Workload | What it answers |
| --- | --- | --- |
| W1 | Load N ∈ {1k, 4k, 16k, 64k} in one batch, then 2,000 single replacements. | Cost per update as the map grows |
| W2 | At N = 16k, batches of 1, 16, 256 and 4,096 replacements. | Whether cost tracks the records changed |
| W3 | At N = 16k, 4,000 alternating deletes and new inserts. | Deletion-heavy history |
| W4 | At N = 4k, 20,000 replacements, every version retained, in windows of 2,000; then 200 claims at 20 old versions. | Whether cost grows with history, and persistence |
| W5 | 1,000 membership and 1,000 absence proofs at each N. | Proof bytes, prove and verify time |
| W6 | At N = 16k, a 256-key comb (every shared-prefix length) and 4,000 keys sharing a 40-bit prefix. | Adversarial key prefixes |
| W7 | At N = 16k, 1,000 proofs × (64 byte mutations, 8 truncations, 1 extension, 4 other keys). | Proof-codec soundness and canonicality |
| W8 | At N = 16k, single replacements with 32 B, 1 KiB and 64 KiB values. | Payload cost apart from structure |
| W9 | D ∈ {1, 4, 16} nested instances of 1,000 records: re-embed each child root in its parent (`Descend` chain) or keep one outer instance map. | Outer instance map versus embedding |
| W10 | The F5 scene fixture's write stream, tiers S and M, seed 7: three record writes per tick, in ten windows; plus a full rebuild of every record at each window boundary. | Realistic mutations from Echo's own fixture, and incremental versus whole-record recomputation |

W10 replays exactly the F5 script in
`crates/warp-core/benches/support/fixtures/think_append.rs`. The experiment
reproduces that generator and asserts Echo's pinned golden digest for F5-S
seed 7 (`efc63336…3d4d`). The record mapping (node, alpha attachment and edge
keyed by `H(e1.record.v0 ‖ kind ‖ id)`) belongs to the experiment.
Engine-side ingress records are not modelled.

As a reference, the Echo `scene_fixtures` bench reruns on the same host at
this branch's base. It reports F5 append and replay times through
`TrustedRuntimeHost` with the v1 state root.

## Decision rule

Declared before any measurement ran.

- **R1, maintenance.** Candidate X dominates Y if, in W1 at N = 64k, both of
  X's mean hashed bytes and mean node bytes written per single update are at
  most half of Y's.
- **R2, proofs.** X's p99 proof size in W5 at N = 64k (membership and
  absence) is at most 1.5 times Y's. X's largest W6 proof is at most 16 KiB.
- **Selection.** Select X as the M1 experimental profile if X passes every
  gate, R1 and R2. If R1 holds and R2 fails, select nothing and report the
  trade-off. If neither candidate dominates, E1 is inconclusive at these
  workloads.
- **Wall time.** Reported but not decisive: one host, no isolation. If the
  wall-time direction contradicts the structural counters by more than a
  factor of 2, that is flagged.
- **W9.** An outer instance map is justified if, at D = 16, its hashed bytes
  per leaf update are at most a quarter of the `Descend` chain's.
- **W10.** Incremental maintenance passes if its mean hashed bytes per tick
  in the last window are at most 1.25 times the first window's. The rebuild's
  growth is reported, not judged.

**Predictions, recorded so they can be wrong.** `sparse-v0` should hash 257
nodes per single update at every N, about 21 KiB. `radix-v0` should hash
about log₂ N + 1 nodes, about 17 at N = 64k. Proofs should be similar, about
600 bytes at 64k for both.

## Limits fixed in advance

- The node table is in memory. Store reads and writes are counted, but no
  storage device is exercised, so cold-path latency is not measured.
- Every version is retained; nothing is ever collected.
- No concurrent writers or worker parity.
- The experiment's maps are not wired into Echo; Echo's root, receipts and
  validity are unchanged.

## Results

The protocol and harness were committed at `aec4b2ec`, before any
measurement ran. One run of `e1 all` at that commit produced
[`results.jsonl`](e1-authenticated-maps/results.jsonl). Host: Apple M5 Pro,
64 GiB, macOS (Darwin 27.0.0), release build, Rust 1.96. The run took 38 s
with peak RSS 4.69 GiB, under a 12 GiB guard.

### Decision

**Select `radix-v0` as the M1 experimental profile.** It passes every gate,
R1 and R2. `sparse-v0` passes every gate and fails R1 against `radix-v0`.

| Rule | Measured | Result |
| --- | --- | --- |
| G1 parity | Every `parity` field is true for both candidates; conformance passes. | Pass, both |
| G3 soundness | W7: 77,000 trials each, 0 false claims, 0 accepted mutated encodings. | Pass, both |
| G4 persistence | W4: 200 claims at 20 old versions, 0 wrong, for each candidate. | Pass, both |
| R1 at 64k | `radix-v0` hashes 1,449 B and writes 1,661 B per update. `sparse-v0` hashes 20,817 B and writes 16,448 B. The ratios are 7.0% and 10.1%. | `radix-v0` dominates |
| R2 at 64k | p99 proof bytes: `radix-v0` 682 (membership) and 680 (absence); `sparse-v0` 674 and 642. The worst ratio is 1.06. The largest W6 `radix-v0` proof is 8,706 B. | Pass |
| W9 | At D = 16, `radix-v0`'s outer instance map hashes 1,364 B per leaf update, against 15,087 B for the `Descend` chain (9.0%). | Outer map justified |
| W10 | `radix-v0`'s mean hashed bytes per tick grow from the first window to the last: 1.65× for tier S, 1.45× for tier M. `sparse-v0`'s grow 1.00×. | **Fails as written** for `radix-v0`; passes for `sparse-v0` |

The W10 failure stands as declared. In W10 the live record set grows: 8×
in tier S, 9.6× in tier M. `radix-v0`'s path length grows with log₂ of the
record count, and so does its cost. The rule I wrote tested growth with live
size where it meant growth with history. W4 is the history test: there,
`radix-v0` holds steady at 1,111–1,117 B per update over 20,000 retained
versions. The rule is not rewritten after the fact. A later version of E1
should declare a W10 bound normalised by depth.

Wall time agrees with the structural counters in direction everywhere, so
nothing is flagged.

### Predictions

- **`sparse-v0`:** predicted 257 hashes per single update. Measured 257 at
  every N, 20,817 B.
- **`radix-v0`:** predicted about 17 hashes at 64k. Measured 17.3.
- **Proofs:** predicted about 600 B at 64k for both. Measured means of
  590–601 B (`radix-v0`) and 557–587 B (`sparse-v0`).

### Measurements

| Workload | `radix-v0` | `sparse-v0` |
| --- | --- | --- |
| W1, 64k, per update | 17.3 hashes, 1,449 B hashed, 1,661 B written, p50 4.4 µs | 257 hashes, 20,817 B, 16,448 B, p50 113 µs |
| W1, 64k, build and retained nodes | 42 ms; 162,584 nodes | 4.32 s; 15,947,176 nodes |
| W2, 4,096-key batch, per key | 273 B hashed | 17,486 B hashed |
| W3, delete / insert | 1,116 / 1,340 B hashed | 1,236 / 20,817 B hashed |
| W4, first / last window | 1,111 / 1,116 B hashed | 20,817 / 20,817 B hashed |
| W5, 64k, verify p50 | 1.8 µs | 27.0 µs (membership), 2.8 µs (absence) |
| W6 comb, update / largest proof | 10,908 B hashed / 8,706 B | 20,817 B / 8,226 B |
| W6 40-bit shared prefix, largest proof | 1,022 B | 994 B |
| W8, 64 KiB values | 1,278 B structural, plus 65,536 B payload | 20,817 B structural, plus 65,536 B payload |
| W9, D = 16, outer map / chain | 1,364 / 15,087 B hashed; read proofs 559 / 6,164 B | 41,634 / 333,072 B; 591 / 6,316 B |
| W10 M, last window, per tick | 3,587 B hashed, p50 9.4 µs | 62,070 B hashed, p50 137 µs |
| W10 M, whole-record rebuild at 15,065 records | 3.95 ms (oracle) | 404 ms (oracle) |

### F5 against Echo's own runtime

On the same host, the Echo `scene_fixtures` bench at this branch's base
(`0545dda9`) ran five valid rounds per tier
([output](e1-authenticated-maps/echo-f5-bench.txt)). It took 0.078 ms per
append for tier S and 1.08 ms per append for tier M, with replay at
5,428 ms for tier M.

On the same tier M stream, incremental `radix-v0` maintenance costs
9.4 µs per tick at the end. A whole-record rebuild costs 3.95 ms at the end,
growing linearly in the record count. This is consistent with the open
hypothesis that F5-M's per-append time is dominated by recomputing the state
root over the whole graph each tick. It does not test that hypothesis: the
experiment's records are not Echo's graph, and no Echo profile was taken. The
direct check is a per-tick count of state-root computations in F5, using the
existing `state_root_computation_count_for_test`.

### What this does not establish

- No storage device, cold paths or eviction.
- No concurrency.
- No Echo integration; that is M1's shadow mode.
- No production profile. Adoption still requires codec verification,
  mutation coverage, publication and recovery behaviour, and legacy
  compatibility, as revision 2 states.

## Later findings

Follow-ups on other branches answer this study's open items.

- **F5's per-append cost is the state root.** Branch `claude/f5-state-root-count`, `docs/studies/f5-state-root-and-staged-recovery.md` there. Every append computes the whole reachable root exactly once. In tier M that is 96.4% of the append time.
- **M1 shadow inside Echo.** Branch `claude/m1-shadow-probe-v2`, `docs/studies/m1-shadow-root-probe.md` there.
  - **Result:** a `radix-v0` application root over every record, updated after each runtime commit, matches its independent rebuild through F5.
  - **Cost:** in tier M, 184 ms of shadow against 5,923 ms of legacy root.
- **M1 must not derive updates from `out_slots`.** A tick patch's `out_slots` omitted the runtime-ingress writes.
  - **What it broke:** the first shadow, which failed parity, and the stale-Action corroboration check, which read the same list.
  - **The repair:** branch `claude/corroboration-write-set`, which also has its own study.
