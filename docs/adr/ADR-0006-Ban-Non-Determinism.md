<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# ADR-0006: Ban Semantic Non-Determinism

- **Status:** Superseded
- **Historical status:** The original proposal predated the repository's ADR status contract.
- **Superseded by:** Executable repository policy and CI witnesses

## Context

The original ADR 0006 was a proposed shell-script bundle and README fragment,
not a durable architectural decision. Its copied paths and commands drifted
from the executable repository policy. Keeping that implementation packet in
the ADR set made stale prose appear authoritative.

## Decision

ADR 0006 reserves the historical namespace position but does not duplicate
executable policy. Semantic non-determinism is constrained by the live ban
scripts, canonical-codec tests, determinism gates, and crate lint policy.
Exceptions belong in the narrow allowlists consumed by those witnesses, with
the justification next to the executable rule.

## Consequences

- The original proposal remains available in Git history.
- This record cannot drift into a second implementation of the ban scripts.
- Changes to the enforcement boundary must change an executable witness.
- A durable architecture change still requires a new ADR; CI configuration is
  evidence, not architectural authority.

## Evidence Anchors

- `scripts/ban-nondeterminism.sh`
- `scripts/ban-unordered-abi.sh`
- `scripts/ban-globals.sh`
- `.github/workflows/det-gates.yml`
- `.github/workflows/ci.yml`
