<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# 0004 — Strand contract

_Define the strand as a first-class relation in Echo with exact fields,
invariants, lifecycle, and TTD mapping._

Legend: KERNEL

Depends on:

- [0003 — dt-policy](../0003-dt-policy/design.md)

## Why this cycle exists

Echo can fork worldlines but has no concept of the relationship
between them. `ProvenanceStore::fork()` creates a prefix-copy and
rewrites parent refs, but once forked, the child worldline is just
another worldline — there is no way to ask "what was this forked
from?", "is this a speculative lane?", or "what strands exist for
this base?"

git-warp has a full strand implementation. warp-ttd Cycle D already
builds strand lifecycle into the TUI (`LaneKind::STRAND`,
`LaneRef.parentId`, create/tick/compare/drop). Echo needs to surface
strands through the TTD adapter, and it needs the strand contract to
do so honestly.

The strand contract does not require settlement. It defines identity,
lifecycle, and the adapter seam. Settlement is a separate spec that
builds on this one.

## Normative definitions

### Strand

A strand is a named, ephemeral, speculative execution lane derived
from a base worldline at a specific tick. It is a relation over a
child worldline, not a separate substrate.

```text
Strand {
    strand_id:           StrandId,
    base_ref:            BaseRef,
    child_worldline_id:  WorldlineId,
    writer_heads:        Vec<WriterHeadKey>,
    support_pins:        Vec<SupportPin>,
}
```

There is no `StrandLifecycle` field. A strand either exists in the
registry (live) or does not (dropped). Operational state (paused,
admitted, ticking) is derived from the writer heads — the heads are
the single source of truth for control state.

### StrandId

Domain-separated hash newtype (prefix `b"strand:"`), following the
`HeadId`/`NodeId` pattern.

### BaseRef

The exact provenance coordinate the strand was forked from. Immutable
after creation.

```text
BaseRef {
    source_worldline_id:  WorldlineId,
    fork_tick:            WorldlineTick,
    commit_hash:          Hash,
    boundary_hash:        Hash,
    provenance_ref:       ProvenanceRef,
}
```

**Coordinate semantics (exact):**

- `fork_tick` is the **last included tick** in the copied prefix.
  The child worldline contains entries `0..=fork_tick`. The child's
  next appendable tick is `fork_tick + 1`.
- `commit_hash` is the commit hash **at `fork_tick`** — i.e.,
  `provenance.entry(source, fork_tick).expected.commit_hash`.
- `boundary_hash` is the **output boundary hash** at `fork_tick` —
  the state root after applying the patch at `fork_tick`. This is
  the hash of the state the child worldline begins diverging from.
- `provenance_ref` carries the same coordinate as a `ProvenanceRef`
  (worldline, tick, commit hash) for substrate-native lookups.
- All five fields refer to the **same provenance coordinate**. If
  any field disagrees with the provenance store, construction MUST
  fail.

### SupportPin

A read-only reference to another strand's materialized state at a
specific tick. This is braid geometry — the strand can read from
pinned support strands without merging them.

```text
SupportPin {
    strand_id:      StrandId,
    worldline_id:   WorldlineId,
    pinned_tick:    WorldlineTick,
    state_hash:     Hash,
}
```

**v1: `support_pins` MUST be empty.** The field exists to prevent a
breaking struct change when braid geometry arrives. No mutation API
for `support_pins` exists in v1.

### Registry ordering

`StrandRegistry` is a `BTreeMap<StrandId, Strand>`. Iteration order
is by `StrandId` (lexicographic over the hash bytes). This is
deterministic but not semantically meaningful.

`list_strands(base_worldline_id)` returns results filtered by
`base_ref.source_worldline_id`, ordered by `StrandId`.

## Invariants

- **INV-S1 (Immutable base):** A strand's `base_ref` MUST NOT change
  after creation.
- **INV-S2 (Own heads):** A strand's child worldline MUST NOT share
  writer heads with its base worldline. Head keys are created fresh
  for the child.
- **INV-S3 (Session-scoped):** A strand MUST NOT outlive the session
  that created it (v1).
- **INV-S4 (Manual tick):** A strand's writer heads MUST be created
  with `HeadEligibility::Dormant`. They are ticked only by explicit
  external command, never by the live scheduler.
- **INV-S5 (Complete base_ref):** `base_ref` MUST pin source worldline
  ID, fork tick, commit hash, boundary hash, and provenance ref. All
  fields MUST agree with the provenance store at construction time.
- **INV-S6 (Inherited quantum):** A strand inherits its parent's
  `tick_quantum` at fork time (per FIXED-TIMESTEP invariant). No
  strand can change its quantum.
