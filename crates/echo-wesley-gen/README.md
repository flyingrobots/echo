<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# echo-wesley-gen

CLI tool that emits Echo Rust structs, operation registries, and optic helper
functions from Wesley contract data.

Wesley is the compiler seam between authored application contracts and Echo's
generic runtime. Generated application helpers build canonical intent/query
requests. Generated contract-host helpers build mutation-rule and read-only
observer material for a trusted Echo host to inspect and install; the helpers
do not install themselves. Neither surface gives application code tick
authority.

The preferred input is GraphQL SDL lowered directly through the published
`wesley-core` crate. The older `echo-ir/v1` JSON stdin path is retained for
fixtures and compatibility while consumers move off the historical JavaScript
generator. Direct SDL lowering records the exact pinned `wesley-core` version
in the provenance bound into each generated Rust artifact hash.

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

`provider_canonical` implements the publication's exact
`edict.canonical-cbor/v1` subset and `edict.digest/v1` domain frame.
`AdmittedProviderContractPackV1::validate_contract_bytes(...)` first requires
those exact canonical bytes and then validates the decoded value against the
named contract's owning root in the authenticated CDDL. Canonical decoding or
hashing alone is not schema admission, and even successful owning-root
validation does not install an artifact or confer Echo runtime authority.

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

`provider_artifacts::generate_provider_primary_artifacts_v1(...)` projects the
normalized semantic model into five canonical-CBOR primary artifacts, fourteen
declarative generated resources, and one exact self-contained CDDL artifact.
Every canonical value is validated against its generated owning root; the
Edict-owned lawpack, target-profile, authority-facts, export, intrinsic, and
operation-profile values are also checked independently against the admitted
contract pack. Manifest edges use Edict domain-framed digests, while Wesley
content references bind exact emitted bytes. Direct adapters and
operation-local obstruction mappings remain explicit, and invocation posture
comes from the admitted optic: affect/reintegration produces a mutation while
revelation/projection produces a bounded observer. These artifacts describe
provider semantics only; they do not install a package or grant Echo runtime
authority.

`provider_provenance::generate_provider_generation_provenance_v1(...)` builds
Wesley's canonical provenance manifest from the generation input, primary
closure, and caller-supplied exact generator component bytes. It immediately
verifies the generator, all three source artifacts, and the five canonical-CBOR
artifacts plus raw CDDL schema. The fourteen resources are transitively bound by
the primary bytes and are not restated as top-level emissions. Provenance and
review are likewise excluded from that set so neither document claims its own
digest. The API performs no executable, path, environment, process, registry,
clock, or network discovery. Each primary closure records the exact Wesley input
digest that produced it, preventing mixed-input attribution even when requested
roles match. Generator coordinates must also be distinct from all declared
source-artifact, generated-artifact, resource, provider, and package-manifest
coordinates.

`provider_review::generate_provider_generation_review_v1(...)` derives Wesley's
canonical `GenerationReviewV1` from that verified provenance wrapper. It copies
the exact input, provenance, generator, projection-role, source, and emitted
identities into deterministic JSON while Wesley keeps the `authoritative` field
false by construction and deserialization. Review is derived tooling evidence;
it neither replaces provenance verification nor admits or authorizes anything
in Echo.

`provider_corpus` frames the generator's exact source and dependency-lock
closure, re-verifies the complete generation chain, and renders the checked
22-file corpus under `schemas/edict-provider/generated/v1/`. The frame uses a
fixed coordinate, crate version, repo-relative source paths, and exact
compile-time bytes for the provider modules, dedicated corpus CLI, manifests,
workspace lockfile, and Rust toolchain. It deliberately excludes authored
semantic/settings/contract inputs already bound by Wesley and every generated
output, preventing a circular generator identity. This is source/dependency-lock
identity, not an executable-build or supply-chain attestation.

