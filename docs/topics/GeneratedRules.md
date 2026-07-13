<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Generated Rule Authorship

Product and application rules are authored in contract languages. Hand-written
Rust rule registration is not a supported product authoring surface. The final
package-shaped lowering corridor is a target boundary, not current end-to-end
implementation truth.

## Current Implementation

Wesley currently emits raw `RewriteRule` builders plus generated operation,
handler, observer, and footprint helpers. Its integration fixture enables
`native_rule_bootstrap` and calls `Engine::register_rule` directly. Wesley does
not yet package that output. It does not emit an `InstalledContractPackage` or
exercise package verification.

Echo separately implements `InstalledContractPackage` verification,
`Engine::register_contract_package`, scheduler-owned execution for registered
handlers, and `rule_pack_id` stamping. No current generator joins Wesley output
to that package-registration path.

The Edict bridge is fixture-only. It accepts a narrow `echo.span-ir/v1` Target
IR subset and produces deterministic attempt-receipt fixtures.

The Edict bridge does not admit a package or execute scheduler work.

`native_rule_bootstrap` is a Cargo feature gate and repository policy boundary.
Default builds omit raw rule constructors and public registration methods, but
a Rust dependency consumer can explicitly enable the feature.

It is not an access-control or security seal. Echo product and adapter code must
not use it as an application authoring escape hatch.

## Target Corridor

The required end state is:

```text
Wesley or Edict source
-> compiler Target IR
-> generated Rust handlers, observers, and footprints
-> generated registry and package metadata
-> InstalledContractPackage verification
-> Engine::register_contract_package
-> scheduler-owned execution
-> receipt / reading evidence carrying package and rule-pack identity
```

Both authoring systems need package emitters that consume their verified source
or Target IR and produce the same package-shaped runtime surface. That work must
reuse Echo's registration and execution path; it must not create a second
execution engine.

## Footprint Honesty

Generated footprints are compile-time claims. Runtime footprint checking in
debug builds is a generator-correctness oracle, not the primary production
authorization mechanism. Release CI must nevertheless execute representative
generated Wesley and Edict packs with `footprint_enforce_release` so a generator
cannot silently ship false declarations.

Conflicts remain explicit receipt rejections. A footprint failure must not
trigger hidden retries or widen access.

## Required Guards

- Default builds do not expose raw `RewriteRule` construction or public raw
  registration.
- Product and adapter crates do not enable `native_rule_bootstrap`.
- Package-qualified generators register through `InstalledContractPackage`,
  not an app-specific engine escape hatch.
- Registry/package identity, operation identity, codec/schema compatibility,
  and footprint metadata are verified before the engine mutates registration
  state.
- Runtime patches and receipts retain the generated package or rule-pack
  identity needed to explain execution.
