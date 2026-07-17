<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Edict Provider Semantic Source

This directory contains Echo-authored semantic input for the external Edict
provider pipeline. It does not contain generated lawpacks, target profiles,
authority facts, provider manifests, schemas, or review projections.

The current source is
[`echo-provider-semantics-v1.json`](echo-provider-semantics-v1.json). Its API is
`echo.edict-provider-semantics/v1`, and its semantic-source coordinate is
`echo.semantic-schema@1`.

## Authority

The checked source names one authority artifact and one canonical domain for
each semantic fact. In the first compatibility closure:

- the Echo semantic declaration owns the package-local records and Core aliases,
  semantic effect, domain obstruction and source mapping, budget, operation,
  generated package-member inventory, and outer invocation/schema inventory;
- Edict owns Core string semantics; `a.b@1.Id` only selects the exact
  `String<max=16,canonical=raw-utf8>` Core coordinate;
- Echo target metadata owns the operation profile and optic template, the
  low-level `rejected` failure taxonomy, write-class resolution, native
  `echo.dpo@1.replace` capability, and its inner `echo.span-ir/v1` domain;
- the semantic declaration's invocation/schema inventory independently owns the
  outer `edict.target-ir.artifact/v1` provider domain and
  `target-ir-artifact` root;
- runtime GraphQL owns none of these first-operation facts; and
- the checked lowerer and verifier prove the bounded provider translation and
  semantic-verification crossings. Echo now owns and implements exact proposal
  admission, package-occurrence corroboration, and the normal proof-carrying
  provider-native installation path; runtime intent admission, scheduling, execution,
  presence-sensitive enforcement, commitment, receipts, readings, and
  observations remain unresolved and Echo-owned.

The source deliberately selects native lowerability and declares no semantic
effect-to-effect direct adapter. Its lawpack projection separately declares the
digest-locked target adapter required to discharge the runtime
`target.replace` effect for `echo.dpo@1`; those are different contracts.

## Executable Schema

The strict Rust model and validator in
`crates/echo-wesley-gen/src/provider_semantics.rs` are the executable schema for
this source version. Every serialized object rejects unknown fields. Validation
normalizes declared sets, rejects duplicate coordinates and keys, resolves every
typed reference, checks effect/profile/capability and explicit semantic-discharge
joins, requires exhaustive
failure-to-obstruction mappings, and requires every generated artifact and
invocation input/output to have its exact Edict contract, domain, format, and
root rule. Every runtime effect must have exactly one native or direct-adapter
implementation, and each lawpack adapter must select a unique target profile.
The validator also rejects recursive type graphs, Echo claims over Edict Core
coordinates, byte-counted string aliases, invalid Edict failure identifiers,
duplicate profile aliases, wrong fact domains or authority kinds, conflicting
inner Target IR domains, wrong package ABI/provider identity, self-referential
provider inventories, incomplete authority-fact projections, and incomplete
lawpack/target-profile resource closure.

The v1 mapping shape intentionally supports only empty bounded failure and
obstruction payloads because it carries no payload transform. A non-empty
payload fails structurally instead of asking a future generator to invent a
field mapping.

There is no hand-maintained JSON Schema snapshot. Adding one without generating
and checking it from the executable model would create a second shape authority.

Run:

```bash
cargo +1.90.0 test -p echo-wesley-gen --test provider_semantic_source
```

## No Discovery

The validator receives explicit source bytes and performs no filesystem,
registry, environment, clock, or network discovery. Nothing under
`schemas/wesley-relocated/` is active authority. A stale declaration supplied
alongside the current source cannot override it: duplicate coordinates fail
before references are resolved. Old relocated SDL remains historical Git
evidence only.

## Generated Outputs