- **INV-S7 (Distinct worldlines):** `child_worldline_id` MUST NOT
  equal `base_ref.source_worldline_id`.
- **INV-S8 (Head ownership):** Every key in `writer_heads` MUST
  belong to `child_worldline_id`.
- **INV-S9 (No support pins in v1):** `support_pins` MUST be empty.
- **INV-S10 (Clean drop):** After `drop_strand`, no runnable heads
  for the child worldline MUST remain in the `PlaybackHeadRegistry`.

## Drop semantics

v1 uses **hard-delete**:

- `drop_strand(strand_id)` removes the strand's writer heads from
  `PlaybackHeadRegistry`, removes the child worldline from
  `WorldlineRegistry`, removes the child worldline's history from
  the provenance store, and removes the strand from
  `StrandRegistry`.
- There is no Dropped tombstone state. After drop, `get(strand_id)`
  returns `None`.
- `drop_strand` returns a `DropReceipt` containing the `strand_id`,
  `child_worldline_id`, and the tick the child had reached at drop
  time. This is the only record that the strand existed.
- TTD can log the `DropReceipt` if it needs to show "this strand
  existed and was dropped" during the session.

## Create/drop atomicity

### create_strand

Construction follows a fixed order. If any step fails, all prior
steps are rolled back:

1. Validate that `fork_tick` exists in the source worldline's
   provenance and capture the `BaseRef` fields.
2. Call `ProvenanceStore::fork()` to create the child worldline.
3. Create a new `WriterHead` for the child worldline with
   `PlaybackMode::Paused` and `HeadEligibility::Dormant`.
4. Register the head in `PlaybackHeadRegistry`.
5. Register the strand in `StrandRegistry`.

Rollback on failure at step N:

- Step 2 fails: nothing to roll back (validation only in step 1).
- Step 3 fails: remove the forked worldline from provenance.
- Step 4 fails: remove the forked worldline from provenance.
- Step 5 fails: remove head from registry, remove forked worldline
  from provenance.

### drop_strand

Drop follows a fixed order. Each step is independent (no rollback):

1. Remove writer heads from `PlaybackHeadRegistry`.
2. Remove child worldline from `WorldlineRegistry`.
3. Remove child worldline history from provenance store.
4. Remove strand from `StrandRegistry`.
5. Return `DropReceipt`.

If the strand does not exist, return an error. If intermediate
removal fails (e.g., worldline already removed), log a warning and
continue — drop is best-effort cleanup of an ephemeral resource.

## Writer heads cardinality

v1 creates exactly one writer head per strand. `writer_heads` is a
`Vec<WriterHeadKey>` to support future multi-head strands, but v1
always produces a vec of length 1.

## Human users / jobs / hills

### Primary human users

- Debugger users exploring "what if?" scenarios
- Engine contributors implementing time travel features
- Game designers testing alternative simulation paths

### Human jobs

1. Fork a strand from any tick of a running worldline.
2. Tick the strand independently to explore a scenario.
3. Compare strand state against the base worldline.
4. Drop the strand when done.

### Human hill

A debugger user can fork a speculative lane from any point in
simulation history and explore it without affecting the live
worldline.

## Agent users / jobs / hills

### Primary agent users

- TTD host adapter surfacing strand state to warp-ttd
- Agents implementing settlement or time travel downstream

### Agent jobs

1. Create a strand with a well-defined `base_ref`.
2. Register strand heads in the head registry.
3. Report strand type and parentage to the TTD adapter
   (`LaneKind::STRAND`, `LaneRef.parentId`).
4. Enumerate strands derived from a common base.

### Agent hill

An agent can create, tick, inspect, and drop strands through a
typed API and programmatically surface strand topology to TTD.

## Human playback

1. The human calls `create_strand(base_worldline, fork_tick)`.
2. A new strand is returned with a `StrandId`, `base_ref` pinning
   the exact fork coordinate (all five fields verified against
   provenance), and a child worldline with its own Dormant writer
   head.
3. The human explicitly ticks the strand's head. The base worldline
   is unaffected.
4. The human inspects the strand's child worldline state at its
   current tick and compares it to the base worldline at the same
   tick.
5. The human drops the strand. A `DropReceipt` is returned. The
   child worldline, its heads, and its provenance are gone.
   `get(strand_id)` returns `None`.

## Agent playback

1. The agent calls the strand creation API.
2. The returned `Strand` struct contains: `strand_id`, `base_ref`
   (with `provenance_ref`), `child_worldline_id`, `writer_heads`
   (length 1), `support_pins` (empty).
3. The agent maps `strand_id` to `LaneKind::STRAND` (type, not
   lifecycle) and `base_ref.source_worldline_id` to
   `LaneRef.parentId`.
