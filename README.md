<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
<!-- markdownlint-disable MD033 -->

<p align="center">
  <img alt="ECHO" src="https://github.com/user-attachments/assets/bef3fab9-cfc7-4601-b246-67ef7416ae75" />
</p>

<p align="center">
  <strong>State is a reading. Echo makes readings lawful, witnessed, and replayable.</strong>
</p>

<p align="center">
  <a href="docs/README.md">Docs</a>
  &middot;
  <a href="docs/architecture/outline.md">Architecture</a>
  &middot;
  <a href="docs/architecture/there-is-no-graph.md">There Is No Graph</a>
  &middot;
  <a href="docs/architecture/continuum-transport.md">Continuum</a>
  &middot;
  <a href="docs/spec/warp-core.md">warp-core</a>
</p>

> Echo owns time, admission, scheduling, and witnessed causal history so
> application code can stay focused on domain semantics.

# Echo

**Echo** is a deterministic runtime for building applications on witnessed
causal history instead of mutable in-memory state.

Traditional runtimes treat "current state" as the source of truth. Echo does
not. In Echo, the durable territory is admitted causal history: submissions,
frontiers, receipts, witnesses, retained artifacts, and replayable evidence.
State-like things such as files, graphs, UI models, build outputs, debugger
snapshots, and query results are **readings** over that history.

Application code does not mutate Echo state directly, and it does not decide
when Echo ticks. Applications submit canonical intents. Echo admits, schedules,
settles, and executes them at runtime-owned tick boundaries, then emits receipts
and evidence-carrying observations.

## A Concrete Problem

Consider a collaborative editor, game simulation, build graph, or contract host
where two users submit work at nearly the same time. Most systems answer with a
mutable object, a lock, a queue, or a best-effort conflict handler. After the
fact, you may have logs, database rows, or a materialized file, but you do not
have machine-checkable evidence that the transition was admitted under a named
law from a named causal basis.

Echo starts from the other end. The durable object is the witnessed causal
history. Inputs become canonical intents. Runtime-owned admission decides what
can proceed. The scheduler admits a deterministic independent subset, commits a
tick, and emits receipts. Reads are bounded observations from explicit
coordinates, not informal snapshots of whatever mutable state happens to exist.

## When To Use Echo

| Need                           | Echo Provides                                              | Example Use Cases                                    |
| ------------------------------ | ---------------------------------------------------------- | ---------------------------------------------------- |
| Deterministic execution        | Runtime-owned ticks, admission, scheduling, and settlement | Simulations, engines, structured editors             |
| Replayability and auditability | Witnessed causal history, receipts, and retained artifacts | Build systems, compliance tools, versioned pipelines |
| Evidence-carrying reads        | Payloads wrapped in `ReadingEnvelope` evidence             | Debugging, time travel, proof-carrying data          |
| Law-governed collaboration     | Intent submission over shared causal history               | Multi-user structured editing                        |
| Causal transport               | Witnessed suffix import/export instead of state sync       | Multi-runtime and peer-to-peer systems               |
| Retained readings              | Content retention plus semantic lookup                     | Audit, forensics, replay, "what happened?" analysis  |

## Philosophy: There Is No Graph

Echo is not a graph database. It is not a mutable state server. It is a
deterministic WARP runtime over witnessed causal history.

| Dimension       | Traditional Runtime                    | Echo                                |
| --------------- | -------------------------------------- | ----------------------------------- |
| Source of truth | Mutable in-memory state                | Witnessed causal history            |
| Change model    | Direct mutation                        | Canonical intent submission         |
| Time authority  | Application callbacks, events, threads | Trusted runtime scheduler           |
| Read model      | "Give me current state"                | Bounded reading from explicit basis |
| Read result     | Bare payload                           | Payload plus `ReadingEnvelope`      |
| Distribution    | Replicate state                        | Exchange witnessed causal suffixes  |

**In Echo, causal history is primary. Everything else is derived.**

Read [There Is No Graph](docs/architecture/there-is-no-graph.md) for the deeper
model.

## Boundary Vocabulary

