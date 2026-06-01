<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Contract QueryView Observer Bridge

Status: core observer bridge, generated query helper checkpoint, installed
contract package registry boundary, and local installed mutation dispatch proof
complete.

Depends on:

- [Installed Wesley contract host dispatch](./PLATFORM_installed-wesley-contract-host-dispatch.md)
- [Contract-aware receipts and readings](../v0.1.0/KERNEL_contract-aware-receipts-and-readings.md)
- [0018 - Contract-Hosted File History Substrate](../../../design/0018-contract-hosted-file-history-substrate/design.md)

## Why now

`echo-wesley-gen` can emit query helpers that build
`ObservationRequest { frame: QueryView, projection: Query { ... } }`.

`warp-core` now routes `QueryView`/`Query` observations to installed contract
query observers when one is available for the generated query op id. The
observer receives the query id, canonical vars bytes, original request, resolved
causal basis, runtime, and provenance store as read-only context. It returns
bounded bytes and residual posture; Echo wraps the bytes in
`ObservationPayload::QueryBytes` and stamps the `ReadingEnvelope` with the
authored observer plan identity.

`echo-wesley-gen --contract-host` now emits std-only query observer helpers for
that boundary: deterministic authored observer plan identity, typed
context-vars decoders that return `Result`, and read-only observer constructors
that install host closures through `warp-core`.

## RED

Added failing tests that prove:

- a missing observer returns typed `UnsupportedQuery`;
- an installed observer receives query id, vars bytes, and basis coordinate;
- emitted `QueryBytes` carry a `ReadingEnvelope` naming the authored observer
  plan;
- changing schema/plan identity, op id, vars, or basis changes artifact
  identity;
- bounded observers can report residual posture.
- generated contract-host output includes query observer helper constructors;
- generated query observers decode typed vars from observer context and return
  typed observer errors for malformed canonical vars;
- generated mutation host helpers and query observer helpers compile and install
  together in a consumer smoke crate.

## GREEN

`ObservationService` routes QueryView/Query observations to an installed
contract observer when one is available.

Return:

```text
ObservationPayload::QueryBytes(bytes)
```

and a ReadingEnvelope that names contract/query identity.

`echo-wesley-gen --contract-host` emits generated query observer helpers that
bind Wesley query definitions to this read-only observer boundary without
adding application nouns to core.

## Acceptance criteria

- Unsupported query op returns typed obstruction/error, not fake empty success.
- Same query, basis, and vars produce stable reading identity.
- Changing schema hash, op id, vars, or basis changes identity.
- Bounded observers can report budget/residual posture.
- Generated query observer helpers install through
  `Engine::register_contract_query_observer`.
- Malformed generated query vars become typed observer errors, not `None`.

## Remaining work

- Contract-aware readings still need package identity carried through the
  evidence surface used by product-facing consumers.

## Non-goals

- Do not add jedit-specific read APIs.
- Do not make generated query helpers call `KernelPort::observe` in this slice.
- Do not require full file materialization for the bridge test.
