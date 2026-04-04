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
    primary_heads:       Vec<WriterHeadKey>,
    support_pins:        Vec<SupportPin>,
    lifecycle:           StrandLifecycle,
}
```

### BaseRef

The exact coordinate the strand was forked from. Immutable after
creation.

```text
BaseRef {
    source_worldline_id:  WorldlineId,
    fork_tick:            WorldlineTick,
    commit_hash:          Hash,
    boundary_hash:        Hash,
}
```

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

### StrandLifecycle

```text
Created → Active → Dropped
```

No persistence across sessions in v1. A strand is created, ticked,
compared, and dropped within a single session.

## Invariants

- **INV-S1 (Immutable base):** A strand's `base_ref` MUST NOT change
  after creation.
- **INV-S2 (Own heads):** A strand's child worldline MUST NOT share
  writer heads with its base worldline. Head keys are created fresh
  for the child.
- **INV-S3 (Session-scoped):** A strand MUST NOT outlive the session
  that created it (v1).
- **INV-S4 (Manual tick):** A strand's child worldline MUST be ticked
  only by explicit external command, never by the live scheduler.
  Strand heads are created Dormant or Paused.
- **INV-S5 (Complete base_ref):** `base_ref` MUST pin source worldline
  ID, fork tick, commit hash, and boundary hash.
- **INV-S6 (Inherited quantum):** A strand inherits its parent's
  `tick_quantum` at fork time (per FIXED-TIMESTEP invariant). No
  strand can change its quantum.

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
3. Report strand lifecycle to the TTD adapter.
4. Enumerate strands derived from a common base.

### Agent hill

An agent can create, tick, inspect, and drop strands through a
typed API and programmatically surface strand topology to TTD.

## Human playback

1. The human calls `create_strand(base_worldline, fork_tick)`.
2. A new strand is returned with a `StrandId`, `base_ref` pinning
   the exact fork coordinate, and a child worldline with its own
   Dormant writer head.
3. The human explicitly ticks the strand's head. The base worldline
   is unaffected.
4. The human inspects the strand's child worldline state at its
   current tick and compares it to the base worldline at the same
   tick.
5. The human drops the strand. The child worldline and its heads
   are removed.

## Agent playback

1. The agent calls the strand creation API.
2. The returned `Strand` struct contains all fields from the
   contract: `strand_id`, `base_ref`, `child_worldline_id`,
   `primary_heads`, `support_pins`, `lifecycle`.
3. The agent maps `strand_id` to `LaneKind::STRAND` and
   `base_ref.source_worldline_id` to `LaneRef.parentId` for the
   TTD adapter.
4. The agent calls `list_strands(base_worldline_id)` and receives
   all strands derived from that base.
5. The agent drops the strand. The lifecycle transitions to Dropped.

## Implementation outline

1. Define `StrandId` as a domain-separated hash newtype (prefix
   `b"strand:"`), following the `HeadId`/`NodeId` pattern.
2. Define `BaseRef`, `SupportPin`, `StrandLifecycle`, and `Strand`
   structs in a new `crates/warp-core/src/strand.rs` module.
3. Define `StrandRegistry` — a `BTreeMap<StrandId, Strand>` with
   create, get, list-by-base, and drop operations. Session-scoped,
   not persisted.
4. Implement `create_strand(base_worldline, fork_tick)`:
    - Call `ProvenanceStore::fork()` to create the child worldline.
    - Capture `base_ref` from the source worldline's provenance at
      `fork_tick`.
    - Create a new `WriterHead` for the child worldline with
      `PlaybackMode::Paused` and `HeadEligibility::Dormant`.
    - Register the head in the `PlaybackHeadRegistry`.
    - Register the strand in the `StrandRegistry`.
    - Return the `Strand`.
5. Implement `drop_strand(strand_id)`:
    - Remove the strand's heads from the head registry.
    - Remove the child worldline from the worldline registry.
    - Remove the child worldline's provenance.
    - Transition lifecycle to Dropped.
    - Remove from strand registry.
6. Implement `list_strands(base_worldline_id)` — filter the strand
   registry by `base_ref.source_worldline_id`.
7. Write the invariant document `docs/invariants/STRAND-CONTRACT.md`
   with the six invariants.

## Tests to write first

- Unit test: `create_strand` returns a strand with correct
  `base_ref` fields (source worldline, fork tick, commit hash,
  boundary hash).
- Unit test: strand's child worldline has its own `WriterHeadKey`,
  distinct from any head on the base worldline.
- Unit test: strand head is created Dormant and Paused.
- Unit test: ticking the strand head advances the child worldline
  without affecting the base worldline's frontier.
- Unit test: `list_strands` returns strands matching the base
  worldline and does not return strands from other bases.
- Unit test: `drop_strand` removes the child worldline, its heads,
  and its provenance. Subsequent `list_strands` does not include it.
- Unit test: `base_ref` is immutable — no API allows changing it
  after creation.
- Shell assertion: `docs/invariants/STRAND-CONTRACT.md` exists and
  contains all six invariant codes (INV-S1 through INV-S6).

## Risks / unknowns

- **Risk: provenance cleanup on drop.** `LocalProvenanceStore` has
  no `remove_worldline` method. We may need to add one, or rely on
  the session-scoped lifetime (the whole store is dropped with the
  session). If adding removal, it must not violate append-only
  invariants for other worldlines that reference the dropped one.
- **Risk: head registry coupling.** `PlaybackHeadRegistry` is
  currently engine-global. Strand heads must not accidentally
  participate in the live scheduler. The Dormant eligibility gate
  should prevent this, but the test must prove it.
- **Unknown: SupportPin implementation.** The `support_pins` field
  is part of the contract but braid geometry (pinning read-only
  support overlays) is deferred to a future cycle. v1 strands have
  empty `support_pins`. The field exists to prevent a breaking
  struct change later.

## Postures

- **Accessibility:** Not applicable — internal API, no UI.
- **Localization:** Not applicable — internal types.
- **Agent inspectability:** All strand fields are public and
  serializable. `StrandRegistry` supports enumeration. The TTD
  adapter mapping is documented.

## Non-goals

- Settlement semantics (KERNEL_strand-settlement, future cycle).
- SupportPin / braid geometry implementation (v1 strands have no
  support pins).
- Strand persistence across sessions (v1 is ephemeral).
- Automatic scheduling of strand heads (v1 is manual tick only).
- TTD adapter implementation (this cycle defines the mapping; the
  adapter is PLATFORM_echo-ttd-host-adapter).
