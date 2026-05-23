<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Prune legacy ttd-browser from Echo

Status: superseded.

`ttd-browser` proved useful browser/WASM ideas before `warp-ttd` existed as
its own debugger product. That history remains useful design archaeology, but
the ownership split is different now:

- Echo owns runtime truth and browser-hostable WASM substrate
- `warp-ttd` owns debugger session semantics and delivery adapters

The Echo-side resolution is not "grow the browser debugger here." The legacy
`ttd-browser` crate has been removed from the active workspace. Future
Echo/browser work should use:

- `warp-wasm` for Echo's app-safe WASM runtime boundary;
- `echo-wasm-abi` for canonical byte DTOs and the `KernelPort` contract;
- `warp-ttd` for debugger session semantics, product UI, and delivery adapters.

The remaining active work is not a `ttd-browser` narrowing task. It is:

- keep generated TTD protocol consumers downstream of `warp-ttd` when Echo
  needs them;
- build a release-grade app-safe JavaScript/browser client above `warp-wasm`;
- keep debugger/product semantics out of Echo core and out of Echo's app-safe
  browser client.

Related:

- Echo design `0005-echo-ttd-witness-surface`
- `warp-ttd` design `0018-browser-ttd-migration`
