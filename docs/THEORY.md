<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo: Theoretical Foundations

This document paraphrases the AIΩN Foundations Series formal mathematical papers underlying Echo's architecture. These theories establish the rigorous foundation upon which the implementation is built.

---

## Paper I: WARP Graphs (Worldline Algebra for Recursive Provenance)

### Source

[![DOI](https://zenodo.org/badge/DOI/10.5281/zenodo.17908005.svg)](https://doi.org/10.5281/zenodo.17908005)

```text
Ross, J. (2025). WARP Graphs: A Worldline Algebra for Recursive Provenance. Zenodo. https://doi.org/10.5281/zenodo.17908005
```

### The Problem: Graphs All The Way Down

Complex software doesn't live in a single flat graph. Real systems are **graphs of graphs of graphs**:

- A compiler juggles syntax trees (graphs), control-flow graphs, and optimization traces
- A database tracks schemas (graphs), query plans (graphs), and execution traces (graphs)
- An AI agent carries a world model (graph), internal goals (graph), and interaction history (graph)

The usual mathematical tools (directed graphs, hypergraphs) are excellent for flat structure but awkward for nested structure. Every project invents its own ad-hoc "graph with attached subgraphs" convention, making it hard to:

- Transport results between systems
- State semantics that talk about the whole stack at once
- Prove properties about nested structures

### The Solution: WARP Graphs

A **WARP graph** (plural: WARPs) is a minimal canonical object for nested graph structure. It has two layers:

1. A **skeleton** - a finite directed multigraph describing the coarse shape
2. **Attachments** - each vertex and edge carries its own WARP graph as payload

> [!note]
> Echo does this differently. In `warp-core`, attachments are **typed atoms** by default (`AtomPayload { type_id, bytes }`) and recursion is represented as explicit, skeleton-visible indirection (`AttachmentValue::Descend(WarpId)`), not as “a whole WARP graph stored inside bytes.” It works like flattened indirection because the rewrite hot path stays skeleton-only (fast and deterministic) while descended structure remains explicit and slice-safe via `WarpState` / `WarpInstance`.

This nesting is **finite and well-founded** - you can't have infinite attachment chains.

> [!note]
> Echo does this differently. The implementation enforces “finite descent” operationally: descended attachments form a parented instance tree (each non-root `WarpInstance` has exactly one `parent` attachment slot), and tick patch replay rejects **dangling portals** and **orphan instances**. It works this way because well-founded recursion needs to be enforceable at the boundary artifact (patch replay), not dependent on recursively decoding arbitrary payload bytes.

### Formal Definition

Fix a set **P** of **atomic payloads** (literals, external IDs, opaque data - the stuff we don't model internally).

The class **WARP** is the **least class** closed under two constructors:

1. **Atoms**: For each `p ∈ P`, there is an atom `Atom(p) ∈ WARP`
2. **Composite**: If `S = (V, E, source, target)` is a finite directed multigraph, and `α: V → WARP` and `β: E → WARP` assign WARPs to vertices and edges, then `(S, α, β) ∈ WARP`

**Translation:** Every WARP is either a bare atom OR a skeleton graph whose vertices/edges carry smaller WARPs.

### Example: Call Graph with Nested Syntax and Provenance

Consider a program with functions `f` and `g` and a single call `f → g`.

**Skeleton S:**

- Vertices: `{v_f, v_g}`
- Edges: `{e_call: v_f → v_g}`

**Attachments:**

- `α(v_f)` = abstract syntax tree of function `f` (itself a WARP)
- `α(v_g)` = abstract syntax tree of function `g` (itself a WARP)
- `β(e_call)` = provenance graph recording optimization choices (itself a WARP)

Each of these attachments can itself have attachments (e.g., a syntax tree node might store profiling data as a nested WARP). **In one object, the high-level call graph and all nested payloads stay coherent.**

### Initial Algebra Formulation

WARPs can be characterized as the **initial algebra** for a polynomial functor:

```math
F(X) = P + \Sigma_{S \in Graphs} (V_S \to X) \times (E_S \to X)
```

This means: to define a function out of WARPs, it suffices to say:

1. How it acts on atoms
2. Given a skeleton S and recursively computed results for all attachments, how to combine them

The result is then **unique**. This gives us structural recursion and induction "for free."

### Depth and Unfoldings

**Depth** of a WARP X:

- Atoms have depth 0
- A composite WARP `(S, α, β)` has depth = 1 + max depth of all attachments

**k-unfolding** `unf_k(X)`:

- Keep all structure at depths 0, ..., k-1 unchanged
- Replace every attachment at depth ≥ k with a placeholder atom

This gives finite-depth approximations of arbitrarily deep WARPs. The **infinite unfolding** `unf_∞(X)` is the colimit of the tower:

```math
unf_0(X) \to unf_1(X) \to unf_2(X) \to ...
```

### Category of WARPs

A **WARP morphism** `f: X → Y` consists of:

1. A graph homomorphism of skeletons `(f_V, f_E)`
2. For every vertex `v`, a morphism of attachments `f_v: α(v) → α'(f_V(v))`
3. For every edge `e`, a morphism of attachments `f_e: β(e) → β'(f_E(e))`

WARPs and their morphisms form a category **𝐖𝐀𝐑𝐏**.

There's a **forgetful functor** `π: 𝐖𝐀𝐑𝐏 → Graph` that forgets attachments and returns just the skeleton.

> [!note]
> Echo does this differently. Echo does not currently expose “WARP morphisms” as a first-class runtime API; instead it treats _identity + hashing + replayable deltas_ as the practical boundary. It works like this because the engine’s guarantees (deterministic replay, patch hashing, slicing) need stable, content-addressed artifacts, while categorical structure can be layered later as tooling/analysis atop the same boundary.

### Relation to Ordinary Graphs

**Ordinary graphs embed into WARPs:**

- Any finite directed multigraph S can be viewed as a shallow WARP by attaching a constant placeholder atom to every vertex and edge
- This is a fully faithful embedding of `Graph → 𝐖𝐀𝐑𝐏` as the subcategory of depth-1 objects

**Hypergraphs embed via typed open graphs:**

- Typed open graphs (category 𝐎𝐆𝐫𝐚𝐩𝐡_T) are cospans `I ↪ G ↩ O`
- This category is **adhesive** (supports DPO rewriting)
- WARPs whose skeletons are typed open graphs are "recursive typed open graphs"
- Double-Pushout (DPO) rewriting lifts from skeletons to full WARP states

> [!note]
> Echo does this differently. `warp-core` does **not** implement categorical DPO/DPOI rewriting yet; rules are currently expressed as deterministic matcher/executor functions plus conservative read/write `Footprint`s. It works like this because the core requirement is determinism + independence checking; DPOI is the mathematical north star, but the implementation keeps the hot path simple while still enforcing the “no hidden edges” and two-plane invariants.

**Key Point:** WARPs **subsume** ordinary graphs and hypergraphs while adding nested structure. Any model expressible in the usual DPO setting can be expressed as a shallow WARP; models that genuinely need nesting get additional power with no change to the underlying machinery.

### Why This Matters for Echo

WARPs are the **canonical state space** for Echo's execution model. They provide:

1. **Nested structure** - syntax, control flow, provenance, traces unified in one object
2. **Well-founded recursion** - can't have circular attachments
3. **Categorical properties** - morphisms, initial algebra, structural induction
4. **Adhesive-friendly** - compatible with DPO graph rewriting
5. **Extensible** - ordinary graphs are just shallow WARPs

Later papers in the AIΩN Foundations series build on this substrate to define:

- Deterministic multiway DPO rewriting on WARPs
- Holographic provenance (boundary encodes interior evolution)
- Observer geometry (rulial distance) over WARP universes

> [!note]
> Echo does this differently. Today, Echo’s runtime intentionally implements the **skeleton projection** `π(U)` as the hot path plus depth-0 typed atoms (and Stage B1 descended instances via explicit portals). It works like this because a game-engine runtime needs predictable performance: anything that must affect matching/scheduling/slicing must be explicit skeleton structure, not an arbitrarily deep recursive object that requires decoding during rewrites.

**WARPs are not a feature. They are the axle everything else rotates on.**

---

## Paper II: Canonical State Evolution and Deterministic Worldlines

### Source (Paper II)

[![DOI](https://zenodo.org/badge/DOI/10.5281/zenodo.17934512.svg)](https://doi.org/10.5281/zenodo.17934512)

```text
Ross, J. (2025). WARP Graphs: Canonical State Evolution and Deterministic Worldlines (v1.0.0). Zenodo. https://doi.org/10.5281/zenodo.17934512
```

### The Problem: Concurrency Without Chaos

Paper I gave us the state object (WARPs). Now we need **dynamics** - how do WARPs evolve?

Real systems are concurrent. Multiple things happen "at once" - not in a strict total order, but in a partial order constrained by causality. Left unmanaged, this creates chaos:

- Replay depends on accidental interleavings
- State evolution is machine-specific (depends on which thread won the race)
- Debugging becomes impossible because you can't reproduce the same execution twice

For Echo, **replay is not a debugging feature - it's part of the semantic contract.** We need concurrency that is:

1. **Deterministic** - same input + same policy → identical output (every time)
2. **Compositional** - local work (inside attachments) commutes with global wiring changes
3. **Provenance-ready** - the scheduler's choices are recorded, not hidden

### The Solution: Two-Plane Operational Semantics

WARP states are **two-plane objects**:

1. **Skeleton plane** - the typed open graph `G` that describes coarse wiring and interfaces
2. **Attachment plane** - the nested WARPs `(α, β)` sitting over each vertex and edge

> [!note]
> Echo does this differently. Echo’s “two-plane” split is implemented as **SkeletonGraph + Attachment Plane** where the skeleton is a deterministic `GraphStore` and attachments are typed atoms (plus explicit `Descend(WarpId)` portals). It works like this because the engine needs the skeleton to be rewrite-visible and hash-committed, while attachments remain opaque unless a rule opts in to decoding.

Evolution happens on **both planes**:

- **Attachment-plane steps** rewrite nested state inside `α(v)` or `β(e)` without changing the skeleton
- **Skeleton-plane steps** rewrite the global wiring `G` and transport attachments along preserved structure

The unit of evolution is a **tick** - an atomic commit of:

1. A finite family of attachment-plane updates
2. A scheduler-selected **batch** of independent skeleton rewrites

### DPOI Rewriting

Rewriting uses **Double-Pushout with Interfaces (DPOI)** - a categorical formalism from algebraic graph transformation.

A **DPOI rule** is a span of monomorphisms:

```math
L \xleftarrow{\ell} K \xrightarrow{r} R
```

Where:

- `L` = left-hand side (what to match)
- `K` = interface (what to preserve)
- `R` = right-hand side (what to replace it with)

A **match** `m: L ↪ G` finds an occurrence of `L` in the host graph `G`. A **DPOI step** `G ⇒ H` deletes `L \ K` (the non-preserved part), then glues in `R` along `K`.

This is standard categorical rewriting - the key insight is how we use it on **two planes at once**.

> [!note]
> Echo does this differently. `warp-core` does not yet implement DPOI rules/matches as explicit categorical objects; instead rules compute a conservative `Footprint` (read/write sets over nodes/edges/attachments/ports) and the scheduler selects a conflict-free subset. It works like DPOI independence because the key operational requirement is “commute when independent,” and conservative footprints give a deterministic, implementable approximation without requiring a full DPOI match engine in the hot loop.

### Ticks: Atomic Units of Concurrency

A **tick** groups concurrent work into an atomic commit:

```math
U = (G; \alpha, \beta)  \Rightarrow[Tick]  U' = (G'; \alpha', \beta')
```

**Inside a tick:**

1. **Attachment updates** settle local state inside attachments
2. **Skeleton publication** commits a batch `B` of independent skeleton rewrites

**Atomicity:** A tick either fully commits or leaves the state unchanged (no partial effects observable).

**Key property:** The tick outcome is **deterministic** - independent of how the internal steps are serialized.

### Independence and Scheduler-Admissible Batches

Two skeleton matches are **independent** if neither deletes structure that the other uses.

For each match `m: L ↪ G` with interface `K ⊆ L`, define:

- **Delete set** `Del(m)` = the part of `L` not preserved by `K`
- **Use set** `Use(m)` = the entire match `m(L)`

Matches `m₁` and `m₂` are **independent** if:

```math
Del(m_1) \cap Use(m_2) = \emptyset  AND  Del(m_2) \cap Use(m_1) = \emptyset
```

**Translation:** Neither deletes structure that the other reads.

A **scheduler-admissible batch** `B` is a finite set of pairwise independent matches. These can be executed in **any order** without changing the result.

> [!note]
> Echo does this differently. Independence is computed over `Footprint`s, not over `Del/Use` sets derived from a DPOI match object. It works like this because the engine can remain agnostic to the matcher implementation: as long as footprints are conservative (over-approx reads/writes), accepted rewrites are guaranteed to be independent and therefore commute.

### Tick-Level Confluence Theorem

**Main Result (Skeleton Plane):** Given a scheduler-admissible batch `B`, any two serializations of the rewrites in `B` yield **isomorphic successor skeletons**.

**Proof sketch:** The DPO parallel-independence theorem says independent steps commute. Any two serializations differ by swapping adjacent independent steps, so they yield the same result (up to isomorphism).

**Consequence:** Once the scheduler picks `B`, the tick outcome is **unique** (up to isomorphism), independent of the internal execution order.

### Deterministic Scheduling and Tick Receipts

Tick confluence says: "given `B`, the outcome is deterministic." But how is `B` chosen?

A **deterministic scheduler** is a total function:

```math
\sigma: WState \to Batch
```

One canonical choice: **left-most greedy filter**

1. Sort all candidate matches `Cand(U)` by a total order (e.g., lexicographic on stable IDs)
2. Walk the list, accepting each match if it's independent of all previously accepted matches
3. The result `B` is scheduler-admissible by construction

A **tick receipt** records what happened:

```math
\rho = (E, \preceq, E_acc, E_rej, meta)
```

Where:

- `E ⊆ Cand(U)` = candidates considered
- `E_acc ⊆ E` = accepted matches (the batch)
- `E_rej = E \ E_acc` = rejected matches
- `(E, ≼)` = tick-event poset (partial order recording "which event blocked which")
- `meta` = optional metadata (stable IDs, rule names, rejection reasons)

**Key insight:** The receipt refines the tick without changing the committed state. It's **provenance**, not semantics.

For the left-most scheduler:

- When match `mᵢ` is rejected because it overlaps an already-accepted match `mⱼ` (where `j < i`), record `mⱼ ≺ mᵢ` in the poset
- Accepted matches are unordered (they're independent)
- Rejected matches are causally after the event that blocked them

This poset is the bridge to Paper III (provenance).

> [!note]
> Echo does this differently. Echo records receipts as deterministic diagnostics (`TickReceipt`) but does **not** treat them as consensus reality for commit identity: commit hash v2 commits to the replayable delta (`patch_digest`), while receipts remain optional explanation. It works like this because replay/slicing require a stable boundary artifact (“what happened”), and explanations (“why”) can evolve without changing the committed transformation.

### No-Delete/No-Clone Under Descent

The two planes can only commute if skeleton publication respects attachment lineage.

**Invariant:** A tick satisfies **no-delete/no-clone-under-descent** if:

1. **No delete under descent:** Any skeleton position `x` with `depth(x) ≥ 1` (has nontrivial attached structure) cannot be deleted
2. **No clone under descent:** Any skeleton position `x` with `depth(x) ≥ 1` has a unique preserved image in the successor (so attachment transport is single-valued)

**Translation:** You can't destroy or duplicate attachment lineage during skeleton publication.

> [!note]
> Echo does this differently. Echo’s descended structure is represented by explicit portals (`AttachmentValue::Descend`) plus `WarpInstance.parent` metadata, not by “attachments with depth ≥ 1” living inside a node/edge. It works like this because lineage constraints are enforced at patch replay: clearing a portal or deleting an instance must satisfy invariants (no dangling portals / no orphan instances), rather than globally forbidding deletes of any node that happens to have payload bytes.

### Two-Plane Commutation Theorem

**Main Result (Two Planes):** Let `U = (G; α, β)` be a WARP state.

Let:

- `A` be an attachment-plane step: `(G; α, β) ⇒ (G; α_A, β_A)`
- `S` be a skeleton publication step that commits batch `B` on `G`, yielding `G'` and transported attachments `(α', β')`

Assume the tick satisfies no-delete/no-clone-under-descent.

Then there exists an attachment-plane step `A'` over `G'` such that:

```text
(G; α, β) ─A→ (G; α_A, β_A)
    │              │
    S│              │S_A
    ↓              ↓
(G'; α', β') ─A'→ (G'; α'', β'')
```

This square **commutes up to canonical isomorphism**.

**Proof sketch:** Attachment updates act inside fibers (they don't touch the skeleton). Skeleton publication transports attachments via a chosen reindexing functor `τ` (a "cleavage" of the projection functor `π: WState → OGraph_T`). Since transport is functorial and no-delete/no-clone guarantees well-defined single-valued transport, the two orderings yield the same result.

**Consequence:** "Attachments-then-skeleton" is equivalent to "skeleton-then-transported-attachments." The operational discipline (do local work first, then publish) is just one valid linearization - the semantics doesn't care.

### Worldlines and Provenance

Given a deterministic scheduler `σ` and a deterministic policy for attachment updates, a run produces a canonical **worldline**:

```math
U_0 \Rightarrow[Tick_1, \rho_1] U_1 \Rightarrow[Tick_2, \rho_2] U_2 \Rightarrow[Tick_3, \rho_3] ...
```

Each `ρᵢ` is a tick receipt recording the scheduler's choices. The global history is linear (ℕ-indexed), but each tick carries internal partial-order structure (the tick-event poset).

> [!note]
> Echo does this differently. `warp-core` operates on a linear sequence of ticks inside an `Engine`, but the full Echo timeline model is intended to be a **DAG** (branch/merge) at a higher layer. It works like this because Paper II’s worldline results are easiest to implement and test on linear histories first; merge semantics and DAG slicing are explicitly specced for later layers without complicating the core tick boundary.

Paper III uses these receipts as first-class provenance payloads.

### Why This Matters for Echo (Paper II)

Paper II provides the **deterministic execution model**:

1. **Concurrency is semantic, not accidental** - independence is defined by footprints, not thread scheduling
2. **Replay is guaranteed** - same state + same policy → identical successor (every time, on every machine)
3. **Provenance is built-in** - tick receipts record scheduler decisions without changing committed state
4. **Two planes commute** - local work and global wiring changes can be reordered without breaking semantics
5. **Ticks are atomic** - no partial effects, clean transaction semantics

This is the foundation for:

- Deterministic replay (required for time-travel debugging)
- Counterfactual branching (swap scheduler policy → explore alternative worldline)
- Provenance traces (Paper III chains tick receipts into holographic boundary)

**Paper I gave us the state space. Paper II gave us the deterministic dynamics. Together, they make deterministic multiway evolution possible.**

---

## Paper III: Computational Holography & Provenance Payloads

### Source (Paper III)

[![DOI](https://zenodo.org/badge/DOI/10.5281/zenodo.17963669.svg)](https://doi.org/10.5281/zenodo.17963669)

```text
Ross, J. (2025). WARP Graphs: Computational Holography & Provenance Payloads. Zenodo. https://doi.org/10.5281/zenodo.17963669
```

**Source:** "WARP Graphs: Computational Holography & Provenance Payloads" by James Ross, December 2025

### The Problem: Logs Are Not Enough

Papers I and II gave us deterministic execution. Now we need **provenance** - the ability to answer:

- "How did this value get computed?"
- "What inputs were needed to produce this output?"
- "Can I verify this result without re-running the entire computation?"

The naive approach is to **log everything** - every intermediate state, every decision, every match. This works but:

1. **Storage explodes** - GB of logs for MB of actual computation
2. **Verification is expensive** - you have to replay everything to check one value
3. **Logs are fragile** - they're often append-only blobs, hard to slice or branch

For Echo, provenance is not "nice to have" - it's **structural**. We need a compact, verifiable, sliceable representation of the full derivation history.

### The Solution: Boundary Encodings and Computational Holography

**Key insight:** For a deterministic computation, the **full interior volume is recoverable from a compact boundary representation**.

The boundary is:

```math
B = (U_0, P)
```

Where:

- `U₀` = initial state
- `P = (μ₀, μ₁, ..., μₙ₋₁)` = provenance payload (ordered sequence of **tick patches**)

A **tick patch** `μᵢ` is the minimal record needed to deterministically advance `Uᵢ → Uᵢ₊₁`. It's a "Git-like patch" for WARP states.

**Computational holography** is the theorem that says: given `(U₀, P)`, you can uniquely reconstruct the entire worldline `U₀ ⇒ U₁ ⇒ ... ⇒ Uₙ`.

The interior (bulk) is encoded by the boundary.

### Tick Patches: What Gets Recorded

A tick patch `μ` must be **sufficient** for deterministic replay. At minimum, it contains:

1. **Rule-pack/policy identifiers** - which rules and scheduler policy were used
2. **Accepted match keys** - content-addressed descriptions of accepted matches (not full re-search)
3. **Attachment deltas** - exact attachment updates (or a deterministic recipe)
4. **Commit flag** - success/abort indicator
5. **Optional trace** - the tick-event poset `ρ` from Paper II (for explanatory audit)

**Patch vs Receipt:**

- **Patch (prescriptive)** - minimal witness for replay: "what happened"
- **Receipt (descriptive)** - full causal explanation: "why it happened that way"

A patch may contain an embedded receipt when full audit is needed, but holography only requires the patch to be **sufficient** for deterministic Apply.

> [!note]
> Echo does this differently. Echo uses **delta-first** patches: `WarpTickPatchV1` records canonical deterministic ops (“these edits happened”) plus conservative `in_slots`/`out_slots` for slicing; it does not store “accepted match keys” as the replay contract. It works like this because replay should not depend on rule-engine semantics forever: applying a delta patch is stable across languages and future refactors, while match keys and receipts can still be stored as optional narrative.

### The Apply Function

There's a deterministic partial function:

```math
Apply: WState \times Labels \rightharpoonup WState
```

Where `Labels` is the space of tick patches. Given a state `Uᵢ` and patch `μᵢ`, Apply produces the next state:

```math
U_{i+1} = Apply(U_i, \mu_i)
```

**Key property:** For patch-deterministic worldlines, `(Uᵢ, μᵢ)` **uniquely determines** `Uᵢ₊₁` (whenever Apply is defined).

This is the interface that makes holography work.

### Provenance Payloads Form a Monoid

Provenance payloads have **algebraic structure**:

**Composition (concatenation):**

```math
P \cdot Q = (\mu_0, \ldots, \mu_{m-1}, \nu_0, \ldots, \nu_{n-1})
```

**Identity (empty payload):**

```math
\varepsilon = ()
```

**Properties:**

1. **Closure:** `P · Q` is a provenance payload
2. **Associativity:** `(P · Q) · R = P · (Q · R)`
3. **Identity:** `ε · P = P = P · ε`

This is the "algebra" in "Worldline Algebra for Recursive Provenance."

**Why this matters:** Worldlines compose. If `(U₀, P)` replays to `Uₖ` and `(Uₖ, Q)` replays to `Uₙ`, then `(U₀, P · Q)` replays to `Uₙ`.

This compositionality enables wormhole compression (collapsing multi-tick segments) and prefix forks (Git-style branching).

> [!note]
> Echo does this differently. At the `warp-core` layer, a “payload” is a linear sequence of tick patches along a single worldline. It works like this because Paper III slicing/holography is easiest to make correct for linear history first; Echo’s higher layers are expected to represent branching/merging explicitly as a commit DAG with merge patches (not just raw payload concatenation).

### Boundary Transition Records (BTRs)

The mathematical boundary `(U₀, P)` is sufficient for replay, but real systems need more:

**BTR format:**

```math
BTR = (h_in, h_out, U_0, P, t, \kappa)
```

Where:

- `h_in` = content hash of initial state `U₀`
- `h_out` = content hash of final state `Uₙ`
- `U₀` = initial state
- `P` = provenance payload
- `t` = timestamp or monotone counter
- `κ` = authentication tag (e.g., digital signature binding everything)

**Why BTRs matter:**

1. **Content-addressed indexing** - deduplicate and index by boundary hashes
2. **Checkpoint and resume** - self-contained segment you can verify independently
3. **Tamper-evidence** - `κ` ensures any modification is detectable
4. **Wormhole carrier** - natural packaging for compressed multi-tick segments

> [!note]
> Echo does this differently. Echo’s primary boundary artifacts today are `Snapshot` and `commit_id` (which commit to `state_root` + `patch_digest`), plus an optional `TickReceipt`. It works like this because timestamps/signatures and archival packaging (a full BTR container) are higher-layer concerns; the core must first guarantee deterministic hashing and replayability so signing/transport can be layered without changing semantics.

### The Provenance Graph

Tick patches declare:

- `In(μ)` = inputs they read
- `Out(μ)` = outputs they produce

The **provenance graph** `𝕡 = (V, E)` is:

- **Vertices** `V` = all values occurring in the replay
- **Edges** `v → w` iff some patch `μᵢ` has `v ∈ In(μᵢ)` and `w ∈ Out(μᵢ)`

Each edge carries the **tick index** of the patch that witnessed it.

**Mapping to W3C PROV:**

- Each tick patch `μ` = PROV Activity
- `In(μ)` = Entities `used` by that activity
- `Out(μ)` = Entities `generatedBy` that activity
- Edges in `𝕡` = `used`/`generatedBy` chains

### Derivation Graphs and Backward Provenance

For any value `v`, its **derivation graph** `D(v)` is the **backward causal cone** - all vertices that can reach `v` via directed paths in `𝕡`.

**Key properties:**

1. **Finite** - the payload is finite, each patch has finite inputs/outputs, so `D(v)` is finite
2. **Acyclic** - deterministic worldlines can't have cycles (causality flows forward in time)

**Backward provenance completeness:** Every produced value has exactly one producing patch.

If patches produce disjoint outputs (no value is produced twice), then the payload is backward provenance complete.

> [!note]
> Echo does this differently. Echo patches operate on **unversioned slot identities** (e.g., “node attachment for `NodeId N`”) but recover producer uniqueness by interpretation along a worldline: `slot@tick_index` is the produced value-version. It works like this because a patch’s digest should not depend on where it lands in a timeline; SSA-like versioning is derived from patch position during slicing/replay, not baked into the patch.

### Computational Holography Theorem

**Statement:** Given boundary encoding `B = (U₀, P)`, the replay `Replay(B)` is **uniquely determined**.

**Translation:** The entire interior worldline `U₀ ⇒ ... ⇒ Uₙ` is encoded by the boundary `(U₀, P)`.

**Proof sketch:** By induction. Each `Uᵢ₊₁ = Apply(Uᵢ, μᵢ)` is uniquely determined (patch-determinism). Induction on `i` yields uniqueness of the full replay.

**Not a tautology:** This only works if patches are **sufficient** and **stable** - they must eliminate ambiguity (tie-breaking, policy choice) and avoid hidden dependencies on ambient state outside the patch boundary.

### Why "Holography" Is More Than Metaphor

**Compactness:** The bulk (full execution) is high-volume. The boundary (payload) is low-dimensional (linear sequence of patches).

**Sufficiency:** The boundary is **information-complete** for reconstruction under determinism.

**Description length:** The payload is a compressed description of the interior computation, relative to a fixed interpreter (Apply + rule-pack). Not Kolmogorov-minimal, but often dramatically shorter than full traces - and crucially, **executable**.

**AdS/CFT analogy (cautious):** Like AdS/CFT holography in physics, a lower-dimensional boundary determines a higher-dimensional bulk. But this is **computational**, not physical - the "duality" is conditional on determinism + patch sufficiency.

The value of the analogy is explanatory, not a claim of physical equivalence.

### Slicing: Partial Materialization

You often don't need the **entire** worldline - just the causal cone for a specific output value.

**Slice payload:**

```math
P|_{D(v)} = (\mu_i)_{i \in I(v)}
```

Where `I(v)` = tick indices whose patches contribute to `D(v)` (in increasing order).

**Slicing theorem:** Under assumptions (footprint completeness, no hidden writes, backward provenance complete), replaying the slice payload `P|_{D(v)}` from `U₀` reconstructs `v` (and its derivation graph `D(v)`) **without materializing the rest of the bulk**.

**Engineering win:** When a consumer only needs to verify one output value, ship the slice payload instead of the full payload - reduces bandwidth and verification cost without weakening determinism.

**Footprint completeness:** Apply depends **only** on the restriction of the current state to `In(μ)` and the patch itself.

**No hidden writes:** Apply affects **only** values in `Out(μ)` (any effect on future applicability is mediated through declared outputs).

> [!note]
> Echo does this differently. Echo’s `in_slots`/`out_slots` are currently conservative and coarse (nodes/edges/attachments/ports), so slices may be larger than theoretically minimal. It works like this because over-approximating reads keeps slices correct; under-approximating reads is a silent correctness failure for Paper III (“looks right, doesn’t slice”).

### Prefix Forks: Git-Style Branching

Under content-addressed (Merkle) storage, **branching avoids duplicating the shared prefix**.

Two worldlines that share a common prefix need only store the shared portion once; divergence occurs only at the point of difference.

**Definition:** Payloads `P` and `Q` **share prefix** `(μ₀, ..., μₖ₋₁)` if they agree on the first `k` patches, then diverge at tick `k`.

**Prefix-deduplicated branching:**

1. Worldlines `Replay(U₀, P)` and `Replay(U₀, Q)` agree on states `U₀, ..., Uₖ`
2. Under content-addressed storage, the shared prefix is stored **once** - only divergent suffixes require additional space

**Git analogy:**

- A **branch** = payload suffix starting from a shared commit
- **Forking** = create new suffix from existing prefix (no duplication under content addressing)
- **Merging** (when semantically meaningful) = payload concatenation `P · Q` (subject to boundary state matching)

This is valuable for exploratory computation, hypothesis testing, "what-if" analysis - fork a worldline, explore an alternative, compare results without duplicating shared history.

> [!note]
> Echo does this differently. Echo’s long-term plan is to treat merges as first-class commits with an explicit **merge patch** that resolves slot conflicts deterministically, rather than as plain payload concatenation. It works like this because “merge” is not just a history operation: it’s a semantic event that must choose outcomes when parents disagree on the same slot (including portals/attachments).

### Wormholes: Provenance-Preserving Compression

A **wormhole** is a single edge that compresses a multi-tick segment while preserving full provenance.

> [!note]
> Echo does this differently. Wormholes (tick-range compression edges) are a published-paper concept but are not yet implemented in `warp-core`. It works this way because correct slicing/replay and portal invariants had to land first; wormholes can then be layered as an optimization that preserves the same patch semantics.

**Wormhole boundary:**

```math
W(U_i, U_{i+k}) = P_{i:k} = (\mu_i, \ldots, \mu_{i+k-1})
```

**Wormhole edge:**

```math
e = (U_i, W(U_i, U_{i+k}), U_{i+k})
```

This represents the compressed k-tick transition `Uᵢ ⇒ᵏ Uᵢ₊ₖ`.

**Why wormholes:**

- **Semantically redundant** - they don't change what happened
- **Operationally useful** - single handle for indexing, checkpointing, replication
- **Checkpoint carriers** - store compressed wormhole, expand only when auditing
- **Compose well** - wormholes concatenate via the payload monoid: `P_{i:k} · P_{i+k:ℓ} = P_{i:k+ℓ}`

**Wormholes + prefix forks:** A shared prefix can be compressed into a single wormhole; subsequent forks attach to the wormhole's output state. Under content-addressed storage, this supports shared-prefix deduplication for worldline families with common ancestry.

### Why This Matters for Echo (Paper III)

Paper III provides the **provenance substrate**:

1. **Compact boundary representation** - store `(U₀, P)` instead of full interior volume
2. **Verifiable replay** - anyone with the boundary can reconstruct and verify the computation
3. **Sliceable provenance** - materialize only the causal cone needed for a specific output
4. **Git-style branching** - fork worldlines at shared prefixes without duplicating history
5. **Tamper-evident packaging** - BTRs ensure any modification is detectable
6. **Provenance graphs** - explicit dependency tracking via `In(μ)` and `Out(μ)`
7. **Wormhole compression** - checkpoint long segments as single edges

This is the foundation for:

- Time-travel debugging (replay from any checkpoint)
- Counterfactual branching (fork at any prefix, explore alternatives)
- Audit trails (verify specific outputs without full re-execution)
- Distributed verification (ship slices instead of full logs)

**Papers I-III together:**

- **Paper I** - the state space (WARPs)
- **Paper II** - the deterministic dynamics (ticks, two-plane semantics)
- **Paper III** - the provenance encoding (boundary holography)

With these three pieces, Echo has:

- Deterministic replay (same boundary → same worldline)
- Provenance-ready execution (tick patches = first-class objects)
- Verifiable computation (boundary encodes interior)

**The revolution will be deterministic. And auditable.**

---

## Paper IV: Rulial Distance & Observer Geometry

### Source (Paper IV)

[![DOI](https://zenodo.org/badge/DOI/10.5281/zenodo.18038297.svg)](https://doi.org/10.5281/zenodo.18038297)

```text
Ross, J. (2025). WARP Graphs: Rulial Distance & Observer Geometry (v1.0.0). Zenodo. https://doi.org/10.5281/zenodo.18038297
```

### The Problem: Which View Is "Right"?

Papers I-III gave us deterministic execution with provenance. But there's a problem:

**Different observers see the same computation differently.**

- A **compiler** sees AST transformations, IR optimizations, and register allocation
- A **compliance auditor** sees only inputs, outputs, and policy decisions
- A **debugger** sees every microstep, state transition, and match candidate
- A **performance analyst** sees CPU profiles, memory allocations, and cache misses

All of these are observing the **same underlying worldline**. But their traces can differ.

The naive question is: "Which observer is right?"

The **correct question** is: "Given two observers that emit different trace languages, what is the **cost of translating** between them under explicit resource constraints?"

This cost has two components:

1. **Description length** - how complex is the translator program?
2. **Distortion** - how much information is lost in translation?

For Echo, this matters because:

- Verify computations without re-running them (translate boundary → bulk)
- Compare alternative observers (which trace format should we deploy?)
- Understand when summarization breaks verification (does distortion exceed tolerance?)

> [!note]
> Echo does this differently. Echo does not yet implement Paper IV’s observer/translator machinery (or compute rulial distance) in code; the current runtime work focuses on producing deterministic, hash-committed boundary artifacts (patches/receipts/state roots) so that observers can be built on top. It works like this because “observer geometry” only becomes meaningful once the underlying history/provenance substrate is stable and replayable.

### The Solution: Observer Geometry via Rulial Distance

**Observers as functors:**

An **observer** `O` is a functor from the history category to a trace space:

```math
O: Hist(\mathcal{U}, R) \to Tr
```

Where:

- `Hist(𝒰, R)` = history category (paths through the multiway graph)
- `Tr` = trace space with a distortion metric `dist_tr`

> [!note]
> Echo does this differently. Echo’s current implementation does not build a full “multiway graph of all possible rewrites” inside `warp-core`; it executes one deterministic worldline per tick stream, and higher layers are expected to represent branching/merging explicitly as a commit DAG. It works like this because enumerating the full multiway space is computationally explosive; Echo treats “multiway” as a policy/tooling layer that can be derived from deterministic patch boundaries when needed.

**Resource budgets:**

An observer is **(τ, m)-bounded** if it can be implemented within time `τ` and memory `m`.

**Why budgets matter:** Without explicit budgets, all observers collapse into "compute the full worldline and output it" - no geometry. Budgets ensure the geometry respects real computational constraints.

### Translators: Converting Between Trace Formats

A **translator** from `O₁` to `O₂` is an algorithmic operator:

```math
T_{12}: Tr \to Tr
```

Such that `T₁₂ ∘ O₁` approximates `O₂`.

**MDL complexity:**

We measure translator complexity using **Minimum Description Length (MDL)**:

- `DL(T)` = length of the translator's code word (in a prefix-free code)

**Key property (subadditivity):** For composable translators,

```math
DL(T_{23} \circ T_{12}) \leq DL(T_{12}) + DL(T_{23}) + c
```

Where `c` is a small constant (prefix-coding overhead).

### Distortion: How Much Gets Lost?

Fix a metric `dist_tr` on trace space. The **lifted distortion** between observers is:

```math
Dist(O, O') = sup_{h \in Hist} dist_tr(O(h), O'(h))
```

**Translation:** Worst-case trace distance over all histories.

**Non-expansiveness assumption:** Post-composition by any translator is 1-Lipschitz:

```math
Dist(T \circ O, T \circ O') \leq Dist(O, O')
```

### Directed Rulial Cost

For observers `O₁, O₂`, the **directed cost** is:

```math
\vec{D}_{\tau,m}(O_1 \to O_2) = \inf_{T_{12} \in Trans_{\tau,m}(O_1, O_2)} (DL(T_{12}) + \lambda \cdot Dist(O_2, T_{12} \circ O_1))
```

Where:

- `λ > 0` = weighting parameter (trade-off between description length and distortion)
- `Trans_{τ,m}(O₁, O₂)` = translators admissible within budgets `(τ, m)`

**Translation:** The cheapest way to translate from `O₁` to `O₂`, balancing translator complexity against residual distortion.

If no translator exists within the budget, `→D_{τ,m} = +∞`.

### Rulial Distance (Symmetrized)

The **rulial distance** is:

```math
D_{\tau,m}(O_1, O_2) = \vec{D}_{\tau,m}(O_1 \to O_2) + \vec{D}_{\tau,m}(O_2 \to O_1)
```

**Properties:**

1. **Non-negativity:** `D_{τ,m}(O₁, O₂) ≥ 0`
2. **Symmetry:** `D_{τ,m}(O₁, O₂) = D_{τ,m}(O₂, O₁)`
3. **Reflexivity:** `D_{τ,m}(O, O) = 0`
4. **Triangle inequality (up to constant):** `D_{τ,m}(O₁, O₃) ≤ D_{τ,m}(O₁, O₂) + D_{τ,m}(O₂, O₃) + 2c`

This makes `D_{τ,m}` a **quasi-pseudometric** - it satisfies all metric axioms except the triangle inequality holds only up to additive constant `2c` (prefix-coding overhead).

**Budget monotonicity:** Relaxing budgets can only decrease distance:

```math
If (\tau', m') \geq (\tau, m), \text{ then } D_{\tau',m'}(O_1, O_2) \leq D_{\tau,m}(O_1, O_2)
```

### Lawvere Metric: The Enriched Category Viewpoint

The underlying translation problem is **directed** - boundary → bulk can be infeasible under strict budgets, while bulk → boundary is cheap (projection).

**Lawvere metric space:** A category enriched over the monoidal poset `([0,∞], ≥, +, 0)`:

- Objects = observers
- Hom-values `d_{τ,m}(O₁, O₂)` = directed cost `→D_{τ,m}(O₁ → O₂)`
- Composition = addition (triangle inequality)
- `d_{τ,m}(O, O) = 0` (reflexivity)
- No symmetry required

**Key insight:** Directed costs compose by addition (triangle inequality), budgets produce `+∞` hom-values (no admissible translator), and asymmetry is the generic case.

### Example: Boundary vs Bulk

Let:

- `O_∂` = boundary observer (outputs `(U₀, P)`)
- `O_bulk` = bulk observer (outputs `(U₀, U₁, ..., Uₙ)`)

**Forgetful projection (`O_bulk → O_∂`):**

- `DL(T_forget) = O(1)` (constant description length)
- `Dist = 0` (no information loss - boundary is already in bulk)
- `→D_{τ,m}(O_bulk → O_∂) = O(1)` (cheap!)

**Replay expansion (`O_∂ → O_bulk`):**

- `DL(T_replay) = O(1)` (the interpreter is fixed)
- `Dist = 0` (exact replay)
- **But:** time cost grows with `|P|` (payload length)
- Under strict budgets: `→D_{τ,m}(O_∂ → O_bulk) = +∞` (infeasible!)
- Under unbounded budgets: `→D_{∞,∞}(O_∂ → O_bulk) = O(1)` (cheap)

**Takeaway:** Replay is **short** (low description length) but **not fast** (high time cost). The geometry captures this asymmetry.

### Multiway Systems and the Ruliad

**Multiway graph:** The directed graph `MW(𝒰, R)` where vertices are states and edges are individual rewrite steps (including alternative matches/orderings).

**History category:** `Hist(𝒰, R)` is the **path category** of the multiway graph:

- Objects = states
- Morphisms = finite directed paths
- Composition = path concatenation

**Deterministic worldlines as functors:** A deterministic worldline defines a functor `W: ℕ → Hist(𝒰, R)` selecting a unique path for fixed boundary data.

**The Ruliad:** The large history space built from all possible computations:

```math
Ruliad = \bigsqcup_{(U_0, R) \in \mathfrak{U} \times \mathfrak{N}} Hist(\mathcal{U}_{U_0,R}, R)
```

(Disjoint union of history categories over initial states and rule packs)

**Translation:** The Ruliad is the "possibility space" containing all potential computations. Deterministic worldlines are small, selected paths within this vast space.

### Chronos, Kairos, Aion: The Three-Layer Time Model

**Chronos** - linear time of a fixed worldline:

- The finite linear order `0 < 1 < ... < n` on committed ticks
- Functor `Chronos: [n] → Hist(𝒰, R)` selecting the unique replay path

**Kairos** - branch events:

- Points where alternative continuations exist in the multiway graph
- Alternative matches, schedules, rule packs, or inputs
- Within-tick conflict points (witnessed by tick-event posets from Paper II)

**Aion** - the possibility space:

- The full history category `Hist(𝒰, R)`
- All finite derivations in the multiway graph
- At largest scale: the Ruliad

**Analogy:**

- **Chronos** = the timeline you're on
- **Kairos** = the moments where you could have branched
- **Aion** = the space of all possible timelines

### Temporal Logic on Histories

To reason about liveness, safety, and reconciliation properties, we introduce a minimal temporal logic.

**Atomic propositions:** Predicates on trace space (observer-relative)

**CTL\*-style language:**

- State formulas: `φ ::= p | ¬φ | (φ ∧ φ) | 𝐀ψ | 𝐄ψ`
- Path formulas: `ψ ::= φ | ¬ψ | (ψ ∧ ψ) | 𝐗ψ | 𝐅ψ | 𝐆ψ | (ψ 𝐔 ψ)`

**Operators:**

- `𝐀` = "for all paths" (all continuations)
- `𝐄` = "there exists a path" (some continuation)
- `𝐗` = "next" (one step ahead)
- `𝐅` = "eventually" (at some future point)
- `𝐆` = "always" (at all future points)
- `𝐔` = "until" (φ holds until ψ becomes true)

**Example (liveness):** `𝐆𝐅 p_expose` = "always eventually, provenance receipts are exposed"

**Example (reconciliation):** `𝐀𝐅 p_merge` = "all branches eventually merge"

**Transport lemma:** If observers `O₁, O₂` are connected by a low-distortion translator, and atomic propositions are δ-robust, then temporal formulas have the same truth values:

```math
O_2 \models \varphi  \iff  (T \circ O_1) \models \varphi
```

**Translation:** Temporal properties transport across observers when translation distortion is below the robustness threshold.

### Observer Geometry as Frame Separation

Within the Ruliad, an observer assigns traces to histories. Two observers may differ substantially on causal structure yet be **near** each other in rulial distance (low translation cost). Conversely, they may agree semantically but be **far** (high translation cost under budgets).

**Rulial balls:** `B_r(O) = {O' : D_{τ,m}(O, O') ≤ r}` collects observers mutually reachable within fixed translation cost.

**Engineering implication:** If a compliance observer is far from a diagnostic observer under deployment budgets, then emitting only compliance traces is **not neutral** - it makes diagnosis expensive or impossible.

### Computability and Engineering

Rulial distance is defined by an infimum over all admissible translators - like Kolmogorov complexity, it's a useful **specification** but not something we compute exactly.

**Engineering approach:**

1. Build explicit translators `T₁₂, T₂₁`
2. Measure/estimate resource usage under `(τ, m)`
3. Use `DL(T₁₂) + λ·Dist(O₂, T₁₂ ∘ O₁)` as an **upper bound** on directed cost
4. Constructing better translators tightens bounds

This turns rulial distance from an abstract infimum into an **actionable design parameter**.

### Why This Matters for Echo (Paper IV)

Paper IV provides the **observer geometry**:

1. **Observers are functors** - resource-bounded mappings from histories to traces
2. **Translators are measured** - MDL description length + trace distortion
3. **Rulial distance is computable** - explicit translators give upper bounds
4. **Direction matters** - Lawvere metric captures asymmetry (boundary ↔ bulk)
5. **Budgets are first-class** - same observers can be near or far depending on `(τ, m)`
6. **Temporal logic transports** - low-distortion translation preserves liveness/safety properties
7. **Three-layer time model** - Chronos (linear), Kairos (branches), Aion (possibility space)

This is the foundation for:

- **Observer design** - choose deployed observer `O` so required views lie in small rulial ball `B_r(O)`
- **Trace format selection** - balance description length vs distortion for verification needs
- **Verification cost bounds** - rulial distance predicts translation cost for compliance/debugging
- **Counterfactual analysis** - Kairos branch points enable "what-if" exploration

**Papers I-IV together:**

- **Paper I** - the state space (WARPs)
- **Paper II** - the deterministic dynamics (ticks, two-plane semantics)
- **Paper III** - the provenance encoding (boundary holography)
- **Paper IV** - the observer geometry (rulial distance)

With these four pieces, Echo has:

- A canonical state space (nested graphs)
- Deterministic execution (scheduler-admissible batches)
- Verifiable provenance (boundary encodings)
- Quantifiable observer separation (translation cost)

**The revolution will be deterministic, auditable, and observer-aware.**

---

## NOTE: Echo is a Game Engine

Echo is a pragmatic, high-performance implementation of the AIΩN Foundations ideas.

That means two things can be true at once:

1. Echo treats the Foundations papers as a **north star** for terminology and semantics.
2. Echo will sometimes choose different engineering trade-offs to stay viable as a game/simulation runtime.

When Echo diverges, it should not be a mystery or an accident.

- Prefer “different but equivalent” implementations (same semantics, different mechanism).
- When semantics genuinely diverge, document the choice and rationale.

> [!note]
> Echo does this differently (by policy). Echo prioritizes determinism + replayability _and_ runtime performance. It works like this because Echo is meant to run real simulations, not just prove theorems — but every deviation from the Foundations series should be explained so readers can map paper concepts to the codebase without guesswork.

For canonical mappings and explicit deviation rationale, see `docs/aion-papers-bridge.md`.
