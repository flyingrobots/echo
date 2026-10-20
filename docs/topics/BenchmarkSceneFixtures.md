<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Benchmark Scene Fixtures

> A performance improvement counts only when the workload is unchanged and
> every declared semantic obligation still holds.

This topic owns the benchmark fixture identity, validation and measurement
contract. It relates to the scheduler performance contract in
[`scheduler-performance-warp-core.md`](../benchmarks/scheduler-performance-warp-core.md).
A fixture is a declared world, what happens to it, what is done
with its history, and what must be true afterward. Each fixture is modeled on
a real use of Echo, or on a shape a real use would produce.

## The fixture contract

### Three parts, and the obligations they carry

```text
fixture = scene × intent stream × history operation, plus declared outcomes
```

- **Scene**: the world before measurement: warps, nodes, edges, attachments.
- **Intent stream**: what is submitted, by which writer heads, with what
  footprints, skew and arrival pattern, and where the logical tick
  boundaries fall.
- **History operation**: what is done with the resulting history: nothing
  (steady ticking), replay or restore, readings at earlier coordinates,
  strands forked and settled or discarded, or suffix export and import.
- **Declared outcomes**: what must hold after the run (see
  [Validation](#validation)).

### Three identities, kept separate

| Identity                     | Defined by                                                                   | Changes when                                                              |
| ---------------------------- | ---------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| **Workload identity**        | The fixture manifest (below)                                                 | The workload changes. Never because Echo changed                          |
| **Expected-result identity** | Declared outcomes, and any goldens under a named root-format version         | Echo's promised semantics or a format version change, with an explanation |
| **Run identity**             | Implementation revision, build, environment, machine conditions, raw samples | Every run                                                                 |

An Echo optimization changes the run identity. It must not silently change
the workload identity, and it may change the expected-result identity only
through an explained, versioned change. "Updated the snapshots" is not an
explanation.

One manifest file may hold all of a fixture's declarations, but the fields
that feed each digest are separated explicitly:

```text
workload_id    = digest(canonical workload specification)
expectation_id = digest(workload_id + assertions + output format versions)
```

Expected roots, receipts and other goldens belong to the expectation part and
never enter the workload digest, so changing a golden cannot change the
workload. Small conformance cases also pin a digest of the generated logical
scene and script: identical dimensions do not establish an identical workload,
because a graph can be rewired without changing its node or edge count.

### The manifest

Each fixture has a versioned manifest that fixes the workload independently of
Echo's implementation:

- generator version, seed, pseudo-random algorithm and sampling rules;
- resolved dimensions: nodes, edges, warps, attachment bytes, ticks, writer
  heads, readers, history operations;
- submission order or delivery rules, including logical tick boundaries;
- the admission, retention, checkpoint and durability settings in force;
- initialization method: direct scene construction, or history built by
  operations (replay fixtures need the latter; see below);
- the evidence class of the fixture (observed, modeled or exploratory) and
  pinned source revisions for any application it models;
- the declared outcomes, in the expectation part (above).

Manifests are small and checked in. Bulk scenes are generated from them.

**Randomness.** `StdRng` and `SmallRng` in `rand` carry no reproducibility
guarantee across versions or platforms, and `warp-core` has no `rand`
dependency; adding one would change `Cargo.lock`, which the provider package
binds. Fixtures therefore use a small pseudo-random generator defined in the
repository (for example SplitMix64), with published test vectors. Test vectors
fix the output sequence only, so the mappings built on it are pinned too:
bounded integers, shuffles and any other distribution, with their arithmetic
and boundary behavior. Only the distributions the implemented fixtures need
are built; Zipf, for example, arrives with the first workload that uses it. A
generator or mapping change bumps the manifest's generator version.

### Requested and realized dimensions

A fixture records the dimensions it was asked for, the dimensions it realized,
and the reason for any difference. A workload is never shrunk silently to fit
a limit; a refusal is a recorded outcome.

Size tiers are explicit tuples, not labels. "100k entities" says little when
each entity owns component nodes and a child warp. Every tier bounds total
nodes, total edges, warps, payload bytes, history length and peak live
strands. An illustrative nested-scene declaration:

| Tier | Entities | Total nodes | Total edges | Child warps | Payload bytes | Ticks | Peak strands |
| ---- | -------: | ----------: | ----------: | ----------: | ------------: | ----: | -----------: |
| S    |    1,000 |    ≤ 10,000 |    ≤ 12,000 |       1,000 |      ≤ 0.5 MB |   600 |            0 |
| M    |   10,000 |   ≤ 100,000 |   ≤ 120,000 |      10,000 |        ≤ 5 MB |   600 |            0 |
| L    |  100,000 | ≤ 1,000,000 | ≤ 1,200,000 |     100,000 |       ≤ 50 MB |   600 |            0 |

The figures are illustrative; each fixture's first implementation sets and
pins its own.

### Two concurrency modes

| Mode                            | Purpose                                        | Required control                                                                       |
| ------------------------------- | ---------------------------------------------- | -------------------------------------------------------------------------------------- |
| **Fixed causal workload**       | Compare implementations and execution profiles | Same declared inputs, causal constraints and promised outcome equivalence on every run |
| **Concurrent arrival workload** | Measure admission, queueing and contention     | A declared arrival process, captured deliveries, and complete outcome accounting       |

In both modes the workload is generated before execution, as a logical
script. Values the runtime produces, such as commit identities or returned
handles, are not predicted: a later step refers to the result of an earlier
named step through an explicit binding. Worker scheduling must never decide
which random numbers are consumed and thereby produce a different workload.

**Overlap is an input; rejection is an outcome.** A fixture declares
footprints, skew and arrival patterns, and reports the conflict and refusal
rates it realized. Exact expected rejection counts belong only to cases
constructed to produce them.

### Validation

Generator conformance is tested independently: test vectors for the
pseudo-random generator, and pinned realized dimensions for each manifest.

Every measured execution then validates its declared outcomes, outside the
timed region. Outcomes cover whatever the fixture exercises:

- accepted, refused and deferred operations;
- readings at declared checkpoints;
- replay and restore results;
- settlement results;
- import behavior.

**Identical final state does not establish identical causal history.** An
`A → B → A` workload can expose a history bug while leaving the final value
unchanged, so validation checks history-level outcomes (receipts, commit
identities, readings at earlier coordinates), not only the final root.

State-root goldens remain useful within an explicit root-format version. Small
cases also need expectations that do not come from Echo itself: hand-computed
values, or invariants stated in the manifest. Shared generators are good;
shared mistakes are still mistakes.

### Result classes and gates

Every execution is classified:

| Class           | Meaning                                                                                                                                 | May enter a performance comparison      |
| --------------- | --------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------- |
| **Valid**       | Generator conformance, required capability coverage and every declared outcome hold. An expected refusal that occurs is a valid outcome | Yes                                     |
| **Invalid**     | A declared outcome failed                                                                                                               | No; timings are kept for diagnosis only |
| **Unsupported** | A required capability is missing. It never counts as passing coverage                                                                   | No                                      |
| **Incomplete**  | The run stopped before its declared work finished                                                                                       | No; timings are kept for diagnosis only |

A stubbed operation must never report an impressive zero.

Gates are two separate policies:

- **Correctness gate:** generator conformance, required capability coverage
  and declared outcomes pass. This gate is enabled first.
- **Performance gate:** a specified comparison protocol finds an actionable
  regression. It becomes blocking only once its environment, comparison method
  and tolerance are established.

Before any speedup is computed, a comparison eligibility check confirms the
same workload identity, compatible expectations, the same measurement
boundaries and execution settings, and valid results on both sides. When they
differ it names the difference ("durability policy changed") instead of
printing a percentage.

### Construction method

Direct scene construction (building a `GraphStore` and wrapping it in a
`WorldlineState`) is valid for isolated microbenchmarks of reading costs.
Fixtures that replay, restore, read earlier coordinates or exchange suffixes
need real history built by admitted operations. The manifest records which
method was used, and a result never mixes them silently.

### Retained evidence

Bulk scenes are regenerated, not stored. Small evidence is checked in:
manifests, generator test vectors, expected outcomes, and minimized regression
traces. Regenerating a million-node world is sensible; a twelve-operation bug
reproducer should be kept.

## Measurement contract

### End-to-end, then components

Every fixture reports an **end-to-end measurement**, plus **clearly defined
component measurements** that explain it. Commit, receipts, roots and the log
can be nested or overlapping costs, so each component declares its boundary
and whether it is contained in another.

Every benchmark declares:

- what starts and stops the clock;
- whether queueing, materialization, allocation and storage synchronization
  are inside it;
- whether it measures cold execution, warmed execution, or continuing history
  growth;
- that setup, validation and reset happen outside measurement;
- whether the build is normal or an instrumented diagnostic build.

The uninstrumented result is authoritative for performance claims. Diagnostic
splits, such as walk against hash, come from instrumented builds and explain
the result; they are never the claim.

### Repetition and comparison

Results come from repeated samples, and baseline and candidate runs are
interleaved in the same session, so a change in machine conditions affects
both. A single quiet baseline does not establish that conditions stayed
comparable. Each result reports its sample count and distribution.

### Work accounting

Latency without a denominator can reward dropping work. Each run reports, as
applicable:

- operation occurrences and delivery attempts, separately: one accepted
  occurrence delivered ten times is one useful commit, not ten;
- submitted, admitted, committed, refused, deferred and unfinished work. These
  are stages a unit of work passes through, not disjoint buckets to sum;
- useful throughput: completed declared work per unit time;
- for sustained workloads, queue growth and latency distributions with sample
  counts;
- for history-heavy fixtures, peak memory, retained history bytes, checkpoint
  bytes, and bytes transferred. Memory figures name their boundary: the
  pre-generated script and the validator can be large, so harness memory is
  reported separately where possible, and combined process memory is labeled
  as such.

### Run conditions

Each result records the commit, toolchain, build profile, environment (native
or VM, with CPU and memory limits), what else was running, and the interleaved
baseline. The timing study found that load average does not reveal
memory-bandwidth contention: concurrent memory-heavy work raised a 66 ms root
to 93–119 ms while the load average looked ordinary.

## Properties every fixture declares

| Property                                                                   | Why it matters                                                       |
| -------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| Reachable size: nodes, edges, bytes, separately                            | The study observed per-node cost rising with scene size on one shape |
| Shape: fan-out, depth, cycles, nodes reached by many paths                 | Walk queue, revisits, edge filter                                    |
| Warp nesting: count and depth, on nodes and on edges                       | The walk enters `Descend` on both; no current bench has one          |
| Attachment bytes: inline, or a reference to content held outside the graph | Hash bytes against hash structure                                    |
| Writes per tick: count, attachment-only or structural, local or scattered  | Incremental-root designs treat these differently                     |
| Footprints, skew and arrival pattern                                       | Inputs; conflict and refusal rates are outputs                       |
| Retention obligations: worldlines, strands, pinned readings                | What may and may not be reclaimed                                    |
| Readers: count, view size, pinned coordinate                               | Readings under concurrent writes                                     |
| History: length, strands, suffix size                                      | Log, replay and transport costs                                      |

## Implementation principles

- **Generated from checked-in manifests.** Bulk scenes are rebuilt from code
  and a manifest; small evidence (manifests, vectors, outcomes, minimized
  traces) is checked in.
- **Self-describing.** A generator prints its requested and realized
  dimensions.
- **Validated on every run.** Declared outcomes are checked outside the timed
  region on every measured execution; generator conformance is tested
  separately.
- **Shared by benches and tests.** The same generators can feed determinism
  tests, so a benchmark scene is also a correctness scene.
- **No probe in the deterministic path.** Timing splits use diagnostic
  patches or feature-gated instrumentation, never `Instant` or `eprintln!` in
  shipped code.

## Implemented fixtures

The C1 star control and F5 append-and-replay workload are implemented.
Their conformance is checked by
`crates/warp-core/tests/scene_fixture_tests.rs`, which CI runs with
`native_rule_bootstrap,trusted_runtime`.

| Part           | Where                                      | What it establishes                                                                                                                                         |
| -------------- | ------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Generator      | `benches/support/fixtures/prng.rs`         | SplitMix64 output vectors, cross-checked against an independent implementation; the bounded-integer mapping pinned separately, including a bound above 2^63 |
| Manifests      | `benches/support/fixtures/manifest.rs`     | `workload_id` and `expectation_id` digested separately, with `edict.canonical-cbor/v1`; a test shows a golden change leaves `workload_id` unchanged         |
| Result classes | `benches/support/fixtures/outcome.rs`      | Valid, invalid, unsupported and incomplete; only valid results are comparable                                                                               |
| C1             | `benches/support/fixtures/star.rs`         | The star scene built independently of `action_scene`, with identical state roots at 1, 100 and 1,000 cells, and a pinned root                               |
| F5             | `benches/support/fixtures/think_append.rs` | Append and replay through the trusted host, validated                                                                                                       |
| Runner         | `benches/scene_fixtures.rs`                | A fixed matrix (tiers S and M, seed 7, five rounds each) of classified rounds; a summary only when every round is valid                                     |

### F5 append and replay

- **Construction.** Each append is an intent submitted through
  `TrustedRuntimeHost`, admitted as an installed native contract package, and
  committed in its own tick. One rule writes the entry node, its payload
  attachment and the bucket-to-entry edge. Buckets are linked to the root, so
  entries are inside the state root.
- **Replay mechanism: applying retained patches.**
  `ProvenanceService::replay_worldline_state` rebuilds the worldline from the
  initial state; Echo checks each tick's state root, patch digest and commit
  hash as it applies them. The fixture then checks that the rebuilt root
  equals the live root.
- **Readings at earlier coordinates.** `replay_worldline_state_at` at evenly
  spaced ticks; each rebuilt root must equal the root recorded when that tick
  committed.
- **Not covered, and reported as such.** Re-executing admitted operations:
  Echo has no re-execution path for native-rule intents on `main`. Restoring
  stored records: WAL recovery is not wired into this fixture yet.
- **Outcomes, with expectations from the tier rather than from Echo.** Every
  append committed and none refused; exactly one admitted append per
  consecutive tick; every entry present with its scripted payload and an edge
  from its bucket; the last recorded root equals the live root; retained-patch
  replay reproduces the live root; checkpoint readings equal their recorded
  roots; the
  supplied script matches the digest recorded from the executed script; the
  optional pinned script digest matches too.
- **Canaries.** The test feeds the validator a missing accepted operation, an
  incorrect historical reading, a failed replay, a duplicated authored
  effect, a missing entry, unfinished work, and a workload other than the
  pinned one, including a real execution under a different seed. Each is
  classified invalid or incomplete and is never
  comparable.
- **Clocks.** `append` covers submit, admit and tick for every append, with
  envelopes built beforehand. `replay` covers the full patch replay.
  Validation, outcome collection and checkpoint readings are outside both. The
  runner is a normal build; it does not measure memory yet.

Gates: the **correctness gate** is the conformance test
above. The **performance gate** is not enabled; its environment, comparison
protocol and tolerance are not yet established.

## Sources

- Echo, `main` at `8f3f48f2`:
    - `README.md` and `docs/architecture/there-is-no-graph.md`: framing.
    - `docs/NORTHSTAR.md`: jedit.
    - `crates/warp-core`: the graph model (`graph.rs`, `attachment.rs`,
      `warp_state.rs`, `snapshot.rs`), footprints (`footprint.rs`), inbox
      budgets (`head_inbox.rs`), operation limits (`echo_operation.rs`), and
      the 65,536-node view limit (`edict_read/view.rs`).
    - Existing benches in `crates/warp-core/benches` and
      `crates/warp-benches/benches`.
- Graft at `9b399de4`: `src/warp/index-head.ts`, `src/warp/ast-emitter.ts`,
  `src/echo/structural-history-client.ts`.
- git-warp at `4889004e`: `docs/READINGS_AND_OPTICS.md`, the census of a real
  Think store.
- XYPH: the README on GitHub (`flyingrobots/xyph`), default branch at `f2e6d267` when this revision was written.
- Dojo: a documentation snapshot of 2026-10-09, including ADR-0003, the JIT
  Context Builder design and the Contextual Claims specification.
- James Ross, _CΩMPUTER: The Geometry of All Possible Machines_ (2025),
  prologue and chapters 1–3.
- The state-root timing study of 2026-10-17
  (`docs/benchmarks/state-root-timing-study.md`, local branch
  `docs/state-root-timing-study`).
- Two external reviews of earlier revisions of this document, supplied by the
  owner on 2026-10-17. The first cites the Rust Rand Book on reproducibility
  and Google Benchmark's documentation on random interleaving.
