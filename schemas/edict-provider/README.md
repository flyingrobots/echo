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
  semantic-verification crossings, while runtime authority for replace package
  admission, installation, scheduling, execution, presence-sensitive
  enforcement, commitment, receipts, readings, and observations remains
  unresolved and Echo-owned.

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
while later Wesley provenance binds exact-byte content references. The empty
declarative conformance corpus cannot carry cases until executable parity
evidence exists. Direct-adapter routes bind the adapter and native capability,
and operation-local failure mappings are not collapsed merely because two
operations share one semantic effect. Read-class operations must use a
revelation/projection optic and generate observer metadata rather than mutation
metadata.

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
authority-facts documents are runtime Echo authority.

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
comparison grants no admission, installation, or runtime authority. Runtime
`reviewPayload` also remains distinct from Wesley `GenerationReviewV1`.
Checked-component promotion and actual host validation of these new envelopes
under the owning CDDL roots are still required before they are admitted provider
outputs.

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

The crate-local `assets/v1/` tree is an exact publication carrier for the same
provider bytes plus the repository sources needed for generator provenance.
Carrier paths are physical packaging details only; authored logical source paths
remain unchanged. `echo-edict-provider-assets --check-package-list` proves
owner/carrier/package agreement and exact Cargo archive selection.

All generated files are derived artifacts. Their digests and review renderings
must never be copied back into this file as authored semantic facts.
