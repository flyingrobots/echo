<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo / git-warp witnessed suffix sync

- Lane: `up-next`
- Legend: `PLATFORM`
- Rank: `1`

## Why now

Echo now has honest publication for neighborhood and settlement runtime truth,
and Continuum has a cleaner "one graph, two temperatures" story.

What is still missing is the concrete handoff boundary:

- Echo should export witnessed causal suffixes
- Echo should import peer suffixes through normal admission
- hot/cold runtime handoff should not degrade into state synchronization
  folklore

## Hill

Prove one honest Echo runtime path for:

- `export_suffix(request) -> CausalSuffixBundle`
- `import_suffix(bundle) -> ImportSuffixResult`

without last-write-wins, silent branch mutation, or looped re-import.

## Done looks like

- one narrow bundle request shape exists over graph/lane/frontier identity
- one bundle export path gathers witnessed transitions plus payload references
- one import path normalizes against local frontier truth and returns:
    - admitted
    - staged
    - braided
    - conflict
    - obstructed
- one duplicate-import case proves idempotence
- one first peer proving target is named:
    - `git-warp` import of Echo-exported suffixes

## Repo Evidence

- `docs/design/0009-witnessed-causal-suffix-sync/design.md`
- `docs/design/0008-strand-settlement/design.md`
- `crates/warp-core/src/settlement.rs`
- `crates/warp-core/src/neighborhood.rs`
- `crates/echo-wasm-abi/src/kernel_port.rs`
- `crates/warp-wasm/src/warp_kernel.rs`
