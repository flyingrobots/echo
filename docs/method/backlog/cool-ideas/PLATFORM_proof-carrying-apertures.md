<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Proof-Carrying Apertures

Status: cool idea, future proof backend lane.

Depends on:

- [Contract-aware receipts and readings](../up-next/KERNEL_contract-aware-receipts-and-readings.md)
- [Contract artifact retention in echo-cas](../up-next/PLATFORM_contract-artifact-retention-in-echo-cas.md)
- [WSC, Verkle, IPA, And Retained Readings](../../../architecture/wsc-verkle-ipa-retained-readings.md)
- [WARPDrive POSIX Materialization Optic](./PLATFORM_warpdrive-posix-optic.md)

## Why later

Echo readings are WARP optic outputs: they name a coordinate, aperture, law,
observer basis, payload, and support posture. Some readings need full
materialization today because the verifier has no compact proof shape for the
claim being made.

Future proof backends should let an optic carry compact support for selected
claims without widening the observer's revelation aperture. IPA fits this shape:
the aperture becomes public inputs or selectors, the hidden territory is a
committed vector, polynomial, or WSC column family, the reading is a claimed
relation or evaluation, and the IPA proof is the transported support.

Doctrine phrase:

```text
Proof-carrying apertures.
```

## What it should look like

Contract-aware readings and retained holograms should be able to carry optional
proof support without requiring proof systems in the first implementation:

- commitment family and commitment ref
- proof family and proof ref
- public inputs hash
- verification posture
- support obligation status
- payload codec and payload ref

The future posture vocabulary should be able to distinguish:

- materialized bytes are present
- selected data is opened by Merkle or Verkle-style witness
- a relation is verified by IPA or equivalent compact proof
- a predicate is verified by a ZK-style proof
- support is rehydratable
- support is obstructed or underdetermined

WSC is the natural future payload layout for WARP-shaped committed structure.
`echo-cas` stores bytes. WSC provides canonical columnar state/read-model bytes.
Merkle, Verkle, IPA, SNARK, STARK, or related proof families may sit above those
bytes as proof backends over retained holograms.

The current architectural direction is captured in
[WSC, Verkle, IPA, And Retained Readings](../../../architecture/wsc-verkle-ipa-retained-readings.md):
WSC gives the table, Verkle gives the root, IPA gives the aperture proof, and
`echo-cas` stores the bytes.

## Acceptance criteria

- `M012` leaves room for commitment refs, proof refs, public-input hashes, and
  verification posture without implementing a proof system.
- A future design packet can describe one proof-carrying reading whose verifier
  does not materialize the full slice.
- The design separates `ReadIdentity` from CAS hash, WSC payload hash, and proof
  identity.
- The design distinguishes materialization, inclusion/opening witnesses,
  relational proofs, and predicate proofs.
- Missing proof support returns an explicit obstruction, rehydration-required,
  or underdetermined posture.

## Non-goals

- Do not implement IPA, Verkle, SNARK, STARK, or polynomial commitments in this
  card.
- Do not make IPA a storage substrate.
- Do not make Verkle the ontology.
- Do not make `echo-cas` depend on WSC or proof systems.
- Do not treat CAS hashes as semantic reading identity.
- Do not treat `Verify(proof) = accept` as admissibility without context,
  authority, policy, and support-obligation checks.

## Notes

Useful primitive stack:

```text
BLAKE3 / content hash
  exact-byte identity

Merkle path
  inclusion or exclusion under a root

Verkle / vector commitment
  compact state-cell or update witness

IPA / polynomial commitment opening
  compact relation or evaluation over committed structure

ZK proof
  predicate verification with bounded revelation
```

The design rule for future proof-friendly optics:

```text
Build optics whose readings can be expressed as relations over committed
columns, vectors, polynomial evaluations, or explicitly supported predicates.
```
