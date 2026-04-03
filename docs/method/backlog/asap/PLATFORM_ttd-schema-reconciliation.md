<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Reconcile TTD protocol schemas with warp-ttd

Echo has local TTD protocol artifacts that predate warp-ttd:

- `ttd-protocol-rs` — generated Rust types from a local GraphQL schema
- `ttd-manifest` — vendored IR for the protocol
- `schemas/ttd-protocol.graphql` — local copy of the schema

warp-ttd is now the canonical debugger project. Its schema at
`schemas/warp-ttd-protocol.graphql` should be the single source of
truth.

Work:

- Point `ttd-protocol-rs` generation at warp-ttd's canonical schema
- Remove or redirect `ttd-manifest` to consume warp-ttd's IR
- Delete Echo's local `schemas/ttd-protocol.graphql` if redundant
- Verify generated types still satisfy `echo-ttd` compliance checker
