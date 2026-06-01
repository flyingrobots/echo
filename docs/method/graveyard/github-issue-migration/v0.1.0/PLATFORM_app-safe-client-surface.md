<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# App-Safe Client Surface

Status: v0.1.0 release blocker if JS/WASM client packages ship.

Depends on:

- [Product-facing intent outcome API](./PLATFORM_product-facing-intent-outcome-api.md)

## Why now

Rust/local host APIs are mandatory for `v0.1.0`. WASM, Node, or browser packages
are mandatory only if they are part of the release artifact set. If they ship,
they must be app-safe by construction.

## Required behavior

Application clients may:

- submit canonical intent bytes or generated intent helpers;
- observe intent outcomes;
- request bounded query readings;
- inspect returned receipt or reading evidence.

Application clients may not:

- tick or step the scheduler;
- access trusted runtime control;
- resume faulted heads;
- install privileged host adapters;
- mutate runtime state from query observers.

## Acceptance criteria

- Published client packages expose only the app-safe surface.
- Raw kernel objects or trusted control ports are not reachable from
  application JavaScript.
- High-level browser/application facades do not re-export the raw
  `dispatch_control_intent_trusted(...)` WASM host-control function.
- Query observer helpers remain read-only.
- Client examples do not call tick, step, start, or trusted runtime control.
- The Rust/local API remains usable if JS/WASM packages are deferred.

## Non-goals

- Do not require browser/Node packaging if those packages are not shipped in
  `v0.1.0`.
- Do not build streaming subscriptions.
- Do not expose raw privileged WASM exports to application code.
