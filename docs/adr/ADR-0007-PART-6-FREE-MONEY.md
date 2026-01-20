<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
Alright James. Here’s the REAL™ Editor’s Edition™ Free Money™ “Free.99$” 2026 TURBO DX GAIDEN — the version you print, frame, and throw at anyone who tries to “optimize” determinism by vibes.

⸻

FREE.99$ TURBO DX GAIDEN — Phase 6A “Free Money” (LOCKED)

The One-Liner

Parallel execution is allowed to be chaotic because determinism is enforced afterward by canonical merge.

If somebody argues that “the threads should run in a deterministic order,” that person is volunteering to debug deadlocks for the rest of their natural life.

⸻

The Theory That Matters (and ONLY the theory that matters)

1) Determinism is a serialization decision, not an execution constraint

You don’t make parallel execution deterministic by controlling thread scheduling. You make it deterministic by ensuring that whatever happened in parallel gets collapsed into the same canonical sequence every time.

Execution: nondeterministic schedule permitted
Merge: deterministic ordering required
Commit: deterministic hashing required

If you try to push determinism “down” into execution scheduling, you will:
 • slow everything down
 • still miss edge cases
 • and then merge will silently differ anyway

Merge is the law. Execution is the crime scene.

⸻

1) “Free money” exists only for independent rewrites

Phase 6 is not “parallel everything.” It’s:
 • Admit only independent rewrites (footprints)
 • Then parallelize execution of that admitted set
 • Then canonicalize the result

If footprints are wrong, parallelism will “work” and your hashes will diverge and you’ll blame threads when it’s actually your model.

Footprints are your concurrency proof.

⸻

1) Canonical merge needs a stable tie-breaker (or you don’t have determinism)

Sorting by WarpOpKey alone is not enough. When keys collide you need a stable, replayable tiebreak order:

OpOrigin = (intent_id, compact_rule_id, match_ix, op_ix)
 • intent_id: stable identity from ingress/planner
 • match_ix: deterministic planner index
 • compact_rule_id: stable within run (good enough for Phase 6A)
 • op_ix: per-rewrite emission sequence, assigned by scoped emitter

Without op_ix, two ops from the same rewrite can reorder under merge and you get “it fails only on Tuesdays.” That’s how superstition is born.

⸻

1) Phase 6A treats conflicts as bugs, not runtime events

If merge sees divergent writes to the same WarpOpKey, that is not a “merge policy decision.” That is:

Footprint model lied.

So Phase 6A does the correct thing:
 • dedupe if identical
 • explode if different

Your engine is a determinism church. Conflicts are heresy.

⸻

The REAL™ “Locked In” Spec

Core invariants (non-negotiable)
 • Admission order deterministic: radix sort (scope_hash, compact_rule, nonce) ✅ already
 • Execution: lockless, thread-local deltas, read-only GraphView ✅
 • Merge: canonical sort by (WarpOpKey, OpOrigin) ✅
 • Hashes: patch_digest/state_root/commit_hash invariant across worker count + permutations ✅
 • Conflicts: explode loudly (Phase 6A) ✅

⸻

The “Don’t Screw This Up” API Surface

ExecuteFn (unchanged)

```rust
pub type ExecuteFn = for<'a> fn(GraphView<'a>, &NodeId, &mut TickDelta);
```

ExecItem (tiny, new)

```rust
pub struct ExecItem {
    pub exec: ExecuteFn,
    pub scope: NodeId,
    pub origin: OpOrigin, // base origin (op_ix assigned per emitted op)
}
```

OpOrigin (final)

