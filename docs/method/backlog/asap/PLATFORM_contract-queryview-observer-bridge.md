<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Contract QueryView Observer Bridge

Status: core observer bridge checkpoint; generated query helper emission
remains.

Depends on:

- [Installed Wesley contract host dispatch](./PLATFORM_installed-wesley-contract-host-dispatch.md)
- [Contract-aware receipts and readings](../up-next/KERNEL_contract-aware-receipts-and-readings.md)
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

## RED

Added failing tests that prove:

- a missing observer returns typed `UnsupportedQuery`;
- an installed observer receives query id, vars bytes, and basis coordinate;
- emitted `QueryBytes` carry a `ReadingEnvelope` naming the authored observer
  plan;
- changing schema/plan identity, op id, vars, or basis changes artifact
  identity;
- bounded observers can report residual posture.

## GREEN

`ObservationService` routes QueryView/Query observations to an installed
contract observer when one is available.

Return:

```text
ObservationPayload::QueryBytes(bytes)
```

and a ReadingEnvelope that names contract/query identity.

## Acceptance criteria

- Unsupported query op returns typed obstruction/error, not fake empty success.
- Same query, basis, and vars produce stable reading identity.
- Changing schema hash, op id, vars, or basis changes identity.
- Bounded observers can report budget/residual posture.

## Remaining work

- `echo-wesley-gen` should emit query observer helper constructors against this
  core boundary.
- Contract-host packaging still needs an installed registry boundary that
  rejects unsupported contract operations before they become runtime-visible
  work or reads.

## Non-goals

- Do not add jedit-specific read APIs.
- Do not make generated query helpers call `KernelPort::observe` in this slice.
- Do not require full file materialization for the bridge test.
