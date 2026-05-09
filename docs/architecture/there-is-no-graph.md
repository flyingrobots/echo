<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# There Is No Graph

_The graph is a coordinate chart over witnessed causal history. It is not
Echo's substrate ontology._

## Rule

There is no privileged, substrate-owned, canonical materialized graph.

The territory is witnessed causal history:

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

The hard formulation is:

```text
Computation is the construction, inspection, and admission of witnessed
readings over causal history.
```

That does not mean state-like values disappear. Runtime state, files,
databases, editor buffers, build artifacts, terminal screens, and generated
code all still exist. They are materialized readings. They are not the
territory.

## WARP Optics

The common WARP shape is:

```text
bounded causal basis/site
+ law
+ observer aperture
+ support obligations
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

All of them are WARP optics producing holograms.

A hologram is a witnessed, law-named artifact carrying enough basis, aperture,
support, evidence, identity, and posture to recreate the claimed object up to
the equivalence relation declared by the optic law.

The optic is stronger than a plain projection. It carries observer geometry:

- who or what is observing;
- which aperture is lawful;
- why the reading is being requested;
- which support must travel with the claim;
- which support may be compressed, redacted, or blocked;
- which law admits or obstructs the result.

## State Machines As A Special Case

Traditional state machines are not abolished. They are demoted.

A conventional mutable-state system is a narrow optic with:

- one privileged observer;
- one privileged materialization;
- one local transition function;
- weak or implicit witness obligations.

Echo may still implement state-like machinery internally. That machinery is an
implementation detail below the public WARP contract. It must not leak into API
language as a universal mutable state object.

## Runtimes As Optics

A WARP optic is not only a small API method. Whole runtimes and tools can be
understood as optics when they expose a law-governed way to admit, observe,
rewrite, retain, or project causal artifacts.

These are product roles, not ontological categories:

| Runtime or tool | WARP optic role                                                   |
| --------------- | ----------------------------------------------------------------- |
| Echo            | live execution and deterministic admission optic                  |
| `git-warp`      | Git-backed causal persistence optic                               |
| Wesley          | semantic/compiler optic over authored contract history            |
| `warp-ttd`      | historical inspection and causal forensics optic                  |
| Graft           | governed aperture and support-obligation optic                    |
| WARPDrive       | POSIX/FUSE materialization and write-back optic for legacy tools  |
| `jedit`         | human-facing console that hosts readings, lanes, and admission UI |

Echo, `git-warp`, Wesley, `warp-ttd`, Graft, WARPDrive, and `jedit` are not
separate kinds of machine at the WARP layer. They are WARP optics with
different apertures, substrates, admission laws, tick shapes, support
obligations, and hologram families.

Wesley is a useful example because it is not a simulator at all. It still has
the WARP shape: authored GraphQL/schema input is projected into semantic
readings, target readings, and witnessed materializations under compiler law.
Generated artifacts are holograms over a semantic coordinate, not magic files.

`warp-ttd` is the same kind of thing from another aperture. It is not outside
the system looking in. It is an observer that asks how a reading became
possible, which suffixes contributed, which obligations moved, which rejected
branches nearly happened, and which support was compressed, redacted, or
blocked.

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

## What Travels

The graph is not the transport payload.

The wrong model is:

```text
Echo graph -> serialize -> git-warp graph -> modify -> send back
```

That smuggles a canonical object model back into the architecture.

The WARP model is:

```text
causal suffix
+ coordinate
+ optic or rule identity
+ support obligations
+ witness refs
+ hologram boundary
-> compatible local reading
```

Each runtime projects the reading appropriate to its own substrate and law.
Echo may project one chart, `git-warp` another, Wesley another, and `warp-ttd`
another. The readings can be compatible without being identical internal
objects.

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
- coordinates;
- optic or rule identifiers;
- support obligations;
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

Continuum is not another runtime and not another graph model. It is the
compatibility membrane that lets independent WARP optics exchange enough
witnessed causal evidence to produce mutually intelligible readings.

## WARPDrive

WARPDrive is the compatibility layer for normal tools.

A mounted path is not primary storage. It is a POSIX-shaped aperture:

```text
read path at coordinate C through optic O -> materialized bytes
```

A write is not an overwrite of substrate truth. It is a candidate suffix:

```text
prior reading + new bytes -> delta/hunk -> Intent -> admission attempt
```

This lets ordinary editors, formatters, shells, and IDEs operate against a
normal-looking directory while Echo, `git-warp`, or another WARP runtime keeps
witnessed causal history as the authority.

Files remain real as boundary readings. They stop being the source of truth.

## Observer Geometry

Observer Geometry is the discipline that prevents "reading" from becoming a
loose synonym for query.

A reading must name or imply:

- observer and purpose;
- aperture;
- causal basis;
- path-sensitive support obligations;
- rights posture;
- budget posture;
- residual, redaction, plurality, or obstruction posture.

Missing support is not a cache miss to paper over. Missing support is an
obstruction, rehydration requirement, redaction, or explicit residual posture.

## API Consequences

Echo APIs must not expose mutable graph handles, global graph APIs, direct
setters, or hidden materialization fallbacks.

External callers either:

- propose an Intent against an explicit causal basis; or
- observe through a bounded optic and receive a reading/hologram; or
- retain/reveal an artifact by semantic identity and evidence posture.

Internal services may keep whatever data structures are practical. They do not
become public mutation authority.

Echo should not become the universal WARP runtime. Echo speaks Continuum and
implements one WARP optic family. It must not absorb Wesley, Graft, `git-warp`,
`warp-ttd`, or WARPDrive as privileged substrate concepts.

## Operational Corrections

WARP does not make hard problems disappear. It makes them typed and
witnessable.

- Reproducibility is not automatic. It becomes a support obligation over
  clocks, randomness, network, filesystem reads, environment variables,
  toolchain versions, policy state, model versions, and human approvals.
- Conflict does not disappear. Text conflict is demoted into semantic,
  support, policy, admission-law, or optic-compatibility conflict.
- Caches do not become truth. A cached reading is valid only for the coordinate,
  aperture, law, witness basis, rights posture, and budget posture it names.
- Files do not disappear at the boundary. WARPDrive makes them materialized
  readings and turns writes into candidate suffixes.

## Mathematical Posture

The useful category-theoretic intuition is:

```text
causal history is the base territory;
optics are dependent/provenance-carrying projections over that territory;
readings are local charts;
holograms are witnessed boundary artifacts;
admission extends the territory with a lawful suffix.
```

A WARP optic is stronger than a plain functor. A functor captures
composition-preserving projection, but WARP also carries observer aperture,
support obligations, redaction/compression/blocking posture, admission law, and
witness production.

Continuum is not itself the colimit. It is the protocol that lets runtimes
exchange diagram fragments, suffixes, coordinates, witnesses, and optic
contracts so they can form compatible readings and lawful admissions.

## Sentence To Keep

```text
There is witnessed causal history.
WARP optics chart it.
Holograms witness those charts.
Materialized graphs are optional readings.
Continuum is the protocol for lawful causal-history exchange.
```

Even shorter:

```text
There is no state.
There are readings with obligations.
```
