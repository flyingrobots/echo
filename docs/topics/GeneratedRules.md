<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Generated Rule Authorship

Product and application rules are authored in contract languages. Hand-written
Rust rule registration is not a supported product authoring surface. The final
package-shaped lowering corridor is partially implemented, but it is not yet
current end-to-end runtime truth.

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
through a pure descriptor preflight.

That provider package and descriptor are compiler/publication artifacts. They
are not an Echo registry entry, `InstalledContractPackage`, installation token,
submitted intent, execution receipt, or observation. The generated helper does
not yet name a value codec or construct a complete invocation.

Echo separately implements `InstalledContractPackage` verification,
`Engine::register_contract_package`, scheduler-owned execution for registered
handlers, and `rule_pack_id` stamping. No current generated Wesley or Edict
client closes the remaining compiler-descriptor-to-trusted-host installation
crossing. The Edict provider package is admitted by its compiler/host contracts;
it is not thereby admitted or installed by Echo.

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
-> generated registry and package metadata
-> InstalledContractPackage verification
-> Engine::register_contract_package
-> scheduler-owned execution
-> receipt / reading evidence carrying package and rule-pack identity
```

Both authoring systems still need generated bridges into Echo's runtime package
surface. Wesley needs a package emitter. Edict already emits a digest-locked
provider publication package, but still needs codec-bound clients and a
registry/package adapter that a trusted host can verify, bind to an executor or
observer, and install as an `InstalledContractPackage`. That work must reuse
Echo's registration and execution path; it must not create a second execution
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
reproducibility, not runtime installation or footprint qualification. Each
authoring path still needs a positive installed generated-pack witness plus a
deliberately false-footprint negative oracle before it may claim that status;
Wesley additionally still needs its package emitter.

Conflicts remain explicit receipt rejections. A footprint failure must not
trigger hidden retries or widen access.

## Required Guards

- Default builds do not expose raw `RewriteRule` construction or public raw
  registration.
- Product and adapter crates do not enable `native_rule_bootstrap`.
- Generators emit artifacts and descriptors without registering themselves.
  Trusted hosts register package-qualified generated material through
  `InstalledContractPackage`, not an app-specific engine escape hatch.
- Registry/package identity, operation identity, codec/schema compatibility,
  and footprint metadata are verified before the engine mutates registration
  state.
- Runtime patches and receipts retain the generated package or rule-pack
  identity needed to explain execution.
