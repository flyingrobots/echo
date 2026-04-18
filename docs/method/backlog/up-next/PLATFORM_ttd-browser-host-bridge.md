<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Narrow ttd-browser into an Echo browser host bridge

`ttd-browser` proved useful browser/WASM ideas before `warp-ttd` existed as
its own debugger product. That history is valuable, but the ownership split is
different now:

- Echo owns runtime truth and browser-hostable WASM substrate
- `warp-ttd` owns debugger session semantics and delivery adapters

So the next honest Echo-side move is not "grow the browser debugger here."
It is:

1. keep the useful browser bridge to Echo WASM
2. narrow `ttd-browser` toward host-adapter-friendly substrate access
3. stop adding standalone debugger/session/UI semantics in this crate

This should answer:

- what `ttd-browser` must still expose for a future Browser TTD delivery
  adapter
- which existing cursor/session/provenance responsibilities belong in a
  temporary compatibility layer only
- whether a rename such as `echo-browser-host` or similar would better match
  the crate's long-term job

Related:

- Echo design `0005-echo-ttd-witness-surface`
- `warp-ttd` design `0018-browser-ttd-migration`
