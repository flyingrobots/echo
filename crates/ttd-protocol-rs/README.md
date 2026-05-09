<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# ttd-protocol-rs

Generated Rust consumer types for the host-neutral TTD protocol.

Echo is not the source of truth for this protocol. The canonical authored
schema lives with `warp-ttd` at:

```text
warp-ttd/schemas/warp-ttd-protocol.graphql
```

This crate is a checked-in downstream consumer artifact produced through the
Wesley TTD generator path. Do not edit `lib.rs` by hand.

Local provenance check:

```sh
cargo xtask wesley sync
```

That check verifies that Echo no longer carries a backup
`schemas/ttd-protocol.graphql`, that the Rust and TypeScript generated
consumers agree on the canonical schema hash, and that Echo runtime compliance
code remains separate from host-neutral debugger protocol nouns.
