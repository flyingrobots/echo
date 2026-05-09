<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# @echo/ttd-protocol-ts

Generated TypeScript consumer types for the host-neutral TTD protocol.

Echo is not the source of truth for this protocol. The canonical authored
schema lives with `warp-ttd` at:

```text
warp-ttd/schemas/warp-ttd-protocol.graphql
```

This package is a checked-in downstream consumer artifact produced through the
Wesley TTD generator path. Do not edit `index.ts`, `types.ts`, `registry.ts`, or
`zod.ts` by hand.

Local provenance check:

```sh
cargo xtask wesley sync
```

That check verifies that the generated Rust and TypeScript consumers agree on
the canonical schema hash and that Echo remains a protocol consumer rather than
a backup protocol owner.
