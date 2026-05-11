<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# 0009 - Strand Runtime Graph Ontology

Cycle: `0009-strand-runtime-graph-ontology`  
Legend: `KERNEL`  
Source backlog item: `docs/method/backlog/up-next/KERNEL_strand-runtime-graph-ontology.md`

## Sponsors

- Human: Runtime / ontology architect
- Agent: Kernel implementation agent

## Hill

Echo gets one explicit graph-native control ontology for strands. A
strand becomes a relation object naming a child worldline, an exact
fork basis, and authorised writer heads. Its current state is obtained
only from the child worldline frontier, and future braid/support
geometry has an honest place to live without reintroducing a second
execution model.

## Playback Questions

### Human

- [ ] If we inspect the runtime graph, can we point to explicit nodes
      and edges that answer what a strand is, what it was forked from,
      which worldline currently carries its state, and which heads may
      write it?
- [ ] Does the graph make it impossible to mistake a strand for a
      private mutable store or frozen snapshot?

### Agent

- [ ] Can I derive the current strand state by following
      `strand -> child_worldline -> current_portal -> Descend(warp)`
      with no strand-local shadow store or side channel?
- [ ] Are support-pin and braid publication objects explicitly marked
      as derived / non-authoritative in the first cut?

## Accessibility and Assistive Reading

- Linear truth / reduced-complexity posture: the packet should answer
  three questions in order: where strand state lives, what graph nouns
  are authoritative, and what remains derived.
- Non-visual or alternate-reading expectations: each node and edge name
  should carry a single stable meaning so a reader can reconstruct the
  ontology from text alone.

## Localization and Directionality

- Locale / wording / formatting assumptions: type labels remain
  ASCII-first, domain-separated, and stable enough to appear in code,
  docs, tests, and generated debugger output.
- Logical direction / layout assumptions: the packet describes graph
  structure and rewrite posture only; no UI layout or visual debugger
  presentation is in scope.

## Agent Inspectability and Explainability

- What must be explicit and deterministic for agents: the authoritative
  node types, authoritative edge types, payload shapes, and the exact
  traversal that yields a strand's current state.
- What must be attributable, evidenced, or governed: any support-pin or
  braid publication object must be identifiable as cache / derived
  publication rather than canonical truth.

## Why this cycle exists

The bootstrap strand contract was useful because it named a relation
between a parent provenance coordinate and a child worldline. It also
smuggled in a bogus second execution model: strands were described as
manual, dormant, strand-specific lanes instead of ordinary participants
in Echo's single `SuperTick` law.

That drift matters because the runtime already has the correct kernel
shape:

- one mutable frontier per worldline
- immutable graph truth beneath that frontier
- content-addressed ingress
- deterministic scheduler admission
- one `super_tick()` path for state change

Until the runtime graph says explicitly where strand state lives and
which objects are authoritative, `0007` and `0008` remain partly
folkloric. We need one graph ontology that tells the truth.

Where older packets conflict on runtime-control ontology, naming, or
first-cut authority claims, this packet governs.

## Design decision

Echo should materialise strand runtime ontology in a dedicated control
warp and make the following objects authoritative in the first cut:

1. worldline control nodes
2. one current-portal node per live worldline
3. strand relation nodes
4. fork-basis nodes
5. writer-head control nodes

Support pins and braid views may also be represented in the graph, but
in the first cut they remain derived / cache publication objects rather
than canonical truth.

## Core laws

### 1. State locality law

A strand's current materialised state lives only on the child
worldline's current frontier.

The strand node itself does not carry mutable current state, overlay
truth, or a descended current graph.

### 2. Relation law

A strand is a relation object.

It names:

- one child worldline
- one precise fork basis
- one or more authorised writer heads

It is not a second scheduler, a private mutable store, or a snapshot
container.

### 3. Fork basis law

A strand must be rooted at one exact parent coordinate, not at an
ambient parent tip clone. The source coordinate may come from any
admissible causal lane and any admissible tick within that lane, not
only from the current frontier.

That basis must remain explicitly recoverable from graph truth.

