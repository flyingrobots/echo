<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# TTD Implementation Task DAG

**Generated:** 2026-01-25
**Visualization:** [`docs/assets/dags/ttd-plan-dag.svg`](../assets/dags/ttd-plan-dag.svg)

---

## Task Summary

| Phase     | Location          | Tasks  | Est. Duration |
| --------- | ----------------- | ------ | ------------- |
| 1a        | Wesley (external) | 5      | ~1 week       |
| 1b        | Wesley (external) | 6      | ~1 week       |
| 1c        | Wesley (external) | 4      | ~1 week       |
| 2         | Echo              | 6      | ~1 week       |
| 3         | Echo              | 4      | ~1 week       |
| 4         | Echo              | 3      | ~3 days       |
| 5         | Echo              | 5      | ~1.5 weeks    |
| 6         | Echo              | 6      | ~2 weeks      |
| 7         | Echo              | 3      | ~3 days       |
| **Total** |                   | **42** | ~9 weeks      |

---

## Phase 1: Wesley Schema Compiler (External Repo)

**Repository:** `flyingrobots/wesley`
**Planning Doc:** `docs/plans/ttd-protocol-compiler.md`

### Phase 1a: Foundation

| ID   | Task                                                | Depends On | Artifact                                         |
| ---- | --------------------------------------------------- | ---------- | ------------------------------------------------ |
| 1a.1 | Directive Parser (`@channel`, `@op`, `@rule`, etc.) | —          | `wesley-core/src/ttd/directives.ts`              |
| 1a.2 | TTD AST Model (channels, ops, rules)                | 1a.1       | `wesley-core/src/ttd/ast.ts`                     |
| 1a.3 | Schema Hasher (canonical ordering)                  | 1a.2       | `wesley-core/src/ttd/hasher.ts`                  |
| 1a.4 | Model Extractor (SDL → structured)                  | 1a.2       | `wesley-core/src/ttd/extractor.ts`               |
| 1a.5 | JSON Manifest Generation                            | 1a.3, 1a.4 | `schema.json`, `manifest.json`, `contracts.json` |

### Phase 1b: Codegen

| ID   | Task                            | Depends On | Artifact                 |
| ---- | ------------------------------- | ---------- | ------------------------ |
| 1b.1 | Rust Types (structs/enums)      | 1a.5       | `<out>/rust/types.rs`    |
| 1b.2 | Rust CBOR Codecs (minicbor)     | 1b.1       | `<out>/rust/cbor.rs`     |
| 1b.3 | Rust Registries (lookup tables) | 1b.1       | `<out>/rust/registry.rs` |
| 1b.4 | TypeScript Types                | 1a.5       | `<out>/ts/types.ts`      |
| 1b.5 | Zod Validators                  | 1b.4       | `<out>/ts/zod.ts`        |
| 1b.6 | TS Registries                   | 1b.4       | `<out>/ts/registry.ts`   |

**Output Crates/Packages:**

- `ttd-protocol-rs` (Rust crate)
- `ttd-protocol-ts` (TypeScript package)

### Phase 1c: Law Compiler

| ID   | Task                                   | Depends On | Artifact                                     |
| ---- | -------------------------------------- | ---------- | -------------------------------------------- |
| 1c.1 | Invariant Parser (expr grammar)        | 1a.5       | `wesley-core/src/ttd/invariants/parser.ts`   |
| 1c.2 | Obligation Compiler (specs → bytecode) | 1c.1       | `wesley-core/src/ttd/invariants/compiler.ts` |
| 1c.3 | Enforcement Bytecode (stack VM)        | 1c.2       | `ttd-manifest`                               |
| 1c.4 | Golden Test Generation                 | 1c.3       | `fixtures/*.cbor`                            |

---

## Phase 2: Receipt & Digest System

**Crates:** `echo-session-proto`, `echo-ttd`, `warp-core`

| ID  | Task                            | Depends On           | Artifact                            |
| --- | ------------------------------- | -------------------- | ----------------------------------- |
| 2.1 | EINT v2 encoder/decoder         | 1b (ttd-protocol-rs) | `echo-session-proto/src/eint_v2.rs` |
| 2.2 | TTDR v2 encoder/decoder         | 2.1                  | `echo-ttd/src/receipts.rs`          |
| 2.3 | `emissions_digest` computation  | —                    | `warp-core/src/digest.rs`           |
| 2.4 | `op_emission_index_digest`      | —                    | `warp-core/src/digest.rs`           |
| 2.5 | `commit_hash:v2` computation    | 2.3, 2.4             | `warp-core/src/digest.rs`           |
| 2.6 | AtomWrite tracking (provenance) | —                    | `warp-core/src/provenance_store.rs` |

---

## Phase 3: Compliance Engine

**Crate:** `echo-ttd`

