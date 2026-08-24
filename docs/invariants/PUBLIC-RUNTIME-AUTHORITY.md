<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Public Runtime Authority

This invariant governs every Cargo package and feature combination admitted to
Echo's public Rust release closure.

1. Constructible passive material is not admitted Echo history.
2. Pure computation and passive storage contracts MAY cross published package
   boundaries.
3. Authority epochs, admission tokens, Tick construction, settlement, receipt
   construction, commit publication, and recovery acceptance MUST remain
   private inside `flyingrobots-echo-runtime`.
4. No public constructor, field, enum variant, deserializer, default,
   conversion, unchecked builder, clone path, test feature, or unsafe helper may
   forge an admitted runtime value.
5. Every selectable feature combination in a published package MUST remain
   authority-safe, whether or not the release closure lists it as supported.
6. Published runtime packages MUST NOT expose or depend on native bootstrap,
   host-test authority, fault injection, repository orchestration, or the
   legacy `warp-core` route.
7. WAL code MAY report mechanical storage observations. Only the runtime may
   admit those observations as authoritative history or accepted recovery.
8. The first alpha MUST use a closed set of retained WAL implementations.
   Third-party durability claims require a separate provider guarantee and
   admission protocol.
9. Verification evidence is a claim about an executable subject. Runtime
   admission is a separate authority decision. API names and types MUST preserve
   that distinction.
10. Every durable, reusable, release-relevant Echo-owned Cargo package MUST use
    the `flyingrobots-echo-*` package prefix. Repository location alone does not
    establish semantic ownership.
11. The public release boundary MUST be proven from packaged artifacts by both
    a positive clean-room lifecycle witness and negative authority-escape
    witnesses.
12. Release engineering does not authorize publication. Registry publication,
    namespace reservation, ownership changes, release tags, and publish-capable
    workflow dispatch require separate explicit human approval.

The complete rationale, target package topology, and current implementation
posture live in
[Public Rust release boundary](../architecture/public-rust-release-boundary.md).
