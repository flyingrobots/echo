<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Topology mutation Intent boundary audit

Status: planned kernel audit.

Depends on:

- [0022 - Continuum transport identity and import idempotence](../../../design/0022-continuum-transport-identity/design.md)

## Why now

Echo now requires external topology-changing operations to be causal and
Intent-driven. The codebase still has useful internal services for provenance
forking, strand registration, support pins, settlement, and witnessed suffix
classification. Those can remain implementation details, but we need a precise
inventory before adding public Intent wrappers.

## Goal

Classify every topology-changing surface as one of:

- internal implementation helper
- read/observation surface
- external mutation surface that must gain an Intent path
- legacy/debug ABI surface that must be documented as temporary

## Likely files touched

- `crates/warp-core/src/provenance_store.rs`
- `crates/warp-core/src/coordinator.rs`
- `crates/warp-core/src/strand.rs`
- `crates/warp-core/src/settlement.rs`
- `crates/warp-core/src/witnessed_suffix.rs`
- `crates/echo-wasm-abi/src/kernel_port.rs`
- `crates/warp-wasm/src/lib.rs`
- `crates/warp-wasm/src/warp_kernel.rs`
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
- No code behavior changes are required unless a test exposes an unsafe public
  mutation path that can be sealed cheaply.

## Non-goals

- Do not implement all wrappers in the audit card.
- Do not delete internal services.
- Do not block read-only compare/plan/observe surfaces.
- Do not add a global graph API.

## Test expectations

- Static or targeted tests should prove any newly classified public mutation
  path is either Intent-backed or explicitly marked legacy/debug.
- Existing settlement and strand tests remain green.
