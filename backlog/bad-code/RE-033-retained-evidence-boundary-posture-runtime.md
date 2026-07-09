<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# RE-033: Project Retained Evidence Boundary Posture

## Problem

`warp-core` has local retained evidence coordinates, refs, and missing-retention
posture, but observer-facing projection still lacks the full boundary posture
needed to distinguish native support, translated support, fixture support,
redaction, authority blockage, unsupported evidence kinds, stale basis, corrupt
content, and proof-backed compaction.

## Why It Matters

A raw retained ref or content hash can cite bytes but cannot answer whether an
observer may reveal those bytes, whether the evidence is native, whether the
proof is strong enough for the current purpose, or whether missing raw material
is actually proof-backed cold evidence. Collapsing those cases would create
false availability and false missing-retention reports.

## Desired Shape

- Project retained evidence through semantic coordinate, witness-ladder layer,
  origin, proof strength, access, and completeness posture.
- Keep citation authority, reveal authority, and admission authority distinct.
- Preserve `MissingRetention` for true local byte-retention absence while
  refining observer-facing obstruction for redaction, unsupported evidence,
  stale basis, authority blockage, corruption, fixture-only support, and
  translated evidence.
- Keep WAL, CAS, checkpoint, and scheduler internals behind the boundary
  posture surface.

## Acceptance Tests

- `native_and_fixture_retained_evidence_do_not_alias`
- `redacted_retained_evidence_is_not_missing_content`
- `unsupported_evidence_kind_obstructs_not_missing_retention`
- `retained_content_hash_does_not_identify_semantic_evidence`
- `available_retained_evidence_ref_does_not_grant_reveal`
