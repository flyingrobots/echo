<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Contract QueryView Observer Bridge

Status: RED/GREEN implementation slice.

Depends on:

- [Installed Wesley contract host dispatch](./PLATFORM_installed-wesley-contract-host-dispatch.md)
- [Contract-aware receipts and readings](../up-next/KERNEL_contract-aware-receipts-and-readings.md)
- [0018 - Contract-Hosted File History Substrate](../../../design/0018-contract-hosted-file-history-substrate/design.md)

## Why now

`echo-wesley-gen` can emit query helpers that build
`ObservationRequest { frame: QueryView, projection: Query { ... } }`, but
`ObservationService` currently rejects QueryView. Generated read helpers stop at
request construction until Echo has a generic observer bridge.

## RED

Add a failing test:

- install a contract query observer for one query op id;
- call `observe(QueryView/Query)` with generated vars bytes;
- assert current behavior is not `UnsupportedQuery`;
- assert the observer receives query id, vars bytes, and basis coordinate.

## GREEN

Route QueryView/Query observations to an installed contract observer when one is
available.

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

## Non-goals

- Do not add jedit-specific read APIs.
- Do not make generated query helpers call `KernelPort::observe` in this slice.
- Do not require full file materialization for the bridge test.
