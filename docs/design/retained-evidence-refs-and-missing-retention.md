<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Retained Evidence Refs And Missing-Retention Posture

Status: accepted and implemented local boundary.

## Claim

Echo now has first-class retained evidence references in `warp-core`. A retained
evidence reference binds:

- installed contract evidence identity;
- retained evidence role;
- semantic digest;
- content-only hash;
- byte length.

Missing retained material returns typed `MissingRetention` obstruction posture.
It does not become empty success, a cache hit, a stale query identity, or a
generic runtime fault.

## Boundary

`RetainedEvidenceCoordinate` names the semantic coordinate for retained
contract material. `RetainedEvidenceRef` then binds that coordinate to retained
bytes by content hash and byte length. `RetainedEvidencePosture` reports either
available evidence or a `ContractObstructionKind::MissingRetention` obstruction.

This is the core/product-facing reference layer above local byte retention. It
does not change `echo-cas`: CAS still names bytes only, and `RetainedBlobIndex`
still owns local semantic byte lookup.

## Invariants

- CAS byte identity is not semantic evidence identity.
- Equal bytes under different semantic coordinates produce different evidence
  reference ids.
- Reading payload refs and reading-envelope refs are distinct roles.
- A query or reading identity does not imply payload retention.
- Missing semantic coordinates and missing retained bytes surface as
  `MissingRetention`.
- Missing-retention obstructions carry the installed contract evidence identity
  when available.

## Non-Goals

- Do not add distributed retention, settlement shells, or replica import.
- Do not add disk persistence or garbage-collection policy.
- Do not treat retained reading refs as query identities.
- Do not make content hashes stand in for semantic coordinates.

## Witnesses

- `cargo test -p warp-core --test retained_evidence_ref_tests`
