<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Benchmark Scene Fixtures

Status: design, approved for implementation. The direction was approved by
the owner on 2026-10-17; two external reviews the owner supplied the same day
tightened the fixture contract. The first implementation slice lands with this
revision. Written by Claude from
the sources listed at the end.

> A performance improvement counts only when the workload is unchanged and
> every declared semantic obligation still holds.

This document defines a suite of scene fixtures for Echo's benchmarking work,
now and later. A fixture is a declared world, what happens to it, what is done
with its history, and what must be true afterwards. Each fixture is modeled on
a real use of Echo, or on a shape a real use would produce.

## Why a suite

Echo's benchmarks today run on one scene. `action_scene.rs` builds one warp: a
root node, N cell nodes with 7-byte attachments, and one edge from the root to
each cell. Its Actions only change attachments. `warp-benches` adds a linear
chain (`snapshot_hash`), a no-op scheduler drain, motion and materialization
benches, and a hub-degree case (`commit_hub_bucket`). No current bench has a
child warp, a deep tree, cross-links, large attachments, structural churn, a
strand, a replay, or a suffix exchange.

The state-root timing study (`docs/benchmarks/state-root-timing-study.md`, on
the local branch `docs/state-root-timing-study`) shows why scene shape and run
conditions have to travel with every number:

- On one star-shaped scene and one machine, the observed cost per node of the
  walk rose from about 50 ns at 1,000 cells to about 300 ns at 300,000. That
  is an observation about that scene, not a law; the suite measures nodes,
  edges, bytes and topology separately so that it can be tested.
- A widely cited figure, 102 ms at 100,000 cells, was reproduced only under
  concurrent memory-heavy load. The quiet figure was 65–72 ms.

Echo's own framing raises the bar further. The README says causal history is
primary and "everything else is derived"; `docs/architecture/there-is-no-graph.md`
calls the graph "a coordinate chart over witnessed causal history". A suite
that only builds graphs and times their roots measures one reading and none of
the history. Echo's particular risk is an optimization that keeps the visible
state while making the history less faithful. The fixture contract below is
designed to make that trade impossible to hide.

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

| Identity | Defined by | Changes when |
| --- | --- | --- |
| **Workload identity** | The fixture manifest (below) | The workload changes. Never because Echo changed |
| **Expected-result identity** | Declared outcomes, and any goldens under a named root-format version | Echo's promised semantics or a format version change, with an explanation |
| **Run identity** | Implementation revision, build, environment, machine conditions, raw samples | Every run |

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
strands. Example for F6:

| Tier | Entities | Total nodes | Total edges | Child warps | Payload bytes | Ticks | Peak strands |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| S | 1,000 | ≤ 10,000 | ≤ 12,000 | 1,000 | ≤ 0.5 MB | 600 | 0 |
| M | 10,000 | ≤ 100,000 | ≤ 120,000 | 10,000 | ≤ 5 MB | 600 | 0 |
| L | 100,000 | ≤ 1,000,000 | ≤ 1,200,000 | 100,000 | ≤ 50 MB | 600 | 0 |

The figures are illustrative; each fixture's first implementation sets and
pins its own.

### Two concurrency modes

| Mode | Purpose | Required control |
| --- | --- | --- |
| **Fixed causal workload** | Compare implementations and execution profiles | Same declared inputs, causal constraints and promised outcome equivalence on every run |
| **Concurrent arrival workload** | Measure admission, queueing and contention | A declared arrival process, captured deliveries, and complete outcome accounting |

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

| Class | Meaning | May enter a performance comparison |
| --- | --- | --- |
| **Valid** | Generator conformance, required capability coverage and every declared outcome hold. An expected refusal that occurs is a valid outcome | Yes |
| **Invalid** | A declared outcome failed | No; timings are kept for diagnosis only |
| **Unsupported** | A required capability is missing. It never counts as passing coverage | No |
| **Incomplete** | The run stopped before its declared work finished | No; timings are kept for diagnosis only |

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

