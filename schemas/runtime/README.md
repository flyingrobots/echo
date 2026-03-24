<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# ADR-0008 Runtime Schema Fragments

These GraphQL SDL fragments are the **human-authored source of truth** for the
Phase 8 ADR-0008 runtime schema freeze.

They are intentionally narrower than the browser/TTD protocol schema:

- they cover stable runtime boundary types only,
- they do **not** include ADR-0009 transport/conflict types,
- and they do **not** replace the current `echo-wasm-abi` adapter DTOs yet.

## Current Fragments

- [artifact-a-identifiers.graphql](artifact-a-identifiers.graphql)
  Runtime identifiers and logical counters.
- [artifact-b-routing-and-admission.graphql](artifact-b-routing-and-admission.graphql)
  Deterministic ingress routing and head-admission policy types.
- [artifact-c-playback-control.graphql](artifact-c-playback-control.graphql)
  Playback control modes and seek-follow-up semantics.
- [artifact-d-scheduler-results.graphql](artifact-d-scheduler-results.graphql)
  Scheduler lifecycle/result metadata and supporting control-plane types.

## Intent

Phase 8 freezes the runtime shape first and wires generation second.

That means these files are allowed to exist before:

- `cargo xtask wesley sync` grows a runtime-schema path,
- Wesley IR is vendored for the runtime freeze set,
- or generated Rust replaces hand-written runtime mirrors.

## Validation

Run the local fragment validator before touching any generation plumbing:

```sh
pnpm schema:runtime:check
```

The validator does two narrow jobs:

- parse-check the SDL fragments via the repo's existing `prettier --check`
  toolchain path,
- and verify that every referenced runtime type is defined somewhere inside the
  local `schemas/runtime/` fragment set.

This keeps Phase 8 moving without pretending Wesley is already stable enough to
own the runtime freeze loop.

The current Echo-side mismatch inventory lives in:

- [Phase 8 Runtime Schema Conformance Audit](../../docs/plans/phase-8-runtime-schema-conformance.md)
- [Phase 8 Runtime Schema Mapping Contract](../../docs/plans/phase-8-runtime-schema-mapping-contract.md)

## Planned Output Contract

Generation is explicitly deferred, but the intended artifact boundary is:

- `schemas/runtime/*.graphql`
  Human-authored source fragments for Artifacts A-D.
- `schemas/runtime/generated/runtime-schema.graphql`
  Planned normalized single-file runtime schema bundle.
- `schemas/runtime/generated/runtime-ir.json`
  Planned vendored Wesley IR snapshot for the frozen runtime schema.
- `schemas/runtime/generated/runtime-manifest.json`
  Planned vendored schema manifest/metadata for deterministic regeneration.
- `crates/echo-runtime-schema/src/lib.rs`
  Current Echo-local shared Rust home for runtime logical counters and core
  opaque ids/key types; planned generated home once runtime Wesley sync exists.

`echo-wasm-abi` remains adapter-owned even after that crate exists. It should
preserve the same logical-counter and opaque-id semantics at the host boundary
without widening the runtime crate's feature surface, and it should keep
explicit wrappers where the CBOR wire contract differs from the runtime
semantic type.

Not every frozen schema type must move into `echo-runtime-schema`. Phase 8
intentionally leaves runtime-local behavior types such as `IntentKind` and
`InboxAddress` hand-written in `warp-core` until a real shared/generated
consumer exists.

## Notes

- These files are SDL **fragments**, not a standalone executable GraphQL API.
- Comments here carry semantic constraints that current GraphQL type syntax
  cannot express directly, such as opaque-hash ids and logical-counter rules.
- Generation plumbing (`cargo xtask wesley sync`) still does not exist for this
  runtime schema tree; Phase 8 is pinning and validating source files before
  generation.
