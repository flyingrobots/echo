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

## Host Boundary

Trusted hosts may install verified generated packages, stage ticketed ingress,
run scheduler passes, configure until-idle policy, and recover faults.
Applications receive submission and observation capabilities without those
controls. Product nouns and product policy remain in application contracts and
adapters.

## Evidence Anchors

- [Registry/provider/host boundary](../adr/0004-registry-provider-host-boundary.md)
- `docs/architecture/application-contract-hosting.md`
- `crates/warp-core/src/trusted_runtime_host.rs`
- `crates/warp-core/src/engine_impl.rs`
