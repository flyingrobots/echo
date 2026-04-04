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

Cycle 0003 — FIXED-TIMESTEP invariant. dt is fixed per worldline via
immutable `tick_quantum` at genesis. No per-tick variable dt. Wall-clock
time never enters semantic history. Cross-worldline operations require
identical `tick_quantum`. First entry in `docs/invariants/`.

## What is next?

Strand contract (KERNEL_strand-contract), then strand settlement
(KERNEL_strand-settlement). The order is deliberate: dt (done) →
strand identity → settlement semantics.

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