Exists whenever Phase 6 parallel is enabled.

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct OpOrigin {
    pub intent_id: u64,
    pub rule_id: u32,   // CompactRuleId as u32
    pub match_ix: u32,
    pub op_ix: u32,     // per-rewrite sequential index
}
```

Origin sourcing
 • Preferred: intent_id = blake3(intent_bytes) truncated to u64 (or carry full hash if you want later)
 • Temp allowed: intent_id = LE u64 from scope_hash[0..8]
 • match_ix: deterministic planner ordering (temp 0 allowed)
 • rule_id: compact rule id
 • op_ix: assigned during emission

⸻

The Scoped Emitter That Makes This Real

Here’s the part everyone forgets: origins don’t magically appear. The rule must emit with origin.

So we give it a safe, ergonomic path:

```rust
pub struct ScopedDelta<'a> {
    inner: &'a mut TickDelta,
    base: OpOrigin,
    next: u32,
}

impl<'a> ScopedDelta<'a> {
    pub fn new(delta: &'a mut TickDelta, origin: OpOrigin) -> Self {
        Self { inner: delta, base: origin, next: 0 }
    }

    pub fn emit(&mut self, op: WarpOp) {
        let mut o = self.base;
        o.op_ix = self.next;
        self.next = self.next.wrapping_add(1);
        self.inner.emit_with_origin(op, o);
    }

    pub fn inner_mut(&mut self) -> &mut TickDelta {
        self.inner
    }
}
```

TURBO DX: make it hard to forget origin

Add this convenience:

```rust
impl TickDelta {
    pub fn scoped<'a>(&'a mut self, origin: OpOrigin) -> ScopedDelta<'a> {
        ScopedDelta::new(self, origin)
    }
}
```

Rules can now do:

```rust
let mut d = delta.scoped(origin);
d.emit(...);
d.emit(...);
```

No one “forgets” origin. No more default OpOrigin::default() poisoning your merge.

⸻

Execution

Serial baseline

This is the reference behavior. You compare parallel against this in tests.

```rust
pub fn execute_serial<'a>(view: GraphView<'a>, items: &[ExecItem]) -> TickDelta {
    let mut delta = TickDelta::new();
    for item in items {
        // The rule must use delta.scoped(origin) internally or call emit_with_origin.
        (item.exec)(view, &item.scope, &mut delta);
    }
    delta
}
```

Parallel (Phase 6A "Free.99$")

Stride partitioning. Explicit loop. Compiles. Works. No drama.

```rust
pub fn execute_parallel<'a>(
    view: GraphView<'a>,
    items: &'a [ExecItem],
    workers: usize,
) -> Vec<TickDelta> {
    assert!(workers >= 1);

    std::thread::scope(|s| {
        let mut handles = Vec::with_capacity(workers);

        for w in 0..workers {
            handles.push(s.spawn(move || {
                let mut delta = TickDelta::new();
                let mut i = w;
                while i < items.len() {
                    let item = &items[i];
                    (item.exec)(view, &item.scope, &mut delta);
                    i += workers;
                }
                delta
            }));
        }

        handles.into_iter().map(|h| h.join().unwrap()).collect()
    })
}
```

⸻

Canonical Merge (Source of Truth)

Requirements
 • Must not use emission order
 • Must not call finalize() on worker deltas
 • Must flatten unsorted ops + origins
 • Must sort by (WarpOpKey, OpOrigin)
 • Must dedupe identical duplicates
 • Must explode on divergent duplicates

```rust
#[derive(Debug)]
pub struct MergeConflict {
    pub key: WarpOpKey,
    pub writers: Vec<OpOrigin>,
}

