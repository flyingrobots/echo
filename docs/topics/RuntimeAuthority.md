<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Runtime Authority

Echo separates application proposals, lawful admission, scheduler execution,
observation, and trusted runtime control. Evidence may connect those phases;
authority does not leak across them.

## Submission and Execution

- Application dispatch submits canonical intent bytes. It does not execute a
  handler, create a tick, or command a tick.
- An admission ticket witnesses lawful eligibility. It is not execution.
- Trusted runtime control stages admitted work and owns scheduler opportunities.
- A tick receipt witnesses the scheduler-owned decision.
- A lawful rejection remains witnessed history and is not an internal fault.
- Retry is a new explicit causal act.

## Control and Faults

Start, stop, cadence, drain, and recovery are trusted runtime-control history.
They authorize or suspend opportunities; they are not application intents.

Rollback is tick-local cleanup of an uncommitted failed attempt. Quarantine is
runtime-local posture after an internal fault. Scoped faults isolate the
culprit; an unscoped fault may quarantine the runtime. Neither posture should
be confused with lawful obstruction or rejection.

## Observation

Queries and `QueryView` resolve an explicit causal basis and invoke registered
read-only observers. They do not tick the runtime or invoke mutation handlers.
Reading envelopes identify their basis, aperture, observer plan, budget, and
evidence posture.

## External Actions

- Edict may construct a typed request value; it receives no authority to
  perform the operation.
- Echo commits `REQUESTED` before it can expose claimable work.
- Runtime-owner adapter registration attenuates operation and scope policy into
  one request-specific authorization.
- Echo commits `CLAIMED` before it can expose an adapter work grant.
- Only an operation-specific adapter possesses filesystem, process, network,
  timer, model, or other world-touching authority.
- Adapter output is untrusted settlement input until Echo validates its exact
  request, attempt, adapter, basis, schema, digest, evidence, and budget
  bindings.
- Raw WAL callers cannot mint `ExternalActionCoordinator` authority. The
  coordinator owns an opaque commit capability and derives causal transaction
  coordinates from one checked local continuation.
- Echo commits `SETTLED` before deterministic program resumption.
- Recovery of a claimed request requires reconciliation. It does not authorize
  blind re-execution.
- Arbitrary recovery reports expose observation-only lifecycle values. Trusted
  local coordinator recovery reconstructs interrupted transition grants.
- Replay consumes the admitted settlement bytes and never invokes the adapter.
- `OutcomeUnknown` is a first-class terminal observation, not an alias for
  failure.
- The bounded workspace-observation profile independently admits exact Edict
  Core and Echo-profile Target IR, binds target profile plus portable operation
  into the runtime operation identity, then exposes only an explicit
  relative-path set through one capability-rooted, no-follow adapter. Compiler
  admission and request recording perform no filesystem read.

## Host Boundary

Trusted hosts may install verified generated packages, stage ticketed ingress,
run scheduler passes, configure until-idle policy, and recover faults.
Applications receive submission and observation capabilities without those
controls. Product nouns and product policy remain in application contracts and
adapters.

The unreleased `echo_runtime::RuntimeHost` is the first sealed Rust facade over
that role. It can construct and recover a local host and WAL shell without
exposing the underlying engine, `TrustedRuntimeHost`, native rule registration,
or constructors for receipts, authority epochs, and committed outcomes. It does
not yet expose installation, submission, scheduling, result, or receipt APIs;
those remain required before an external host can prove the complete release
lifecycle. The crate remains `publish = false`, so this boundary is executable
release-engineering evidence rather than a published API promise.

The accepted alpha boundary keeps admission, epoch authority, Tick
construction, settlement, receipt construction, commit publication, and
recovery acceptance in private modules of the same package as the sealed
facade. A separate published implementation crate would require `pub`
inter-package APIs that every downstream caller could invoke. Opaque admitted
types must also refuse indirect construction through deserialization, defaults,
raw conversions, public variants, unchecked builders, or feature-gated test
seams.

Verification and admission remain distinct. A verifier makes evidence about an
executable subject; the runtime correlates that evidence and decides whether to
admit and install the subject. Public API names must not claim that the runtime
performed verification unless it actually invoked the verifier.

Every feature declared by a published runtime package must remain
authority-safe in every selectable combination. Release metadata may describe
tested configurations, but it is not an access-control boundary. The complete
accepted target and current implementation posture are defined by
[Public Rust release boundary](../architecture/public-rust-release-boundary.md)
and the
[public runtime authority invariant](../invariants/PUBLIC-RUNTIME-AUTHORITY.md).

## Evidence Anchors

- [Registry/provider/host boundary](../adr/0015-registry-provider-host-boundary.md)
- [Durable external-action settlement](../adr/0026-durable-external-action-settlement.md)
- [External actions](ExternalActions.md)
- `docs/architecture/application-contract-hosting.md`
- `docs/architecture/public-rust-release-boundary.md`
- `docs/invariants/PUBLIC-RUNTIME-AUTHORITY.md`
- `crates/warp-core/src/trusted_runtime_host.rs`
- `crates/warp-core/src/engine_impl.rs`
- `crates/echo-runtime/src/lib.rs`
