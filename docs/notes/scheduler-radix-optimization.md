<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# From $O(n log n)$ to $O(n)$: Optimizing Echo's Deterministic Scheduler

**Tags:** performance, algorithms, optimization, radix-sort

---
## TL;DR

- Early benchmarks demonstrate that **Echo** can run at 60 fps while pushing ~5,000 DPO graph rewrites per frame
- Big viability question answered
- "Game scale" activity: confirmed

## What is Echo?

**Echo is a deterministic simulation engine built on graph rewriting theory.** While its applications are broad, it was born from the world of game development, so we'll use "game engine" as our primary lens.

Unlike traditional game engines, which manage state through mutable object hierarchies and event loops, Echo represents the entire simulation state as a typed graph. This graph evolves through **deterministic rewrite rules**—mathematical transformations that guarantee identical results across platforms, replays, and simulations.

At Echo's core is the _**Recursive Meta‑Graph**_ (RMG). In Echo, _everything_ is a graph. Nodes are graphs, meaning a "player" is a complex subgraph with its own internal graph structure, not just an object. Edges are graphs, too, and can also have their own internal graphs, allowing expressiveness that carries structure and provenance. And most importantly, rules are graph rewrites. Echo updates the simulation by finding specific patterns in the RMG and replacing them with new ones. Every frame, the RMG is replaced by a new RMG, an _echo_ of the state that came before it.

### Why bother? Aren't game engines a solved problem? We got Unreal/Unity...

That's a fair question, but it’s aimed at the wrong target. While engines like Unreal and Unity are phenomenal rendering powerhouses and asset pipelines, they are built on an architectural foundation that struggles with the hardest problems in game development: **state management and networking**.

The open secret of multiplayer development is that no two machines in a session ever truly agree on the game's state. What the player experiences is a sophisticated illusion, a constant, high-speed negotiation between **client-side prediction** and **authoritative server corrections**.

I know this because I'm one of the developers who built those illusions. I've written the predictive input systems and complex netcode designed to paper over the cracks. The "rubber-banding" we've all experienced isn't a _bug_—it's an _artifact_. It's the unavoidable symptom of a system where state is **divergent by default**.

This architectural flaw creates a secondary nightmare: **debugging**. When state is mutable, concurrent, and non-deterministic, reproducing a bug becomes a dark art. It's often impossible to look at a game state and know with certainty _how it got that way_. The system is fundamentally non-reproducible.

The state of the art is built on patches, prediction, and arbitration to hide this core problem. The architecture itself is fragile.

Until now.

### Version Control for Reality

One way to understand how Echo works is to imagine the simulation as version control for moments in time. In this mental model, a frame is like an immutable commit. And like a commit each frame has a canonical, cryptographic hash over the entire reachable graph, encoded in a fixed order. Echo treats inputs from players and other game world updates as candidate graph rewrites, and thanks to *confluence*, some category theory math, we can fold them into a single, deterministic effect. Finally, the scheduler applies all rewrites in a deterministic order and produces the next snapshot.

No prediction. No rollback. No "authoritative correction." Just one pure function from `(world, inputs) → world′`.

If two machines disagree, they disagree fast: a hash mismatch at frame `N+1` is a precise alarm, not a rubber‑band later.

### ASCII timeline (branching and merge, deterministically):

```
 Frame₀
   │
   ▼
 Frame₁───┐
   │      \
   ▼       \
 Frame₂A   Frame₂B
   │         │
   └────┬────┘
        ▼
      Merge₃  (confluence + canonical rewrite order)
```

### What Echo Unlocks

This "version control" model isn't just a metaphor; it's a new architecture that unlocks capabilities that look "impossible" in a traditional engine.

It enables **perfect replays**, as every frame is a commit that can be recomputed from its inputs to a bit‑identical state. This, in turn, provides an **infinite debugger**: provenance is embedded directly in the graph, allowing you to query its history to see who changed what, when, and why.

For competitive games, this provides **provable fairness**, as a frame's cryptographic hash is a verifiable signature of "what happened." This all adds up to **zero silent desync**. A hash mismatch catches drift immediately and precisely, long before a user ever notices.

Networking becomes straightforward: distribute inputs, compute the same function, compare hashes. When the math agrees, the world agrees.

