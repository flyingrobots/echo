<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Retro: 0019-reading-envelope-family-boundary

Cycle: `0019-reading-envelope-family-boundary`
Design: [`docs/design/0019-reading-envelope-family-boundary/`](../../../design/0019-reading-envelope-family-boundary/)
Witness: [`witness/`](./witness/)

## Outcome

- Status: Accepted.
- Summary: Closed the `M032` backlog item by naming the generic Echo
  reading-envelope family boundary and adding a regression test that proves
  reading envelope posture participates in observation artifact identity.

## Evidence

- `docs/design/0019-reading-envelope-family-boundary/reading-envelope-family-boundary.md`
  distinguishes authored observer families, compiled or installed artifacts,
  and runtime-emitted reading values.
- `docs/BEARING.md` points current direction at the accepted boundary.
- `docs/architecture/application-contract-hosting.md` cites the boundary as the
  generic read-side target for hosted contracts.
- `crates/warp-core/src/observation.rs` includes
  `reading_envelope_posture_participates_in_artifact_identity`.
- Verification:
    - `cargo fmt --all -- --check`
    - `cargo test -p warp-core reading_envelope_posture_participates_in_artifact_identity`
    - `cargo test -p warp-core --lib reading_envelope_posture_participates_in_artifact_identity`
    - `pnpm docs:build`
    - `pnpm exec prettier --check docs/design/0019-reading-envelope-family-boundary/reading-envelope-family-boundary.md docs/BEARING.md docs/architecture/application-contract-hosting.md`
    - `pnpm exec markdownlint-cli2 docs/design/0019-reading-envelope-family-boundary/reading-envelope-family-boundary.md docs/BEARING.md docs/architecture/application-contract-hosting.md`

## Drift Check

- The boundary stays generic: no `jedit` nouns, no Graft/editor/rope API, and
  no GraphQL-first runtime API were added to Echo core.
- The design explicitly treats CAS bytes as retention, while `ReadIdentity`
  names the semantic question answered by those bytes.
- The new test guards against treating `ReadingEnvelope` as decorative metadata
  by proving budget posture changes artifact identity even when coordinate and
  payload are unchanged.

## Follow-Up

- Implement QueryView observers against this boundary.
- Add authored observer installation only through generic plan/artifact
  identity, not application nouns.
- Keep retained-reading work keyed by `ReadIdentity` plus byte identity.
