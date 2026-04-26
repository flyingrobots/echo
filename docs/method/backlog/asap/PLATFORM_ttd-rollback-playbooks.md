<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Commit-ordered rollback playbooks for TTD integration

Ref: #282

Status: active and not yet implemented. Echo has live TTD-facing surfaces, but
no owned rollback playbook for protocol or adapter breakage.

Document rollback procedures for the TTD integration path. If a `warp-ttd`
protocol change or Echo adapter change breaks the integration, the repo needs a
commit-ordered rollback sequence instead of improvised reverts.

Current integration seams:

- `crates/ttd-protocol-rs` and `packages/ttd-protocol-ts` are generated
  protocol consumers.
- `crates/echo-ttd` owns Echo-side compliance and violation reporting.
- `crates/ttd-browser`, `apps/ttd-app`, and `crates/echo-wasm-bindings/src/ttd.rs`
  are local browser/debugger adapter surfaces.
- `warp-ttd` owns debugger protocol semantics; Echo owns generated consumer
  wiring and substrate-side compatibility.

Work:

- Write the rollback sequence for protocol schema changes:
    - revert generated Rust/TS protocol artifacts
    - revert adapter glue that depends on the schema
    - restore compliance expectations or mark them explicitly incompatible
    - rerun the narrow TTD/compliance verification commands
- Write the rollback sequence for Echo adapter changes that break a stable
  `warp-ttd` protocol.
- State which repo owns each revert decision and which commits must move first.
- Coordinate with `PLATFORM_ttd-schema-reconciliation` so schema provenance and
  rollback provenance do not drift separately.

Done looks like:

- One playbook doc names the rollback owner split between Echo and `warp-ttd`.
- The playbook includes before/after validation commands.
- The sequence distinguishes protocol rollback from Echo adapter rollback.
- Generated artifacts remain treated as consumers, not protocol authority.
