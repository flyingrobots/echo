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

- `TickReceiptDisposition::Applied` => `Derived`
- `TickReceiptDisposition::Rejected(FootprintConflict)` => `Obstruction`
- `NeighborhoodSite::Singleton` => `Derived`
- `NeighborhoodSite::Braided` => `Plural`
- `SettlementDecision::ImportCandidate` => `Derived`
- `SettlementDecision::ConflictArtifact` => `Conflict`

The next step is not inventing more local nouns. It is selecting one generated
Continuum family slice that can carry this same outcome algebra and shell /
witness layering across the Echo boundary.