| Term              | Meaning In Echo                                          |
| ----------------- | -------------------------------------------------------- |
| Intent            | Canonical proposed work from an application or adapter   |
| Admission law     | The named rule that decides whether work can proceed     |
| Footprint         | Declared read/write support used for conflict checks     |
| Tick              | Runtime-owned logical commit boundary                    |
| Receipt           | Machine-checkable outcome evidence for admitted work     |
| Reading           | Bounded observation over causal history                  |
| `ReadingEnvelope` | Evidence wrapper for where a reading came from           |
| WARP optic        | Lawful projection from explicit basis and aperture       |
| Witnessed suffix  | Causal-history exchange unit, not state synchronization  |
| Retained artifact | Durable evidence or reading payload by semantic identity |

## How It Works

Echo currently implements two callback-shaped compatibility paths plus the
first generic hook-free executable-operation runtime slice. The Edict provider
mutation closure spans publication through provider-native Echo execution and
recovery, but ambient host callbacks still determine its application
semantics. Wesley packaging and the generated bounded-read corridor remain a
separate incomplete path.

The Wesley compatibility path emits raw `RewriteRule` builders and generated
helpers. Its integration fixture enables the policy-gated
`native_rule_bootstrap` feature and registers those rules directly. It does
not emit an `InstalledContractPackage` or exercise package verification.

Edict accepts exact semantic-source, contract-pack, and settings bytes. Its
compiler pipeline emits canonical semantic artifacts, runs a deterministic
lowerer and a structurally separate verifier path, and publishes a
digest-locked provider package plus a generated Rust helper projection. A
separate verifier path is not, by itself, an independently implemented
verifier. Echo later consumes the resulting proposal through runtime-owned
admission, corroboration, installation, invocation, receipt, and recovery
crossings.
The helper performs pure, fail-closed preflight across exact package, Target
IR, bundle, profile, schema,
codec, obstruction, ABI, helper API, operation, and footprint claims. It
exposes typed `Id`, `Input`, and `Output` codecs, packs typed input into
canonical EINT v1, and builds a provider-generic borrowed registry plus an
opaque, non-installing provider package proposal when explicitly bound to
matching host callbacks. Matching callback claims are cross-binding evidence,
not proof of callback semantics.
The helper does not construct an `InstalledContractPackage`, register or
install anything, invoke callbacks during proposal construction, schedule
execution, or mint runtime authority.

The trusted runtime host can now compare that proposal with an independently
constructed `ProviderContractAdmissionPolicyV1`. Exact agreement on the
host-owned occurrence claim and complete provider registry yields an opaque
`AdmittedProviderContractPackageV1`; release, semantic, schema, operation, and
other proposition drift fails with stable typed errors. This crossing admits
the pinned proposal claim. It does not load or rehash the package, mutate the
engine registry, install handlers, invoke callbacks, schedule work, or emit a
runtime receipt.

`echo-wesley-gen` can separately corroborate that admitted claim with exact
package bytes and consume the resulting opaque proof through Echo's sealed
runtime-owner installation port. `TrustedRuntimeHost` then installs a distinct
provider record and the package, root, mutation-operation, and scheduler-rule
indexes atomically, without invoking the handler or fabricating legacy
Wesley/GraphQL metadata. Before any index changes, the installation crossing
applies the same structural checks used by retained-evidence recovery to the
package reference, operation coordinate, and Target IR identity.

After a generated client submits canonical EINT v1 bytes, the trusted host can
admit a witnessed submission for an installed provider mutation. Echo requires
the exact EINT intent-kind domain and an installed provider operation before it
stages work through the shared scheduler. Runtime evidence binds the installed
package id, exact package reference, semantic operation, Target IR, and
scheduler rule. An applied outcome requires a receipt entry from that exact
provider rule; a same-scope system acknowledgement cannot stand in for provider
execution. The tagged WAL codec retains that evidence, and a fresh host can
recover the same outcome after independently reinstalling the same package
configuration without rerunning callbacks or duplicating work. Provider
evidence never invents a legacy retained-contract coordinate.

Those crossings prove the first local provider-mutation execution and recovery
path. They do not authenticate a caller or session, authorize an application
target, validate codec-owned input against an operation schema at Echo's generic
ingress boundary, support provider-native reads, or turn package metadata into
runtime authority. Echo separately retains the Wesley compatibility path for
host-constructed `InstalledContractPackage` values.

