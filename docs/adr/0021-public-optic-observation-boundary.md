<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# ADR 0021: Public Optic and Observation Boundary

- **Status:** Accepted
- **Date:** 2026-07-13
- **Partially supersedes:** [ADR 0011](ADR-0011-explicit-observation-contract.md)

## Context

ADR 0011 established explicit, read-only observation at a named worldline
coordinate, frame, and projection. It also said that `observe(...)` would
remain the only canonical public read entrypoint. Echo now has a richer WARP
optic boundary: an authored optic names its causal basis, focus, bounded
aperture, law, projection, budget, and evidence posture, while the observation
service remains useful as the deterministic primitive that resolves a
coordinate and materializes a projection.

The WASM boundary currently exposes `observe_optic`, raw `observe`, and
neighborhood projection functions. Treating those exports as equivalent
product contracts would let an adapter escape the optic's aperture, budget,
residual, and obstruction semantics.

## Decision

The canonical application and adapter read boundary is a bounded WARP optic.
Product reads use `observe_optic` and receive either an `OpticReading` or a
typed obstruction. Adapters must not privately materialize a wider reading to
hide an obstruction or budget limit.

`ObservationService::observe` is the internal lowering primitive for explicit
coordinate, frame, and projection resolution. An optic may lower through that
primitive after validating its aperture and budget. The primitive does not by
itself confer optic law, support, capability, budget, residual, or evidence
posture.

Raw observation and neighborhood exports are lower-level ABI surfaces, not the
canonical application adapter contract. They retain ADR 0011's explicit
coordinate and read-only requirements and must not acquire application-specific
semantics that bypass the optic boundary.

## Rejected Alternatives

- Treat every public read export as an interchangeable product API.
- Remove the observation primitive and duplicate coordinate resolution in each
  optic.
- Let adapters widen an aperture or erase an obstruction to satisfy a UI.
- Make editor, debugger, or inspector nouns part of Echo's read ontology.

## Consequences

- Application adapters have one WARP-shaped read contract: `observe_optic`.
- Observation remains reusable as a deterministic internal mechanism without
  impersonating the public optic law.
- Lower-level ABI exports must remain explicit and read-only, and cannot justify
  a second product-facing observation model.
- Tests must distinguish optic validation and obstruction behavior from raw
  observation projection behavior.

## Evidence Anchors

- `crates/warp-core/src/observation.rs`
- `crates/warp-core/src/optic.rs`
- `crates/warp-wasm/src/lib.rs`
- `docs/architecture/echo-optics-adapter-notes.md`
