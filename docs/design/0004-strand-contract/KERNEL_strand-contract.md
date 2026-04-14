<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Strand contract for Echo

Depends on: 0003 (dt policy)

Define the strand as a first-class relation in Echo, not just a fork
that happens to exist. Write a spec with exact fields, invariants,
lifecycle, and TTD mapping.

This note should be read as the **bootstrap strand hill**, not the final
parity claim. Echo needs a first honest strand contract now, but braid
geometry, settlement, and richer neighborhood publication still remain
follow-on work if Echo is going to reach conceptual parity with
`git-warp`.

## What this delivers

A spec (`SPEC-0011-strand-contract.md`) defining:

```text
Strand = {
    strand_id,
    fork_basis_ref,            // source worldline, fork tick, commit hash, boundary hash
    child_worldline_id,  // created by fork()
    writer_heads,        // own head keys, not shared with parent
    support_pins,        // read-only references to other strands (braid geometry)
}
```

## Key design decisions (already made)

- The child worldline is created by `ProvenanceStore::fork()`. The
  "overlay" is the child suffix after `fork_tick`, not a separate
  substrate.
- Strands are **ephemeral in v1**. No persistence across sessions.
  This is a bootstrap posture, not a claim that ephemerality is the
  semantic essence of strands.
- **Own head keys.** Do not share the parent's heads. Use the same
  head infrastructure but give the child worldline its own
  `WriterHeadKey`s.
- **Manual ticks in v1.** Create the child worldline, create its
  heads paused or dormant, let the debugger or API explicitly tick.
  This matches Echo's existing Dormant/Admitted control plane.
- **Surface to TTD now**, not after settlement is solved. warp-ttd
  already has `LaneKind::STRAND` and `LaneRef.parentId`. The adapter
  seam is waiting.
- **Braid is geometry, not history.** Braid = pinning read-only
  support overlays. Settlement (history) is a separate spec. v1 may
  still land with empty `support_pins`, but that is a bootstrap limit,
  not the target model.

## Invariants to specify

- INV-S1: A strand's `fork_basis_ref` is immutable after creation.
- INV-S2: A strand's child worldline shares no writer heads with
  its base worldline.
- INV-S3: A strand cannot outlive the session that created it (v1).
- INV-S4: A strand's child worldline is ticked only by explicit
  external command, never by the live scheduler.
- INV-S5: `fork_basis_ref` must pin: source worldline ID, fork tick,
  commit hash, and state/boundary hash.

## TTD mapping

- `LaneKind::STRAND` ← strand type
- `LaneRef.parentId` ← `fork_basis_ref.source_lane_id`
- Strand create/tick/compare/drop maps to warp-ttd Cycle D operations

## Relationship to other backlog items

- Supersedes design questions 1–5 in `KERNEL_strands-and-braiding`
- Enables `KERNEL_strand-settlement` (settlement spec)
- Requires follow-on `KERNEL_braid-geometry-and-neighborhood-publication`
- Unblocks `PLATFORM_echo-ttd-host-adapter` for strand surface
- Unblocks time travel MVP (TT2) fork/compare workflow
