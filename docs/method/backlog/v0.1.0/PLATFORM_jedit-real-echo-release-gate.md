<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# jedit Real Echo Release Gate

Status: active release blocker.

Depends on:

- [External contract proof fixture](./PLATFORM_external-contract-proof-fixture.md)
- [Reference trusted runtime host loop](./PLATFORM_reference-trusted-runtime-host-loop.md)
- [App-safe client surface](./PLATFORM_app-safe-client-surface.md)

Echo `v0.1.0` is blocked until jedit works on Echo from the sibling jedit
repository.

## Why now

The in-repo external contract fixture proved the generic mechanics. It did not
prove application ergonomics, cross-repo generated artifact use, real WASM or
host packaging, or the product pressure of a serious external application.

jedit is the release witness because it is external, product-shaped, and strict
about keeping application nouns out of Echo core.

## Required path

```text
sibling application-owned contract
-> Wesley generated Echo runtime artifacts
-> Echo installs a generic generated package
-> application submits canonical intent
-> trusted Echo host stages work and authorizes scheduler opportunities
-> Echo scheduler emits tick receipts
-> application observes intent outcome
-> application queries bounded reading
-> retained receipt and reading evidence can be inspected
-> replay reproduces the result
```

## Acceptance criteria

- [ ] A documented jedit command runs against a clean Echo checkout or
      published Echo artifact.
- [ ] The proof uses generated jedit/Wesley contract artifacts rather than a
      second hand-authored protocol shape.
- [ ] product capabilities remain application-side nouns; Echo core and Echo
      package boundaries do not define or import them.
- [ ] opaque read-basis tokens remain supporting app-safe tokens, not primary
      runtime coordinates.
- [ ] sibling application code does not access scheduler control.
- [ ] scheduler `start`, `until_idle`, and fault recovery remain trusted host
      authority.
- [ ] at least one application mutation uses non-trivial vars.
- [ ] at least one bounded reading returns product-shaped payload plus Echo
      reading evidence.
- [ ] at least one non-happy path is covered: unsupported operation/query,
      lawful rejection, admission obstruction, residual reading, or missing
      retention.
- [ ] retained receipt and reading evidence can be loaded or inspected by
      semantic coordinate.
- [ ] replay reproduces the same outcome and reading.
- [ ] Echo core contains no application product APIs.

## Current blocker

The existing jedit opt-in real Echo WASM witness still reflects an older model:
it tries to carry scheduler control through app-facing dispatch. Current Echo
correctly rejects that as forbidden control intent. The release work is to
update the jedit witness around the app/host split, not to weaken Echo.

## Non-goals

- Do not build all of jedit.
- Do not move application product nouns into Echo.
- Do not grant application code tick authority.
- Do not replace jedit's fake transport harness before the real witness is
  ready.
- Do not require full Continuum transport/import.
- Do not require full observer-rights governance.
