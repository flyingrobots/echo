<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# TimeStream retention + spool compaction + wormhole density

Status: complete.

Ref: #244

Resolution: obsolete. Do not implement this card as written. It came from an
early model that treated retained event streams, spools, and checkpoint density
as the center of the design. The current model is bounded optics over witnessed
causal history:

- Echo does not materialize the entire graph state every tick.
- Witness receipts, causal coordinates, frontiers, and retained artifacts are
  substrate truth.
- Optics read bounded regions by slicing the required causal history and
  lowering only what the aperture asks for.
- `echo-cas` may cache sliced readings and retained artifacts, but the cache is
  keyed by semantic read identity plus content hash, not content hash alone.
- Evicting a cache entry must not change canonical history.
- Missing required evidence returns an explicit obstruction or
  rehydration-required posture.

The active follow-up is
`docs/method/backlog/up-next/PLATFORM_contract-artifact-retention-in-echo-cas.md`
plus the doctrine in `docs/design/continuum-runtime-and-cas-readings.md`.

Historical acceptance for this card is intentionally closed as superseded
rather than satisfied. Future storage work should talk about bounded replay,
retained readings, semantic cache keys, rehydration, and explicit obstruction.
