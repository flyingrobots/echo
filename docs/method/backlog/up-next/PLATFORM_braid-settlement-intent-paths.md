<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Braid and settlement Intent paths

Status: planned implementation slice.

Depends on:

- [Topology mutation Intent boundary audit](./KERNEL_topology-mutation-intent-boundary-audit.md)
- [Strand and support Intent paths](./PLATFORM_strand-and-support-intent-paths.md)

## Why now

Braids and settlement decide how plural causal histories are projected,
retained, imported, or collapsed. Those decisions must be causal and replayable,
not direct host mutations.

## Goal

Add Intent-level external paths for braid member append/collapse/settlement and
strand settlement execution, while keeping compare/plan/read surfaces
side-effect free.

## Likely files touched

- `crates/echo-wasm-abi/src/kernel_port.rs`
- `crates/warp-core/src/settlement.rs`
- `crates/warp-core/src/neighborhood.rs`
- `crates/warp-core/src/optic.rs`
- `crates/warp-core/src/cmd.rs`
- `crates/warp-wasm/src/lib.rs`
- `crates/warp-wasm/src/warp_kernel.rs`
- `crates/warp-core/tests/**`

## Acceptance criteria

- Compare/plan settlement remain read-only publication surfaces.
- Executing settlement has an Intent equivalent and records causal receipt
  evidence.
- Appending a braid member, settling/collapsing a braid, or admitting a braid
  projection is represented as an Intent when exposed externally.
- Preserved plurality remains typed; it is not hidden as success/no-op.
- Direct settlement execution ABI surfaces, if retained temporarily, are marked
  compatibility/debug and are not required by jedit-style or Continuum-style
  flows.

## Non-goals

- Do not add jedit text operations.
- Do not flatten support pins into imports.
- Do not implement network transport.
- Do not make braid projection cached text canonical.

## Test expectations

- Settlement execution through Intent emits receipt/witness evidence.
- Direct settlement execution is not required by the public flow test.
- Plural and conflict outcomes remain visible.
- Braid member append requires explicit basis and does not mutate stale
  projection silently.
