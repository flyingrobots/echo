<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# WESLEY Protocol Consumer Cutover

Coordination: `WESLEY_protocol_surface_cutover`

Status: complete. Echo's local TTD protocol crates/packages are marked as
generated consumers, and the local `cargo xtask wesley sync` check verifies the
downstream consumer handoff. Full external regeneration from the canonical
`warp-ttd` bundle remains with `PLATFORM_ttd-schema-reconciliation`.

## Completion evidence

- Added `cargo xtask wesley sync` as a Rust xtask provenance check for the
  generated protocol consumers.
- Added `crates/ttd-protocol-rs/README.md` and
  `packages/ttd-protocol-ts/README.md` to name Echo as a downstream consumer of
  `warp-ttd/schemas/warp-ttd-protocol.graphql`.
- The check proves Echo has no local `schemas/ttd-protocol.graphql`, the Rust
  and TypeScript generated consumers agree on schema hash, and `echo-ttd`
  runtime compliance stays separate from host-neutral protocol nouns.

Echo still carries local TTD protocol artifacts that predate the current
Continuum ownership split:

- `crates/ttd-protocol-rs`
- `packages/ttd-protocol-ts`
- `crates/echo-ttd/src/compliance.rs`

For the current Wesley-sponsored hill, Echo should stop acting like a backup
source of truth for the host-neutral debugger protocol and become a boring
consumer of the canonical authored schema plus Wesley-generated bundle.

Current repo truth:

- Echo no longer carries a local `schemas/ttd-protocol.graphql`
- local Rust and TypeScript protocol consumers name the canonical `warp-ttd`
  schema hash
- `cargo xtask wesley sync` verifies the consumer handoff locally
- exact external regeneration from the canonical bundle is tracked separately in
  `PLATFORM_ttd-schema-reconciliation`

Work:

- point local protocol crates and packages at the chosen canonical protocol
  bundle
- remove or clearly mark vendored schema and IR copies as derived or temporary
- keep Echo-owned hot runtime semantics and schema fragments separate from
  host-neutral debugger protocol nouns
- reconcile the advertised regeneration command with the actual repo tooling
- verify the local compliance lane still passes against generated artifacts
- coordinate with `PLATFORM_ttd-schema-reconciliation` instead of reopening the
  ownership question from scratch
