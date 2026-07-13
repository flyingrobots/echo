<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# ADR 0020: Retained Reading Storage and Proof Boundary

- **Status:** Accepted
- **Date:** 2026-07-13

## Context

Retained readings need deterministic bytes, semantic identity, and sometimes a
compact proof that a bounded aperture agrees with committed material. Those are
different responsibilities. Treating a physical encoding, a content hash, or a
cryptographic opening as causal authority would collapse storage, meaning, and
admission into one dishonest identity.

## Decision

WSC is Echo's deterministic columnar physical representation for WARP-shaped
retained material. It is not Echo's causal ontology and does not become history,
admission authority, or recovery authority merely because bytes decode.

`echo-cas` owns exact byte identity. `ReadIdentity` owns the semantic question
answered by those bytes: causal basis, observer or optic, aperture, law,
projection, and evidence posture. A CAS hash must not impersonate a read
identity, and a read identity must not claim that payload bytes are retained.
`echo-cas` remains format-agnostic: it stores opaque bytes and must not depend
on WSC or a proof backend.

CAS hashes, WSC payload digests, commitment roots, proof digests, and
`ReadIdentity` values remain distinct typed identities. Adapters must not reuse
one representation or wrapper as another merely because two values currently
use the same hash algorithm or byte width.

Commitment and opening proofs are an optional layer above WSC and CAS.
Verkle-style commitments and IPA-style openings are the preferred vector-opening
family when bounded openings are required. Echo must report them as unsupported
until an executable verifier validates the claimed proof system and binds the
opening to the exact reading identity.

Successful proof verification establishes only the proposition named by that
proof. It does not confer capability, admission, scheduler, tick, WAL, recovery,
or reveal authority.

Any proof-bearing retained-reading envelope, when supported, must name the
payload layout and codec, commitment family and root, proof family and
reference, opened coordinates or aperture, and verification posture. Residual
or obstructed material remains explicit rather than being erased by a
successful partial opening.

Product structures such as ropes, buffers, editor checkpoints, and UI models
remain application-owned. Echo retains and proves generic causal material and
reading boundaries; it does not absorb application nouns into the runtime.

## Rejected Alternatives

- Use a CAS content hash as the semantic identity of a retained reading.
- Treat a valid commitment opening as permission to admit, execute, or reveal.
- Make editor or application checkpoint structures part of Echo core.
- Claim Verkle/IPA support before a verifier and negative proof witnesses exist.

## Consequences

- Storage adapters must preserve the distinction between byte identity and
  semantic reading identity.
- CAS stays reusable by consumers that know nothing about WSC or proofs.
- Missing bytes, unsupported proof systems, failed openings, and denied reveal
  authority produce distinct typed postures.
- Proof backends can evolve without changing Echo's causal ontology.
- Application adapters may retain domain-specific structures, but cross the
  Echo boundary through generic witnessed material and explicit coordinates.

## Evidence Anchors

- `crates/warp-core/src/wsc/mod.rs`
- `crates/warp-core/src/optic.rs`
- `crates/warp-core/src/proof.rs`
- `crates/echo-cas/src/lib.rs`
- `docs/architecture/echo-optics-adapter-notes.md`
- `docs/architecture/application-contract-hosting.md`
