<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
<!-- markdownlint-disable MD033 -->

<p align="center">
<img alt="ECHO" src="https://github.com/user-attachments/assets/bef3fab9-cfc7-4601-b246-67ef7416ae75" />
</p>

<h3 align="center">
  <strong>A deterministic WARP runtime for witnessed causal history, bounded optics, and holographic readings.</strong>
</h3>

<p align="center">
<a href="docs/index.md">Docs</a> •
<a href="docs/architecture/outline.md">Architecture</a> •
<a href="docs/architecture/there-is-no-graph.md">There Is No Graph</a> •
<a href="docs/architecture/wsc-verkle-ipa-retained-readings.md">WSC / Verkle / IPA</a> •
<a href="docs/spec/warp-core.md">warp-core</a>
</p>

<p align="center">
<a href="https://github.com/flyingrobots/echo/actions/workflows/determinism.yml"><img src="https://github.com/flyingrobots/echo/actions/workflows/determinism.yml/badge.svg" alt="Determinism CI" /></a>
<a href="https://github.com/flyingrobots/echo/actions/workflows/ci.yml"><img src="https://github.com/flyingrobots/echo/actions/workflows/ci.yml/badge.svg" alt="CI" /></a>
<img src="https://img.shields.io/badge/platforms-Linux%20%7C%20macOS%20%7C%20Windows-blue" alt="Platforms" />
</p>

# Echo

Echo is the hot runtime optic in the WARP stack.

It does not treat a graph, database, file tree, editor buffer, or in-memory
object heap as the ultimate truth. Echo's substrate is witnessed causal history:
admitted transitions, frontiers, receipts, witnesses, patches, checkpoints,
retained readings, and boundary artifacts.

The hard doctrine is:

```text
There is no privileged graph.
There are causal histories and lawful readings of those histories.
```

Echo turns that doctrine into runtime machinery.

It admits canonical intents, schedules deterministic work, settles speculative
paths, emits evidence-bearing receipts, serves bounded observations, and retains
the artifacts needed to replay or verify what happened. Graph-shaped state is a
reading. Files are readings. Build outputs are readings. Debugger views are
readings. Echo exists to make those readings lawful, witnessed, and
replayable.

## Why It Exists

Traditional systems pretend there is one mutable global state:

```text
program + state -> mutated state
```

That model leaks. It turns concurrency into locks, collaboration into merge
pain, debugging into archaeology, and generated artifacts into "trust me, this
script probably ran."

Echo follows the WARP model instead:

```text
causal basis + optic law + support obligations -> witnessed reading

reading + intent + admission law -> witnessed suffix

witnessed suffix + optic -> new reading
```

The result is not "no state." State-like values still exist everywhere. The
difference is authority: materialized state is a chart, cache, viewport, or
hologram. It is not the territory.

## Core Ontology

| Concept            | Meaning in Echo                                                                                                                       |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------- |
| **Causal history** | The witnessed substrate: admitted transitions, frontiers, receipts, witnesses, and retained boundary artifacts.                       |
| **WARP optic**     | A bounded, law-named operation over causal history. It may admit, observe, retain, reveal, import, or materialize.                    |
| **Reading**        | An observer-relative artifact emitted from a coordinate, aperture, and projection law.                                                |
| **Hologram**       | A witnessed output carrying enough basis, law, aperture, evidence, identity, and posture to recreate the claim at its declared level. |
| **Witness**        | Evidence that a transition or reading followed from a named basis under a named law.                                                  |
| **Shell**          | A retained boundary artifact such as a tick patch, suffix bundle, provenance payload, or checkpoint base.                             |
| **ReadIdentity**   | The semantic question a retained payload answers. It is intentionally separate from the CAS byte hash.                                |

The front-door architecture note is
[There Is No Graph](docs/architecture/there-is-no-graph.md).

## What Echo Owns

Echo owns the generic hot runtime path:

- canonical intent ingress;
- deterministic scheduling and footprint checks;
- rewrite settlement;
- worldline and provenance retention;
- replayable tick patches;
- Merkle commitments over state and patch boundaries;
- observation artifacts and `ReadingEnvelope` metadata;
- WASM/session boundaries for browser and host integration;
- `echo-cas` retention for bytes, witnesses, receipts, and cached readings.

Echo does **not** own application nouns.

Names like `ReplaceRange`, `JeditBuffer`, `CounterIncrement`,
`RenameSymbol`, or `GraftProjection` belong in authored contracts,
Wesley-generated code, application adapters, or fixtures. They must not become
Echo substrate APIs.

## WARP Runtime Flow

Echo's hot path is deliberately boring:

1. External callers submit canonical intent bytes.
2. Inbox sequencing derives content identity and canonical pending order.
3. Rules propose candidate rewrites with explicit footprints.
4. The scheduler admits a deterministic independent subset.
5. The engine applies admitted rewrites.
6. Echo emits receipts, tick patches, provenance, and hashes.
7. Observation services resolve coordinates and return readings.
8. Retention stores the bytes and witness material needed for replay or
   obstruction.

The point is not to mutate a global graph. The point is to admit and observe
witnessed causal structure through explicit laws.

## Application Contracts

Applications talk to Echo through generated contracts, not app-specific runtime
APIs.

The current shape is:

