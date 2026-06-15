---
audit-date: 2026-06-15
audit-commit: 5f85dae5727d36acf4a82aad8d7cdb0488cb67be
audit-status: keep
topics:
    - sync
    - git-warp
    - suffix
accuracy: 0.90
issue: 489
findings:
    - claim: "one narrow bundle request shape exists over graph/lane/frontier identity"
      ruling: true
      evidence:
          filepath: crates/warp-core/src/witnessed_suffix.rs
          line: 1
    - claim: "one bundle export path gathers witnessed transitions plus payload references"
      ruling: true
      evidence:
          filepath: crates/warp-core/src/witnessed_suffix.rs
          line: 1
    - claim: "one import path normalizes against local frontier truth and returns outcome kinds"
      ruling: true
      evidence:
          filepath: crates/warp-core/src/witnessed_suffix.rs
          line: 1
    - claim: "one duplicate-import case proves idempotence"
      ruling: true
      evidence:
          filepath: crates/warp-core/src/witnessed_suffix.rs
          line: 1
    - claim: "git-warp import of Echo-exported suffixes is verified"
      ruling: false
---

<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo / git-warp witnessed suffix sync

- Lane: `up-next`
- Legend: `PLATFORM`
- Rank: `1`

## Why now

Echo now has honest publication for neighborhood and settlement runtime truth,
and Continuum has a cleaner "shared causal history, two temperatures" story.

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

- one narrow bundle request shape exists over graph/lane/frontier identity [🟢, 95%, [crates/warp-core/src/witnessed_suffix.rs:1](file:///Users/james/git/echo/crates/warp-core/src/witnessed_suffix.rs#L1)]
- one bundle export path gathers witnessed transitions plus payload references [🟢, 95%, [crates/warp-core/src/witnessed_suffix.rs:1](file:///Users/james/git/echo/crates/warp-core/src/witnessed_suffix.rs#L1)]
- one import path normalizes against local frontier truth and returns: [🟢, 95%, [crates/warp-core/src/witnessed_suffix.rs:1](file:///Users/james/git/echo/crates/warp-core/src/witnessed_suffix.rs#L1)]
    - admitted
    - staged
    - braided
    - conflict
    - obstructed
- one duplicate-import case proves idempotence [🟢, 95%, [crates/warp-core/src/witnessed_suffix.rs:1](file:///Users/james/git/echo/crates/warp-core/src/witnessed_suffix.rs#L1)]
- one first peer proving target is named:
    - `git-warp` import of Echo-exported suffixes [🔴, 90%, (no supporting evidence was found - may be false 🤥)]

## Repo Evidence

- `docs/design/0009-witnessed-causal-suffix-sync/design.md`
- `docs/design/0008-strand-settlement/design.md`
- `crates/warp-core/src/settlement.rs`
- `crates/warp-core/src/neighborhood.rs`
- `crates/echo-wasm-abi/src/kernel_port.rs`
- `crates/warp-wasm/src/warp_kernel.rs`
