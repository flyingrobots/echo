<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# TASKS-DAG

This living list documents open issues and the inferred dependencies contributors capture while planning. Run `scripts/generate-tasks-dag.js` (requires Node.js + Graphviz `dot`) to render the DAG found at `docs/assets/dags/tasks-dag.svg`.

## [#19: Spec: Persistent Store (on-disk)](https://github.com/flyingrobots/echo/issues/19)

- Status: Open
- Blocked by:
    - [#28: Draft spec document (header/ULEB128/property/string-pool)](https://github.com/flyingrobots/echo/issues/28)
    - Confidence: strong
    - Evidence: Inferred: Epic completion depends on Draft Spec task
- Evidence: `crates/echo-config-fs` exists for tool preferences, but no dedicated graph store crate (e.g. `echo-store`) exists yet.

## [#20: Spec: Commit/Manifest Signing](https://github.com/flyingrobots/echo/issues/20)

- Status: Open
- Blocked by:
    - [#32: Draft signing spec](https://github.com/flyingrobots/echo/issues/32)
    - Confidence: strong
    - Evidence: Inferred: Epic completion depends on Draft Spec task
    - [#33: CI: sign release artifacts (dry run)](https://github.com/flyingrobots/echo/issues/33)
    - Confidence: strong
    - Evidence: Inferred: Epic completion depends on constituent task
    - [#34: CLI verify path](https://github.com/flyingrobots/echo/issues/34)
    - Confidence: strong
    - Evidence: Inferred: Epic completion depends on constituent task
    - [#35: Key management doc](https://github.com/flyingrobots/echo/issues/35)
    - Confidence: strong
    - Evidence: Inferred: Epic completion depends on constituent task
    - [#36: CI: verify signatures](https://github.com/flyingrobots/echo/issues/36)
    - Confidence: strong
    - Evidence: Inferred: Epic completion depends on constituent task

## [#21: Spec: Security Contexts (FFI/WASM/CLI)](https://github.com/flyingrobots/echo/issues/21)

- Status: Open
- Blocked by:
    - [#37: Draft security contexts spec](https://github.com/flyingrobots/echo/issues/37)
    - Confidence: strong
    - Evidence: Inferred: Epic completion depends on Draft Spec task
    - [#38: FFI limits and validation](https://github.com/flyingrobots/echo/issues/38)
    - Confidence: strong
    - Evidence: Inferred: Epic completion depends on constituent task
    - [#39: WASM input validation](https://github.com/flyingrobots/echo/issues/39)
    - Confidence: strong
    - Evidence: Inferred: Epic completion depends on constituent task
    - [#40: Unit tests for denials](https://github.com/flyingrobots/echo/issues/40)
    - Confidence: strong
    - Evidence: Inferred: Epic completion depends on constituent task

## [#22: Benchmarks & CI Regression Gates](https://github.com/flyingrobots/echo/issues/22)

- Status: Completed
- (No detected dependencies)

## [#23: CLI: verify/bench/inspect](https://github.com/flyingrobots/echo/issues/23)

- Status: Open
- Evidence: `crates/warp-cli/src/main.rs` is currently a placeholder printing "Hello, world!".
- (No detected dependencies)

## [#24: Editor Hot-Reload (spec + impl)](https://github.com/flyingrobots/echo/issues/24)

- Status: Open
- (No detected dependencies)

## [#25: Importer: TurtlGraph → Echo store](https://github.com/flyingrobots/echo/issues/25)

- Status: Open
- (No detected dependencies)

## [#26: Plugin ABI (C) v0](https://github.com/flyingrobots/echo/issues/26)

- Status: In Progress
- (No detected dependencies)

## [#27: Add golden test vectors (encoder/decoder)](https://github.com/flyingrobots/echo/issues/27)

- Status: Completed
- (No detected dependencies)

## [#28: Draft spec document (header/ULEB128/property/string-pool)](https://github.com/flyingrobots/echo/issues/28)

- Status: Completed
- Blocks:
    - [#19: Spec: Persistent Store (on-disk)](https://github.com/flyingrobots/echo/issues/19)
    - Confidence: strong
    - Evidence: Inferred: Epic completion depends on Draft Spec task

## [#29: Prototype header+string-pool encoder](https://github.com/flyingrobots/echo/issues/29)

- Status: Open
- (No detected dependencies)

## [#30: Prototype header+string-pool decoder](https://github.com/flyingrobots/echo/issues/30)

- Status: Open
- (No detected dependencies)

## [#32: Draft signing spec](https://github.com/flyingrobots/echo/issues/32)

- Status: Completed
- Blocks:
    - [#20: Spec: Commit/Manifest Signing](https://github.com/flyingrobots/echo/issues/20)
    - Confidence: strong
    - Evidence: Inferred: Epic completion depends on Draft Spec task

## [#33: CI: sign release artifacts (dry run)](https://github.com/flyingrobots/echo/issues/33)

- Status: Open
- Blocks:
    - [#20: Spec: Commit/Manifest Signing](https://github.com/flyingrobots/echo/issues/20)
    - Confidence: strong
    - Evidence: Inferred: Epic completion depends on constituent task

## [#34: CLI verify path](https://github.com/flyingrobots/echo/issues/34)

- Status: Open
- Blocks:
    - [#20: Spec: Commit/Manifest Signing](https://github.com/flyingrobots/echo/issues/20)
    - Confidence: strong
    - Evidence: Inferred: Epic completion depends on constituent task

## [#35: Key management doc](https://github.com/flyingrobots/echo/issues/35)

- Status: Open
- Blocks:
    - [#20: Spec: Commit/Manifest Signing](https://github.com/flyingrobots/echo/issues/20)
    - Confidence: strong
    - Evidence: Inferred: Epic completion depends on constituent task

## [#36: CI: verify signatures](https://github.com/flyingrobots/echo/issues/36)

- Status: Open
- Blocks:
    - [#20: Spec: Commit/Manifest Signing](https://github.com/flyingrobots/echo/issues/20)
    - Confidence: strong
    - Evidence: Inferred: Epic completion depends on constituent task

## [#37: Draft security contexts spec](https://github.com/flyingrobots/echo/issues/37)

- Status: Completed
- Blocks:
    - [#21: Spec: Security Contexts (FFI/WASM/CLI)](https://github.com/flyingrobots/echo/issues/21)
    - Confidence: strong
    - Evidence: Inferred: Epic completion depends on Draft Spec task

## [#38: FFI limits and validation](https://github.com/flyingrobots/echo/issues/38)

- Status: In Progress
- Blocks:
    - [#21: Spec: Security Contexts (FFI/WASM/CLI)](https://github.com/flyingrobots/echo/issues/21)
    - Confidence: strong
    - Evidence: Inferred: Epic completion depends on constituent task

## [#39: WASM input validation](https://github.com/flyingrobots/echo/issues/39)

- Status: In Progress
- Blocks:
    - [#21: Spec: Security Contexts (FFI/WASM/CLI)](https://github.com/flyingrobots/echo/issues/21)
    - Confidence: strong
    - Evidence: `crates/warp-wasm/src/lib.rs` implements `validate_object_against_args` for schema checks.

## [#40: Unit tests for denials](https://github.com/flyingrobots/echo/issues/40)

- Status: Open
- Blocks:
    - [#21: Spec: Security Contexts (FFI/WASM/CLI)](https://github.com/flyingrobots/echo/issues/21)
    - Confidence: strong
    - Evidence: Inferred: Epic completion depends on constituent task

## [#41: README+docs](https://github.com/flyingrobots/echo/issues/41)

- Status: Completed
- (No detected dependencies)

## [#47: Scaffold CLI subcommands](https://github.com/flyingrobots/echo/issues/47)

- Status: Open
- (No detected dependencies)

## [#48: Implement 'verify'](https://github.com/flyingrobots/echo/issues/48)

- Status: Open
- (No detected dependencies)

## [#49: Implement 'bench'](https://github.com/flyingrobots/echo/issues/49)

- Status: Open
- (No detected dependencies)

## [#50: Implement 'inspect'](https://github.com/flyingrobots/echo/issues/50)

- Status: Open
- (No detected dependencies)

## [#51: Docs/man pages](https://github.com/flyingrobots/echo/issues/51)

- Status: Open
- (No detected dependencies)

## [#75: Draft hot-reload spec](https://github.com/flyingrobots/echo/issues/75)

- Status: Open
- (No detected dependencies)

## [#76: File watcher/debounce](https://github.com/flyingrobots/echo/issues/76)

- Status: Open
- (No detected dependencies)

## [#77: Atomic snapshot swap](https://github.com/flyingrobots/echo/issues/77)

- Status: Open
- (No detected dependencies)

## [#78: Editor gate + tests](https://github.com/flyingrobots/echo/issues/78)

- Status: Open
- (No detected dependencies)

## [#79: Docs/logging](https://github.com/flyingrobots/echo/issues/79)

- Status: Open
- (No detected dependencies)

## [#80: Draft importer spec](https://github.com/flyingrobots/echo/issues/80)

- Status: Open
- (No detected dependencies)

## [#81: Minimal reader](https://github.com/flyingrobots/echo/issues/81)

- Status: Open
- (No detected dependencies)

## [#82: Echo store loader](https://github.com/flyingrobots/echo/issues/82)

- Status: Open
- (No detected dependencies)

## [#83: Integrity verification](https://github.com/flyingrobots/echo/issues/83)

- Status: Open
- (No detected dependencies)

## [#84: Sample + tests](https://github.com/flyingrobots/echo/issues/84)

- Status: Open
- (No detected dependencies)

## [#85: Draft C ABI spec](https://github.com/flyingrobots/echo/issues/85)

- Status: Completed
- (No detected dependencies)

## [#86: C header + host loader](https://github.com/flyingrobots/echo/issues/86)

- Status: In Progress
- (No detected dependencies)

## [#87: Version negotiation](https://github.com/flyingrobots/echo/issues/87)

- Status: Open
- (No detected dependencies)

## [#88: Capability tokens](https://github.com/flyingrobots/echo/issues/88)

- Status: Open
- (No detected dependencies)

## [#89: Example plugin + tests](https://github.com/flyingrobots/echo/issues/89)

- Status: Open
- (No detected dependencies)

## [#103: Policy: Require PR↔Issue linkage and 'Closes #…' in PRs](https://github.com/flyingrobots/echo/issues/103)

- Status: Open
- (No detected dependencies)

## [#170: TT1: StreamsFrame inspector support (backlog + cursors + admission decisions)](https://github.com/flyingrobots/echo/issues/170)

- Status: Open
- Blocks:
    - [#171: TT2: Time Travel MVP (pause/rewind/buffer/catch-up)](https://github.com/flyingrobots/echo/issues/171)
    - Confidence: medium
    - Evidence: Inferred: TT2 task depends on TT1 Inspector scaffolding
    - [#205: TT2: Reliving debugger MVP (scrub timeline + causal slice + fork branch)](https://github.com/flyingrobots/echo/issues/205)
    - Confidence: medium
    - Evidence: Inferred: TT2 task depends on TT1 Inspector scaffolding
- Blocked by:
    - [#246: TT1: Security/capabilities for fork/rewind/merge in multiplayer](https://github.com/flyingrobots/echo/issues/246)
    - Confidence: medium
    - Evidence: Inferred: TT1 Implementation blocks on TT1 Spec clarifications
    - [#245: TT1: Merge semantics for admitted stream facts across worldlines](https://github.com/flyingrobots/echo/issues/245)
    - Confidence: medium
    - Evidence: Inferred: TT1 Implementation blocks on TT1 Spec clarifications
    - [#244: TT1: TimeStream retention + spool compaction + wormhole density](https://github.com/flyingrobots/echo/issues/244)
    - Confidence: medium
    - Evidence: Inferred: TT1 Implementation blocks on TT1 Spec clarifications
    - [#243: TT1: dt policy (fixed timestep vs admitted dt stream)](https://github.com/flyingrobots/echo/issues/243)
    - Confidence: medium
    - Evidence: Inferred: TT1 Implementation blocks on TT1 Spec clarifications

## [#171: TT2: Time Travel MVP (pause/rewind/buffer/catch-up)](https://github.com/flyingrobots/echo/issues/171)

- Status: Open
- Blocks:
    - [#172: TT3: Rulial diff / worldline compare MVP](https://github.com/flyingrobots/echo/issues/172)
    - Confidence: weak
    - Evidence: Inferred: TT3 task depends on TT2 MVP
    - [#204: TT3: Provenance heatmap (blast radius / cohesion over time)](https://github.com/flyingrobots/echo/issues/204)
    - Confidence: weak
    - Evidence: Inferred: TT3 task depends on TT2 MVP
    - [#199: TT3: Wesley worldline diff (compare query outputs/proofs across ticks)](https://github.com/flyingrobots/echo/issues/199)
    - Confidence: weak
    - Evidence: Inferred: TT3 task depends on TT2 MVP
- Blocked by:
    - [#170: TT1: StreamsFrame inspector support (backlog + cursors + admission decisions)](https://github.com/flyingrobots/echo/issues/170)
    - Confidence: medium
    - Evidence: Inferred: TT2 task depends on TT1 Inspector scaffolding

## [#172: TT3: Rulial diff / worldline compare MVP](https://github.com/flyingrobots/echo/issues/172)

- Status: Open
- Blocked by:
    - [#171: TT2: Time Travel MVP (pause/rewind/buffer/catch-up)](https://github.com/flyingrobots/echo/issues/171)
    - Confidence: weak
    - Evidence: Inferred: TT3 task depends on TT2 MVP

## [#173: S1: Deterministic Rhai surface (sandbox + claims/effects)](https://github.com/flyingrobots/echo/issues/173)

- Status: Open
- (No detected dependencies)

## [#174: W1: Wesley as a boundary grammar (hashable view artifacts)](https://github.com/flyingrobots/echo/issues/174)

- Status: Open
- (No detected dependencies)

## [#185: M1: Domain-separated hash contexts for core commitments (state_root/patch_digest/commit_id)](https://github.com/flyingrobots/echo/issues/185)

- Status: Completed
- (No detected dependencies)

## [#186: M1: Domain-separated digest context for RenderGraph canonical bytes](https://github.com/flyingrobots/echo/issues/186)

- Status: Completed
- (No detected dependencies)

## [#187: M4: Worldline convergence property suite (replay-from-patches converges)](https://github.com/flyingrobots/echo/issues/187)

- Status: Open
- (No detected dependencies)

## [#188: M4: Kernel nondeterminism tripwires (forbid ambient HostTime/entropy sources)](https://github.com/flyingrobots/echo/issues/188)

- Status: Open
- (No detected dependencies)

## [#189: M4: Concurrency litmus suite for scheduler determinism (overlap detection + canonical reduction)](https://github.com/flyingrobots/echo/issues/189)

- Status: Open
- (No detected dependencies)

## [#190: M4: Determinism torture harness (1-thread vs N-thread + snapshot/restore fuzz)](https://github.com/flyingrobots/echo/issues/190)

- Status: Open
- (No detected dependencies)

## [#191: TT0: Session stream time fields (HistoryTime ordering vs HostTime telemetry)](https://github.com/flyingrobots/echo/issues/191)

- Status: Open
- (No detected dependencies)

## [#192: TT0: TTL/deadline semantics are ticks/epochs only (no host-time semantic deadlines)](https://github.com/flyingrobots/echo/issues/192)

- Status: Open
- (No detected dependencies)

## [#193: W1: Schema hash chain pinning (SDL→IR→bundle) recorded in receipts](https://github.com/flyingrobots/echo/issues/193)

- Status: Open
- (No detected dependencies)

## [#194: W1: SchemaDelta vocabulary (read-only MVP) + wesley patch dry-run plan](https://github.com/flyingrobots/echo/issues/194)

- Status: Open
- (No detected dependencies)

## [#195: Backlog: JS-ABI packet checksum v2 (domain-separated hasher context)](https://github.com/flyingrobots/echo/issues/195)

- Status: Open
- (No detected dependencies)

## [#198: W1: Provenance as query semantics (tick directive + proof objects + deterministic cursors)](https://github.com/flyingrobots/echo/issues/198)

- Status: Open
- (No detected dependencies)

## [#199: TT3: Wesley worldline diff (compare query outputs/proofs across ticks)](https://github.com/flyingrobots/echo/issues/199)

- Status: Open
- Blocked by:
    - [#171: TT2: Time Travel MVP (pause/rewind/buffer/catch-up)](https://github.com/flyingrobots/echo/issues/171)
    - Confidence: weak
    - Evidence: Inferred: TT3 task depends on TT2 MVP

## [#202: Spec: Provenance Payload (PP) v1 (canonical envelope for artifact lineage + signatures)](https://github.com/flyingrobots/echo/issues/202)

- Status: In Progress
- (No detected dependencies)

## [#203: TT1: Constraint Lens panel (admission/scheduler explain-why + counterfactual sliders)](https://github.com/flyingrobots/echo/issues/203)

- Status: Open
- (No detected dependencies)

## [#204: TT3: Provenance heatmap (blast radius / cohesion over time)](https://github.com/flyingrobots/echo/issues/204)

- Status: Open
- Blocked by:
    - [#171: TT2: Time Travel MVP (pause/rewind/buffer/catch-up)](https://github.com/flyingrobots/echo/issues/171)
    - Confidence: weak
    - Evidence: Inferred: TT3 task depends on TT2 MVP

## [#205: TT2: Reliving debugger MVP (scrub timeline + causal slice + fork branch)](https://github.com/flyingrobots/echo/issues/205)

- Status: Open
- Blocked by:
    - [#170: TT1: StreamsFrame inspector support (backlog + cursors + admission decisions)](https://github.com/flyingrobots/echo/issues/170)
    - Confidence: medium
    - Evidence: Inferred: TT2 task depends on TT1 Inspector scaffolding

## [#206: M2.1: DPO concurrency theorem coverage (critical pair / rule composition litmus tests)](https://github.com/flyingrobots/echo/issues/206)

- Status: Open
- (No detected dependencies)

## [#207: Backlog: Run noisy-line test for naming (Echo / WARP / Wesley / Engram)](https://github.com/flyingrobots/echo/issues/207)

- Status: Open
- (No detected dependencies)

## [#222: Demo 2: Splash Guy — deterministic rules + state model](https://github.com/flyingrobots/echo/issues/222)

- Status: Open
- Blocks:
    - [#226: Demo 2: Splash Guy — docs: networking-first course modules](https://github.com/flyingrobots/echo/issues/226)
    - Confidence: medium
    - Evidence: Inferred: Docs follow Implementation

## [#223: Demo 2: Splash Guy — lockstep input protocol + two-peer harness](https://github.com/flyingrobots/echo/issues/223)

- Status: Open
- Blocks:
    - [#226: Demo 2: Splash Guy — docs: networking-first course modules](https://github.com/flyingrobots/echo/issues/226)
    - Confidence: medium
    - Evidence: Inferred: Docs follow Implementation

## [#224: Demo 2: Splash Guy — controlled desync lessons (make it fail on purpose)](https://github.com/flyingrobots/echo/issues/224)

- Status: Open
- Blocks:
    - [#226: Demo 2: Splash Guy — docs: networking-first course modules](https://github.com/flyingrobots/echo/issues/226)
    - Confidence: medium
    - Evidence: Inferred: Docs follow Implementation

## [#225: Demo 2: Splash Guy — minimal rendering / visualization path](https://github.com/flyingrobots/echo/issues/225)

- Status: Open
- Blocks:
    - [#226: Demo 2: Splash Guy — docs: networking-first course modules](https://github.com/flyingrobots/echo/issues/226)
    - Confidence: medium
    - Evidence: Inferred: Docs follow Implementation

## [#226: Demo 2: Splash Guy — docs: networking-first course modules](https://github.com/flyingrobots/echo/issues/226)

- Status: Open
- Blocked by:
    - [#222: Demo 2: Splash Guy — deterministic rules + state model](https://github.com/flyingrobots/echo/issues/222)
    - Confidence: medium
    - Evidence: Inferred: Docs follow Implementation
    - [#223: Demo 2: Splash Guy — lockstep input protocol + two-peer harness](https://github.com/flyingrobots/echo/issues/223)
    - Confidence: medium
    - Evidence: Inferred: Docs follow Implementation
    - [#224: Demo 2: Splash Guy — controlled desync lessons (make it fail on purpose)](https://github.com/flyingrobots/echo/issues/224)
    - Confidence: medium
    - Evidence: Inferred: Docs follow Implementation
    - [#225: Demo 2: Splash Guy — minimal rendering / visualization path](https://github.com/flyingrobots/echo/issues/225)
    - Confidence: medium
    - Evidence: Inferred: Docs follow Implementation

## [#231: Demo 3: Tumble Tower — Stage 0 physics (2D AABB stacking)](https://github.com/flyingrobots/echo/issues/231)

- Status: In Progress
- Blocks:
    - [#238: Demo 3: Tumble Tower — docs course (physics ladder)](https://github.com/flyingrobots/echo/issues/238)
    - Confidence: medium
    - Evidence: Inferred: Docs follow Implementation
    - [#232: Demo 3: Tumble Tower — Stage 1 physics (rotation + angular, OBB contacts)](https://github.com/flyingrobots/echo/issues/232)
    - Confidence: strong
    - Evidence: Inferred: Stage 1 physics depends on Stage 0
- Evidence: `crates/warp-geom` implements primitives (AABB, Transform), but solver logic for "stacking" is not yet visible in the top-level modules.

## [#232: Demo 3: Tumble Tower — Stage 1 physics (rotation + angular, OBB contacts)](https://github.com/flyingrobots/echo/issues/232)

- Status: Open
- Blocks:
    - [#238: Demo 3: Tumble Tower — docs course (physics ladder)](https://github.com/flyingrobots/echo/issues/238)
    - Confidence: medium
    - Evidence: Inferred: Docs follow Implementation
    - [#233: Demo 3: Tumble Tower — Stage 2 physics (friction + restitution)](https://github.com/flyingrobots/echo/issues/233)
    - Confidence: strong
    - Evidence: Inferred: Stage 2 physics depends on Stage 1
- Blocked by:
    - [#231: Demo 3: Tumble Tower — Stage 0 physics (2D AABB stacking)](https://github.com/flyingrobots/echo/issues/231)
    - Confidence: strong
    - Evidence: Inferred: Stage 1 physics depends on Stage 0

## [#233: Demo 3: Tumble Tower — Stage 2 physics (friction + restitution)](https://github.com/flyingrobots/echo/issues/233)

- Status: Open
- Blocks:
    - [#238: Demo 3: Tumble Tower — docs course (physics ladder)](https://github.com/flyingrobots/echo/issues/238)
    - Confidence: medium
    - Evidence: Inferred: Docs follow Implementation
    - [#234: Demo 3: Tumble Tower — Stage 3 physics (sleeping + stack stability)](https://github.com/flyingrobots/echo/issues/234)
    - Confidence: strong
    - Evidence: Inferred: Stage 3 physics depends on Stage 2
- Blocked by:
    - [#232: Demo 3: Tumble Tower — Stage 1 physics (rotation + angular, OBB contacts)](https://github.com/flyingrobots/echo/issues/232)
    - Confidence: strong
    - Evidence: Inferred: Stage 2 physics depends on Stage 1

## [#234: Demo 3: Tumble Tower — Stage 3 physics (sleeping + stack stability)](https://github.com/flyingrobots/echo/issues/234)

- Status: Open
- Blocks:
    - [#238: Demo 3: Tumble Tower — docs course (physics ladder)](https://github.com/flyingrobots/echo/issues/238)
    - Confidence: medium
    - Evidence: Inferred: Docs follow Implementation
- Blocked by:
    - [#233: Demo 3: Tumble Tower — Stage 2 physics (friction + restitution)](https://github.com/flyingrobots/echo/issues/233)
    - Confidence: strong
    - Evidence: Inferred: Stage 3 physics depends on Stage 2

## [#235: Demo 3: Tumble Tower — lockstep harness + per-tick fingerprinting](https://github.com/flyingrobots/echo/issues/235)

- Status: Open
- Blocks:
    - [#238: Demo 3: Tumble Tower — docs course (physics ladder)](https://github.com/flyingrobots/echo/issues/238)
    - Confidence: medium
    - Evidence: Inferred: Docs follow Implementation

## [#236: Demo 3: Tumble Tower — controlled desync breakers (physics edition)](https://github.com/flyingrobots/echo/issues/236)

- Status: Open
- Blocks:
    - [#238: Demo 3: Tumble Tower — docs course (physics ladder)](https://github.com/flyingrobots/echo/issues/238)
    - Confidence: medium
    - Evidence: Inferred: Docs follow Implementation

## [#237: Demo 3: Tumble Tower — visualization (2D view + debug overlays)](https://github.com/flyingrobots/echo/issues/237)

- Status: Open
- Blocks:
    - [#238: Demo 3: Tumble Tower — docs course (physics ladder)](https://github.com/flyingrobots/echo/issues/238)
    - Confidence: medium
    - Evidence: Inferred: Docs follow Implementation

## [#238: Demo 3: Tumble Tower — docs course (physics ladder)](https://github.com/flyingrobots/echo/issues/238)

- Status: Open
- Blocked by:
    - [#231: Demo 3: Tumble Tower — Stage 0 physics (2D AABB stacking)](https://github.com/flyingrobots/echo/issues/231)
    - Confidence: medium
    - Evidence: Inferred: Docs follow Implementation
    - [#232: Demo 3: Tumble Tower — Stage 1 physics (rotation + angular, OBB contacts)](https://github.com/flyingrobots/echo/issues/232)
    - Confidence: medium
    - Evidence: Inferred: Docs follow Implementation
    - [#233: Demo 3: Tumble Tower — Stage 2 physics (friction + restitution)](https://github.com/flyingrobots/echo/issues/233)
    - Confidence: medium
    - Evidence: Inferred: Docs follow Implementation
    - [#234: Demo 3: Tumble Tower — Stage 3 physics (sleeping + stack stability)](https://github.com/flyingrobots/echo/issues/234)
    - Confidence: medium
    - Evidence: Inferred: Docs follow Implementation
    - [#235: Demo 3: Tumble Tower — lockstep harness + per-tick fingerprinting](https://github.com/flyingrobots/echo/issues/235)
    - Confidence: medium
    - Evidence: Inferred: Docs follow Implementation
    - [#236: Demo 3: Tumble Tower — controlled desync breakers (physics edition)](https://github.com/flyingrobots/echo/issues/236)
    - Confidence: medium
    - Evidence: Inferred: Docs follow Implementation
    - [#237: Demo 3: Tumble Tower — visualization (2D view + debug overlays)](https://github.com/flyingrobots/echo/issues/237)
    - Confidence: medium
    - Evidence: Inferred: Docs follow Implementation

## [#239: Tooling: Reliving debugger UX (Constraint Lens + Provenance Heatmap)](https://github.com/flyingrobots/echo/issues/239)

- Status: Open
- (No detected dependencies)

## [#243: TT1: dt policy (fixed timestep vs admitted dt stream)](https://github.com/flyingrobots/echo/issues/243)

- Status: Open
- Blocks:
    - [#170: TT1: StreamsFrame inspector support (backlog + cursors + admission decisions)](https://github.com/flyingrobots/echo/issues/170)
    - Confidence: medium
    - Evidence: Inferred: TT1 Implementation blocks on TT1 Spec clarifications

## [#244: TT1: TimeStream retention + spool compaction + wormhole density](https://github.com/flyingrobots/echo/issues/244)

- Status: Open
- Blocks:
    - [#170: TT1: StreamsFrame inspector support (backlog + cursors + admission decisions)](https://github.com/flyingrobots/echo/issues/170)
    - Confidence: medium
    - Evidence: Inferred: TT1 Implementation blocks on TT1 Spec clarifications

## [#245: TT1: Merge semantics for admitted stream facts across worldlines](https://github.com/flyingrobots/echo/issues/245)

- Status: Open
- Blocks:
    - [#170: TT1: StreamsFrame inspector support (backlog + cursors + admission decisions)](https://github.com/flyingrobots/echo/issues/170)
    - Confidence: medium
    - Evidence: Inferred: TT1 Implementation blocks on TT1 Spec clarifications

## [#246: TT1: Security/capabilities for fork/rewind/merge in multiplayer](https://github.com/flyingrobots/echo/issues/246)

- Status: Open
- Blocks:
    - [#170: TT1: StreamsFrame inspector support (backlog + cursors + admission decisions)](https://github.com/flyingrobots/echo/issues/170)
    - Confidence: medium
    - Evidence: Inferred: TT1 Implementation blocks on TT1 Spec clarifications

## [#270: Hardening: Fuzz the ScenePort boundary (proptest)](https://github.com/flyingrobots/echo/issues/270)

- Status: Open
- Blocks:
    - [#21: Spec: Security Contexts (FFI/WASM/CLI)](https://github.com/flyingrobots/echo/issues/21)
    - Confidence: medium
    - Evidence: Hardening the port boundary provides evidence for security context enforcement.

## [#271: Performance: SIMD-accelerated canonicalization](https://github.com/flyingrobots/echo/issues/271)

- Status: Open
- (No detected dependencies)

## [#272: Tooling: Causal Visualizer (MockAdapter -> DOT)](https://github.com/flyingrobots/echo/issues/272)

- Status: Open
- (No detected dependencies)

## [#284: CI: Per-crate gate overrides in det-policy classification system](https://github.com/flyingrobots/echo/issues/284)

- Status: Open
- (No detected dependencies)

## [#285: CI: Auto-generate DETERMINISM_PATHS from det-policy.yaml DET_CRITICAL entries](https://github.com/flyingrobots/echo/issues/285)

- Status: Completed
- (No detected dependencies)

## [#286: CI: Add unit tests for classify_changes.cjs and matches()](https://github.com/flyingrobots/echo/issues/286)

- Status: Open
- (No detected dependencies)

## [#287: Docs: Document ban-nondeterminism.sh allowlist process in RELEASE_POLICY.md](https://github.com/flyingrobots/echo/issues/287)

- Status: Open
- (No detected dependencies)

## Backlog: Add BLD-001 claim for G4 build reproducibility

- Status: Open
- Evidence: `generate_evidence.cjs` has no claim entry for G4. `CLAIM_MAP.yaml` has no BLD-001 declaration. The `build-repro` job runs and `validate-evidence` checks artifact presence, but no VERIFIED/UNVERIFIED status is emitted into the evidence pack. The release policy blocker matrix references G4 but the evidence chain cannot enforce it.
- (No detected dependencies)

## Backlog: Add macOS parity claim (DET-002 is Linux-only)

- Status: Completed
- Evidence: `generate_evidence.cjs:33` maps DET-002 solely to `det-linux-artifacts`. The `det-macos-artifacts` are gathered and presence-validated, but no claim captures macOS parity results. A macOS-specific divergence would go undetected by the evidence system.
- (No detected dependencies)

## Backlog: Add concurrency controls to det-gates.yml

- Status: Completed
- Evidence: `det-gates.yml` has no `concurrency:` block. Multiple runs for the same PR can pile up, burning CI minutes. Standard fix: `concurrency: { group: det-gates-${{ github.head_ref || github.ref }}, cancel-in-progress: true }`.
- (No detected dependencies)

## Backlog: Expand #286 scope to cover validate_claims.cjs and generate_evidence.cjs

- Status: Open
- Blocked by:
    - [#286: CI: Add unit tests for classify_changes.cjs and matches()](https://github.com/flyingrobots/echo/issues/286)
    - Confidence: medium
    - Evidence: Both scripts now export their main functions (M1/M2 in det-hard). Edge cases to cover: 'local' sentinel, missing artifacts, malformed evidence JSON.

## Backlog: Simplify docs crate path list in det-policy.yaml

- Status: Completed
- Evidence: The `docs` entry in `det-policy.yaml` mixes directory globs with 20+ individual top-level filenames. Growing unwieldy; any new top-level file that doesn't match an existing crate pattern triggers `require_full_classification` failure. Consider a glob simplification or a catch-all mechanism.
- (No detected dependencies)

---

Rendering note (2026-01-09):

- The tasks DAG renderer intentionally omits isolated nodes (issues with no incoming or outgoing edges) to keep the visualization readable. The script computes `connectedNodeIds` from edges, builds `filteredNodes`, and logs how many nodes were dropped. See `scripts/generate-tasks-dag.js` for the filtering logic.