`provider_package` purely assembles the 22 generated files and two explicit
provider components into one 24-member, non-self-referential package closure,
then derives the separate `edict.provider-manifest/v1` JSON file. The Echo-owned
package root is a domain-framed canonical-CBOR value over the exact manifest
routes, 24 schema bindings (nine invocation domains, the generated artifact
profile, and 14 generated-resource domains), and raw SHA-256 of every
non-manifest member.
Canonical-CBOR artifact routes retain their Edict domain-framed identities;
CDDL, Wesley evidence JSON, components, and every physical member retain raw
exact-byte identities. Digest admission requires exact deterministic manifest
JSON, one shared semantic-source/generator reference proven against the packaged
Wesley provenance and review, the exact sorted 25-file inventory, and an
external caller pin. The resulting proof authenticates package occurrence only.
Edict schema-registry construction and component contract preflight remain
separate required crossings before guest execution. `provider_package` can then
consume that `DigestAdmittedProviderPackageV1` with an independently
Echo-admitted `AdmittedProviderContractPackageV1`. Exact
`echo.edict-provider@1` coordinate and strict lowercase `sha256:` package-root
agreement produce an opaque `DigestCorroboratedProviderContractPackageV1`.
This composition does not derive registry semantics from package bytes, install
or invoke anything, or grant runtime authority.

`provider_package::install_digest_corroborated_provider_contract_package_v1(...)`
is the proof-owning installation adapter. It consumes the corroborated token and
delegates through `warp-core`'s sealed `ProviderContractPackageInstallerV1`
runtime-owner port. The `TrustedRuntimeHost` lower primitive accepts the
already-corroborated reference and admitted proposal; it does not authenticate
package bytes and is not exposed through the application handle. A successful
installation retains the exact occurrence and provider reference, complete
owned provider registry, and mutation-rule identity in a distinct provider
record. Echo atomically adds provider-package, package-root, mutation-operation,
and shared scheduler-rule indexes without fabricating legacy Wesley/GraphQL
metadata or installed-contract evidence. Installation invokes no callbacks.
Provider-native ingress and invocation, WAL persistence, receipts,
observations, and the separate generated bounded-read path remain subsequent
Echo crossings.

`echo-edict-provider-package` is the explicit publication boundary for that
distribution. Before any filesystem action it proves that the package's 22
`generated/` members are the current checked provider corpus introduced by #652,
then writes the two
checked components and derived manifest through the same no-follow,
unexpected-entry-refusing filesystem boundary used by the artifact corpus. The
checked 25-file result lives under `schemas/edict-provider/package/v1/`.
`--check` reports sorted missing, changed, or unexpected package members and
never repairs or creates them. The shared boundary validates a strictly sorted,
unique expected inventory before resolving the root, caps that inventory at 256
files and 64 MiB, caps an actual scan at 1,024 entries, and never opens or reads
an unexpected regular file.

`echo-edict-provider-assets` maintains the exact 38-file package-local carrier
tree under `assets/v1/`. The physical carrier names are packaging locations,
not replacement source identities: generator provenance continues to name the
original repository-relative authored paths. Read-only mode requires every
carrier to match its fixed owner, requires generated artifacts and components
to match their checked package copies, and can prove that `cargo package --list`
selects exactly the complete carrier inventory. Explicit `--write` mode copies
authoritative owners without requiring the temporarily stale package copy,
allowing the honest staged sequence artifact generation, carrier sync, package
generation, then final carrier corroboration. Each fixed owner leaf is opened
without following its final symbolic link and read twice through the same
retained descriptor; file-type, length, or byte disagreement refuses a moving
owner. It never discovers a preferred owner or normalizes authored bytes. The
20-file generator source closure and its carriers include the exact manifest
and implementation occurrences for both `echo-edict-canonical` and
`echo-registry-api`, including the provider-generic registry vocabulary, so
provenance binds the canonicalization, operation-id, and registry laws actually
executed by provider generation.

`echo-edict-provider-assets -- --sync-component-resources` adds the lowerer and
verifier resource trees to the ordinary package-carrier operation. Without
`--write` it checks both the carrier and exact component resources; with
`--write` it synchronizes both. `--check-package-list` composes with the same
invocation, so no accepted flag is silently skipped.

