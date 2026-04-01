<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Continuum

Status: Draft architecture memo

Continuum is the larger system that Echo belongs to.

For the formal-to-product bridge from AIΩN, WARP, observer geometry, and
optics into Echo, `git-warp`, `warp-ttd`, and Wesley, see
[`docs/continuum-foundations.md`](docs/continuum-foundations.md).

It is a causal operating system with two runtime temperatures:

- `cold` for durable, asynchronous, multi-writer causal storage
- `hot` for deterministic, high-throughput causal execution

The intended contract story is:

```text
GraphQL schema
  -> Wesley
    -> Rust types
    -> TypeScript types
    -> canonical binary codecs
    -> manifests
    -> invariants
      -> hot runtime (Echo)
      -> cold runtime (git-warp)
      -> observer/control plane (warp-ttd)
```

The point is not "three related projects."

The point is one causal model, multiple runtime temperatures, and one contract compiler that prevents the runtimes, tools, and protocols from drifting into mutually incompatible folklore.

## One Sentence

`git-warp` is the cold substrate, Echo is the hot substrate, `warp-ttd` is the observer and control plane, and Wesley is the contract compiler that keeps them speaking one causal language.

## Foundational Stack

At the highest level:

- WARP supplies the causal substrate
- observer geometry supplies the epistemic model
- optics supply the lawful localized transformation boundary
- Wesley compiles the shared contracts
- Echo and `git-warp` implement two runtime temperatures over that model
- `warp-ttd` exposes the observer and control plane over both

This memo focuses on platform architecture. The companion memo in
[`docs/continuum-foundations.md`](docs/continuum-foundations.md) explains why
that stack is the right bridge from theory to runtime design.

## Why This Exists

There are two different problems here, and they should not be forced into the same runtime:

- Durable, asynchronous, distributed, multi-writer causal storage
- Deterministic, replayable, high-throughput execution

Trying to make one engine be equally good at both is how architecture gets stupid.

Continuum says:

- put cold concerns in the cold runtime
- put hot concerns in the hot runtime
- keep the debugger, protocol, and contract surfaces shared
- make compatibility come from generated contracts, not hand-maintained glue

## The Four Pillars

### 1. `git-warp` is the cold runtime

`git-warp` is the Git-native WARP substrate.

It is optimized for:

- durable graph storage
- append-only causal history
- disconnected and later-synchronized writers
- deterministic convergence after replication
- worldline, observer, and strand semantics over Git objects and refs

This is where you want to live when the primary problem is:

- asynchronous collaboration
- durable replicated state
- provenance-preserving storage
- causal history that survives process and host boundaries

Cold does not mean weak.

Cold means the system is optimized for durability, convergence, and causal integrity over time, not for a realtime simulation hot path.

### 2. Echo is the hot runtime

Echo is the deterministic execution substrate.

It is optimized for:

- immutable snapshot reads
- private delta writes
- canonical merge
- deterministic scheduling
- replayable execution artifacts
- high-throughput, replayable graph rewriting

This is where you want to live when the primary problem is:

- fast deterministic execution
- simulation and stepping
- scheduler behavior
- replay-grade runtime artifacts
- controlled, inspectable state transitions at tick speed

Hot does not mean sloppy.

Hot means the system is optimized for execution pressure, not for Git-native durability or asynchronous merge as the main event.

### 3. `warp-ttd` is the observer and control plane

`warp-ttd` is not supposed to be a host-specific debugger.

It is the cross-host observer and debugger protocol for WARP-based systems.

It owns:

- playback heads
- lane catalogs
- frame inspection
- receipt inspection
- effect emission inspection
- delivery observation inspection
- explicit debug controls like pause, step, seek, fork, and compare

Its job is not to know substrate internals.

Its job is to consume a host adapter boundary and expose a consistent investigative surface across runtimes.

That means the same debugger vocabulary should work across:

- Echo
- `git-warp`
- future WARP-based hosts

### 4. Wesley is the contract compiler

Wesley is the compatibility membrane.

The target role of Wesley in Continuum is larger than "generate database things."

Within Continuum, Wesley is the schema compiler that turns GraphQL SDL plus `@wes_*` directives into:

- Rust types
- TypeScript types
- manifests
- registries
- invariants
- canonical codec contracts
- generated boundary artifacts for host adapters and tools

If Continuum works, it works because the contract compiler is strong enough that Echo, `git-warp`, and `warp-ttd` stop hand-maintaining the same nouns in different dialects.

## The Shared Causal Vocabulary

Continuum only works if the core nouns stay stable across temperatures.

The important nouns are:

- `worldline`
  A causal history, not just a timeline.