Examples that must remain lawful include:

- forking from a canonical worldline frontier
- forking from a historical coordinate in that worldline
- forking from a speculative lane at one exact earlier coordinate

What matters is not frontier-ness. What matters is that the basis is one
precise admissible coordinate.

### 4. Head capability law

Writer heads attached to a strand remain ordinary writer heads under the
same control law as every other head.

They may author work only for the child worldline they are authorised to
serve unless a future cross-lane mechanism is introduced explicitly.

### 5. Control warp law

Runtime control ontology should live in a dedicated control warp rather
than polluting arbitrary application graphs.

### 6. Derived braid law

Support-pin geometry and braid publication may be represented in the
runtime graph, but in the first cut they are not authoritative stores of
semantic truth.

## Runtime control warp

This packet assumes a dedicated runtime-control warp instance, for
example:

```text
warp:sys/runtime
```

The runtime-control warp contains lane, strand, fork-basis, and
writer-head ontology. Application / domain graphs remain in their own
materialised warp instances and are reached through explicit portal /
descend structure.

## Authoritative node types

### `sys/runtime`

The root control node for the runtime-control warp.

This node anchors the live control ontology and owns edges to currently
live worldlines, strands, and heads.

### `sys/worldline`

A live worldline control noun.

Alpha attachment type:

```text
payload:sys/worldline_meta/v1
```

Minimum payload:

```text
WorldlineMeta {
    worldline_id:   WorldlineId,
    frontier_tick:  WorldlineTick,
    tick_quantum:   WorldlineTick,
}
```

This node is authoritative for the identity of the worldline and its
current frontier coordinate in the runtime.

### `sys/worldline/current_portal`

A dedicated portal node whose alpha attachment is:

```text
Descend(WarpId)
```

This node exists because one attachment slot cannot simultaneously carry
both structured atom payload and a descended warp target. Splitting the
portal into its own node keeps the graph honest.

### `sys/strand`

A strand relation node.

Alpha attachment type:

```text
payload:sys/strand_meta/v1
```

Minimum payload:

```text
StrandMeta {
    strand_id: StrandId,
}
```

The strand node is intentionally small. Its semantics come primarily
from edges to the child worldline, fork basis, and authorised heads.

### `sys/fork_basis`

A first-class basis object for strand origin.

Alpha attachment type:

```text
payload:sys/fork_basis/v1
```

Minimum payload:

```text
ForkBasisMeta {
    basis_ref: ForkBasisRef,
}
```

`ForkBasisRef` names one exact admissible coordinate in any causal
lane. It supersedes the older worldline-only fork-basis naming.

This node reifies the exact source lane relation, fork tick, commit
hash, boundary hash, and provenance reference used to create the
strand. The basis object must be able to refer to a historical interior
coordinate in either a canonical worldline or a speculative strand, not
only a live frontier.

### `sys/head/writer`

A writer-head control node.

Alpha attachment type:

```text
payload:sys/writer_head_meta/v1
```

Minimum payload:

```text
WriterHeadMeta {
    head_key:         WriterHeadKey,
    eligibility:      HeadEligibility,
    playback_mode:    PlaybackMode,
    public_inbox:     Option<InboxAddress>,
    is_default_writer: bool,
}
```

This node makes head control state graph-visible instead of hiding it in
scattered registries.

## Authoritative edge types

### `edge:runtime/worldline`

```text
sys/runtime -> sys/worldline
```

Meaning: this worldline is live in the current runtime.

Cardinality: zero or more from the runtime root.

### `edge:runtime/strand`

```text
sys/runtime -> sys/strand
```

Meaning: this strand relation object is live in the current runtime.

Cardinality: zero or more from the runtime root.

### `edge:runtime/head`

```text
sys/runtime -> sys/head/writer
```

Meaning: this writer head is live in the current runtime.

Cardinality: zero or more from the runtime root.

### `edge:worldline/current`

```text
sys/worldline -> sys/worldline/current_portal
```

Meaning: this edge identifies the worldline's current materialised state.

