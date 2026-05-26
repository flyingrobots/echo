<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# BEARING

Last updated: 2026-05-26.

This signpost summarizes current direction. It does not create commitments or
replace backlog items, design docs, retros, or CLI status. If it disagrees with
code, the code wins and this file should be corrected.

The WARP paper-to-Echo noun map is maintained in
`docs/design/warp-optic-implementation-map.md`.

The feature bar for the eventual `v0.1.0` release is maintained in
`docs/design/v0.1.0-release-plan.md`.

The current external release gate is maintained in
`docs/design/v0.1.0-jedit-release-gate.md`.

Trusted runtime-control history is defined in
`docs/design/trusted-runtime-control-history.md`.

The causal WAL doctrine and recovery design is defined in
`docs/design/causal-wal-end-to-end.md`.

The WAL/WSC storage relationship is tracked in
`docs/method/backlog/v0.1.0/PLATFORM_wal-wsc-storage-relationship.md`.

The active forty-slice jedit/Echo recovery gate is planned in
`docs/design/echo-authoritative-jedit-recovery-40-slice-plan.md`.

The current sequencing filter for audited work items is maintained in
`docs/design/work-item-sequencing-and-prioritization.md`.

The filesystem lane for release-bar backlog cards is
`docs/method/backlog/v0.1.0/`.

The production-core app-noun guard is `scripts/check-no-app-nouns-in-core.sh`.
It checks that hardcoded jedit/Stack Witness fixture shortcuts stay out of
Echo crate source. The guard is intentionally production-source-scoped: tests
and docs may still carry app-shaped fixtures as external-consumer examples,
but production Echo code must remain generic.

## Current Bearing

Echo has a local witnessed intent pipeline into deterministic execution:
application ingress can become witnessed submission history, lawful admission
evidence, ticketed runtime ingress, scheduler-owned handler dispatch, receipt
correlation, and observable intent outcome.

The current priority is to prove that pipeline with `jedit` as a real external
consumer. The in-repo external fixture remains valuable, but it is no longer
the `v0.1.0` release gate. Echo is not ready to release until jedit can submit
an application-owned contract intent, let a trusted Echo host authorize
scheduler opportunities, observe the outcome, query a bounded reading, recover
after process death from Echo-owned WAL truth, and materialize from recovered
causal basis without moving application nouns into Echo core.

The immediate durability hill is no longer "prove that a WAL exists." Echo now
has a serious WAL substrate and runtime ACK witness. The hill is proving that a
sibling application can use that substrate as the authority after the
application forgets local state, retries, crashes, or attempts to fall back to a
legacy model.

Runtime ACK drift gate: `cargo xtask test-slice runtime-wal-ack`.

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

- WAL-backed accepted-submission evidence exists in Echo runtime ACK fixtures.
  What is not yet true is the sibling-application proof that jedit production
  edits recover from that evidence across process death.
- Product-facing clients do not yet have polished ABI/helper surfaces for
  per-intent applied/rejected semantics.
- Contract-aware obstruction taxonomy and product-facing error surfaces still
  need release-grade stabilization.
- The semantic retention layer has local proof surfaces. App-safe readings can
  report generic missing-retention posture, but WSC-backed portable retained
  artifact, witness, receipt, and reading recovery remains future work.
- Generic external contract proof exists, but the release gate now requires
  real `jedit` follow-through from the sibling repository: crash windows,
  duplicate retry, receipt-to-reading correlation, legacy-memory tripwires,
  and materialization from recovered Echo causal basis.

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

| Area                           | Status   | Notes                                                                                                        |
| :----------------------------- | :------- | :----------------------------------------------------------------------------------------------------------- |
| WitnessedIntentSubmission      | Partial  | WAL-backed runtime ACK evidence exists; sibling-app production recovery remains the active proof gap.        |
| SchedulerWorkCandidate         | Complete | The admission ladder can resolve the scheduler work candidate fixture.                                       |
| LawWitness                     | Complete | The admission ladder can resolve the law witness fixture.                                                    |
| AdmissionTicket                | Complete | Echo can issue `OpticAdmissionTicket` evidence without executing.                                            |
| TicketedRuntimeIngress         | Complete | Ticketed ingress stages admitted submissions through runtime-owner authority without ticking.                |
| ReceiptCorrelation             | Complete | Scheduler-owned tick receipts correlate back to ticketed ingress, tickets, and submissions.                  |
| IntentOutcomeObservation       | Complete | Core exposes read-only product outcome states with applied/rejected receipt evidence and typed obstructions. |
| InstalledContractHostDispatch  | Complete | Installed packages can dispatch mutation handlers through witnessed, ticketed, scheduler-owned ticks.        |
| ConflictPolicy / ExplicitRetry | Partial  | Tick-scale conflict rejection is final and blocker-attributed; user-facing retry helpers remain future.      |
| QueryViewObserverBridge        | Complete | Core routes QueryView/Query to installed observers, and Wesley emits host helper constructors.               |
| Replay/DIND proof              | Partial  | Local installed intent pipeline replay converges; broader DIND/replay closure remains future work.           |

## Future Scope Boundaries

- Replica transport/import optics, settlement shells, adversarial transport,
  and idempotent import of already-adjudicated outcomes remain future work.
- Durable control-plane/provenance fault evidence remains future work; current
  scheduler fault quarantine is runtime-local posture.
- Ephemeral Scratch, Author-Only Speculative Lane, and Shared/Admitted Lane are
  paper-level privacy/runtime policy concepts. The local contract-host pipeline
  does not yet implement that full social lane model.

## Completed Work Archive

Completed slice plans are archived outside the active signpost:

