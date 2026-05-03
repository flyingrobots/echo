<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# 0012 — Witnessed suffix posture canonicalization

_Add a RED fence for named canonical construction of witnessed suffix local
admission postures._

Legend: [PLATFORM](../../method/legends/PLATFORM.md)

Depends on:

- [Witnessed suffix admission evaluator](../../design/witnessed-suffix-admission-evaluator.md)
- [Continuum runtime and CAS readings](../../design/continuum-runtime-and-cas-readings.md)
- [0011 — Optic and observer runtime doctrine](../0011-optic-observer-runtime-doctrine/design.md)

Source card:

- `docs/method/backlog/inbox/PLATFORM_witnessed-suffix-admission-hardening.md`

## Why this cycle exists

PR #323 made the witnessed suffix local admission evaluator deterministic and
honest around source suffix evidence, basis resolution, and output posture
ordering. The evaluator now canonicalizes posture vectors before returning ABI
visible outcomes.

That is not enough once future call sites construct
`WitnessedSuffixLocalAdmissionPosture` directly. Raw enum construction can
reintroduce caller-order leakage or duplicate provenance refs before the
evaluator gets a chance to normalize the shape. This cycle starts by making
that missing named construction path executable as RED.

GREEN 1 adds the named constructors and migrates ordinary test fixtures to use
them. Raw enum construction remains only where a test is asserting raw shape or
evaluator normalization of raw posture input.

## Human users / jobs / hills

### Primary human users

Maintainers reviewing witnessed suffix admission, settlement, and Continuum
runtime work.

### Human jobs

1. Review one focused RED and decide whether the expected posture constructor
   shape is acceptable.
2. Confirm the slice does not reopen transport, sync, ABI, or broad Continuum
   design.

### Human hill

A maintainer can review the canonical posture construction contract and the
small helper implementation without transport, sync, or ABI noise.

## Agent users / jobs / hills

### Primary agent users

Future coding agents extending witnessed suffix admission call sites.

### Agent jobs

1. Use named posture constructors instead of hand-building posture vectors.
2. Programmatically determine that posture refs are canonicalized and duplicate
   refs are rejected.

### Agent hill

An agent can run a single warp-core witnessed suffix test target and verify the
canonical constructor contract.

## Human playback

1. The human reads the RED tests in
   `crates/warp-core/src/witnessed_suffix_tests.rs`.
2. The tests name `admissible`, `staged`, `plural`, and `conflict` constructor
   paths on `WitnessedSuffixLocalAdmissionPosture`.
3. The human can decide the next GREEN implementation without inspecting
   transport or ABI code.

## Agent playback

1. The agent runs `cargo test -p warp-core --lib witnessed_suffix`.
2. The command passes the constructor contract examples and existing evaluator
   defense tests.
3. The agent determines the slice remains local to warp-core witnessed suffix
   posture construction.

## Implementation outline

1. RED: add tests for named canonical construction of admissible, staged,
   plural, and conflict local postures.
2. RED: move the backlog card into this design packet.
3. GREEN 1: add the named constructors and duplicate-ref error.
4. GREEN 1: keep ordinary fixtures on the canonical constructor path; leave raw
   enum construction only for raw-shape and evaluator-defense tests.

## Tests to write first

- constructor canonicalizes admissible refs
- constructor canonicalizes staged refs
- constructor canonicalizes plural candidate refs
- constructors reject duplicate refs with a named error
- constructors reject duplicate refs after canonical sorting
- conflict constructor names reason, source ref, digest, and overlap evidence

## Risks / unknowns

- The constructor names may change during GREEN. If so, update only the tests
  and design wording in the same narrow slice.
- Duplicate handling may need a wider posture validation policy if empty ref
  vectors also become invalid. This RED only demands duplicate rejection.
- Direct enum construction remains available for tests that deliberately assert
  raw shape or raw evaluator input. Ordinary clean fixtures should prefer named
  constructors.

## Postures

- **Accessibility:** Not applicable; this is Rust API/test surface only.
- **Localization:** Not applicable; no user-facing strings are introduced.
- **Agent inspectability:** The RED names exact constructor paths and expected
  failure so future agents can implement without broad repo exploration.

## Non-goals

- Do not change transport, sync, import execution, or ABI shape.
- Do not redesign witnessed suffix admission.
- Do not implement Continuum proof, IPA, or commitment machinery.
- Do not audit every admission or settlement vector in this slice.
- Do not weaken the evaluator's existing obstruction posture.
