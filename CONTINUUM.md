<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Continuum

Status: Draft platform memo

This is the short platform memo for the multi-repo system around Echo.

If you want the longer bridge from AIΩN, WARP, observer geometry, and optics
into the runtime design, read
[`docs/continuum-foundations.md`](docs/continuum-foundations.md).

## One Sentence

Continuum is a shared causal platform where `git-warp` provides the cold
substrate, Echo provides the hot substrate, `warp-ttd` observes both and can
control them through explicit capabilities, and Wesley compiles the contracts
that keep them interoperable.

## Thesis

The point is one causal model, multiple runtime temperatures, and one contract
compiler that prevents the runtimes, tools, and protocols from drifting into
mutually incompatible folklore.

There are two different problems here, and they should not be forced into the
same runtime:

- durable, asynchronous, distributed, multi-writer causal storage
- deterministic, replayable, high-throughput execution

Trying to make one engine be equally good at both is how architecture gets
stupid.

## Runtime Temperatures

`cold` and `hot` are not branding. They are a way to describe which physical
problem a runtime is optimized to solve.

### `cold`

Cold is optimized for:

- durable history
- disconnected or asynchronous writers
- replication and later convergence
- provenance-preserving storage
- causal reconstruction across hosts and time

That is `git-warp`.

### `hot`

Hot is optimized for:

- deterministic stepping
- scheduler-sensitive execution
- immutable snapshot reads
- private delta writes
- canonical merge
- replay-grade runtime artifacts

That is Echo.

### Rule

Temperature is a property, not a hierarchy.

Hot is not "better than" cold. Cold is not "legacy hot." They solve different
problems and should keep different correctness and performance priorities.

## Minimum Shared Contract Surface

The platform only works if a small set of shared surfaces is actually
canonical.

At minimum, that surface should include:

- protocol envelopes
- causal coordinates
- receipts, effect emissions, and delivery observations
- capability declarations
- manifest and registry identifiers

These are the nouns that must stay stable across hot and cold hosts.

If these drift across repos, Continuum collapses into adapter spaghetti.

## Ownership

| Layer      | Owns                                                                                          | Does Not Own                                                                  |
| ---------- | --------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| `git-warp` | Git-native causal storage, async convergence, durable history, cold-world transport           | hot execution semantics, debugger UX, schema compilation                      |
| Echo       | deterministic execution, scheduler and commit semantics, replayable runtime artifacts         | cold-world storage semantics, cross-host debugger ownership, schema authority |
| `warp-ttd` | cross-host observer/control protocol, playback vocabulary, host adapters, inspection surfaces | host-specific runtime semantics, storage internals, schema compilation        |
| Wesley     | source-of-truth schemas, generated Rust/TS/codec/manifests, conformance fixtures              | runtime policy, storage policy, debugger policy                               |

This ownership split is the architecture. If it gets mushy, the platform gets
mushy.

## Wesley's Job

Within Continuum, Wesley's platform role is contract compilation.

That does not mean "Wesley already fully does everything described here today."
It means this is the scope Wesley needs to own for the platform to stay
coherent.

For shared contract surfaces, Wesley should generate:

- Rust types
- TypeScript types
- canonical codec contracts
- manifests and registry ids
- validation helpers
- conformance fixtures

The rule is simple:

No handwritten shadow contract if a Wesley schema is supposed to own that
surface.

## Concrete Example: Receipt Flow

The architecture needs at least one end-to-end artifact flow that is boringly
explicit. A receipt flow is a good candidate.

Target shape:

1. A Wesley schema defines the shared envelope family:
   `Coordinate`, `Receipt`, `EffectEmission`, `DeliveryObservation`, and
   `Capability`.
2. Wesley generates Rust types, TypeScript types, codec metadata, and fixture
   vectors for that family.
3. Echo emits receipt-shaped records at the hot execution boundary.
4. `git-warp` persists or exposes compatible receipt-shaped records on the cold
   history side.
5. `warp-ttd` inspects both through the same generated protocol family instead
   of host-specific packet folklore.
6. Conformance tests prove that Rust and TS round-trip the same bytes and that
   both hosts satisfy the same envelope contract.

That is what "same causal model" should cash out to at the artifact level.

## Current Reality vs Target State

### Current reality

The shape is already visible:

- Echo already behaves like a hot deterministic runtime.
- `git-warp` already behaves like a cold causal substrate.
- `warp-ttd` already treats itself as a cross-host debugger.
- Wesley already owns real schema-driven code generation.

There is already evidence of convergence:

- `warp-ttd` explicitly models both `ECHO` and `GIT_WARP` as host kinds.
- Echo already points at Wesley as a GraphQL-to-Rust/TypeScript compiler path.
- Wesley already contains WARP- and TTD-related schemas.
- `warp-ttd` already treats its Wesley schema as a source of truth for protocol
  contracts.

### Target state

What is not yet true is the full end-to-end guarantee.

This memo is target-state architecture, not a claim that the whole chain is
already frozen and proven.

The missing work is:

- freezing the minimum shared contract surface
- making both runtimes consume generated artifacts for that surface
- publishing a clear ownership map for shared nouns
- adding artifact-level compatibility tests across hot and cold hosts

That distinction is what keeps this memo honest.

## Rules

These are the rules that matter most.

### 1. One noun, one owner

If `receipt`, `worldline`, `coordinate`, `capability`, or `delivery
observation` is globally shared, it needs one contract owner and one generated
surface.

### 2. Translation belongs at generated boundaries

If hot and cold hosts need to share a noun, the shared representation should
come from a schema-generated contract.

Ad hoc JSON adapters are how cross-repo systems rot.

### 3. Substrate facts stay substrate facts

Effects, receipts, worldlines, and delivery observations are substrate facts.

Tooling may interpret them. Tooling must not silently rewrite them.

### 4. Capability boundaries are explicit

Observation and control are different powers. Hosts must declare what they
allow, and the protocol must keep those powers separate.

### 5. History is never silently rewritten

Replay, rewind, seek, fork, and compare must stay explicit,
provenance-bearing operations.

No debugger convenience is allowed to fake canonical history.

## Near-Term Work

If this architecture is real, the next moves are not vague.

1. Freeze the minimum shared contract surface.
2. Pick one artifact family, preferably receipts, and implement the full
   Wesley -> Rust/TS/codecs -> Echo/`git-warp` -> `warp-ttd` path.
3. Add cross-host conformance and codec round-trip tests.
4. Publish the ownership map for shared nouns and protocol families.

## Why "Causal Operating System"

"Operating system" here does not mean kernel, drivers, or syscalls.

It means a shared causal control environment over multiple runtimes:

- common nouns
- common protocol envelopes
- common observer/control surfaces
- common contract compilation
- shared provenance discipline

If that phrasing earns itself, it earns itself by the contract surface staying
honest.

## Non-Goals

Continuum is not trying to:

- collapse hot and cold runtimes into one implementation
- hide performance tradeoffs behind fake universality
- treat GraphQL as a thin veneer over incompatible internal models
- let every repo define its own version of the same protocol
- force every tool to understand every substrate internal

## Short Version

- `git-warp` is cold
- Echo is hot
- `warp-ttd` observes and controls both
- Wesley keeps the shared contract surface honest

That is Continuum.
