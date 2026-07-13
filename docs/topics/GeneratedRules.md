<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Generated Rule Authorship

Product and application rules are authored in contract languages and lowered
into generated Echo packages. Hand-written Rust rule registration is not a
public authoring surface.

## Boundary

The `native_rule_bootstrap` feature exists for internal fixtures, generated-code
bootstrap, and transitional engine tests. It gates raw rule types and
registration APIs; disabling the feature must make those surfaces unavailable
to ordinary consumers.

The supported flow is:

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

Echo already owns package verification, registry installation, scheduler-owned
execution, and `rule_pack_id` stamping. The missing Edict bridge is a generator
that consumes Edict Target IR and emits the same package-shaped runtime surface;
it is not a second execution engine.

## Footprint Honesty

Generated footprints are compile-time claims. Runtime footprint checking in
debug builds is a generator-correctness oracle, not the primary production
authorization mechanism. Release CI must nevertheless execute representative
generated Wesley and Edict packs with `footprint_enforce_release` so a generator
cannot silently ship false declarations.

Conflicts remain explicit receipt rejections. A footprint failure must not
trigger hidden retries or widen access.

## Required Guards

- Default consumers cannot import raw `RewriteRule` construction or call raw
  registration.
- Generated packages install through `InstalledContractPackage`, not an
  app-specific engine escape hatch.
- Registry/package identity, operation identity, codec/schema compatibility,
  and footprint metadata are verified before the engine mutates registration
  state.
- Runtime patches and receipts retain the generated package or rule-pack
  identity needed to explain execution.
