<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Reconcile Relocated Wesley Echo Schemas

Status: inbox.

## Why now

During the Echo contract-hosting roadmap work, old Wesley-local Echo SDL files
appeared in an untracked `schemas/wesley-relocated/` directory. They described
Echo CAS-facing payloads and WASM ABI response shapes that had been removed
from generic Wesley during the domain-empty extraction.

Those files were not retained as active Echo schema truth because they
duplicated and diverged from current Echo-owned sources:

- current runtime schema fragments under `schemas/runtime/`;
- current WASM ABI DTOs under `crates/echo-wasm-abi`;
- current worldline/storage shapes under `crates/warp-core`;
- current registry boundaries under `echo-registry-api` and `echo-wesley-gen`.

Carrying stale SDL under `schemas/` would make future agents treat old Wesley
artifacts as active schema authority.

## What it should look like

Create a reconciliation note or design packet that compares any old relocated
Wesley Echo schema material against current Echo truth.

The packet should answer:

- whether any old `@wes_codec` / `@wes_version` annotation ideas should become
  current Echo schema policy;
- whether any CAS-facing payload shape should be re-authored under
  `schemas/runtime/` or another Echo-owned schema family;
- whether `echo-wasm-abi` should ever get GraphQL SDL source, or stay
  hand-authored Rust DTOs with spec docs;
- how `RegistryInfo.registry_version` should be represented across
  `echo-wasm-abi` and `echo-registry-api`;
- whether old relocated SDL should be archived as historical reference, or left
  only in Git/Wesley history.

## Acceptance criteria

- The packet cites current repo truth before citing old relocated material.
- No stale relocated SDL is committed under active `schemas/` paths.
- If old SDL is archived, it lives under an explicit non-authoritative archive
  path and has a warning that it is stale reference only.
- If old SDL concepts are reused, they are re-authored into current schema or
  code surfaces instead of copied forward blindly.
- The decision preserves the ownership split:
    - Echo owns runtime, CAS, WASM ABI, and engine-specific schema truth.
    - `warp-ttd` owns host-neutral debugger protocol truth.
    - Continuum owns shared cross-engine contract-family truth.
    - Wesley compiles owner-provided schemas; it does not own product schemas.

## Non-goals

- Do not restore `schemas/wesley-relocated/` as active schema truth.
- Do not duplicate current Rust DTOs as stale GraphQL SDL.
- Do not change ABI shapes in this card.
- Do not add generation plumbing in this card.
- Do not move `warp-ttd` or Continuum protocol schemas into Echo.
