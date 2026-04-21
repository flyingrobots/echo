<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Import outcome idempotence and loop law

Depends on:

- [PLATFORM_witnessed-suffix-admission-shells](../asap/PLATFORM_witnessed-suffix-admission-shells.md)

## Why now

Suffix-shell work already says the right big thing:

- export/import witnessed suffixes
- treat remote import as normal admission after normalization
- surface explicit outcome algebra

What still needs its own explicit law is what repeated import means and how Echo
prevents import loops from turning old history into fake novelty.

This is where a lot of distributed systems quietly drift into folklore:

- "it should probably be idempotent"
- "we will somehow notice duplicates"

That is not strong enough for the runtime boundary Echo now wants to expose.

## What it should look like

- imported history retains durable source provenance:
    - source runtime identity
    - source writer identity
    - original transition identity
    - import receipt lineage
- re-import of already-known suffixes is explicitly classified, not treated as
  fresh admission
- import outcomes remain inspectable under repeated import
- loop prevention is part of the contract, not just a transport optimization

## Done looks like

- one packet states idempotence and anti-loop rules explicitly
- the import outcome algebra includes honest repeat-import posture
- tests prove:
    - first import
    - repeat import of the same suffix
    - self-history arriving again through a colder peer
- the runtime can explain why a bundle was not novel without collapsing into
  silent "no-op" folklore

## Repo evidence

- `docs/design/0009-witnessed-causal-suffix-sync/design.md`
- `docs/WARP_DRIFT.md`
