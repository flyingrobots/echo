<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Retro: 0020-echo-cas-browser

Cycle: `0020-echo-cas-browser`
Design: [`docs/design/0020-echo-cas-browser/`](../../../design/0020-echo-cas-browser/)
Witness: [`witness/`](./witness/)

## Outcome

- Status: Accepted.
- Summary: Closed the `T-4-3-1` MemoryTier WASM compilation gate by proving
  `echo-cas` builds for `wasm32-unknown-unknown` and adding a CI job that runs
  the same target build on every PR.

## Evidence

- `.github/workflows/ci.yml` includes `Build echo-cas (wasm32)`, which runs
  `cargo build --target wasm32-unknown-unknown -p echo-cas`.
- `docs/design/0020-echo-cas-browser/echo-cas-browser.md` records the accepted
  WASM compilation gate and local witnesses.
- `docs/method/backlog/up-next/PLATFORM_echo-cas-js-bindings.md` preserves the
  deferred JavaScript binding follow-up as a separate visible backlog item.
- Verification:
    - `cargo build --target wasm32-unknown-unknown -p echo-cas`
    - `cargo test -p echo-cas`

## Drift Check

- This cycle did not add JavaScript bindings, persistence, DiskTier, async CAS,
  or browser-specific mutation semantics.
- `echo-cas` remains content-addressed storage only. CAS hashes still name
  bytes, not semantic read identity or Echo ontology.
- The follow-up JS binding task is visible in the backlog instead of hidden
  inside this completed cycle.

## Follow-Up

- Implement `WasmBlobStore` bindings in the separate
  `PLATFORM_echo-cas-js-bindings.md` backlog item.
- Decide later whether local pre-push verification should include a narrow
  `echo-cas` WASM smoke lane or leave this as CI-only.
