<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# 0013 Generic Observer API And Plan

## One Sentence

Echo should host compiled observer plans over generic worldline truth by
returning deterministic tick results and holographic slice inputs while keeping
app-specific observer semantics out of handwritten substrate APIs.

## Why This Exists

The current optic handoff is now much clearer:

- applications author set-side operations as app contracts
- Echo admits those intents against generic substrate truth
- Echo returns deterministic result and receipt envelopes
- applications then obtain readings through observers

That still leaves one major runtime seam unnamed:

> what generic Observer API does Echo provide so app-authored observers can be
> hosted lawfully without turning Echo into an app server?

The answer cannot be:

- handwritten app-specific methods on Echo
- arbitrary host callbacks running over runtime truth
- implicit full-worldline materialization to satisfy every read

The answer has to be a generic observer boundary that accepts compiled plans,
works over sliced holographic inputs, and emits observer-relative readings.

## Core Split

Echo should preserve four distinct objects:

1. **Intent envelope**
    - compiled app request for the optic's set side
2. **Tick result**
    - deterministic admission outcome, receipt envelope, hologram reference,
      frontier update
3. **Observer plan**
    - compiled app request for the optic's get side
4. **Reading envelope**
    - observer-relative result emitted from a hosted or one-shot observer

These objects serve different jobs and must not be collapsed.

## What Echo Hosts

Echo should host:

- generic worldlines and causal history
- generic tick admission and receipt production
- hologram or boundary-style slice inputs
- observer instances carrying runtime observer state
- sliced reading production over the needed causal cone

Echo should not host:

- app-specific observer names as handwritten public methods
- app-specific payload semantics as handwritten runtime APIs
- arbitrary observer callbacks supplied by the host

## Static Versus Runtime

Echo should treat these as already-compiled static inputs:

- observer aperture and slice policy
- basis identifiers or basis plan
- observer state schema identity
- update and emission plan identity
- rights, exposure, and revelation constraints
- materialization budgets

Echo should treat these as runtime instance state:

- current observer state value
- current frontier or hologram reference
- current slice input
- emitted reading envelope

## Proposed Generic API Shape

The substrate-facing shape should move toward four generic operations:

1. `submit_intent(...) -> TickResult`
2. `register_observer(plan) -> ObserverHandle`
3. `advance_observer(handle, frontier_or_hologram) -> ReadingEnvelope`
4. `read_once(plan, frontier_or_hologram) -> ReadingEnvelope`

Possible neighboring helpers:

- `dispose_observer(handle)`
- `snapshot_observer_state(handle)`
- `resume_observer(handle, state)`

The exact transport encoding can vary by surface, but the semantic shape should
stay stable.

## Tick Result Contract

For this observer-facing substrate handoff, the post-admission result should be
thought of as:

```text
TickResult = (outcome, receipt, hologram_ref, frontier)
```

This does not mean Echo must fully materialize an app view immediately.

The important point is:

- admission result is one object
- later reading is a second object

The same UI may ask for both immediately, but they remain different semantic
jobs.

## Reading Contract

Observer reads should be thought of as:

```text
(ObserverPlan, Frontier | HologramRef)
  -> slice causal cone
  -> update observer state
  -> emit ReadingEnvelope
```

The reading envelope should carry at least:

- observer handle or observer identity
- frontier or hologram reference
- reading payload
- reading payload hash or trace identity where needed

## One-Shot Versus Hosted Observers

Echo should support at least two classes of lawful observer use.

### One-shot observer

- no long-lived runtime handle required
- may still be memoryless or accumulative from the plan's perspective
- useful for immediate post-tick reads

### Hosted observer

- persistent runtime handle
- state advances across several frontiers or holograms
- useful for debugger/session/tooling flows

The distinction should remain explicit because hosted state is a real runtime
object, not merely a nicer query wrapper.

## Rights And Exposure

Observer legality is not only a performance issue.

Echo's generic Observer API should also enforce compiled constraints such as:

- exposure tier
- revelation tier
- redaction class
- whether receipt-only or witness-bearing details may surface
- whether the observer may see canonical-only, strand-aware, or braid-aware
  truth

Those rights should come from the compiled observer plan, not from ad hoc host
switches.

## Relationship To Current ObservationRequest

The current `ObservationRequest` path is still useful, but it is only an early
substrate read seam.

It is not yet the full observer lifecycle because it does not explicitly model:

- compiled observer plan identity
- hosted observer instance state
- one-shot versus persistent observer distinction
- observer-specific update and emission law

That means `observe(...)` should be treated as the current bridge, not the
finished observer boundary.

## Immediate Next Step

The next substrate-facing implementation move should be:

1. freeze one generic observer plan shape
2. freeze one reading envelope shape
3. prove one memoryless observer through the generic API
4. prove one accumulative observer through the generic API

The first application-facing proving target should be a `jedit`
`worldlineSnapshot` observer compiled into a generic observer plan rather than
a handwritten Echo method.
