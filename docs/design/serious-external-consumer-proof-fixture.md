<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Serious External Consumer Proof Fixture

Status: implemented local proof fixture.

This packet records the v0.1.0 external-consumer proof slice. The fixture is
intentionally shaped like a real application contract, but it remains outside
Echo core. Echo proves generic hosting behavior; it does not import editor,
document, buffer, rope, or product-specific nouns into production APIs.

## Claim

A serious external-consumer-shaped contract can use the local contract-host
path:

```text
generated-style package
-> trusted host install
-> app-safe submit
-> ticketed runtime ingress
-> scheduler-owned tick
-> applied/rejected intent outcome
-> bounded QueryView reading
-> retained reading and receipt evidence
```

The fixture uses document-edit semantics only inside a test package. The generic
runtime boundary still sees package metadata, operation ids, canonical EINT
bytes, query ids, readings, receipts, and retained evidence coordinates.

## Implemented Surface

`external_consumer_contract_fixture_tests` now installs a generated-style
`jedit`-shaped hot-text package with:

- one `replaceRange` mutation operation;
- one `documentWindow` QueryView operation;
- non-trivial canonical vars bytes;
- scheduler-owned mutation execution;
- overlapping write footprint conflict;
- bounded QueryView reading;
- installed package evidence on readings and receipt correlations;
- semantic retention of the reading payload and receipt evidence.

## Invariants

- Application nouns stay in the external fixture, not `warp-core` production
  APIs.
- Application submission does not tick.
- Query observers remain read-only.
- Conflict rejection is final for that tick attempt.
- Retained payload lookup requires semantic coordinates, not raw CAS hash
  guessing.
- Package evidence is metadata, not execution or query authority.

## Evidence

- `cargo test -p warp-core --features "native_rule_bootstrap trusted_runtime" --test external_consumer_contract_fixture_tests`

## Remaining Release Work

This fixture is serious enough to pressure the generic contract-host path, but
it is still an in-repo test fixture. Later work can point the same shape at an
actual external `jedit`-owned generated package once that package is ready.
