<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Reference Trusted Runtime Host Loop

Status: v0.1.0 release blocker.

Depends on:

- [Product-facing intent outcome API](./PLATFORM_product-facing-intent-outcome-api.md)

## Why now

The release model is clear:

```text
application submits intents
trusted runtime owner ticks
application observes outcomes
```

Echo needs a documented local host loop that demonstrates the trusted runtime
owner role without requiring a daemon or distributed runtime.

## Responsibilities

The reference host loop owns:

- contract package installation;
- trusted runtime control;
- fixed logical tick or until-idle policy;
- fault recovery authority;
- receipt publication;
- query service.

## Acceptance criteria

- A clean example hosts one installed contract package locally.
- Application code submits through the app-safe API only.
- The host loop owns scheduler control and tick cadence.
- The host can run until idle without exposing that capability to application
  code.
- Runtime-local fault recovery is host-owned.
- The example can observe intent outcomes and query readings after ticks.

## Non-goals

- Do not build a production daemon.
- Do not make wall-clock cadence semantic history.
- Do not let application code access trusted runtime control.