```text
Application UI / adapter
  -> Wesley-generated contract client
  -> canonical operation variables
  -> EINT intent bytes
  -> Echo dispatch_intent(...)
  -> Echo causal admission and receipts
  -> Echo observe(...)
  -> ReadingEnvelope + payload bytes
  -> generated/application decoding
  -> UI
```

This is why a serious text editor such as `jedit` can own its rope model,
buffer law, edit-group law, checkpoint policy, and UI behavior while Echo stays
generic. Echo hosts the generated contract, verifies artifact metadata, admits
intents, emits readings, and retains bytes. It does not become a text editor.

See [Application Contract Hosting](docs/architecture/application-contract-hosting.md).

## Retained Readings: WSC, Verkle, IPA, CAS

Echo's retained-reading direction is:

```text
WSC   = canonical columnar bytes for a reading or checkpoint
Verkle = authenticated commitment/index over those bytes
IPA   = compact proof mechanism for opening bounded apertures
echo-cas = content-addressed byte retention
```

Short version:

```text
WSC gives us the table.
Verkle gives us the root.
IPA gives us the aperture proof.
echo-cas stores the bytes.
```

Current reality:

- `warp-core` has WSC writing, validation, and borrowed view support.
- `echo-cas` stores opaque bytes by `BLAKE3(bytes)`.
- retained reading identity is intentionally separate from CAS byte identity.

Future direction:

- WSC-backed retained readings and checkpoints;
- Verkle or equivalent authenticated indexes over WSC coordinates;
- IPA or equivalent compact opening proofs for proof-carrying apertures;
- bounded reads that can verify selected rows, chunks, or ranges without
  materializing the full retained reading.

This is future proof infrastructure, not a new ontology. WSC is not truth.
Verkle is not truth. IPA is not storage. CAS is not semantic identity.

See [WSC, Verkle, IPA, And Retained Readings](docs/architecture/wsc-verkle-ipa-retained-readings.md).

## Determinism Posture

Echo is built around exact replay and cross-platform convergence.

The runtime treats nondeterminism as an input discipline problem:

- no ambient wall-clock time in admitted simulation law;
- no unseeded randomness inside ticks;
- platform-sensitive math is pinned behind deterministic representations;
- canonical CBOR is used at ABI boundaries;
- footprint declarations constrain parallel work;
- receipts and patches carry the evidence needed for replay and audit.

The slogan is not "parallelism is safe because we hope so." The rule is:

```text
parallel work is admitted only when the runtime can prove the admitted subset
is lawful for the current basis.
```

## Core Crates

| Crate                      | Role                                                                                                     |
| -------------------------- | -------------------------------------------------------------------------------------------------------- |
| `warp-core`                | Hot runtime kernel: worldlines, scheduling, settlement, observation, WSC, receipts, and core WARP state. |
| `echo-wasm-abi`            | Canonical host/runtime DTOs, `KernelPort`, canonical CBOR helpers, observation and dispatch surfaces.    |
| `warp-wasm`                | Browser/JavaScript boundary around the runtime kernel.                                                   |
| `warp-cli`                 | Native CLI for WSC inspection, validation, and runtime support tooling.                                  |
| `echo-registry-api`        | Minimal generic registry boundary for generated application contracts.                                   |
| `echo-wesley-gen`          | Wesley-to-Echo Rust generator for generated DTOs, op ids, registry metadata, and contract helpers.       |
| `echo-cas`                 | Content-addressed byte store. It stores bytes; typed identity lives above it.                            |
| `echo-ttd` / `ttd-browser` | Time-travel/debugging protocol surfaces and browser bridges.                                             |
| `echo-dind-*`              | Cross-platform determinism harnesses and evidence tooling.                                               |

## Quick Start

Install hooks and check the current Method view:

```bash
make hooks
cargo xtask method status --json
```

Run a fast runtime slice:

```bash
cargo xtask test-slice warp-core-smoke
```

Build the docs:

```bash
pnpm docs:build
```

Run the determinism harness:

```bash
cargo xtask dind run
```

Inspect a WSC snapshot:

```bash
export SNAPSHOT=/path/to/state.wsc

cargo run -p warp-cli -- inspect "$SNAPSHOT"
cargo run -p warp-cli -- inspect "$SNAPSHOT" --tree
cargo run -p warp-cli -- verify "$SNAPSHOT"
cargo run -p warp-cli -- --format json verify "$SNAPSHOT"
```

## Documentation Map

- [Docs index](docs/index.md)
- [Current bearing](docs/BEARING.md)
- [Runtime model](docs/architecture/outline.md)
- [There Is No Graph](docs/architecture/there-is-no-graph.md)
- [Application Contract Hosting](docs/architecture/application-contract-hosting.md)
- [WSC, Verkle, IPA, And Retained Readings](docs/architecture/wsc-verkle-ipa-retained-readings.md)
- [warp-core spec](docs/spec/warp-core.md)
- [WASM ABI contract](docs/spec/SPEC-0009-wasm-abi.md)
- [Theory map](docs/theory/THEORY.md)
- [Contributor workflow](docs/workflows.md)

---

<p align="center">
<sub>Built by <a href="https://github.com/flyingrobots">FLYING•ROBOTS</a>.</sub>
</p>
