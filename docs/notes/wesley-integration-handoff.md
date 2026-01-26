<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Wesley Integration Handoff

**Date:** 2026-01-25
**Branch:** `ttd-spec`
**Status:** Echo TTD work blocked on Wesley

---

## Executive Summary

All Wesley-independent TTD work in Echo is complete. The remaining tasks form a dependency chain through Wesley's Phase 1b and 1c outputs. This document briefs the next agent on what Wesley needs to deliver and how it integrates with Echo.

---

## What's Done in Echo

### Completed Tasks

| Task | Artifact                        | Description                                   |
| ---- | ------------------------------- | --------------------------------------------- |
| 2.3  | `warp-core/src/snapshot.rs`     | `compute_emissions_digest()`                  |
| 2.4  | `warp-core/src/snapshot.rs`     | `compute_op_emission_index_digest()`          |
| 2.5  | `warp-core/src/snapshot.rs`     | `compute_tick_commit_hash_v2()`               |
| 2.6  | `warp-core/src/worldline.rs`    | `AtomWrite` tracking                          |
| 3.2  | `echo-ttd/src/compliance.rs`    | `PolicyChecker`, channel policy validation    |
| 5.3  | `warp-wasm/src/ttd.rs`          | WASM bindings for digests, compliance, codecs |
| 5.4  | `ttd-browser/`                  | `TtdEngine` WASM struct for browser           |
| 6.1  | `apps/ttd-app/`                 | React UI scaffold (24 files)                  |
| 6.2  | `packages/echo-renderer-three/` | Three.js ScenePort adapter                    |
| —    | `echo-session-proto/`           | EINT v2 + TTDR v2 wire codecs                 |

### Uncommitted Work

```text
apps/ttd-app/           # 24 files - React UI scaffold with mock engine
```

---

## What Wesley Needs to Deliver

### Phase 1b: Codegen (ttd-protocol-ts, ttd-protocol-rs)

**Output:** Generated TypeScript and Rust types from Wesley SDL schemas.

**Echo needs:**

1. **`ttd-protocol-ts`** — TypeScript types for the TTD app
    - Replace `apps/ttd-app/src/types/ttd.ts` (placeholder types)
    - Types needed: `WorldlineId`, `CursorId`, `TruthFrame`, `Violation`, `Obligation`, etc.

2. **`ttd-protocol-rs`** — Rust types for ttd-controller
    - Used by Tasks 5.1, 5.2 (TtdController, ObservedWorldAPI)

### Phase 1c: Law Compiler (ttd-manifest)

**Output:** Compiled schema manifests with invariant/obligation bytecode.

**Echo needs:**

1. **`ttd-manifest`** — Schema manifest with:
    - `schema_hash` for EINT/TTDR headers
    - Channel policies (for ComplianceIndex)
    - Obligation contracts (producesWithin, requiresChannels, etc.)

---

## Blocked Echo Tasks

These tasks cannot proceed until Wesley delivers:

| Task                       | Waiting For          | What It Does                    |
| -------------------------- | -------------------- | ------------------------------- |
| 3.1 ComplianceIndex        | `ttd-manifest`       | JIT frame compliance checking   |
| 3.3 Emission contracts     | 3.1 + `ttd-manifest` | Verify emission contracts       |
| 3.4 Footprint verification | `ttd-manifest`       | Rule footprint validation       |
| 4.1 Obligation evaluation  | Phase 1c             | Evaluate obligation expressions |
| 4.2 Deadline tracking      | 4.1                  | Track obligation deadlines      |
| 4.3 ObligationStatus       | 4.2                  | Delta emissions for obligations |
| 5.1 TtdController          | 3.1, 4.3             | Meta-WARP for TTD               |
| 5.2 ObservedWorldAPI       | 5.1                  | Abstraction over observed app   |
| 5.5 ttd-native             | 5.2                  | Desktop app (egui + wgpu)       |

---

## Integration Points

### 1. Schema Hash Flow

```text
Wesley SDL → Wesley Compiler → schema.json (contains schema_hash)
                                    ↓
                            Echo uses schema_hash in:
                            - EINT v2 header (84 bytes)
                            - TTDR v2 header (244 bytes)
                            - Compliance verification
```

### 2. Channel Policies

Wesley defines policies in SDL:

```wesley
channel ttd.head {
  policy: StrictSingle
}

channel ttd.errors {
  policy: Log
}
```

Echo's `PolicyChecker` validates emissions against these policies.

### 3. Obligations

Wesley defines obligations:

```wesley
obligation ProducesHead {
  trigger: TTD_SEEK
  contract: producesWithin(ttd.head, 3)
}
```

Echo's `OblTracker` (Task 4.x) evaluates these at runtime.

---

## Wesley Repo Context

**Location:** Likely `/Users/james/git/wesley/` (check Redis or ask user)

**Key files to create/update:**

1. `docs/plans/ttd-protocol-compiler.md` — Planning doc (may exist)
2. `src/codegen/typescript.rs` — TS type generation
3. `src/codegen/rust.rs` — Rust type generation
4. `src/compiler/manifest.rs` — Manifest compilation
5. `src/compiler/obligations.rs` — Obligation bytecode

**Wesley SDL for TTD:** The TTD schema should define:

- Atom types (ttd/cursor, ttd/session, ttd/worldline, etc.)
- Channel types (ttd.head, ttd.errors, ttd.compliance, etc.)
- Obligation contracts

---

## Quick Start for Next Agent

1. **Bootstrap session:**

    ```bash
    XRANGE echo:agent:handoff - + COUNT 1
    search_nodes("Wesley")
    search_nodes("TTD")
    ```

2. **Read Wesley planning doc:**

    ```text
    /Users/james/git/wesley/docs/plans/ttd-protocol-compiler.md
    ```

3. **Check Wesley task DAG:**

    ```text
    /Users/james/git/echo/docs/plans/ttd-task-dag.md (Phase 1a/1b/1c)
    ```

4. **Start with Phase 1b** if 1a (SDL parser) is done:
    - Implement TypeScript codegen first (unblocks ttd-app)
    - Then Rust codegen (unblocks ttd-controller)

---

## Test Commands

### Echo (verify nothing broke)

```bash
cargo test -p ttd-browser
cargo test -p warp-wasm
cargo test -p echo-ttd
cargo clippy --workspace -- -D warnings
```

### Wesley (once implemented)

```bash
# Generate types
wesley codegen --lang typescript --schema ttd.sdl --out ttd-protocol-ts/
wesley codegen --lang rust --schema ttd.sdl --out ttd-protocol-rs/

# Compile manifest
wesley compile --schema ttd.sdl --out ttd-manifest/
```

---

## Key Knowledge Graph Entities

Query these for architectural context:

- `Wesley_Staging_Model` — v0/v1/v2 maturity layers
- `TTD_Protocol_Boundary` — JIT! vs EINT separation
- `TTD_Intent_Protocol` — EINT v2 opcodes
- `TtdController_WARP` — Meta-WARP architecture
- `MBUS_Client_Pattern` — How ttd-browser/ttd-native consume MBUS

---

## Contact

If blocked or unclear, the human (James) can clarify requirements.

Good luck in the multiverse! 🚀
