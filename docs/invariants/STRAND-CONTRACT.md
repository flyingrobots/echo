<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# STRAND-CONTRACT

**Status:** Normative | **Legend:** KERNEL | **Cycle:** 0004

## Invariant

A strand is a named, session-scoped speculative execution lane derived
from a base worldline. It records an immutable parent anchor, owns local
divergence over a closed footprint, and realizes reads against an explicit
parent basis. The current implementation may still use a child worldline
created by `ProvenanceStore::fork()` as a realization detail, but the
public invariant is live-basis semantics: parent movement outside the owned
footprint may flow through, and parent movement inside the owned footprint
requires explicit revalidation, conflict, or obstruction.

A strand either exists in the `StrandRegistry` (live) or does not (dropped).
Dropping a strand releases the live handle and implementation-local caches or
worldline machinery. It must not be interpreted as proof that the speculative
lane was never real.

## Invariants

The following invariants are normative. "MUST" and "MUST NOT" follow
RFC 2119 convention.

### INV-S1 — Immutable parent anchor

A strand's `base_ref` MUST NOT change after creation. The `BaseRef`
pins the exact provenance coordinate the strand was forked from:
source worldline ID, fork tick (last included tick in the copied
prefix), commit hash at fork tick, output boundary hash (state root
after applying the patch), and a `ProvenanceRef` handle.

The anchor is not the full realized basis forever. Live reads and settlement
planning MUST compare the anchor with current parent history and report the
resulting basis posture.

### INV-S2 — Own heads

A strand's child worldline MUST NOT share writer heads with its base
worldline. Head keys are created fresh for the child, using the same
`WriterHead` infrastructure but with `WriterHeadKey.worldline_id`
set to the child worldline.

### INV-S3 — Session-scoped

A strand MUST NOT outlive the session that created it (v1). No
strand persistence across sessions.

### INV-S4 — Deterministic Tick

A strand's worldline MUST NOT be ticked by the live scheduler. It MUST
advance only through ordinary ingress + `super_tick()` coordination.

### INV-S5 — Complete base_ref

`base_ref` MUST pin: source worldline ID, fork tick, commit hash,
boundary hash, and provenance ref. All fields MUST agree with the
provenance store at construction time. If any field disagrees,
construction MUST fail.

### INV-S6 — Inherited quantum

A strand inherits its parent's `tick_quantum` at fork time (per
[FIXED-TIMESTEP](./FIXED-TIMESTEP.md) invariant). No strand can
change its quantum.

### INV-S7 — Distinct worldlines

`child_worldline_id` MUST NOT equal `base_ref.source_worldline_id`.
A strand is always a distinct worldline from its base.

### INV-S8 — Head ownership

Every key in `writer_heads` MUST belong to `child_worldline_id`.
No head may reference a different worldline.

### INV-S9 — Support pins are validated, live, and read-only

`support_pins` MAY be non-empty once braid geometry is enabled, but every
declared pin MUST:

- target a live strand
- name that strand's child worldline correctly
- avoid self-reference and duplicate targets
- remain read-only support, not write authority

### INV-S10 — Live-basis revalidation

When a strand is realized at a frontier, the runtime MUST report one of these
basis postures:

- parent remains at the strand anchor;
- parent advanced outside the strand-owned closed footprint;
- parent advanced inside the owned footprint and revalidation is required.

Reads, settlement, and comparison MUST preserve that posture instead of
pretending every strand remains a frozen fork from its anchor.

### INV-S11 — Clean drop

After `drop_strand`, no runnable heads for the child worldline MUST
remain in the `PlaybackHeadRegistry`. Drop is hard-delete: the
strand, its child worldline, its heads, and its provenance are all
removed from the live session machinery. `get(strand_id)` returns `None` after
drop. A `DropReceipt` is returned as the session-local proof that the strand was
dropped. This cleanup rule is lifecycle hygiene, not the ontology of a strand.

## Rationale

Echo can fork worldlines via `ProvenanceStore::fork()`, but a strand is not
merely a copied prefix. The strand contract names the relationship explicitly:
what parent coordinate anchored it, what local divergence it owns, what parent
basis it is being realized against, and what posture is required when parent
history moves.

This enables warp-ttd to surface strand topology through its existing
`LaneKind::STRAND` and `LaneRef.parentId` protocol, and it provides
the foundation for the settlement spec (which imports operations from
strands into base worldlines under channel policy).

## Cross-references

- [FIXED-TIMESTEP](./FIXED-TIMESTEP.md) — inherited quantum
- [SPEC-0004 — Worldlines](../spec/SPEC-0004-worldlines-playback-truthbus.md)
- [SPEC-0005 — Provenance Payload](../spec/SPEC-0005-provenance-payload.md)
- `docs/design/0008-strand-settlement/design.md`
- `docs/design/0010-live-basis-settlement-plan/design.md`
- `warp_core::strand` — code-level implementation
