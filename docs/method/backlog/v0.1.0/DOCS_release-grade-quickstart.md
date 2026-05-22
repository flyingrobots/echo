<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Release-Grade Quickstart

Status: initial executable quickstart implemented; product guide polish remains.

Depends on:

- [External contract proof fixture](./PLATFORM_external-contract-proof-fixture.md)
- [Reference trusted runtime host loop](./PLATFORM_reference-trusted-runtime-host-loop.md)
- [Versioned contract and API compatibility](./PLATFORM_versioned-contract-api-compatibility.md)

## Why now

`v0.1.0` should be buildable by a developer who did not help author the
internals. The quickstart is the executable proof that the public path is
understandable and honest.

## Required path

The quickstart should show:

1. write or use a small GraphQL contract;
2. generate Wesley helpers;
3. install the package into Echo;
4. submit an intent without ticking;
5. run a trusted host loop;
6. observe the intent outcome;
7. query a bounded reading;
8. inspect retained evidence;
9. replay locally.

## Acceptance criteria

- [x] Commands pass on a clean checkout.
- [x] The guide names which APIs are application-facing and which are host-only.
- [x] Examples do not call tick from application code.
- [ ] Error examples include unsupported operation/query and missing retention
      or bounded residual posture.
- [x] The guide links to the version compatibility policy.

## Implemented local slice

`docs/quickstart-local-contract-host.md` now documents the executable local
contract-host release witness, the app/host API split, compatibility checks,
and retention boundary. The command path is
`cargo xtask test-slice contract-path-release`.

## Non-goals

- Do not write a marketing landing page.
- Do not require a full editor, automation app, or distributed deployment.
- Do not promise streaming subscriptions.
