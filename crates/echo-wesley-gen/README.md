<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# echo-wesley-gen

CLI tool that emits Echo Rust structs, operation registries, and optic helper
functions from Wesley contract data.

Wesley is the compiler seam between authored application contracts and Echo's
generic runtime. Generated application helpers build canonical intent/query
requests. Generated contract-host helpers install mutation handlers and
read-only query observers. Neither surface gives application code tick
authority.

The preferred input is GraphQL SDL lowered directly through the published
`wesley-core` crate. The older `echo-ir/v1` JSON stdin path is retained for
fixtures and compatibility while consumers move off the historical JavaScript
generator.

Echo's external Edict provider uses a separate strict source contract:
[`schemas/edict-provider/echo-provider-semantics-v1.json`](../../schemas/edict-provider/echo-provider-semantics-v1.json).
`provider_semantics::parse_provider_semantic_source_v1(...)` validates explicit
source bytes without filesystem discovery, normalizes set-like declarations,
and rejects duplicate coordinates or dangling authority, type, failure,
obstruction, profile, budget, capability, adapter, and schema references with
stable error kinds. It also checks exhaustive obstruction mappings,
bounded type closure, Edict Core alias ownership, failure identifiers, exact
fact-domain/authority families, capability-owned write classes, explicit
semantic discharge, and complete lawpack/target-profile/invocation/domain/root
contracts. Every runtime effect resolves to exactly one native capability or
direct adapter, and every lawpack adapter has a unique target-profile selector.
Authority-facts outputs name Edict issue #157 as their canonical contract owner.
The separately declared provider manifest pins the later package root's exact
ABI and provider coordinate and cannot inventory itself.
This source is not accepted through the tolerant historical `echo-ir/v1` path.

The Edict-owned schema authority is a separate explicit input. The exact
Apache-2.0 contract pack checked under
[`schemas/edict-provider/contracts/v1/`](../../schemas/edict-provider/contracts/v1/README.md)
is admitted through
`provider_contract_pack::admit_provider_contract_pack_v1(...)`. Admission
requires the pinned Edict PR #162 CDDL and manifest publication, verifies every
embedded contract resource and provenance record, and performs no discovery or
mutable coordinate resolution. This authenticates the schema publication; it
does not by itself claim that a generated artifact is a valid schema instance.

`provider_generation::build_provider_generation_input_v1(...)` joins that
admitted pack with exact Echo semantic-source bytes and the checked versioned
generation settings. It constructs Wesley's canonical extension-generation
input in memory, binds exact source materials for later provenance verification,
and derives the six primary output roles from the validated source. The current
closure carries an empty Wesley Shape and operation catalog because it declares
no GraphQL authority source; it does not synthesize `a.b@1.t` as GraphQL.
While the checked settings select no Shape source, any semantic input that
declares GraphQL authority fails closed until explicit SDL bytes are supported.
The normalized semantic model is stable under set ordering, while the
generation-input digest changes when raw authored bytes change because it binds
the exact source artifact.

## Usage

```bash
# Generate Rust code directly from GraphQL SDL
cargo run -p echo-wesley-gen -- --schema schema.graphql

# Write generated Rust from GraphQL SDL to a file
cargo run -p echo-wesley-gen -- --schema schema.graphql --out generated.rs

# Generate Rust code to stdout
cat ir.json | cargo run -p echo-wesley-gen --

# Write to a file
cat ir.json | cargo run -p echo-wesley-gen -- --out generated.rs

# Emit std-only warp-core contract-host helpers for installed mutation handlers
# and query observers
cat ir.json | cargo run -p echo-wesley-gen -- --contract-host --out generated.rs
```

## Notes

- Supports ENUM and OBJECT kinds from Wesley IR.
- Preserves per-operation directive metadata as `OpDef::directives_json`; Echo
  admission tooling owns any interpretation of `wes_footprint`.
- Emits footprint certificate constants for operations with `@wes_footprint`;
  those certificates include the generated Rust artifact manifest hash and the
  operation argument shape, and hosts can verify them through
  `echo_registry_api::verify_contract_artifact` before treating the generated
  artifact as compile-time-certified.
- GraphQL SDL operation ids are derived deterministically and fail closed on
  collision. The generator never increments a collided id because operation ids
  are persisted ABI.
- Generated query optic helpers use Echo ABI's domain-separated BLAKE3
  `query_vars_digest_v1(...)`; ad hoc variable digests are not accepted for
  retained reading identity.
- `--contract-host` emits opt-in, std-only mutation helpers for installing
  generated operations as `warp-core` command rules. The generated surface
  matches scheduler-materialized EINT runtime ingress events by op id, decodes
  typed vars, provides the base runtime-ingress read footprint, and builds a
  `RewriteRule` from host-supplied executor and footprint functions. It does
  not generate the application mutation body or grant application code tick
  authority.
- `--contract-host` also emits std-only query observer helpers for installing
  generated queries as read-only `warp-core::ContractQueryObserver` instances.
  The generated surface stamps deterministic authored observer plan identity,
  decodes typed vars from observer context with `Result`, and accepts a host
  closure that returns `ContractQueryObserverResult` or
  `ContractQueryObserverError`. Query observers cannot tick the runtime or write
  through this generated boundary. They do not receive mutable runtime,
  scheduler control, or `TickDelta`.
- Optional fields become `Option<T>`; lists become `Vec<T>` (wrapped in Option when not required).
- Unknown scalar names are emitted as identifiers as-is (so ensure upstream IR types are valid Rust idents).
- Runtime optic artifact imports preserve Wesley-owned canonical admission
  requirement bytes, codec id, and digest directly from
  `OpticAdmissionRequirementsArtifact`; Echo stores them as opaque registry
  payload and does not reserialize Wesley requirement structs.
- Edict provider lawpacks, profiles, source-partitioned authority facts,
  generated-artifact profiles, schemas, provenance, review JSON, and the later
  package manifest are generated projections of the checked Echo provider
  semantic source. They are not alternate authored inputs.