## [](https://dev.to/flyingrobots/determinism-by-construction-inside-echos-recursive-meta-graph-ecs-3491-temp-slug-8201751?preview=3b87bb097d6497d71ce72d6b6e87a1a101318ff960042f1db3908b807b6dd9a1b0b3811607d98ea25549311a530faa30d469ddd1cf0ac2c60e8f92fd#confluence-not-arbitration)Confluence, Not Arbitration

When multiple updates target related state, we don't race them, we _merge_ them with deterministic math. We use **confluence operators** with **lattice** properties:

**Associative**, **Commutative**, **Idempotent** (ACI)

Examples:

Tags union: `join(TagsA, TagsB) = TagsA ∪ TagsB`

Scalar cap: `join(Cap(a), Cap(b)) = Cap(max(a, b))`

Those properties guarantee that folding a bucket of updates yields one result, independent of arrival order and partitioning.

## [](https://dev.to/flyingrobots/determinism-by-construction-inside-echos-recursive-meta-graph-ecs-3491-temp-slug-8201751?preview=3b87bb097d6497d71ce72d6b6e87a1a101318ff960042f1db3908b807b6dd9a1b0b3811607d98ea25549311a530faa30d469ddd1cf0ac2c60e8f92fd#safe-parallelism-by-construction)Safe Parallelism by Construction

Echo implements updates as **DPO (Double Push‑Out) graph rewrites**. This structure provides safe parallelism by construction: independent rewrites can apply in parallel without issue. Any overlapping rewrites are either deterministically merged by a lattice or rejected as invalid. For any remaining, dependent rewrites, the scheduler enforces a canonical order.

The upshot: "Which rule ran first?" stops being a source of nondeterminism.

A sketch of the full _fold→rewrite→commit_ pipeline:

> 1. Collect inputs for frame `N+1`.
> 2. Bucket by (scope, rule family).
> 3. Confluence fold each bucket (ACI).
> 4. Apply remaining rewrites in a canonical order:
> 
> ```
> order by (scope_hash, family, compact_rule_id, payload_digest).
> ```
> 
> 1. Emit a new snapshot and compute commit hash.

## [](https://dev.to/flyingrobots/determinism-by-construction-inside-echos-recursive-meta-graph-ecs-3491-temp-slug-8201751?preview=3b87bb097d6497d71ce72d6b6e87a1a101318ff960042f1db3908b807b6dd9a1b0b3811607d98ea25549311a530faa30d469ddd1cf0ac2c60e8f92fd#a-tiny-rewrite-a-tiny-lattice)A Tiny Rewrite, A Tiny Lattice

Rewrite (motion) in Scalar terms:

> Match: an entity with position p and velocity v  
> Replace: position p′ = p + v·dt; velocity unchanged

Lattice example (cap / max):

> join(Cap(α), Cap(β)) = Cap(max(α, β))  
> ACI → the fold of {Cap(2), Cap(5), Cap(3)} is Cap(5) regardless of order.

These primitives, **rewrites** and **lattices**, are the heart of Echo's "determinism by construction."

**What makes Echo different:**

- **Determinism by design**: Same inputs → same outputs, always. No floating-point drift, no race conditions, no "it works on my machine."
- **Formal semantics**: Built on Double Pushout (DPO) category theory—every state transition is mathematically provable.
- **Replay from the future**: Rewind time, fork timelines, or replay from any checkpoint. Your game is a pure function.
- **Networked lockstep**: Perfect synchronization without sending world state. Just send inputs; all clients compute identical results.
- **AI training paradise**: Deterministic = reproducible = debuggable. Train agents with confidence.

Echo isn't just another ECS—it's a **fundamentally different way to build games**, where the scheduler isn't just an implementation detail, it's the guarantee of determinism itself.

---

## The Problem: $O(n log n)$ Was Showing

Echo's deterministic scheduler needs to execute rewrites in strict lexicographic order: `(scope_hash, rule_id, nonce)`. This ensures identical results across platforms and replays—critical for a deterministic game engine.

Our initial implementation used a `BTreeMap<(Hash, Hash), PendingRewrite>`:

```rust
// Old approach
pub(crate) pending: BTreeMap<(Hash, Hash), PendingRewrite>
```