Cardinality: exactly one outgoing edge from each live `sys/worldline`
node.

### `edge:strand/child_worldline`

```text
sys/strand -> sys/worldline
```

Meaning: this strand's live state is carried by that child worldline.

Cardinality: exactly one outgoing edge from each `sys/strand` node.

### `edge:strand/fork_basis`

```text
sys/strand -> sys/fork_basis
```

Meaning: this strand is rooted at that exact fork basis.

Cardinality: exactly one outgoing edge from each `sys/strand` node.

### `edge:fork_basis/source_lane`

```text
sys/fork_basis -> (sys/worldline | sys/strand)
```

Meaning: this fork basis refers to the exact source causal lane from
which the strand was created. The target may be a canonical worldline or
a speculative strand.

Cardinality: exactly one outgoing edge from each `sys/fork_basis` node.

### `edge:strand/authorized_head`

```text
sys/strand -> sys/head/writer
```

Meaning: the target writer head is authorised to submit work for this
strand's child worldline.

Cardinality: one or more outgoing edges from each `sys/strand` node.

Validation rule: every authorised head must target the same child
worldline named by `edge:strand/child_worldline`.

## Authoritative traversal contracts

### Current strand state

The current strand materialisation is obtained by following:

```text
sys/strand
  -> edge:strand/child_worldline
sys/worldline
  -> edge:worldline/current
sys/worldline/current_portal
  -> Descend(WarpId)
```

That descended warp is the current graph state for the strand's child
worldline.

### Source basis

The source basis of a strand is obtained by following:

```text
sys/strand
  -> edge:strand/fork_basis
sys/fork_basis
  -> edge:fork_basis/source_lane
```

### Authorised heads

The writer heads allowed to author child-lane work are obtained by
following:

```text
sys/strand
  -> edge:strand/authorized_head
```

## Derived / cache graph objects

The following objects may exist in the runtime graph, but in the first
cut they are not authoritative truth.

### Support pins

Support pins may be represented as:

```text
edge:strand/support_pin
```

from one `sys/strand` node to another, with beta attachment type:

```text
payload:sys/support_pin/v1
```

Minimum payload:

```text
SupportPinMeta {
    pinned_tick: WorldlineTick,
    state_hash:  Hash,
}
```

First-cut status:

- recomputable
- versioned
- safe to drop and rebuild
- not the sole authority for braid semantics

### Braid publication

Braid publication may later use derived nodes such as:

- `sys/braid_view`
- `sys/braid_cell`

with edges such as:

- `edge:braid/basis`
- `edge:braid/participant`
- `edge:braid/cell`

But this packet does not make those objects authoritative.

## Rewrite and mutation posture

This packet describes ontology, not a second execution engine.

The intended operational posture is:

- strand creation may remain a bootstrap helper in this cycle so long as
  it produces truthful graph objects and an exact `ForkBasis`
- the creation surface must accept any admissible lane/tick coordinate as
  fork input, not just current frontier coordinates
- steady-state lane evolution still occurs only through ordinary intent
  admission under `super_tick()`
- no `tick_strand()` API or equivalent side channel should exist after
  this cycle is implemented
- support-pin and braid publication objects may be rebuilt from
  authoritative runtime truth

## Non-goals

- [ ] Make support pins authoritative semantic truth in this cycle.
- [ ] Introduce a large persisted braid ontology before we have honest
      strand state locality.
- [ ] Make fork creation itself a tick-admitted causal event in this
      cycle.
- [ ] Solve settlement recursion here; `0008` remains the history-law
      follow-on.
- [ ] Replace application / domain graph ontology with runtime-control
      nodes.

## Backlog Context

Echo already has the right kernel law for worldlines: one frontier, one
scheduler, one `super_tick()` path. The missing piece is an explicit
runtime graph ontology that says where strand state lives and what the
strand object actually is.

Without that ontology:

- strand docs drift toward manual ticking folklore
- support pins risk becoming accidental truth stores
- braid and settlement have no stable graph nouns to attach to

This packet creates the authoritative runtime nouns first so later work
can recurse honestly under the same causal law.
