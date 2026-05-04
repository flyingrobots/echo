<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Contract Artifact Retention In echo-cas

Status: planned platform implementation.

Depends on:

- [Contract-aware receipts and readings](./KERNEL_contract-aware-receipts-and-readings.md)
- [Echo Continuum Runtime And CAS Readings](../../../design/continuum-runtime-and-cas-readings.md)
- [echo-cas Browser](./PLATFORM_echo-cas-browser.md)

## Why now

Echo's doctrine says `echo-cas` stores retained witnesses and cached readings,
but the contract-hosting path needs concrete retention rules for generated
contract artifacts.

CAS hashes name bytes. Semantic lookup keys name the question those bytes
answer.

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

## Acceptance criteria

- Stored contract receipt can be loaded by content hash.
- Stored contract reading can be loaded by content hash.
- Semantic lookup includes contract and schema identity.
- Cached reading is not reused for a newer live frontier unless a proof of
  containment or equivalent witness relation exists.
- Garbage collection remains storage policy and does not mutate truth.

## Non-goals

- Do not build a full distributed CAS protocol.
- Do not implement proof-carrying retention.
- Do not add app-specific indexes.
- Do not make CAS content hashes stand in for reading identity.
