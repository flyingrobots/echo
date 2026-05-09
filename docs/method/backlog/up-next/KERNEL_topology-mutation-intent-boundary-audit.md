<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# WARP optic boundary audit for topology and history operations

Status: planned kernel audit.

Depends on:

- [0022 - Continuum transport identity and import idempotence](../../../design/0022-continuum-transport-identity/design.md)

## Why now

Echo now requires external topology-changing operations to be causal and
Intent-driven. The codebase still has useful internal services for provenance
forking, strand registration, support pins, settlement, and witnessed suffix
classification. Those can remain implementation details, but we need a precise
inventory before adding public Intent wrappers.

The stronger doctrine is that topology mutation is only one posture of the
same WARP optic shape. Tick admission, transport import, fork, merge, braid,
settlement, support mutation, inverse admission, observation, materialization,
and hologram slicing all choose a bounded causal basis/site, apply a law, and
produce a witnessed hologram. This audit should keep the write-side focus
narrow while naming that shared boundary.

## Goal

Classify every topology/history/projection surface as one of:

- internal implementation helper
- read/observation surface
- external mutation surface that must gain an Intent path
- legacy/debug ABI surface that must be documented as temporary
- retention/reveal surface that must be keyed by read identity and witness basis

## Likely files touched

- `crates/warp-core/src/provenance_store.rs`
- `crates/warp-core/src/coordinator.rs`
- `crates/warp-core/src/strand.rs`
- `crates/warp-core/src/settlement.rs`
- `crates/warp-core/src/witnessed_suffix.rs`
- `crates/warp-core/src/observation.rs`
- `crates/echo-wasm-abi/src/kernel_port.rs`
- `crates/warp-wasm/src/lib.rs`
- `crates/warp-wasm/src/warp_kernel.rs`
- `docs/architecture/there-is-no-graph.md`
- `docs/architecture/continuum-transport.md`

## Acceptance criteria

- The audit lists every current direct external mutation candidate, including:
    - provenance fork
    - strand registration
    - support pin/unpin
    - settlement execution
    - braid/member mutation surfaces if present
    - import suffix admission
    - inverse/compensating operations if present
- Each surface is classified as internal, read-only, Intent-required, or
  legacy/debug temporary.
- The audit identifies the minimum Intent wrappers needed for the next runtime
  cuts.
- The audit identifies read/materialization/retention surfaces that must stay
  observer-relative and hologram/read-identity keyed rather than becoming
  hidden graph-state fallbacks.
- No code behavior changes are required unless a test exposes an unsafe public
  mutation path that can be sealed cheaply.

## Non-goals

- Do not implement all wrappers in the audit card.
- Do not delete internal services.
- Do not block read-only compare/plan/observe surfaces.
- Do not add a global graph API.
- Do not turn materialization or retention into canonical graph state.

## Test expectations

- Static or targeted tests should prove any newly classified public mutation
  path is either Intent-backed or explicitly marked legacy/debug.
- Static or targeted tests should prove classified read/materialization paths
  either return bounded readings/holograms or are explicitly internal helpers.
- Existing settlement and strand tests remain green.
