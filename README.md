<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

<p align="center">
  <img alt="ECHO" src="https://github.com/user-attachments/assets/bef3fab9-cfc7-4601-b246-67ef7416ae75" />
</p>

<p align="center">
  <strong>A deterministic WARP runtime for witnessed causal history, bounded observation, and graph-shaped readings</strong>
</p>

<p align="center">
  <a href="docs/index.md">Docs</a> •
  <a href="docs/architecture/outline.md">Architecture</a> •
  <a href="CONTINUUM.md">Continuum</a> •
  <a href="docs/spec/warp-core.md">warp-core</a> •
  <a href="https://github.com/flyingrobots/aion">AIΩN Framework</a>
</p>

<p align="center">
    <a href="https://github.com/flyingrobots/echo/actions/workflows/determinism.yml" ><img src="https://github.com/flyingrobots/echo/actions/workflows/determinism.yml/badge.svg" alt="Determinism CI" /></a>
    <a href="https://github.com/flyingrobots/echo/actions/workflows/ci.yml" ><img src="https://github.com/flyingrobots/echo/actions/workflows/ci.yml/badge.svg" alt="CI" /></a>
    <img src="https://img.shields.io/badge/platforms-Linux%20%7C%20macOS%20%7C%20Windows-blue" alt="Platforms" />
</p>

Echo is the hot runtime layer for WARP. It admits causal work into witnessed
history, lowers bounded optics into outcomes, witnesses, and retained shells,
emits reading artifacts for observers, and preserves enough structure for
deterministic replay, settlement, import, and inspection.

Echo still uses graph structures where they are the right carrier. The stronger
doctrine is that a graph is not the only substrate truth. Graph-shaped structure
is often an observer-relative reading over witnessed causal history.

---

## The Problem With Your Runtime

You model state as a hierarchy of mutable containers. You manage concurrency
through locks, mutexes, and prayer. You "debug" by scattering print statements
across a non-deterministic execution that will never reproduce the same way
twice. When something goes wrong, you squint at logs and guess.

Echo moves the hard part to witnessed admission.

Kernel history is immutable once admitted. Parallel work lowers through bounded
optics, emits private deltas, and merges only when the runtime can prove the
claims are lawful. There is nothing to lock inside the admitted history, and
there is no mystery transition: every accepted change carries witness material
that can be inspected later.

---

## WARP Runtime Model

WARP means **W**orldline **A**lgebra for **R**ecursive **P**rovenance.

The runtime model is:

- **Witnessed causal history** is the semantic substrate.
- **Graph-shaped readings** are observer-relative views over that history.
- **Optics** define bounded lowering, admission, witness, and retention.
- **Observers** define what a read can project, preserve, accumulate, and emit.
- **Shells** are retained carriers that make replay, audit, transport, or
  revelation possible.

Echo can materialize graphs, snapshots, checkpoints, and deltas as runtime
carriers. Those carriers are useful, but they are not the whole ontology. The
durable question is: what was admitted, under what law, with what witness, and
what can a bounded observer lawfully read from it?

---

## How Admission Evolves

When Echo ticks, it does not mutate a canonical global object. Work is lowered
through an optic:

```text
Optic = (ObserverPlan, OpticSlice, LoweringSurface, AdmissionLaw, RetentionContract)

Lower(frontier, weave) = (Outcome, Witness, Shell)

Outcome(X) = Derived(X) | Plural(X) | Conflict | Obstruction
```

DPO-style graph rewriting is one concrete lowering pattern Echo uses. The
larger runtime contract is more general:

- **Outcome** says what was admitted or why admission did not collapse to a
  single value.
- **Witness** carries the evidence needed to review the judgment.
- **Shell** preserves the carrier required for replay, audit, transport, or
  later reading.

Materialized state still matters. It is a cache, checkpoint, shell, or reading
surface. It is not the only source of truth.

## Determinism by Construction

