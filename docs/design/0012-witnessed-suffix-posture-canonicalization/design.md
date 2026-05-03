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

A maintainer can see the canonical posture construction contract before any
production helper is added.

## Agent users / jobs / hills

### Primary agent users

Future coding agents extending witnessed suffix admission call sites.

### Agent jobs

1. Use named posture constructors instead of hand-building posture vectors.
2. Programmatically determine that posture refs are canonicalized and duplicate
   refs are rejected.

### Agent hill

An agent can run a single warp-core witnessed suffix test target and see the
missing canonical constructor contract fail for the intended reason.

## Human playback

1. The human reads the RED tests in
   `crates/warp-core/src/witnessed_suffix_tests.rs`.
2. The tests name `admissible`, `staged`, `plural`, and `conflict` constructor
   paths on `WitnessedSuffixLocalAdmissionPosture`.
3. The human can decide the next GREEN implementation without inspecting
   transport or ABI code.

## Agent playback

1. The agent runs `cargo test -p warp-core --lib witnessed_suffix`.
2. The command fails because the named posture constructors and duplicate-ref
   error type do not exist yet.
3. The agent determines the next implementation is local to warp-core
   witnessed suffix posture construction.

## Implementation outline

1. RED only: add tests for named canonical construction of admissible, staged,
   plural, and conflict local postures.
2. RED only: move the backlog card into this design packet.
3. Stop before implementing production helpers.

## Tests to write first

- constructor canonicalizes admissible refs
- constructor canonicalizes staged refs
- constructor canonicalizes plural candidate refs
- constructors reject duplicate refs with a named error
- conflict constructor names reason, source ref, digest, and overlap evidence

## Risks / unknowns

- The constructor names may change during GREEN. If so, update only the tests
  and design wording in the same narrow slice.
- Duplicate handling may need a wider posture validation policy if empty ref
  vectors also become invalid. This RED only demands duplicate rejection.
- Existing direct enum construction in tests remains as fixture setup until the
  GREEN cycle decides whether to migrate it.

## Postures

- **Accessibility:** Not applicable; this is Rust API/test surface only.
- **Localization:** Not applicable; no user-facing strings are introduced.
- **Agent inspectability:** The RED names exact constructor paths and expected
  failure so future agents can implement without broad repo exploration.

## Non-goals

- Do not implement the constructors in this RED slice.
- Do not change transport, sync, import execution, or ABI shape.
- Do not redesign witnessed suffix admission.
- Do not implement Continuum proof, IPA, or commitment machinery.
- Do not audit every admission or settlement vector in this slice.
- Do not weaken the evaluator's existing obstruction posture.
