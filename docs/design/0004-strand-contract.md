<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# 0004 — Strand contract

_Define the strand as a first-class relation in Echo with exact fields,
invariants, fork-basis semantics, and TTD mapping._

Legend: KERNEL

Depends on:

- [0003 — dt-policy](./0003-dt-policy.md)

## Why this cycle exists

Echo can fork worldlines but has no concept of the relationship
between them. `ProvenanceStore::fork()` creates a prefix-copy and
rewrites parent refs, but once forked, the child worldline is just
another worldline — there is no way to ask "what was this forked
from?", "is this a speculative lane?", or "what strands exist for
this source lane?"

git-warp has a full strand implementation. warp-ttd Cycle D already
builds strand topology into the TUI (`LaneKind::STRAND`,
`LaneRef.parentId`, create/inspect/compare/drop). Echo needs to surface
strands through the TTD adapter, and it needs the strand contract to
do so honestly.

The strand contract does not require settlement in the same cycle. It
does need to stop lying about scope.

`git-warp` is currently ahead on speculative-lane richness. It already
has durable strands, braid-capable reads, comparison workflows, and a
clearer path toward settlement. Echo does not need to copy that engine
internally, but if parity is the real target then Echo's strand plan
must be read as a **bootstrap slice**, not the endpoint.

This cycle therefore has two jobs:

1. define the exact bootstrap contract Echo can land now
2. make explicit which additional capabilities are required before Echo
   can claim conceptual parity with `git-warp`

Settlement remains a separate spec, but the strand contract must leave
an honest path to settlement, braid geometry, and shared debugger
publication instead of freezing a too-small model as if it were final.

## Scope posture

This packet defines the **bootstrap strand contract** under the runtime
ontology now governed by
[0009 — Strand Runtime Graph Ontology](./0009-strand-runtime-graph-ontology.md).

That bootstrap is sufficient for:

- fork
- speculative authoring through ordinary ingress + `super_tick()`
- precise source-lane provenance
- TTD lane typing and parentage
- basic compare workflows

It is **not** sufficient for full parity with `git-warp`.

Parity requires follow-on work in at least four areas:

1. **Braid geometry**
    - support pins or equivalent multi-lane read composition
    - participating-lane publication
    - honest local plurality for debugger surfaces
2. **Settlement**
    - compare
    - plan
    - import
    - conflict artifact publication
3. **Retention policy**
    - session-scoped ephemerality is acceptable for bootstrap
    - it is not the only valid long-term model
4. **Shared observer/debugger publication**
    - neighborhood core
    - reintegration detail
    - receipt shell

Nothing in the bootstrap contract should block those later capabilities.

## Bootstrap normative definitions

### Strand

A strand is a named speculative execution lane rooted at one exact
admissible source-lane coordinate. It is a relation over a child
worldline, not a separate substrate.

```text
Strand {
    strand_id:           StrandId,
    fork_basis_ref:      ForkBasisRef,
    child_worldline_id:  WorldlineId,
    writer_heads:        Vec<WriterHeadKey>,
    support_pins:        Vec<SupportPin>,
}
```

There is no `StrandLifecycle` field. A strand either exists in the
registry (live) or does not (dropped). Its materialised state is
obtained solely from the child worldline frontier. Operational control
comes from ordinary writer-head eligibility and playback posture under
Echo's single `super_tick()` law, not from a strand-specific ticking
subsystem.

### StrandId

Domain-separated hash newtype (prefix `b"strand:"`), following the
`HeadId`/`NodeId` pattern.

### ForkBasisRef

The exact admissible coordinate the strand was forked from. Immutable
after creation.

```text
ForkBasisRef {
    source_lane_id:   LaneId,
    fork_tick:        WorldlineTick,
    commit_hash:      Hash,
    boundary_hash:    Hash,
    provenance_ref:   ProvenanceRef,
}
```

**Coordinate semantics (exact):**

- `source_lane_id` names the precise source lane. In the first runtime
  cut this may point to a canonical worldline or to a speculative lane
  resolved at one exact coordinate.
- `fork_tick` is the **last included tick** in the copied prefix. The
  child worldline contains entries `0..=fork_tick`. The child's next
  appendable tick is `fork_tick + 1`.
- `commit_hash` is the commit hash **at `fork_tick`** for the source
  coordinate.