The isolated `tests/edict-provider-host-v1` gate pins Edict revision
`c75c3f550d049485ba00eae0dc272c6dd6aca11f` and consumes the exact checked
package. It constructs the native 24-domain schema registry, validates all 19
canonical package members under their owning roots, binds every lawpack and
target-profile resource field to exact packaged bytes, prepares both frozen-WIT
components, and validates both request kinds without invoking guest code. That
is pre-execution package readiness, not Echo installation or runtime authority.

The dedicated `echo-edict-provider-artifacts` binary is the explicit filesystem
boundary. The caller-supplied `--out` path is resolved once under ambient
filesystem authority; its final corpus-root entry is opened or created without
following a symlink. Every operation beneath that acquired root inventories and
writes through retained, no-follow directory capabilities. Generation refuses
every unexpected entry it observes before creating or replacing an expected
path; otherwise it replaces only the 22 expected leaves and preserves an
existing destination if replacement fails. `--check` renders expected bytes in
memory, reads the target tree through the same bounded handles, reports sorted
missing/changed/unexpected drift, and returns before every directory-creation or
write path. This boundary does not claim that unrelated ancestors used to locate
the requested root are symlink-free.

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

# Emit std-only warp-core contract-host material for trusted host installation
cat ir.json | cargo run -p echo-wesley-gen -- --contract-host --out generated.rs

# Rebuild the checked Edict provider artifact corpus from exact inputs
cargo +1.90.0 run --locked -p echo-wesley-gen \
  --bin echo-edict-provider-artifacts --

# Report checked-corpus drift without rewriting anything
cargo +1.90.0 run --locked -p echo-wesley-gen \
  --bin echo-edict-provider-artifacts -- --check

# Publish the self-contained digest-locked provider package
cargo +1.90.0 run --locked -p echo-wesley-gen \
  --bin echo-edict-provider-package --

# Report package drift without rewriting anything
cargo +1.90.0 run --locked -p echo-wesley-gen \
  --bin echo-edict-provider-package -- --check
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
- Edict provider profiles carry the Echo-owned
  `echo.semantic-operation-id.fnv1-32/v1` law and exact `u32` id derived from
  the semantic operation coordinate plus generic query/mutation kind. The law
  is separate from GraphQL field identity and reserves the top two ids for Echo
  protocol envelopes: `u32::MAX` for scheduler control and `u32::MAX - 1` for
  witnessed suffix import. Generation refuses either reserved result and
  package-local collisions without salting or probing. The generated CDDL
  constrains `operationId` to `0..4294967293`; that numeric-domain proof does not
  replace semantic recomputation of the coordinate-and-kind law or complete-set
  collision checking. Carrying the id in canonical package content does not
  register, install, authorize, or execute an operation.
- The checked Edict helper implements the profile-owned `le-binary-v1` codec
  with distinct bounded `Id`, `Input`, and `Output` types, fail-closed decoding,
  and canonical EINT v1 packing. Its EINT `vars` bytes remain opaque and
  codec-owned. After exact bundle binding it exposes a borrowed provider-generic
  registry and can preflight one explicit host mutation implementation into an
  opaque package proposal. The proposal cross-binds Target IR, bundle, profile,
  value, obstruction, ABI, helper, rule, and footprint identities but neither
  proves arbitrary callback semantics nor installs itself. Query refusal is
  specific to this mutation proposal; authored reads remain a separate bounded
  observer/optic path.
- GraphQL SDL operation ids are derived deterministically and fail closed on
  collision or either Echo protocol reservation. The generator never increments
  a collided id because operation ids are persisted ABI.
- Generated query optic helpers use Echo ABI's domain-separated BLAKE3
  `query_vars_digest_v1(...)`; ad hoc variable digests are not accepted for
  retained reading identity.
- `--contract-host` emits opt-in, std-only mutation helpers that construct
  `warp-core` command rules for trusted host installation. The generated surface
  matches scheduler-materialized EINT runtime ingress events by op id, decodes
  typed vars, provides the base runtime-ingress read footprint, and builds a
  `RewriteRule` from host-supplied executor and footprint functions. It does
  not generate the application mutation body or grant application code tick
  authority.
- `--contract-host` also emits std-only query observer helpers that construct
  read-only `warp-core::ContractQueryObserver` instances for trusted host
  installation.
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
