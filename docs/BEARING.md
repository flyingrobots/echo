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

The parity runway is now explicit in design:

- `0007` defines braid geometry and native neighborhood publication
- `0008` defines strand settlement as compare -> plan -> import ->
  conflict artifact
- `0011` explains the neighborhood publication stack from admission truth to
  host-visible `NeighborhoodCore`

Runtime and boundary truth now match that direction more closely:

- `NeighborhoodSite` is live in kernel/runtime truth
- `NeighborhoodCore` is now exported through the wasm ABI boundary

Earlier cleanup also removed the old local browser debugger product path
(`warp-viewer`, session hub/gateway/client, `ttd-app`). Repo truth now points
at:

- Echo browser/WASM host bridge surfaces
- `warp-ttd` as the browser debugger destination
- generated protocol consumers as downstream artifacts, not protocol owners

## What is next?

Two implementation cuts and one contract cut:

1. make `0008` real in kernel/runtime truth:
   compare/plan/import/conflict artifact publication
2. land one Wesley-generated proof slice against the shared Continuum observer
   contract, then narrow `ttd-browser` / `echo-session-proto` around that
   reality
3. move host consumers such as `continuum-demo` onto Echo's native
   neighborhood-core boundary instead of app-level reconstruction

## What feels wrong?

- settlement nouns exist only as a new design packet and placeholder event
  kinds, not a shipped compare/plan/import path.
- Echo's richer runtime schema (typed IDs, dual tick clocks, ingress routing,
  scheduler introspection) still is not surfaced cleanly through the canonical
  shared observer/debugger contract.
- the host/demo side still has places where neighborhood-core shape is
  reconstructed outside Echo instead of consumed from the new native boundary.
- Echo still lacks an explicit CLI/MCP agent boundary, so agent use depends on
  repo-local APIs and bridge folklore instead of one inspectable surface.