Separately, Echo can now admit and install an exact canonical
`ExecutableOperationPackageV1`, admit an exact-basis invocation under explicit
authority and delegated budget, evaluate its data-only
`EchoOperationProgramV1` privately, and either commit one patch or return typed
noncommit evidence. Only committed operation consequences enter the
operation-tick WAL. The initial generic program performs an anchored typed-node
alpha-attachment compare-and-set. Its receipt binds the admitted package,
operation, subordinate program, invocation, complete evaluation basis,
authority, declared and actual footprints, budgets, patch, result, and terminal
outcome. Runtime-control installation and committed execution-kernel records
permit callback-free fresh-host recovery. Program bytes explicitly bind the
interpreter and intrinsic profiles, while the parent patch and singleton tick
evidence bind the admitted installation. A program digest cannot confer
operation identity, invocability, or authority.

That generic runtime witness is not yet the Jim/Jedit vertical. No real Edict
compiler output, Jedit rope lawpack, or `ReplaceRange` operation uses it, and it
does not yet claim structurally separate target verification, scheduler batch
composition, or independently implemented semantic conformance. It also
temporarily reuses `TrustedRuntimeHost`'s joint `native_rule_bootstrap` and
`trusted_runtime` feature gate. The program itself has no native hooks, but the
host surface must be decoupled from the legacy bootstrap feature before a
product can remove that compatibility feature.

The following sequence is the existing Wesley bootstrap fixture:

```mermaid
sequenceDiagram
    participant Dev
    participant Wesley
    participant Fixture
    participant Echo

    Dev->>Dev: Author GraphQL contract
    Dev->>Wesley: Compile with echo-wesley-gen
    Wesley-->>Fixture: RewriteRule builders + helpers
    Fixture->>Echo: register_rule via native_rule_bootstrap
    Fixture->>Echo: Submit canonical intent
    Echo-->>Fixture: DispatchResponse with ingress evidence
    Echo->>Echo: Runtime-owned admission, scheduling, tick
    Fixture->>Echo: Send bounded optic request
    Echo-->>Fixture: OpticReading or typed obstruction
```

These corridors are not one pipeline with optional source nouns.

The current Wesley compatibility path is:

```text
GraphQL source
-> generated RewriteRule builders and helpers
-> trusted-host direct native rule registration
-> canonical intent
-> Echo admission, scheduling, and receipt
```

The current Edict provider-v1 compatibility path is:

```text
Edict source
-> canonical Core meaning and verified Echo Target IR
-> digest-locked provider package and generated helper
-> opaque provider proposal with explicit host binding
-> trusted-host exact proposal-claim admission
-> exact package corroboration and provider-native installation
-> exact EINT-kind and installed-operation admission
-> scheduler-owned callback execution
-> receipts and WAL recovery bound to package, operation, Target IR, and rule
```

The current executable-operation runtime slice is:

```text
canonical ExecutableOperationPackageV1 bytes
-> Echo-owned package and invocation admission
-> installed data-only EchoOperationProgramV1
-> canonical Action submission retained before acknowledgement
-> runtime-owned admission into the ordinary head inbox
-> scheduler selection at one exact basis
-> bounded private Echo evaluation during Tick construction
-> one composite Tick consequence with typed per-Action outcomes
-> decided Tick WAL retention before state, frontier, and Receipt publication
-> callback-free pending-Action and decided-Tick recovery
```

The first two paths are callback-shaped compatibility infrastructure. The
third proves Echo-owned execution of admitted data-only meaning through
separate update-only compare-and-set and single-node create-if-absent program
profiles. Independent Actions for one head can share one scheduler-owned Tick;
conflicting or obstructed Actions retain typed outcomes without hidden
mutation. No real Edict compiler output, Jedit operation, or Graft operation
uses this route yet. The next convergence crossing must bind a real
application-owned Edict operation and lawpack to the executable-operation
package without reintroducing a native implementation.

## Contracts And Boundaries

Echo core is intentionally generic. Application nouns belong in authored
contracts and generated adapters, not in the runtime kernel.

- Wesley compatibility fixtures define nouns, operations, and queries in
  GraphQL and use directives such as `@wes_op` and `@wes_footprint` for
  operation and footprint claims.