Echo achieves parallelism without making scheduler timing semantically visible.
Runtime rules read from an immutable basis and write to private deltas. Deltas
merge in canonical order only after footprint and admission checks pass. The
result is identical whether the host runs 1 thread or 32.

This is not "mostly deterministic." It is 0-ULP deterministic:

- Bare host floats are banned from deterministic kernel semantics. Echo uses
  fixed-point or otherwise platform-invariant scalar surfaces.
- System time is banned. Simulation time is a causal property of the worldline,
  not a wall-clock reading.
- Unseeded randomness is banned. If a tick uses randomness, the seed is part of
  the admitted input.

The same tick hash on Linux, macOS, and Windows. The same tick hash today, next
year, and on hardware that doesn't exist yet. Given the same admitted input, the
kernel output is not just likely identical. It is required to converge.

**Footprint enforcement** is the mechanism that makes this survive parallelism.
Optics declare their bounded regions. The scheduler proves independence. Any
delta that violates its declared contract is poisoned: not patched, not retried,
but structurally rejected.

---

## Observation and Reading Artifacts

Observation is not a passive state query.

The read side is modeled as:

```text
StructuralObserver = (Projection, ObserverBasis, ObserverState, UpdateLaw, EmissionLaw)
```

Echo distinguishes:

- **ObserverPlan**: the authored or compiled revelation discipline.
- **ObserverInstance**: a runtime observer plus accumulated state when
  observation is stateful.
- **ReadingArtifact**: the emitted result, with coordinate, payload, witness,
  basis, budget, rights, and residual posture.

The current WASM ABI wraps observations in `ReadingEnvelope` metadata so host
tools can see how a read was resolved. A live strand read can report parent
basis posture. A bounded observer can receive a lawful reading without
pretending it saw the whole kernel state.

This is the practical version of Echo's holography. Witnessed provenance,
retained shells, checkpoints, and reading envelopes let tools reconstruct prior
readings and causal slices according to explicit retention contracts. Time
travel is not a debugger bolted on afterward. It is a read discipline over
witnessed history.

---

## Settlement, Strands, and WARP

Echo is the engine layer of a larger WARP stack: a recursive witnessed
admission architecture that governs the transition from private speculation to
shared causal reality.

The important runtime separations are:

- **Commit / Lower**: judge bounded claims and produce an outcome, witness, and
  shell.
- **Fold / Retain**: preserve what replay, audit, transport, or revelation
  requires.
- **Reveal / Observe**: emit an observer-relative reading under aperture, basis,
  budget, and rights.
- **Settle**: compare speculative or remote claims against a live basis and
  return import, plurality, conflict, or obstruction.

Strands are live speculative lanes, not just frozen snapshots. Settlement uses
live-basis reports, parent movement, overlap checks, target-local replay, and
conflict artifacts to decide whether a suffix can be imported cleanly. Remote
exchange is witnessed suffix admission, not state sync.

---

## Current Runtime Surfaces

| Surface                       | Current role                                                                 |
| ----------------------------- | ---------------------------------------------------------------------------- |
| **`warp-core`**               | Hot runtime semantics: worldlines, strands, observation, settlement, ticks.  |
| **`ObservationService`**      | Canonical read path that emits observation artifacts and reading posture.    |
| **`SettlementService`**       | Strand comparison, planning, import candidates, and conflict artifacts.      |
| **`NeighborhoodSiteService`** | Witness-bearing neighborhood/site publication surface.                       |
| **`echo-wasm-abi`**           | Current ABI DTOs and canonical CBOR boundary.                                |
| **`warp-wasm`**               | wasm-bindgen host/browser wrapper over the current ABI.                      |
| **`method` / `xtask`**        | Filesystem Method workflow automation, including `cargo xtask method inbox`. |

