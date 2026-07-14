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
- runtime implementation authority for actual replace execution remains
  unresolved until the lowerer and verifier slices prove presence-sensitive
  replace behavior.

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

Issue #652 will compile this source into the declared Edict lawpack, target
profile, two source-partitioned authority-facts documents, generated-artifact
profile, self-contained CDDL schema, deterministic review artifact, manifest
subresources, and Wesley-owned `generationProvenance` metadata. Resources marked
`external` are explicit digest-locked generator inputs; placeholder digests
are forbidden.

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

External Edict contract inputs come from the checked
[`contracts/v1/`](contracts/v1/README.md) publication merged in
[Edict PR #162](https://github.com/flyingrobots/edict/pull/162). Echo passes the
CDDL and manifest bytes explicitly to
`provider_contract_pack::admit_provider_contract_pack_v1(...)`, which verifies
the pinned publication identity, exact inventories, resource bytes, digests,
and provenance before generation. The semantic source selects those contracts
but does not authenticate arbitrary caller bytes under their coordinates.
Contract-pack admission authenticates schema authority; generated values must
still pass the owning CDDL root during output admission.

Issue #655, after the lowerer and verifier components exist, assembles those
outputs and generates the package-root `edict.provider-manifest/v1` for
`echo.edict-provider@1`, implementing exact world
`edict:target-provider@1.0.0`. The manifest is declared separately and is never
listed inside its own artifact inventory. Runtime `reviewPayload` invocation
output is distinct from #652's build-time `reviewArtifact`.

All generated files are derived artifacts. Their digests and review renderings
must never be copied back into this file as authored semantic facts.
