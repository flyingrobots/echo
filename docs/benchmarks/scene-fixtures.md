<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Benchmark Scene Fixtures

Status: design, approved by the owner on 2026-10-17. Nothing here is
implemented yet. Written by Claude from the sources listed at the end.

This document defines a suite of scene fixtures: reproducible worlds, intent
streams and history operations that serve as baselines for Echo's benchmarking
work, now and later. Each fixture is modeled on a real use of Echo, or on a
shape that a real use would produce, so that a change measured against the
suite is measured against workloads Echo is meant to serve.

## Why a suite

Echo's benchmarks today run on one scene. `action_scene.rs` builds one warp: a
root node, N cell nodes with 7-byte attachments, and one edge from the root to
each cell. Its Actions only change attachments. `warp-benches` adds a linear
chain (`snapshot_hash`), a no-op scheduler drain, motion and materialization
benches, and a hub-degree case (`commit_hub_bucket`). No current bench has a
child warp, a deep tree, cross-links, large attachments, structural churn, a
strand, a replay, or a suffix exchange.

The state-root timing study (`docs/benchmarks/state-root-timing-study.md`,
on the local branch `docs/state-root-timing-study`) showed why that matters.

- The walk's cost per node rose from about 50 ns at 1,000 cells to about
  300 ns at 300,000.
- A widely cited figure, 102 ms at 100,000 cells, turned out to describe a
  contended machine. The quiet figure was 65–72 ms.

A number without its scene shape and its run conditions is not a baseline.

Echo's own framing raises the bar further. The README says causal history is
primary and "everything else is derived"; `docs/architecture/there-is-no-graph.md`
says the graph is "a coordinate chart over witnessed causal history". A suite
that only builds graphs and times their roots measures one reading and none of
the history. Each fixture here therefore has three parts.

## The shape of a fixture

```text
fixture = scene × intent stream × history operation
```

- **Scene**: the world before measurement: warps, nodes, edges, attachments.
- **Intent stream**: what is submitted per tick, by how many writer heads,
  with what footprints, contention and rejection rate.
- **History operation**: what is done with the resulting history: nothing
  (steady ticking), replay or restore, reads at earlier coordinates, strands
  forked and settled or discarded, or suffix export and import between runtimes.

The same scene can be measured under several intent streams and history
operations. A game world, for example, can be measured ticking, rolling back
or replaying.

## Properties every fixture declares

A fixture sets these deliberately and prints them next to its results.

| Property | Why it matters |
| --- | --- |
| Reachable size N | The walk and hash grow faster than N |
| Shape: fan-out, depth, cycles, nodes reached by many paths | Walk queue, revisits, edge filter |
| Warp nesting: count and depth, on nodes and on edges | The walk enters `Descend` on both; no current bench has one |
| Attachment bytes: inline, or a reference to content held outside the graph | Hash bytes against hash structure |
| Writes per tick: count, attachment-only or structural, local or scattered | Incremental-root designs treat attachment-only changes and structural changes differently |
| Contention: random overlap, or many writers piling onto a few targets | Conflicts and rejections |
| Churn and unreachable fraction | Deleted subgraphs leave the root but stay in the store |
| Readers: count, view size, pinned coordinate | Readings under concurrent writes |
| History: length, strands, suffix size | Log, replay and transport costs |

## Fixture catalog

Fixtures are grouped by priority. Tier 1 models the applications that already
plan to run on Echo, by their own records. Tier 2 adds distinct shapes from
other intended uses. Tier 3 holds synthetic controls that vary one property at
a time. Sizes are proposals; each fixture has S, M and L tiers so that one
binary can run quickly in CI and at full size on a quiet machine.

### Tier 1: Echo's first users

#### F1. XYPH planning

XYPH is an agentic planning substrate. Its README says it is moving toward
being a native Continuum participant, with work recorded as receipt-bearing,
capability-bound transitions.

- **Scene.** A rooted DAG, the "Genealogy of Intent": human-authorized intent
  roots → quests → acceptance criteria → evidence. Settlement records (Scrolls)
  are attached to finished quests. Provenance chains are long; every quest
  traces to an intent. Tiers: 1k / 10k / 100k quests.
- **Intent stream.** Agents ask for the next quest and claim it. A claim is
  exclusive, so many agents pile onto the few quests that are ready at once.
  Capability-bound actions declare their reads, writes and spend before they
  run.
- **History.** Speculative worldlines settled into `worldline:live`; offline
  participants that diverge and exchange suffixes.
- **Reads.** Read-dominated. `briefing`, `next` and `context` compute derived
  judgments (`nextLawfulActions`, `blockingReasons`) over bounded views.
- **Stresses.** Writers piling onto the same targets, read-heavy bounded
  views, provenance depth, settlement.

