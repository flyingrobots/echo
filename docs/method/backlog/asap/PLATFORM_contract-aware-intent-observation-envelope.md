<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Existing EINT, Registry, And Observation Boundary Inventory

Status: active design correction and inventory candidate.

Depends on:

- [Wesley compiled contract hosting doctrine](./PLATFORM_wesley-compiled-contract-hosting-doctrine.md)
- [Reading envelope family boundary](../up-next/PLATFORM_reading-envelope-family-boundary.md)
- [Observer plans and reading artifacts](./PLATFORM_observer-plan-reading-artifacts.md)

## Why now

Echo already exposes generic WASM calls such as `dispatch_intent(...)` and
`observe(...)`. It also already has EINT v1 intent envelopes, registry metadata
exports, a generic `echo-registry-api::RegistryProvider`, and
`echo-wesley-gen` output that includes a generated registry provider.

The risk now is planning duplicate substrate. Echo does not need a second
intent envelope or a second registry model before a consumer can use it. The
next slice should inventory the existing path and identify the narrow missing
bridge for Wesley-generated app consumers.

## Current repo truth to preserve

- `echo-wasm-abi` defines EINT v1 as
  `"EINT" || op_id:u32le || vars_len:u32le || vars`.
- `warp-wasm` exposes `dispatch_intent(...)` as the write/control ingress.
- `KernelPort::dispatch_intent(...)` is already app-agnostic over canonical
  intent bytes.
- `warp-wasm` exposes `get_registry_info`, `get_codec_id`,
  `get_registry_version`, and `get_schema_sha256_hex`.
- `echo-registry-api` defines the app-supplied `RegistryProvider` interface.
- `echo-wesley-gen` emits op ids, op catalogs, `GeneratedRegistry`, and
  `REGISTRY`.
- `observe(...)` already returns `ObservationArtifact` with `ReadingEnvelope`
  metadata for built-in observations.
- Existing schema validation helpers in `warp-wasm` are currently test-only.

## Questions to answer

1. Should app-level generated code validate op ids and vars before calling
   `dispatch_intent(...)`, leaving Echo to ingest opaque canonical EINT bytes?
2. Should `warp-wasm` link an app-supplied `RegistryProvider` and reject
   unknown op ids or malformed vars at the WASM boundary?
3. Is EINT v1 sufficient for app contract identity when one generated registry
   is installed, or does multi-family hosting require an EINT v2 or registry
   scope rule?
4. How should Wesley-generated `QUERY` operations relate to `observe(...)`,
   `ObservationRequest`, `ReadingEnvelope`, and built-in observer plans?
5. Is `RegistryInfo` metadata enough for `jedit` handshakes, or does browser
   code need the full generated op catalog at runtime?

## What it should look like

Add a design/inventory packet before writing RED tests.

## Acceptance criteria

- The packet cites the existing EINT, registry metadata, `RegistryProvider`,
  `echo-wesley-gen`, and observation/read-envelope surfaces.
- The packet states whether the next implementation validates op ids in
  app-level generated code, Echo's WASM boundary, or both.
- The packet states whether EINT v1 remains the contract path for the next
  consumer proof.
- The packet states the first narrow missing bridge for generated query/read
  operations.
- Existing backlog cards are corrected so they do not claim that basic intent
  ingress or registry metadata are missing.

## Non-goals

- Do not create a new intent envelope unless the inventory proves EINT v1 is
  insufficient.
- Do not implement a new registry model.
- Do not move app-specific validation into Echo core by default.
- Do not add dynamic loading.
- Do not add application payload types.
- Do not change `jedit`.
