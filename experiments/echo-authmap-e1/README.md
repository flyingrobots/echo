<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Experiment E1: complete authenticated maps

This separate Rust 1.96 workspace holds two complete, persistent authenticated maps over 32-byte keys: `sparse-v0`, a fixed-depth sparse Merkle tree, and `radix-v0`, a compressed binary radix trie. Both support insert, replace, delete and batches, keep every version readable, and give membership and absence proofs with strict canonical codecs. An independent oracle rebuilds each root from the full record set. Nothing in Echo depends on this crate, and its `v0` codecs are not Echo commitment profiles.

[The E1 study](../../docs/studies/e1-authenticated-maps.md) owns the protocol, decision rule and results.

```sh
cargo +1.96.0 test --release --offline --locked --manifest-path experiments/echo-authmap-e1/Cargo.toml
cargo +1.96.0 clippy --release --offline --locked --manifest-path experiments/echo-authmap-e1/Cargo.toml --all-targets -- -D warnings
cargo +1.96.0 run --release --offline --locked --manifest-path experiments/echo-authmap-e1/Cargo.toml --bin e1 -- all
```

The harness prints one JSON object per line. The largest workload (`sparse-v0` at 64,000 keys) retains about 15 million nodes; run it under the project's resource guard.
