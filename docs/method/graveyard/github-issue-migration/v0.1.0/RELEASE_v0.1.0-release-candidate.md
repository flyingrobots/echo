<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# v0.1.0 Release Candidate

Status: terminal v0.1.0 release blocker.

Depends on:

- [v0.1.0 replay and DIND proof](./TEST_v0.1.0-replay-dind-proof.md)
- [Release-grade quickstart](./DOCS_release-grade-quickstart.md)
- [Authority boundary audit](./SECURITY_authority-boundary-audit.md)
- [Versioned contract and API compatibility](./PLATFORM_versioned-contract-api-compatibility.md)
- [jedit real Echo release gate](./PLATFORM_jedit-real-echo-release-gate.md)

## Why now

This card closes the release lane after the implementation, proof, docs, and
authority checks are merged. It should not be pulled until the remaining
`v0.1.0/` cards are complete or explicitly removed from the release bar.

## Release candidate criteria

A `v0.1.0` release candidate exists when:

- all Required Feature Clusters have merged tests;
- the jedit-on-Echo release gate passes on clean Echo and jedit checkouts;
- the quickstart is executable from scratch;
- the authority-boundary audit is green;
- the replay/DIND proof is green;
- package/version metadata is stable;
- no P1 backlog item targets the `v0.1.0` feature bar.

## Acceptance criteria

- The release plan is up to date and links every blocking card.
- The release lane is empty, closed, or contains only explicitly deferred
  post-release cards.
- CI and local release witnesses pass.
- CHANGELOG and release notes describe the supported contract-host surface.
- Version tags and package publish steps are documented before execution.

## Non-goals

- Do not expand the release bar during release-candidate work.
- Do not merge release work with unrelated implementation slices.
- Do not publish without explicit user approval.
