<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Strand and support Intent paths

Status: planned implementation slice.

Depends on:

- [Topology mutation Intent boundary audit](./KERNEL_topology-mutation-intent-boundary-audit.md)
- [Security/capabilities for fork/rewind/merge](./KERNEL_time-travel-capabilities.md)

## Why now

Strands and support pins are topology-changing. They affect the causal geometry
that later reads, settlement, and braids observe. External callers should not
create strands or support geometry through direct service mutation calls.

## Goal

Add narrow Intent-level external paths for creating a contract/runtime strand
from an explicit basis and for pinning/unpinning support when that is exposed to
application flows.

## Likely files touched

- `crates/echo-wasm-abi/src/kernel_port.rs`
- `crates/warp-core/src/strand.rs`
- `crates/warp-core/src/coordinator.rs`
- `crates/warp-core/src/cmd.rs`
- `crates/warp-wasm/src/warp_kernel.rs`
- `crates/warp-core/tests/strand_contract_tests.rs`

## Acceptance criteria

- Create-strand/fork external path is an EINT Intent against an explicit parent
  coordinate.
- Support pin and unpin external paths are EINT Intents when exposed outside
  the runtime.
- Direct registry/service calls remain internal implementation details.
- Stale basis, missing capability, missing provenance, duplicate strand, and
  invalid support geometry return typed obstruction/conflict posture.
- Successful operations emit tick/receipt evidence.

## Non-goals

- Do not implement full braid settlement here.
- Do not add editor-specific strand nouns.
- Do not delete internal `StrandRegistry` or `ProvenanceService` APIs.

## Test expectations

- Creating a strand through the Intent path records causal evidence.
- Direct external mutation is not required by tests.
- Stale or missing basis does not silently create a strand.
- Support pin/unpin requires an Intent path when used externally.
