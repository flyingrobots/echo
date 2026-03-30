<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

<p align="center">
  <img alt="ECHO" src="https://github.com/user-attachments/assets/bef3fab9-cfc7-4601-b246-67ef7416ae75" />
</p>

<p align="center">
  <strong>State is a graph. Time is a hash chain. Determinism isn't optional.</strong>
</p>

<p align="center">
  <a href="docs/guide/start-here.md">Get Started</a> •
  <a href="docs/architecture-outline.md">Architecture</a> •
  <a href="docs/index.md">Docs</a> •
  <a href="https://github.com/flyingrobots/aion">AIΩN Framework</a>
</p>

<p align="center">
    <a href="https://github.com/flyingrobots/echo/actions/workflows/determinism.yml" ><img src="https://github.com/flyingrobots/echo/actions/workflows/determinism.yml/badge.svg" alt="Determinism CI" /></a>
    <a href="https://github.com/flyingrobots/echo/actions/workflows/ci.yml" ><img src="https://github.com/flyingrobots/echo/actions/workflows/ci.yml/badge.svg" alt="CI" /></a>
    <img src="https://img.shields.io/badge/platforms-Linux%20%7C%20macOS%20%7C%20Windows-blue" alt="Platforms" />
</p>

---

## The Trick

Echo runs graph rewrites in parallel across all your cores. No mutexes. No
locks. No lock-free spinning. No CRDTs. No synchronization of any kind.

How? Every rule reads from an **immutable snapshot**. Every rule writes to
its own **private delta**. When all rules finish, the deltas merge in
**canonical order**. Same inputs, same hashes, same result&mdash;whether you
have 1 thread or 32.

```text
  ┌─ Rule A ──► private delta A ─┐
  │                               │
  Snapshot ─┤─ Rule B ──► private delta B ─├──► Canonical Merge ──► Commit ──► Hash
  │                               │
  └─ Rule C ──► private delta C ─┘

  Nothing is shared. Nothing is locked. Order doesn't matter.
```

The concurrency problem doesn't get solved. It gets
**structurally prevented from existing.**

Benchmark: 1M entities, 100 ticks, 10 cores. This approach matches Rayon's
optimized work-stealing thread pool at ~4.7x speedup&mdash;while also being
deterministic, which Rayon cannot offer.
([Parallelism study](docs/benchmarks/parallelism-study.md))

## How Independence Is Proven

Each rule declares a **footprint**&mdash;the graph regions it reads and
writes. The scheduler checks footprints before execution: if two rules
touch the same structure, they're serialized deterministically. If they
don't, they're provably independent and can run in any order with identical
results.

At runtime, footprint guards **enforce** the contract:

- **Reads** are checked via guarded graph views that reject undeclared access
- **Writes** are validated post-execution against the declared footprint
- **Violations** poison the delta&mdash;it cannot be committed

This isn't honor system. It's runtime proof.

## Cross-Platform Determinism

```text
$ cargo xtask dind run
[DIND] Running 50 seeds across 3 platforms...
[DIND] linux-x64:   7f3a9c...d82e1a ✅
[DIND] macos-arm64: 7f3a9c...d82e1a ✅
[DIND] windows-x64: 7f3a9c...d82e1a ✅

Hashes match. Determinism verified.
```

Same hashes. Every platform. Every thread count. Every run. Not
approximately&mdash;identical bytes, cryptographically verified.

Echo eliminates every known source of nondeterminism:

- **Float math**: LUT-based trig with 0-ULP golden vectors, no stdlib transcendentals
- **Iteration order**: `BTreeMap` everywhere, `HashMap` is [banned from core crates](scripts/ban-nondeterminism.sh)
- **Global state**: `OnceLock`, `lazy_static`, `thread_local`, `static mut` are all [banned](scripts/ban-globals.sh)
- **Time and entropy**: `SystemTime`, `Instant::now`, `rand::` are all [banned](scripts/ban-nondeterminism.sh)
- **Wire format**: [Canonical CBOR encoding](docs/SPEC_DETERMINISTIC_MATH.md), no platform-dependent serialization
- **ABI boundaries**: Unordered containers are [banned from wire-format code](scripts/ban-unordered-abi.sh)

The fast local nondeterminism ban runs in the **pre-commit hook**. The broader
ban suite, including the global-state and unordered-ABI guards above, is
enforced in CI. You cannot quietly merge nondeterministic code. The claim
register and the build reject it.

See [Determinism Claims v0.1](docs/determinism/DETERMINISM_CLAIMS_v0.1.md) for
the full claim register with CI gates and evidence artifacts.

## Time Travel Debugging

Every tick stores the full WARP state, the rewrite bundle (all legal moves),
the interference pattern, and the collapse decision. History isn't
reconstructed&mdash;it's **recorded by construction**.

- **Rewind**: step backward to any prior tick with perfect fidelity
- **Fork**: branch into a counterfactual worldline from any point
- **Diff**: overlay actual vs. alternative worldlines to find where they diverge

This is always available. You don't enable recording. You don't configure
tracing. Deterministic replay is a property of the architecture, not a
feature you turn on.

## What Is WARP?

WARP (Worldline Algebra for Recursive Provenance) is the graph algebra
underneath Echo. State is a **WARP graph**&mdash;a typed, directed graph
where nodes and edges can contain nested graphs. Change happens through
**DPO-inspired rewriting**: match a pattern, cut it out, glue in a
replacement along a typed interface.

