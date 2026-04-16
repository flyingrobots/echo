<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# 0012 Dynamic Footprint Binding Runtime

## One Sentence

Echo should treat rewrite footprint law as static at the level of slots,
binding sources, closure operators, create/update surfaces, and forbidden
surfaces, while treating concrete node/head/range membership as a runtime
binding problem.

## Why This Exists

The current bounded rewrite proof slice closes one important gap:

- user-authored rewrite logic no longer needs a public native Rust callback
  seam
- Wesley can generate a bounded Rust surface from a declared footprint
- Echo can consume that surface and prove undeclared capability access fails at
  compile time

That proof is still flat. Real consumer rewrites, especially hot graph rewrites
like `ReplaceRangeAsTick`, do not operate on pre-known static node identities.
They need runtime bindings such as:

- worldline id
- base head id
- byte range
- dynamically derived local focus closure
- optionally dynamically derived affected-anchor closure

This note states how Echo should handle that without reopening arbitrary graph
reach.

## Core Split

Echo should preserve this split:

- **static footprint schema**
    - declared slots
    - binding sources
    - closure operators
    - create/update surfaces
    - forbidden surfaces
- **dynamic footprint binding**
    - concrete ids supplied by the caller
    - relation bindings resolved from current runtime truth
    - derived closure membership resolved from actual graph state

Wesley owns the first half. Echo owns the second.

## Runtime Responsibilities

Given a Wesley-compiled structured rewrite contract, Echo should be responsible
for:

1. binding direct slots from invocation arguments
2. binding relation-derived slots from already-bound slots
3. resolving declared closure operators against runtime truth
4. enforcing cardinality and basis validity for those bindings
5. exposing only the declared slot/closure/create/update capabilities to the
   rewrite implementation
6. rejecting stale, ambiguous, or invalid bindings at runtime

Echo should not allow the implementation to compensate for missing bindings by
performing arbitrary extra traversal outside the declared closure grammar.

## Example Shape

For a rewrite like `ReplaceRangeAsTick`, the runtime binding problem is:

- bind `worldline` from `worldlineId`
- bind `baseHead` from `baseHeadId`
- derive `touchedRope` from `baseHead` plus `[startByte, endByte]`
- optionally derive `affectedAnchors` from `worldline`, `baseHead`, and the
  edit window
- create the next head, tick, receipt, and any local rope/blob nodes
- update the worldline's canonical-head relation

The runtime may discover:

- one touched leaf
- many touched rope nodes
- zero affected anchors
- many affected anchors

Those are runtime truths. The compile-time honesty claim is only that the
rewrite cannot reach beyond:

- the bound slots
- the declared closures
- the declared create/update surfaces

## Failure Split

Echo should distinguish two failure classes clearly.

### Compile-time honesty failures

These are handled by Wesley-generated surfaces and Rust compilation:

- implementation tries to access undeclared capability
- implementation tries to touch forbidden surfaces
- implementation tries to create/update undeclared graph nouns

### Runtime binding failures

These are handled by Echo at invocation or admission time:

- bound ids do not resolve
- relation-based binding does not exist
- closure cardinality is wrong for the declared contract
- basis/head/worldline relationships are stale or invalid
- logical no-op means no admitted rewrite should be minted

## Immediate Implication

The next serious runtime seam is not "more flat footprint tests." It is:

- define structured runtime binding objects for slots and closures
- bind one real rewrite family against them
- prove that the runtime can resolve dynamic focus honestly without reopening
  arbitrary graph traversal

That is the path from the current proof slice to real hot-graph consumers.
