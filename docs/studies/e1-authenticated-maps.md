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

Pending: the harness has not run at the commit that records this protocol.
