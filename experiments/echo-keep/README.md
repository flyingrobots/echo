<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Experimental Echo–Keep identity bridge

This separate Rust 1.96 workspace pins Keep at `3165890e9291cfb5fe10e81a9d7cd151f3e59464`. It does not enter Echo’s default workspace or change `echo-cas`’s Rust 1.90 dependency graph.

`IdentityBinding::from_source` computes raw BLAKE3 for Echo and the version-1 Keep domain, bytes, and exact-length hash from one bounded stream. The binding keeps Keep coordinates private and has no persisted encoding. `verify_source` rechecks both identities and length. Neither method proves presence, retention, publication, or durability.

The conformance witness stages the named Keep golden vectors into `ReferenceStore`, reconstructs them, and rechecks exact bytes, both identities, and length. The largest fixture is one MiB. Source failures, limits, and coordinate substitution refuse a complete binding. This does not establish a durable backend or a physical-content port.

Run these checks inside the admitted, bounded Docker worker:

```sh
cargo +1.96.0 test --manifest-path experiments/echo-keep/Cargo.toml --locked
cargo +1.96.0 clippy --manifest-path experiments/echo-keep/Cargo.toml --locked --all-targets -- -D warnings
cargo +1.96.0 fmt --manifest-path experiments/echo-keep/Cargo.toml -- --check
cargo +1.90.0 check --locked -p echo-cas
```

[The canonical boundary](../../docs/architecture/echo-keep-physical-content-boundary.md) owns the contract. [Issue #759](https://github.com/flyingrobots/echo/issues/759) owns this identity slice.