| ID  | Task                           | Depends On                  | Artifact                     |
| --- | ------------------------------ | --------------------------- | ---------------------------- |
| 3.1 | ComplianceIndex computation    | 2.2, 2.5, 1c (ttd-manifest) | `echo-ttd/src/compliance.rs` |
| 3.2 | Channel policy checks          | —                           | `echo-ttd/src/compliance.rs` |
| 3.3 | Emission contract verification | 3.1, 3.2, 1c (ttd-manifest) | `echo-ttd/src/compliance.rs` |
| 3.4 | Footprint verification         | 2.6, 1c (ttd-manifest)      | `echo-ttd/src/compliance.rs` |

---

## Phase 4: Eventual Obligations

**Crate:** `echo-ttd`

| ID  | Task                    | Depends On                  | Artifact                      |
| --- | ----------------------- | --------------------------- | ----------------------------- |
| 4.1 | Obligation evaluation   | 3.3, 3.4, 1c (ttd-manifest) | `echo-ttd/src/obligations.rs` |
| 4.2 | Deadline tracking       | 4.1                         | `echo-ttd/src/obligations.rs` |
| 4.3 | ObligationStatus deltas | 4.2                         | `echo-ttd/src/obligations.rs` |

---

## Phase 5: Platform Crates

**Crates:** `ttd-controller`, `ttd-browser`, `ttd-native`, `warp-wasm`

| ID  | Task                                  | Depends On    | Artifact                                |
| --- | ------------------------------------- | ------------- | --------------------------------------- |
| 5.1 | TtdController WARP Schema (meta-WARP) | 2.3, 3.1, 4.3 | `crates/ttd-controller/`                |
| 5.2 | ObservedWorldAPI abstraction          | 5.1           | `crates/ttd-controller/src/observed.rs` |
| 5.3 | warp-wasm bindings                    | —             | `crates/warp-wasm/`                     |
| 5.4 | ttd-browser (WASM + wasm-bindgen)     | 5.2, 5.3      | `crates/ttd-browser/`                   |
| 5.5 | ttd-native (egui + wgpu)              | 5.2           | `crates/ttd-native/`                    |

---

## Phase 6: Browser TTD App

**Location:** `apps/ttd-app`

| ID  | Task                        | Depends On                | Artifact                       |
| --- | --------------------------- | ------------------------- | ------------------------------ |
| 6.1 | UI Framework (React/Svelte) | 5.4, 1b (ttd-protocol-ts) | `apps/ttd-app/`                |
| 6.2 | Three.js 3D Visualization   | —                         | `apps/ttd-app/src/viz/`        |
| 6.3 | Time Stepper Controls       | 6.1                       | `apps/ttd-app/src/components/` |
| 6.4 | Graph View (nodes/edges)    | 6.1, 6.2                  | `apps/ttd-app/src/components/` |
| 6.5 | Provenance Drawer           | 6.1                       | `apps/ttd-app/src/components/` |
| 6.6 | Fork Comparison             | 6.4                       | `apps/ttd-app/src/components/` |

---

## Phase 7: Demo Polish

| ID  | Task                           | Depends On | Artifact              |
| --- | ------------------------------ | ---------- | --------------------- |
| 7.1 | Demo Scenarios (Counter, Game) | 6.5, 6.6   | `apps/ttd-app/demos/` |
| 7.2 | Golden Test Vectors            | 7.1        | `fixtures/`           |
| 7.3 | Documentation Polish           | 7.1        | `docs/`               |

---

## Critical Path

The longest dependency chain (critical path):

```text
1a.1 → 1a.2 → 1a.3 → 1a.5 → 1c.1 → 1c.2 → 1c.3 → 3.1 → 3.3 → 4.1 → 5.1 → 5.2 → 5.4 → 6.1 → 6.4 → 6.6 → 7.1
```

**Parallelizable Work:**

- Phase 1b (Codegen) and Phase 1c (Law Compiler) can run in parallel after 1a.5
- Phase 2 tasks 2.3, 2.4, 2.6 have no dependencies and can start immediately after Wesley Phase 1b
- Phase 5.5 (ttd-native) and Phase 5.4 (ttd-browser) can run in parallel after 5.2

---

## Cross-Repo Dependencies

```text
┌─────────────────────────────────────────────────────────────────┐
│  flyingrobots/wesley                                            │
│  └─► ttd-protocol-rs (crate)                                   │
│  └─► ttd-protocol-ts (package)                                 │
│  └─► ttd-manifest (enforcement tables)                         │
└───────────────────────────┬─────────────────────────────────────┘
                            │ dev dependency
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│  flyingrobots/echo                                              │
│  └─► crates/echo-session-proto (consumes ttd-protocol-rs)      │
│  └─► crates/echo-ttd (consumes ttd-manifest)                   │
│  └─► apps/ttd-app (consumes ttd-protocol-ts)                   │
└─────────────────────────────────────────────────────────────────┘
```
