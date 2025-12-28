<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# WARP Runtime Architecture (Phase 1 Blueprint)

This document captures the consensus that emerged for Echo’s Phase 1 implementation: the entire runtime, assets, and tooling operate on top of the WARP graph engine. Every concept—worlds, systems, entities, components, assets, pipelines—is a graph node. The engine executes deterministic DPO rewrite rules over that graph each tick, emitting snapshots for replay, networking, and tooling.

---

## Everything Is a Graph

- `World`: graph node whose edges point to `System` subgraphs.
- `System`: rewrite rule graph. Pattern `L`, interface `K`, output `R`.
- `Entity`: graph node with edges to `Component` nodes (`Has` edges).
- `Component`: leaf node with payload (POD data, asset reference, etc.).
- `Timeline`: sequence of rewrite transactions / snapshots.
- `Asset`: graph nodes that hold binary payloads (meshes, shaders).
- `Importer/Exporter`: graph describing pipelines—each step is a node with rewrite rule.

---

## Tick Loop (Deterministic Scheduler)

> **Note**: This is the target Phase 1 API design. The current `warp-core` crate
> is a bootstrap skeleton; consult `crates/warp-core/src/lib.rs` for the working
> interfaces.

```rust
loop {
    let tx = engine.begin();

    let rewrites = scheduler.collect(world_root, &engine);
    for rewrite in rewrites {
        engine.apply(tx, rewrite.rule, &rewrite.scope, &rewrite.params)?;
    }

    let snapshot = engine.commit(tx)?;
    publish_inspector_frames(snapshot);
    process_delayed_events(snapshot);
}
```

- Scheduler walks the graph, gathers rewrite intents, orders by `(scope_hash, rule_id)`.
- Disjoint scopes execute in parallel under the DPOi scheduler.
- Commit produces a `Snapshot` hash captured in the branch tree and Confluence.

---

## Execution Walkthrough

1. **Begin transaction** – `engine.begin()` returns `TxId`.
2. **Collect rewrites** – scheduler matches system patterns, computes scope hashes.
3. **Apply rules** – each rule operates on matched subgraph, updating payloads / edges.
4. **Commit** – atomic swap of graph store, emit snapshot + commit log entry.
5. **Emit frames** – inspector, entropy, Codex logs read from snapshot.

---

## Branching & Replay

- Forking = capturing snapshot hash and starting new rewrite sequence.
- Rollback = load prior snapshot, replay commits.
- Merge = deterministic three-way merge via Confluence rules.

---

## Tools & Networking

- Tooling (Echo Studio, inspector) consumes snapshots and rewrite logs.
- Networking exchanges rewrite transactions (scope hash, rule id, params hash).
- Deterministic merge ensures peers converge on identical snapshots.

---

## Implementation Notes

- WARP engine runs in Rust (`warp-core`).
- Rhai scripts issue rewrite intents via bindings; remain deterministic.
- TypeScript tools (via WASM) visualize the same graphs.

---

This loop—the recursive execution of graph rewrite rules—is the heart of Echo’s deterministic multiverse runtime.
