<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Reconcile TTD protocol schemas with warp-ttd

Status: active and partially implemented. Echo's generated Rust and TypeScript
protocol consumers are already labeled as generated from the canonical
`warp-ttd` protocol. The remaining gap is provenance/tooling: the advertised
regeneration command does not exist locally, and Echo still needs a verified
handoff from the external canonical schema to the checked-in generated
artifacts.

Echo has local TTD protocol artifacts that must stay downstream of `warp-ttd`:

- `ttd-protocol-rs` — generated Rust types from the canonical `warp-ttd`
  protocol
- `packages/ttd-protocol-ts` — generated TypeScript types from the same
  canonical protocol

warp-ttd is now the canonical debugger project. Its schema at
`schemas/warp-ttd-protocol.graphql` should be the single source of
truth; Echo should consume it through a reproducible generation path rather than
acting as a backup schema owner.

Work:

- Reconcile `crates/ttd-protocol-rs/Cargo.toml` advertising
  `cargo xtask wesley sync` with the actual repo tooling.
- Point protocol generation at the canonical `warp-ttd` schema or document the
  exact external bundle handoff if generation stays outside this repo.
- Keep generated crates/packages clearly marked as downstream consumers, not
  backup protocol owners.
- Verify generated types still satisfy the `echo-ttd` compliance checker and
  local browser adapter surfaces.
- Coordinate with `PLATFORM_WESLEY_protocol-consumer-cutover` instead of
  reopening protocol ownership from scratch.