- `tick`
  A logical step in a worldline.
- `receipt`
  The provenance-bearing record of what was admitted, rejected, emitted, or observed.
- `effect emission`
  A substrate fact that the runtime produced an outbound effect candidate.
- `delivery observation`
  What actually happened to that effect at a sink.
- `coordinate`
  An explicit location in causal history.
- `observer`
  A shaped read, not raw omniscience.
- `strand`
  A speculative or branched lane.
- `playback head`
  A coordination primitive for stepping through a causal surface.
- `capability`
  What a host is explicitly willing to allow, especially for control operations.

These are not branding terms.

They are the compatibility surface.

If these drift across repos, Continuum collapses into adapter spaghetti.

## Runtime Temperatures

### Cold

Cold runtime properties:

- append-first
- durable
- replication-friendly
- eventually convergent
- tolerant of disconnected writers
- optimized for storage integrity and causal reconstruction

Cold runtime examples:

- Git-backed graph storage
- durable worldlines
- patch history
- observer-friendly historical reads

### Hot

Hot runtime properties:

- deterministic
- throughput-sensitive
- scheduler-sensitive
- replay-oriented
- optimized for execution pressure and immediate stepping

Hot runtime examples:

- tick execution
- private deltas and canonical merge
- scheduler admission and conflict handling
- fast replay and inspection artifacts

### Why the split matters

The hot runtime should not be punished by cold-storage constraints on every step.

The cold runtime should not be distorted into pretending it is a game-loop engine.

Continuum gets strength by refusing to collapse those concerns.

## What Binary Compatibility Means Here

The target is not "these systems feel conceptually similar."

The target is stronger:

- Echo and `git-warp` share schema-defined causal nouns
- host-visible protocol envelopes are generated from Wesley schemas
- Rust and TypeScript representations are generated from the same source
- codec behavior is canonical and testable
- observers, debuggers, and tools consume generated contracts, not handwritten parallel models

In practice, binary compatibility means:

- the same schema can define message envelopes and read models used on both sides
- a debugger or host adapter can speak to hot and cold runtimes through the same generated protocol family
- artifact translation becomes explicit and mechanical instead of improvised

The contract compiler is what makes "same idea" become "same bytes."

## Current Reality vs Target State

### What exists today

Today, the pieces are already visible:

- Echo already behaves like a hot deterministic runtime
- `git-warp` already behaves like a cold causal substrate
- `warp-ttd` already models itself as a cross-host debugger
- Wesley already owns schema-driven code generation and contract logic in multiple places

There is already evidence of convergence:

- `warp-ttd` explicitly models both `ECHO` and `GIT_WARP` as host kinds
- Echo already points at Wesley as a GraphQL-to-Rust/TypeScript compiler path
- Wesley already contains WARP- and TTD-related schemas
- `warp-ttd` already treats its Wesley schema as the source of truth for protocol contracts

### What is not yet done

What is not yet fully locked is the whole end-to-end compatibility story.

This memo is a target-state architecture memo, not a claim that every boundary is already frozen and proven.

The missing work is the part that turns the shape into hard guarantees:

- shared schema ownership for the key causal protocols
- generated Rust plus TS plus codec artifacts that both runtimes actually consume
- artifact-level compatibility tests across hot and cold hosts
- a smaller set of stable, named contract surfaces
- explicit rules about which repo owns which noun

## The Contract Membrane

The contract membrane is the most important architectural idea in Continuum.

It should work like this:

1. A schema is authored in GraphQL SDL.
2. Wesley validates directive usage and schema structure.
3. Wesley emits generated artifacts for each consumer domain.
4. Hot and cold runtimes conform to the generated contract, not an informal translation.
5. `warp-ttd` consumes host adapters that expose the shared protocol family.

The membrane should generate at least these families of artifacts:

- protocol types
- registries and manifest ids
- canonical codec metadata
- validation helpers
- invariants and capability surfaces
- conformance fixtures

The rule is simple:

No handwritten shadow contract if a Wesley schema is supposed to own that surface.

## What Each Layer Owns

### `git-warp` owns

- Git-native causal storage
- patch history on the cold substrate
- asynchronous multi-writer convergence
- durable observer and strand semantics on Git-backed state
- provenance-preserving reads over durable history

### Echo owns

- deterministic execution
- scheduling and commit semantics for the hot path
- replayable execution artifacts
- high-throughput rewrite application
- immediate step/seek/fork execution semantics where runtime throughput matters

### `warp-ttd` owns

- the cross-host debugger protocol family
- host adapter boundaries
- investigation state
- delivery adapters such as CLI, TUI, MCP, and web surfaces
- host-agnostic causal inspection and control semantics