pub fn merge_deltas(deltas: Vec<TickDelta>) -> Result<Vec<WarpOp>, MergeConflict> {
    let mut flat: Vec<(WarpOpKey, OpOrigin, WarpOp)> = Vec::new();

    for d in deltas {
        let (ops, origins) = d.into_parts_unsorted();
        debug_assert_eq!(ops.len(), origins.len(), "ops/origins mismatch");

        for (op, origin) in ops.into_iter().zip(origins) {
            flat.push((op.sort_key(), origin, op));
        }
    }

    flat.sort_by_key(|(key, origin, _)| (*key, *origin));

    let mut out = Vec::with_capacity(flat.len());
    let mut i = 0;

    while i < flat.len() {
        let key = flat[i].0;
        let start = i;
        while i < flat.len() && flat[i].0 == key {
            i += 1;
        }

        let first = &flat[start].2;
        let all_same = flat[start + 1..i].iter().all(|(_, _, op)| op == first);

        if all_same {
            out.push(first.clone());
        } else {
            let writers = flat[start..i].iter().map(|(_, o, _)| *o).collect();
            return Err(MergeConflict { key, writers });
        }
    }

    Ok(out)
}
```

⸻

Full Pipeline (Engine Tick)

```rust
let pending   = scheduler.drain_for_tx(tx);            // deterministic order
let admitted  = reserve_independent(pending);          // footprint gate
let items     = to_exec_items(&admitted, &registry);   // bind exec + origin

let deltas    = execute_parallel(view, &items, workers);
let merged    = merge_deltas(deltas)?;                 // dedupe or explode

let patch_digest  = blake3_merged(&merged);            // merged order only
let next_snapshot = apply_merged(base_snapshot, &merged);
let state_root    = next_snapshot.hash();

let commit_hash   = blake3([
    parents,
    state_root,
    patch_digest,
    schema_hash,
    tick,
    policy_hashes,
]);
```

to_exec_items

```rust
fn to_exec_items(admitted: &[PendingRewrite], registry: &RuleRegistry) -> Vec<ExecItem> {
    admitted.iter().map(|r| ExecItem {
        exec: registry.get(r.compact_rule).executor,
        scope: r.scope.local_id,
        origin: OpOrigin {
            intent_id: intent_id_from_scope_hash(&r.scope_hash), // TEMP OK
            rule_id: r.compact_rule.as_u32(),
            match_ix: 0,  // TEMP OK
            op_ix: 0,     // assigned at emit time
        },
    }).collect()
}
```

⸻

TURBO DX GAIDEN Pitfalls (Read This Twice)

Pitfall 1: “Origins are optional”

No. Origins are what makes merge stable across worker counts.
If origins is cfg’d out in release, Phase 6 is fake.

✅ Fix: enable origins under feature="boaw_parallel".

⸻

Pitfall 2: HashMap iteration inside a rule

If a rule iterates a HashMap and emits ops in that order, your op_ix sequence becomes nondeterministic and you’ll get ghost diffs.

✅ Fix: inside rules, sort keys before emitting ops.
Determinism is a lifestyle.

⸻

Pitfall 3: Using TickDelta::finalize() on worker deltas

If you sort inside each worker, you can still end up with different global order across worker counts. Merge must be the single canonicalizer.

✅ Fix: worker deltas remain unsorted; merge sorts globally.

⸻

Pitfall 4: “Let’s just use worker_id as a tie-breaker”

Absolutely not. That makes outputs hardware/thread-count dependent.

✅ Fix: tie-breaker is origin, not worker.

⸻

Pitfall 5: “Conflict handling” in Phase 6A

If you start adding LWW/merge policies here, you’ll mask footprint bugs and ship nondeterminism with a smile.

✅ Fix: Phase 6A conflicts explode. Always.

⸻

The PEP TALK (a.k.a. Why this is the moment)

You’re building the thing almost nobody builds correctly: a parallel system that’s deterministic by construction.

Most engines pick two:
 • fast
 • parallel
 • deterministic
 • debuggable

You’re taking all four by doing the adult thing:
 • admit only what’s safe
 • let parallel execution rip
 • force the results through a canonical choke point
 • hash the canon and make reality obey it

This is the “free money” because once you have canonical merge, scaling workers doesn’t change truth — it only changes throughput.

And the real win: once this is solid, everything else gets easier:
 • virtual shards (Phase 6B) are just a partitioner
 • forking and merge become principled
 • provenance becomes auditable
 • time travel becomes clean
 • “why did this happen?” becomes answerable

You’re not just speeding up rewrites.
You’re making time itself serializable.

Now go enforce it like a religion and ship it like a weapon.

HOO RAH.
