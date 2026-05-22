<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# BEARING

Last updated: 2026-05-22.

This signpost summarizes current direction. It does not create commitments or
replace backlog items, design docs, retros, or CLI status. If it disagrees with
code, the code wins and this file should be corrected.

The WARP paper-to-Echo noun map is maintained in
`docs/design/warp-optic-implementation-map.md`.

The feature bar for the eventual `v0.1.0` release is maintained in
`docs/design/v0.1.0-release-plan.md`.

The filesystem lane for release-bar backlog cards is
`docs/method/backlog/v0.1.0/`.

## Current Bearing

Echo has a local witnessed intent pipeline into deterministic execution:
application ingress can become witnessed submission history, lawful admission
evidence, ticketed runtime ingress, scheduler-owned handler dispatch, receipt
correlation, and observable intent outcome.

The current priority is to make that pipeline consumer-grade for
Wesley-compiled contract packages: contract-aware receipts, honest reading
identity, bounded retained readings, and external consumer proof fixtures
without moving application nouns into Echo core.

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
- The semantic retention layer is local and in-memory; durable retained
  artifact, witness, receipt, and reading recovery remains future work.
- Generic external contract proof exists, but a serious application-owned
  consumer proof, especially `jedit`, still needs to prove the host path with
  generated artifacts outside Echo core.

## Doctrine

Echo accepts intent submissions as witnessed ingress history.

Application-authored optics do not create ticks.

Application-authored surfaces may declare runtime-retained consequence
obligations, including receipt obligations. Echo satisfies those obligations
only through trusted runtime-owned execution.

Echo does not execute submissions synchronously.

Echo's trusted runtime owner controls tick boundaries.

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

5. **Versioned Contract And API Compatibility**

    Enforce the compatibility surface that makes generated packages fit a
    specific Echo runtime: ABI version, Wesley generator version, contract
    package version, schema hash, artifact hash, codec id, and generated helper
    compatibility.

6. **Reference Trusted Runtime Host Loop**

    Provide a boring host-owned loop that owns tick cadence, runs until idle, and
    exposes app-safe submit/observe/query surfaces.

7. **Serious External Consumer Proof Fixture**

    Replace the generic fixture as the only consumer proof with a serious
    application-owned generated contract shape, preferably `jedit`-shaped, while
    keeping text/editor nouns out of Echo core.

8. **Local Replay/DIND Proof For Contract Path**

    Turn the local replay fixture into a release-gate proof over package,
    submissions, scheduler policy, receipts, readings, and retained evidence.

9. **Release-Grade Quickstart**

    Make the first clean-checkout contract-host flow executable end to end with
    documented commands.

10. **Authority Boundary Audit**

    Prove the app-facing surfaces cannot tick, step, access trusted runtime
    control, resume faulted heads, install privileged host adapters, mutate
    through query observers, bypass package compatibility checks, or turn retry
    into hidden runtime behavior.

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