| Property | Why it matters |
| --- | --- |
| Reachable size: nodes, edges, bytes, separately | The study observed per-node cost rising with scene size on one shape |
| Shape: fan-out, depth, cycles, nodes reached by many paths | Walk queue, revisits, edge filter |
| Warp nesting: count and depth, on nodes and on edges | The walk enters `Descend` on both; no current bench has one |
| Attachment bytes: inline, or a reference to content held outside the graph | Hash bytes against hash structure |
| Writes per tick: count, attachment-only or structural, local or scattered | Incremental-root designs treat these differently |
| Footprints, skew and arrival pattern | Inputs; conflict and refusal rates are outputs |
| Retention obligations: worldlines, strands, pinned readings | What may and may not be reclaimed |
| Readers: count, view size, pinned coordinate | Readings under concurrent writes |
| History: length, strands, suffix size | Log, replay and transport costs |

## Fixture catalog

Fixtures are grouped by priority and labeled by **evidence class**:

- **Observed**: derived from a recorded workload of the application.
- **Modeled**: derived from the application's code or design, not from a
  recorded workload.
- **Exploratory**: a shape worth measuring, not yet tied to a concrete user.

The catalog is a direction and a backlog, not a list of release gates. A small
gate set is chosen from it (see [Sequence](#proposed-sequence-of-changes));
broader sweeps run on dedicated machines.

### Tier 1: Echo's first users

#### F1. XYPH planning (modeled)

XYPH is an agentic planning substrate. Its README says it is moving toward
being a native Continuum participant, with work recorded as receipt-bearing,
capability-bound transitions.

- **Scene.** A rooted DAG, the "Genealogy of Intent": human-authorized intent
  roots → quests → acceptance criteria → evidence. Settlement records (Scrolls)
  are attached to finished quests. Provenance chains are long; every quest
  traces to an intent.
- **Intent stream.** Agents ask for the next quest and claim it. A claim is
  exclusive, so many agents pile onto the few quests that are ready at once.
  Capability-bound actions declare their reads, writes and spend before they
  run.
- **History.** Speculative worldlines settled into `worldline:live`; offline
  participants that diverge and exchange suffixes.
- **Reads.** Read-dominated. `briefing`, `next` and `context` compute derived
  judgments (`nextLawfulActions`, `blockingReasons`) over bounded views.
- **Outcomes.** At most one active claim per quest within the receiving
  history and the claim's lifetime, with the rule for independent branches
  scoped explicitly. Exactly one winner is required only in constructed cases
  where a successful claim is expected; zero successes can be legitimate when
  no request is lawful or admissible. Every refusal is accounted for, and
  settled quests carry their evidence.
- **Stresses.** Writers piling onto the same targets, read-heavy bounded
  views, provenance depth, settlement.

#### F2. Dojo claims and JIT context (modeled)

Dojo's ADR-0003 makes Echo its causal runtime.

- **Scene.** A forest of contextual claim terms: frames wrap claims, which can
  be grouped, offered as alternatives, negated or guarded, to depth 3–10. Each
  term points at a few hub entities, such as people. Raw transcripts stay
  outside the graph; the graph holds references to their content.
- **Intent stream.** Batches of claims written as artifacts are processed.
- **History and reads.** Many short-lived agents, each given a bounded view
  (the JIT context payload) at a pinned coordinate while writes continue.
  Interactive latency is the budget.
- **Boundaries.** Bounded readings below, at and above the 65,536-node view
  limit, with the expected behavior of each specified; large scenes also
  serve legitimately small readings.
- **Stresses.** Nested small trees with hubs, payloads by reference, many
  concurrent bounded readers.

#### F3. Graft: code over time (modeled)

Modeled on Graft's indexer (`src/warp/index-head.ts`). Whole-file subtree
replacement is **the modeled indexing strategy**; its inclusion here does not
endorse it as the ideal long-term design.

- **Scene.** A directory tree 5–10 levels deep → files → 10–100 symbols per
  file, with one 1–100 KB AST payload per file. Cross-file `references` and
  `calls` edges form cycles through mutual imports. Commit nodes touch files.
  Variant F3b puts each file's AST in a child warp of 1k–10k nodes.
- **Intent stream.** Per commit, one file's subtree is replaced: its symbols
  deleted and re-added, in batches of at most 64 files. Every file in a commit
  also writes the shared `dir:` and `commit:` nodes.
- **History.** Long; readings of a file at earlier commits.
- **Stresses.** Structural subtree replacement, large payloads, hot shared
  nodes, deep trees, nested warps (F3b).

#### F4. Structured editing (modeled; jedit fidelity pending)

`docs/NORTHSTAR.md` names jedit as the immediate product pressure, with an
app-facing surface of `optic.applyIntent(...)` and `optic.textWindow(...)`.
Until jedit's operations and representation are settled, this fixture is
named "structured editing" and does not claim jedit coverage; it adopts
jedit's model when one exists.

- **Scene.** A document tree (sections → paragraphs → runs) with ordered
  siblings.
- **Intent stream.** Several writers editing near the same place; speculative
  edits as strands.
- **Reads.** Windowed readings over the region each writer is viewing.
- **History.** Offline edits merged by suffix import.
- **Stresses.** Ordered-list edits, local contention, windowed readings,
  strands.

#### F5. Think: append and replay (modeled from an observed census)

Based on the census of a real Think store in git-warp's
`docs/READINGS_AND_OPTICS.md`, and the append shape it prescribes. It is
labeled census-derived, and is compared with Think's implementation when that
is available; that comparison does not block it.

- **Scene.** A few bucket nodes (day or topic) with a wide star of entries
  under each; tiny payloads; sparse cross-links between entries. History is
  built by admitted operations, not direct construction.
- **Intent stream.** One append per tick, with an empty read set.
- **History operation.** Replay of the whole history, and readings at earlier
  coordinates. "Replay" names one of three different experiments, and F5
  declares which it runs:

  | Operation | What it establishes |
  | --- | --- |
  | Re-execute admitted operations | Execution reproduces the required effects and evidence |
  | Apply retained patches | Recorded transitions reconstruct the required readings |
  | Restore stored records | Persistence preserves and retrieves the required records |

  Reading an original receipt back does not show that replay independently
  reproduced it; that is a persistence check, which supports a different claim.
  The mechanisms F5 uses, and the equality checked for each, are listed under
  [F5 in the first slice](#f5-in-the-first-slice).
- **Outcomes.** For each replay mechanism used, its declared equalities hold,
  and each checkpoint reading matches.
- **Regression variant.** One page node rewritten on every append, the
  anti-pattern the census found, whose patches grew quadratically.
- **Stresses.** Log growth, replay, wide buckets.

### Tier 2: other uses with distinct shapes

| Fixture | Class | Scene | Intent stream | History operation | Stresses |
| --- | --- | --- | --- | --- | --- |
| **F6. Real-time game** | exploratory | Regions → entities → component nodes (32–64 B); an NPC behavior tree in a child warp each; short-lived collision edges | 10–30% of entities move (attachment-only); spawns, despawns, region moves | Steady ticking; replay | Many writes per tick, attachment-only path, many small warps |
| **F6r. Rollback** | exploratory, non-gating | F6 | F6 plus late inputs | Re-simulate the last k ticks in a strand per late input | Strand fork and settle per frame |
| **F7. Web server** | exploratory | Large cold user and session store; static routing tree; a short-lived subgraph per request | Many writer heads; hot keys on a Zipf distribution | Steady; create/delete churn | Churn, unreachable storage, hot keys, inbox budgets |
| **F8. Build pipeline** | exploratory | DAG with heavy fan-in and long chains | A source changes; invalidation spreads along reverse edges | Provenance readings at earlier coordinates | Reverse index, cascading writes, nodes reached by many paths |
| **F9. Compiler-shaped transformation** | exploratory | Module → functions, each with a control-flow child warp → blocks → instruction trees; passes as `Descend` on edges | A pass rewrites one function's interior | Steady | Deep nesting, `Descend` on edges, thousands of small warps |
| **F10. Overlapping neighborhood operations** | exploratory | Bodies and joints; dense and cyclic | Each operation touches one body's neighborhood | Steady | Cycles, naturally high footprint overlap |
| **F11. Ledger / contract host** | exploratory | Accounts in a star; receipts as an append-only log | Two-account transfers with hot accounts on a Zipf distribution | Audit readings far back | Two-key footprints, conflict rate set by skew, retention |
| **F12. Local-first sync** | modeled (XYPH, F4) | Any Tier 1 scene | 2–10 runtimes diverge for m ticks | Suffix export and import with a declared overlap | Transport, import admission, conflicts at import |
| **F13. Counterfactual planner** | exploratory | Any scene | Fork 10–1,000 strands at one tick | Run each n ticks, settle one, discard the rest | Strand creation, divergence, settlement |

Notes on Tier 2:

- **F6 frame budget.** 16.67 ms is the application's whole frame at 60 Hz,
  not Echo's. F6 declares an Echo budget per run, and reports single-tick
  execution time and sustained 60 Hz service separately.
- **F6r rollback** stays exploratory and non-gating until late-input
  admission, branch isolation, handling of external effects, and settlement
  are specified. Until then it does not claim rollback-netcode coverage.
- **F9 and F10** measure graph operations with compiler and physics shapes.
  They do not claim to measure compilers or physics unless representative
  work of that kind is implemented.

### Tier 3: synthetic controls

Controls rarely vary literally one property: changing a tree's branching
factor also changes its depth. Each control therefore declares its
**manipulated parameter**, holds the rest at the star scene's defaults where
possible, and reports every property that changed as a result.

| Control | Manipulated | Values | Co-varying, reported |
| --- | --- | --- | --- |
| C1. Star | (baseline: reproduces today's `action_scene` exactly) | 1k–300k cells | — |
| C2. Chain | Depth | up to N | Fan-out falls to 1 |
| C3. Balanced tree | Branching factor | 2, 8, 64 | Depth |
| C4. Random DAG | Density | sparse, dense | Paths per node, depth |
| C5. Unreachable storage | Unreachable bytes, at fixed reachable size | 0×, 1×, 9× the reachable store | Total store size |
| C6. Nested warps | Nesting depth; child-warp count | depth 1–8; 1–10k warps | Nodes per warp |
| C7. Attachment size | Inline bytes | 8 B, 1 KB, 64 KB | Total payload bytes |
| C8. Hub degree | One node's edge count | (exists: `commit_hub_bucket`) | — |

**C5 and retention.** "Unreachable from the measured root" does not mean "safe
to reclaim": another worldline, a strand or a pinned reading may still need
that material. C5 names the root against which reachability is measured and
the retention obligations still in force, and holds reachable size fixed
while unreachable storage grows, so only one side of the experiment changes.

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

## Proposed sequence of changes

Each step builds and passes the existing checks on its own.

1. **Thin complete slice (landed; see [The first slice](#the-first-slice)).**
   Manifest format, the in-repository pseudo-random generator with test
   vectors, C1 reproducing today's `action_scene` exactly, outcome
   validation, the reporting contract, and a small F5 append-and-replay
   scenario built by admitted operations. The first correctness gate is C1
   and F5-S. Needs: nothing.
2. **Root and mutation coverage.** C6 with nested warps on both nodes and
   edges, C5 unreachable storage, C7 attachment sizes, then F3 and F6. Needs: 1.
3. **Consumer readings.** F1, F2 and F4 against concrete models, with
   bounded-reading behavior verified at and around the view limit. The
   bounded-reading harness is built and validated first. Needs: 1.
4. **Strands and transport.** Each harness is built and validated before the
   scenarios that use it: then F6r, F12 and F13. Needs: 1; F6r also needs 2.

The rest of the catalog is backlog, added one fixture per change.

## The first slice

This revision lands step 1. Everything below is implemented and checked by
`crates/warp-core/tests/scene_fixture_tests.rs`, which CI runs with
`native_rule_bootstrap,trusted_runtime`.

| Part | Where | What it establishes |
| --- | --- | --- |
| Generator | `benches/support/fixtures/prng.rs` | SplitMix64 output vectors, cross-checked against an independent implementation; the bounded-integer mapping pinned separately, including a bound above 2^63 |
| Manifests | `benches/support/fixtures/manifest.rs` | `workload_id` and `expectation_id` digested separately, with `edict.canonical-cbor/v1`; a test shows a golden change leaves `workload_id` unchanged |
| Result classes | `benches/support/fixtures/outcome.rs` | Valid, invalid, unsupported and incomplete; only valid results are comparable |
| C1 | `benches/support/fixtures/star.rs` | The star scene built independently of `action_scene`, with identical state roots at 1, 100 and 1,000 cells, and a pinned root |
| F5 | `benches/support/fixtures/think_append.rs` | Append and replay through the trusted host, validated |
| Runner | `benches/scene_fixtures.rs` | Repeated, classified rounds; a summary only when every round is valid |

### F5 in the first slice

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
  from its bucket; the last recorded root equals the live root; the pinned
  script digest matches.
- **Canaries.** The test feeds the validator a missing accepted operation, an
  incorrect historical reading, a failed replay, a duplicated authored
  effect, a missing entry, unfinished work, and a workload other than the
  pinned one. Each is classified invalid or incomplete and is never
  comparable.
- **Clocks.** `append` covers submit, admit and tick for every append, with
  envelopes built beforehand. `replay` covers the full patch replay.
  Validation, outcome collection and checkpoint readings are outside both. The
  runner is a normal build; it does not measure memory yet.

Gates after this slice: the **correctness gate** is the conformance test
above. The **performance gate** is not enabled; its environment, comparison
protocol and tolerance are not yet established.

## Later ideas

- **History twins (first among these).** Pairs with the same final state but
  different required history, such as direct assignment against `A → B → A`,
  or different causal provenance. Each pair names which observations must
  agree and which must differ. Two cases need different invariants:
    - **Distinct occurrences with identical payloads** must keep whatever
      distinction the occurrence contract requires.
    - **Redelivery of an already accepted occurrence** follows the retry and
      idempotency contract and must not create a second authored effect just
      because it arrived again. Delivery acknowledgements or retry
      diagnostics may differ without implying a new causal operation.
- **Validator canaries.** Feed the validator deliberately wrong small results:
  a missing accepted operation, an incorrect historical reading, a duplicated
  authored effect after redelivery. Each must become an invalid result and be
  excluded from comparisons. This tests the mechanism that protects every
  later benchmark claim.
- **Cost trajectories.** Measure across a scenario's lifetime: before churn,
  after churn, while readings are pinned, after pins are released, and after
  lawful reclamation. A single endpoint can hide a steadily worsening system.
- **Contract-preserving minimization.** Reduce a failing fixture while keeping
  the relevant witness (the causal dependency, nested warp, conflict or pinned
  reading), or, for a performance regression, a statistically repeatable
  slowdown.
- **Result capsules.** Package each published result with its manifest,
  implementation revision, expected and observed outcomes, raw samples and
  environment record, so that someone else can inspect and rerun it.

## Decisions on earlier open questions

These follow the external review's recommendations; the owner may override
them.

| Question | Decision |
| --- | --- |
| Crate location | Start within the existing bench and test support arrangement, which preserves the provider's `Cargo.lock` constraint. Extract a crate only when concrete sharing needs justify it, after checking the lockfile impact. |
| jedit model | Use jedit's actual operations and representation. Until they are settled, F4 is "structured editing" with jedit fidelity pending. |
| Rollback | F6r is exploratory and non-gating until its semantics are specified. |
| Think source | F5 proceeds as a census-derived workload and is compared with Think's implementation when available. |

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
- James Ross, *CΩMPUTER: The Geometry of All Possible Machines* (2025),
  prologue and chapters 1–3.
- The state-root timing study of 2026-10-17
  (`docs/benchmarks/state-root-timing-study.md`, local branch
  `docs/state-root-timing-study`).
- Two external reviews of earlier revisions of this document, supplied by the
  owner on 2026-10-17. The first cites the Rust Rand Book on reproducibility
  and Google Benchmark's documentation on random interleaving.
