<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Registry Provider Wiring And Host Boundary Decision

Status: active planned design decision.

Depends on:

- [Existing EINT, registry, and observation boundary inventory](./PLATFORM_contract-aware-intent-observation-envelope.md)

## Why now

Echo already has a generic registry interface:
`echo-registry-api::RegistryProvider`. `echo-wesley-gen` already emits a
`GeneratedRegistry` implementation and op catalog from Wesley IR.

The missing decision is not "invent a registry." The missing decision is how
the existing generated registry is wired into consumers and whether Echo itself
should consult it during `dispatch_intent(...)` / `observe(...)` or leave that
validation to app-level generated code.

## What it should look like

Make one explicit host-boundary decision:

Option A:

- app-level generated code validates op ids and vars using `REGISTRY`
- app-level generated code packs EINT
- Echo ingests canonical EINT bytes opaquely
- Echo exposes registry metadata for handshake

Option B:

- `warp-wasm` or the installed kernel links an app-supplied
  `RegistryProvider`
- Echo rejects unknown op ids or malformed vars before ingress
- generated query/read ops get a clear path through `observe(...)`

Either option must preserve Echo's app-agnostic substrate boundary.

## Acceptance criteria

- The decision cites `echo-registry-api`, `echo-wesley-gen`, EINT v1, and
  current `RegistryInfo` exports.
- The decision explains where op id lookup happens for the first consumer.
- The decision explains where vars payload validation happens for the first
  consumer.
- The decision explains whether generated `QUERY` ops are app-level helpers,
  built-in `observe(...)` requests, or a future observe bridge.
- Any later implementation reuses `RegistryProvider` rather than creating a
  parallel registry abstraction.

## Non-goals

- Do not implement WASM dynamic module loading.
- Do not fetch contracts over the network.
- Do not add jedit-specific registration.
- Do not invent a new registry trait while `RegistryProvider` is sufficient.
- Do not make the host boundary a broad runtime facade.
