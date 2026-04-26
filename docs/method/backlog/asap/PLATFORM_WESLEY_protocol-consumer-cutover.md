<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# WESLEY Protocol Consumer Cutover

Coordination: `WESLEY_protocol_surface_cutover`

Status: active and partially implemented. Echo's local TTD protocol
crates/packages are already marked as generated consumers, but the repo still
has protocol ownership and regeneration-command drift to reconcile.

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
- the remaining drift is ownership language and consumer wiring around the
  generated crates/packages
- `crates/ttd-protocol-rs/Cargo.toml` advertises `cargo xtask wesley sync`,
  but no `xtask wesley` command exists in this repo yet

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