- Edict semantic sources define admitted operations, capabilities, lawpacks,
  target profiles, and schemas. The Echo provider path deterministically
  lowers and verifies that meaning into a digest-locked publication package
  and generated helper projection.
- The current Wesley compatibility generator emits Rust rule builders and
  helper code, not a verified installable package or a supported external
  application SDK.
- Echo's package registry and scheduler path is implemented independently of
  both compiler publication paths.
- Provider v1 binds host-supplied executors as explicitly transitional
  compatibility evidence; matching identities do not prove callback
  semantics.
- For newly authored executable operations, a trusted Echo runtime verifies
  and installs admitted package material, then Echo interprets the bound
  data-only program. The host supplies no application matcher, executor, or
  footprint callback.
- A compiler must not create a second execution engine, and a host must not
  substitute ambient application behavior for admitted executable meaning.

See [Generated Rule Authorship](docs/topics/GeneratedRules.md) for the exact
current/target boundary, including the separate Wesley compatibility and Edict
provider paths and the absent release footprint-qualification lane.

```graphql
type Mutation {
    increment(input: IncrementInput!): CounterValue!
        @wes_op(name: "increment")
        @wes_footprint(reads: ["CounterValue"], writes: ["CounterValue"])
}
```

## Core Guarantees

- **Runtime-owned time**: application code cannot tick Echo or choose scheduler
  boundaries.
- **Deterministic runtime judgments**: Echo owns admission, scheduling, tick
  formation, rule selection, settlement, and evidence construction.
- **Evidence-first observations**: readings carry basis, observer, witness,
  budget, rights, and residual posture.
- **Replayable history**: submissions, receipts, witnesses, and retained
  artifacts are structured for audit and replay.
- **Domain separation**: Echo core stays generic; application semantics live in
  contracts.
- **Continuum-oriented transport**: Echo is built for witnessed causal suffix
  exchange, not blind state synchronization.

## Determinism, Ticks, And The Scheduler

Echo enforces determinism by narrowing every application action into explicit,
canonical evidence before the scheduler can act on it:

- application input enters as canonical EINT bytes, not ad hoc callbacks;
- admitted compiler-generated metadata names operation ids, exact codecs where
  declared, and footprint or requirements claims;
- Echo-owned admission decides whether submitted work can become scheduler
  work;
- the scheduler drains eligible work in deterministic order under explicit
  conflict and footprint rules;
- admitted data-only operation programs are interpreted by Echo; legacy
  compatibility callbacks run only during scheduler-owned ticks but do not, by
  callback binding alone, prove deterministic application consequences;
- every committed tick emits receipt evidence that can be replayed and checked.

Echo deterministically controls its runtime judgments and evidence. End-to-end
deterministic consequences additionally require every executed semantic
implementation to satisfy its declared execution contract. The
executable-operation corridor makes that requirement structural by deriving
the consequence from admitted data-only program bytes; the callback-shaped
compatibility corridors do not yet provide the same proof.

`dispatch_intent(...)` is ingress. It is not "run this now." A host may run Echo
on a fixed wall-clock cadence or in an until-idle loop, but that cadence is
trusted runtime policy. The semantic tick is a logical scheduler-owned
coordinate, not application timing.

When a tick is attempted, Echo treats it as failure-atomic scheduler work:
lawful conflicts or obstructions become receipt evidence, while internal
runtime faults roll back uncommitted writes and quarantine the affected lane
instead of silently retrying forever.

## What Echo Owns vs. What You Own

Echo owns:

- causal history, frontiers, and runtime coordinates;
- admission, scheduling, ticks, and settlement;
- receipts, witnesses, reading envelopes, and retained artifacts;
- bounded observation machinery;
- generic contract hosting and suffix import/export surfaces.

You own:

- domain semantics;
- product policy and UI;
- authored application contracts, including Edict semantic sources and
  supported Wesley GraphQL contracts;
- generated contract helpers and host integrations.

## FAQ

### Is This Just Git?

Close enough to be useful, but not close enough to be correct. Git got the
most important thesis right: history is the authority. A Git commit is not a
diff applied to mutable state. It is a node in a witnessed causal graph that can
be replayed, audited, and verified.

Git fixes three things that Echo generalizes.

