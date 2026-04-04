<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# BEARING

This signpost summarizes direction. It does not create commitments or
replace backlog items, design docs, retros, or CLI status.

## Where are we going?

Current priority: build out METHOD tooling and align Echo with
warp-ttd (the official time-travel debugger) and git-warp (the
reference WARP substrate).

## What just shipped?

Cycle 0004 — strand contract. `Strand`, `BaseRef`, `StrandRegistry`
types in `warp-core/src/strand.rs`. Ten invariants (INV-S1 through
INV-S10). Invariant validation on registry insert. Hard-delete drop
with `DropReceipt`. Second entry in `docs/invariants/`.

Prior: cycle 0003 — FIXED-TIMESTEP invariant.

## What is next?

Strand settlement (KERNEL_strand-settlement). The trilogy: dt (done) →
strand contract (done) → settlement semantics.

## What feels wrong?

- The docs corpus is still ~25% fiction. The audit is written; the
  cleanup hasn't been pulled as a cycle yet.
- The warp-ttd protocol was shaped by git-warp's simpler model. Echo's
  richer runtime schema (typed IDs, dual tick clocks, ingress routing,
  scheduler introspection) isn't surfaced through the protocol yet.
- Echo's pre-warp-ttd crates (echo-ttd, ttd-browser, ttd-protocol-rs)
  need reconciliation with warp-ttd's canonical schema.
- RED and GREEN can't be separate commits under the current lint
  policy (clippy denies `todo!()`). The discipline is preserved but
  the commit structure doesn't show it.
