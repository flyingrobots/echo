<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Experimental Echo–Keep content backend

This separate Rust 1.96 workspace pins Keep at `3165890e9291cfb5fe10e81a9d7cd151f3e59464`. It does not enter Echo’s default workspace or change `echo-cas`’s Rust 1.90 dependency graph.

`IdentityBinding::from_source` computes raw BLAKE3 for Echo and the version-1 Keep domain, bytes, and exact-length hash from one bounded stream. The binding keeps Keep coordinates private and has no persisted encoding. `verify_source` rechecks both identities and length. Neither method proves presence, retention, publication, or durability.

The conformance witness stages the named Keep golden vectors into `ReferenceStore`, reconstructs them, and rechecks exact bytes, both identities, and length. The largest fixture is one MiB. Source failures, limits, and coordinate substitution refuse a complete binding. This identity witness establishes no durable backend.

The optional `reference-adapter` feature exposes `KeepReferenceAdapter` through Echo's complete-object physical-content port. It is disabled by default. The adapter privately binds Echo hashes and exact lengths to Keep blob and layout identities, stages expected content, commits explicitly, and verifies the Keep receipt plus Echo bytes before atomic destination promotion. No old-backend fallback occurs.

The constructor requires limits for retained physical payload bytes, each logical object, object count, and layout entries. Payload capacity does not bound metadata or total process RSS; the object and entry caps bound metadata growth. Process death loses both the ReferenceStore and in-memory bindings. Reconstruction receipts establish no restart durability, authenticated absence, retention, synchronization, or pinned durable generation.

The feature suite reuses the exact MemoryTier/DiskTier conformance helper. Adapter tests cover limits, coordinate substitutions, missing store state, failed promotion, source interruption and recreation. Actual upstream missing-chunk and malformed-record refusals are exercised through public Keep APIs and the same quarantine helper; those tests do not inject corruption into Keep's inaccessible private chunk tables.

Run these checks inside the admitted, bounded Docker worker:

```sh
cargo +1.96.0 test --manifest-path experiments/echo-keep/Cargo.toml --locked
cargo +1.96.0 test --manifest-path experiments/echo-keep/Cargo.toml --locked --features reference-adapter
cargo +1.96.0 clippy --manifest-path experiments/echo-keep/Cargo.toml --locked --all-targets --all-features -- -D warnings
cargo +1.96.0 fmt --manifest-path experiments/echo-keep/Cargo.toml -- --check
cargo +1.90.0 check --locked -p echo-cas
```

[The canonical boundary](../../docs/architecture/echo-keep-physical-content-boundary.md) owns the contract. [Issue #759](https://github.com/flyingrobots/echo/issues/759) owns the identity slice; [issue #761](https://github.com/flyingrobots/echo/issues/761) owns the optional reference adapter.

The isolated graph has its own dependency-policy CI check. It derives all license, ban, advisory, and source rules from the root policy, with one scoped allowance for the pinned Keep Git source. The production workspace policy remains unchanged.
