<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# ADR 0022: Application-Requested Causal-Anchor Admission

- **Status:** Accepted
- **Date:** 2026-07-14

## Context

Echo's first causal-anchor API exposed `CausalAnchorFact::from_request` and
accepted a caller-provided `admitted_by_receipt_id`. The implementation was
honest in its documentation that this operation created only a canonical value,
but the public nouns were not honest: an application could manufacture a value
named `Fact` with a field named as though Echo had admitted it.

Applications need to identify meaningful domain boundaries, while Echo must own
the transition that makes those claims causal history. The WAL is the durability
mechanism for that transition, not a second semantic ledger. Materialization
caches and physical retention pins are related resources, not synonyms for the
anchor fact.

## Decision

Echo recognizes two distinct values and one later transition:

1. `CausalAnchorAdmissionRequest` is an application proposal. It names the
   subject, basis frontier, retained roots, optional materialization roots, and
   purpose. It contains no Echo receipt identity.
2. `CausalAnchorClaim` is the canonical, shape-validated form of that proposal.
   It is not admitted history and does not prove root existence, authority,
   retention, or receipt provenance.
3. A trusted Echo admission transition derives the receipt identity, constructs
   `CausalAnchorFact`, commits the transition through the WAL, and only then
   publishes and returns the admitted fact and receipt.

The admission transition is Echo-owned control history. It is not an
application contract tick and does not mutate application-domain state. An
application-domain checkpoint may reference the admitted anchor and separately
validate what the roots mean.

An Echo receipt proves only the proposition it binds: Echo admitted the
canonical claim at the named, validated causal basis. Application root semantics
remain application-owned. Materialization roots remain derived artifacts.
Physical CAS retention remains a separate resource policy and must not be
inferred merely from the existence of an anchor fact.

## Rejected Alternatives

- Let applications provide a value called an Echo admission receipt.
- Treat canonical value construction as causal admission.
- Use a Jim-local or other app-local adapter to mint `authority: echo` evidence.
- Make the anchor a graph-wide materialized snapshot.
- Treat the WAL, projection cache, or retention index as an independent semantic
  authority.

## Consequences

- The value-only constructor moves to `CausalAnchorClaim`; callers cannot create
  `CausalAnchorFact` directly.
- Repeated admission of the same claim may produce distinct facts because Echo's
  receipt identity is part of the admitted fact digest.
- The trusted admission path must be WAL-backed and recoverable before any app
  can call an anchor Echo-admitted.
- Jim must consume Echo-produced anchor identity and receipt evidence rather
  than reproduce Echo's digest algorithm.
- Root support, retention policy, and domain checkpoint validation remain
  explicit follow-on checks instead of being implied by naming.

## Evidence Anchors

- `crates/warp-core/src/causal_anchor.rs`
- `crates/warp-core/tests/causal_anchor_tests.rs`
- `docs/topics/CausalAnchors.md`
- [CA-01 milestone](https://github.com/flyingrobots/echo/milestone/35)
