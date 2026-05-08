<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Retro: 0021-parent-drift-owned-footprint-revalidation

Cycle: `0021-parent-drift-owned-footprint-revalidation`
Design: [`docs/design/0021-parent-drift-owned-footprint-revalidation/`](../../../design/0021-parent-drift-owned-footprint-revalidation/)
Witness: [`witness/`](./witness/)

## Outcome

- Status: Accepted.
- Summary: Pulled `M014` and closed it as existing implementation/evidence
  consolidation. The runtime, settlement planner, witnessed-suffix ABI, and
  observation artifacts already carry the parent-drift/owned-footprint
  revalidation law required by the backlog card.

## Evidence

- `crates/warp-core/src/strand.rs` defines `Strand::live_basis_report(...)`,
  `StrandRevalidationState`, and `StrandOverlapRevalidation`.
- `crates/warp-core/src/settlement.rs` maps parent movement inside the
  strand-owned closed footprint to explicit clean, obstructed, or conflict
  revalidation posture.
- `crates/warp-core/src/witnessed_suffix.rs` preserves settlement basis reports
  and overlap revalidation through ABI conversion.
- `crates/warp-core/src/observation.rs` carries `ObservationBasisPosture` on
  reading artifacts, including `StrandParentAdvancedDisjoint` and
  `StrandRevalidationRequired`.
- Verification:
    - `cargo test -p warp-core live_basis_report`
    - `cargo test -p warp-core strand_frontier_observation_reports_overlap_revalidation_posture`

## Drift Check

- No new runtime code was needed for this cycle.
- The accepted shape stays generic: no jedit/editor/Graft/rope nouns were added
  to Echo core.
- The live-strand read surface does not pretend overlapped parent movement is
  clean. It names the revalidation posture in the reading artifact instead.
- Settlement remains append/admission oriented; parent drift revalidation does
  not rewrite old provenance.

## Follow-Up

- Add an obstruction-specific overlap revalidation fixture when a natural
  patch-level obstruction case is available.
- Keep future optic/read APIs consuming `ObservationBasisPosture` instead of
  inventing a separate parent-drift vocabulary.
