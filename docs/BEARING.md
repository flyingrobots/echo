<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# BEARING

Last updated: 2026-04-26.

This signpost summarizes current direction. It does not create commitments or
replace backlog items, design docs, retros, or CLI status. If it disagrees with
code, the code wins and this file should be corrected.

## Where are we going?

Current priority: make Echo's WARP optics observable, documented, and fast to
iterate without turning docs into a museum or a second codebase:

- Echo owns hot runtime truth in `warp-core`.
- Echo exposes current browser-hostable substrate through the WASM ABI, not a
  pile of historical ABI versions.
- Observer-relative reading metadata travels in `ReadingEnvelope`.
- Method cycles and dated audit ledgers track planning decisions.
- Local iteration speed is a first-class hill, because slow gates make every
  design/code/doc correction more expensive.

## What just shipped?

The runtime-doctrine cutover is no longer just design text:

- `0007` has runtime shape through `crates/warp-core/src/neighborhood.rs` and
  `NeighborhoodSiteService`.
- `0008` has runtime shape through `crates/warp-core/src/settlement.rs` and
  `SettlementService`.
- `crates/warp-wasm/src/warp_kernel.rs` exposes neighborhood and settlement
  surfaces through the WASM kernel boundary.
- `crates/echo-wasm-abi/src/kernel_port.rs` is currently ABI version 9 and
  makes observation requests name observer plan, optional instance, budget, and
  rights while carrying `ReadingEnvelope` inside observation artifacts.
- `docs/design/0019-reading-envelope-family-boundary/reading-envelope-family-boundary.md`
  names the shared read-side family boundary for authored observer plans,
  installed artifacts, runtime reading values, and retained reading identity.
- `docs/spec/SPEC-0009-wasm-abi.md` now documents the current ABI contract
  instead of pretending to preserve ABI v1-v5.

## What is next?

1. Audit `docs/` five documents at a time, score each one against code, and
   delete or relocate aggressively. The live docs corpus contains current,
   useful, navigable truth; git history is the archive.
2. Fold the Optic/Observer doctrine into the runtime path toward WARP optics,
   anchored by `docs/design/0011-optic-observer-runtime-doctrine/design.md`.
3. Improve local iteration by separating quick doc/code lanes from full release
   gates while keeping full verification before publication.
4. Implement QueryView observers against the accepted reading-envelope family
   boundary instead of adding a parallel read-result wrapper.

## What feels wrong?

- `docs/` still mixes living specs, Method backlog items, generated assets,
  historical audits, book sources, and stale top-level signposts.
- The docs site still has broken or stale navigation surfaces from earlier
  reorganizations.
- Local verification remains too coarse for doc-only or narrow ABI changes.
- We still lack one boring, inspectable agent boundary for "observe runtime,
  ask question, get evidence-backed answer."
