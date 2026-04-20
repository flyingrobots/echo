<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Witnessed suffix admission shells

Refines:

- [Echo / git-warp witnessed suffix sync](./PLATFORM_echo-git-warp-witnessed-suffix-sync.md)

## Why now

Echo already has real publication surfaces for settlement and
neighborhood truth. The remaining risk is semantic downgrade:
export/import could still devolve into packet sync, patch shipping, or
state-sync folklore.

Paper VII's stronger target is tighter:

- remote suffixes are transported claim families
- import is ordinary witnessed admission after normalization to a
  comparable frontier
- the durable object is an admission shell / hologram, not a vague
  bundle of patches

This note exists to pin that stronger target before Echo bakes an
older sync contract into its runtime and ABI.

## What it should look like

- Echo exports a **witnessed suffix shell** rather than a naked patch
  stream.
- The export shell names:
    - graph / lane identity
    - source frontier and claimed base frontier
    - transported local site or comparable basis
    - payload / provenance references
    - witness required for replay, audit, and bounded revelation
- Import is an admission act, not a patch-apply loop.
- Import returns an explicit outcome algebra member:
    - admitted
    - staged
    - plural / braided
    - conflict
    - obstruction
- Independent imports are expected to converge up to shell
  equivalence, not merely "same eventual state."
- Divergence is never silently swallowed into "skipped writer" style
  behavior.

## Done looks like

- one export path produces a typed suffix shell / hologram
- one import path normalizes to a comparable frontier before deciding
- one proof test shows independent import order yields shell-equivalent
  retained results
- one non-independent case returns explicit plural/conflict/obstruction
  outcome rather than pretending commutativity
- the Echo / git-warp boundary speaks in suffix shells and admission
  outcomes, not state snapshots

## Repo evidence

- `docs/design/0009-witnessed-causal-suffix-sync/design.md`
- `docs/design/0008-strand-settlement/design.md`
- `crates/warp-core/src/settlement.rs`
- `crates/echo-wasm-abi/src/kernel_port.rs`

## Non-goals

- Do not settle the final network transport encoding here.
- Do not require full multi-peer trust policy in the first slice.
- Do not regress to "sync means same final state" as the only success
  criterion.