#### F2. Dojo claims and JIT context

Dojo's ADR-0003 makes Echo its causal runtime.

- **Scene.** A forest of contextual claim terms: frames wrap claims, which can
  be grouped, offered as alternatives, negated or guarded, to depth 3–10. Each
  term points at a few hub entities, such as people. Raw transcripts stay
  outside the graph; the graph holds references to their content.
- **Intent stream.** Batches of claims written as artifacts are processed.
- **History and reads.** Many short-lived agents, each given a bounded view
  (the JIT context payload) at a pinned coordinate while writes continue.
  Interactive latency is the budget.
- **Stresses.** Nested small trees with hubs, payloads by reference, many
  concurrent bounded readers.

#### F3. Graft: code over time

Modeled on Graft's indexer (`src/warp/index-head.ts`).

- **Scene.** A directory tree 5–10 levels deep → files → 10–100 symbols per
  file, with one 1–100 KB AST payload per file. Cross-file `references` and
  `calls` edges form cycles through mutual imports. Commit nodes touch files.
  Variant F3b puts each file's AST in a child warp of 1k–10k nodes.
- **Intent stream.** Per commit, one file's subtree is replaced: its symbols
  deleted and re-added, in batches of at most 64 files. Every file in a commit
  also writes the shared `dir:` and `commit:` nodes.
- **History.** Long; reads of a file's history at earlier commits.
- **Stresses.** Structural subtree replacement, large payloads, hot shared
  nodes, deep trees, nested warps (F3b).

#### F4. jedit: collaborative structured editing

`docs/NORTHSTAR.md` names jedit as the immediate product pressure. Its
app-facing surface is `optic.applyIntent(...)` and `optic.textWindow(...)`.

- **Scene.** A document tree (sections → paragraphs → runs) with ordered
  siblings.
- **Intent stream.** Several writers editing near the same place; speculative
  edits as strands.
- **Reads.** `textWindow` readings over the region each writer is viewing.
- **History.** Offline edits merged by suffix import.
- **Stresses.** Ordered-list edits, local contention, windowed readings,
  strands.

#### F5. Think: thought capture

Modeled on the census of a real Think store in git-warp's
`docs/READINGS_AND_OPTICS.md`, and the append shape it prescribes.

- **Scene.** A few bucket nodes (day or topic) with a wide star of entries
  under each; tiny payloads; sparse cross-links between entries.
- **Intent stream.** One append per tick, with an empty read set.
- **History.** Long: 100k+ ticks. Operation: full replay.
- **Regression variant.** One page node rewritten on every append, the
  anti-pattern the census found, whose patches grew quadratically.
- **Stresses.** Log growth, replay, wide buckets.

### Tier 2: other intended uses with distinct shapes

| Fixture | Scene | Intent stream | History operation | Stresses |
| --- | --- | --- | --- | --- |
| **F6. Real-time game** | Regions → entities → component nodes (32–64 B); an NPC behavior tree in a child warp each; short-lived collision edges; 1k / 10k / 100k entities | 10–30% of entities move (attachment-only); spawns, despawns, region moves | Steady 60 Hz ticking; replay | Many writes per tick, attachment-only path, many small warps, 16.67 ms frame |
| **F6r. Rollback** | F6 | F6 plus late inputs | Re-simulate the last k ticks in a strand per late input | Strand fork and settle per frame |
| **F7. Web server** | Large cold user and session store; static routing tree; a short-lived subgraph per request | Many writer heads; hot keys on a Zipf distribution | Steady; create/delete churn | Churn, unreachable fraction, hot keys, inbox budgets |
| **F8. Build pipeline** | DAG with heavy fan-in and long chains | A source changes; invalidation spreads along reverse edges | Provenance reads at earlier coordinates | Reverse index, cascading writes, nodes reached by many paths |
| **F9. Compiler: edges that contain graphs** | Module → functions, each with a control-flow child warp → blocks → instruction trees; transformation passes as `Descend` on edges | A pass rewrites one function's interior | Steady | Deep nesting, `Descend` on edges, thousands of small warps |
| **F10. Physics constraints** | Bodies and joints; dense and cyclic (ragdolls, chains, stacks) | The solver touches each body's neighborhood | Steady | Cycles, naturally high footprint overlap |
| **F11. Ledger / contract host** | Accounts in a star; receipts as an append-only log | Two-account transfers with hot accounts on a Zipf distribution | Audit reads far back | Two-key footprints, conflict rate set by skew, retention |
| **F12. Local-first sync** | Any Tier 1 scene | 2–10 runtimes diverge for m ticks | Suffix export and import with a set conflict rate | Transport, import admission, conflicts at import |
| **F13. Counterfactual planner** | Any scene | Fork 10–1,000 strands at one tick | Run each n ticks, settle one, discard the rest | Strand creation, divergence, settlement |