- `boundary_hash` is the **output boundary hash** at `fork_tick` —
  the state root after applying the patch at `fork_tick`. This is the
  hash of the state the child worldline begins diverging from.
- `provenance_ref` carries the same coordinate as a `ProvenanceRef`
  for substrate-native lookups.
- All five fields refer to the **same admissible coordinate**. If any
  field disagrees with the provenance store, construction MUST fail.

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

**Bootstrap note:** the first strand cut left `support_pins` empty to avoid
premature braid semantics. That restriction is superseded by
[0007 — Braid geometry and neighborhood publication](./0007-braid-geometry-and-neighborhood-publication.md),
which permits validated non-empty support pins as read geometry and
publication inputs. Under `0009`, however, support pins remain derived /
cache truth in the first cut rather than authoritative graph ontology.

If Echo stopped at the empty-only posture, it would still lag `git-warp` on
braid-capable speculative work.

### Registry ordering

`StrandRegistry` is a `BTreeMap<StrandId, Strand>`. Iteration order
is by `StrandId` (lexicographic over the hash bytes). This is
deterministic but not semantically meaningful.

`list_strands_by_source_lane(source_lane_id)` returns results filtered by
`fork_basis_ref.source_lane_id`, ordered by `StrandId`.

## Invariants

- **INV-S1 (Immutable fork basis):** A strand's `fork_basis_ref` MUST NOT
  change after creation.
- **INV-S2 (Own heads):** A strand's child worldline MUST NOT share
  writer heads with its source lane. Head keys are created fresh for
  the child worldline.
- **INV-S3 (Bootstrap session scope):** A strand MUST NOT outlive the
  session that created it in the bootstrap landing. Long-term retention
  policy remains an explicit follow-on design axis, not a semantic truth
  about what a strand is.
- **INV-S4 (Single tick law):** A strand advances only through ordinary
  intent admission under Echo's global `super_tick()` path. No
  strand-specific tick path is authoritative.
- **INV-S5 (Complete fork basis):** `fork_basis_ref` MUST pin source lane
  ID, fork tick, commit hash, boundary hash, and provenance ref. All
  fields MUST agree with the provenance store at construction time.
- **INV-S6 (Inherited quantum):** A strand inherits its parent's
  `tick_quantum` at fork time (per FIXED-TIMESTEP invariant). No
  strand can change its quantum.
- **INV-S7 (Distinct worldlines):** `child_worldline_id` MUST NOT
  equal the source-basis carrier. A strand is always represented by a
  distinct child worldline, even when its source lane was itself
  speculative.
- **INV-S8 (Head ownership):** Every key in `writer_heads` MUST
  belong to `child_worldline_id`.
- **INV-S9 (Validated support pins):** support pins, when present, MUST be
  live, correctly resolved, non-duplicated, and read-only.
- **INV-S10 (Clean drop):** After `drop_strand`, no runnable heads
  for the child worldline MUST remain in the `PlaybackHeadRegistry`.

## Bootstrap drop semantics

The bootstrap landing uses **hard-delete**:

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

## Bootstrap create/drop atomicity

### create_strand

Construction follows a fixed order. If any step fails, all prior
steps are rolled back:

1. Validate that the requested source lane / tick coordinate is admissible
   and capture the `ForkBasisRef` fields.
2. Call `ProvenanceStore::fork()` to create the child worldline.
3. Create one or more new `WriterHead`s for the child worldline under the
   ordinary writer-head control model.
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

## Bootstrap writer-head cardinality

v1 creates exactly one writer head per strand. `writer_heads` is a
`Vec<WriterHeadKey>` to support future multi-head strands, but v1
always produces a vec of length 1.

Again, this is a bootstrap constraint, not a statement that a strand is
inherently single-head forever.

## Parity target that bootstrap must not block

### 1. Strands must grow beyond singleton publication

The bootstrap contract is good enough to say:

- this is a speculative lane
- it came from this source basis coordinate
- it has its own writer head

That is not enough for mature debugger or comparison work.

For parity, Echo eventually needs a first-class local-site publication
path that can say:

- which lanes participate in the current local site
- whether the site is singleton or plural
- what the nearby alternatives are

The current strand contract should therefore be read as the minimal lane
identity foundation, not the whole neighborhood story.

### 2. Braid geometry is required for parity

`git-warp` already treats braid as a real composite read presentation.

Echo does not need to implement the full final braid model in this
cycle, but it does need to keep the door open for:

- read-only support overlays
- explicit support-pin mutation APIs
- participating-lane publication
- observer/debugger surfaces that can inspect more than one lane at a
  local site

The old bootstrap posture of keeping `support_pins` empty was acceptable only
because the next cycle made braid geometry real. That follow-on now exists as
[0007 — Braid geometry and neighborhood publication](./0007-braid-geometry-and-neighborhood-publication.md).

### 3. Settlement must remain separate from braid

The current split is correct:

- braid is geometry
- settlement is history/import/conflict law

But parity requires both, not just one.

Echo needs:

- braid-capable speculative reads
- compare / plan / import / conflict artifacts

That is why `KERNEL_strand-settlement` remains required even after this
packet lands.

### 4. Retention must become policy, not essence

Session-scoped strands are a sensible bootstrap safety posture.

They should not harden into the theory as if "strands are ephemeral"
were an essential truth. For parity with `git-warp`, Echo needs a
clearer retention policy axis:

- session-scoped
- lease-scoped
- or durable

The current bootstrap may choose the first. The type/theory should not
pretend the others are impossible.

The same distinction applies to debugger work. A TTD observation does
not itself create a strand. An explicit "continue from here" or "what
if?" action does. In Echo v1, that debugger-created strand should be
understood as session-scoped scratch by default. Durable author-only
retention is a later policy extension, not something observation gets
for free.

### 5. Shared debugger publication must become explicit

Even after bootstrap strands land, Echo will still not be aligned if the
host adapter has to invent:

- neighborhood core
- reintegration detail
- receipt shell

Strands therefore need to feed the later Continuum-aligned publication
boundaries, not remain only an Echo-local kernel feature.

## Human users / jobs / hills

### Primary human users

- Debugger users exploring "what if?" scenarios
- Engine contributors implementing time travel features
- Game designers testing alternative simulation paths

### Human jobs

1. Fork a strand from any admissible lane/tick coordinate.
2. Author speculative intents against the child worldline.
3. Let `super_tick()` advance the strand through the ordinary scheduler.
4. Compare strand state against its source basis.
5. Drop the strand when done.

### Human hill

A debugger user can fork a speculative lane from any admissible point in
simulation history and explore it without affecting the source lane.

## Agent users / jobs / hills

### Primary agent users

- TTD host adapter surfacing strand state to warp-ttd
- Agents implementing settlement or time travel downstream

### Agent jobs

1. Create a strand with a well-defined `fork_basis_ref`.
2. Register strand heads in the head registry.
3. Report strand type and parentage to the TTD adapter
   (`LaneKind::STRAND`, `LaneRef.parentId`).
4. Enumerate strands derived from a common source lane.

### Agent hill

An agent can create, inspect, and drop strands through a typed API,
author intents through ordinary ingress, and programmatically surface
strand topology to TTD.

## Human playback

1. The human calls `create_strand(source_lane, fork_tick)`.
2. A new strand is returned with a `StrandId`, `fork_basis_ref`
   pinning the exact fork coordinate (all five fields verified against
   provenance), and a child worldline with its own writer head.
3. The human submits speculative intents for the child worldline. The
   source lane is unaffected.
4. Echo advances the runtime through `super_tick()`. The strand moves
   only because the ordinary scheduler admitted those intents.
5. The human inspects the strand's child worldline state at its
   current tick and compares it to the source basis.
6. The human drops the strand. A `DropReceipt` is returned. The
   child worldline, its heads, and its provenance are gone.
   `get(strand_id)` returns `None`.

## Agent playback

1. The agent calls the strand creation API.
2. The returned `Strand` struct contains: `strand_id`, `fork_basis_ref`
   (with `provenance_ref`), `child_worldline_id`, `writer_heads`
   (length 1), `support_pins` (empty until explicitly pinned through braid
   geometry APIs).
3. The agent maps `strand_id` to `LaneKind::STRAND` (type, not
   lifecycle) and `fork_basis_ref.source_lane_id` to
   `LaneRef.parentId`.
4. The agent calls `list_strands_by_source_lane(source_lane_id)` and
   receives all live strands derived from that source lane, ordered by
   `StrandId`.
5. The agent drops the strand. `get(strand_id)` returns `None`.
   The `DropReceipt` carries the strand_id, child worldline, and
   final tick.

## Bootstrap implementation outline

