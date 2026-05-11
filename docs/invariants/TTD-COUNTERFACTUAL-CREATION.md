<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# TTD-COUNTERFACTUAL-CREATION

**Status:** Normative | **Legend:** PLATFORM | **Cycle:** 0005

## Invariant

A TTD reading or inspection act does not by itself create new causal truth in
Echo. A counterfactual lane exists only after an explicit fork rooted at one
exact admissible source-lane coordinate. In Echo v1, strands created through
TTD or any comparable debugger surface are session-scoped speculative lanes by
default.

## Invariants

The following invariants are normative. "MUST" and "MUST NOT" follow
RFC 2119 convention.

### INV-TC1 — Observation is read-only

Read-side observation, replay, and inspection MUST NOT create strands, mutate
the live frontier, or silently rewrite canonical history.

### INV-TC2 — Explicit fork required

Continuing from an earlier coordinate or exploring a debugger-driven
counterfactual MUST go through an explicit strand-creation act. Echo MUST NOT
materialise a speculative lane merely because a user sought to a historical
coordinate.

### INV-TC3 — Exact fork basis

Every debugger-created strand MUST be rooted at one exact `ForkBasisRef`.
Echo MUST be able to name the source lane, fork tick, commit hash, boundary
hash, and provenance handle for that fork.

### INV-TC4 — Session-scoped by default in v1

In Echo v1, debugger-created strands MUST default to session-scoped scratch or
minimally retained speculative work. They MUST NOT silently become durable
shared history.

### INV-TC5 — Promotion is separate from creation

Creating a strand and promoting work into shared admitted history are distinct
acts. Echo MUST NOT treat debugger counterfactual creation as implicit shared
publication.

### INV-TC6 — Future durable retention must carry provenance posture

If Echo later supports debugger-created strands that outlive the creating
session, the retained history MUST be able to name creator, tool or session
origin, fork basis, and retention or revelation posture.

## Rationale

TTD is a revelation surface. Strands are speculative causal lanes. Collapsing
those two roles would turn inspection into hidden mutation and would destroy
the distinction between read-side explanation and write-side counterfactual
creation.

The current Echo bootstrap already supports the crucial separation: strands are
explicit objects with exact `ForkBasisRef`s, and the runtime advances them only
through ordinary `super_tick()` admission. What v1 does not yet provide is a
durable author-only retention tier beyond the session boundary. That remains a
policy extension, not an excuse to blur observation and forking.

## Cross-references

- [STRAND-CONTRACT](./STRAND-CONTRACT.md) — strand ontology and single tick law
- [0004 — Strand contract](../design/0004-strand-contract.md)
- [0005 — Echo TTD witness surface](../design/0005-echo-ttd-witness-surface.md)
