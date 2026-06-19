<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# BEARING

Last updated: 2026-06-15.

This signpost summarizes current direction. It does not create commitments or
replace backlog items, design docs, retros, or CLI status. If it disagrees with
code, the code wins and this file should be corrected.

The WARP paper-to-Echo noun map is maintained in
`docs/design/warp-optic-implementation-map.md`.

The post-PR #545 strands and braids hardening roadmap is maintained in
`docs/design/braids-and-strands-roadmap.md`.

Its goalpost design packet is maintained under
`docs/design/braids-and-strands-hardening/`.

The current Graft-to-Echo readiness boundary is maintained in
`docs/design/graft-echo-native-readiness-boundary.md`.

The feature bar for the eventual `v0.1.0` release is maintained in
`docs/design/v0.1.0-release-plan.md`.

The current external release gate is maintained in
`docs/design/v0.1.0-jedit-release-gate.md`.

Trusted runtime-control history is defined in
`docs/design/trusted-runtime-control-history.md`.

The causal WAL doctrine and recovery design is defined in
`docs/design/causal-wal-end-to-end.md`.

The Echo-owned file aperture design is defined in
`docs/design/echo-owned-file-aperture.md`.

The WAL/WSC storage relationship is tracked by
[#521 WAL/WSC Storage Relationship](https://github.com/flyingrobots/echo/issues/521)
and grounded in `docs/design/causal-wal-end-to-end.md`.

The WAL/WSC/durability goalpost roadmap is maintained in
`docs/design/wal-wsc-durability-roadmap.md`.

The next ten jedit release-gate slices are planned in
`docs/design/v0.1.0-jedit-next-ten-slices.md`.

The current sequencing filter for audited work items is maintained in
`docs/design/work-item-sequencing-and-prioritization.md`.

GitHub Issues are the live backlog. `docs/method/backlog/` remains only as a
legacy workspace-discovery marker.

The production-core app-noun guard is `scripts/check-no-app-nouns-in-core.sh`.
It checks that hardcoded jedit/Stack Witness fixture shortcuts stay out of
Echo crate source. The guard is intentionally production-source-scoped: tests
and docs may still carry app-shaped fixtures as external-consumer examples,
but production Echo code must remain generic.

## Current Bearing

The active architecture hardening focus is the post-PR #545 braids and strands
campaign. Track progress through the goalpost and slice checklist in
`docs/design/braids-and-strands-roadmap.md`. Each goalpost has a focused design
document under `docs/design/braids-and-strands-hardening/`; implementation PRs
must check off slices in the roadmap only when the slice actually lands.

This hardening campaign does not replace the `v0.1.0` external-app release
gate. It protects the newly landed strand, braid, proof, sealed-member,
identity, witness, and plurality surfaces before more callers depend on them.

The current Graft posture is deliberately bounded: Graft can proceed with
schema, model, adapter, and local Echo witness design work, but it must not yet
claim production-grade Echo-native structural history through a stable
TypeScript dependency or durable retained evidence path. The boundary is
recorded in `docs/design/graft-echo-native-readiness-boundary.md`.

Echo has a local witnessed intent pipeline into deterministic execution:
application ingress can become witnessed submission history, lawful admission
evidence, ticketed runtime ingress, scheduler-owned handler dispatch, receipt
correlation, and observable intent outcome.

The release priority remains proving that pipeline with `jedit` as a real external
consumer. The in-repo external fixture remains valuable, but it is no longer
the `v0.1.0` release gate. Echo is not ready to release until jedit can submit
an application-owned contract intent, let a trusted Echo host authorize
scheduler opportunities, observe the outcome, query a bounded reading, retain
evidence, and replay the result without moving application nouns into Echo core.

The immediate durability hill is Echo's causal WAL: Echo may only claim what
its WAL can recover. The WAL must make accepted submissions, tick outcomes,
runtime posture, retained-material references, and side-effect authorization
crash-recoverable without giving applications tick or WAL authority.

## What Is Already True

- Echo has deterministic execution through `WorldlineRuntime`,
  `SchedulerCoordinator::super_tick(...)`, and `Engine::commit_with_state(...)`.
- Application-facing `dispatch_intent(...)` submits canonical EINT bytes; it does
  not tick the runtime.
- Trusted runtime control owns scheduler runs through the separate
  `TrustedKernelControlPort` boundary.
- Fixed logical timestep doctrine exists. Wall-clock cadence is host/runtime
  owner policy, not semantic Echo history.
- Tick receipts exist and witness scheduler-owned candidate outcomes.
- Scheduler-owned tick receipts can be correlated back to ticketed runtime
  ingress records, admission ticket digests, and witnessed submission ids.
- Core can observe a witnessed submission as unknown, pending, or decided by a
  scheduler-owned tick receipt.
- Core exposes scheduler-owned EINT contract-host helpers so installed
  `cmd/*` handlers can match operation ids, borrow canonical vars bytes for
  generated decoding, and declare the standard runtime-ingress read footprint.
- `echo-wesley-gen --contract-host` emits std-only mutation helper rules for
  that seam: stable command-rule names, op-id matchers, typed vars decoders,
  base runtime-ingress read footprints, and rule constructors that accept
  host-supplied executor and footprint functions.
- Core routes `QueryView`/`Query` observations to installed contract query
  observers keyed by generated query op id. Observers receive canonical vars
  bytes and the resolved causal basis, emit `QueryBytes`, and stamp the
  `ReadingEnvelope` with authored observer plan identity.
- App-safe observation and WASM ABI surfaces carry generic retained-evidence
  posture for installed contract QueryView readings. The envelope names
  missing reading-envelope coordinates and missing reading-payload content
  refs without exposing trusted runtime control or importing application nouns.
- `echo-wesley-gen --contract-host` emits std-only query observer helpers for
  that seam: deterministic authored observer plan identity, typed context-vars
  decoders, and read-only observer constructors that install host closures into
  `warp-core`.
- Footprint conflicts are explicit receipt rejections, not hidden retries.
- Failed `SuperTick` attempts are failure-atomic: uncommitted runtime,
  provenance, and receipt-correlation writes are rolled back before any fault
  posture is recorded.
- Scoped internal scheduler faults quarantine the culprit writer head. Healthy
  unrelated heads remain eligible for later scheduler-owned ticks.
- Unscoped scheduler faults quarantine the runtime until trusted recovery.
- The optic admission ladder resolves through AdmissionTicket and currently
  can stage ticketed runtime ingress through an explicit runtime-owner authority
  token without ticking.
- Echo implements the WARP paper's application/compiler seam with generated
  request helpers, mutation host helpers, and query observer host helpers while
  keeping Echo core free of application nouns.

## What Is Not Yet True

- Accepted submissions are not yet durable restart-proof ingress history;
  current replay records prove deterministic import shape, not persistence.
- Product-facing clients do not yet have polished ABI/helper surfaces for
  per-intent applied/rejected semantics.
- Contract-aware obstruction taxonomy and product-facing error surfaces still
  need release-grade stabilization.
- The semantic retention layer is local and in-memory. App-safe readings can
  now report generic missing-retention posture, but durable retained artifact,
  witness, receipt, and reading recovery remains future work.
- Generic external contract proof exists, but the release gate now requires
  real `jedit` follow-through from the sibling repository. jedit now has a
  local app/host split for its opt-in real-WASM witness; the remaining gap is
  moving from the old stack-witness fixture shape to retained evidence, replay,
  and a jedit-owned generated contract path.

## Doctrine

Echo accepts intent submissions as witnessed ingress history.

Application-authored optics do not create ticks.

Application-authored surfaces may declare runtime-retained consequence
obligations, including receipt obligations. Echo satisfies those obligations
only through trusted runtime-owned execution.

Echo does not execute submissions synchronously.

Echo's trusted runtime owner controls tick boundaries.

Start, Stop, SetCadence, and DrainUntilIdle are trusted runtime-control
history. They authorize or suspend scheduler opportunities; they do not create
ticks and they are not application/domain intents.

A tick receipt witnesses the scheduler-owned decision.

A rejected candidate remains witnessed history.

Rollback is tick-local cleanup of an uncommitted failed scheduler transaction.

Quarantine is runtime-local control posture after an internal fault. Durable
fault evidence remains a follow-up control-plane/provenance boundary.

Lawful rejection is not a fault.

Fault recovery is trusted runtime control, not application behavior.

Retry is a new explicit causal act.

AdmissionTicket is not execution.

TickReceipt is not AdmissionTicket.

QueryView remains an observer-relative read. It does not mutate state, tick the
runtime, or execute handlers outside scheduler-owned writes.

QueryView/Query routes to installed contract query observers when a matching
observer is registered. This is a real bridge, not the full observer-rights or
revelation lattice.

Transport arrival is not semantic Echo history. Echo acceptance is semantic
ingress history.

Submission order may be witnessed. Submission order must not decide scheduler
order.

Continuum is the protocol-shaped causal medium. Echo is a concrete
deterministic WARP runtime implementation for that medium, not the primary
runtime of Continuum and not an application framework.

## Cross-Repo Optic Admission Role

Echo owns runtime-local optic admission behavior. Wesley compiles artifacts and
registration descriptors; Echo registers them, returns runtime-local handles,
admits or obstructs invocations, instruments access, and emits witnesses or
readings. Authority layers issue grants and capability presentations.
Applications such as jedit hide artifact handles, basis references, and runtime
coordinates behind product-facing adapters.

Echo should not wait on a new Wesley product lane for the installed registry
boundary. Coordinate with Wesley only when artifact identity, generated helper
shape, or footprint compatibility changes.

## Pipeline

Evidence phase:

```text
canonical EINT
-> witnessed submission
-> admission gates
-> scheduler work candidate
-> law witness
-> admission ticket
```

Runtime phase:

```text
admission ticket
-> ticketed runtime ingress
-> scheduler-owned tick
-> tick receipt
-> observable intent outcome
```

The hinge is:

```text
AdmissionTicket + witnessed submission -> ticketed runtime ingress
```

## Roadmap Status

| Area                           | Status   | Notes                                                                                                                     |
| :----------------------------- | :------- | :------------------------------------------------------------------------------------------------------------------------ |
| WitnessedIntentSubmission      | Partial  | Runtime records witnessed submissions and restores local persistence images; host durable storage remains follow-up work. |
| SchedulerWorkCandidate         | Complete | The admission ladder can resolve the scheduler work candidate fixture.                                                    |
| LawWitness                     | Complete | The admission ladder can resolve the law witness fixture.                                                                 |
| AdmissionTicket                | Complete | Echo can issue `OpticAdmissionTicket` evidence without executing.                                                         |
| TicketedRuntimeIngress         | Complete | Ticketed ingress stages admitted submissions through runtime-owner authority without ticking.                             |
| ReceiptCorrelation             | Complete | Scheduler-owned tick receipts correlate back to ticketed ingress, tickets, and submissions.                               |
| IntentOutcomeObservation       | Complete | Core exposes read-only product outcome states with applied/rejected receipt evidence and typed obstructions.              |
| InstalledContractHostDispatch  | Complete | Installed packages can dispatch mutation handlers through witnessed, ticketed, scheduler-owned ticks.                     |
| ConflictPolicy / ExplicitRetry | Partial  | Tick-scale conflict rejection is final and blocker-attributed; user-facing retry helpers remain future.                   |
| QueryViewObserverBridge        | Complete | Core routes QueryView/Query to installed observers, and Wesley emits host helper constructors.                            |
| Replay/DIND proof              | Partial  | Local installed intent pipeline replay converges; broader DIND/replay closure remains future work.                        |

## Future Scope Boundaries

- Replica transport/import optics, settlement shells, adversarial transport,
  and idempotent import of already-adjudicated outcomes remain future work.
- Durable control-plane/provenance fault evidence remains future work; current
  scheduler fault quarantine is runtime-local posture.
- Ephemeral Scratch, Author-Only Speculative Lane, and Shared/Admitted Lane are
  paper-level privacy/runtime policy concepts. The local contract-host pipeline
  does not yet implement that full social lane model.

## Causal WAL Forty-Five Slice Plan

Track progress here. Check off slices just before committing the slice that
satisfies its acceptance criteria.

### PR 1: Doctrine And Grammar

- [x] **Slice 1: WAL doctrine hardening**
    - User story: As an Echo maintainer, I need the WAL doctrine to forbid
      semantic confusion before code exists.
    - Acceptance criteria: records are recorded, transactions are committed,
      submission acceptance stays distinct from runtime admission, read-only
      recovery cannot truncate, and explicit durable outboxes are allowed.
    - Test plan: `git diff --check`; stale-term grep for forbidden record names
      and intake/admission blur.

- [x] **Slice 2: Record grammar and naming spec**
    - User story: As an implementer, I need WAL record names that do not imply
      history before commit.
    - Acceptance criteria: ordinary record names use `*Recorded` or equivalent;
      no causal record kind uses `*Committed`; `WalTransactionCommit` is the
      only commit boundary.
    - Test plan: unit/schema-linter tests reject misleading record names.

- [x] **Slice 3: Transaction shape and affected frontiers**
    - User story: As a recovery implementer, I need commit markers to bind the
      exact frontiers affected by each transaction.
    - Acceptance criteria: first-cut transactions are contiguous, non-interleaved,
      and bind `affected_frontiers` or `affected_frontiers_root`.
    - Test plan: transaction fixtures prove intake, tick, posture, and checkpoint
      transactions bind distinct frontier kinds.

- [x] **Slice 4: `WalStorePort` contract**
    - User story: As a storage adapter author, I need a port contract that makes
      strict durability impossible to fake.
    - Acceptance criteria: port shape covers writer epoch acquisition, append,
      flush commit, segment reads, sealing, truncation, manifest publication, and
      epoch close; object-store conditional manifest rules are named.
    - Test plan: compile-contract tests for the trait once implemented; docs grep
      for all required operations.

- [x] **Slice 5: Release-gate witness list**
    - User story: As a reviewer, I need named crash and recovery witnesses before
      implementation starts.
    - Acceptance criteria: BEARING and WAL design name crash-before-ACK,
      read-only recovery, checkpoint validation, materialization replay,
      overlapping writer epoch, strict object-store, and record-name witnesses.
    - Test plan: doc assertions or grep checks for every witness name.

### PR 2: WAL Core In Memory

- [x] **Slice 6: Core WAL identifiers and record kinds**
    - User story: As a runtime developer, I need typed WAL identifiers and record
      kinds before writing frames.
    - Acceptance criteria: `Lsn`, `WalTransactionId`, `WriterEpochId`,
      `WalRecordKind`, and durable mode/posture types exist without app nouns.
    - Test plan: unit tests prove record-kind vocabulary and app-noun guard.

- [x] **Slice 7: Frame metadata and digest domains**
    - User story: As a recovery implementer, I need frames that separate torn
      writes from semantic commitment.
    - Acceptance criteria: frame headers bind LSN, transaction id, local index,
      record kind, payload digest, previous frame digest, codec/schema identity,
      and redaction posture.
    - Test plan: frame metadata round-trip and digest-domain tests.

- [x] **Slice 8: Record payload and transaction builder**
    - User story: As Echo, I need a transaction builder that records payloads
      before committing them.
    - Acceptance criteria: contiguous transaction builder appends records,
      computes payload roots, and refuses append-after-commit.
    - Test plan: transaction builder tests for order, record count, and closed
      transaction behavior.

- [x] **Slice 9: Commit marker validation**
    - User story: As recovery, I need commit markers that can be checked before
      replay.
    - Acceptance criteria: commit marker validates first/last LSN, record count,
      records root, affected frontier root, previous commit digest, and durability
      mode.
    - Test plan: fixtures fail on mismatched LSN range, count, roots, and chain.

- [x] **Slice 10: In-memory `WalStorePort`**
    - User story: As a test writer, I need deterministic WAL behavior without
      filesystem complexity.
    - Acceptance criteria: in-memory store appends frames, flushes commits,
      exposes segment streams, simulates torn tails, and never claims filesystem
      strictness.
    - Test plan: port tests for append/read/flush and synthetic crash tails.

### PR 3: Recovery Foundation

- [x] **Slice 11: Writer epoch fencing model**
    - User story: As Echo, I need stored evidence proving which writer had append
      authority.
    - Acceptance criteria: writer epochs bind fencing token, process identity,
      host identity, previous epoch id, previous final commit digest, and lease or
      lock evidence.
    - Test plan: overlapping epoch recovery fixture blocks recovery.

- [x] **Slice 12: Recovery scanner and tail posture**
    - User story: As Echo, I need recovery to group committed transactions and
      classify incomplete tails by mode.
    - Acceptance criteria: writable recovery may truncate validated incomplete
      tails; read-only recovery reports would-truncate without mutating storage.
    - Test plan: `read_only_recovery_reports_uncommitted_tail_without_truncating`
      and torn-tail tests.

- [x] **Slice 13: Pure replay reducer skeleton**
    - User story: As Echo, I need replay to apply facts, not rerun scheduler or
      application callbacks.
    - Acceptance criteria: `apply_committed_transaction(before, tx) -> after`
      exists as a pure reducer boundary with no wall clock, random, network,
      scheduler, app callback, or external I/O dependency.
    - Test plan: deterministic replay tests compare repeated reduction over the
      same committed transactions.

- [x] **Slice 14: Semantic transaction validators**
    - User story: As recovery, I need semantic checks after bytes and digests pass.
    - Acceptance criteria: validators check record kind authority, affected
      frontiers, retained-material refs, submission identity, and tick authority
      posture before replay.
    - Test plan: invalid causal transaction fixtures block recovery even with
      valid frame checksums.

- [x] **Slice 15: WAL schema and authority linter**
    - User story: As a reviewer, I need mechanical protection against app nouns
      and authority leaks.
    - Acceptance criteria: linter rejects app/product nouns and authority-leak
      record names such as app tick, client runtime control, application receipt,
      or document state delta.
    - Test plan: passing generic fixture and failing forbidden-noun/authority
      fixtures.

### PR 4: Submission Durability

- [x] **Slice 16: Submission acceptance transaction**
    - User story: As an app, if Echo returns accepted evidence, I need that
      acceptance to survive restart.
    - Acceptance criteria: `submit_intent` writes `SubmissionAcceptedRecorded`
      before returning accepted evidence.
    - Test plan: `accepted_submission_is_not_returned_before_wal_commit`.

- [x] **Slice 17: Accepted pending recovery**
    - User story: As Echo, accepted-but-not-ticked submissions must recover as
      pending.
    - Acceptance criteria: pending inbox rebuilds from committed acceptance
      transactions without transport re-arrival.
    - Test plan: `crash_after_submission_commit_recovers_pending_submission`.

- [x] **Slice 18: Crash-before-ACK retry posture**
    - User story: As a client, retry after crash-before-ACK must be deterministic.
    - Acceptance criteria: same submission id plus same envelope returns stable
      duplicate posture after recovery.
    - Test plan:
      `crash_after_submission_commit_before_ack_retry_returns_duplicate_posture`.

- [x] **Slice 19: Submission idempotency rules**
    - User story: As Echo, I must not collapse intentional repeated intents.
    - Acceptance criteria: same id plus different envelope is protocol violation;
      new id plus same envelope is new unless explicit dedupe policy says
      otherwise.
    - Test plan: `same_payload_new_submission_id_is_not_duplicate_without_policy`.

- [x] **Slice 20: Submission recovery certificate posture**
    - User story: As a host, I need restart output that explains recovered
      submission counts and posture.
    - Acceptance criteria: recovery certificate reports pending, decided,
      rejected, obstructed, and faulted submission counts.
    - Test plan: recovery certificate fixtures for clean and obstructed intake.

### PR 5: Tick Transactions

- [x] **Slice 21: Tick transaction staging**
    - User story: As Echo, scheduler outputs must remain staged until WAL commit.
    - Acceptance criteria: tick receipts, state deltas, correlations, and retained
      refs are staged before publish.
    - Test plan: `crash_before_tick_commit_commits_no_receipt`.

- [x] **Slice 22: Tick commit publish boundary**
    - User story: As an observer, visible receipts must imply recoverable receipts.
    - Acceptance criteria: indexes publish only after tick transaction flush.
    - Test plan: `crash_after_tick_commit_recovers_receipt_and_state_delta`.

- [x] **Slice 23: Receipt correlation rebuild**
    - User story: As a debugger/app, receipt correlation must survive restart.
    - Acceptance criteria: receipt-by-submission, receipt-by-ticket, and
      ticket-by-submission indexes rebuild from committed WAL transactions.
    - Test plan: `committed_receipt_correlation_rebuilds_after_restart`.

- [x] **Slice 24: Tick rollback compatibility**
    - User story: As Echo, WAL must not weaken failure-atomic tick rollback.
    - Acceptance criteria: failed attempted ticks commit no partial tick
      transaction; scoped fault quarantine remains separate runtime posture.
    - Test plan: existing rollback/quarantine tests plus WAL no-partial fixtures.

- [x] **Slice 25: Lawful rejection persistence**
    - User story: As Echo, lawful conflict/rejection is history, not an internal
      fault.
    - Acceptance criteria: rejected candidates commit receipt evidence and do not
      quarantine heads.
    - Test plan: conflict receipt recovery fixture and `lawful_rejection_does_not_fault_head`.

### PR 6: Retention And Readings

- [x] **Slice 26: Retained material ordering**
    - User story: As recovery, committed material refs must point at durable
      material or typed obstruction.
    - Acceptance criteria: material is durable before committed WAL reference.
    - Test plan: `missing_retained_material_returns_typed_obstruction`.

- [x] **Slice 27: Reading ref recovery**
    - User story: As an observer, retained reading refs must survive restart.
    - Acceptance criteria: reading refs rebuild by semantic coordinate and
      retained material digest.
    - Test plan: retained QueryView reading lookup after recovery.

- [x] **Slice 28: Semantic identity versus CAS identity**
    - User story: As Echo, byte identity must not masquerade as query identity.
    - Acceptance criteria: semantic coordinate and CAS digest remain distinct in
      WAL records and rebuilt indexes.
    - Test plan: same payload with different query coordinate remains distinct.

- [x] **Slice 29: Security and redaction posture**
    - User story: As an operator, I need to distinguish missing evidence from
      policy-hidden evidence.
    - Acceptance criteria: present, redacted, encrypted-key-unavailable, missing,
      corrupt, and obstructed postures exist.
    - Test plan: recovery/inspection fixtures for each posture.

- [x] **Slice 30: Retention obstruction scope matrix**
    - User story: As recovery, missing material should fault only at the correct
      scope.
    - Acceptance criteria: payload, receipt, state-delta, runtime-control, and
      diagnostic material failures map to documented obstruction/fault scope.
    - Test plan: scoped missing-material fixture suite.

### PR 7: Checkpoints And Inspector

- [x] **Slice 31: Checkpoint writer**
    - User story: As Echo, checkpoints should accelerate replay without creating
      history.
    - Acceptance criteria: temp write, fsync, atomic rename, directory fsync, and
      checkpoint binding to WAL chain are implemented.
    - Test plan: checkpoint round-trip and corrupt checkpoint fallback tests.

- [x] **Slice 32: Checkpoint validation without publication record**
    - User story: As recovery, valid checkpoint files should remain usable after
      crash before publication evidence.
    - Acceptance criteria: validated checkpoint can be used without
      `CheckpointPublicationRecorded`; publication record remains audit/index
      evidence.
    - Test plan:
      `valid_checkpoint_without_checkpoint_published_record_can_be_used_after_validation`.

- [x] **Slice 33: Checkpoint publication obstruction**
    - User story: As recovery, publication evidence must not lie about missing or
      invalid checkpoint material.
    - Acceptance criteria: publication without checkpoint blocks or obstructs by
      documented scope.
    - Test plan:
      `checkpoint_published_without_checkpoint_blocks_or_obstructs_according_to_scope`.

- [x] **Slice 34: Recovery certificate**
    - User story: As jedit/operator/debugger, I need a precise restart report.
    - Acceptance criteria: certificate reports checkpoint, LSN range, replayed
      transactions, tail posture, obstruction count, and final roots.
    - Test plan: clean, tail-truncated, and obstructed recovery certificates.

- [x] **Slice 35: Read-only WAL inspector**
    - User story: As an operator, I need inspection without mutating storage.
    - Acceptance criteria: `echo wal doctor --json`, `inspect`, and read-only
      recovery report posture without truncation.
    - Test plan: inspector reports would-truncate and leaves files unchanged.

### PR 8: Filesystem And Object Stores

- [x] **Slice 36: Filesystem WAL adapter**
    - User story: As Echo, strict filesystem durability must use real fsync
      boundaries.
    - Acceptance criteria: segment creation, append, file fsync, rename, and
      directory fsync are explicit.
    - Test plan: filesystem crash fixtures for torn frame and missing segment.

- [x] **Slice 37: Object-store manifest adapter**
    - User story: As Echo, strict object-store durability must not pretend fsync
      exists.
    - Acceptance criteria: content-addressed object writes, version/ETag
      verification, conditional manifest commit, and read-after-write posture are
      required.
    - Test plan: `strict_object_store_requires_conditional_manifest_commit`.

- [x] **Slice 38: Segment repair and truncation protocol**
    - User story: As recovery, tail repair must be explicit and mode-sensitive.
    - Acceptance criteria: writable stores can truncate incomplete tails; read-only
      stores can only report would-truncate.
    - Test plan: segment-gap, duplicate-segment, and torn-tail matrix.

- [x] **Slice 39: Crash matrix harness**
    - User story: As Echo, lifecycle ambiguity must be testable at every boundary.
    - Acceptance criteria: harness injects crash points around submit, tick,
      checkpoint, material, and index publication.
    - Test plan: generated crash matrix suite.

- [x] **Slice 40: WAL shadow replay in CI**
    - User story: As a maintainer, every mutating integration path should prove
      live state equals replayed state.
    - Acceptance criteria: selected tests run live scenario, replay WAL into fresh
      runtime, and compare state/index/receipt/reading roots.
    - Test plan: CI slice for shadow replay.

### PR 9: Outbox, Tooling, And Jedit Gate

- [x] **Slice 41: Side-effect outbox core**
    - User story: As Echo, external effects must be authorized by committed
      history before escaping.
    - Acceptance criteria: durable outbox records effect id, expected artifact,
      materialization intent, and idempotency token.
    - Test plan: `external_effect_requires_committed_outbox_authorization`.

- [x] **Slice 42: Materialization observation replay detection**
    - User story: As recovery, existing external artifacts should be detected
      before retry.
    - Acceptance criteria: recovery verifies path, digest, and metadata before
      retrying or recording observation.
    - Test plan: `materialization_replay_detects_existing_artifact_before_retry`.

- [x] **Slice 43: Causal commit evidence projection**
    - User story: As [warp-ttd], I need Echo-projected commit evidence, not raw
      WAL access.
    - Acceptance criteria: Echo exposes `CausalCommitEvidence` and recovery
      posture through read-model facts; no raw segment path or recovery authority
      is required.
    - Test plan: accepted pending, decided applied, rejected, obstructed, faulted,
      and durability-unknown projection fixtures.

- [x] **Slice 44: `jedit` WAL recovery gate**
    - User story: As [jedit], after crash/restart I need submitted edits to
      recover as not accepted, accepted pending, applied, rejected, or obstructed.
    - Acceptance criteria: stable submission id, recovered posture, export from
      committed causal basis, no jedit nouns in Echo core.
    - Test plan: sibling `jedit` crash/restart fixture and retry-after-ACK-loss
      fixture.

- [x] **Slice 45: WAL release readiness audit**
    - User story: As an operator, I need a final audit before trusting the WAL as
      the jedit durability foundation.
    - Acceptance criteria: all WAL release gates pass, docs match code, app-noun
      guard is green, and deferred post-WAL scope is explicit.
    - Test plan: full WAL slice suite, `cargo xtask test-slice`, app-noun guard,
      DIND/replay where available, and cross-repo jedit witness.

### No-Count WAL Hardening Plan

The WAL hardening matrix is tracked in
[`docs/design/causal-wal-hardening-matrix.md`](design/causal-wal-hardening-matrix.md).
This planning slice does not consume one of the twenty hardening slices. Agents
must keep the checklist below and the plan document aligned before committing a
completed slice.

### PR 10: WAL Fixture And Corpus Hardening

- [x] **Slice 46: WAL hardening fixture surface**
    - User story: As an Echo maintainer, I need deterministic fixtures that can
      build valid WAL histories and damage them at exact byte/transaction
      boundaries.
    - Acceptance criteria: fixtures create filesystem WAL roots, append committed
      transactions, append uncommitted tails, truncate/corrupt segments, recover
      read-only/writable reports, and name every scenario in assertion failures.
    - Test plan: fixture recovery, uncommitted-tail, torn-tail, and read-only
      non-mutation witnesses.

- [x] **Slice 47: WAL recovery golden corpus**
    - User story: As Echo, I need a fixed corpus of minimal WAL shapes that
      proves recovery posture across clean, partial, and corrupt histories.
    - Acceptance criteria: clean, empty, tail, torn, corrupt digest, bad magic,
      and unknown-kind corpus cases assert transaction count, LSNs, posture, and
      blocking/inspectable status.
    - Test plan: `wal_recovery_golden_*` fixtures listed in the hardening matrix.

- [x] **Slice 48: Submission ACK crash matrix**
    - User story: As an app retrying after a crash, I need Echo to distinguish
      never accepted from accepted-before-ACK.
    - Acceptance criteria: recovery never invents accepted evidence before
      commit; retry after commit-before-ACK is stable; conflicting retry is a
      protocol violation; same envelope with new id is not deduped without
      policy.
    - Test plan: submission intake crash matrix fixtures.

- [x] **Slice 49: Tick commit and publish crash matrix**
    - User story: As Echo, visible tick outcomes must imply committed
      recoverable history.
    - Acceptance criteria: tick receipt, runtime delta, receipt correlation, and
      index publication boundaries are tested; commit-before-publish rebuilds
      indexes; uncommitted tick attempts do not become history.
    - Test plan: tick commit/publish crash fixtures.

- [x] **Slice 50: Segment corruption matrix**
    - User story: As recovery, segment corruption must produce deterministic
      posture instead of panic, silent skip, or partial history.
    - Acceptance criteria: torn header, torn payload, torn digest, bad magic,
      corrupt digest, unknown disk kind, segment gap, and duplicate segment cases
      are covered; read-only recovery never mutates files.
    - Test plan: segment corruption matrix fixtures.

### PR 11: WAL Semantic And Checkpoint Hardening

- [x] **Slice 51: Writer epoch fencing matrix**
    - User story: As Echo, recovery must detect split-writer evidence instead of
      merging conflicting histories.
    - Acceptance criteria: epoch metadata includes fencing evidence; overlapping
      epochs, unknown previous epochs, chain gaps, and fencing mismatches fault
      deterministically.
    - Test plan: writer epoch fencing fixtures.

- [x] **Slice 52: Transaction contiguity and commit semantics**
    - User story: As Echo, only complete contiguous committed WAL transactions
      become history.
    - Acceptance criteria: frames are contiguous; commit binds LSN range, count,
      and records root; interleaving is rejected; record names never imply
      committed history.
    - Test plan: transaction contiguity and commit mismatch tests.

- [x] **Slice 53: Semantic validator negative cases**
    - User story: As Echo, byte-valid WAL transactions must still be rejected if
      they violate runtime law.
    - Acceptance criteria: digest-valid semantic violations are rejected for
      authority, transaction kind, runtime-control, and frontier-transition
      mismatches.
    - Test plan: semantic validator negative fixtures.

- [x] **Slice 54: Checkpoint crash matrix**
    - User story: As recovery, checkpoints must accelerate replay without
      creating or erasing history.
    - Acceptance criteria: crash-before-rename, rename-before-publication,
      missing material, corrupt latest checkpoint, and checkpoint-ahead-of-WAL
      cases are covered.
    - Test plan: checkpoint crash matrix fixtures.

- [x] **Slice 55: Retained material before reference matrix**
    - User story: As Echo, committed references must not point at unavailable
      retained material without typed obstruction.
    - Acceptance criteria: missing payload, receipt, state-delta, reading,
      checkpoint, and diagnostic material map to documented scopes.
    - Test plan: retained material obstruction fixtures.

### PR 12: WAL Outbox, Projection, And Inspector Hardening

- [x] **Slice 56: Side-effect outbox crash matrix**
    - User story: As Echo, external effects must never escape before committed
      authorization and must be idempotent after crash.
    - Acceptance criteria: effect authorization, existing artifact detection,
      mismatch obstruction, observation commit, and idempotency token behavior
      are covered.
    - Test plan: side-effect outbox crash fixtures.

- [x] **Slice 57: Recovery reducer determinism**
    - User story: As Echo, replay must apply committed facts without scheduler
      callbacks, wall clock, random, network, or app code.
    - Acceptance criteria: pure replay is deterministic; transaction ordering is
      commit-chain order; frontier mismatch rejects; no app/scheduler callback is
      needed.
    - Test plan: pure replay determinism fixtures.

- [x] **Slice 58: Shadow replay harness**
    - User story: As a maintainer, every mutating WAL path should prove live
      state equals recovered state.
    - Acceptance criteria: helper runs live scenario, recovers from WAL, compares
      roots/indexes/receipts/readings, and reports first mismatch.
    - Test plan: submission, tick, retention, outbox, and mismatch shadow replay
      fixtures.

- [x] **Slice 59: Causal commit evidence projection matrix**
    - User story: As [warp-ttd] or an operator, I need commit evidence posture
      without raw WAL ownership.
    - Acceptance criteria: accepted pending, applied, rejected, obstructed, and
      absent evidence postures project transaction id, LSN, epoch, digest, and
      durability mode without raw segment authority.
    - Test plan: causal commit evidence projection fixtures.

- [x] **Slice 60: WAL doctor and inspector contract tests**
    - User story: As an operator, I need inspection commands/read models to
      report truth without mutating storage.
    - Acceptance criteria: doctor reports clean, would-truncate, obstructed,
      corrupt, and missing-material postures; report shape is stable; files are
      unchanged in read-only mode.
    - Test plan: WAL doctor and recovery certificate contract fixtures.

### PR 13: WAL Runner, Adapter, And Release Gate Hardening

- [x] **Slice 61: Crashpoint runner contract**
    - User story: As Echo, I need a future CLI/BATS crash runner contract that
      mirrors Rust fixture semantics before it shells out to real processes.
    - Acceptance criteria: canonical crashpoint names exist for submission, tick,
      checkpoint, material, and index boundaries; process-kill cuts remain marked
      future until implemented.
    - Test plan: crashpoint manifest fixtures.

- [x] **Slice 62: Filesystem strict sync evidence**
    - User story: As Echo, strict filesystem mode must make sync boundaries
      inspectable enough for tests to prove ACK ordering.
    - Acceptance criteria: commit flush, segment creation, manifest rename, and
      checkpoint rename sync evidence are covered; missing sync evidence blocks
      strict-mode claims.
    - Test plan: filesystem sync evidence fixtures.

- [x] **Slice 63: Object-store manifest negative matrix**
    - User story: As Echo, strict object-store mode must reject adapters that
      cannot prove conditional manifest semantics.
    - Acceptance criteria: every missing capability has a distinct validation
      error; read-after-write uncertainty blocks strict mode; manifest commit is
      compare-and-swap shaped.
    - Test plan: object-store capability negative fixtures.

- [x] **Slice 64: Security and redaction posture matrix**
    - User story: As Echo, recovery and inspection must distinguish missing
      material from policy-hidden or encrypted material.
    - Acceptance criteria: present, redacted, encrypted-key-unavailable, missing,
      corrupt, and obstructed postures remain distinct and never become silent
      success.
    - Test plan: security/redaction posture fixtures.

- [x] **Slice 65: WAL hardening release gate**
    - User story: As Echo, I need one gate that tells us whether the WAL is
      trustworthy enough for the next real-app persistence push.
    - Acceptance criteria: readiness check aggregates app-noun guard, shadow
      replay, crash matrix, doctor, outbox, semantic validator, filesystem,
      object-store, and projection coverage.
    - Test plan: WAL hardening gate fixtures plus app-noun and doc checks.

### PR 14: WAL Segment Layout And Placement Hardening

- [x] **Slice 66: Canonical segment namespace**
    - User story: As Echo, WAL segment files must live under a logical namespace
      that does not encode wall-clock semantics.
    - Acceptance criteria: canonical segment paths use `segments/` plus
      zero-padded logical segment id; recovery scans that namespace; creating
      that namespace syncs the WAL root directory in strict filesystem mode.
    - Test plan: canonical segment path, namespace sync, and recovery scan
      fixtures.

- [x] **Slice 67: Wall-clock placement policy guard**
    - User story: As Echo, storage adapters may organize bytes by time only when
      time is non-authoritative placement metadata.
    - Acceptance criteria: wall-clock placement cannot be authoritative; causal
      segment id placement remains allowed.
    - Test plan: segment placement policy fixtures.

- [x] **Slice 68: Legacy flat segment compatibility**
    - User story: As recovery, older flat-layout WAL roots should remain
      inspectable while the canonical namespace moves forward.
    - Acceptance criteria: a root without `segments/` can still be scanned;
      duplicate ids across legacy and canonical layouts are rejected.
    - Test plan: legacy flat scan and cross-layout duplicate fixtures.

- [x] **Slice 69: Canonical gap and rewrite behavior**
    - User story: As recovery, canonical segment gaps and writable tail rewrites
      must preserve the logical segment namespace.
    - Acceptance criteria: segment gaps under `segments/` block recovery;
      writable truncation rewrites retained records back under `segments/`.
    - Test plan: canonical segment gap and writable rewrite fixtures.

- [x] **Slice 70: Segment id rotation guard**
    - User story: As Echo, segment rotation must fail deterministically instead
      of overflowing logical segment identity.
    - Acceptance criteria: next segment id is monotonic and overflow returns a
      typed error.
    - Test plan: segment id overflow fixture.

### PR 15: WAL Segment Manifest And Layout Gate Hardening

- [x] **Slice 71: Segment manifest entry shape**
    - User story: As Echo, segment manifest entries must bind logical segment id,
      canonical relative path, digest, and LSN range.
    - Acceptance criteria: manifest entries derive from frames and never require
      wall-clock path semantics.
    - Test plan: segment manifest entry fixture.

- [x] **Slice 72: Segment layout release gate**
    - User story: As Echo, the release readiness audit must include segment
      layout policy explicitly.
    - Acceptance criteria: readiness reports `segment_layout_policy` as blocked
      until set; a fully green gate requires it.
    - Test plan: release readiness gate fixtures.

- [x] **Slice 73: Manifest-addressed placement doctrine**
    - User story: As a storage adapter author, date/path placement must be
      understood as byte placement, not causal truth.
    - Acceptance criteria: BEARING records that wall-clock paths are
      non-authoritative and segment id/digest/commit chain decide truth.
    - Test plan: docs lint plus segment placement policy fixtures.

- [x] **Slice 74: Canonical layout migration witness**
    - User story: As an operator, moving from flat first-cut layout to
      `segments/` must not erase recoverability.
    - Acceptance criteria: canonical layout is preferred; legacy flat roots
      remain readable when no canonical namespace exists.
    - Test plan: legacy flat compatibility fixture.

- [x] **Slice 75: Segment layout drift gate**
    - User story: As a maintainer, future changes must not accidentally make
      wall-clock directory structure part of recovery truth.
    - Acceptance criteria: hardening tests assert canonical relative paths do not
      include date partitions and wall-clock authoritative placement is rejected.
    - Test plan: segment manifest and placement-policy hardening fixtures.

### PR 16: WAL Segment Rotation Hardening

- [x] **Slice 76: Active segment id enforcement**
    - User story: As Echo, frames must be appended only to the active logical
      segment they claim.
    - Acceptance criteria: filesystem append rejects frames whose header segment
      id differs from the active segment id.
    - Test plan: inactive segment append rejection fixture.

- [x] **Slice 77: Canonical segment rotation**
    - User story: As Echo, segment rotation should seal the current segment and
      create the next canonical segment under `segments/`.
    - Acceptance criteria: rotation returns the prior segment seal, advances the
      active segment id, creates the next segment file, and records strict sync
      evidence for the new segment; rotation does not overwrite an existing
      next segment.
    - Test plan: rotation creation, duplicate-protection, and sync evidence
      fixtures.

- [x] **Slice 78: Rotation tail safety**
    - User story: As Echo, rotation must not seal a segment containing
      uncommitted frames or a torn tail.
    - Acceptance criteria: rotation rejects segments with uncommitted tails using
      typed store errors.
    - Test plan: uncommitted-tail rotation rejection fixture.

- [x] **Slice 79: Multi-segment recovery**
    - User story: As recovery, committed transactions split across rotated
      segments must replay as one logical WAL stream.
    - Acceptance criteria: recovery scans multiple canonical segments, preserves
      clean tail posture, and reports the last committed LSN.
    - Test plan: rotated multi-segment recovery fixture.

- [x] **Slice 80: Rotation authority guard**
    - User story: As Echo, only the active writer epoch may rotate WAL segments.
    - Acceptance criteria: epoch mismatch rejects rotation before any new segment
      is created.
    - Test plan: rotation epoch mismatch fixture.

### PR 17: WAL Manifest Validation Hardening

- [x] **Slice 81: Manifest read roundtrip**
    - User story: As Echo, published filesystem manifests must be readable as
      structured WAL evidence.
    - Acceptance criteria: manifest files decode back into `WalManifest` without
      relying on ad hoc string parsing.
    - Test plan: filesystem manifest roundtrip fixture.

- [x] **Slice 82: Manifest segment-count validation**
    - User story: As recovery, a published manifest must not lie about segment
      count.
    - Acceptance criteria: validation compares manifest segment count with
      scanned canonical/legacy segment files and rejects mismatches.
    - Test plan: manifest segment-count mismatch fixture.

- [x] **Slice 83: Manifest commit-anchor validation**
    - User story: As recovery, a published manifest must match the last
      committed LSN and commit digest recovered from segment contents.
    - Acceptance criteria: last-LSN and last-digest mismatches reject with typed
      store errors.
    - Test plan: manifest last-LSN and last-digest mismatch fixtures.

- [x] **Slice 84: Manifest tail safety**
    - User story: As recovery, a manifest cannot validate while segments contain
      an uncommitted tail.
    - Acceptance criteria: validation rejects uncommitted or torn tails before
      accepting the manifest summary.
    - Test plan: manifest uncommitted-tail rejection fixture.

- [x] **Slice 85: Manifest validation release gate**
    - User story: As a maintainer, release readiness must require manifest
      validation coverage.
    - Acceptance criteria: readiness reports `segment_manifest_validation` as a
      distinct blocked gate until enabled.
    - Test plan: release readiness manifest-validation gate fixture.

### PR 18: Runtime WAL ACK Integration

- [x] **Slice 86: Runtime WAL adapter port**
    - User story: As a trusted runtime host, I need a local WAL adapter at the
      app-facing ACK boundary without giving the application append authority.
    - Acceptance criteria: `TrustedRuntimeHost` can configure an in-memory
      runtime WAL adapter as read-only evidence; applications still only receive
      `TrustedRuntimeApp`.
    - Test plan: trusted-host loop test proving the configured WAL is visible
      only as host evidence after app submission.

- [x] **Slice 87: Submission acceptance transaction wiring**
    - User story: As a caller, returned accepted submission evidence must be
      backed by a committed submission-intake WAL transaction when using the
      WAL-backed ACK path.
    - Acceptance criteria: `submit_intent_with_runtime_wal_ack(...)` records
      `SubmissionAcceptedRecorded` plus acceptance evidence before returning the
      handle.
    - Test plan:
      `runtime_wal_ack_submit_commits_acceptance_before_returning_handle`.

- [x] **Slice 88: Duplicate submit ACK posture**
    - User story: As a retrying client, resubmitting the same accepted envelope
      must not spray duplicate WAL acceptance transactions.
    - Acceptance criteria: duplicate intake returns the original submission id
      and leaves the submission-acceptance transaction count unchanged when WAL
      evidence already exists; a duplicate from legacy non-WAL intake backfills
      exactly one acceptance transaction before returning.
    - Test plan:
      `runtime_wal_ack_duplicate_submit_does_not_append_second_acceptance` and
      `runtime_wal_ack_duplicate_without_prior_wal_backfills_acceptance`.

- [x] **Slice 89: Pre-ACK WAL failure rollback**
    - User story: As Echo, if the WAL cannot commit accepted-submission evidence,
      the in-memory intake mutation must not remain visible.
    - Acceptance criteria: WAL build failure restores the pre-submit runtime and
      returns a typed host WAL error.
    - Test plan: `runtime_wal_ack_failure_rolls_back_intake_mutation` and
      `runtime_wal_ack_path_requires_configured_runtime_wal`.

- [x] **Slice 90: Tick receipt transaction wiring**
    - User story: As Echo, visible tick receipts should eventually be backed by
      committed scheduler-tick WAL transactions.
    - Acceptance criteria: host-owned scheduler runs record receipt and
      correlation facts before publishing product-facing receipt evidence.
    - Test plan: trusted-host applied-intent fixture plus recovered receipt
      index witness.

- [x] **Slice 91: Tick commit-before-publish rollback guard**
    - User story: As Echo, a tick WAL failure must not leave a half-visible
      receipt/outcome.
    - Acceptance criteria: tick WAL failure either restores runtime/provenance
      state or blocks receipt publication under a typed runtime fault posture.
    - Test plan: injected tick-WAL failure fixture.

- [x] **Slice 92: Runtime index rebuild contract**
    - User story: As recovery, WAL-backed submission and receipt indexes should
      rebuild without scheduler callbacks.
    - Acceptance criteria: recovered indexes answer pending/applied/rejected
      posture from committed WAL transactions only.
    - Test plan: pure in-memory recovery fixture for submit plus tick records.

- [x] **Slice 93: WAL-backed recovery certificate in runtime**
    - User story: As an operator, restart should produce inspectable evidence
      about what committed history was replayed.
    - Acceptance criteria: recovery certificate covers checkpoint, LSN range,
      commit digest, tail posture, and recovered counts.
    - Test plan: recovery certificate fixture over committed and truncated-tail
      WAL shapes.

- [x] **Slice 94: Echo recovery posture contract**
    - User story: As a real app consumer, sibling applications should be able
      to distinguish not-accepted, accepted-pending, decided, rejected, and
      obstructed work from Echo recovery evidence without Echo importing
      application nouns.
    - Acceptance criteria: Echo exposes only generic submission/receipt posture;
      sibling applications map that posture to product terms outside Echo.
    - Test plan: Echo CLI fixture emits generic recovery JSON; the sibling
      `jedit` fixture remains the next cross-repo consumer witness.

- [x] **Slice 95: Runtime ACK drift gate**
    - User story: As a maintainer, docs and tests should fail if Echo claims
      durable ACK semantics without a WAL-backed witness.
    - Acceptance criteria: release readiness names runtime ACK coverage as a
      distinct gate through `cargo xtask test-slice runtime-wal-ack`.
    - Test plan: runtime ACK readiness gate fixture plus stale-claim grep:
      `cargo xtask test-slice runtime-wal-ack`.

## Recently Completed Slice Batch

1. **Contract-Aware Receipts And Readings**

    Installed QueryView readings and installed mutation receipt correlations
    now carry contract package evidence: package id, schema hash, artifact hash,
    codec identity, operation/query id, and operation kind.

2. **Contract Reading Identity And Bounded Payloads**

    QueryView readings now carry `QueryReadingIdentity`, binding query id, vars
    digest, resolved basis digest, requested aperture digest, observer plan, and
    installed contract evidence when present.

3. **Contract Artifact Retention In `echo-cas`**

    `echo-cas` now has a local semantic retention index above content-only
    blobs for contract artifacts, receipts, witnesses, reading payloads,
    reading envelopes, and observer artifacts.

4. **Contract Retention And Semantic Lookup Seams**

    Semantic retention lookup now supports bounded byte ranges under caller
    budget while requiring exact semantic coordinate match.

5. **External Contract Proof Fixture**

    The installed contract pipeline now has a generic external-consumer-shaped
    proof covering mutation, QueryView reading, retained evidence, and replay
    without application nouns in Echo core.

6. **Versioned Contract And API Compatibility**

    Generated packages now verify Echo contract ABI, Wesley generator,
    contract-host helper API, codec, schema, registry layout, and footprint
    compatibility at package install. Receipts and readings can cite the
    verified compatibility metadata without treating it as execution authority.

7. **Reference Trusted Runtime Host Loop**

    `TrustedRuntimeHost` now owns generated package installation, ticketed
    ingress staging, scheduler passes, until-idle policy, and read-only
    observation service access. `TrustedRuntimeApp` can submit and observe
    without receiving tick, package-install, ingress-staging, or fault-recovery
    authority.

8. **Serious External Consumer Proof Fixture**

    The contract-host path now has a serious external-consumer fixture covering
    non-trivial mutation, overlapping write conflict, bounded QueryView
    reading, retained reading payload, and retained receipt evidence while
    keeping application nouns in test fixture code.

9. **Local Replay/DIND Proof For Contract Path**

    `cargo xtask test-slice contract-path-release` now runs the narrow local
    v0.1 contract-host release witness: installed contract pipeline replay,
    reference trusted host loop, and the serious external consumer fixture.

10. **Release-Grade Quickstart And Authority Audit**

    `docs/quickstart-local-contract-host.md` now documents the executable local
    contract-host path. `docs/design/v0.1.0-authority-boundary-audit.md` records
    the app/host authority split, current evidence, and deferred release risks.

## Release Gate Shift

`v0.1.0` is officially delayed until the jedit/Echo proof passes outside the
Echo repository.

The required shape is:

```text
sibling application-owned contract and adapters
-> Wesley generated Echo runtime artifacts
-> Echo installs a generic generated package
-> application submits canonical intent
-> trusted Echo host stages work and authorizes scheduler opportunities
-> application observes applied, rejected, or obstructed outcome
-> application queries a bounded reading
-> retained evidence and replay prove the same result
```

The proof must preserve the authority split:

- application code does not tick;
- product capabilities remain application-owned nouns and must not appear in
  Echo core or Echo package boundary APIs;
- application-facing code uses product capabilities rather than Echo runtime
  coordinates;
- application code does not send scheduler control through
  `dispatch_intent`;
- trusted host code owns package install, ticketed ingress staging, scheduler
  passes, until-idle policy, and fault recovery;
- Echo core does not import application product nouns or product concepts;
- the existing fake transport may stay as a local harness, but the release
  witness must run against the real Echo boundary.

The current opt-in jedit real-WASM stack witness now uses a separate trusted
host-control transport. Agents can run jedit's JSON-capable CLI witness:

```sh
ECHO_WARP_WASM_DIR=/path/to/echo/crates/warp-wasm \
  node scripts/jedit-echo-witness.mjs --json
```

The next integration slice should improve witness content: retained evidence,
replay, and a jedit-owned generated contract path, not scheduler authority.

## Next Candidate Slices

1. **Contract Obstruction Taxonomy**

    Stabilize contract-hosted obstruction names for unsupported operations,
    unsupported queries, admission obstructions, runtime faults,
    missing-retention posture, stale basis, residual readings, and budget
    limits. Product-facing APIs should consume typed obstruction posture instead
    of broad strings or catch-all runtime errors.

2. **Retained Evidence Refs And Missing-Retention Posture**

    Lift the local semantic retention index into typed retained evidence refs
    that receipt, reading, witness, and artifact surfaces can cite. Missing
    retained material should return explicit obstruction/posture, not empty
    success or content-hash guesswork.

3. **Durable Witnessed Submission Persistence**

    Accepted-but-not-yet-ticked submissions should survive restart without
    becoming half-accepted, uncorrelatable history.

4. **Product-Facing Intent Outcome API**

    Wrap the current core outcome observation into a developer-facing local API
    that preserves the authority boundary and does not tick synchronously.

5. **Release Candidate Cleanup**

    Polish product-facing adapters, finish any remaining release-card checkboxes,
    and run the broader CI/DIND release gates before cutting a release
    candidate.

Direct `native_rule_bootstrap` registration remains an internal fixture and
transitional engine-test path. Contract-host proofs that need package identity,
registry verification, or generated operation/package binding guarantees should
install through the package boundary.

These slices must not implement hidden retry, execution outside
scheduler-owned ticks, wall-clock cadence semantics, app-controlled tick
authority, or application-domain APIs inside Echo core.

## Do Not Regress

Implementation improvements over the paper examples that must be preserved:

- application optics do not create ticks;
- application dispatch does not execute synchronously;
- application dispatch does not command ticks;
- `AdmissionTicket` is distinct from `TickReceipt`;
- `AdmissionTicket` is not execution;
- `LawWitness` precedes and is bound by `AdmissionTicket`;
- query observers are read-only;
- `QueryView` bridge and Wesley query observer helpers exist;
- fault quarantine is runtime-local unless durable evidence is explicitly
  added;
- conflict rejection is final for that tick attempt, and retry is a new causal
  act.