Issue #652 now compiles the primary closure into the declared Edict lawpack,
target profile, two source-partitioned authority-facts documents,
generated-artifact profile, fourteen manifest subresources, and self-contained
CDDL schema. Every canonical output passes its owning generated root, and every
Edict-owned output also passes the independently admitted upstream root. The
Wesley-owned `generationProvenance` document binds exact source, settings,
generator, and six primary output byte identities and immediately verifies all
of them. Its emitted set contains the five canonical primary artifacts plus the
raw self-contained CDDL; fourteen resources are transitively bound through
those artifacts rather than incorrectly promoted into the primary projection.
The primary wrapper retains its producing Wesley input digest, and generator
coordinates cannot alias any declared artifact, resource, provider, or package
coordinate. Exact source reordering therefore preserves all primary emitted
bytes while intentionally moving the provenance identity.
Wesley's non-authoritative `GenerationReviewV1` is then derived from the
verified input/provenance pair and deterministically copies its generator,
roles, sources, and emitted identities. It cannot claim semantic or runtime
authority. The exact 22-file result is checked under
[`generated/v1/`](generated/README.md): five canonical-CBOR primary artifacts,
fourteen canonical-CBOR resources, raw self-contained CDDL, canonical Wesley
provenance JSON, and canonical non-authoritative review JSON. The dedicated
generator binds a fixed source/dependency-lock frame and its `--check` mode
reports missing, changed, or unexpected files without rewriting them. Resources
marked `external` are explicit digest-locked generator inputs; placeholder
digests are forbidden.

[`generation-settings-v1.json`](generation-settings-v1.json) is the explicit,
versioned settings input for that invocation. The first closure selects no
GraphQL Shape source: `a.b@1.t` remains an Echo semantic operation and is not
invented as a GraphQL root field. The canonical Wesley generation input binds
the exact semantic-source bytes, admitted Edict CDDL and manifest bytes, and
settings bytes. Its primary projection-role set excludes provenance and review
because those are derived envelopes over the primary outputs and cannot include
their own digests.

The validator's normalized semantic projection is insensitive to ordering of
set-like declarations. The generation-input digest intentionally still changes
when the authored JSON bytes are reordered, because later provenance binds the
exact source artifact rather than mislabeling normalized bytes as the checked
file. Generated semantic artifact bytes remain a function of the normalized
model.

Echo-owned resource documents use their declared schema API as their wire
`apiVersion`; generated resource digests are framed by the resource coordinate.
The lawpack and target-profile manifests bind those domain-framed identities,
while later Wesley provenance binds exact-byte content references. The
declarative conformance corpus names twelve reviewed obligations: six consumed
by the isolated host executor and six by the package executor, spanning one
accepted, nine rejected, and two refused outcomes. It carries no pass flag, result,
evidence path, or runtime receipt; declarations are obligations rather than
evidence or runtime authority. Any future case must extend the owning closed
CDDL vocabulary together with a typed executable witness. Direct-adapter routes
bind the adapter and native capability, and operation-local failure mappings
are not collapsed merely because two operations share one semantic effect.
Read-class operations must use a revelation/projection optic and generate
observer metadata rather than mutation metadata.

