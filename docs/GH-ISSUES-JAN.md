<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# GitHub Issues Audit - January 2026

**Audit Date**: 2026-01-17
**Total Open Issues**: 76
**Closeable**: 6
**Legitimately Open**: 70

---

## Executive Summary

| Category | Total | Open | Closeable | Confidence |
| ---------- | ------- | ------ | ----------- | ------------ |
| Time Travel (TT) | 13 | 13 | 0 | HIGH |
| Demo (Splash Guy + Tumble Tower) | 13 | 13 | 0 | HIGH |
| Wesley (W) | 4 | 4 | 0 | HIGH |
| Core/Runtime (M) | 7 | 5 | 2 | HIGH |
| Tooling/CLI | 18 | 18 | 0 | HIGH |
| Security | 10 | 6 | 4 | HIGH |
| Spec/Store/Plugin | 18 | 17 | 1 | HIGH |

### Issues Recommended for Closure

| Issue | Title | Evidence |
| ------- | ------- | ---------- |
| #189 | M4: Concurrency litmus suite | `dpo_concurrency_litmus.rs` implements 3 litmus families; SPEC-0003 documents mapping |
| #188 | M4: Kernel nondeterminism tripwires | `scripts/ban-nondeterminism.sh` + CI integration complete |
| #41 | README+docs (security) | TASKS-DAG.md marks complete; README.md + spec docs exist |
| #37 | Draft security contexts spec | `docs/spec-capabilities-and-security.md` covers capability tokens and fault codes |
| #32 | Draft signing spec | `docs/spec-capabilities-and-security.md` has SecurityEnvelope + Ed25519 spec |
| #103 | Policy: Require PR-Issue linkage | AGENTS.md lines 37-44 contain complete policy |

---

## Detailed Findings by Category

### 1. Time Travel Issues (TT) - 13 Open

All TT issues remain **legitimately open**. The spec foundation (`docs/spec-time-streams-and-wormholes.md`) is well-written but explicitly defers 4 questions as "Open Questions".

| Issue | Title | Status | Evidence | Confidence |
| ------- | ------- | -------- | ---------- | ------------ |
| #246 | TT1: Security/capabilities for fork/rewind/merge | **OPEN** | Listed as Open Question #1 in spec; no capability model for multiplayer fork/rewind/merge | HIGH |
| #245 | TT1: Merge semantics for stream facts | **OPEN** | Listed as Open Question #2 (high priority); paradox quarantine rules undefined | HIGH |
| #244 | TT1: TimeStream retention + spool compaction | **OPEN** | Listed as Open Question #4; no retention/compaction rules | HIGH |
| #243 | TT1: dt policy (fixed vs variable timestep) | **OPEN** | Listed as Open Question #3; no policy decision recorded | HIGH |
| #205 | TT2: Reliving debugger MVP | **OPEN** | No spec for event DAG/slice algorithm; blocked by #170 | HIGH |
| #204 | TT3: Provenance heatmap | **OPEN** | No spec for blast radius/cohesion; blocked by TT2 | HIGH |
| #203 | TT1: Constraint Lens panel | **OPEN** | No admission trace or counterfactual schema defined | HIGH |
| #199 | TT3: Wesley worldline diff | **OPEN** | No CLI UX or spec; blocked by TT2 | HIGH |
| #192 | TT0: TTL/deadline semantics | **OPEN** | No explicit expires_at_tick semantic TTL in TT0 docs | MEDIUM |
| #191 | TT0: Session stream time fields | **OPEN** | Session ordering fields not explicitly enumerated | MEDIUM |
| #172 | TT3: Rulial diff / worldline compare | **OPEN** | No export format or diff implementation; blocked by #171 | HIGH |
| #171 | TT2: Time Travel MVP | **OPEN** | Spec exists but no runtime API; blocked by #170 | HIGH |
| #170 | TT1: StreamsFrame inspector support | **OPEN** | Explicitly "PLANNED" in spec-editor-and-inspector.md; no code | HIGH |

