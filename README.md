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



# What is Echo?

Echo is a Rust implementation of the [WARP](https://github.com/flyingrobots/aion) (Worldline Algebra for Recursive Provenance) architecture.

Traditional applications model state as a hierarchy of mutable containers, relying on locks and mutexes to manage concurrency. This approach leads to non-deterministic execution, making bugs difficult to reproduce and debug.

**Echo fundamentally changes this model:** Instead of mutating a global state, Echo treats **witnessed causal history** as the ultimate source of truth. Graph-shaped structures are treated merely as observer-relative *views* over that history, rather than the core reality. Once kernel history is admitted, it is immutable. Parallel work is handled via private deltas that merge only when mathematically proven to be lawful, eliminating the need for runtime locks.

# At a Glance

| Feature | How Echo Achieves It |
|---|---|
| **Lock-Free Parallelism** | Immutable bases, private deltas, canonical merging, and strict footprint checks. |
| **0-ULP Determinism** | Platform-invariant math, logical (not system) time, and seeded randomness. |
| **Witnessed Admission** | Every accepted state transition carries reviewable cryptographic evidence. |
| **Bounded Optics** | State modifications (lowering) produce an explicit outcome, a witness, and a retained shell. |
| **Observer-Relative Reads** | Data reads carry coordinates, basis info, witnesses, and context. |
| **Live Settlement** | Speculative paths (strands) are settled against live evidence before merging. |

# Core Architecture

## The WARP Runtime Model

Echo relies on a specific set of concepts to manage state and history:

* **Witnessed Causal History:** The immutable, underlying semantic truth of the system.
* **Graph-Shaped Readings:** Filtered, observer-relative views projected from causal history.
* **Optics:** The rules defining how changes are lowered, admitted, witnessed, and retained.
* **Observers:** The rules defining what a read operation can project, preserve, accumulate, and output.
* **Shells:** Retained data packages that enable deterministic replay, auditing, network transport, and state revelation.

Materialized state in Echo is just a cache, checkpoint, or reading surface—never the definitive source of truth. The only thing that truly matters is what was admitted, the laws governing it, the witness that proves it, and what an observer is allowed to read from it.

## How State Evolves (Admission)

When Echo steps forward (ticks), it does not mutate a global object. Instead, work is evaluated through an **Optic**:

```text
Optic = (ObserverPlan, OpticSlice, LoweringSurface, AdmissionLaw, RetentionContract)

Lower(frontier, weave) = (Outcome, Witness, Shell)

Outcome(X) = Derived(X) | Plural(X) | Conflict | Obstruction

```

* **Outcome:** Determines whether the change was admitted or why it failed.
* **Witness:** Provides the evidence required to audit the decision.
* **Shell:** Packages the data required for future replays or reads.

# Observation and Artifacts

In Echo, observation is an active, structured process, not a passive query.

```text
StructuralObserver = (Projection, ObserverBasis, ObserverState, UpdateLaw, EmissionLaw)

```

Observations yield a `ReadingArtifact` containing the payload, coordinates, basis, budget, and witness. The WASM ABI (currently v6) wraps these in a `ReadingEnvelope` so host tools understand exactly how a read was resolved. This "holographic" approach allows tools to seamlessly reconstruct prior states or causal slices without bolting on an external debugger.

# Determinism by Construction

Echo achieves exact, cross-platform reproducibility (0-ULP determinism). The kernel output will be identical whether running on 1 thread or 32, across Linux, macOS, or Windows, today or ten years from now.

To enforce this, Echo strictly bans:

* **Bare host floats:** All math uses fixed-point or platform-invariant scalars.
* **System wall-clock time:** Simulation time is an intrinsic property of the worldline.
* **Unseeded randomness:** Any tick utilizing randomness must include the seed as part of the admitted input.
* **Footprint enforcement** ensures parallelism remains deterministic. Optics declare bounded regions; the scheduler proves independence. Any proposed delta that violates its contract is structurally rejected—never patched or retried.

# Runtime Surfaces & Stack

Echo serves as the engine layer governing the transition from private speculation (strands) to shared causal reality.

# Core Components

| Component | Role |
|---|---|
| **warp-core** | Hot runtime kernel handling worldlines, strands, observation, and settlement. |
| **echo-wasm-abi** | Current ABI v6 DTOs and canonical CBOR boundary. *Note: v6 is a compatibility epoch, not a promise of support for v1-v5.* |
| **warp-wasm** | wasm-bindgen boundary for browser and JavaScript environments. |
| **warp-cli** | Native CLI for inspection and verification. |
| **ObservationService** | Canonical read path emitting observation artifacts. |
| **SettlementService** | Handles strand comparison, import candidates, and conflicts. |
| **echo-cas** | Content-addressed storage substrate. |
| **echo-ttd & ttd-browser** | Time-travel/debugging protocol surfaces and their browser bridges. |
| **echo-dind-*** | Cross-platform harness for verifying hash convergence. |

# Quick Start

## 1. Repository Setup

Install the necessary guardrails and verify the current operational Method view.

```bash
make hooks
cargo xtask method status --json

```

## 2. Run a Fast Runtime Slice

Execute the narrow test-slice path for rapid local iteration.

```bash
cargo xtask test-slice warp-core-smoke

```

## 3. Build Documentation

The documentation build serves as an active regression gate.
```bash
pnpm docs:build

```

## 4. Run the Determinism Harness

Use Docker-in-Docker (DIND) to verify cross-platform hash convergence.

```bash
cargo xtask dind run

```

# Documentation Directory

* **Docs**: Main documentation map (runtime, replay, observation).
* **Bearing**: Repository direction and near-term priorities.
* **Architecture**: System architecture and layer model.
* **WARP Drift**: Adjustments regarding strands and suffix admission.
* **Optic & Observer Doctrine**: Core definitions for runtime nouns.
* **WASM ABI (v6)**: The active host/runtime contract.
* **Method**: Operational workflow and backlog automation rules.
* **DIND**: Instructions for the determinism testing harness.
* **Theory**: Theoretical foundations of the WARP model.
* **Continuum**: The multi-repository system model.

---

<p align="center">
<sub>Built by <a href="[https://github.com/flyingrobots](https://github.com/flyingrobots)">FLYING•ROBOTS</a></sub>
</p>
