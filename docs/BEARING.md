<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# BEARING

This signpost summarizes direction. It does not create commitments or
replace backlog items, design docs, retros, or CLI status.

## Where are we going?

Current priority: finish the Echo-side cutover to the Continuum ownership
split:

- Echo owns hot runtime truth and browser-hostable WASM substrate
- `warp-ttd` owns debugger session semantics and browser delivery
- shared protocol truth stays canonical outside Echo

## What just shipped?

Prototype viewer-stack removal. Echo no longer carries the old local browser
debugger product path (`warp-viewer`, session hub/gateway/client, `ttd-app`).
Repo truth now points at:

- Echo browser/WASM host bridge surfaces
- `warp-ttd` as the browser debugger destination
- generated protocol consumers as downstream artifacts, not protocol owners

## What is next?

Two bounded cleanup cuts:

1. narrow `ttd-browser` into a real Echo browser host bridge
2. split `echo-session-proto` so retained TTD/browser frame contracts stop
   living in the same conceptual bucket as dead hub/WVP transport residue

## What feels wrong?

- `ttd-browser` still carries too much legacy debugger-shaped surface for a
  crate that should now be a host bridge.
- `echo-session-proto` still mixes retained TTDR/EINT/browser framing with the
  older WVP/session-hub protocol family.
- Echo's richer runtime schema (typed IDs, dual tick clocks, ingress routing,
  scheduler introspection) still is not surfaced cleanly through the canonical
  `warp-ttd` protocol.
