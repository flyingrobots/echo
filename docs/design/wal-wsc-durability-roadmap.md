<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# WAL/WSC Durability Roadmap

Status: active roadmap packet.

Last updated: 2026-06-19.

## Thesis

Echo already has real WAL, WSC, retained-evidence, and CAS primitives. The next
durability work is not "invent a WAL" or "invent WSC." The next work is to make
their relationship release-grade:

```text
WAL bytes are the durable commit authority.
WARP graph WAL facts are projected evidence.
WSC carries or references that evidence.
CAS stores bytes by content hash.
Retained evidence refs bind semantic coordinates to byte identity.
Recovery bootstraps from WAL root or storage manifest material.
```

The unsafe model is circular:

```text
graph facts tell Echo where the WAL is
WAL replay tells Echo how to rebuild graph facts
```

The release model is acyclic:

```text
configured WAL root or storage manifest
-> validate segment headers, writer epochs, commit chains, and manifests
-> replay committed transactions
-> rebuild indexes, graph projections, reading refs, and WSC exports
```

This roadmap is the bridge between the implemented primitives and the release
claim. Goalposts are milestones. Slices are PR-sized GitHub issue bodies.

## Current Evidence

This packet was written after reading the current implementation. Code remains
the source of truth.

| Surface           | Current evidence                                                         | What is already true                                                                                                                                                                                                                                                                    |
| ----------------- | ------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| WAL core          | `crates/warp-core/src/causal_wal.rs`                                     | Typed LSNs, transactions, record kinds, append authorities, writer epochs, commit markers, recovery reports, recovery certificates, retained-material records, materialization outbox records, filesystem segment storage, manifests, and read-only/writable recovery modes exist.      |
| Runtime ACK WAL   | `crates/warp-core/src/trusted_runtime_host.rs`                           | Runtime ACK tests commit accepted submissions and scheduler tick receipts before app-visible success, roll back on WAL failure, and rebuild submission/receipt indexes from committed WAL material.                                                                                     |
| Filesystem WAL    | `FilesystemWalStore` in `causal_wal.rs`                                  | Segment files live under the canonical `segments/` namespace, commit append syncs the file, manifests are written atomically, read-only recovery does not truncate, and writable recovery can truncate uncommitted tails.                                                               |
| WAL release gates | `docs/BEARING.md`, `docs/design/causal-wal-hardening-matrix.md`, `xtask` | The previous WAL hardening batch is recorded as complete, and `cargo xtask test-slice runtime-wal-ack` guards the runtime ACK witness.                                                                                                                                                  |
| WSC core          | `crates/warp-core/src/wsc/`                                              | WSC writing, validation, borrowed views, CLI verify/inspect, and single-warp graph reconstruction are implemented.                                                                                                                                                                      |
| WSC store         | `crates/warp-core/src/wsc/store.rs`                                      | WSC store envelopes bind record kind, basis digest, schema hash, tick, payload digest, and payload length. The in-memory store stages writes, publishes commit markers, lists only committed envelopes, and recovers accepted submissions, receipt correlations, and retention records. |
| Retained refs     | `crates/warp-core/src/retained_evidence.rs`                              | Retained evidence refs separate semantic coordinates, content hashes, byte lengths, missing-coordinate posture, and missing-content posture.                                                                                                                                            |
| CAS               | `crates/echo-cas/`                                                       | CAS hash is content-only. `MemoryTier` stores opaque bytes. `RetainedBlobIndex` maps semantic coordinates to content hashes and supports bounded reads with typed missing-coordinate and missing-blob errors.                                                                           |

## Remaining Gaps

The next release work should address these gaps in order:

1. Runtime WAL ACK is gated through an in-memory adapter. The strict filesystem
   WAL implementation exists separately, but the trusted runtime host does not
   yet run its app-facing ACK path against a configured filesystem WAL root.
2. `CausalCommitEvidence` exists, but the fuller `WalRoot`,
   `WalWriterEpoch`, `WalSegmentRef`, `WalCommitAnchor`, and
   `RecoveryCertificateRef` projection family is still mostly doctrine.
3. WSC can store causal-history envelopes, but the three release export modes
   are not implemented as concrete export/import profiles:
   ref-only, self-contained, and CAS-addressed.
