<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# JITOS Engineering Standard: The Living Specification

**Status:** Active  
**Version:** 1.0.0  
**Context:** Development Methodology & Contributor Workflow

## 1. Abstract

The JITOS operating system rejects the traditional dichotomy between "code" and "documentation." Given the paradigm-shifting nature of the Causal Operating System (Recursive Metagraphs, Event Sourcing, Schrödinger Workspaces), static text is insufficient to convey system behavior.

Instead, JITOS adopts the **"5x Duty" Methodology**. Every feature added to the kernel must simultaneously serve five distinct purposes through a single, unified codebase. We do not write documentation *about* the OS; we compile the OS *into* the documentation.

## 2. The 5x Duty Model

Every Major Feature Specification (SPEC) acts as a unified artifact fulfilling these five roles:

1. **Documentation:** A narrative explanation of the feature (the "Why" and "What").
2. **Implementation:** The actual, production-grade Rust code (the "How").
3. **Interactive Demo:** A WebAssembly-compiled instance of the kernel running in the browser, allowing real-time state manipulation.
4. **Living Test:** A visual verification suite where the "Demo" acts as a graphical test runner.
5. **Certification:** A gamified proof-of-competence that issues a cryptographic hash to users who successfully drive the kernel to a target state, proving they understand the concept.

## 3. Workflow Architecture

The following diagram illustrates how a single Rust source feed generates the Kernel, the Spec, and the Verification assets simultaneously.

```mermaid
graph TD
    subgraph "The Source of Truth"
        Source[crates/echo-kernel<br/>(Pure Rust / No_Std)]
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

## 4. The Contributor Lifecycle

Under this methodology, the "Onboarding" process is identical to the "Testing" process.

1. **The Challenge:** A new contributor navigates to `spec-001.jitos.dev` (The Rewrite).
2. **The Context:** They read the narrative explaining *why* JITOS uses append-only storage.
3. **The Interaction:** They use the embedded WASM demo to attempt a rewrite. This executes the **actual** `echo-kernel` crate logic in their browser.
4. **The Validation:** If they correctly perform the operation (e.g., creating a transaction rather than mutating a value), the Kernel state updates successfully.
5. **The Certification:** The UI detects the valid state transition and generates a `Completion Hash`.
6. **The Contribution:** The contributor includes this hash in their Pull Request, proving they have interacted with and understood the subsystem they are modifying.

## 5. Technical Stack

To enable this workflow, we strictly separate **Logic** from **IO**.

- **Logic (The Kernel):** Written in `no_std` Rust. It manages the Graph, Time, and Inversion Engine. It knows nothing about files, sockets, or screens.
- **The Spec Runner (WASM):** Uses **Leptos** and **Trunk** to bind the Kernel Logic to DOM elements.
- **The OS Runner (Native):** Binds the Kernel Logic to physical hardware drivers and NVMe storage.

## 6. Definition of Done

A feature is not "Done" until:

- [ ] The Core Logic is written in `crates/echo-kernel`.
- [ ] A `specs/spec-XXX` directory is created.
- [ ] The Spec page explains the concept.
- [ ] The Spec page imports the Kernel and creates an interactive visualization.
- [ ] A "Win Condition" is defined in the UI that issues a completion badge.