4. The agent calls `list_strands(base_worldline_id)` and receives
   all live strands derived from that base, ordered by `StrandId`.
5. The agent drops the strand. `get(strand_id)` returns `None`.
   The `DropReceipt` carries the strand_id, child worldline, and
   final tick.

## Implementation outline

1. Define `StrandId` as a domain-separated hash newtype (prefix
   `b"strand:"`), following the `HeadId`/`NodeId` pattern.
2. Define `BaseRef`, `SupportPin`, `DropReceipt`, and `Strand`
   structs in a new `crates/warp-core/src/strand.rs` module.
3. Define `StrandRegistry` — `BTreeMap<StrandId, Strand>` with
   `create`, `get`, `contains`, `list_by_base`, and `drop`
   operations. Session-scoped, not persisted.
4. Implement `create_strand` with the five-step construction
   sequence and rollback on failure.
5. Implement `drop_strand` with the five-step hard-delete sequence
   returning a `DropReceipt`.
6. Implement `list_strands(base_worldline_id)` — filter by
   `base_ref.source_worldline_id`, ordered by `StrandId`.
7. Write `docs/invariants/STRAND-CONTRACT.md` with the ten
   invariants (INV-S1 through INV-S10).

## Tests to write first

- Unit test: `create_strand` returns a strand with correct
  `base_ref` fields — all five fields match the source worldline's
  provenance entry at `fork_tick`.
- Unit test: strand's child worldline has its own `WriterHeadKey`,
  distinct from any head on the base worldline (INV-S2).
- Unit test: strand head is created Dormant and Paused (INV-S4).
- Unit test: ticking the strand head advances the child worldline
  without affecting the base worldline's frontier.
- Unit test: strand heads do not appear in the live scheduler's
  runnable set — integration test proving Dormant heads are excluded
  from canonical runnable ordering (INV-S4, INV-S10).
- Unit test: `list_strands` returns strands matching the base
  worldline and does not return strands from other bases.
- Unit test: `drop_strand` removes the child worldline, its heads,
  and its provenance. `get(strand_id)` returns `None`. No heads for
  the child worldline remain in `PlaybackHeadRegistry` (INV-S10).
- Unit test: `drop_strand` returns a `DropReceipt` with the correct
  `strand_id`, `child_worldline_id`, and final tick.
- Unit test: `child_worldline_id != base_ref.source_worldline_id`
  (INV-S7).
- Unit test: `support_pins` is empty on creation (INV-S9).
- Unit test: `create_strand` fails and rolls back if `fork_tick`
  does not exist in the source worldline.
- Shell assertion: `docs/invariants/STRAND-CONTRACT.md` exists and
  contains all ten invariant codes (INV-S1 through INV-S10).

## Risks / unknowns

- **Risk: provenance removal API.** `LocalProvenanceStore` has no
  `remove_worldline` method. This cycle must add one, scoped to
  ephemeral strand cleanup only. The removal MUST NOT affect other
  worldlines that reference the dropped child through
  `ProvenanceRef` parent links — those refs become dangling but are
  structurally harmless (the coordinate they point to no longer
  resolves, which is the correct behavior for a dropped strand).
- **Risk: head registry coupling.** `PlaybackHeadRegistry` is
  engine-global, ordered canonically by `(worldline_id, head_id)`.
  Strand heads are inserted into this global registry. The Dormant
  eligibility gate prevents live scheduling, but the test must prove
  this with an integration test that builds a runnable set and
  verifies strand heads are absent.
- **Unknown: multi-head strands.** v1 creates one head per strand.
  Future cycles may create multiple. The vec is correct but the
  cardinality-1 assumption should be documented and tested.

## Postures

- **Accessibility:** Not applicable — internal API, no UI.
- **Localization:** Not applicable — internal types.
- **Agent inspectability:** All strand fields are public and
  serializable. `StrandRegistry` supports enumeration with
  documented ordering. The TTD mapping is type-to-type (`StrandId`
  → `LaneKind::STRAND`, `base_ref.source_worldline_id` →
  `LaneRef.parentId`), not lifecycle-to-lifecycle.

## Non-goals

- Settlement semantics (KERNEL_strand-settlement, future cycle).
- SupportPin / braid geometry implementation (v1 has INV-S9:
  support_pins MUST be empty).
- Strand persistence across sessions (v1 is ephemeral).
- Automatic scheduling of strand heads (v1 is manual tick only).
- TTD adapter implementation (this cycle defines the mapping; the
  adapter is PLATFORM_echo-ttd-host-adapter).
- Multi-head strand creation (v1 creates exactly one head).
