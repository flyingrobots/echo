<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Reconcile TTD protocol schemas with warp-ttd

Status: active and partially implemented. Echo's generated Rust and TypeScript
protocol consumers are labeled as generated from the canonical `warp-ttd`
protocol, and `cargo xtask wesley sync` now verifies local downstream-consumer
provenance. The remaining gap is the full external handoff from the canonical
schema bundle to checked-in generated artifacts.

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

- Extend the current `cargo xtask wesley sync` provenance check into a
  regeneration or bundle-ingest path once the external canonical bundle is
  published for Echo consumption.
- Point protocol generation at the canonical `warp-ttd` schema or document the
  exact external bundle handoff if generation stays outside this repo.
- Keep generated crates/packages clearly marked as downstream consumers, not
  backup protocol owners.
- Verify generated types still satisfy the `echo-ttd` compliance checker and
  local browser adapter surfaces.
- Preserve the completed WESLEY protocol consumer cutover decision instead of
  reopening protocol ownership from scratch.
