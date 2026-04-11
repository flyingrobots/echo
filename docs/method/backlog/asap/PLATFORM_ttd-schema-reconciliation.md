<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Reconcile TTD protocol schemas with warp-ttd

Echo has local TTD protocol artifacts that predate warp-ttd:

- `ttd-protocol-rs` — generated Rust types still described as if Wesley or a
  repo-local schema were the direct source of truth
- `packages/ttd-protocol-ts` — generated TypeScript package carrying the same
  ownership ambiguity

warp-ttd is now the canonical debugger project. Its schema at
`schemas/warp-ttd-protocol.graphql` should be the single source of
truth.

Work:

- Point `ttd-protocol-rs` generation at warp-ttd's canonical schema
- Keep generated crates/packages clearly marked as downstream consumers, not
  backup protocol owners
- Verify generated types still satisfy `echo-ttd` compliance checker