4. WSC store is currently an in-memory port. There is no filesystem/object
   WSC store adapter or CLI export/import flow for causal-history bundles.
5. Retained evidence and `echo-cas` have semantic identities, but durable
   retained byte storage is still memory-local unless another host supplies
   the storage medium.
6. Recovery does not yet have one unified release fixture that starts from a
   WAL root or manifest, rebuilds graph/WSC/retention projections, and proves
   app-facing outcomes/readings without a pre-existing graph.

## Goalpost 0: Roadmap And Doctrine Lock

Purpose: make the next work visible, non-circular, and issue-sized before
runtime changes resume.

Status: in progress on the `cycle/521-wal-wsc-storage-relationship` branch.

### GP0-S1 - WAL/WSC Doctrine Guard

Parent issues: [#521](https://github.com/flyingrobots/echo/issues/521).

Scope:

- Keep BEARING, WorkItems, sequencing, and the causal WAL design linked to
  issue `#521`.
- Mechanically check WAL authority, graph projection, WSC modes, locator
  semantics, bootstrap recovery, and naming grammar.

Acceptance:

- `scripts/check-wal-wsc-doctrine.sh` fails when the active docs lose the
  doctrine.
- The guard is covered by `scripts/tests/check_wal_wsc_doctrine_test.sh`.

Witness:

```bash
scripts/check-wal-wsc-doctrine.sh
scripts/tests/check_wal_wsc_doctrine_test.sh
```

### GP0-S2 - Durability Roadmap Packet

Parent issues:
[#521](https://github.com/flyingrobots/echo/issues/521),
[#522](https://github.com/flyingrobots/echo/issues/522),
[#519](https://github.com/flyingrobots/echo/issues/519).

Scope:

- Add this roadmap.
- Link it from active signposts.
- Split the remaining work into goalposts and PR-sized slices.
- Remove or quarantine stale filesystem-backlog references for WAL/WSC work.

Acceptance:

- The roadmap distinguishes implemented primitives from missing release joins.
- Each future slice names parent issues, acceptance criteria, and a witness.
- Active signposts point to GitHub issues and this packet, not deleted backlog
  files.

Witness:

```bash
scripts/check-wal-wsc-doctrine.sh
pnpm exec markdownlint-cli2 docs/design/wal-wsc-durability-roadmap.md
```

## Goalpost 1: Durable Runtime WAL Join

Purpose: connect the trusted runtime ACK path to a configured durable WAL
adapter without granting applications WAL append, scheduler, or recovery
authority.

Current gap: `TrustedRuntimeWal` proves the semantics with `InMemoryWalStore`.
`FilesystemWalStore` proves strict filesystem storage separately. The release
join is not yet implemented.

### GP1-S1 - Runtime WAL Store Adapter Boundary

Parent issues:
[#510](https://github.com/flyingrobots/echo/issues/510),
[#518](https://github.com/flyingrobots/echo/issues/518),
[#521](https://github.com/flyingrobots/echo/issues/521).

Scope:

- Introduce a trusted-host-owned runtime WAL adapter boundary over
  `WalStorePort`.
- Keep application APIs limited to submission handles and outcome observation.
- Preserve the existing in-memory runtime WAL adapter for fast tests.

Acceptance:

- `TrustedRuntimeHost` owns the adapter configuration.
- `TrustedRuntimeApp` exposes no WAL append, flush, truncate, manifest, tick,
  or recovery method.
- Existing runtime WAL ACK tests still pass through the adapter boundary.

Witness:

```bash
cargo test -p warp-core --features "native_rule_bootstrap trusted_runtime host_test" --test trusted_runtime_host_loop_tests runtime_wal_ack
```

### GP1-S2 - Filesystem Runtime WAL ACK Path

Parent issues:
[#510](https://github.com/flyingrobots/echo/issues/510),
[#518](https://github.com/flyingrobots/echo/issues/518).

Scope:

- Add a host API for configuring a runtime WAL root backed by
  `FilesystemWalStore`.
- Commit accepted submissions and scheduler tick receipts through filesystem
  WAL before app-visible ACK/outcome success.
- Recover pending and decided submissions from the filesystem root after host
  reconstruction.

Acceptance:

- A test submits an intent through the filesystem runtime WAL ACK path,
  reconstructs a fresh host/recovery view from the WAL root, and observes the
  same submission id, envelope digest, receipt posture, and recovery
  certificate indexes.
- The app never receives the WAL root or store handle.

Witness:

```bash
cargo test -p warp-core --features "native_rule_bootstrap trusted_runtime host_test" --test trusted_runtime_host_loop_tests filesystem_runtime_wal_ack
```

### GP1-S3 - Filesystem Runtime WAL Failure Atomicity

Parent issues:
[#510](https://github.com/flyingrobots/echo/issues/510),
[#518](https://github.com/flyingrobots/echo/issues/518).

Scope:

- Inject filesystem WAL append/flush/manifest failures around submission and
  tick boundaries.
- Prove failed WAL commits roll back app-visible submission intake or tick
  outcome publication.

Acceptance:

- A failed pre-ACK submission WAL commit leaves no witnessed submission visible.
- A failed scheduler tick WAL commit leaves no receipt correlation or applied
  outcome visible.
- Multi-head tick failure rolls back every tick record from the attempted
  scheduler pass.

Witness:

```bash
cargo test -p warp-core --features "native_rule_bootstrap trusted_runtime host_test" --test trusted_runtime_host_loop_tests filesystem_runtime_wal_failure
```

### GP1-S4 - Runtime WAL Recovery CLI Contract

Parent issues:
[#510](https://github.com/flyingrobots/echo/issues/510),
[#517](https://github.com/flyingrobots/echo/issues/517).

Scope:

- Extend CLI/read-model coverage so a filesystem runtime WAL root reports
  accepted-pending, decided-applied, decided-rejected, and obstructed posture
  through stable JSON.
- Keep the CLI read-only.

Acceptance:

- `echo-cli wal doctor --format json` reports recovery posture and tail
  posture for a runtime WAL root.
- `echo-cli wal submission-posture --format json` reports retry posture,
  recovered posture, receipt digest, and ticket digest for filesystem WAL
  roots produced by runtime ACK tests.

Witness:

```bash
cargo test -p warp-cli --test cli_integration wal_submission_posture
```

### GP1-S5 - Durable Runtime WAL Gate

Parent issues:
[#510](https://github.com/flyingrobots/echo/issues/510),
[#526](https://github.com/flyingrobots/echo/issues/526).

Scope:

- Add an `xtask` test slice for the filesystem-backed runtime WAL release
  witness.
- Keep the current `runtime-wal-ack` slice as the fast semantic gate.

Acceptance:

- `cargo xtask test-slice durable-runtime-wal` or equivalent runs the
  filesystem ACK, filesystem failure, CLI posture, stale-claim, and man-page
  checks.

Witness:

```bash
cargo xtask test-slice durable-runtime-wal
```

## Goalpost 2: WAL Evidence Projection

Purpose: project WAL-backed evidence into WARP-readable facts without making
those facts bootstrap recovery authority.

Current gap: `CausalCommitEvidence` exists, but the broader graph-projected WAL
fact family is not yet implemented as a coherent surface.

### GP2-S1 - WAL Projection Fact Types

Parent issues:
[#521](https://github.com/flyingrobots/echo/issues/521),
[#525](https://github.com/flyingrobots/echo/issues/525).

Scope:

- Add typed projection records for `WalRoot`, `WalWriterEpoch`,
  `WalSegmentRef`, `WalCommitAnchor`, and `RecoveryCertificateRef`.
- Keep storage locators separate from causal identity.

Acceptance:

- `WalSegmentRef` identity is based on writer epoch, LSN range, commit digest
  chain, segment digest, commit anchors, and sealed posture.
- Raw absolute paths cannot participate in projection identity.
- Projection records are read-model evidence, not `WalStorePort`.

Witness:

```bash
cargo test -p warp-core wal_projection_fact_identity
```

### GP2-S2 - Projection From WAL Recovery

Parent issues: [#521](https://github.com/flyingrobots/echo/issues/521).

Scope:

- Build projection records from `RecoveryScanReport`, WAL manifests, segment
  seals, and recovery certificates.
- Support absent and obstructed postures without inventing evidence.

Acceptance:

- Recovery from a WAL root yields deterministic projection records.
- Missing manifests or unavailable locators produce typed projection
  obstruction, not empty success.
- Projection does not mutate WAL or graph storage.

Witness:

```bash
cargo test -p warp-core wal_projection_from_recovery
```

### GP2-S3 - WARP Graph Materialization Of WAL Evidence

Parent issues:
[#521](https://github.com/flyingrobots/echo/issues/521),
[#522](https://github.com/flyingrobots/echo/issues/522).

Scope:

- Materialize the projection records into a generic `GraphStore` shape suitable
  for WSC serialization.
- Keep graph nodes as projected facts.

Acceptance:

- `WalRoot` connects to writer epochs, segments, commit anchors, and recovery
  certificate refs.
- The materialized graph can be serialized to WSC without raw WAL append
  authority.
- Rebuilding the graph from the same WAL recovery evidence produces identical
  graph/WSC bytes.

Witness:

```bash
cargo test -p warp-core wal_projection_graph_materializes_deterministically
```

### GP2-S4 - Projection Authority Negative Cases

Parent issues:
[#521](https://github.com/flyingrobots/echo/issues/521),
[#525](https://github.com/flyingrobots/echo/issues/525).

Scope:

- Prove graph-projected WAL facts cannot append records, validate recovery, or
  bypass current revocation/storage checks.

Acceptance:

- There is no public path from projected facts to `WalStorePort::append_frame`,
  `flush_commit`, `truncate_tail_after`, or `publish_manifest`.
- Importing projected facts without WAL bytes/manifests produces observation
  evidence only, not recovered causal authority.

Witness:

```bash
cargo test -p warp-core wal_projection_cannot_bootstrap_recovery
```

## Goalpost 3: WSC Causal-History Export And Import

Purpose: turn the doctrine in [#522](https://github.com/flyingrobots/echo/issues/522)
into concrete export/import profiles.

Current gap: WSC causal-history envelopes exist for accepted submissions,
receipt correlations, and retention records. WAL segment evidence export modes
do not yet exist as concrete profiles.

### GP3-S1 - WSC Causal-History Export Profiles

Parent issues:
[#522](https://github.com/flyingrobots/echo/issues/522),
[#521](https://github.com/flyingrobots/echo/issues/521).

Scope:

- Add a versioned export profile model for:
  `ref-only`, `self-contained`, and `CAS-addressed`.
- Define what evidence each profile must carry.

Acceptance:

- Ref-only profile carries projected graph facts, locators, segment digests,
  LSN ranges, and commit anchors.
- Self-contained profile carries enough segment bytes or retained material to
  validate the segment digest and commit chain without external WAL storage.
- CAS-addressed profile carries content hashes and semantic refs without
  claiming the CAS tier is itself causal authority.

Witness:

```bash
cargo test -p warp-core wsc_causal_history_export_profiles
```

### GP3-S2 - Ref-Only WSC Export Fixture

Parent issues: [#522](https://github.com/flyingrobots/echo/issues/522).

Scope:

- Export one recovered WAL segment reference plus accepted submission and
  receipt evidence as WSC.
- Keep locator strings non-authoritative.

Acceptance:

- Import validates the WSC payload, projected identities, segment digest,
  LSN range, and commit anchors.
- Import reports missing segment bytes as an explicit ref-only dependency, not
  as corruption.
- Absolute host paths are rejected or normalized out of causal identity.

Witness:

```bash
cargo test -p warp-core wsc_ref_only_export_preserves_wal_identity
```

### GP3-S3 - Self-Contained WSC Export Fixture

Parent issues: [#522](https://github.com/flyingrobots/echo/issues/522).

Scope:

- Export WAL segment bytes or bundled retained material inside WSC.
- Validate segment bytes against segment digest and commit chain on import.

Acceptance:

- Tampering with embedded segment bytes produces a typed digest/commit-chain
  obstruction.
- Self-contained import can rebuild accepted submission and receipt indexes
  without access to the original filesystem WAL root.

Witness:

```bash
cargo test -p warp-core wsc_self_contained_export_replays_segment_bytes
```

### GP3-S4 - CAS-Addressed WSC Export Fixture

Parent issues:
[#522](https://github.com/flyingrobots/echo/issues/522),
[#519](https://github.com/flyingrobots/echo/issues/519).

Scope:

- Export WSC graph facts plus content-addressed refs to WAL segments and
  retained material.
- Use `echo-cas` as byte storage, not semantic authority.

Acceptance:

- Import succeeds when required CAS blobs are present and match their content
  hashes.
- Import reports missing CAS blobs as typed missing material.
- Equal bytes under different semantic coordinates do not alias.

Witness:

```bash
cargo test -p warp-core wsc_cas_addressed_export_requires_present_blobs
cargo test -p echo-cas semantic_retention
```

### GP3-S5 - WSC Store Durability Adapter

Parent issues:
[#522](https://github.com/flyingrobots/echo/issues/522),
[#526](https://github.com/flyingrobots/echo/issues/526).

Scope:

- Add a filesystem or object-backed `WscStorePort` implementation.
- Preserve staged write plus commit-marker visibility semantics.

Acceptance:

- Staged WSC envelope material is not listed or read as committed until its
  marker is published.
- Restart reads committed envelopes in deterministic order.
- Torn envelope or marker material returns typed obstruction.

Witness:

```bash
cargo test -p warp-core filesystem_wsc_store
```

### GP3-S6 - WSC Export/Import CLI

Parent issues:
[#522](https://github.com/flyingrobots/echo/issues/522),
[#506](https://github.com/flyingrobots/echo/issues/506).

Scope:

- Add read-only CLI export/inspect/import-verification commands for WSC
  causal-history bundles.
- Avoid mutating Echo history during import verification.

Acceptance:

- CLI can export a fixture WAL root as ref-only WSC.
- CLI can verify self-contained WSC without the original WAL root.
- CLI reports unavailable WAL/CAS material as typed obstruction JSON.

Witness:

```bash
cargo test -p warp-cli --test cli_integration wsc_causal_history
```

## Goalpost 4: Retained Evidence Durability

Purpose: make retained-evidence durability honest across WAL, WSC, and CAS.

Current gap: semantic retained refs and memory CAS exist. Durable retained byte
storage, restart recovery, and app-facing reveal semantics are not fully joined.

### GP4-S1 - Retained Ref Crosswalk

Parent issues:
[#519](https://github.com/flyingrobots/echo/issues/519),
[#512](https://github.com/flyingrobots/echo/issues/512),
[#513](https://github.com/flyingrobots/echo/issues/513).

Scope:

- Define the exact crosswalk among `RetainedEvidenceRef`,
  `RetainedMaterialRecord`, `ReadingRefRecord`, `SemanticBlobCoordinate`, and
  `BlobHash`.
- Keep query identity, reading identity, semantic coordinate, and byte digest
  separate.

Acceptance:

- Equal bytes under different semantic coordinates produce distinct retained
  refs.
- Query identity alone does not imply retained payload availability.
- Retained reading id does not stand in for payload digest.

Witness:

```bash
cargo test -p warp-core --test retained_evidence_ref_tests
cargo test -p warp-core --test optic_retention_tests
```

### GP4-S2 - Durable Retained Blob Tier

Parent issues:
[#519](https://github.com/flyingrobots/echo/issues/519),
[#512](https://github.com/flyingrobots/echo/issues/512).

Scope:

- Add a filesystem-backed or host-supplied durable tier for `echo-cas`.
- Preserve content-only hash semantics.

Acceptance:

- Bytes written by content hash survive process reconstruction.
- `put_verified` rejects mismatched bytes without changing the store.
- Missing blobs remain absence, not generic I/O success.
- Listing, if added, returns sorted `BlobHash` values.

Witness:

```bash
cargo test -p echo-cas disk_tier
```

### GP4-S3 - WAL-After-Retention Commit Ordering

Parent issues:
[#519](https://github.com/flyingrobots/echo/issues/519),
[#521](https://github.com/flyingrobots/echo/issues/521).

Scope:

- Ensure WAL records retained material refs only after the referenced bytes are
  available or intentionally recorded as missing/redacted/obstructed posture.
- Make crash boundaries explicit.

Acceptance:

- Crash before WAL reference may leave orphan retained bytes, but no false
  causal claim.
- Crash after WAL reference can recover the retained ref or typed missing
  material posture.
- Retained material loss faults only at the appropriate scope:
  submission, receipt/ticket, runtime global, reading, or diagnostic loss.

Witness:

```bash
cargo test -p warp-core retained_material_wal_commit_order
```

### GP4-S4 - Retained Evidence WSC Export

Parent issues:
[#519](https://github.com/flyingrobots/echo/issues/519),
[#522](https://github.com/flyingrobots/echo/issues/522).

Scope:

- Export recovered retained material and reading refs through WSC.
- Support ref-only, self-contained, and CAS-addressed retained evidence modes.

Acceptance:

- Ref-only export names semantic coordinate and material digest without
  claiming bytes are available.
- Self-contained export can reveal the retained payload after import.
- CAS-addressed export succeeds only when required content hashes are present.

Witness:

```bash
cargo test -p warp-core wsc_retained_evidence_export_modes
```

### GP4-S5 - App-Safe Missing Retention Semantics

Parent issues:
[#519](https://github.com/flyingrobots/echo/issues/519),
[#517](https://github.com/flyingrobots/echo/issues/517).

Scope:

- Ensure app-safe readings and outcomes report missing retention as typed
  obstruction/posture, not empty success.

Acceptance:

- Missing reading payload bytes return missing-retention posture.
- Missing reading envelope bytes return missing-retention posture.
- Missing receipt evidence does not erase the applied/rejected causal outcome;
  it reports unavailable support material.

Witness:

```bash
cargo test -p warp-core retained_reading_missing_payload_is_not_empty_success
```

## Goalpost 5: Unified Bootstrap And Replay

Purpose: prove Echo can restart from durable causal-history material without a
pre-existing graph and without hidden application execution.

### GP5-S1 - Recovery Plan Object

Parent issues:
[#510](https://github.com/flyingrobots/echo/issues/510),
[#521](https://github.com/flyingrobots/echo/issues/521).

Scope:

- Introduce a recovery plan/report shape that starts from a WAL root or storage
  manifest, optionally selects a checkpoint, then replays committed suffix
  transactions.

Acceptance:

- The plan records bootstrap source, checkpoint posture, replay suffix, tail
  posture, index roots, retained-material posture, and projected evidence
  posture.
- The plan does not require graph WAL nodes as input.

Witness:

```bash
cargo test -p warp-core recovery_plan_bootstraps_from_wal_root
```

### GP5-S2 - Projection Rebuild After Recovery

Parent issues:
[#521](https://github.com/flyingrobots/echo/issues/521),
[#526](https://github.com/flyingrobots/echo/issues/526).

Scope:

- After WAL recovery, rebuild submission, receipt, retention, materialization,
  and graph/WSC projection indexes from committed transactions only.

Acceptance:

- Rebuilt indexes match live indexes for the fixture.
- Uncommitted tails do not appear in rebuilt indexes.
- Recovery does not call scheduler handlers, contract observers, wall clock,
  network, or app code.

Witness:

```bash
cargo test -p warp-core wal_recovery_rebuilds_all_durability_indexes
```

### GP5-S3 - Materialization Outbox Recovery

Parent issues:
[#519](https://github.com/flyingrobots/echo/issues/519),
[#526](https://github.com/flyingrobots/echo/issues/526).

Scope:

- Reconcile side-effect materialization intent and observation records with
  retained material and recovery posture.

Acceptance:

- Existing artifacts are detected by digest before retry.
- Effects are not replayed blindly after restart.
- Missing or mismatched artifacts return typed materialization posture.

Witness:

```bash
cargo test -p warp-core materialization_outbox_recovery
```

### GP5-S4 - Process-Kill Crashpoint Runner

Parent issues:
[#526](https://github.com/flyingrobots/echo/issues/526),
[#524](https://github.com/flyingrobots/echo/issues/524).

Scope:

- Promote the current `process.kill.after_wal_commit` future descriptor into a
  real process-level crashpoint witness.

Acceptance:

- A child process commits WAL material, exits before publication, and a parent
  process recovers the committed history.
- A crash before commit does not appear as accepted/decided history.

Witness:

```bash
cargo test -p echo-dind-tests wal_process_crashpoints
```

### GP5-S5 - DIND Durability Convergence Gate

Parent issues:
[#526](https://github.com/flyingrobots/echo/issues/526),
[#524](https://github.com/flyingrobots/echo/issues/524).

Scope:

- Add the WAL/WSC/retention durability path to DIND or equivalent convergence
  verification.

Acceptance:

- Live execution, WAL recovery, WSC export/import, and retained-material reveal
  converge on the same app-facing outcome and bounded reading.
- Corrupt or missing support material produces typed obstruction instead of
  divergence.

Witness:

```bash
cargo xtask dind
```

## Goalpost 6: Release Gate And Issue Closure

Purpose: convert the implementation path into release claims and issue closure.

### GP6-S1 - Durability Release Test Slice

Parent issues:
[#524](https://github.com/flyingrobots/echo/issues/524),
[#526](https://github.com/flyingrobots/echo/issues/526).

Scope:

- Add one narrow `xtask` slice that runs the durable runtime WAL, WSC export,
  retained evidence, and stale-claim guards.

Acceptance:

- The slice is fast enough for PR use or clearly classified as release-gate
  only.
- It is documented in `docs/workflows.md`.

Witness:

```bash
cargo xtask test-slice durability-release
```

### GP6-S2 - Documentation Truth Gate

Parent issues:
[#521](https://github.com/flyingrobots/echo/issues/521),
[#522](https://github.com/flyingrobots/echo/issues/522),
[#519](https://github.com/flyingrobots/echo/issues/519).

Scope:

- Extend doc guards so stale durability claims fail locally.
- Keep docs aligned with code status.

Acceptance:

- Docs cannot claim filesystem runtime ACK durability before the filesystem
  runtime WAL witness lands.
- Docs cannot claim WSC import recovery authority without WAL-backed
  validation.
- Docs cannot claim retained payload recovery from posture-only refs.

Witness:

```bash
cargo test -p xtask durability_stale_claims
scripts/check-wal-wsc-doctrine.sh
```

### GP6-S3 - Umbrella Issue Closure Audit

Parent issues:
[#521](https://github.com/flyingrobots/echo/issues/521),
[#522](https://github.com/flyingrobots/echo/issues/522),
[#519](https://github.com/flyingrobots/echo/issues/519),
[#510](https://github.com/flyingrobots/echo/issues/510),
[#526](https://github.com/flyingrobots/echo/issues/526).

Scope:

- Audit every acceptance criterion in the umbrella issues against code,
  witnesses, and docs.
- Close only criteria that are demonstrably implemented.

Acceptance:

- The audit names the commit/test/doc evidence for every closed criterion.
- Remaining criteria become new GitHub issues or stay open explicitly.

Witness:

```bash
gh issue view 521
gh issue view 522
gh issue view 519
cargo xtask test-slice durability-release
```

## Recommended Execution Order

Do this next:

1. GP1-S1 Runtime WAL Store Adapter Boundary.
2. GP1-S2 Filesystem Runtime WAL ACK Path.
3. GP1-S3 Filesystem Runtime WAL Failure Atomicity.
4. GP2-S1 WAL Projection Fact Types.
5. GP2-S2 Projection From WAL Recovery.
6. GP3-S1 WSC Causal-History Export Profiles.
7. GP3-S2 Ref-Only WSC Export Fixture.
8. GP4-S1 Retained Ref Crosswalk.
9. GP4-S2 Durable Retained Blob Tier.
10. GP5-S1 Recovery Plan Object.

The reason to start with GP1 is practical: Echo should not expand export and
retention claims while the product-facing runtime ACK path is still proving
durability through an in-memory adapter. Once the runtime path uses a durable
WAL adapter, graph projection and WSC export have a real source of authority.

## Non-Goals

- Do not store the WAL inside the WARP graph as the primary recovery
  mechanism.
- Do not make WSC a second commit authority.
- Do not make CAS hashes semantic reading identity.
- Do not treat ref-only WSC as self-contained recovery material.
- Do not let application code append WAL records, publish manifests, tick the
  scheduler, or invoke trusted recovery.
- Do not introduce jedit nouns into Echo core.
- Do not close umbrella issues because a related primitive exists; close them
  only when the integrated release claim has a witness.
