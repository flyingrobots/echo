<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# ADR 0013: Echo and Continuum Authority Boundary

- **Status:** Accepted
- **Date:** 2026-07-13

## Context

Transport language repeatedly encouraged accidental state-sync semantics and
blurred the boundary between receiving bytes and admitting causal history.

## Decision

Continuum is the protocol for lawful causal-history exchange. Echo is a
deterministic WARP runtime over history it has admitted under named law.

Transport arrival is not semantic Echo history. A suffix, claim, witness, or
retained shell becomes Echo history only after identity, basis, capability,
support, budget, evidence, and law checks produce explicit admission evidence.
Applications and transports do not acquire scheduler, tick, WAL, or recovery
authority by submitting material.

## Consequences

- Import APIs must expose acceptance, duplication, obstruction, and rejection
  posture instead of reporting generic synchronization success.
- Storage locations, arrival order, and peer-local mutable state are not causal
  identity.
- Echo may materialize graphs or other readings, but Continuum exchanges
  witnessed causal claims rather than mutable application state.

## Evidence Anchors

- `docs/architecture/continuum-transport.md`
- `crates/warp-core/src/witnessed_suffix.rs`
- `crates/warp-core/src/admission.rs`
