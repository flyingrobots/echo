<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Retained Evidence Refs And Missing-Retention Posture

Status: design boundary for retained evidence projection. Existing local
`RetainedEvidenceCoordinate`, `RetainedEvidenceRef`, and
`RetainedEvidencePosture` primitives remain accepted; the broader
Observer-Geometry/Continuum posture model below governs future runtime and ABI
projection work.

## Claim

Echo retained evidence refs are boundary evidence posture objects. They cite
semantic retained support across receipt, reading, witness, and artifact
surfaces without collapsing evidence origin, proof strength, access rights,
completeness, or witness-ladder layer into raw content availability.

A retained evidence ref may bind retained bytes, but the public claim is not
"CAS hash exists." The public claim is:

```text
observer / optic surface
+ basis and aperture
+ semantic retained coordinate
+ witness-ladder layer
+ evidence origin, proof strength, access, and completeness posture
-> justified retained evidence posture
```

This prevents missing retained material from becoming empty success, a cache
hit, stale query identity, fixture/native evidence aliasing, or a generic
runtime fault.

## Existing Local Boundary

Echo already has the local reference layer needed for byte-backed retained
evidence:

- `RetainedEvidenceCoordinate` names contract evidence identity, retained role,
  and semantic digest.
- `RetainedEvidenceRef` binds that coordinate to content hash and byte length.
- `RetainedEvidencePosture` reports available evidence, missing coordinate, or
  missing content with `ContractObstructionKind::MissingRetention`.
- `RetainedBlobIndex` in `echo-cas` remains the local semantic byte index above
  content-only CAS.

Those primitives intentionally keep CAS byte identity separate from semantic
evidence identity. Equal bytes under different semantic coordinates produce
different evidence reference ids, and reading payload refs do not alias reading
envelope refs.

## Observer Boundary

Retained evidence crosses an observer or optic boundary only as posture. A
runtime may retain bytes, proofs, receipts, or shells locally, but the boundary
answer must be phrased in terms of the caller's lawful reading frame:

- observer plan or optic surface;
- causal basis and bounded aperture;
- semantic coordinate or read identity;
- retained support obligation;
- rights, budget, residual, and obstruction posture;
- evidence origin, proof strength, access, and completeness.

This follows the Echo rule that observation is revelation, not authorship. A
reading may cite retained evidence without mutating history, and citation is
not the same authority as byte revelation.

## Witness Ladder

Continuum-style boundary evidence has three non-collapsible layers:

| Layer              | Echo role                                  | Examples                                                                             |
| ------------------ | ------------------------------------------ | ------------------------------------------------------------------------------------ |
| Reintegration core | Locates the claim in causal history        | coordinates, parent hashes, lane boundaries, read identity, basis/frontier           |
| Witness core       | Proves admission or observation lawfulness | signatures, replay certificates, Merkle openings, ZK proofs, transport-square proofs |
| Receipt shell      | Carries operational explanation            | routing, debug metadata, correlation ids, human-facing explanation                   |

`RetainedEvidenceRole` names what the retained material is for. The witness
ladder names which boundary layer the material occupies. The two dimensions are
orthogonal: a receipt may cite witness-core material, and a reading envelope may
carry reintegration facts plus receipt-shell posture.

These layers may reference one another by digest, but they must not collapse
into one blob. Redacting receipt-shell metadata must not invalidate witness-core
proofs, and witness-core availability must not imply receipt-shell reveal
permission.

## Boundary Posture Axes

Future ABI and product-facing projection should treat retained evidence posture
as a small lattice over four axes:

| Axis           | Question                           | Example values                                                                                    |
| -------------- | ---------------------------------- | ------------------------------------------------------------------------------------------------- |
| Origin         | Where did this evidence come from? | native, translated, fixture, derived, opaque                                                      |
| Proof strength | What justifies it?                 | digest-only, signature, replay certificate, Merkle opening, ZK proof, composite proof             |
| Access         | What may this observer do?         | revealable, citation-only, redacted, authority-blocked, key-unavailable, unsupported              |
| Completeness   | Is the obligated support present?  | complete, partial, declared-lost, stale, missing-coordinate, missing-content, corrupt, obstructed |

The current `RetainedEvidencePosture` covers the local completeness cases that
exist today. It must not be read as the complete boundary posture lattice.

## Obstruction Rules

The public retained-evidence boundary must preserve these distinctions:

- Missing coordinate is not missing content.
- Missing content is not redaction.
- Redaction is not successful availability.
- Authority-blocked evidence is not missing retention.
- Unsupported evidence kind is an obstruction, not a cache miss.
- Fixture evidence is not native evidence.
- Translated/substrate evidence is not native Echo witnesshood.
- Corrupt content is not unavailable content.
- Stale basis is not absent retention.
- Available citation is not byte revelation.

The low-level `MissingRetention` obstruction remains valid for the local byte
reference path, but observer-facing APIs should refine it when the failure is
actually capability denial, unsupported proof kind, redaction, stale basis,
corruption, fixture-only support, translated support, or budget exhaustion.

## Implementation Shape

The next runtime slice should not add a hash lookup helper such as
`retained_ref_for_hash(hash)`. That would erase the semantic coordinate,
observer purpose, evidence layer, rights posture, and retained support
obligation.

Instead, project retained evidence through a structure shaped like:

```text
RetainedEvidenceBoundaryPosture {
  coordinate,
  role,
  layer,
  origin,
  proof_strength,
  access,
  completeness,
  obstruction,
}
```

The exact Rust type can be smaller than this sketch, but it must preserve the
axes. When the runtime lacks an axis today, it should report a conservative
unknown, opaque, blocked, or unsupported posture instead of claiming native
available evidence.

## Invariants

- CAS byte identity is not semantic evidence identity.
- Query identity and reading identity do not imply payload retention.
- Payload and envelope evidence are different roles.
- Native, translated, fixture, redacted, and obstructed evidence never alias.
- Retained proof availability does not imply retained plaintext availability.
- Citation authority, reveal authority, and admission authority are separate.
- Missing support that is declared should downgrade posture; it becomes witness
  debt only if Echo reports stronger support than the ledger justifies.
- Hot, warm, and cold compaction tiers may all preserve evidence differently;
  absence of raw bytes is not automatically absence of proof-backed evidence.

## Non-Goals

- Do not implement distributed retention, settlement shells, replica import, or
  suffix transport here.
- Do not add disk persistence or garbage-collection policy here.
- Do not expose WAL, checkpoint, CAS path, or scheduler implementation details
  as boundary evidence posture.
- Do not make content hashes stand in for semantic coordinates.
- Do not make a successful retained ref imply permission to reveal bytes.

## Witnesses

Existing local tests:

- `cargo test -p warp-core --test retained_evidence_ref_tests`

Target tests for the next implementation slice:

- native and fixture retained evidence do not produce the same boundary posture;
- redacted evidence returns citation or redaction posture, not missing content;
- unsupported proof kind returns obstruction, not missing retention;
- content hash alone cannot identify semantic retained evidence;
- available retained evidence citation does not imply reveal permission.
