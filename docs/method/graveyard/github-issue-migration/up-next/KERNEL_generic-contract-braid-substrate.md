<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Generic Contract Braid Substrate

Status: planned kernel/runtime implementation.

Depends on:

- [Intent-only contract runtime mutations](./KERNEL_intent-only-contract-runtime-mutations.md)
- [Contract Strands And Counterfactuals](./KERNEL_contract-strands-and-counterfactuals.md)
- [0018 - Contract-Hosted File History Substrate](../../../design/0018-contract-hosted-file-history-substrate/design.md)

## Why now

jedit needs an ordered projection of edit strands over a file worldline, but
Echo should model that as a generic braid substrate.

## What it should look like

A braid records:

- braid id;
- baseline worldline/ref;
- ordered member refs;
- current projection ref/digest;
- contract family/schema identity when contract-backed;
- basis/revalidation posture.

The simple law is sequential: each new member forks from the current projection
frontier unless the caller explicitly requests a different basis.

## Acceptance criteria

- Braid creation is an intent.
- Braid member append is an intent.
- Each member has an `orderIndex`.
- Each member records source base/ref, source tip/ref, and
  `projectionAfterDigest`.
- Braid projection is observable.
- Braid projection can return complete, residual, plural, obstructed, or
  conflict posture.
- Settlement/collapse/admission is an intent.

## Non-goals

- Do not add jedit or editor nouns to Echo core.
- Do not flatten support pins into imports.
- Do not require full production collaboration policy in the first braid slice.