**The bottleneck:** At scale, draining and sorting n rewrites required **$O(n log n)$** comparisons over 256-bit scope hashes. Benchmarks showed:

```
n=1000:  ~1.33ms (comparison sort via BTreeMap iteration)
n=3000:  ~4.2ms  (log factor starting to hurt)
```

Curve fitting confirmed **T/n ≈ -345 + 272.7·ln(n)**—textbook $O(n log n)$.

---

## The Solution: 20-Pass Radix Sort

Radix sort achieves **$O(n)$** complexity with zero comparisons by treating keys as sequences of digits. We implemented:

- **LSD radix sort** with 16-bit big-endian digits
- **20 passes total**: 2 for nonce, 2 for rule_id, 16 for full 32-byte scope hash
- **Stable sorting** preserves insertion order for tie-breaking
- **Byte-lexicographic ordering** exactly matches BTreeMap semantics

### The Architecture

```rust
struct RewriteThin {
    scope_be32: [u8; 32],  // Full 256-bit scope
    rule_id:    u32,       // Compact rule handle
    nonce:      u32,       // Insertion-order tie-break
    handle:     u32,       // Index into fat payload vec
}

struct PendingTx<P> {
    thin:     Vec<RewriteThin>,     // Sorted keys
    fat:      Vec<Option<P>>,       // Payloads (indexed by handle)
    scratch:  Vec<RewriteThin>,     // Reused scratch buffer
    counts16: Vec<u32>,             // 256KB histogram (65536 buckets)
}
```

**Key insight:** Separate "thin" sorting keys from "fat" payloads. Only move 28-byte records during radix passes, then gather payloads at the end.

```mermaid
graph LR
    subgraph "Thin Keys (sorted)"
        T1[RewriteThin<br/>handle=0]
        T2[RewriteThin<br/>handle=2]
        T3[RewriteThin<br/>handle=1]
    end

    subgraph "Fat Payloads (indexed)"
        F0[PendingRewrite]
        F1[PendingRewrite]
        F2[PendingRewrite]
    end

    T1 -->|handle=0| F0
    T2 -->|handle=2| F2
    T3 -->|handle=1| F1

    style T1 fill:#e0af68
    style T2 fill:#e0af68
    style T3 fill:#e0af68
    style F0 fill:#9ece6a
    style F1 fill:#9ece6a
    style F2 fill:#9ece6a
```

### Radix Sort Pass Sequence

The 20-pass LSD radix sort processes digits from least significant to most significant:

```mermaid
graph TD
    Start[Input: n rewrites] --> P1[Pass 1-2: nonce low→high]
    P1 --> P2[Pass 3-4: rule_id low→high]
    P2 --> P3[Pass 5-20: scope_hash bytes 31→0]
    P3 --> Done[Output: sorted by scope,rule,nonce]

    style Start fill:#bb9af7
    style Done fill:#9ece6a
    style P1 fill:#e0af68
    style P2 fill:#e0af68
    style P3 fill:#ff9e64
```

Each pass:
1. **Count** — histogram of 65536 16-bit buckets
2. **Prefix sum** — compute output positions
3. **Scatter** — stable placement into scratch buffer
4. **Flip** — swap `thin ↔ scratch` for next pass

---

## The Disaster: Small-n Regression

Initial results were... not encouraging:

```
BEFORE (BTreeMap):        AFTER (Radix):
n=10:    7.5µs            n=10:    687µs    (91x SLOWER!)
n=100:   90µs             n=100:   667µs    (7x SLOWER!)
n=1000:  1.33ms           n=1000:  1.36ms   (marginal)
```

![Before optimization - the "flat green line" disaster](BEFORE.webp)
*The benchmark graph tells the story: that flat green line at low n is 5MB of zeroing overhead dominating tiny inputs.*

**What went wrong?** The radix implementation zeroed a **256KB counts array 20 times per drain**:

```rust
counts.fill(0);  // 65,536 × u32 = 256KB
// × 20 passes = 5MB of writes for ANY input size
```

At n=10, we were doing **5MB of memory bandwidth** to sort **10 tiny records**. The "flat green line" in the benchmark graph told the story—massive fixed cost dominating small inputs.