### Tier 3: synthetic controls

Each control varies one property and holds the rest at the star scene's
defaults. They let a regression in a use-case fixture be traced to its cause.

| Control | Varied | Values |
| --- | --- | --- |
| C1. Star | (baseline: today's `action_scene`) | 1k–300k cells |
| C2. Chain | Depth | up to N |
| C3. Balanced tree | Branching factor | 2, 8, 64 |
| C4. Random DAG | Density, nodes reached by many paths | sparse, dense |
| C5. Unreachable fraction | Leftovers in the store | 0%, 50%, 90% |
| C6. Nested warps | Nesting depth, child-warp count | depth 1–8; 1–10k warps |
| C7. Attachment size | Inline bytes | 8 B, 1 KB, 64 KB |
| C8. Hub degree | One node's edge count | (exists: `commit_hub_bucket`) |

## What every run reports

Each run reports these as separate lines, not one total:

- admission and scheduling;
- commit;
- state root, split into walk and hash;
- receipts;
- write-ahead log;
- replay or restore;
- readings (with view size);
- strand and transport operations, where used.

Alongside them, each run reports the fixture's declared properties (above) and
its run conditions:

- commit, toolchain and build profile;
- environment (native or VM, with CPU and memory limits);
- what else was running on the machine;
- a quiet baseline taken in the same session.

The timing study showed that load average alone does not reveal memory-bandwidth
contention. Concurrent memory-heavy work raised a 66 ms root to 93–119 ms
while the load average looked ordinary.

## Implementation principles

- **Generated, not stored.** Each fixture is a deterministic generator taking
  a seed and a size tier, and producing the scene and the intent stream.
  Fixtures are rebuilt from code; no fixture data is checked in.
- **Self-describing.** A generator prints its realized statistics: N, depth,
  warp count, attachment bytes, writes per tick, unreachable fraction.
- **Checked once, then trusted.** Each generator has a test that pins its
  statistics and its initial state root for each tier and seed. A change to a
  generator is a visible test change, not a silent baseline shift.
- **Shared by benches and tests.** The same generators can feed determinism
  tests, so a benchmark scene is also a correctness scene.
- **No probe in the deterministic path.** Timing splits such as walk versus
  hash use diagnostic patches or feature-gated instrumentation, never
  `Instant` or `eprintln!` in shipped code.

## Proposed sequence of changes

Each step builds and passes the existing checks on its own. Dependencies are
stated as "needs".

1. **Fixture framework, plus controls C1–C7.** Generator trait, seed and tier
   handling, statistics output, pinned-statistics tests. C1 must reproduce
   today's `action_scene` exactly, so existing results stay comparable.
   Needs: nothing.
2. **F3 Graft and F6 game.** Together they cover structural churn,
   attachment-only churn, nested warps and large payloads, the largest gaps.
   Needs: 1.
3. **Run reporting.** The per-cost report lines and the run-conditions record.
   Needs: 1. Independent of 2.
4. **F1 XYPH and F2 Dojo.** Needs 1 for the scenes. Reader-heavy measurement
   needs a bounded-reading harness; that harness is part of this step.
5. **History operations: replay and reads at earlier coordinates.** Then
   F5 Think and F11 ledger, which depend on them. Needs: 1.
6. **Strands: F6r rollback and F13 planner.** Needs: 1, and 2 for F6r.
7. **Transport: F12 local-first sync, then F4 jedit's merge path.** Needs: 1.
8. **Remaining Tier 2: F7, F8, F9, F10.** One fixture per change. Needs: 1.

## Open questions

- **Crate location.** The trusted-host benches live in `warp-core` because
  their dependencies would change `Cargo.lock`, which the provider package
  binds. Should fixtures live there too, or in a new crate with that
  constraint handled separately?
- **jedit's document model.** F4 should copy jedit's model rather than invent
  one. Is there a concrete one to follow yet?
- **Rollback.** Is rollback netcode (F6r) an intended game use, or is replay
  the extent of it?
- **Think's source.** F5 is based on git-warp's census, not on Think's code,
  which was not available when this was written.

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
- git-warp: `docs/READINGS_AND_OPTICS.md`, the census of a real Think store.
- XYPH: the README on GitHub (`flyingrobots/xyph`).
- Dojo: a documentation snapshot of 2026-10-09, including ADR-0003, the JIT
  Context Builder design and the Contextual Claims specification.
- James Ross, *CΩMPUTER: The Geometry of All Possible Machines* (2025),
  prologue and chapters 1–3: build pipelines, physics constraint graphs,
  behavior trees, compilers, and edges that contain graphs.
- The state-root timing study of 2026-10-17
  (`docs/benchmarks/state-root-timing-study.md`, local branch
  `docs/state-root-timing-study`).
