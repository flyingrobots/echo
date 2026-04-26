<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Continuum Foundations

Status: Draft architecture memo

This document is the formal-to-engineering bridge for Continuum.

It explains how the AIΩN / WARP / observer-geometry stack maps onto the
actual multi-repo system:

- Echo as the `hot` runtime
- `git-warp` as the `cold` runtime
- `warp-ttd` as the observer and control plane
- Wesley as the contract compiler and compatibility membrane

This memo is not a claim that every boundary is already frozen and proven.
It is the intended system model, written in a way that makes the missing work
obvious instead of magical.

See also:

- `CONTINUUM.md` in the repo root for the platform-level summary
- [`/docs/architecture/outline.md`](/architecture/outline) for the
  Echo-specific runtime story
- [0011 — Optic and observer runtime doctrine](./design/0011-optic-observer-runtime-doctrine/design.md)
  for the Echo runtime subset of the optic and observer formalism

## One Sentence

Continuum is a causal computing stack where WARP provides the substrate,
observer geometry provides the epistemic model, optics provide the lawful
boundary object, Wesley compiles the contracts, Echo executes the hot path,
`git-warp` preserves the cold path, and `warp-ttd` lets an observer inspect
and control both.

## Why This Memo Exists

Without this bridge, the repo story fragments into four partial truths:

- Echo looks like an isolated deterministic runtime
- `git-warp` looks like a Git-native graph store with its own language
- `warp-ttd` looks like a debugger protocol floating above both
- Wesley looks like a code generator with unclear architectural authority

Those are incomplete views.

The stronger reading is:

- the AIΩN papers define the substrate and the geometry
- the observer-geometry texts define how bounded agents see and compare
  histories
- the optic note defines the lawful rewrite boundary
- Wesley is the compiler for those shared boundaries
- Echo and `git-warp` are two runtime temperatures over one causal model
- `warp-ttd` is the observer-facing control plane over both

## The Foundational Stack

### 1. WARP is the substrate

WARP gives the base object model:

- state is structured graph
- change is lawful rewrite
- execution is worldline
- history is causal, not just chronological
- provenance is first-class
- replay is a substrate property, not optional tooling

This is the foundation under Echo's runtime, `git-warp`'s storage model, and
`warp-ttd`'s debugging vocabulary.

### 2. Observer geometry is the epistemic layer

The observer-geometry work adds the missing constraint:

- systems do not see "the whole truth" directly
- they see projections of causal history through bounded observers
- disagreement is not just data mismatch; it is observer-relative geometry
- replicas, debuggers, and operators are all observers with different bases,
  projections, and budgets

This matters because Continuum is not merely a runtime stack. It is a stack for
making causal state visible, transportable, comparable, and steerable under
observer limits.

### 3. Optics are the bridge from theory to executable boundary

The optic note is the most important engineering hinge in the current theory
stack.

It gives a clean shape for a lawful localized transformation:

- projection
- footprint
- local rewrite
- witness
- reintegration

In that framing:

- the observer is not the whole optic; it is closer to the projection half
- the footprint marks what is actually in scope
- the witness records semantic reversibility or local proof context
- the receipt records the operational envelope of what happened
- reintegration stitches the local rewrite back into the larger state

That is exactly the kind of object Wesley should compile, runtimes should
execute, and `warp-ttd` should explain.

## The Core Translation

The cleanest translation from theory to system architecture is:

| Formal Layer      | Engineering Meaning                                    | Concrete Owner                         |
| ----------------- | ------------------------------------------------------ | -------------------------------------- |
| WARP state        | structured causal state                                | Echo, `git-warp`                       |
| worldline         | committed execution or storage history                 | Echo, `git-warp`                       |
| observer          | bounded shaped read of history                         | `warp-ttd`, host adapters              |
| footprint         | declared visibility / interference boundary            | Echo today, later schema-owned         |
| witness           | semantic evidence for a local rewrite                  | runtime, debugger, generated contracts |
| receipt           | operational record of admission, emission, observation | runtime and debugger protocol          |
| optic             | lawful localized transformation boundary               | future shared contract surface         |
| contract compiler | generated shared boundary artifacts                    | Wesley                                 |

The key point is that these are not separate inventions. They are different
faces of the same object.

## Runtime Temperatures

Continuum has two runtime temperatures because one machine should not pretend to
solve two different physical problems equally well.

### `hot` means immediate execution pressure

The `hot` runtime is Echo.

Hot runtime properties:

- deterministic step execution
- scheduler-sensitive admission
- immutable snapshot reads
- private delta writes
- canonical merge
- replay-grade artifact production
- immediate inspection, stepping, diffing, and forking

Hot is where collapse happens under tight execution pressure.

### `cold` means durable causal transport and reconstruction

The `cold` runtime is `git-warp`.

Cold runtime properties:

- durable append-first history
- disconnected or asynchronous writers
- replication and later convergence
- suffix transport over unseen local history
- explicit conflict surfacing instead of silent overwrite
- provenance-preserving storage semantics

Cold is where histories survive machines, process boundaries, and human delay.

### The split is not aesthetic

The split exists because:

- hot runtimes should not pay cold-storage penalties on every step
- cold runtimes should not be bent into fake game loops
- the debugger should inspect both through the same causal language
- the contract layer should stop them from drifting into separate folklore

## Observer Geometry in Product Terms

Observer geometry makes several product decisions legible.

### `warp-ttd` is an observer plane, not just a debugger UI

`warp-ttd` exists because the system needs an explicit observer surface:

- inspect coordinates in causal history
- move playback heads
- compare divergent worldlines
- examine effect emission and delivery observation
- seek, pause, fork, and diff with declared capability boundaries

That is not host-specific implementation detail. It is the operator-facing
projection layer over causal state.

### Replication is an observer problem

The observer-geometry replica framing makes `git-warp` easier to describe.

A replica is not merely "another copy of the data." It is an observer over
shared causal history with:

- an accepted spine
- a frontier
- a projection

Replication then becomes transport across causal history, not naive overwrite or
folk-merge semantics.

### Debugging is geometry reading

Time-travel debugging becomes coherent under this model:

- a bug is a wrong turn in a worldline
- divergence is measurable between worldlines
- replay is re-entry into preserved causal history
- diff is structural comparison between histories, not text archaeology

This is why `warp-ttd` matters. It is not "nice tooling." It is the observer
surface required by the ontology.

## The WARP Optic as the Missing Shared Object

If Continuum has one architectural center of gravity, it should be the optic
shape.

An optic-shaped causal boundary would let the system express one operation in a
way that all layers can consume:

1. projection
2. footprint
3. local rewrite
4. witness
5. reintegration

That one declaration can drive:

- runtime admission
- conflict and interference analysis
- witness validation
- receipt formation
- debugger explanation
- schema-level contract generation
- binary codec emission

This is a far better long-term shape than parallel handwritten models for
"runtime operation," "debug packet," and "wire schema."

## Wesley's Real Job

Within Continuum, Wesley should be described as a contract compiler first.

Its real job is not merely "generate code from GraphQL."

Its real job is to turn shared causal contracts into:

- Rust types
- TypeScript types
- canonical binary codecs
- manifests and registries
- invariants
- capability declarations
- conformance fixtures

The bigger point is authority.

If a cross-repo noun is shared, Wesley should be the mechanism that keeps the
shared representation honest. Otherwise the system falls back to handwritten
parallel models and eventually degenerates into adapter spaghetti.

## What Binary Compatibility Should Mean

Binary compatibility in Continuum should mean more than "the ideas line up."

It should mean:

- the same schema defines the host-visible envelopes
- Rust and TypeScript types are generated from the same source
- codecs are canonical and round-trip identically
- manifests and ids are stable across hosts
- conformance fixtures pass against both hot and cold runtimes
- the debugger can talk to both hosts through one generated protocol family

In other words:

same nouns, same contracts, same bytes.

## What It Does Not Yet Mean

This memo does not claim that all of that is already locked today.

The current honest state is:

- the shape is visible
- the scaffolding is real
- several schemas already point in the right direction
- host kinds and protocol families already show convergence
- full artifact-level proof is still a deliverable, not a completed fact

That distinction matters. A strong architecture memo names the missing proof
obligations instead of pretending they do not exist.

## Ownership Map

### Echo owns

- deterministic hot execution
- scheduler and commit semantics
- runtime replay artifacts
- fast worldline stepping and inspection support

### `git-warp` owns

- durable cold causal storage
- Git-native history and transport
- replication and convergence over asynchronous collaboration
- preservation of causal lineage over time

### `warp-ttd` owns

- cross-host observer protocol
- host adapter boundary
- operator controls and investigative surfaces
- causal inspection and comparison workflows

### Wesley owns

- contract authorship workflow
- generated cross-language types
- generated codec and manifest artifacts
- the discipline that prevents hot and cold from inventing their own dialects

## Architectural Laws

These laws keep Continuum from degrading into a slogan.

### 1. One causal noun, one contract owner

If `worldline`, `receipt`, `observer`, `coordinate`, `strand`, or
`delivery observation` is globally shared, it needs one source-of-truth
contract owner.

### 2. Substrate facts stay substrate facts

Effects, receipts, and history are not allowed to become tool-local fiction.
Tools may interpret them. Tools must not quietly rewrite them.

### 3. History is never rewritten by convenience

Replay, seek, rewind, fork, compare, and transport must remain explicit
provenance-bearing operations.

### 4. Translation belongs at generated boundaries

Cross-repo translation should happen through generated contracts, not ad hoc
JSON folklore.

### 5. Capability boundaries stay explicit

Observation and control are different powers. Hosts must declare which powers
they allow, and the protocol has to keep them separate.

### 6. Temperatures are complementary, not hierarchical

Hot is not "more advanced cold." Cold is not "legacy hot." They are different
runtime regimes over one causal model.

## Proof Obligations

If this architecture is real, the following proof obligations have to exist:

1. Canonical schemas for the shared causal nouns and protocol families
2. Generated Rust, TS, and codec artifacts actually consumed by the runtimes
3. Round-trip tests proving byte-identical cross-language codec behavior
4. Host adapter conformance tests for Echo and `git-warp`
5. Fixture playback proving the debugger can inspect both through the same
   protocol family
6. An explicit ownership map saying which repo owns which contract surface

Without those, Continuum is still directionally right, but not fully hardened.

## Why This Matters

The payoff here is not just cleaner documentation.

The payoff is that the entire stack stops looking like four ambitious projects
that happen to rhyme and starts looking like one coherent causal platform:

- one substrate story
- one observer story
- one lawful boundary object
- one contract compiler
- two runtime temperatures
- one observer and control plane

That is the architecture worth building.

## Short Version

If you need the compressed version:

- WARP supplies the substrate
- observer geometry explains bounded sight and comparison
- optics supply the lawful local transformation shape
- Wesley compiles the shared contracts
- Echo runs the hot path
- `git-warp` preserves the cold path
- `warp-ttd` lets observers inspect and control both

That is Continuum in its strongest form.
