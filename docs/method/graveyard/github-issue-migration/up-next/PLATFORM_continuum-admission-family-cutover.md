---
audit-date: 2026-06-15
audit-commit: 5f85dae5727d36acf4a82aad8d7cdb0488cb67be
audit-status: archive
topics:
    - continuum
    - admission
    - schema
accuracy: 0.80
issue: 485
findings:
    - claim: "TickReceiptDisposition::Applied/Rejected maps to Derived/Obstruction outcome kinds"
      ruling: true
      evidence:
          filepath: crates/warp-core/src/receipt.rs
          line: 143
    - claim: "NeighborhoodSite::Singleton/Braided maps to Derived/Plural outcome kinds"
      ruling: true
      evidence:
          filepath: crates/warp-core/src/neighborhood.rs
          line: 69
    - claim: "SettlementDecision::ImportCandidate/ConflictArtifact/PluralAlternative maps to Derived/Conflict/Plural outcome kinds"
      ruling: true
      evidence:
          filepath: crates/warp-core/src/settlement.rs
          line: 253
---

<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# PLATFORM - Continuum admission family cutover

Echo now has enough doctrine to stop hand-waving about admission at the shared
boundary. The next platform cut is to choose a small admission-facing family
slice and move it toward the Continuum/Wesley proof path.

This cycle should connect:

- `BoundedSite`
- admission outcome family
- shell versus witness layering
- policy identity where it affects published causal meaning

to the existing `PLATFORM_continuum-proof-family-runtime-cutover` work, without
trying to move the whole runtime across the boundary at once.

The current runtime now has enough truthful mapping to drive that platform cut:

- `TickReceiptDisposition::Applied` => `Derived` [🟢, 95%, [crates/warp-core/src/receipt.rs:143](file:///Users/james/git/echo/crates/warp-core/src/receipt.rs#L143)]
- `TickReceiptDisposition::Rejected(FootprintConflict)` => `Obstruction`
- `NeighborhoodSite::Singleton` => `Derived` [🟢, 95%, [crates/warp-core/src/neighborhood.rs:69](file:///Users/james/git/echo/crates/warp-core/src/neighborhood.rs#L69)]
- `NeighborhoodSite::Braided` => `Plural`
- `SettlementDecision::ImportCandidate` => `Derived` [🟢, 95%, [crates/warp-core/src/settlement.rs:253](file:///Users/james/git/echo/crates/warp-core/src/settlement.rs#L253)]
- `SettlementDecision::ConflictArtifact` => `Conflict`

The next step is not inventing more local nouns. It is selecting one generated
Continuum family slice that can carry this same outcome algebra and shell /
witness layering across the Echo boundary.

The first concrete target for that cut is now:

- Continuum packet: `docs/design/0022-neighborhood-core-and-admission-outcome-family/README.md`
- authored family: `schemas/continuum-neighborhood-core-family.graphql`
