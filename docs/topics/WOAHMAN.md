<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# The WoahMan Ledger

> Profound things noticed while reading Echo very, very closely.

This is a living companion to the deep dives in `docs/topics/`. Each entry is
one structural idea in the codebase that made a careful reader stop and say
_woah_ — not a feature, not a benchmark, but a place where the design changes
how you think about the problem. Entries are short on purpose: the deep dives
carry the mechanics; this ledger carries the sparks.

**Adding an entry:** one insight per entry, a few sentences, a code pointer,
and a link to the dive that unpacks it. If it needs a diagram, it belongs in a
deep dive; if it fits in a breath, it belongs here.

**Evidence rule:** claims rest on source code, never on comments or docs —
comments drift, code executes. Where an entry states behavior, it was read
from the executing path; anything weaker is labeled _inferred_ or
_comment-derived_ inline. Entry №18 records the one time this ledger trusted
a comment, and what the audit found.

Sources so far:
[The Causal WAL and WSC](wal-wsc/README.md) ·
[The SuperTick](supertick/README.md)

---

## Identity is content

### №1 — There is no queue anywhere

Echo's "inbox" is a `BTreeMap` keyed by `ingress_id = BLAKE3(payload)`. That
one decision quietly deletes three classic distributed-systems problems: even
_admission order_ is content order (two hosts that saw different arrival
orders admit identically), duplicates cannot exist (same bytes, same key),
and there is no side-channel queue whose contents could diverge from committed
history — because the pending work then materializes as ordinary graph nodes
that get snapshotted, hashed, and replayed like everything else.
→ `head_inbox.rs`, `engine_impl.rs` (`materialize_runtime_ingress_event`) ·
[SuperTick §4](supertick/README.md#4-from-intent-to-candidate)

### №2 — An intent is a node whose name is its content

An admitted intent isn't dispatched to a handler; it _becomes a graph node_
whose `NodeId` **is** its `ingress_id`, and dispatch is pattern matching on
the graph. Idempotence stops being a retry heuristic bolted on top and becomes
a consequence of naming. An event no rule matches isn't an error — it's inert,
replayable history.
→ `engine_impl.rs` · [SuperTick §4.2](supertick/README.md#42-the-intent-becomes-a-node)

### №3 — A storage locator is not causal identity

`WalSegmentRef::identity_digest()` deliberately **excludes** the storage path
from the hash. Identity is writer epoch + LSN range + digest chain + commit
anchors. The consequence is exact: a moved file is the same history; an edited
file is not. Most systems conflate "where the bytes live" with "what the bytes
mean" and pay for it at migration time. Echo priced them apart from day one.
→ `causal_wal.rs` · [WAL/WSC §6.2](wal-wsc/README.md#62-the-wal-projection-graph)

### №4 — The naming convention is machine-checked

WAL record kinds must end in `Recorded`, `Issued`, `Retained`, `Observed` —
never `Committed` — because records _record_ and only transactions _commit_.
That's not a style-guide plea; it's a function
(`obeys_recorded_not_committed_grammar()`) plus a schema linter. Grammar as a
tested invariant: the vocabulary cannot drift into overclaiming durability,
because the compiler-adjacent tooling would catch the lie.
→ `causal_wal.rs` · [WAL/WSC §3.2](wal-wsc/README.md#32-the-record-grammar)

---

## Order is the arbiter

### №5 — Determinism by canonical merge, not execution order

The parallel executor doesn't tame the race — it lets workers steal shards
through a bare atomic counter, genuinely racy — and then _erases the race_ by
sorting every emitted op by `(WarpOpKey, OpOrigin)` and deduplicating. Worker
identity, claim order, and finish order simply cease to exist in the output.
Determinism is not a property of how the code ran; it's a property of how the
results are folded.
→ `parallel/merge.rs` · [SuperTick §6.3](supertick/README.md#63-the-canonical-merge)

### №6 — Priority is order, order is content, nothing else votes

When two rewrites want the same resources, the winner is whichever comes first
in ascending `(scope_hash, rule_id, nonce)` — an order derived entirely from
content hashes. No thread, no arrival time, no wall clock, no map-iteration
luck gets a vote anywhere in the tick. "Priority" in Echo is not a knob; it is
a position in a hash-derived total order, which means it replays.
→ `scheduler.rs` · [SuperTick §5.2](supertick/README.md#52-check-then-mark-in-plan-order)

### №7 — Time becomes real only if the pass succeeds

`GlobalTick` is incremented _speculatively_ at the start of a SuperTick, used
to derive the run id and stamp every commit — but written back to the runtime
only after every head succeeds. A failed pass doesn't roll the clock back; the
clock never moved. The tick tense is subjunctive until the last head commits.
→ `coordinator.rs` (`super_tick`) · [SuperTick §3](supertick/README.md#3-the-pass)

---

## Judgment leaves a witness

### №8 — A rejection is history too

A footprint conflict is recorded as a _lawful decision_
(`RejectedFootprintConflict`), digested into the tick, and persisted through
the WAL. After a crash, recovery can **prove** an intent lost its resources
fairly — not merely fail to find it applied. Most systems can only witness
success; Echo witnesses defeat with the same rigor.
→ `receipt.rs`, `causal_wal.rs` (`WalTickDecision`) ·
[SuperTick §5.3](supertick/README.md#53-the-receipt--rejection-is-history)

### №9 — Losers get named blockers

The tick receipt's `blocked_by` list is a minimal causality witness: for each
rejected candidate, the exact indices of the earlier-accepted rewrites whose
footprints intersect it. Not just _that_ you lost — _to whom_, as a poset edge
list, auditable and replayable. Conflict arbitration usually evaporates the
moment it happens; here it fossilizes.
→ `engine_impl.rs` (`reserve_for_receipt`) ·
[SuperTick §5.3](supertick/README.md#53-the-receipt--rejection-is-history)

### №10 — Commit to replay, not narration

Commit hash v2 commits to `state_root` + `patch_digest` (+ parents + policy)
and deliberately **excludes** the plan, decision, and rewrites digests — they
ride along as diagnostics. The reasoning is a razor: if you can rebuild the
state and the delta, you can rebuild everything else; if you can't, no digest
of the story would save you. The story is kept; it's just not what the world
is named after.
→ `snapshot.rs` (`compute_commit_hash_v2`), `docs/spec/merkle-commit.md` ·
[SuperTick §7](supertick/README.md#7-sealing-the-tick)

---

## Trust nothing, verify twice

### №11 — The footprint is a contract with two enforcement points

A rewrite declares its read/write appetite _before_ running. The reserve gate
enforces it against other rewrites (generation-stamped set checks), and then
— in debug and enforce builds — a `FootprintGuard` enforces it against the
rewrite _itself_: touch anything outside your declaration and your delta is
poisoned. And if a merge conflict appears anyway, the engine doesn't resolve
it — it explodes, because disjoint-by-contract footprints producing divergent
writes means _the model lied_, and a lying model is a bug, not a case.
→ `footprint_guard.rs`, `parallel/merge.rs` ·
[SuperTick §6.2](supertick/README.md#62-workers)

### №12 — The tick mutates the world through its own replay artifact

Merged ops aren't applied by bespoke commit code — they're wrapped in a
`WarpTickPatchV1` and applied through `patch.apply_to_state`, the same path a
replayer uses. "What we did" and "what a replayer would do" are the same code,
so they cannot drift. The system eats its own dogfood _per tick, on the hot
path_.
→ `engine_impl.rs` (`apply_reserved_rewrites`) ·
[SuperTick §6.4](supertick/README.md#64-op-phase-ordering)

### №13 — A bundle is not trusted because it is a bundle

Importing a self-contained WSC export re-runs `recover_wal_segment_bytes` on
the embedded bytes: the same frame integrity, commit chain, and tail checks as
real filesystem recovery. Portability does not purchase credulity — evidence
is re-derived from bytes every time it crosses a boundary, and the DIND
convergence gate stands as a permanent witness that all paths agree.
→ `wsc/store.rs` · [WAL/WSC §6.4](wal-wsc/README.md#64-the-three-export-profiles)

### №14 — The format that _forbids_ evidence

Each WSC export profile marks evidence required, optional, or **forbidden** —
a ref-only bundle _may not_ embed segment bytes; a CAS-addressed bundle _may
not_ carry locators. Forbidding information is rarer and stranger than
requiring it, and it's the move that prevents undefined hybrids: a bundle
cannot half-claim self-containment and make a validator guess.
→ `wsc/store.rs` (`wsc_causal_history_export_profile`) ·
[WAL/WSC §6.4](wal-wsc/README.md#64-the-three-export-profiles)

---

## Crash is an input, not an exception

### №15 — One fsync, three semantics

Frames append unsynced; the single `sync_all()` on the commit disk-record
simultaneously (1) buys the transaction's durability, (2) defines the ACK
boundary the trusted host may return through, and (3) defines the exact
partition line recovery uses to separate history from tail. One syscall,
three load-bearing meanings — durability priced per _decision_, not per byte.
→ `causal_wal.rs` (`FilesystemWalStore::flush_commit`) ·
[WAL/WSC §3.8](wal-wsc/README.md#38-on-disk)

### №16 — Interruption and corruption are different types

A disk record cut off by EOF is a _torn tail_ — a typed posture
(`WouldTruncateAfter(lsn)` / `TruncatedAfter(lsn)`), expected, handled,
reported. A disk record whose digest fails is _corruption_ — an error, full
stop. Most log implementations blur these into one "bad file" path; Echo's
recovery treats being interrupted as a normal input to be classified and
being altered as a violation to be refused.
→ `causal_wal.rs` (`read_segment_bytes`) ·
[WAL/WSC §4.2](wal-wsc/README.md#42-tail-posture--the-crash-boundary-made-typed)

### №17 — The SuperTick is atomic against panics, not just errors

Every head commit runs inside `catch_unwind`. On a raw panic from rule code,
the coordinator rolls back receipt correlations, restores runtime _and_
provenance from checkpoints, records a scoped scheduler fault — and only
_then_ `resume_unwind`s the original panic. The panic still happens; it just
happens in a world where the half-tick already never existed.
→ `coordinator.rs` (`super_tick`) · [SuperTick §3](supertick/README.md#3-the-pass)

---

## Comments lie; code cannot

### №18 — The constant that claimed to be committed

This entry originally repeated a source comment: that `NUM_SHARDS = 256` is
"recorded in the commit hash domain via `compute_patch_digest_v2`." The code
refutes it — that function hashes the format version, policy id, rule-pack
id, commit status, slots, and ops, and nothing else (code-verified,
`tick_patch.rs#L812-L832@9d65a4b2`). What actually keeps sharding
non-semantic is №5: the canonical merge erases shard assignment from the
output, so no digest needs to commit to it (inferred, high confidence — the
merge's inputs carry no shard-derived values). The corrected _woah_ is
double: the architecture is so order-centric that a "frozen protocol
constant" turns out not to need freezing — and the ledger's own first
correction is a live demonstration of why claims here carry evidence labels.
→ `tick_patch.rs` (`compute_patch_digest_v2`), `parallel/merge.rs` ·
[SuperTick §6.1](supertick/README.md#61-work-units)

### №19 — Determinism has the same shape at every altitude

The state root hashes nodes in ascending id order with per-bucket sorted
edges. WSC serializes nodes in ascending id order with per-bucket sorted
edges. The plan drains in content-hash order; the merge sorts in op-key order;
the runnable set iterates in head-key order. One idea — _canonical order
derived from content_ — instantiated at six different layers, which is why the
fingerprint, the file, and the replay all agree by construction rather than by
reconciliation.
→ `snapshot.rs`, `wsc/build.rs`, `scheduler.rs`, `parallel/merge.rs`, `head.rs`
· both dives, everywhere

---

> ```text
> Woah.
> ```
