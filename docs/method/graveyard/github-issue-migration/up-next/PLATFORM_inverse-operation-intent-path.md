<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Inverse operation Intent path

Status: planned implementation slice.

Depends on:

- [Topology mutation Intent boundary audit](./KERNEL_topology-mutation-intent-boundary-audit.md)

## Why now

Undo/unapply must not delete history or rewrite old provenance. The only lawful
write-side surface is to append a contract-defined inverse or compensating
operation through Echo admission.

## Goal

Add the generic Intent-level path for requesting contract inverse admission
against an explicit target tick/receipt range and current basis.

## Likely files touched

- `crates/echo-wasm-abi/src/kernel_port.rs`
- `crates/warp-core/src/witnessed_suffix.rs`
- `crates/warp-core/src/optic.rs`
- `crates/warp-core/src/cmd.rs`
- `crates/warp-core/src/provenance_store.rs`
- `crates/warp-core/tests/**`

## Acceptance criteria

- External unapply/undo submits an Intent against explicit target receipt/tick
  and current basis.
- The contract or installed handler produces inverse Intent(s) or typed
  obstruction.
- Original target ticks/receipts remain in provenance.
- Resulting inverse tick/receipt links back to the original target evidence.
- Missing inverse fragments, stale basis, compressed/cold unavailable history,
  or unmappable causal spans return typed obstruction/conflict posture.

## Non-goals

- Do not implement generic blind inverse of `WarpOp` as the user-facing model.
- Do not delete or rewrite historical ticks.
- Do not add app-specific text editing operations to Echo core.
- Do not solve all retention/compaction policy here.

## Test expectations

- RED/GREEN fixture appends an inverse tick rather than removing history.
- Provenance length increases.
- Original tick remains present.
- Missing inverse evidence obstructs deterministically.
