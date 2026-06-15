<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Goalpost 1: Lawful Construction And Typed Failures

Status: planned.

Roadmap:
[`../braids-and-strands-roadmap.md`](../braids-and-strands-roadmap.md)

## Decision Summary

Echo will stop presenting law-bearing strand and braid/proof failures as casual
public data shapes. `Strand<P>` construction moves behind posture-aware doors,
proof validation reports structured `ProofError` variants, and braid transition
failures report typed transition kinds instead of string action names.

## Invariant

No law-looking strand value or transition failure is created by accident.
Typestate narrows ordinary construction, but live registry and runtime posture
checks remain the final admission law.

## Sponsored Human

A maintainer wants hard API boundaries around strand posture and braid/proof
failures so future refactors cannot create fake `Shared`-looking values or
parse behavior out of display text.

## Sponsored Agent

An agent needs typed constructors, fixture builders, and error variants so it
can write regression tests against behavior without inferring invariants from
public fields or error strings.

## Scope

This goalpost includes:

- private or controlled `Strand<P>` construction;
- read-only accessors where public data remains needed;
- fixture builders for tests;
- structured `ProofError`;
- typed `BraidTransitionKind`;
- negative tests for forged construction and string parsing.

## Non-Goals

This goalpost does not include:

- changing settlement semantics;
- replacing runtime posture checks with typestate;
- adding real cryptographic proof verification;
- adding historical braid membership views.

## Slices

| Slice  | Work                                               | Witness                                          |
| ------ | -------------------------------------------------- | ------------------------------------------------ |
| GP1-S1 | Make `Strand<P>` construction posture-aware        | compile-fail or API test for public forgery      |
| GP1-S2 | Replace public test construction with fixtures     | tests use builders instead of public literals    |
| GP1-S3 | Replace proof validation strings with `ProofError` | tests assert exact error variants                |
| GP1-S4 | Replace action strings with `BraidTransitionKind`  | tests assert exact invalid transition kind       |
| GP1-S5 | Add negative capability tests                      | tests reject forged strands and string contracts |

## Acceptance

- External callers cannot set `_marker` or `retention_posture` directly.
- `Strand<Shared>` cannot be publicly constructed with `AuthorOnly` retention
  posture.
- `ProofEnvelope` callers can branch on `ProofError` variants.
- Braid transition failures identify a typed transition kind.
- Display text remains human-facing only.
