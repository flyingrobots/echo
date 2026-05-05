<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Observer plans and reading artifacts

Status: complete. `ObservationService` and the ABI now emit one-shot built-in
observation artifacts with `ReadingEnvelope` metadata, and observation
requests explicitly name observer plan, optional hosted observer instance,
budget, and rights posture. Authored plans and hosted/stateful observer
instances are typed at the boundary and fail closed until an installed observer
host exists; query-shaped reads still exist only as a placeholder plan and
return unsupported at runtime.

Depends on:

- `crates/echo-wasm-abi/src/kernel_port.rs`
- [0006 — Echo Continuum alignment](../../../design/0006-echo-continuum-alignment/design.md)
- [0005 — Echo TTD witness surface](../../../design/0005-echo-ttd-witness-surface/design.md)
- [0011 — Optic and observer runtime doctrine](../../../design/0011-optic-observer-runtime-doctrine/design.md)

## Why now

Echo has the right instinct at the ABI boundary:

- `dispatch_intent(...)`
- `observe(...)`
- neighborhood publication
- settlement publication

But the public/runtime story is still too thin on the revelation side.
An observer is not "just a query" and a reading is not "just a state
snapshot." The current doctrine is stronger:

- app/authored observer spec is not the runtime observer instance
- the observer is only the revelation-side object, not the whole optic
- the observer basis is not the same thing as the parent basis used to realize
  a strand
- reads should come back as witness-bearing artifacts over causal
  history

Echo needs an explicit observer-plan boundary instead of letting
"observation" collapse back into ad hoc materialization.

Current implementation note: `ObservationArtifact` now carries ABI-visible
`reading: ReadingEnvelope`, which covers observer plan identity, optional
observer instance, observer basis, witness refs, parent-basis posture, budget
posture, rights posture, and residual posture for one-shot observations.
Authored `ObserverPlan` and hosted/stateful observer instances are represented
as boundary nouns and return typed unsupported errors until a contract observer
host is installed.

## What it should look like

- One authored/configured **ObserverPlan** shape exists for the read
  side.
- That plan names at least:
    - aperture / projection
    - basis
    - observer state schema
    - update law
    - emission law
    - slice budget
    - rights / exposure tier
- The runtime distinguishes:
    - observer spec / plan
    - observer instance / accumulated state
    - emitted reading artifact
- `observe(request)` returns a bounded **reading artifact**, not a
  raw "full state" story.
- A reading artifact carries:
    - frontier / coordinate
    - reading payload
    - witness or shell reference
    - parent-basis posture when the read is strand-relative
    - observer-basis metadata for native distinctions retained by the reading
    - budget posture
    - obstruction / plurality / residual when relevant
- One-shot observation and hosted/stateful observation should share
  the same artifact family.

## Done looks like

- ABI request/response types make plan/source/budget/rights explicit.
- The docs stop teaching "observer = filtered state read".
- One end-to-end path proves:
    - dispatch intent
    - get admission/result envelope
    - observe via plan at a frontier or shell
    - receive a reading artifact with witness-bearing metadata
- TTD/host integration can consume readings without demanding a
  universal materialized graph object.

## Repo evidence

- `crates/warp-core/src/observation.rs`
- `crates/echo-wasm-abi/src/kernel_port.rs`
- `crates/warp-wasm/src/warp_kernel.rs`
- `docs/design/0006-echo-continuum-alignment/design.md`
- `docs/design/0005-echo-ttd-witness-surface/design.md`

## Non-goals

- Do not embed app-specific observer business logic into Echo.
- Do not require a full app-code compiler surface in the same slice.
- Do not remove low-level diagnostic materialization helpers that are
  still useful for tests and proofs.
