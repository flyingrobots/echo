<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Advanced Guide — Echo

This is the second-track manual for Echo. Use it when you need the deeper doctrine behind the theoretical foundations (AION Foundations), Wesley schema integration, and internal Spec details.

For orientation and the productive-fast path, use the [GUIDE.md](./GUIDE.md).

## Theoretical Foundations

Echo implements ideas from the **AIΩN Foundations** paper series.

### WARP Graphs (OG-I)

A worldline algebra for recursive provenance. State is a finite directed multigraph where nodes and edges can contain nested graphs.

### Deterministic Convergence (OG-IV)

Independent rewrites commute under footprint conflict rules. State convergence does not imply provenance convergence—two worldlines can arrive at the same state through different histories.

### Rulial Distance & Observer Geometry

An observer is a structural five-tuple (Projection, Basis, State, Update, Emission). Echo surfaces what survives each layer of observation.

## Internal Specs

- **[warp-core spec](./docs/spec-warp-core.md)**: The transactional kernel and commit semantics.
- **[Tick Patch spec](./docs/spec-warp-tick-patch.md)**: The binary boundary for causal transitions.
- **[Merkle Commit](./docs/spec-merkle-commit.md)**: Snapshot hashing and state integrity.
- **[Deterministic Math](./docs/SPEC_DETERMINISTIC_MATH.md)**: Rules for 0-ULP cross-platform math.

## Wesley Integration

The simulation protocol and graph schemas are increasingly defined via Wesley.

- **Schema**: `schemas/runtime-schema.graphql`
- **Compiler**: Wesley generates bit-exact Rust/TS bridges and Zod validators.

## Performance & Scaling

Echo uses **WSC (Write-Streaming Columnar)**, a zero-copy snapshot format for fast state reload and verification. The hot render loop is optimized through reusable framebuffers and footprint-based scheduling.

---

**The goal is inevitably. Every continuation from the past is explicit, capability-gated, and provenance-bearing.**
