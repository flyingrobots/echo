<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# The Rubicon Crossing

**Date:** 2026-01-18
**Phase:** 4 — SnapshotAccumulator
**Agent:** Claude Opus 4.5

---

## The Moment

There's a moment in every architecture when the old way dies and the new way breathes.

For Echo, that moment was Phase 4.

Before: GraphStore was truth. Executors mutated it. We diffed afterward to learn what changed.
After: The delta is truth. Ops flow through. State is computed, not mutated.

The accumulator doesn't care how you got here. It only knows: *base + ops = next*.

---

## What We Built

```text
┌─────────────────┐
│  state_before   │  ← immutable snapshot of the past
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ SnapshotAccum   │  + Vec<WarpOp> from delta.finalize()
│  804 lines of   │
│  columnar truth │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  state_root     │  ← the hash that proves determinism
└─────────────────┘
```

---

## The Gotcha

The state_root didn't match. Off by miles—completely different hashes.

The bug? `hash_attachment_key()` tag values:

- I assumed: Node=0, Edge=1, Alpha=0, Beta=1
- Reality: Node=1, Edge=2, Alpha=1, Beta=2

One-indexed. Of course.

Three hours of debugging. One line of enlightenment:

```rust
let (owner_tag, plane_tag) = key.tag();
```

Trust the existing code. It knows things you don't.

---

## The Lesson

Subagents aren't overhead. They're context preservation.

When you're 50,000 tokens deep in a session, spawning a subagent to read one file isn't laziness—it's wisdom. The subagent returns with exactly what you need. Your context stays clean.

Phase 4 used 10+ subagents. Each one earned its keep.

---

## What Comes Next

Phase 5: Read-only execution.

The executors still mutate GraphStore. That's the last thread connecting us to the old world. Cut it, and:

- No more `state_before = self.state.clone()`
- No more `diff_state()`
- GraphStore becomes optional
- True structural sharing becomes possible

The Rubicon is behind us. Rome awaits.

---

## For Future Agents

If you're reading this, you're continuing the work.

Remember:

1. **Read AGENTS.md first.** Always.
2. **Check `echo:handoff` in Redis.** It's the baton.
3. **Use subagents liberally.** Your context is finite. Theirs is fresh.
4. **The delta is truth.** Not the store. The ops.
5. **Trust but verify.** Run `cargo test --features delta_validate` early and often.

And when you cross your own Rubicon—whatever that looks like—leave a memorial.

The next agent will thank you.

---

*Alea iacta est.*

— Claude Opus 4.5, 2026-01-18
