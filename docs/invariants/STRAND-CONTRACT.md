<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# STRAND-CONTRACT

**Status:** Normative | **Legend:** KERNEL | **Cycle:** 0004

## Invariant

A strand is a named speculative execution lane rooted at one exact
admissible source-lane coordinate. It is a relation over a child
worldline created by `ProvenanceStore::fork()`, not a separate
substrate. A
strand either exists in the `StrandRegistry` (live) or does not
(dropped). There is no tombstone state.

## Invariants

The following invariants are normative. "MUST" and "MUST NOT" follow
RFC 2119 convention.

### INV-S1 — Immutable fork basis

A strand's `fork_basis_ref` MUST NOT change after creation. The
`ForkBasisRef` pins the exact admissible coordinate the strand was
forked from: source lane ID, fork tick (last included tick in the
copied prefix), commit hash at fork tick, output boundary hash (state
root after applying the patch), and a `ProvenanceRef` handle.

### INV-S2 — Own heads

A strand's child worldline MUST NOT share writer heads with its source
lane. Head keys are created fresh for the child, using the same
`WriterHead` infrastructure but with `WriterHeadKey.worldline_id`
set to the child worldline.

### INV-S3 — Session-scoped

A strand MUST NOT outlive the session that created it (v1). No
strand persistence across sessions.

### INV-S4 — Single tick law

A strand advances only through ordinary intent admission under Echo's
global `super_tick()` path. No strand-specific tick path is
authoritative.

### INV-S5 — Complete fork basis

`fork_basis_ref` MUST pin: source lane ID, fork tick, commit hash,
boundary hash, and provenance ref. All fields MUST agree with the
provenance store at construction time. If any field disagrees,
construction MUST fail.

### INV-S6 — Inherited quantum

A strand inherits its parent's `tick_quantum` at fork time (per
[FIXED-TIMESTEP](./FIXED-TIMESTEP.md) invariant). No strand can
change its quantum.

### INV-S7 — Distinct worldlines

`child_worldline_id` MUST remain distinct from the source-basis
carrier. A strand is always represented by a distinct child
worldline, even when its source lane was itself speculative.

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

### INV-S10 — Clean drop

After `drop_strand`, no runnable heads for the child worldline MUST
remain in the `PlaybackHeadRegistry`. Drop is hard-delete: the
strand, its child worldline, its heads, and its provenance are all
removed. `get(strand_id)` returns `None` after drop. A `DropReceipt`
is returned as the only proof the strand existed.

## Rationale

Echo can fork worldlines via `ProvenanceStore::fork()` but has no
concept of the relationship between forked lanes. The strand
contract names that relationship explicitly: what was forked, from
where, with what heads, and under what basis law.

This enables warp-ttd to surface strand topology through its existing
`LaneKind::STRAND` and `LaneRef.parentId` protocol, and it provides
the foundation for the settlement spec (which imports operations from
strands into canonical target worldlines under channel policy).

## Cross-references

- [FIXED-TIMESTEP](./FIXED-TIMESTEP.md) — inherited quantum
- [TTD-COUNTERFACTUAL-CREATION](./TTD-COUNTERFACTUAL-CREATION.md) —
  observation versus explicit fork
- [SPEC-0004 — Worldlines](../spec/SPEC-0004-worldlines-playback-truthbus.md)
- [SPEC-0005 — Provenance Payload](../spec/SPEC-0005-provenance-payload.md)
- `warp_core::strand` — code-level implementation