**Dependency Chain**: TT0 spec questions (#243-246) → TT1 StreamsFrame (#170) → TT2 MVP (#171) → TT3 features (#172, #199, #204)

---

### 2. Demo Issues - 13 Open

All demo issues remain **legitimately open**. No demo game implementations exist.

#### Demo 3: Tumble Tower (Physics Ladder)

| Issue | Title | Status | Evidence | Confidence |
| ------- | ------- | -------- | ---------- | ------------ |
| #238 | Docs course (physics ladder) | **OPEN** | Only modules 00-01 exist; no physics-specific course | HIGH |
| #237 | Visualization (2D view + overlays) | **OPEN** | warp-viewer is general 3D; no 2D physics viewer | HIGH |
| #236 | Controlled desync breakers | **OPEN** | No physics-specific nondeterminism toggles | HIGH |
| #235 | Lockstep harness + fingerprinting | **OPEN** | DIND harness exists but not for Tumble Tower | HIGH |
| #234 | Stage 3 physics (sleeping + stability) | **OPEN** | No sleep/wake implementation | HIGH |
| #233 | Stage 2 physics (friction + restitution) | **OPEN** | No physics material system | HIGH |
| #232 | Stage 1 physics (rotation + OBB) | **OPEN** | No rotational physics | HIGH |
| #231 | Stage 0 physics (2D AABB stacking) | **OPEN** | warp-geom has AABB geometry only, no physics sim | HIGH |

#### Demo 2: Splash Guy (Grid Arena)

| Issue | Title | Status | Evidence | Confidence |
| ------- | ------- | -------- | ---------- | ------------ |
| #226 | Docs: networking-first course | **OPEN** | Modules 02-09 planned but not implemented | HIGH |
| #225 | Minimal rendering/visualization | **OPEN** | No Splash Guy renderer exists | HIGH |
| #224 | Controlled desync lessons | **OPEN** | No Splash Guy desync breakers | HIGH |
| #223 | Lockstep input protocol | **OPEN** | No input log format for Splash Guy | HIGH |
| #222 | Deterministic rules + state model | **OPEN** | No grid arena/balloon/explosion code | HIGH |

---

### 3. Wesley Issues (W) - 4 Open

All Wesley issues remain **legitimately open**. Scaffolding exists but core requirements unmet.

| Issue | Title | Status | Evidence | Confidence |
| ------- | ------- | -------- | ---------- | ------------ |
| #198 | W1: Provenance as query semantics | **OPEN** | No tick directive, proof objects, or deterministic cursors | HIGH |
| #194 | W1: SchemaDelta vocabulary | **OPEN** | No SchemaDelta enum, no dry-run patch planning | HIGH |
| #193 | W1: Schema hash chain pinning | **OPEN** | Only single-level schema_sha256; no full chain or receipts | HIGH |
| #174 | W1: Wesley as boundary grammar | **OPEN** | IR codegen exists but no grammar definition, AST normalization, or backends | HIGH |

---

### 4. Core/Runtime Issues (M) - 5 Open, 2 Closeable

| Issue | Title | Status | Evidence | Confidence |
| ------- | ------- | -------- | ---------- | ------------ |
| #190 | M4: Determinism torture harness | **OPEN** | Current torture is single-threaded only; no 1-thread vs N-thread | HIGH |
| #189 | M4: Concurrency litmus suite | **CLOSEABLE** | `dpo_concurrency_litmus.rs` implements 3 litmus families; SPEC-0003 documents | HIGH |
| #188 | M4: Kernel nondeterminism tripwires | **CLOSEABLE** | `scripts/ban-nondeterminism.sh` + CI at `.github/workflows/ci.yml:220` | HIGH |
| #187 | M4: Worldline convergence suite | **OPEN** | No proptest-driven patch-replay harness | HIGH |
| #186 | M1: Domain-separated digest for RenderGraph | **OPEN** | `echo-graph` uses plain blake3::hash() without derive-key | HIGH |
| #185 | M1: Domain-separated hash contexts | **OPEN** | `warp-core` uses Hasher::new() without derive-key contexts | HIGH |
| #173 | S1: Deterministic Rhai surface | **OPEN** | No Rhai crate dependency or embedding; FFI mentions only | HIGH |

---

### 5. Tooling/CLI Issues - 18 Open

All tooling issues remain **legitimately open**. The CLI is a placeholder.

#### CLI Core (#47-51)

| Issue | Title | Status | Evidence | Confidence |
| ------- | ------- | -------- | ---------- | ------------ |
| #51 | Docs/man pages | **OPEN** | warp-cli prints "Hello, world!" only | HIGH |
| #50 | Implement 'inspect' | **OPEN** | No subcommands implemented | HIGH |
| #49 | Implement 'bench' | **OPEN** | warp-benches exists but no CLI wrapper | HIGH |
| #48 | Implement 'verify' | **OPEN** | No verify subcommand | HIGH |
| #47 | Scaffold CLI subcommands | **OPEN** | main.rs is placeholder; no clap | HIGH |

#### Hot-Reload Epic (#75-78)

| Issue | Title | Status | Evidence | Confidence |
| ------- | ------- | -------- | ---------- | ------------ |
| #75 | Draft hot-reload spec | **OPEN** | No spec-editor-hot-reload.md | HIGH |
| #76 | File watcher/debounce | **OPEN** | No notify crate file watcher | HIGH |
| #77 | Atomic snapshot swap | **OPEN** | No version counter + deferred cleanup | HIGH |
| #78 | Editor gate + tests | **OPEN** | No cfg(editor) feature gate | HIGH |

#### Importer Epic (#80-84)

| Issue | Title | Status | Evidence | Confidence |
| ------- | ------- | -------- | ---------- | ------------ |
| #80 | Draft importer spec | **OPEN** | No spec-importer-turtlegraph.md | HIGH |
| #81 | Minimal reader | **OPEN** | No TurtleGraph bundle reader | HIGH |
| #82 | Echo store loader | **OPEN** | No importer store loader | HIGH |
| #83 | Integrity verification | **OPEN** | No importer-specific digest checking | HIGH |
| #84 | Sample + tests | **OPEN** | No sample bundle or tests | HIGH |

#### Other Tooling

| Issue | Title | Status | Evidence | Confidence |
| ------- | ------- | -------- | ---------- | ------------ |
| #79 | Docs/logging | **OPEN** | No execution-plan or decision-log docs | HIGH |
| #239 | Reliving debugger UX | **OPEN** | No ConstraintLens/ProvenanceHeatmap implementation | HIGH |
| #202 | Provenance Payload v1 | **OPEN** | No PP envelope spec or implementation | HIGH |
| #195 | JS-ABI checksum v2 | **OPEN** | No domain-separated BLAKE3 checksums; backlog | HIGH |

---

### 6. Security Issues - 6 Open, 4 Closeable

| Issue | Title | Status | Evidence | Confidence |
| ------- | ------- | -------- | ---------- | ------------ |
| #41 | README+docs | **CLOSEABLE** | TASKS-DAG.md marks complete; README.md + spec docs exist | HIGH |
| #40 | Unit tests for denials | **OPEN** | No comprehensive FFI/WASM denial test suite | HIGH |
| #39 | WASM input validation | **OPEN** | Partial validation in echo-wasm-abi; incomplete | MEDIUM |
| #38 | FFI limits and validation | **OPEN** | Basic null checks only; no caps or safe math | HIGH |
| #37 | Draft security contexts spec | **CLOSEABLE** | `docs/spec-capabilities-and-security.md` covers tokens + fault codes | HIGH |
| #36 | CI: verify signatures | **OPEN** | No signature verification in CI | HIGH |
| #35 | Key management doc | **OPEN** | No key storage/rotation documentation | HIGH |
| #34 | CLI verify path | **OPEN** | warp-cli is placeholder | HIGH |
| #33 | CI: sign release artifacts | **OPEN** | No artifact signing in CI | HIGH |
| #32 | Draft signing spec | **CLOSEABLE** | `docs/spec-capabilities-and-security.md` has SecurityEnvelope + Ed25519 | HIGH |

---

### 7. Spec/Store/Plugin Issues - 17 Open, 1 Closeable

#### Persistent Store Spec (#19, #27-30)

| Issue | Title | Status | Evidence | Confidence |
| ------- | ------- | -------- | ---------- | ------------ |
| #30 | Prototype decoder | **OPEN** | No persistent store decoder | HIGH |
| #29 | Prototype encoder | **OPEN** | No persistent store encoder | HIGH |
| #28 | Draft spec document | **OPEN** | No spec-persistent-store.md | HIGH |
| #27 | Golden test vectors | **OPEN** | golden-vectors.md is for CBOR, not header/tables | HIGH |
| #19 | Spec: Persistent Store | **OPEN** | Persistent store format is "Planned" | HIGH |

#### Plugin ABI (#26, #85-89)

| Issue | Title | Status | Evidence | Confidence |
| ------- | ------- | -------- | ---------- | ------------ |
| #89 | Example plugin + tests | **OPEN** | No toy plugin or CI test | HIGH |
| #88 | Capability tokens | **OPEN** | Spec exists but no runtime enforcement | HIGH |
| #87 | Version negotiation | **OPEN** | No plugin ABI version handshake | HIGH |
| #86 | C header + host loader | **OPEN** | No .h file; warp-ffi README notes follow-up | HIGH |
| #85 | Draft C ABI spec | **OPEN** | No spec-plugin-abi-c.md | HIGH |
| #26 | Plugin ABI (C) v0 | **OPEN** | warp-ffi is minimal; no capability tokens or negotiation | HIGH |

#### Other Specs

| Issue | Title | Status | Evidence | Confidence |
| ------- | ------- | -------- | ---------- | ------------ |
| #207 | Noisy-line naming test | **OPEN** | Manual task; no decision-log.md | HIGH |
| #103 | Policy: PR-Issue linkage | **CLOSEABLE** | AGENTS.md lines 37-44 contain complete policy | HIGH |
| #25 | Importer: TurtlGraph | **OPEN** | No importer implementation | HIGH |
| #24 | Editor Hot-Reload | **OPEN** | Mentioned as "Future Extension" | HIGH |
| #23 | CLI: verify/bench/inspect | **OPEN** | warp-cli is placeholder | HIGH |
| #22 | Benchmarks & CI Gates | **OPEN** | warp-benches exists but no CI gates | HIGH |
| #21 | Spec: Security Contexts | **OPEN** | No FFI/WASM/CLI security contexts spec | HIGH |
| #20 | Spec: Commit/Manifest Signing | **OPEN** | Spec exists but no implementation | HIGH |

---

## Project Board Analysis

**Project**: Echo (Project #9)

### Status Distribution

| Status | Count | Percentage |
| -------- | ------- | ------------ |
| Done | 24 | 33.8% |
| Blocked | 8 | 11.3% |
| No Status | 39 | 54.9% |
| In Progress | 0 | 0% |
| Todo | 0 | 0% |

### Key Findings

1. **No active work tracked**: Zero items "In Progress" or "Todo"
2. **High "No Status" count**: 55% of items unassigned
3. **All Done items are PRs**: 24 merged Pull Requests
4. **Coverage gaps**: 37 open issues (44%) not on board, including entire Demo milestones

### Blocked Items (8)

| Issue | Title | Milestone |
| ------- | ------- | ----------- |
| #26 | Plugin ABI (C) v0 | 1C - Rhai/TS Bindings |
| #25 | Importer: TurtlGraph | 1F - Tooling Integration |
| #24 | Editor Hot-Reload | 1F - Tooling Integration |
| #23 | CLI: verify/bench/inspect | M2.2 - Playground Slice |
| #22 | Benchmarks & CI Gates | M1 - Golden Tests |
| #21 | Spec: Security Contexts | 1E - Networking & Confluence |
| #20 | Spec: Commit/Manifest Signing | 1F - Tooling Integration |
| #19 | Spec: Persistent Store | 1F - Tooling Integration |

### Untracked Issues (37)

Notable gaps:

- **Demo 2 (Splash Guy)**: 5 issues not on board (#222-226)
- **Demo 3 (Tumble Tower)**: 8 issues not on board (#231-238)
- **Newer M1/M4 issues**: #185-190 not tracked
- **Several backlog items**: #195, #202, #207, #239

---

## Recommendations

### Immediate Actions

1. **Close 6 issues**: #189, #188, #41, #37, #32, #103
2. **Triage No Status items**: Assign proper status to 39 unassigned board items
3. **Add Demo milestones to board**: Demo 2 and Demo 3 issues should be tracked

### Maintenance

1. **Review blocked items**: Determine if blockers resolved or should be backlogged
2. **Add newer issues**: Issues #185+ should be added to board
3. **Use In Progress/Todo**: Start using these statuses to reflect workflow

### Priority Focus Areas

Based on dependency chains:

1. **TT1 StreamsFrame (#170)** - Blocks TT2 and TT3 milestones
2. **Wesley boundary grammar (#174)** - Blocks W1 milestone
3. **Domain-separated hashing (#185, #186)** - Security foundation
4. **CLI scaffolding (#47)** - Enables verify/bench/inspect

---

Generated by automated audit on 2026-01-17
