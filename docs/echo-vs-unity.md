# Echo vs. Unity — Why Echo Is Fundamentally Different

> TL;DR: Unity is an object‑oriented, imperative frame loop over a mutable GameObject tree. Echo is a deterministic, typed graph‑rewriting engine (RMG) where state changes are explicit rewrite rules applied as atomic transactions along a fixed‑tick timeline. Echo treats time, branching, and merging as first‑class citizens.

## One‑Screen Summary

- Data Model
  - Unity: GameObject hierarchy with Components; behavior lives in scripts.
  - Echo: Recursive Meta Graph (RMG). Everything (entities, components, systems, events, proxies, contacts) is a typed graph node or edge.
- State Change
  - Unity: imperative mutation in `Update()`/`FixedUpdate()`; side effects are common.
  - Echo: Double‑Pushout deterministic rewrites (DPOi). Rules transform subgraphs under snapshot isolation; no hidden mutation.
- Scheduling
  - Unity: per‑frame script execution order (configurable, but often non‑deterministic in practice).
  - Echo: deterministic local scheduler orders rewrites by `(rule_id, scope_hash)`; disjoint scopes commute.
- Time (Chronos/Kairos/Aion)
  - Unity: `Time.deltaTime` and frame/update loops; time travel/branching are external.
  - Echo: fixed Chronos ticks; Kairos branches for “what‑ifs”; Aion tags to rank event significance; sub‑tick TOI for CCD.
- Persistence
  - Unity: scenes/prefabs + runtime state; save systems are custom.
  - Echo: content‑addressed snapshots; identical inputs ⇒ identical snapshot hashes; easy replay/diff.
- Networking
  - Unity: sync transforms/commands; engine/stack dependent; determinism optional.
  - Echo: Confluence replication — peers apply the same ordered rewrites and converge by construction.
- Tooling
  - Unity: Inspector reflects live object state; Profiler/Timeline optional.
  - Echo: Inspector reflects the live graph, rewrite stream, determinism hashes, and collision/CCD events.
- Determinism & Testing
  - Unity: cross‑platform determinism is hard; physics often divergent by platform.
  - Echo: determinism is a design constraint; math, scheduling, PRNG, and collision policies are deterministic and quantized.

## What “Everything Is a Graph” Means

- Typed Nodes: `Transform`, `Collider`, `TemporalProxy`, `PotentialPair`, `Contact`, `ContactEvent`, `System`, `Tick`, etc.
- Typed Edges: `has_component(entity→transform)`, `produced_in(x→tick)`, `pair_of(pair→a,b)`, `event_of(evt→contact)`.
- Rewrites: each frame phase is a set of DPO rules (BuildTemporalProxy, BroadPhasePairing, NarrowPhaseDiscrete/CCD, ContactEvents, GC). Rules are pure functions of host graph scope; they emit a new snapshot.
- Confluence: identical inputs + rule order ⇒ identical graph. Branch merges work because each branch does deterministic rewrites to the same canonical form.

## How “Move an Entity” Differs

- Unity (conceptual):
```csharp
// MonoBehaviour script
void Update() {
  transform.position += velocity * Time.deltaTime;
}
```
- Echo (conceptual):
```rust
// A rewrite rule with matcher + executor (see rmg-core demo motion rule)
// LHS: node with Motion payload; RHS: update position deterministically.
engine.register_rule(motion_rule());
let tx = engine.begin();
engine.apply(tx, "motion/update", &entity_id)?; // enqueues when LHS matches
let snap = engine.commit(tx)?;                  // atomic commit + snapshot hash
```
Key differences: Echo’s update is a named, scoped rewrite with auditability and a stable hash; the engine controls ordering and applies the rule atomically.

## Time as a First‑Class Concept

- Chronos: fixed‑tick clock; sub‑tick Time‑of‑Impact (TOI) for CCD is quantized to eliminate drift.
- Kairos: branching points (e.g., alternate inputs or CCD substeps) create speculative timelines; merges are deterministic.
- Aion: significance decides logging/detail budgets (e.g., bullets get CCD and higher precision; dust particles do not).

## Collision/CCD (Why It Fits Graph Rewrites)

- Broad phase adds `TemporalProxy` and `PotentialPair` nodes; narrow phase adds `Contact` and `Toi` nodes; events add `ContactEvent` nodes. All are created through deterministic rewrite rules with canonical IDs.
- See: `docs/spec-geom-collision.md` for the rules, IDs, and scheduler mapping.

## Persistence & Networking

- Snapshots are content‑addressed; deterministic rewrites guarantee convergent hashes.
- Replication: send ordered rewrite packets `{tx_id, rule_id, scope_hash, snapshot_hash}`; peers apply locally; no “state drift by replication schedule”.

## Authoring & Extensibility

- Replace ad‑hoc side effects with explicit rewrite rules.
- Ports/Adapters isolate non‑deterministic systems (render, input, OS clocks) from the core graph.
- Tools reason about intent (rules) rather than incidental effects.

## When (Not) to Use Echo

- Use Echo if you need: deterministic multiplayer, time‑travel debugging, massive branching simulations, reproducible simulations, precise merges.
- Use Unity if you need: integrated editor + full renderer stack today, rich asset pipelines, large plugin ecosystem, rapid prototyping without determinism constraints.
- Bridge: Echo can power headless sims and feed renderers/tooling via adapters.

## Migration Sketch

- Start by modeling gameplay state as graph nodes/edges.
- Port side‑effectful logic into rewrite rules with explicit match/replace.
- Introduce fixed Chronos ticks; route randomness through Echo’s PRNG.
- Adopt time‑aware collision: fat AABBs, deterministic pairs, quantized TOI.
- Add Confluence replication; verify identical snapshots across nodes.

## Appendix: Concept Mapping

| Unity Concept | Echo Equivalent |
| --- | --- |
| GameObject/Component | Typed nodes in RMG (entity + components as nodes/edges) |
| Scene | Snapshot (content‑addressed) + graph of assets |
| Update/FixedUpdate | Deterministic rule scheduler phases |
| Physics collision callbacks | Contact/ContactEvent nodes via rewrite rules |
| Prefab | Graph template; rewrite rules to instantiate variants |
| Undo/Redo | Timeline replay; branch/merge via snapshots |
| Netcode | Confluence replication of rewrites |

---

See also:
- `docs/architecture-outline.md` (core principles & loop)
- `docs/spec-rmg-core.md` (RMG, snapshots, confluence)
- `docs/spec-geom-collision.md` (time‑aware collision & CCD)
- `docs/phase1-geom-plan.md` (delivery milestones)

