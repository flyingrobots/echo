<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Generated Rule Authorship

Product and application rules are authored in contract languages. Hand-written
Rust rule registration is not a supported product authoring surface. The first
Edict mutation closure now reaches provider-native scheduling, receipts, and
WAL recovery. Wesley packaging and the generated bounded-read corridor remain
incomplete and must not borrow that mutation evidence.

## Current Implementation

Wesley currently emits raw `RewriteRule` builders plus generated operation,
handler, observer, and footprint helpers. Its integration fixture enables
`native_rule_bootstrap` and calls `Engine::register_rule` directly. Wesley does
not yet package that output. It does not emit an `InstalledContractPackage` or
exercise package verification.

The Edict provider path authenticates exact authored semantic source and
generation settings, emits canonical provider artifacts, lowers the supported
mutation closure into `echo.span-ir/v1` Target IR, and independently verifies
that crossing. It publishes checked lowerer and verifier components inside a
digest-locked provider package with exact provenance and non-authoritative
review evidence. The lowerer also emits a requested-only Rust helper that binds
the exact Target IR, bundle propositions, profiles, schemas, ABI versions,
footprint obligation, obstruction mapping, and semantic operation identity
through a pure descriptor preflight. The generated-artifact profile owns the
`le-binary-v1` value codec; the helper implements distinct typed `Id`, `Input`,
and `Output` encoders/decoders, rejects malformed or trailing bytes, and packs
the typed input into canonical EINT v1. The EINT `vars` payload remains opaque,
codec-owned bytes rather than a universal canonical-CBOR value.

That provider package and descriptor are compiler/publication artifacts. They
are not an Echo registry entry, installed package, submitted intent, execution
receipt, or observation. The descriptor exposes a borrowed, provider-generic
registry and can combine its generated matcher with one explicitly
identity-bound host implementation to produce an opaque, non-installing
provider package proposal. Proposal preflight fails closed across the complete
operation, Target IR, bundle, profile, schema, codec, obstruction, ABI, helper
API, and footprint claims. Matching callback claims are cross-binding evidence,
not proof that arbitrary callback code implements the claimed semantics.
The result is a non-installing provider package proposal.

`TrustedRuntimeHost` can now consume that proposal under an independently
constructed `ProviderContractAdmissionPolicyV1`. Exact agreement on the
host-owned occurrence claim and complete provider registry yields an opaque
`AdmittedProviderContractPackageV1`; semantic and release mismatches remain
distinct typed failures. This is claim admission, not package-byte admission:
the crossing performs no package loading or rehashing. It also performs no
installation, registry mutation, callback invocation, scheduling, receipt, or
observation. Echo's existing `InstalledContractPackage` verification and
`Engine::register_contract_package` remain the Wesley compatibility path.
`echo-wesley-gen` now consumes the admitted token with an independently produced
`DigestAdmittedProviderPackageV1`; exact provider-coordinate and strict
lowercase SHA-256 package-root agreement yields an opaque
`DigestCorroboratedProviderContractPackageV1`. This corroborates occurrence
only, not registry semantics or callback behavior. A proof-owning
`echo-wesley-gen` adapter now consumes that token through `warp-core`'s sealed
runtime-owner installer port. `TrustedRuntimeHost` creates a distinct owned
provider record and atomically updates provider package, root, operation, and
shared scheduler-rule indexes. It invokes no callback and invents no
Wesley/GraphQL metadata or legacy installed-contract evidence. Generated code
cannot install itself, and the application handle exposes no installation
surface.

An installed provider mutation can now enter Echo through
`TrustedRuntimeHost::admit_provider_contract_submission_v1(...)` after ordinary
witnessed submission. The outer intent kind must be exactly canonical EINT v1,
and the encoded operation id must resolve to the installed provider mutation
before staging. Echo uses the existing scheduler and retains a distinct
`ProviderV1` invocation-evidence proposition binding the installed package id,
exact package reference, operation, Target IR, and scheduler rule. A provider
outcome is applied only when the exact bound provider rule appears in the tick
receipt; a system acknowledgement cannot satisfy that proposition. Provider
evidence survives the tagged WAL and fresh-host recovery after the same package
is reinstalled as host configuration, without fabricating legacy contract
coordinates or rerunning work.

`native_rule_bootstrap` is a Cargo feature gate and repository policy boundary.
Default builds omit raw rule constructors and public registration methods, but
a Rust dependency consumer can explicitly enable the feature.

It is not an access-control or security seal. Echo product and adapter code must
not use it as an application authoring escape hatch.

## Target Corridor

The required end state is:

```text
Wesley or Edict source
-> verified mutation Target IR or lawful read/observer semantics
-> generated Rust handlers, bounded observers, and footprints
-> generated typed codecs, EINT helpers, registry, and package metadata
-> opaque provider package proposal with explicit host binding
-> Echo-owned exact proposal-claim admission
-> exact package corroboration
-> provider-native installation
-> existing atomic engine rule and operation indexes
-> exact EINT-kind and installed-operation admission
-> scheduler-owned execution
-> receipt / WAL evidence carrying package, operation, Target IR, and rule identity
```

Wesley still needs a package emitter. Edict already emits a digest-locked
provider publication package, codec-bound mutation client, borrowed registry,
and fail-closed package proposal. Echo admits the exact proposal claim under
independent trusted-host policy; `echo-wesley-gen` corroborates that token with
independently admitted exact package bytes and consumes the proof into a
provider-native installed record with atomic Echo-owned indexes. Echo now
admits and dispatches that installed mutation through the shared scheduler with
provider-specific receipt and WAL evidence. It still needs the separate
generated bounded-observer path for authored reads. The current mutation
proposal intentionally rejects `Query`; that refusal does not turn a read into
a mutation or eliminate the independent observer/optic corridor. Installation
and invocation reuse Echo's existing indexes, scheduler, receipt, and WAL
machinery without fabricating Wesley metadata or creating a second execution
engine.

## Footprint Honesty

Generated footprints are compile-time claims. Runtime footprint checking in
debug builds is a generator-correctness oracle, not the primary production
authorization mechanism. Release CI must nevertheless execute representative
generated Wesley and Edict packs with `footprint_enforce_release` so a generator
cannot silently ship false declarations.

The `footprint_enforce_release` qualification lane is not wired into CI.
No Wesley or Edict package is currently footprint-release-qualified. The
digest-locked Edict provider package proves publication identity and
reproducibility, not runtime installation or footprint qualification. The Edict
path now has positive provider-native installation and invocation witnesses,
but it still needs a deliberately false-footprint negative oracle before it may
claim release qualification. Wesley still needs both a package emitter and its
own positive installed generated-pack witness.

Conflicts remain explicit receipt rejections. A footprint failure must not
trigger hidden retries or widen access.

## Required Guards

- Default builds do not expose raw `RewriteRule` construction or public raw
  registration.
- Product and adapter crates do not enable `native_rule_bootstrap`.
- Generators emit artifacts, descriptors, and opaque proposals without
  registering themselves. Trusted hosts independently admit proposal claims,
  corroborate exact package evidence, and register package-qualified generated
  material through the appropriate provider-native or Wesley compatibility
  record—not an app-specific engine escape hatch.
- Registry/package identity, operation identity, codec/schema compatibility,
  and footprint metadata are verified before the engine mutates registration
  state.
- Provider invocation requires the exact EINT intent kind, an installed
  provider operation, and receipt evidence from the exact bound scheduler rule.
- Runtime patches and receipts retain the generated package or rule-pack
  identity needed to explain execution.
