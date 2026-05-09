<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Continuum Transport

_Echo exports and imports witnessed causal suffix bundles through Continuum
families. It does not synchronize materialized state._

## Boundary Rule

Continuum is the shared WARP protocol layer. Like HTTP, it lets independent
runtimes exchange lawful boundary artifacts without sharing implementation
internals. Unlike a graph database protocol, it does not claim there is one
canonical graph to synchronize.

Continuum transport uses Echo's witnessed suffix evidence model:

```text
export_suffix -> CausalSuffixBundle
import_suffix -> ImportSuffixResult
```

The shared Continuum runtime-boundary family must name Echo's actual transport
objects:

- `WitnessedSuffixShell`
- `CausalSuffixBundle`
- `WitnessedSuffixAdmissionResponse`
- `WitnessedSuffixAdmissionOutcome`
- `ImportOutcome`

The older generic `SuffixShell` wording is not the canonical boundary. It was a
placeholder for the shape now present in Echo.

## Ownership

Continuum owns the shared authored family.

Echo owns the runtime evidence shape for this boundary because Echo is the first
runtime implementing and consuming it. If a Continuum schema or registry row
does not match Echo's witnessed suffix model, update Continuum.

Wesley compiles the authored family. It does not define transport semantics.

Other Continuum-speaking tools may consume the family, but they do not get to
weaken the causal evidence contract into state snapshots, summary strings, or
host-time ordering.

Echo and `git-warp` are compatible because they exchange witnessed causal
history through Continuum-shaped families. They are not compatible because they
both model "the graph." There is no substrate-owned graph.

## Transport Object

A `CausalSuffixBundle` is a compact witnessed history object:

- source and target provenance frontiers
- ordered source provenance entries
- boundary witness when the suffix has no importable entries yet
- deterministic source shell identity
- deterministic bundle identity
- optional basis/overlap evidence reused from settlement

It is not:

- a materialized graph snapshot
- a reading cache
- a raw patch stream
- a peer mutation command
- a last-writer-wins delta

## Import Law

Import is ordinary admission at a distance.

Inbound transport admission is Intent-driven:

```text
transport adapter receives bytes
-> adapter forms a canonical import proposal
-> dispatch_intent(EINT import intent)
-> ingress / scheduler / admission
-> tick + receipt / witness
```

The runtime must:

1. Verify the source shell and bundle identities.
2. Resolve the explicit target basis.
3. Reuse retained prior import outcomes for idempotence.
4. Classify the result as `Admitted`, `Staged`, `Plural`, `Conflict`, or
   `Obstructed`.
5. Emit a receipt or witness for the local decision.

The runtime must not:

- silently mutate the current frontier when the base is stale
- dedupe by visible state hash alone
- dedupe by runtime-local tick, Lamport clock, or receipt hash alone
- hide self-history loops as new remote work
- collapse alternate support paths into no-op folklore

The host adapter may receive, decode, retain, and cache transported bytes. It
must not mutate worldlines, strands, braids, settlement state, provenance, or
retained import outcomes directly. A transported suffix affects Echo history
only when an import Intent is admitted.

## Causal Mutation Rule

The same rule applies to every external topology-changing operation:

- fork worldline / create strand
- append braid member
- collapse or settle braid
- merge / settlement import
- pin or unpin support when exposed to application flows
- admit transported causal suffix
- append inverse / compensating operation

External callers propose these operations as Intents against explicit causal
bases. Echo admits, stages, pluralizes, conflicts, or obstructs them under a
named law and emits receipts.

Internal services and evaluators may remain implementation details. They are not
public mutation authority.

## Idempotence

Exact bundle re-import is not new work. It is the same import question returning
again. Echo may return the retained result or emit a local receipt pointing at
the prior result, but it must preserve the evidence that the bundle was already
adjudicated.

Same final state is not enough for idempotence. Two bundles can produce the
same visible reading while preserving different provenance, support, or intent
observer structure.

## Relation To Optics

Transport uses the same WARP shape as optics:

```text
slice/project/normalise -> lower/admit -> pack/retain
```

Distribution changes the basis construction and transport path. It does not
create a second admission law.

More generally, tick admission, transport import, fork, merge, braid,
settlement, support mutation, inverse admission, observation, materialization,
and hologram slicing are all WARP optic operations over witnessed causal
history. Their outputs are holograms with different effect postures: admitted
history, observer-relative reading, retained artifact, or obstruction.

## Current Design Packet

The active decision packet is:

- `docs/design/0022-continuum-transport-identity/design.md`

The earlier suffix-sync packet remains the broad design ancestor:

- `docs/design/0009-witnessed-causal-suffix-sync/design.md`
