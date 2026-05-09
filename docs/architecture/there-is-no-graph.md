<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# There Is No Graph

_The graph is a coordinate chart over witnessed causal history. It is not
Echo's substrate ontology._

## Rule

There is no privileged, substrate-owned, canonical materialized graph.

Echo's durable substrate is witnessed causal history:

- admitted transitions;
- frontiers;
- lane identities;
- payload hashes;
- receipts;
- witnesses;
- checkpoints;
- suffixes;
- boundary artifacts;
- retained readings.

Graph-like structure exists as an observer-relative holographic reading over
that history. It may be retained, cached, transported, compared, revealed, or
debugged. It does not become substrate truth by being materialized.

## WARP Optics

The common WARP shape is:

```text
bounded causal basis/site
+ law
+ capability, budget, and evidence posture
-> witnessed hologram
```

Everything public should be understood through this shape:

- tick admission;
- graph rewrite admission;
- transport import;
- fork, merge, braid, and settlement;
- support pinning;
- inverse or compensating operation admission;
- observation;
- hologram slicing;
- materialization;
- retention and reveal.

The difference is effect posture, not ontology.

| Surface             | Optic posture               | Resulting hologram                         |
| ------------------- | --------------------------- | ------------------------------------------ |
| Intent / admission  | propose causal rewrite      | receipt, tick, provenance, outcome         |
| Transport import    | propose suffix admission    | import receipt, staged/plural/conflict law |
| Topology operation  | propose lane/topology law   | topology receipt and witness               |
| Observation         | project causal history      | reading envelope                           |
| Materialization     | lower a bounded projection  | materialized reading artifact              |
| Retention / reveal  | persist or recover artifact | retained hologram bytes plus identity      |
| Debug / explanation | inspect law and evidence    | explanation over named basis               |

All of them are WARP optics producing holograms. A hologram is a witnessed,
law-named artifact carrying enough basis, aperture, evidence, identity, and
posture to recreate the claimed object up to the equivalence relation declared
by the optic law.

## Runtimes As Optics

A WARP optic is not only a small API method. Whole runtimes and tools can be
understood as optics when they expose a law-governed way to admit, observe,
rewrite, retain, or project causal artifacts.

| Runtime or tool | WARP optic role                                                    |
| --------------- | ------------------------------------------------------------------ |
| Echo            | real-time deterministic simulation optic over causal history       |
| `warp-ttd`      | debugger/inspection optic over witnessed history and effect traces |
| `git-warp`      | optic that projects WARP history onto Git as a primitive substrate |
| Wesley          | compiler optic rewriting authored schema into IR and artifacts     |

Echo, `warp-ttd`, `git-warp`, and Wesley are therefore not four things
implementing one hidden graph model. They are different WARP optics with
different laws, postures, and artifact families.

Wesley is a useful example because it is not a simulator at all. It still has
the WARP shape: authored GraphQL/schema input is rewritten under compiler law
into IR and output artifacts. That sequence can be treated as a witnessed
artifact pipeline rather than a magical source-code generator.

## Graph-Shaped Readings

A graph-shaped reading is legal and useful. Echo may expose graph-shaped views,
indexes, cached readings, and materialized projections.

The safety rule is that every graph-shaped object must remain scoped to the
question it answers:

- causal coordinate or frontier;
- optic or observer law;
- aperture or local site;
- witness basis;
- rights posture;
- budget posture;
- projection and reducer versions;
- residual, plurality, or obstruction posture.

No graph-shaped reading may pretend to be the runtime itself.

## Continuum

Continuum is the shared WARP protocol layer.

The useful analogy is HTTP: Continuum lets independent WARP runtimes exchange
lawful causal-history artifacts without sharing an implementation under the
hood. It is not a claim that every runtime stores the same graph. It is a claim
that runtimes can exchange, admit, retain, observe, and compare witnessed
causal history through shared boundary families.

Echo and `git-warp` are compatible because they can speak this causal-history
protocol. They are not compatible because they both model a canonical graph.
There is no such graph.

Continuum-speaking runtimes exchange things such as:

- witnessed suffix bundles;
- receipts;
- witness refs;
- frontier identities;
- payload refs;
- admission outcomes;
- reading envelopes;
- retained hologram identities.

They do not exchange:

- runtime internals;
- scheduler state;
- private cache layout;
- materialized state as truth;
- graph database objects;
- host-time ordering folklore.

## API Consequences

Echo APIs must not expose mutable graph handles, global graph APIs, direct
setters, or hidden materialization fallbacks.

External callers either:

- propose an Intent against an explicit causal basis; or
- observe through a bounded optic and receive a reading/hologram; or
- retain/reveal an artifact by semantic identity and evidence posture.

Internal services may keep whatever data structures are practical. They do not
become public mutation authority.

## Sentence To Keep

```text
There is witnessed causal history.
WARP optics chart it.
Holograms witness those charts.
Materialized graphs are optional readings.
Continuum is the protocol for lawful causal-history exchange.
```