That DPO/DPOI story is the **north-star semantics**, not a claim that
`warp-core` already ships a full categorical DPO engine. Today the runtime
enforces order-independence with conservative `Footprint`s and the scheduler's
reservation/conflict path; see [Theory](docs/THEORY.md) and
[SPEC-0003](docs/spec/SPEC-0003-dpo-concurrency-litmus-v0.md) for the intended
semantics versus the current pragmatic subset.

The combination gives you:

- **Immutable state**: graphs are never mutated in place; rewrites produce new graphs
- **Append-only history**: every tick is a cryptographic commit in a hash chain
- **Deterministic convergence**: independent rewrites commute under today's footprint conflict rules, with DPOI as the intended north star
- **Nested structure**: a node can contain an entire sub-universe (graphs all the way down)

> **Naming:** Echo is the product. WARP is the underlying algebra. The `warp-*`
> and `echo-*` crates are internal modules&mdash;same project, different layers.

## Project Status

> [!WARNING]
> **Echo is early. Sharp edges.**
>
> - **Stable:** Core determinism, hashing, replay invariants, parallel execution
> - **Changing:** Schema/IR, APIs, file formats, viewer protocol
> - **Not yet:** Nice UX, polished docs, batteries-included examples
>
> If you need a plug-and-play game engine today, this isn't that (yet).
> If you need deterministic, replayable state transitions you can prove, it is.

### Roadmap

1. **WARPSITE**&mdash;a website powered by WARP graph rewriting
2. **Splash Guy**&mdash;a demo game designed to introduce Echo concepts
3. **Tumble Tower**&mdash;a demo game designed to demonstrate Echo's physics determinism

See also: [Wesley](https://github.com/flyingrobots/wesley)&mdash;a GraphQL-to-Rust/TypeScript
schema compiler that will generate typed WARP schemas, footprints, and
deterministic serialization from a single source of truth.

## Quick Tour

```bash
# Install hooks (formats code, runs clippy, checks docs)
make hooks

# Run the test suite
cargo test --workspace

# Run determinism verification
cargo xtask dind run
```

Run `warp-core` with extra delta validation enabled:

```bash
cargo test -p warp-core --features delta_validate
```

```bash
# Launch the viewer
cargo run -p warp-viewer

# Build the docs site
make docs
```

## The Stack

**Core** &mdash; `crates/warp-core`

- Graph-rewrite engine with transactional commits
- Deterministic math (fixed-point, PRNG, Vec3/Mat4/Quat)
- Parallel execution via snapshot + private delta + canonical merge
- **Footprint guards**&mdash;runtime enforcement of declared read/write sets
- **Materialization bus**&mdash;order-independent output channels
- **WSC** (Write-Streaming Columnar)&mdash;zero-copy snapshot format for fast state reload + verification

**Pipeline** &mdash; `crates/echo-session-*`

- Unix socket hub with gapless diff streaming
- WebSocket gateway for browser tools
- Canonical CBOR wire format

**Tools** &mdash; `crates/warp-viewer`, `crates/echo-dind-*`

- Native GPU viewer with per-frame hash verification
- **DIND** (Determinism-in-Determinism)&mdash;cross-platform test harness that proves hash convergence

## Research Foundation

Echo implements ideas from the **AIΩN Foundations** paper series:

1. [WARP Graphs: A Worldline Algebra for Recursive Provenance](https://doi.org/10.5281/zenodo.17908005)
2. [Canonical State Evolution and Deterministic Worldlines](https://doi.org/10.5281/zenodo.17934512)
3. [Computational Holography & Provenance Payloads](https://doi.org/10.5281/zenodo.17963669)
4. [Rulial Distance & Observer Geometry](https://doi.org/10.5281/zenodo.18038297)

Part of the [AIΩN Framework](https://github.com/flyingrobots/aion).

## Reference

- [Architecture Outline](docs/architecture-outline.md) &mdash; design rationale and high-level system overview
- [Configuration Reference](docs/guide/configuration-reference.md) &mdash; engine parameters, protocol constants, environment variables
- [Cargo Feature Flags](docs/guide/cargo-features.md) &mdash; all compile-time features across the workspace
- [Deterministic Math Policy](docs/SPEC_DETERMINISTIC_MATH.md) &mdash; normative rules for IEEE 754 handling

## Contributing

Determinism is sacred. Before you change anything:

1. Read [`CONTRIBUTING.md`](CONTRIBUTING.md)
2. Run `make hooks` to install the guardrails
3. Write tests. If it's not tested, it's not deterministic.

The repo guardrails cover:

- No global state ([`ban-globals.sh`](scripts/ban-globals.sh))
- No wall-clock time or uncontrolled randomness ([`ban-nondeterminism.sh`](scripts/ban-nondeterminism.sh))
- No unordered iteration in wire-format code ([`ban-unordered-abi.sh`](scripts/ban-unordered-abi.sh))

`make hooks` installs the fast local subset. CI runs the broader determinism
ban suite before merge.

## Requirements

- **Rust** &mdash; pinned in `rust-toolchain.toml` (currently 1.90.0)
- **Node.js 18+** &mdash; for the docs site (VitePress)

## License

Dual-licensed under Apache 2.0 and MIND-UCAL 1.0. See [`LEGAL.md`](LEGAL.md) for details.

---

<p align="center">
  <sub>Built by <a href="https://github.com/flyingrobots">FLYING•ROBOTS</a></sub>
</p>
