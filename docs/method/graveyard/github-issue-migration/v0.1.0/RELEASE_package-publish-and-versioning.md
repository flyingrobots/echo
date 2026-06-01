<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Package Publish And Versioning

Status: v0.1.0 release blocker.

Depends on:

- [Versioned contract and API compatibility](./PLATFORM_versioned-contract-api-compatibility.md)
- [JS/WASM/Browser client release surface](./PLATFORM_js-wasm-browser-client-release-surface.md)
- [Release-grade quickstart](./DOCS_release-grade-quickstart.md)

## Why now

Real applications need to know whether a generated contract package, runtime
ABI, WASM ABI, and client helper set fit together. The jedit release gate should
not depend on local path accidents or unversioned generated artifacts.

## Required behavior

Echo release artifacts must state compatibility for:

- Echo crate/package version;
- Echo ABI version;
- WASM ABI version, if shipped;
- Wesley generator version range;
- contract package version;
- schema hash;
- artifact hash;
- codec id;
- generated helper compatibility;
- installed package metadata.

## Acceptance criteria

- [ ] Echo documents release artifact names and publish targets.
- [ ] Echo defines compatibility checks for generated contract packages.
- [ ] Publish dry-run or equivalent release command is documented.
- [ ] jedit can consume Echo through documented versioned artifacts or an
      explicitly documented local-development override.
- [ ] The release candidate checklist includes package metadata verification.
- [ ] CHANGELOG and release docs name package/API compatibility expectations.

## Test plan

- Add package metadata fixture with compatible and incompatible generated
  package versions.
- Add release dry-run check where the repository supports one.
- Add quickstart validation that does not depend on an undocumented local path.

## Non-goals

- Do not publish until the release candidate gate is green.
- Do not force all neighboring repos to release in one commit.
- Do not treat local sibling path overrides as release-grade compatibility.