First, Git fixes the reading. Its causal history is over filesystem snapshots,
and its normal optic is "a directory of files at a commit." Echo's causal
history is over arbitrary admitted transitions. A reading might be a document,
a counter, a game state, a build artifact, or a semantic projection of a
GraphQL contract. The optic is authored. Git's optic is fixed.

Second, Git does not have runtime admission law. Anything can be committed.
Merge conflicts are surfaced to a human who resolves them outside the system.
There is no footprint check, deterministic independent subset, or receipt that
proves the merge was lawful under a named rule. Echo admits intents under
explicit law, checks footprints, and emits machine-verifiable receipts.

Third, Git reads are not structured observations. Checking out a commit
materializes files, but the checkout does not carry an observer, aperture,
basis, support posture, or witness envelope. Echo's readings name their
coordinate, law, basis, aperture, and evidence posture.

The one-line version: Git is a WARP optic with one aperture, no runtime
admission law, and no structured reads. It proved the thesis. WARP generalized
it.

### Is This Git For Anything?

No. "Git for anything" sounds like a storage layer with a broader substrate,
which misses the core point. The substrate difference is not the interesting
part. Admission law is.

Git records that someone committed something. Echo asks whether a transition is
lawful at a frontier under explicit rules. The scheduler proves the admitted
subset is independent before applying it. It emits a receipt that says what was
admitted, what was rejected or obstructed, and why. That lawfulness lives inside
the runtime instead of being assumed outside it.

The second missing piece is structured observation. Git's read is checkout.
Echo's read names a coordinate, a law, an aperture, and the support that must
travel with the result. The reading is bounded and can be wrong in detectable
ways.

So this is not a logging framework with branches. Echo is a runtime where
lawfulness is a first-class runtime property.

### Is This A Blockchain?

No. Blockchains and Echo both use append-only witnessed history. That is the
overlap.

Blockchains are built to solve Byzantine consensus: getting parties who do not
trust each other to agree on a shared ledger without a central authority. Proof
of work, proof of stake, global replication, and economic incentives all follow
from that problem.

Echo assumes cooperative participants under known law. There is no token, no
mining, no global validator set, and no adversarial consensus layer. Echo
witnesses are evidence that a transition followed from a named basis under a
named rule, not proof that anonymous validators agreed.

A blockchain's history is primarily a trust protocol. Echo's history is a
computation substrate for replayable, verifiable work.

### Is This Event Sourcing?

Event sourcing appends domain events and rebuilds state by replaying them.
That is a close cousin, but the authority is still usually a privileged
aggregate or materialized object. Replay is a recovery strategy.

Echo does not treat the log as a backup for an aggregate. Causal history is the
territory. Readings are derived under explicit observation law, and transitions
are admitted under explicit admission law. Receipts and witnesses are part of
the runtime contract, not an audit feature bolted on later.

### Is This The Actor Model?

No. Actors pass messages and mutate private state. That is still a mutable
state-machine model. Echo intents may look like messages, but they are not
fire-and-forget calls into private mutable objects. They are admitted under law,
scheduled by the runtime, witnessed, retained, and observed through bounded
readings.

### Is This A Database With Extra Steps?

No. A database makes the current value authoritative and exposes query and
transaction interfaces around it. History, if present, is often an audit log.

Echo makes witnessed causal history authoritative. The "current value" is a
reading derived from that history. That difference changes the guarantees under
failure, concurrency, replay, and debugging.

Use Postgres if you need a mutable relational store with transactions and
queries. Echo is appropriate when exact replay, cross-participant verification,
deterministic parallel admission, or evidence-carrying readings are the point.

### Are Witnesses And Receipts Just Logs?

No. A log line records that code said something. It does not prove that an
operation was lawful.

A receipt in Echo is machine-checkable outcome evidence. It names the candidate
work, the scheduler decision, the causal basis, the rule family, and the
outcome. Witnesses and retained artifacts are part of the evidence graph that
lets a later observer verify where a transition or reading came from.

### Are Readings Just Queries?

No. A normal query asks for the current value of something in a privileged
state store. It usually has no coordinate, aperture, witness basis, rights
posture, residual posture, or proof obligation.

An Echo reading is a lawful, bounded, witnessed observation. It names where it
was taken from, what observer and projection law produced it, what budget or
rights applied, and what evidence travels with the result.

### Does Determinism Mean Single-Threaded?

