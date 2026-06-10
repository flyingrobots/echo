<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# WARPDrive POSIX Materialization Optic

Status: cool idea.

Depends on:

- [There Is No Graph](../../../architecture/there-is-no-graph.md)
- [Echo Optics API Design](../../../design/0018-echo-optics-api-design/design.md)
- [Continuum Transport](../../../architecture/continuum-transport.md)

## Why

Humans and legacy tools operate on files, directories, and save events. The
Continuum operates on witnessed causal history, coordinates, optics, suffixes,
and holograms.

WARPDrive is the compatibility optic between those worlds:

```text
POSIX/FUSE read -> bounded reading/materialization at a WARP coordinate
POSIX/FUSE write -> delta/hunk -> causal Intent/admission attempt
```

This keeps files as boundary readings instead of substrate truth.

## Goal

Design a WARPDrive architecture packet for a FUSE/POSIX mount that materializes
path-like readings from WARP coordinates and translates writes back into
candidate causal suffixes.

## Likely files touched

- `docs/architecture/there-is-no-graph.md`
- `docs/design/0018-echo-optics-api-design/design.md`
- `docs/design/continuum-runtime-and-cas-readings.md`
- future WARPDrive repository or crate, if this graduates out of cool ideas

## Acceptance criteria

- The design states that mounted files are materialized readings, not canonical
  truth.
- Reads name coordinate, optic, aperture, witness basis, budget posture, and
  residual/obstruction posture.
- Writes compute a delta against the prior reading and submit an Intent against
  an explicit causal basis.
- Stale basis, missing evidence, policy denial, and conflict return typed
  obstructions instead of silently mutating current state.
- The design explains how multiple human/agent lanes can operate without Git
  worktrees by mounting different coordinates or strands.
- The design keeps Echo, `git-warp`, Wesley, Graft, and `warp-ttd` as peer
  WARP optics rather than making WARPDrive a god runtime.

## Non-goals

- Do not implement FUSE in this card.
- Do not replace Git in current developer workflows in this card.
- Do not make files substrate truth.
- Do not require every WARP runtime to share an internal graph representation.

## Test expectations

- Future tests should prove read identity includes coordinate and optic law,
  not just path bytes.
- Future tests should prove a stale write is rejected, staged, or obstructed
  explicitly.
- Future tests should prove cache hits cannot answer a different coordinate or
  aperture.