- [`bearing-completed-items-archive.md`](design/bearing-completed-items-archive.md)
  records the completed WAL slices 1-95 and the completed retained-evidence
  release-gate batch that previously lived inline here.
- [`causal-wal-hardening-matrix.md`](design/causal-wal-hardening-matrix.md)
  keeps the detailed WAL hardening matrix for historical inspection.
- [`v0.1.0-jedit-next-ten-slices.md`](design/v0.1.0-jedit-next-ten-slices.md)
  keeps the completed retained-evidence release-gate batch detail.

## Current Goalpost

The current goalpost is the \[Echo]\[jedit] external-app causal durability gate:

a production jedit edit submitted through the app-owned port is durably accepted
by Echo, safely retryable without duplicate application, decided only by Echo's
trusted scheduler, observed only through Echo bounded reading evidence,
recoverable after restart from Echo WAL history, mapped by jedit into
editor-facing status without Echo learning editor nouns, and materializable from
the recovered causal basis without reading legacy memory.

The full forty-slice PRD and test plan lives in
[`echo-authoritative-jedit-recovery-40-slice-plan.md`](design/echo-authoritative-jedit-recovery-40-slice-plan.md).

## Active Gated Recovery Campaign

Track progress here. Check off slices just before committing the slice that
satisfies its acceptance criteria. The detailed PRD and test plan for each slice
is authoritative in
[`echo-authoritative-jedit-recovery-40-slice-plan.md`](design/echo-authoritative-jedit-recovery-40-slice-plan.md).

This is a gated campaign, not forty equal-status tasks. Slice 0 is a no-count
doctrine slice. Numeric slice ids stay stable for references, but execution
follows the gate order below.

### Gate 0: Invariant Charter

- [x] Slice 0: \[Echo]\[jedit] authority invariant charter and posture state
      machine.

### Gate A: Echo Recovery Truth

- [x] Slice 1: \[Echo] WAL reality and stale claim cleanup.
- [x] Slice 2: \[Echo] accepted-submission evidence contract.
- [x] Slice 3: \[Echo] generic recovery posture taxonomy.
- [x] Slice 4: \[Echo] duplicate submission and idempotency law.
- [x] Slice 5: \[Echo] crashpoint fixture contract for external apps.
- [x] Slice 36: \[Echo] no-editor-nouns trusted-runtime guard.
- [x] Slice 6: \[Echo] filesystem accepted-submission restart fixture.
- [x] Slice 7: \[Echo] crash-before-ACK retry witness.
- [x] Slice 8: \[Echo] conflicting duplicate rejection witness.
- [x] Slice 9: \[Echo] app-safe submission posture read surface.
- [x] Slice 10: \[Echo] recovery certificate submission posture counts.
- [ ] Slice 11: \[Echo] receipt correlation recovery witness.
- [ ] Slice 12: \[Echo] tick commit-before-publish failure injection.
- [ ] Slice 13: \[Echo] bounded reading identity recovery witness.
- [ ] Slice 14: \[Echo] retained material recovery obstruction witness.
- [ ] Slice 15: \[Echo] receipt-to-reading causal chain read model.
- [ ] Slice 16: \[Echo] causal commit evidence JSON contract.
- [ ] Slice 17: \[Echo] generic external-app recovery gate command.

### Gate B: jedit Consumes Echo Truth

- [ ] Slice 18: \[jedit] Echo recovery port interface.
- [ ] Slice 19: \[jedit] Echo recovery adapter implementation.
- [ ] Slice 20: \[jedit] generic-to-editor posture mapping.
- [ ] Slice 21: \[jedit] stable edit submission identity.
- [ ] Slice 22: \[jedit] recovery evidence report fields.
- [ ] Slice 32: \[jedit] production legacy memory static guard.
- [ ] Slice 33: \[jedit] release-gate runtime tripwire mode.
- [ ] Slice 23: \[jedit] recovered bounded reading path.
- [ ] Slice 24: \[jedit] happy-path recovery gate scenario.
- [ ] Slice 25: \[jedit] retry after local amnesia scenario.
- [ ] Slice 34: \[jedit] materialize artifact from recovered causal basis.

### Gate C: Crash-Window Authority

- [ ] Slice 26: \[Echo]\[jedit] crash runner harness.
- [ ] Slice 27: \[Echo]\[jedit] before-accept-response crash window.
- [ ] Slice 28: \[Echo]\[jedit] after-accept-before-tick crash window.
- [ ] Slice 29: \[Echo]\[jedit] after-receipt-before-reading crash window.
- [ ] Slice 30: \[Echo]\[jedit] after-reading-before-observe crash window.
- [ ] Slice 31: \[Echo]\[jedit] after-observe-before-local-status crash window.

### Gate D: Boundary, Report, And Drift Lock

- [ ] Slice 35: \[Echo]\[jedit] side-effect authorization boundary.
- [ ] Slice 37: \[Echo]\[jedit] malicious adapter negative suite.
- [ ] Slice 38: \[Echo]\[jedit] JSON causal durability report.
- [ ] Slice 39: \[Echo]\[jedit] human causal receipt card.
- [ ] Slice 40: \[Echo]\[jedit] aggregate gate, docs, and drift audit.

## Scope Discipline

This forty-slice goal does not pull in full Continuum transport, settlement
shells, full WSC archive/import/export, object-store strict durability, the full
observer-rights revelation lattice, Graft structural intelligence, Think
speculative lanes, or complete editor product polish.

The proof is intentionally narrower: jedit forgets, Echo remembers, and jedit
accepts Echo's recovered answer through app-owned ports without Echo importing
editor concepts.

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
