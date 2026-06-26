<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# WAL/WSC Release Closure Audit

Status: evidence audit.

This audit records the closure boundary for the WAL/WSC durability issues that
fed the GP6 release-gate slices:

- [#519 Retained Evidence Durability Boundary](https://github.com/flyingrobots/echo/issues/519)
- [#521 WAL/WSC Storage Relationship](https://github.com/flyingrobots/echo/issues/521)
- [#522 WSC Causal-History Storage](https://github.com/flyingrobots/echo/issues/522)
- [#526 v0.1.0 Replay And DIND Proof](https://github.com/flyingrobots/echo/issues/526)
- [#370 Track Echo v0.1.0 release bar](https://github.com/flyingrobots/echo/issues/370)

It is not a replacement for GitHub issue state. Close issues only when the PR
that carries the cited evidence is merged and CI confirms the commands below.

## Release Witness

The joined release witness is:

```sh
cargo xtask test-slice durability-release
```

That slice joins these evidence families:

- filesystem runtime WAL ACK and filesystem WAL failure atomicity;
- CLI submission posture JSON;
- WSC retained evidence recovery and conflict obstruction;
- WSC topology recovery and uncommitted staged-envelope exclusion;
- topology WAL recovery for strand forks, braid shells, and suffix imports;
- typed missing-material obstruction for retained evidence;
- stale durability claim guards;
- WAL/WSC doctrine shell guard;
- generated man-page freshness.

The direct stale-claim and doctrine witnesses remain separately callable:

```sh
cargo test -p xtask durability_stale_claims
scripts/check-wal-wsc-doctrine.sh
```

## Criteria Audit

### #521 WAL/WSC Storage Relationship

Ready to close after merge.

Evidence:

- `docs/design/causal-wal-end-to-end.md` defines WAL bytes as durable commit
  authority, graph facts as projected evidence, storage locators as non-causal
  identity, WSC export modes, record naming, and recovery bootstrap from WAL
  root or storage manifest material.
- `docs/design/wal-wsc-durability-roadmap.md` preserves the stable doctrine.
- `scripts/check-wal-wsc-doctrine.sh` fails if the required doctrine is removed
  from BEARING, WorkItems, sequencing, the WAL design, release contract, WAL
  doctrine, or the WAL topic.
- `cargo xtask test-slice durability-release` runs the doctrine guard with the
  durability and WSC witnesses.

### #522 WSC Causal-History Storage

Partially ready. Keep the umbrella open unless the project owner chooses to
close the doctrine/storage subset and track full import separately.

Evidence now present:

- ref-only, self-contained, and CAS-addressed WSC modes are documented in
  `docs/design/causal-wal-end-to-end.md`;
- WSC retained-evidence envelopes round-trip and recover from committed WSC
  store entries in `crates/warp-core/tests/wsc_store_tests.rs`;
- topology records round-trip through WSC envelopes, recover from committed WSC
  store entries, ignore uncommitted staged entries, and reject conflicting
  duplicate strand forks;
- the stale-claim guard rejects prose that says WSC import recovery is
  authoritative without WAL-backed validation.

Still open:

- a full Continuum replica import fixture remains outside this storage slice;
- hostile-network and governance concerns remain explicitly non-goals here.

### #519 Retained Evidence Durability Boundary

Ready to close after merge for the retained-evidence boundary described by the
issue.

Evidence:

- `docs/design/causal-wal-end-to-end.md` distinguishes retained-evidence
  posture from durable recovery evidence, semantic lookup identity, and byte
  identity.
- `crates/warp-core/tests/causal_wal_tests.rs` covers typed obstruction for
  missing retained material.
- `crates/warp-core/tests/wsc_store_tests.rs` covers retained-material WSC
  round-trip, committed-store recovery, basis mismatch obstruction, conflicting
  material digest obstruction, and conflicting reading id obstruction.
- `cargo test -p xtask durability_stale_claims` rejects posture-only retained
  payload recovery claims.

### #526 v0.1.0 Replay And DIND Proof

Ready to close after merge for the documented narrow release witness.

Evidence:

- `cargo xtask test-slice contract-path-release` remains the local
  contract-host release witness.
- `cargo xtask test-slice durability-release` adds the joined WAL/WSC recovery
  witness for durability and retained evidence.
- Broader DIND remains valuable, but #526 already permits the narrower
  documented release witness.

### #370 Echo v0.1.0 Release Bar

Keep open.

The durability and replay criteria are now backed by concrete local witnesses,
but #370 tracks the full release bar. Authority boundary, clean-checkout
quickstart, package/versioning, and release operations remain broader than this
WAL/WSC closure audit.

## Closure Rule

Do not close umbrella release issues from prose alone. The closure event should
name:

- the merged PR;
- the commit or merge commit;
- the commands that passed locally or in CI;
- any criteria intentionally left open with their owning issue.
