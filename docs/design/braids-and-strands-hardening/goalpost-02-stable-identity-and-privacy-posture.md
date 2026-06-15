<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Goalpost 2: Stable Identity And Privacy Posture

Status: planned.

Roadmap:
[`../braids-and-strands-roadmap.md`](../braids-and-strands-roadmap.md)

## Decision Summary

Echo will lock the young proof, braid shell, and member reference identity
surfaces with golden vectors before more callers depend on them. The same
goalpost makes deterministic blinding salt semantics explicit so reproducible
local behavior is never mistaken for unlinkability.

## Invariant

Digest identity drift is intentional, visible, and compatibility-classed.
Deterministic blinding defaults are reproducibility tools, not privacy
boundaries.

## Sponsored Human

A maintainer wants refactors to fail loudly when they change braid shell,
proof, or sealed member identity.

## Sponsored Agent

An agent needs hand-reviewable vector fixtures and explicit privacy wording so
it can distinguish stable identity promises from E1 scaffolding and test-only
fixtures.

## Scope

This goalpost includes:

- replay-trace `ProofEnvelope` digest vectors;
- proofless and proof-bearing `BraidShell` vectors;
- revealed and sealed `BraidMemberRef` vectors;
- salt-effect vectors;
- compatibility class labels;
- privacy docs for deterministic and caller-supplied blinding material.

## Non-Goals

This goalpost does not include:

- implementing verifier backends;
- changing shell semantics beyond identity hardening;
- claiming unlinkability from deterministic defaults;
- introducing sealed membership presentation tokens.

## Slices

| Slice  | Work                                         | Witness                                 |
| ------ | -------------------------------------------- | --------------------------------------- |
| GP2-S1 | Add replay-trace `ProofEnvelope` vectors     | vector fixture test                     |
| GP2-S2 | Add proofless/proof-bearing shell vectors    | vector fixture test                     |
| GP2-S3 | Add revealed/sealed member reference vectors | vector and salt-effect tests            |
| GP2-S4 | Mark compatibility classes                   | fixture metadata or docs assertion      |
| GP2-S5 | Document deterministic blinding salt risk    | docs/examples plus salt-path regression |

## Acceptance

- CI catches digest drift for proof envelopes and braid shells.
- Sealed member reference vectors prove caller-supplied salt changes the
  commitment.
- Public stable identity changes require a migration path, version bump, or
  declaration that no prior stable identity was published.
- Privacy-sensitive examples never use deterministic defaults as the privacy
  boundary.