The two authority-facts outputs use Edict's `edict.authority-facts/v1` domain
and bind their contract owner to
[Edict #157](https://github.com/flyingrobots/edict/issues/157). Echo does not
define a second authority-facts wire schema. That Edict-owned canonical
CBOR/CDDL contract and its compiler-consumer bridge landed in
[Edict PR #159](https://github.com/flyingrobots/edict/pull/159).

The generated target-profile lowerer and verifier resources are declarative
contract documents. They are not executable component bytes. Issue #655 binds
the exact lowerer and verifier components, including their frozen WIT world
attestations, when it assembles the provider manifest.

The first checked executable lowerer now lives under
[`components/v1/`](components/v1/README.md). It consumes only the frozen Edict
WIT request, accepts the exact generated mutation closure, and has exact Target
IR byte parity with Edict's built-in Echo wrapper. Unsupported reads and other
semantics produce typed refusals instead of invented artifacts. The checked
verifier lives beside it and independently checks the exact Core-to-Target-IR
relation under the generated target profile and semantic closure. Through the
pinned Edict host, exact request artifacts and the declared report schema are
preflighted before invocation. Returned accepted and well-formed rejected
reports are then admitted and receive host-authored manifests, while an
unsupported output-role overclaim remains a typed refusal with neither response
nor manifest. Independent fresh-store replay and separate host processes
reproduce all three completed outcomes identically. Both checked components
remain uninstalled package material; neither they nor the generated
authority-facts documents are runtime Echo authority. The promoted lowerer is
189,668 bytes with SHA-256
`f2063b66798fbb1c2b27c3af56e4b78184ffc22c9ed9c7a32c483d05b8c1d382`; the
promoted verifier is 189,922 bytes with SHA-256
`632cc5134861c0b31ccc9ca77d4a09fe757094964369d057b62ca6ba6ad38ad7`.

External Edict contract inputs come from the checked
[`contracts/v1/`](contracts/v1/README.md) publication merged in
[Edict PR #162](https://github.com/flyingrobots/edict/pull/162). Echo passes the
CDDL and manifest bytes explicitly to
`provider_contract_pack::admit_provider_contract_pack_v1(...)`, which verifies
the pinned publication identity, exact inventories, resource bytes, digests,
and provenance before generation. The semantic source selects those contracts
but does not authenticate arbitrary caller bytes under their coordinates.
Contract-pack admission authenticates schema authority; generated values must
still pass the owning CDDL root during output admission. Echo's pure
`AdmittedProviderContractPackV1::validate_contract_bytes(...)` boundary now
enforces both exact `edict.canonical-cbor/v1` bytes and that named root. This is
generation-time artifact validation, not runtime installation or authority;
Echo must still explicitly admit any package, operation, or consequence.

Issue #655 consumes these checked lowerer and verifier components, assembles
them with the generated semantic artifacts, and generates the package-root
`edict.provider-manifest/v1` for
`echo.edict-provider@1`, implementing exact world
`edict:target-provider@1.0.0`. The manifest is declared separately and is never
listed inside its own artifact inventory. Runtime `reviewPayload` invocation
output is distinct from #652's build-time `reviewArtifact`.

The #656 native lowerer model declares three exact sorted outputs:
`generated.echo-dpo` / `echo.generated-artifact/v1`, `review.echo-dpo` /
`echo.review-payload/v1`, and `target-ir.echo-dpo` /
`edict.target-ir.artifact/v1`. The first two are canonical-CBOR envelopes at
`generated/echo_dpo.rs` and `review/echo_dpo.json`; the non-authoritative review
subjects the exact generated-artifact digest. `echo.span-ir/v1` remains the
semantic Target IR coordinate and `edict.target-ir.artifact/v1` its distinct
artifact output and digest domain. `echo.dpo.bundle/v1` is a target-bundle
profile rather than a contract-bundle occurrence. Final semantic and release
bundle identities are compared explicitly after assembly using the separate
`edict.bundle.semantic/v1` and `edict.bundle.release/v1` propositions. That
comparison also checks the semantic operation, Echo ABI and helper API, provider
and operation schemas, generated and operation profiles, and
the exact abstract
footprint obligation/algebra. It does not claim a concrete static read/write
footprint. Every framed resource is checked as a complete
coordinate/domain/digest identity. The comparison grants no admission,
installation, or runtime authority. The canonical generated-artifact profile
now carries `operationIdLaw: "echo.semantic-operation-id.fnv1-32/v1"` and the
exact persisted `operationId` for each semantic coordinate and generic
query/mutation kind. The top two ids are Echo protocol reservations:
`u32::MAX` for scheduler control and `u32::MAX - 1` for witnessed suffix import.
Either reserved result and any package-local collision refuse without salting
or probing. The owning CDDL limits `operationId` to `0..4294967293`, proving only
the numeric application domain; semantic generation must still recompute the
coordinate-and-kind law and collision-check the complete set. Generated source
now carries public expected constants for the packaged law and id, requires them
as untrusted bundle claims, and refuses disagreement before returning a
private-state descriptor that exposes the matched id claim. It does not
independently derive a runtime identity.

The generated-artifact profile owns the exact `le-binary-v1` value-codec claim.
Generated Rust implements it with semantically distinct `Id`, `Input`, and
`Output` types, preserves raw UTF-8 under the authored scalar bound, and fails
closed on malformed, over-bound, truncated, or trailing bytes. Descriptor
methods encode/decode the exact input and output and `pack_intent(...)` wraps
the encoded input in canonical EINT v1. Echo treats the EINT `vars` bytes as
opaque bytes owned by that selected codec; canonical CBOR is not a universal
operation-variable law.

The matched descriptor also exposes a borrowed provider-generic registry and
can bind its generated matcher to one explicitly identified host mutation
implementation. It produces only an opaque, non-installing package proposal
after comparing every Target IR, semantic/release bundle, target/generated/
operation profile, provider/value schema, codec, obstruction, operation-id,
ABI, helper-API, rule-name, and footprint claim. Identity equality detects
cross-binding but does not prove arbitrary callback semantics. The proposal
does not authenticate, install, register, schedule, execute, persist, observe,
or receipt anything; those remain trusted Echo crossings. Its constructor is
mutation-specific and refuses a `Query`. Authored reads remain a separate
bounded observer/optic path and must not be lowered as synthetic mutations.

`TrustedRuntimeHost` can independently admit the proposal's complete occurrence
and registry claims under `ProviderContractAdmissionPolicyV1`, yielding an
opaque `AdmittedProviderContractPackageV1`. `echo-wesley-gen` can then consume
that token with an independently produced `DigestAdmittedProviderPackageV1`.
Only the exact `echo.edict-provider@1` coordinate and a strict lowercase
`sha256:` package root whose raw suffix equals the admitted occurrence hash
produce `DigestCorroboratedProviderContractPackageV1`. This second crossing
corroborates package occurrence; it does not derive registry semantics or
arbitrary callback correctness from package bytes. Neither token installs by
itself or schedules, executes, persists, observes, receipts, or grants Echo
runtime authority.

The proof-owning
`install_digest_corroborated_provider_contract_package_v1(...)` adapter consumes
the corroborated token through `warp-core`'s sealed runtime-owner installer
port. Its `TrustedRuntimeHost` lower primitive does not authenticate package
bytes; that proposition remains owned by the consumed proof, and no equivalent
application surface exists. Installation retains the exact occurrence and
provider reference, full owned provider registry, and mutation-rule identity in
a provider-specific record. Provider package, root, mutation-operation, and
shared scheduler-rule indexes update atomically. No legacy Wesley/GraphQL
metadata or evidence is invented and no callback runs during installation.
Provider-native intent ingress, invocation, WAL persistence, receipts, and
observations remain separate future crossings. Authored reads likewise remain
on their independent bounded observer/optic path.

Runtime `reviewPayload` remains distinct from Wesley `GenerationReviewV1`.
Both refreshed components have crossed checked promotion, and the actual pinned
host admits the generated envelope under its owning `generated-artifact` CDDL
root. The isolated host helper witness is green for exact bundle binding, typed
codec round trips and refusal cases, EINT packing, the borrowed registry, and
non-installing package proposal. The review envelope still requires its
independent host-side CDDL admission before that output is admitted.

The package closure contains the 22 generated files plus the exact lowerer and
verifier components. Its Echo-owned provider digest binds the typed manifest
routes, 24 domain-to-root bindings (nine invocation domains, the generated
artifact profile, and 14 generated-resource domains), and raw SHA-256 of all 24
physical members without hashing the derived manifest into itself. The five routed
canonical-CBOR artifacts use their Edict domain-framed identities in the
manifest, while CDDL, Wesley JSON evidence, and components use raw exact-byte
identities. Pure digest admission also rebinds the generated routes to the
packaged provenance/review and requires an external expected provider
reference. This is package-occurrence evidence, not Edict component or schema
compatibility proof and not Echo runtime installation or authority.

The exact 25-file distribution is checked under
[`package/v1/`](package/README.md). Its dedicated publisher first requires its
22 `generated/` members to equal the current checked provider corpus introduced
by #652 byte-for-byte, then
writes only the two exact components, those generated members, and the derived
manifest. Run `echo-edict-provider-package --check` to report drift without
creating, deleting, or rewriting package files.

The isolated Edict c75 host gate then consumes that exact checked package. It
constructs all 24 native schema bindings, validates the five canonical primaries
and 14 generated resources, proves every owner field names the expected exact
resource digest, prepares both components, and obtains both opaque request
proofs without guest invocation. Schema-valid resource substitution, reference
swaps, authority-source disagreement, and malformed contract material fail
before execution. This does not grant Echo runtime authority.

The crate-local `assets/v1/` tree is an exact 38-file publication carrier for
the same provider bytes plus the repository sources needed for generator
provenance. The compile-time generator identity enumerates a 20-file source
closure, including the provider-generic registry implementation. Carrier paths
are physical packaging details only; authored logical source paths remain
unchanged. `echo-edict-provider-assets --check-package-list` proves
owner/carrier/package agreement and exact Cargo archive selection.

All generated files are derived artifacts. Their digests and review renderings
must never be copied back into this file as authored semantic facts.