1. Define `StrandId` as a domain-separated hash newtype (prefix
   `b"strand:"`), following the `HeadId`/`NodeId` pattern.
2. Define `ForkBasisRef`, `SupportPin`, `DropReceipt`, and `Strand`
   structs in a new `crates/warp-core/src/strand.rs` module.
3. Define `StrandRegistry` — `BTreeMap<StrandId, Strand>` with
   `create`, `get`, `contains`, `list_by_source_lane`, and `drop`
   operations. Session-scoped, not persisted.
4. Implement `create_strand` with the five-step construction
   sequence and rollback on failure.
5. Implement `drop_strand` with the five-step hard-delete sequence
   returning a `DropReceipt`.
6. Implement `list_strands_by_source_lane(source_lane_id)` — filter by
   `fork_basis_ref.source_lane_id`, ordered by `StrandId`.
7. Write `docs/invariants/STRAND-CONTRACT.md` with the ten
   invariants (INV-S1 through INV-S10).

## Required follow-on work for parity

This packet is only correct if the next queue makes the missing
capabilities explicit.

Required follow-ons:

1. braid geometry and neighborhood publication
2. settlement / compare / import / conflict artifacts
3. retention and capability policy for timeline mutation
4. Continuum/Wesley publication of strand-facing shared observer nouns

## Tests to write first

- Unit test: `create_strand` returns a strand with correct
  `fork_basis_ref` fields — all five fields match the source
  coordinate at `fork_tick`.
- Unit test: strand's child worldline has its own `WriterHeadKey`,
  distinct from any head on the source lane (INV-S2).
- Unit test: the only way to advance a strand is through ordinary
  ingress + `super_tick()` (INV-S4).
- Unit test: strand-authored intents advance the child worldline
  without affecting the source lane frontier.
- Unit test: `list_strands_by_source_lane` returns strands matching the
  source lane and does not return strands from other source lanes.
- Unit test: `drop_strand` removes the child worldline, its heads,
  and its provenance. `get(strand_id)` returns `None`. No heads for
  the child worldline remain in `PlaybackHeadRegistry` (INV-S10).
- Unit test: `drop_strand` returns a `DropReceipt` with the correct
  `strand_id`, `child_worldline_id`, and final tick.
- Unit test: `child_worldline_id` is distinct from the source lane's
  live worldline frontier carrier (INV-S7).
- Unit test: new strands start with `support_pins` empty until explicitly
  pinned.
- Unit test: support pins validate live targets, reject duplicates/self pins,
  and remain read-only (INV-S9).
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
  Strand heads are inserted into this global registry. The risk is no
  longer accidental live scheduling through a strand-specific gate; it
  is accidental over-capability. Tests must prove that strand heads can
  author only for their child worldline and participate in scheduling
  only through ordinary ingress + `super_tick()`.
- **Unknown: multi-head strands.** v1 creates one head per strand.
  Future cycles may create multiple. The vec is correct but the
  cardinality-1 assumption should be documented and tested.
- **Unknown: retention posture beyond bootstrap.** Session-scoped
  deletion is simple, but parity with `git-warp` may eventually require
  explicit durable or lease-scoped strands. That decision should be made
  as policy, not smuggled in as type essence.
- **Unknown: braid publication shape.** `support_pins` is enough to
  avoid a breaking struct rewrite, but not enough to define how plural
  local sites should publish into shared debugger contracts.

## Postures

- **Accessibility:** Not applicable — internal API, no UI.
- **Localization:** Not applicable — internal types.
- **Agent inspectability:** All strand fields are public and
  serializable. `StrandRegistry` supports enumeration with
  documented ordering. The TTD mapping is type-to-type (`StrandId`
  → `LaneKind::STRAND`, `fork_basis_ref.source_lane_id` →
  `LaneRef.parentId`), not lifecycle-to-lifecycle.

## Non-goals

- Settlement semantics (KERNEL_strand-settlement, future cycle).
- Full braid geometry implementation in this cycle. Bootstrap alone left
  `support_pins` empty, but that posture was not the endpoint.
- Strand persistence across sessions (v1 is ephemeral).
- Any strand-specific execution model separate from ordinary ingress +
  `super_tick()`.
- TTD adapter implementation (this cycle defines the mapping; the
  adapter is PLATFORM_echo-ttd-host-adapter).
- Multi-head strand creation (v1 creates exactly one head).