### Wesley owns

- source-of-truth schemas for shared contracts
- type generation
- codec and manifest generation
- invariants and evidence about contract outputs
- the discipline that prevents drift between Rust, TypeScript, and binary boundary behavior

## Architectural Rules

These rules are the part that keeps Continuum from degrading into a slogan.

### Rule 1: Substrate facts stay substrate facts

Effects, receipts, worldlines, and delivery observations are substrate facts.

Tooling may interpret them.

Tooling must not silently rewrite them.

### Rule 2: History is never silently rewritten

Rewind, replay, seek, and fork must always remain explicit, provenance-bearing operations.

No debugger convenience is allowed to fake canonical history.

### Rule 3: Translation belongs at generated boundaries

If hot and cold runtimes need to share a noun, the shared representation should come from a schema-generated contract.

Ad hoc JSON adapters are how cross-repo systems rot.

### Rule 4: Temperature is a property, not a hierarchy

Hot is not "better than" cold.

Cold is not "legacy hot."

They solve different problems and should keep their own performance and correctness priorities.

### Rule 5: Capability boundaries are explicit

Observation and control are different things.

The protocol must keep them separate, and hosts must declare what they allow.

### Rule 6: One noun, one owner

If `receipt`, `strand`, `worldline`, or `delivery observation` means something shared, that meaning needs one contract owner and one generated surface.

## How Data Moves Across Continuum

At a high level:

```text
Cold causal state (`git-warp`)
  -> shared contract surface
    -> observer/control plane (`warp-ttd`)

Hot causal execution (Echo)
  -> shared contract surface
    -> observer/control plane (`warp-ttd`)

Wesley
  -> keeps the two surfaces compatible
```

And at the artifact level:

```text
Schema
  -> generated ids
  -> generated codec contracts
  -> generated Rust types
  -> generated TS types
  -> generated manifests
  -> host adapters
  -> debugger consumers
```

## Why Call It A Causal Operating System

"Operating system" here does not mean kernel, drivers, or syscalls.

It means a shared causal control plane over multiple runtimes:

- common nouns
- common debugger surfaces
- common contract compilation
- shared provenance discipline
- explicit control over observation, replay, fork, and comparison

Continuum is the operating environment for causal systems built on WARP, not a conventional OS.

## Non-Goals

Continuum is not trying to do these things:

- collapse hot and cold runtimes into one implementation
- hide performance tradeoffs behind a fake universal substrate
- make GraphQL a marketing veneer over incompatible internal models
- let every repo define its own flavor of the same protocol
- force all tools to know every substrate internals detail

## Near-Term Work

If this architecture is real, the next important moves are not vague.

### 1. Freeze the shared nouns

Decide exactly which nouns are globally shared across Echo, `git-warp`, `warp-ttd`, and Wesley:

- worldline
- coordinate
- receipt
- effect emission
- delivery observation
- playback head
- strand
- capability

Then stop letting them drift.

### 2. Promote the right schemas into first-class Continuum contracts

The key shared contract families should be obviously canonical and intentionally owned.

That likely includes:

- TTD protocol
- shared causal receipt and effect envelopes
- runtime registry and manifest surfaces
- host capability declaration surfaces

### 3. Generate the contract surfaces that matter most

The most important generated outputs are the ones that remove parallel manual maintenance:

- Rust protocol types
- TS protocol types
- canonical codec bindings
- manifests and registries
- conformance fixtures

### 4. Add cross-host compatibility tests

Continuum needs proof, not just architecture prose.

That means:

- Echo host adapter conformance tests
- `git-warp` host adapter conformance tests
- shared fixture playback across hot and cold surfaces where appropriate
- codec roundtrip tests across Rust and TS

### 5. Publish the ownership map

Someone reading the system should know:

- which repo owns the hot runtime
- which repo owns the cold runtime
- which repo owns the debugger protocol
- which repo owns the contract compiler
- which schemas are canonical for cross-repo compatibility

## The Real Payoff

If Continuum succeeds, the payoff is not only cleaner code reuse.

The payoff is a system where:

- durable and realtime causal runtimes can coexist without pretending to be the same machine
- one debugger can inspect both
- one contract compiler can keep the boundary honest
- one causal vocabulary can stay stable across multiple temperatures

That is a much more interesting architecture than "database project over here, engine project over there, debugger project somewhere else."

That is a coherent platform.

## Short Version

If you need the shortest possible summary:

- `git-warp` is cold
- Echo is hot
- `warp-ttd` sees both
- Wesley keeps them compatible

That is Continuum.