The current WASM ABI version is **6**. That number is a compatibility epoch for
host/runtime mismatch detection, not a support promise for ABI v1 through v5.
The public world-state read export is `observe(...)`; the write/control ingress
is `dispatch_intent(...)`; scheduler metadata comes through
`scheduler_status()`.

Removed or intentionally absent public hooks such as `step(...)`,
`snapshot_at(...)`, and `render_snapshot(...)` are not the current boundary.

---

## In Short

| Property                                | How                                                                 |
| --------------------------------------- | ------------------------------------------------------------------- |
| **Parallelism without synchronization** | Immutable bases, private deltas, canonical merge, footprint checks. |
| **0-ULP cross-platform determinism**    | Platform-invariant math, logical time, seeded randomness.           |
| **Witnessed admission**                 | Every accepted transition carries reviewable evidence.              |
| **Bounded optics**                      | Lowering produces outcome, witness, and retained shell.             |
| **Observer-relative reads**             | Reading artifacts carry coordinate, basis, witness, and posture.    |
| **Live settlement**                     | Strands and suffixes settle against live basis evidence.            |
| **Method workflow**                     | Current work is tracked as live operational Markdown plus `xtask`.  |

## Quick Start

### 1. Repository Setup

Install the guardrails and verify the current Method view.

```bash
make hooks
cargo xtask method status --json
```

### 2. Run a Fast Runtime Slice

Use the narrow test-slice path for local iteration.

```bash
cargo xtask test-slice warp-core-smoke
```

### 3. Keep Docs as a Gate

The docs build is a real regression gate.

```bash
pnpm docs:build
```

### 4. Run the Deeper Determinism Harness

Use DIND when you need cross-platform hash convergence evidence.

```bash
cargo xtask dind run
```

## Stack

| Component           | Role                                                                 |
| :------------------ | :------------------------------------------------------------------- |
| **`warp-core`**     | Hot runtime kernel for worldlines, strands, observation, settlement. |
| **`echo-wasm-abi`** | ABI v6 DTOs, canonical CBOR envelopes, host/runtime contract.        |
| **`warp-wasm`**     | wasm-bindgen boundary for browser and JavaScript tooling.            |
| **`method`**        | Method workflow automation and backlog file generation.              |
| **`warp-cli`**      | Native CLI inspection and verification surface.                      |
| **`echo-cas`**      | Content-addressed storage substrate.                                 |
| **`echo-ttd`**      | Time-travel/debugging protocol surfaces.                             |
| **`ttd-browser`**   | Browser-hostable TTD/runtime bridge surfaces over Echo WASM.         |
| **`echo-dind-*`**   | Cross-platform harness for hash convergence verification.            |
| **`echo-app-core`** | Application lifecycle, orchestration, and effect pipelines.          |

## Documentation

- **[Docs](./docs/index.md)**: Live docs map for runtime, replay, observation, and determinism.
- **[Bearing](./docs/BEARING.md)**: Current repo bearing and near-term priorities.
- **[Architecture](./docs/architecture/outline.md)**: Architecture map and layer model.
- **[WARP Drift](./docs/architecture/WARP_DRIFT.md)**: Current doctrine corrections around strands, observation, and suffix admission.
- **[Optic and Observer Doctrine](./docs/design/0011-optic-observer-runtime-doctrine/design.md)**: Runtime noun stack for optics, observers, witnesses, shells, and readings.
- **[WASM ABI](./docs/spec/SPEC-0009-wasm-abi.md)**: Current ABI v6 contract.
- **[Method](./docs/method/README.md)**: Operational workflow and backlog automation.
- **[DIND](./docs/determinism/dind-harness.md)**: Determinism verification and the "Drill Sergeant" discipline.
- **[Theory](./docs/theory/THEORY.md)**: Theoretical foundations.
- **[Continuum](./CONTINUUM.md)**: The multi-repo system model and hot-runtime role.

---

<p align="center">
  <sub>Built by <a href="https://github.com/flyingrobots">FLYING•ROBOTS</a></sub>
</p>
