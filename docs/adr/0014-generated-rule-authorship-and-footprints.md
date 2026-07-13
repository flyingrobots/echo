<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# ADR 0014: Generated Rule Authorship and Footprints

- **Status:** Accepted
- **Date:** 2026-07-13

## Context

Public hand-written Rust rule registration would let applications bypass
contract identity, generated operation bindings, footprint declarations, and
package verification.

## Decision

Application and product rules are authored in Wesley, Edict, or another
approved contract language and lowered into generated Echo packages.
Hand-written native Rust rules are forbidden as a public authoring path.

`native_rule_bootstrap` remains limited by repository policy to generated
bootstrap code, internal fixtures, and transitional engine tests. Cargo permits
dependency consumers to opt into the feature explicitly.

The feature is a policy and compatibility boundary, not an access-control seal.

Declared footprints are compile-time contracts. Debug runtime enforcement is a
generator-correctness oracle, and CI must exercise representative generated
packages with release enforcement.

## Consequences

- Raw rule constructors and public registration stay unavailable in default
  builds. Repository checks must prevent product and adapter crates from
  enabling the bootstrap feature as an authoring bypass.
- Generated packages carry package, schema, operation, codec, compatibility,
  and footprint identity through installation and execution evidence.
- A footprint conflict is an explicit rejection; it does not trigger hidden
  retry or ambient access widening.

## Evidence Anchors

- [Generated rule authorship](../topics/GeneratedRules.md)
- `docs/invariants/DECLARATIVE-RULE-AUTHORSHIP.md`
- `crates/warp-core/src/contract_registry.rs`
- `crates/warp-core/src/engine_impl.rs`
- `crates/warp-core/src/footprint_guard.rs`
