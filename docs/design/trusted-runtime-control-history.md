<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Trusted Runtime Control History

Status: design baseline for the jedit `v0.1.0` release gate.

This packet records the causal status of trusted runtime-control commands such
as `Start`, `Stop`, `SetCadence`, and `DrainUntilIdle`. It sharpens the
authority language used by the reference trusted runtime host loop and the WASM
trusted-control boundary.

## Core Claim

Trusted runtime-control commands are causal runtime-control history, not
application or domain history.

```text
Start/Stop authorize or suspend scheduler opportunities.
Start/Stop do not create ticks.
Start/Stop are not submitted through application intent dispatch.
TickReceipt remains scheduler-owned execution evidence.
```

Echo should record enough trusted control history to explain why ticks did or
did not happen, but it must not let application code command individual ticks.

## Authority Split

| Plane                     | May Do                                                                                                 | Must Not Do                                                                       |
| :------------------------ | :----------------------------------------------------------------------------------------------------- | :-------------------------------------------------------------------------------- |
| Application/domain intent | Submit canonical domain work and observe outcomes.                                                     | Start, stop, tick, drain until idle, set cadence, or recover faults.              |
| Trusted runtime control   | Start or stop scheduler opportunities, set cadence policy, drain under trusted policy, recover faults. | Mutate application state directly or pretend control commands are domain intents. |
| Scheduler execution       | Select eligible work, attempt ticks, emit receipts, roll back failed attempts.                         | Treat wall-clock callback timing as domain truth.                                 |

The product host may expose a Start/Stop button or command to an operator, but
that command targets the trusted runtime-control port. It is not an
application-authored contract intent and it is not part of the application
mutation vocabulary.

## Causal Record Shape

The implementation names may change, but the evidence shape should remain:

```text
RuntimeControlRecord {
  control_id,
  control_generation,
  authority,
  command,
  cadence_policy?,
  effective_boundary,
  command_digest
}

TickReceipt {
  tick_generation,
  control_generation,
  selected_work_digest,
  outcome_digest
}
```

This gives audit and replay a direct answer to:

- who enabled or suspended ticking;
- what cadence policy was in force;
- which logical control epoch a tick belongs to;
- which work the scheduler selected;
- what outcome was committed.

## Cadence Is Policy, Not Semantic Time

`Start { tick_frequency: 60Hz }` should mean:

```text
enable the trusted scheduler loop under a fixed-cadence host policy
```

It should not mean:

```text
execute tick N immediately
```

Wall-clock timestamps may be diagnostic evidence, but replay must not require
sleeping for the original wall-clock intervals. Semantic replay should replay
logical control epochs, accepted submissions, scheduler policy, selected work,
and receipts. Operational audit may report wall-clock observations separately.

## Replay Interpretation

Echo needs two replay attitudes:

1. **Semantic replay**

    Rebuild the same logical results from accepted submissions, runtime-control
    records, scheduler policy, retained contract material, and committed
    receipts. This replay runs as fast as the host permits.

2. **Operational audit replay**

    Explain runtime posture over time: stopped, running, cadence changed,
    faulted, drained, or idle. This explains pending gaps and delayed outcomes
    without making wall-clock cadence part of domain semantics.

In both modes, application dispatch remains asynchronous with respect to
execution. A submitted intent may remain pending until the trusted runtime host
has legal scheduler opportunities and the scheduler chooses it.

## Stop Semantics

`Stop` suspends future scheduler opportunities after a safe boundary. It must
not interrupt a half-committed tick. A tick attempt already in progress either:

- commits completely and emits its receipt; or
- rolls back under the failure-atomic scheduler transaction rules.

## Drain Semantics

`DrainUntilIdle` is a trusted host policy command used by tests, local hosts,
and controlled automation. It is not an app-safe convenience API. A high-level
application facade may request that its trusted host drain, but the actual
authority remains with the host/runtime owner.

## Hard Boundaries

- No `StartRuntime`, `StopRuntime`, `TickNow`, or `RunUntilIdle` application
  intents.
- No contract handler may call `super_tick`.
- No app-facing package facade may expose raw trusted-control exports.
- No tick receipt is emitted merely because `Start` was recorded.
- No wall-clock jitter may change semantic outcome without receipt evidence.
- No `Stop` may commit a half-tick.
- No recovery command may be accepted through app intent dispatch.

## Impact On The jedit Release Gate

jedit may expose product-shell Start/Stop/cadence controls for a local Echo
host. Those controls must flow through a trusted host adapter, not through
jedit application intents or generated contract mutations.

The release witness should eventually report:

- the runtime-control command used to start the host;
- the control generation or equivalent control evidence bound to ticks;
- app submission evidence distinct from control evidence;
- tick receipts owned by the scheduler, not by the app command.

## Related Documents

- `docs/design/reference-trusted-runtime-host-loop.md`
- `docs/design/wasm-trusted-runtime-host-control-boundary.md`
- `docs/design/v0.1.0-jedit-release-gate.md`
- `docs/design/v0.1.0-jedit-next-ten-slices.md`
