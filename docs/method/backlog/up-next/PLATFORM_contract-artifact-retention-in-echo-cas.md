<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Contract Artifact Retention In echo-cas

Status: planned platform implementation.

Depends on:

- [Contract-aware receipts and readings](./KERNEL_contract-aware-receipts-and-readings.md)
- [Echo Continuum Runtime And CAS Readings](../../../design/continuum-runtime-and-cas-readings.md)
- [echo-cas Browser Integration](../../../design/0020-echo-cas-browser/echo-cas-browser.md)
- [WSC, Verkle, IPA, And Retained Readings](../../../architecture/wsc-verkle-ipa-retained-readings.md)

## Why now

Echo's doctrine says `echo-cas` stores retained witnesses and cached readings,
but the contract-hosting path needs concrete retention rules for generated
contract artifacts and bounded optic readings.

CAS hashes name bytes. Semantic lookup keys name the question those bytes
answer. This matches existing `echo-cas` policy: CAS hashes are content-only,
while domain separation belongs in typed references and semantic coordinates
above the blob store.

This card also carries the modern replacement for the retired retention wording
in #244: Echo should stay holographic. It should retain witnesses, receipts,
coordinates, and cached bounded readings; it should not materialize the entire
graph state every tick. When memory or disk pressure appears, cache and index
eviction is legal storage policy, but required evidence must either be
rehydrated or produce an explicit obstruction.

## What it should look like

Define and implement minimal retention for:

- intent payload blobs
- contract receipts
- witness refs
- reading payloads
- reading envelopes
- observer artifacts

Semantic lookup should include contract identity, schema hash, basis, observer
or intent kind, aperture or payload identity, and law/projection version where
applicable.

Storage tiers may use content-defined chunking for large retained artifacts or
reading payloads. Variable chunk sizes, MIME-aware chunk policy, and buzhash-like
chunk boundary selection are implementation options for deduplication and space
savings. Those choices are not causal semantics and must not affect Intent
identity, tick identity, receipt identity, read identity, or replay outcome.

The future retained-reading stack is WSC-backed and proof-ready: WSC provides
canonical columnar reading/checkpoint bytes, Verkle-style commitments may
authenticate WSC coordinates, IPA-style openings may support bounded apertures,
and `echo-cas` remains byte retention only.

## Acceptance criteria

- Stored contract receipt can be loaded by content hash.
- Stored contract reading can be loaded by content hash.
- Semantic lookup includes contract and schema identity.
- Cached reading is not reused for a newer live frontier unless a proof of
  containment or equivalent witness relation exists.
- Large retained payloads may be stored through chunked CAS layout without
  changing their semantic read identity.
- Missing locally retained witness material returns obstruction or
  rehydration-required posture, not a fake cache hit.
- Garbage collection remains storage policy and does not mutate truth.

## Non-goals

- Do not build a full distributed CAS protocol.
- Do not implement proof-carrying retention.
- Do not add app-specific indexes.
- Do not make CAS content hashes stand in for reading identity.
- Do not change `echo-cas` content-hash policy for contract semantics.