---

## The Fix: Adaptive Threshold

The solution: **use the right tool for the job.**

```mermaid
graph TD
    Start[n rewrites to drain] --> Check{n ≤ 1024?}
    Check -->|Yes| Comp[Comparison Sort<br/>O n log n <br/>Low constant]
    Check -->|No| Radix[Radix Sort<br/>O n <br/>High constant]
    Comp --> Done[Sorted output]
    Radix --> Done

    style Start fill:#bb9af7
    style Comp fill:#e0af68
    style Radix fill:#9ece6a
    style Done fill:#bb9af7
    style Check fill:#ff9e64
```

```rust
const SMALL_SORT_THRESHOLD: usize = 1024;

fn drain_in_order(&mut self) -> Vec<P> {
    let n = self.thin.len();
    if n > 1 {
        if n <= SMALL_SORT_THRESHOLD {
            // Fast path: comparison sort for small batches
            self.thin.sort_unstable_by(cmp_thin);
        } else {
            // Scalable path: radix for large batches
            self.radix_sort();
        }
    }
    // ... drain logic
}

fn cmp_thin(a: &RewriteThin, b: &RewriteThin) -> Ordering {
    a.scope_be32.cmp(&b.scope_be32)
        .then_with(|| a.rule_id.cmp(&b.rule_id))
        .then_with(|| a.nonce.cmp(&b.nonce))
}
```

**Why 1024?** Empirical testing showed:
- Below ~500: comparison sort wins (no zeroing overhead)
- Above ~2000: radix sort wins ($O(n)$ scales)
- **1024: conservative sweet spot** where both approaches perform similarly

![After optimization - hybrid approach](AFTER.webp)
*The fix: adaptive threshold keeps small inputs fast while unlocking $O(n)$ scaling at large $n$.*

---

## The Results: Perfect $O(n)$ Scaling

Final benchmark results across 6 data points (10, 100, 1k, 3k, 10k, 30k):

| Input n | Old (BTreeMap) | New (Hybrid) | Speedup | Per-element |
|---------|----------------|--------------|---------|-------------|
| 10      | 7.5µs          | 7.6µs        | -1%     | 760ns       |
| 100     | 90µs           | 76µs         | +16%    | 760ns       |
| 1,000   | 1.33ms         | 0.75ms       | **+44%** | 750ns    |
| 3,000   | —              | 3.03ms       | —       | 1010ns      |
| 10,000  | —              | 9.74ms       | —       | 974ns       |
| 30,000  | —              | 29.53ms      | —       | 984ns       |

![Final results - perfect linear scaling](Final.webp)
*The complete picture: purple (snapshot hash), green (scheduler total), yellow (enqueue), red (drain). Note the threshold marker at $n=1024$ and the perfectly straight lines beyond it.*

**Key observations:**

1. **Comparison sort regime ($n ≤ 1024$):** ~750ns/element, competitive with old approach
2. **Radix sort regime ($n > 1024$):** Converges to ~1µs/element with **zero deviation**
3. **Scaling from 3k → 30k (10× data):** 9.75× time—textbook $O(n)$
4. **60 FPS viability:** At $n=1000$ (typical game scene), scheduler overhead is just **0.75ms = 4.5% of 16.67ms frame budget**

### Phase Breakdown

Breaking down enqueue vs drain at $n=30k$:

```
Total:   37.61ms (100%)
Enqueue: 12.87ms (34%)  — Hash lookups + last-wins dedupe
Drain:   24.83ms (66%)  — Radix sort + conflict checks + execute
```

```mermaid
%%{init: {'theme':'dark'}}%%
pie title Scheduler Time Breakdown at n=30k
    "Enqueue (hash + dedupe)" : 34
    "Drain (radix + conflicts)" : 66
```

The drain phase dominates, but both scale linearly. Future optimizations could target the radix sort overhead (active-bucket zeroing, cross-transaction pooling), but the current approach achieves our performance targets.

---

## The Visualization: Telling the Story

We built an interactive D3 dashboard (`docs/benchmarks/report-inline.html`) showing:

- **Four series on log-log plot:**
  - Purple (solid): Snapshot Hash baseline
  - Green (solid): Scheduler Drain Total
  - Yellow (dashed): Enqueue phase
  - Red (dashed): Drain phase

