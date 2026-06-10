<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Authority Boundary Audit

Status: initial local audit recorded; final release security review remains.

Depends on:

- [Product-facing intent outcome API](./PLATFORM_product-facing-intent-outcome-api.md)
- [App-safe client surface](./PLATFORM_app-safe-client-surface.md)
- [Reference trusted runtime host loop](./PLATFORM_reference-trusted-runtime-host-loop.md)

## Why now

The release is only credible if application code cannot reach trusted runtime
authority. Echo's central contract-host promise is that applications submit and
observe while the runtime owner controls ticks.

## Audit targets

Verify that application-facing paths cannot:

- tick, step, start, stop, or run the scheduler;
- access `TrustedKernelControlPort` or equivalent host-only capabilities;
- resume faulted heads;
- install privileged host adapters;
- mutate state through query observers;
- bypass package install compatibility checks;
- turn retry into hidden runtime behavior.

## Acceptance criteria

- [x] Tests prove app-facing dispatch cannot tick or access trusted runtime
      control.
- [ ] WASM/Node/browser exports are app-safe if those packages ship.
- [x] Generated helpers target app-safe request APIs or host-only install APIs
      explicitly.
- [x] Host-only APIs are documented as runtime-owner authority.
- [x] Security review records deferred risks before release candidate.

## Implemented local slice

`docs/design/v0.1.0-authority-boundary-audit.md` records the current
authority-boundary evidence and deferred risks. The local release witness keeps
application code on `TrustedRuntimeApp` and trusted runtime control on
`TrustedRuntimeHost`.

## Non-goals

- Do not build a sandbox or capability system beyond the release surface.
- Do not treat method names as authority boundaries.
- Do not make query observers a mutation API.