No. Determinism means the same admitted causal inputs produce the same
observable outputs. Echo can admit independent work in parallel when the
declared footprints prove that the work is safe to run together. Determinism is
an input discipline and admission-law problem, not a mandatory global lock.

### Is This Only For Distributed Systems?

No. Echo's predecessor was a deterministic game-engine runtime. Receipts,
footprint-checked parallelism, replayable ticks, and witnessed readings are
valuable in a single process on one machine. Distribution becomes easier
because the model is explicit about history and evidence, but distribution is
not the reason the model exists.

### How Does This Relate To CRDTs?

CRDTs solve convergence: replicas that receive the same operations eventually
converge. Echo solves lawfulness: transitions are admitted under explicit rules
with evidence. The ideas are complementary. A CRDT merge strategy could be
expressed as an admission law hosted by Echo.

### What Is The Boundary With Normal Tools?

Normal tools should stay normal. WARPDrive is the intended filesystem boundary:
a mounted path looks like a directory to editors, shells, IDEs, and formatters.
Reads are materialized readings. Writes become candidate intents. Echo remains
authoritative underneath without requiring every tool to learn Echo's model.

### Is Echo Production-Ready?

No. See [Current Reality](#current-reality). The kernel and several evidence
surfaces are working, but the current target is a usable local deterministic
contract host, not a general production platform.

### Who Is Echo For?

Echo is for runtime engineers, compiler authors, tool builders, and systems
architects who have hit the wall on mutable-state models and need exact replay,
lawful admission, or evidence-carrying observations. It is not a drop-in web
backend, a CRUD framework, or a better database.

### How Do I Debug With Echo?

Debugging is forensics over evidence. Because admitted transitions produce
receipts and readings carry witness posture, you can ask how a reading became
possible and trace it through named causal evidence instead of searching logs
for coincidental text.

### How Does Schema Evolution Work?

Contract identity, schema identity, operation ids, and generated helper or
package identity are explicit in admitted compiler output. A schema change
produces new contract artifact identity. Old readings name the schema and
operation identity they used, so multiple contract versions can coexist
without silently invalidating old receipts.

### Where Should Contributors Start?

Start with `warp-core` and read
[There Is No Graph](docs/architecture/there-is-no-graph.md) before changing
runtime boundaries. Echo core must not grow application nouns such as
`increment_counter`, `save_buffer`, or product-specific APIs. Those belong in
authored application contracts and generated adapters above the runtime
boundary.

## Quick Start For Contributors

```bash
make hooks
cargo xtask hello-echo
cargo xtask test-slice warp-core-smoke
cargo xtask dind run
```

Live work, priorities, and status are maintained in GitHub Issues, Projects,
pull requests, and review threads. Architectural decisions live in
[`docs/adr/`](docs/adr/), while current doctrine lives in
[`docs/topics/`](docs/topics/).

## Benchmarks And Gates

Echo treats determinism and performance as executable claims, not aspirations.
CI includes deterministic math guards, materialization determinism, DIND replay
checks, decoder security tests, reproducible WASM builds, rustdoc warnings,
clippy lanes, and criterion-based performance regression gates.

Scheduler benchmarks live in
[Scheduler Performance](docs/benchmarks/scheduler-performance-warp-core.md).
Run them locally with:

```bash
cargo bench -p warp-benches
```

## Key Crates

- `warp-core`: deterministic runtime kernel
- `echo-wasm-abi`: public ABI and wire DTOs
- `echo-wesley-gen`: Wesley contract helper generator
- `echo-cas`: content-addressed retention and semantic lookup

## Onramps

- Building applications:
  [Application Contract Hosting](docs/architecture/application-contract-hosting.md)
- Understanding the model:
  [There Is No Graph](docs/architecture/there-is-no-graph.md)
- Core runtime details: [warp-core spec](docs/spec/warp-core.md)
- Causal transport: [Continuum Transport](docs/architecture/continuum-transport.md)
- Documentation map: [Docs](docs/README.md)

## Current Reality

Echo has a working deterministic kernel, installed contract hosting, witnessed
intent submission, scheduler-owned execution, observation envelopes, semantic
retention, suffix transport surfaces, and playback tooling.

Built by [FLYING ROBOTS](https://github.com/flyingrobots).
