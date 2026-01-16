<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# JITOS Engineering Standard: The Living Specification

**Status:** In Progress (living spec scaffold exists; certification pending)  
**Version:** 1.0.0  
**Context:** Development Methodology & Contributor Workflow

> **Implementation Status Legend:**
> - ✅ **Implemented** — enforced in this repo today (CI or runtime)
> - ⚠️ **Partial** — some pieces exist, others are in-flight
> - 🗺️ **Planned** — vision/aspirational, not yet implemented

## 1. Abstract ⚠️

The JITOS operating system rejects the traditional dichotomy between "code" and "documentation." Given the paradigm-shifting nature of the Causal Operating System (WARP graphs, Event Sourcing, Schrödinger Workspaces), static text is insufficient to convey system behavior.

Instead, JITOS adopts the **"5x Duty" Methodology**. Every feature added to the kernel must simultaneously serve five distinct purposes through a single, unified codebase. We do not write documentation *about* the OS; we compile the OS *into* the documentation.

## 1.1 Current Status in This Repository (Echo) ✅

This document describes the **target** JITOS workflow, but not every element is implemented yet in this repo.

Last reviewed: **2026-01-16**.

- Implemented ✅:
  - A living-spec scaffold exists for **Spec-000** at `specs/spec-000-rewrite/` (Leptos + Trunk).
  - WASM-friendly DTOs live in `crates/echo-wasm-abi/` and the current “demo kernel” wrapper lives in `crates/echo-wasm-bindings/`.
- Planned 🗺️:
  - A `no_std` kernel crate named `crates/echo-kernel`.
  - Hosted spec domains like `spec-001.jitos.dev`.
  - Automatic UI-issued “Completion Hash” contributor certification.

**Enforced today (CI)** ✅: `cargo fmt -- --check`, `cargo clippy --all-targets -- -D warnings -D missing_docs`, `cargo test` (workspace + warp-core variants), `scripts/ban-globals.sh`, `scripts/ban-nondeterminism.sh`.  
**Future vision** 🗺️: hosted spec domains, certification UI, and full `no_std` kernel split.

## 2. The 5x Duty Model 🗺️

**Status:** Largely aspirational today; the demo + docs exist, but the full five-way unification is not yet enforced.

Every Major Feature Specification (SPEC) acts as a unified artifact fulfilling these five roles:

1. ✅ **Documentation:** A narrative explanation of the feature (the "Why" and "What").
2. ⚠️ **Implementation:** The actual, production-grade Rust code (the "How").
3. ⚠️ **Interactive Demo:** A WebAssembly-compiled instance of the kernel running in the browser, allowing real-time state manipulation.
4. 🗺️ **Living Test:** A visual verification suite where the "Demo" acts as a graphical test runner.
5. 🗺️ **Certification:** A gamified proof-of-competence that issues a cryptographic hash to users who successfully drive the kernel to a target state, proving they understand the concept.

## 3. Workflow Architecture ⚠️

The following diagram illustrates how a single Rust source feed generates the Kernel, the Spec, and the Verification assets simultaneously.

```mermaid
graph TD
    subgraph "The Source of Truth"
        Source[crates/echo-wasm-bindings<br/>(Current demo kernel / WASM DTOs)]
    end

    subgraph "Build Targets"
        Native[Native Target<br/>x86_64 / Aarch64]
        Wasm[WASM Target<br/>wasm32-unknown]
    end

    subgraph "The Living Spec (Web)"
        Page[Spec Page<br/>(Leptos/HTML)]
        UI[Interactive UI]
        Narrative[Docs & Theory]
    end

    subgraph "Outputs"
        Binary[Production OS Binary]
        Cert[Contributor Certificate]
    end

    Source -->|Compiles| Native
    Source -->|Compiles| Wasm
    
    Native --> Binary
    
    Wasm --> UI
    UI -->|Embedded In| Page
    Narrative -->|Embedded In| Page
    
    User((User / Dev)) -->|Reads| Narrative
    User -->|Manipulates| UI
    UI -->|Calls| Source
    
    UI -->|Verifies Success| Cert

    style Source fill:#f96,stroke:#333,stroke-width:2px
    style Page fill:#bbf,stroke:#333,stroke-width:2px
    style Cert fill:#9f9,stroke:#333,stroke-width:2px
```

## 4. The Contributor Lifecycle ⚠️

Under this methodology, the "Onboarding" process is identical to the "Testing" process.

1. ⚠️ **The Challenge:** A new contributor runs the current living spec locally (e.g. `make spec-000-dev`).
2. ⚠️ **The Context:** They read the narrative explaining *why* JITOS uses append-only storage.
3. ⚠️ **The Interaction:** They use the embedded WASM demo to attempt a rewrite. This executes the current demo kernel logic compiled to WASM.
4. ⚠️ **The Validation:** If they correctly perform the operation (e.g., creating a transaction rather than mutating a value), the Kernel state updates successfully.
5. 🗺️ **The Certification:** A future UI win-condition will generate a `Completion Hash` for contributor certification.
6. ✅ **The Contribution (today):** The contributor includes the relevant test evidence (and/or spec screenshots/logs) in their PR, and reviewers validate the spec + tests.

## 5. Technical Stack ⚠️

To enable this workflow, we strictly separate **Logic** from **IO**.

- ✅ **Logic (Kernel slices):** Written in Rust crates under `crates/`. The long-term goal is to isolate a `no_std` kernel core, but the current Spec-000 demo kernel is not `no_std` yet.
- ⚠️ **The Spec Runner (WASM):** Uses **Leptos** and **Trunk** to bind the Kernel Logic to DOM elements.
- 🗺️ **The OS Runner (Native):** Binds the Kernel Logic to physical hardware drivers and NVMe storage.

## 6. Definition of Done ⚠️

A feature is not "Done" until:

- [x] ✅ Formatting is clean: `cargo fmt` (enforced in CI).
- [x] ✅ The code builds and tests pass: `cargo test` (enforced in CI).
- [x] ✅ Public APIs are documented and the docs gate is clean: `cargo clippy --all-targets -- -D missing_docs` (enforced in CI).
- [ ] ⚠️ SPDX header policy is satisfied: `scripts/check_spdx.sh --check --all` (defined, not enforced in CI yet).
- [ ] ⚠️ If the change is spec-facing: a `specs/spec-XXX` directory exists and the spec page explains the concept.
- [ ] ⚠️ If the change is spec-facing: the spec imports the relevant kernel slice and provides an interactive demo harness.
- [ ] 🗺️ If/when certification is enabled: the spec defines a deterministic “win condition” that can emit a completion proof (planned; not yet implemented).