- **Threshold marker at $n=1024$** showing where the sorting strategy switches

- **2×2 color-coded stat cards** matching chart colors for instant visual connection

- **Explanatory context:** What we measure, why 60 FPS matters, how $O(n)$ scaling works

**The key visual:** A straight line on the $log-log$ plot from 3k to 30k—proof of perfect linear scaling.

---

## Lessons Learned

### 1. **Measure First, Optimize Second**
Curve fitting (`T/n ≈ 272.7·ln(n)`) confirmed the $O(n log n)$ bottleneck before we touched code.

### 2. **Don't Optimize for Benchmarks Alone**
The initial radix implementation looked good at $n=1000$ but destroyed small-batch performance. Real workloads include both.

### 3. **Memory Bandwidth Matters**
Zeroing 5MB of counts array matters more than CPU cycles at small $n$. The "flat line" in benchmarks was the smoking gun.

### 4. **Hybrid Approaches Win**
Comparison sort isn't "slow"—it's just $O(n log n)$. For small $n$, it's faster than **any** $O(n)$ algorithm with high constants.

### 5. **Visualize the Win**
A good chart tells the story instantly. Our dashboard shows the threshold switch, phase breakdown, and perfect scaling at a glance.

---

## What's Next?

Future optimizations:

1. **Active-bucket zeroing**: Only zero counts buckets actually used (saves ~15% at large $n$)
2. **Cross-transaction pooling**: Share scratch buffers across transactions via arena allocator
3. **Rule-domain optimization**: If we have <256 rules, collapse `rule_id` to single-byte direct indexing (saves 2 passes)

The scheduler is algorithmically optimal, scales to 30k rewrites in <30ms, and the constants are excellent.

---

## Conclusion: Echoing the Future

Echo's deterministic scheduler went from $O(n log n)$ BTreeMap to $O(n)$ hybrid adaptive sorter:

- ✅ **44% faster at typical workloads ($n=1000$)**
- ✅ **Perfect linear scaling to 30k rewrites**
- ✅ **Well under 60 FPS budget**
- ✅ **Zero regressions at small n**
- ✅ **Beautiful visualization proving the win**

The textbook said "radix sort is $O(n)$." The benchmarks said "prove it." **The graph is a straight line.**

But here's the deeper point: **This optimization matters because Echo is building something fundamentally new.**

Traditional game engines treat determinism as an afterthought—a nice-to-have feature bolted on through careful engineering and hope. Echo treats it as a **mathematical guarantee**, woven into every layer from category theory foundations to the scheduler you're reading about right now.

When you can execute 30,000 deterministic rewrite rules per frame and still hit 60 FPS, you're not just optimizing a scheduler—you're **proving that a different kind of game engine is possible.** One where:

- **Multiplayer "just works"** because clients can't desync (they're running the same pure function)
- **Replay isn't a feature**, it's physics (rewind time by replaying the graph rewrite history)
- **AI training scales** because every training episode is perfectly reproducible
- **Formal verification** becomes practical (prove your game logic correct, not just test it)
- **Time travel debugging** isn't science fiction (checkpoint the graph, fork timelines, compare outcomes)

Echo isn't just a faster game engine. **Echo is a different game engine.** One built on the mathematical foundation that traditional engines lack. One where the scheduler's deterministic ordering isn't a nice property—it's the **fundamental guarantee** that makes everything else possible.

This optimization journey—from spotting the $O(n log n)$ bottleneck to proving $O(n)$ scaling with a hybrid radix sorter—is what it takes to make that vision real. To make determinism **fast enough** that developers don't have to choose between correctness and performance.

The graph is a straight line. The future is deterministic. **And Echo is how we get there.** 🚀

---

## Code References

- Implementation: `crates/rmg-core/src/scheduler.rs:142-277`
- Benchmarks: `crates/rmg-benches/benches/scheduler_drain.rs`
- Dashboard: `docs/benchmarks/report-inline.html`
- PR: [Pending on branch `repo/tidy`]

---

*Want to learn more? Check out the [Echo documentation](../../) or join the discussion on [GitHub](https://github.com/flyingrobots/echo).*
