<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Scene Port Adapter — Task Breakdown

**Plan:** `docs/plans/scene-port-adapter-plan.md`
**DAG:** `docs/assets/dags/scene-port-adapter-dag-elk.svg`
**Plan ID:** SPA

---

## Overview

This document breaks down the Scene Port Adapter implementation into granular tasks with clear requirements, acceptance criteria, and scope boundaries. Each task is designed to be completable in a single focused session.

**Legend:**

- 🟢 = Ready to start (no blockers)
- 🟡 = Blocked by dependencies
- ✅ = Complete

---

## Phase 1: Rust Port Crate (`echo-scene-port`)

### SPA.1.1 — Create Crate Structure

**Status:** 🟢 Ready

**Description:**
Bootstrap the `echo-scene-port` crate with Cargo.toml and lib.rs shell. This crate defines the pure domain contract between TTD Controller and renderers.

**Requirements:**

- [ ] Create `crates/echo-scene-port/Cargo.toml`
- [ ] Create `crates/echo-scene-port/src/lib.rs` with module declarations
- [ ] Add crate to workspace `Cargo.toml`
- [ ] Configure `no_std` support with `std` as default feature

**Acceptance Criteria:**

- [ ] `cargo build -p echo-scene-port` passes
- [ ] `cargo build -p echo-scene-port --no-default-features` passes
- [ ] `cargo tree -p echo-scene-port` shows zero external dependencies
- [ ] No serde in dependency tree

**Scope:**

- ✅ In scope: Cargo.toml, lib.rs shell, workspace integration
- ❌ Not in scope: Type implementations (separate tasks)

**Files:**

````text
crates/echo-scene-port/
├── Cargo.toml
└── src/
    └── lib.rs
```text

---

### SPA.1.2 — Implement types.rs

**Status:** 🟡 Blocked by SPA.1.1

**Description:**
Implement core scene types: `Hash`, `NodeKey`, `EdgeKey`, `LabelKey`, `NodeShape`, `EdgeStyle`, `ColorRgba8`, `NodeDef`, `EdgeDef`, `LabelDef`, `LabelAnchor`, `SceneOp`, `SceneDelta`.

**Requirements:**

- [ ] Define `Hash = [u8; 32]` and key type aliases
- [ ] Define `NodeShape` enum (Sphere, Cube)
- [ ] Define `EdgeStyle` enum (Solid, Dashed)
- [ ] Define `ColorRgba8 = [u8; 4]`
- [ ] Define `NodeDef` struct with key, position, radius, shape, color
- [ ] Define `EdgeDef` struct with key, a, b, width, style, color
- [ ] Define `LabelAnchor` enum (Node, World)
- [ ] Define `LabelDef` struct with key, text, font_size, color, anchor, offset
- [ ] Define `SceneOp` enum (UpsertNode, RemoveNode, UpsertEdge, RemoveEdge, UpsertLabel, RemoveLabel, Clear)
- [ ] Define `SceneDelta` struct with session_id, cursor_id, epoch, ops

**Acceptance Criteria:**

- [ ] All types derive `Clone, Debug, PartialEq`
- [ ] All types use `alloc` (Vec, String) not `std`
- [ ] No `Option` fields except where spec requires
- [ ] Compiles with `--no-default-features`

**Scope:**

- ✅ In scope: MVP types only (no MoveNode, no ClearLayer, no layers, no materials)
- ❌ Not in scope: Serde derives, serialization, validation logic

**Files:**

```text
crates/echo-scene-port/src/types.rs
```text

---

### SPA.1.3 — Implement camera.rs

**Status:** 🟡 Blocked by SPA.1.1

**Description:**
Implement `CameraState` and `ProjectionKind` types for camera configuration.

**Requirements:**

- [ ] Define `ProjectionKind` enum (Perspective, Orthographic)
- [ ] Define `CameraState` struct with position, target, up, projection, fov_y_radians, ortho_scale, near, far
- [ ] Implement `Default` for `CameraState`

**Acceptance Criteria:**

- [ ] Default camera: position [0,0,5], target [0,0,0], up [0,1,0], perspective, 60° FOV
- [ ] All fields are non-optional (spec provides defaults)
- [ ] Derives `Clone, Debug, PartialEq`

**Scope:**

- ✅ In scope: CameraState, ProjectionKind, Default impl
- ❌ Not in scope: Camera interpolation, animation

**Files:**

```text
crates/echo-scene-port/src/camera.rs
```text

---

### SPA.1.4 — Implement highlight.rs

**Status:** 🟡 Blocked by SPA.1.2

**Description:**
Implement `HighlightState` for selection and hover feedback. This is UI state, separate from scene ops.

**Requirements:**

- [ ] Define `HighlightState` struct with selected_nodes, selected_edges, hovered_node, hovered_edge
- [ ] Implement `Default` for `HighlightState` (empty selections)

**Acceptance Criteria:**

- [ ] `selected_nodes` and `selected_edges` are `Vec<_>` (multi-select)
- [ ] `hovered_node` and `hovered_edge` are `Option<_>` (at most one)
- [ ] Default is all empty/None
- [ ] Uses `NodeKey` and `EdgeKey` from types.rs

**Scope:**

- ✅ In scope: HighlightState struct, Default impl
- ❌ Not in scope: focusNode (Phase 2), highlight colors

**Files:**

```text
crates/echo-scene-port/src/highlight.rs
```text

---

### SPA.1.5 — Implement canon.rs

**Status:** 🟡 Blocked by SPA.1.1

**Description:**
Implement float canonicalization helpers for deterministic comparison and hashing.

**Requirements:**

- [ ] Implement `canonicalize_f32(x: f32) -> f32`
    - Truncate to 6 decimal places
    - Convert -0.0 to 0.0
    - Panic on NaN/Infinity
- [ ] Implement `canonicalize_position(p: [f32; 3]) -> [f32; 3]`
- [ ] Document that these are for comparison/hashing, not mutation

**Acceptance Criteria:**

- [ ] `canonicalize_f32(-0.0) == 0.0`
- [ ] `canonicalize_f32(1.23456789) == 1.234567`
- [ ] `canonicalize_f32(f32::NAN)` panics
- [ ] `canonicalize_f32(f32::INFINITY)` panics

**Scope:**

- ✅ In scope: Float canonicalization for codec layer
- ❌ Not in scope: Actually using these in type constructors

**Files:**

```text
crates/echo-scene-port/src/canon.rs
```text

---

### SPA.1.6 — Implement port.rs (ScenePort trait)

**Status:** 🟡 Blocked by SPA.1.2, SPA.1.3, SPA.1.4

**Description:**
Define the `ScenePort` trait that all renderer adapters must implement.

**Requirements:**

- [ ] Define `ScenePort` trait with:
    - `fn apply_scene_delta(&mut self, delta: &SceneDelta)`
    - `fn set_camera(&mut self, camera: &CameraState)`
    - `fn set_highlight(&mut self, highlight: &HighlightState)`
    - `fn render(&mut self)`
    - `fn resize(&mut self, width: u32, height: u32, dpr: f32)`
    - `fn reset_cursor(&mut self, cursor_id: &[u8; 32])`
    - `fn dispose(&mut self)`
- [ ] Document each method's contract

**Acceptance Criteria:**

- [ ] Trait is object-safe (can be used as `dyn ScenePort`)
- [ ] `render()` takes no parameters (profiling is adapter concern)
- [ ] `reset_cursor` documented as "epoch tracking only, not scene state"
- [ ] All methods take `&mut self`

**Scope:**

- ✅ In scope: Trait definition, documentation
- ❌ Not in scope: RenderContext, FrameResult (adapter-local)

**Files:**

```text
crates/echo-scene-port/src/port.rs
```text

---

## Phase 2: Rust Codec Crate (`echo-scene-codec`)

### SPA.2.1 — Create Codec Crate Structure

**Status:** 🟡 Blocked by SPA.1.6

**Description:**
Bootstrap the `echo-scene-codec` crate for CBOR serialization. This crate depends on `echo-scene-port` and adds serialization.

**Requirements:**

- [ ] Create `crates/echo-scene-codec/Cargo.toml`
- [ ] Add dependency on `echo-scene-port`
- [ ] Add dependency on `minicbor` with alloc feature
- [ ] Create `src/lib.rs` with module declarations
- [ ] Add crate to workspace

**Acceptance Criteria:**

- [ ] `cargo build -p echo-scene-codec` passes
- [ ] Depends on `echo-scene-port` (path dependency)
- [ ] Depends on `minicbor` for CBOR

**Scope:**

- ✅ In scope: Crate structure, dependencies
- ❌ Not in scope: Codec implementations

**Files:**

```text
crates/echo-scene-codec/
├── Cargo.toml
└── src/
    └── lib.rs
```text

---

### SPA.2.2 — Implement CBOR Codec

**Status:** 🟡 Blocked by SPA.2.1

**Description:**
Implement CBOR encode/decode for all `echo-scene-port` types using minicbor.

**Requirements:**

- [ ] Implement `Encode` and `Decode` for all types
- [ ] Use canonical CBOR encoding (sorted keys, minimal integers)
- [ ] Handle `SceneOp` as tagged union
- [ ] Export `encode_delta`, `decode_delta` convenience functions

**Acceptance Criteria:**

- [ ] Roundtrip: encode(x) → decode → x for all types
- [ ] Encoding is deterministic (same input = same bytes)
- [ ] Empty delta encodes to minimal bytes

**Scope:**

- ✅ In scope: CBOR codec for all port types
- ❌ Not in scope: JSON codec, MessagePack

**Files:**

```text
crates/echo-scene-codec/src/cbor.rs
```text

---

### SPA.2.3 — Implement MockAdapter

**Status:** 🟡 Blocked by SPA.2.2

**Description:**
Implement a headless `MockAdapter` that implements `ScenePort` for testing. Applies deltas to in-memory maps without GPU.

**Requirements:**

- [ ] Implement `MockAdapter` struct with HashMaps for nodes, edges, labels
- [ ] Implement `ScenePort` trait
- [ ] Track epoch per cursor_id
- [ ] Provide getters for counts: `node_count()`, `edge_count()`, `label_count()`
- [ ] Implement `reset_cursor` (clears epoch tracking only)

**Acceptance Criteria:**

- [ ] `apply_scene_delta` updates maps correctly
- [ ] Duplicate epoch is no-op (idempotent)
- [ ] `reset_cursor` allows epoch restart
- [ ] `render()` is no-op (no GPU)
- [ ] Two MockAdapters given same deltas have identical state

**Scope:**

- ✅ In scope: Headless state tracking, epoch management
- ❌ Not in scope: Visual rendering, GPU operations

**Files:**

```text
crates/echo-scene-codec/src/mock_adapter.rs
```text

---

### SPA.2.4 — Write Codec Roundtrip Tests

**Status:** 🟡 Blocked by SPA.2.2

**Description:**
Write tests verifying CBOR encode/decode roundtrip for all types.

**Requirements:**

- [ ] Test roundtrip for `NodeDef`, `EdgeDef`, `LabelDef`
- [ ] Test roundtrip for `SceneOp` (all variants)
- [ ] Test roundtrip for `SceneDelta` (empty, single op, multiple ops)
- [ ] Test roundtrip for `CameraState`, `HighlightState`
- [ ] Test encoding determinism (same input = same bytes)

**Acceptance Criteria:**

- [ ] All roundtrip tests pass
- [ ] Determinism test encodes twice and compares bytes

**Scope:**

- ✅ In scope: Roundtrip tests, determinism tests
- ❌ Not in scope: Fuzz testing, malformed input testing

**Files:**

```text
crates/echo-scene-codec/tests/roundtrip_tests.rs
```text

---

### SPA.2.5 — Write MockAdapter Tests

**Status:** 🟡 Blocked by SPA.2.3

**Description:**
Write tests verifying MockAdapter correctly applies deltas and handles epoch semantics.

**Requirements:**

- [ ] Test UpsertNode → node appears
- [ ] Test RemoveNode → node removed, attached labels removed
- [ ] Test UpsertEdge → edge appears
- [ ] Test edge with missing endpoint → edge tracked but marked invalid
- [ ] Test Clear → all maps empty
- [ ] Test epoch idempotency (same epoch = no-op)
- [ ] Test `reset_cursor` → allows epoch restart
- [ ] Test two cursors have independent epochs

**Acceptance Criteria:**

- [ ] All tests pass
- [ ] Tests are deterministic (no timing dependencies)

**Scope:**

- ✅ In scope: State machine tests, epoch tests
- ❌ Not in scope: Performance tests, stress tests

**Files:**

```text
crates/echo-scene-codec/tests/mock_adapter_tests.rs
```text

---

## Phase 3: TypeScript Package Bootstrap

### SPA.3.1 — Create Package Structure

**Status:** 🟢 Ready (can parallel with Rust work)

**Description:**
Bootstrap the `echo-renderer-three` TypeScript package with build tooling.

**Requirements:**

- [ ] Create `packages/echo-renderer-three/package.json`
- [ ] Create `tsconfig.json` (ES modules, strict mode)
- [ ] Create `vitest.config.ts`
- [ ] Create `src/index.ts` (empty exports)
- [ ] Add `three` as peer dependency

**Acceptance Criteria:**

- [ ] `pnpm install` succeeds
- [ ] `pnpm tsc --noEmit` passes
- [ ] `pnpm test` runs (even if no tests yet)
- [ ] Package exports ESM

**Scope:**

- ✅ In scope: Package structure, build config, test config
- ❌ Not in scope: Type definitions, implementations

**Files:**

```text
packages/echo-renderer-three/
├── package.json
├── tsconfig.json
├── vitest.config.ts
└── src/
    └── index.ts
```text

---

### SPA.3.2 — Implement SceneDelta.ts Types

**Status:** 🟡 Blocked by SPA.3.1, mirrors SPA.1.2

**Description:**
Implement TypeScript types mirroring Rust `types.rs`. Use `Uint8Array` for Hash types.

**Requirements:**

- [ ] Define `Hash = Uint8Array` (length 32)
- [ ] Define `NodeKey`, `EdgeKey`, `LabelKey` as Hash aliases
- [ ] Implement `hashToHex(h: Hash): string` helper
- [ ] Implement `hexToHash(hex: string): Hash` helper
- [ ] Define `NodeShape` enum
- [ ] Define `EdgeStyle` enum
- [ ] Define `ColorRgba8` type
- [ ] Define `NodeDef`, `EdgeDef`, `LabelDef`, `LabelAnchor` interfaces
- [ ] Define `SceneOp` discriminated union
- [ ] Define `SceneDelta` interface

**Acceptance Criteria:**

- [ ] Types match Rust definitions (manual review)
- [ ] `hashToHex` produces 64-char lowercase hex string
- [ ] `hexToHash` is inverse of `hashToHex`
- [ ] No `any` types

**Scope:**

- ✅ In scope: Type definitions, hash helpers
- ❌ Not in scope: Validation, CBOR codec

**Files:**

```text
packages/echo-renderer-three/src/types/SceneDelta.ts
```text

---

### SPA.3.3 — Implement CameraState.ts

**Status:** 🟡 Blocked by SPA.3.1, mirrors SPA.1.3

**Description:**
Implement TypeScript types for camera state.

**Requirements:**

- [ ] Define `ProjectionKind` type ("Perspective" | "Orthographic")
- [ ] Define `CameraState` interface
- [ ] Export `DEFAULT_CAMERA` constant

**Acceptance Criteria:**

- [ ] Matches Rust `CameraState` structure
- [ ] `DEFAULT_CAMERA` matches Rust defaults

**Scope:**

- ✅ In scope: Type definition, default constant
- ❌ Not in scope: Camera math, interpolation

**Files:**

```text
packages/echo-renderer-three/src/types/CameraState.ts
```text

---

### SPA.3.4 — Implement HighlightState.ts

**Status:** 🟡 Blocked by SPA.3.2, mirrors SPA.1.4

**Description:**
Implement TypeScript types for highlight state.

**Requirements:**

- [ ] Define `HighlightState` interface
- [ ] Export `EMPTY_HIGHLIGHT` constant

**Acceptance Criteria:**

- [ ] Uses `NodeKey[]` and `EdgeKey[]` for selections
- [ ] Uses `NodeKey | undefined` for hover
- [ ] `EMPTY_HIGHLIGHT` has empty arrays

**Scope:**

- ✅ In scope: Type definition, empty constant
- ❌ Not in scope: Highlight logic

**Files:**

```text
packages/echo-renderer-three/src/types/HighlightState.ts
```text

---

### SPA.3.5 — Implement types/index.ts

**Status:** 🟡 Blocked by SPA.3.2, SPA.3.3, SPA.3.4

**Description:**
Create the types index with exports and adapter-local types (RenderContext, Profiler, ScenePort interface).

**Requirements:**

- [ ] Re-export all types from SceneDelta.ts, CameraState.ts, HighlightState.ts
- [ ] Define `RenderContext` interface (frameIndex, timeSeconds, dtSeconds, width, height, dpr)
- [ ] Define `Profiler` interface (markStart, markEnd)
- [ ] Export `NULL_PROFILER` constant (no-op implementation)
- [ ] Define `ScenePort` interface (TypeScript version)

**Acceptance Criteria:**

- [ ] All public types exported
- [ ] `RenderContext` is adapter-local (not in Rust port crate)
- [ ] `NULL_PROFILER` has no-op methods
- [ ] `ScenePort` interface matches Rust trait

**Scope:**

- ✅ In scope: Type exports, adapter-local types
- ❌ Not in scope: FrameResult (removed from MVP)

**Files:**

```text
packages/echo-renderer-three/src/types/index.ts
```text

---

## Phase 4: TypeScript SceneState

### SPA.4.1 — Implement SceneState.ts

**Status:** 🟡 Blocked by SPA.3.2

**Description:**
Implement pure state machine for applying scene ops. No Three.js dependency.

**Requirements:**

- [ ] Create `SceneState` class with Maps for nodes, edges, labels (string keys via hashToHex)
- [ ] Implement `apply(ops: SceneOp[]): void`
- [ ] Handle all SceneOp variants:
    - UpsertNode → set in map
    - RemoveNode → delete from map, delete anchored labels
    - UpsertEdge → set in map
    - RemoveEdge → delete from map
    - UpsertLabel → set in map
    - RemoveLabel → delete from map
    - Clear → clear all maps
- [ ] Implement `isEdgeValid(edge: EdgeDef): boolean`

**Acceptance Criteria:**

- [ ] Pure TypeScript, no Three.js imports
- [ ] Maps use `hashToHex(key)` as string keys
- [ ] RemoveNode cascades to anchored labels
- [ ] Clear empties all three maps

**Scope:**

- ✅ In scope: State machine, map operations
- ❌ Not in scope: Three.js objects, rendering

**Files:**

```text
packages/echo-renderer-three/src/adapter/SceneState.ts
```text

---

### SPA.4.2 — Write SceneState Tests

**Status:** 🟡 Blocked by SPA.4.1

**Description:**
Write unit tests for SceneState.

**Requirements:**

- [ ] Test UpsertNode creates entry
- [ ] Test UpsertNode replaces existing entry
- [ ] Test RemoveNode deletes entry
- [ ] Test RemoveNode cascades to labels with Node anchor
- [ ] Test UpsertEdge creates entry
- [ ] Test RemoveEdge deletes entry
- [ ] Test Clear empties all maps
- [ ] Test isEdgeValid returns false when endpoint missing

**Acceptance Criteria:**

- [ ] All tests pass
- [ ] Tests use mock Hash values (can be any 32 bytes)
- [ ] No flaky tests

**Scope:**

- ✅ In scope: Unit tests for state machine
- ❌ Not in scope: Integration tests, Three.js tests

**Files:**

```text
packages/echo-renderer-three/src/__tests__/sceneState.test.ts
```text

---

## Phase 5: TypeScript ShaderManager

### SPA.5.1 — Port ShaderManager.ts

**Status:** 🟡 Blocked by SPA.3.1

**Description:**
Port ShaderManager from original codebase, removing singletons and time dependencies.

**Requirements:**

- [ ] Create `ShaderManager` class (not singleton)
- [ ] Implement `#include` directive processing
- [ ] Port shader chunks (fresnel, env-sample, etc.)
- [ ] Remove `uTime` uniform from all chunks
- [ ] Remove film grain noise
- [ ] If noise needed, require explicit seed parameter

**Acceptance Criteria:**

- [ ] `new ShaderManager()` works (no getInstance)
- [ ] `#include <fresnel>` resolves correctly
- [ ] Grep for `performance.now` returns zero matches
- [ ] Grep for `Math.random` returns zero matches

**Scope:**

- ✅ In scope: Shader composition, chunk registry
- ❌ Not in scope: Time-based effects, film grain

**Files:**

```text
packages/echo-renderer-three/src/shaders/ShaderManager.ts
packages/echo-renderer-three/src/shaders/chunks/*.glsl (if needed)
```text

---

## Phase 6: TypeScript AssetManager

### SPA.6.1 — Port AssetManager.ts

**Status:** 🟡 Blocked by SPA.3.1

**Description:**
Port AssetManager with URL-based keying (not content hash).

**Requirements:**

- [ ] Create `AssetManager` class (not singleton)
- [ ] Constructor takes optional `THREE.LoadingManager`
- [ ] Implement `getTexture(url: string): THREE.Texture`
- [ ] Cache textures by URL
- [ ] Implement `dispose(): void` to release all textures

**Acceptance Criteria:**

- [ ] `new AssetManager()` works (no getInstance)
- [ ] Same URL returns same texture instance
- [ ] Two AssetManager instances don't share cache
- [ ] `dispose()` calls `texture.dispose()` on all cached textures

**Scope:**

- ✅ In scope: URL-keyed texture cache
- ❌ Not in scope: Content-addressed hashing, cubemaps (Phase 2)

**Files:**

```text
packages/echo-renderer-three/src/assets/AssetManager.ts
```text

---

## Phase 7: TypeScript RenderCore

### SPA.7.1 — Implement ThreeRenderCore.ts

**Status:** 🟡 Blocked by SPA.3.1

**Description:**
Implement WebGLRenderer wrapper with no RAF ownership and no timing calls.

**Requirements:**

- [ ] Create `ThreeRenderCore` class
- [ ] Constructor takes `canvas: HTMLCanvasElement` and options
- [ ] Options: `antialias?: boolean`, `pixelRatio?: number`
- [ ] Implement `render(scene: THREE.Scene, camera: THREE.Camera): void`
- [ ] Implement `resize(width: number, height: number, dpr: number): void`
- [ ] Implement `dispose(): void`

**Acceptance Criteria:**

- [ ] No `requestAnimationFrame` calls
- [ ] No `performance.now` calls
- [ ] `render()` only takes scene and camera
- [ ] Grep for banned patterns returns zero

**Scope:**

- ✅ In scope: WebGLRenderer wrapper, resize handling
- ❌ Not in scope: Post-processing (Phase 2), multi-scene

**Files:**

```text
packages/echo-renderer-three/src/core/ThreeRenderCore.ts
```text

---

### SPA.7.2 — Implement CameraController.ts

**Status:** 🟡 Blocked by SPA.3.3

**Description:**
Implement camera controller that applies CameraState to Three.js cameras.

**Requirements:**

- [ ] Create `CameraController` class
- [ ] Maintain internal PerspectiveCamera and OrthographicCamera
- [ ] Implement `apply(state: CameraState, aspect: number): THREE.Camera`
- [ ] Switch camera type based on `state.projection`
- [ ] Update position, target (lookAt), up, FOV/orthoScale, near, far

**Acceptance Criteria:**

- [ ] Perspective camera uses fovYRadians (converted to degrees)
- [ ] Orthographic camera uses orthoScale for bounds
- [ ] `apply()` returns the active camera
- [ ] Camera matrices updated via `updateProjectionMatrix()`

**Scope:**

- ✅ In scope: Camera state application
- ❌ Not in scope: Camera animation, orbit controls

**Files:**

```text
packages/echo-renderer-three/src/core/CameraController.ts
```text

---

## Phase 8: TypeScript Basic Renderers

### SPA.8.1 — Implement NodeRenderer.ts

**Status:** 🟡 Blocked by SPA.3.2, SPA.4.1

**Description:**
Implement basic node rendering with spheres and boxes.

**Requirements:**

- [ ] Create `NodeRenderer` class
- [ ] Maintain `meshes: Map<string, THREE.Mesh>` (hex keys)
- [ ] Create shared SphereGeometry and BoxGeometry
- [ ] Implement `sync(nodes: Map<string, NodeDef>, scene: THREE.Scene): void`
    - Remove meshes for deleted nodes
    - Create meshes for new nodes
    - Update position, scale, color for existing nodes
- [ ] Implement `dispose(): void`

**Acceptance Criteria:**

- [ ] Sphere nodes render as spheres
- [ ] Cube nodes render as boxes
- [ ] Scale is set from `radius`
- [ ] Color is set from `color` (RGBA)
- [ ] Transparent when alpha < 255

**Scope:**

- ✅ In scope: Basic mesh rendering
- ❌ Not in scope: Instanced rendering (Phase 2), custom materials

**Files:**

```text
packages/echo-renderer-three/src/objects/NodeRenderer.ts
```text

---

### SPA.8.2 — Implement EdgeRenderer.ts

**Status:** 🟡 Blocked by SPA.3.2, SPA.4.1

**Description:**
Implement basic edge rendering with lines.

**Requirements:**

- [ ] Create `EdgeRenderer` class
- [ ] Maintain `lines: Map<string, THREE.Line>` (hex keys)
- [ ] Implement `sync(edges, nodes, scene)`:
    - Remove lines for deleted edges
    - Create lines for new edges
    - Update geometry when node positions change
    - Hide lines with missing endpoints (visible = false)
- [ ] Implement `dispose(): void`

**Acceptance Criteria:**

- [ ] Edges render as lines between node positions
- [ ] Edge with missing endpoint is hidden, not error
- [ ] Color is set from `color`
- [ ] Line width is ignored (WebGL limitation) — document this

**Scope:**

- ✅ In scope: Basic line rendering
- ❌ Not in scope: Tubes (Phase 2), dashed lines (Phase 2), arrows

**Files:**

```text
packages/echo-renderer-three/src/objects/EdgeRenderer.ts
```text

---

### SPA.8.3 — Implement LabelRenderer.ts

**Status:** 🟡 Blocked by SPA.3.2, SPA.4.1

**Description:**
Implement basic label rendering with CanvasTexture sprites.

**Requirements:**

- [ ] Create `LabelRenderer` class
- [ ] Maintain `sprites: Map<string, THREE.Sprite>` (hex keys)
- [ ] Implement `createLabelCanvas(def: LabelDef): HTMLCanvasElement`
- [ ] Implement `sync(labels, nodes, scene)`:
    - Remove sprites for deleted labels
    - Create sprites for new labels
    - Update position based on anchor (Node or World) + offset
    - Skip labels with missing node anchor
- [ ] Implement `dispose(): void`

**Acceptance Criteria:**

- [ ] Labels render as billboard sprites
- [ ] Text rendered via Canvas 2D API
- [ ] Labels track anchor node position
- [ ] Labels with missing anchor are skipped (no error)

**Scope:**

- ✅ In scope: CanvasTexture sprites
- ❌ Not in scope: SDF fonts (Phase 2), rich text

**Files:**

```text
packages/echo-renderer-three/src/objects/LabelRenderer.ts
```text

---

## Phase 9: TypeScript HighlightRenderer

### SPA.9.1 — Implement HighlightRenderer.ts

**Status:** 🟡 Blocked by SPA.3.4, SPA.8.1, SPA.8.2

**Description:**
Implement highlight rendering with color tinting (no outlines).

**Requirements:**

- [ ] Create `HighlightRenderer` class
- [ ] Track original colors for restoration
- [ ] Implement `apply(highlight, nodeMeshes, edgeLines)`:
    - Reset all materials to original colors
    - Apply yellow tint to selected nodes/edges
    - Apply cyan tint to hovered node/edge
- [ ] Use `color.lerp()` for tinting

**Acceptance Criteria:**

- [ ] Selected items get yellow tint
- [ ] Hovered items get cyan tint
- [ ] Clearing highlight restores original colors
- [ ] No outline/stencil complexity

**Scope:**

- ✅ In scope: Color tinting
- ❌ Not in scope: Outlines (Phase 2), glow effects

**Files:**

```text
packages/echo-renderer-three/src/objects/HighlightRenderer.ts
```text

---

## Phase 10: TypeScript ThreeSceneAdapter

### SPA.10.1 — Implement ThreeSceneAdapter.ts

**Status:** 🟡 Blocked by SPA.3.5, SPA.4.1, SPA.7.1, SPA.7.2, SPA.8.1, SPA.8.2, SPA.8.3, SPA.9.1

**Description:**
Implement the main adapter that implements ScenePort interface.

**Requirements:**

- [ ] Create `ThreeSceneAdapter` class implementing `ScenePort`
- [ ] Constructor takes `canvas` and optional `ThreeSceneAdapterOptions`
- [ ] Options: `antialias`, `pixelRatio`, `profiler`
- [ ] Wire together: SceneState, ThreeRenderCore, CameraController, all renderers
- [ ] Implement `applySceneDelta`:
    - Track lastEpochByCursor (Map<string, number>)
    - Skip if epoch <= lastEpoch (idempotent)
    - Apply ops via SceneState
    - Sync all renderers
- [ ] Implement `setCamera`, `setHighlight`, `render`, `resize`, `resetCursor`, `dispose`

**Acceptance Criteria:**

- [ ] Implements all ScenePort methods
- [ ] Duplicate epoch is no-op
- [ ] `resetCursor` clears epoch tracking only (not scene)
- [ ] `render` uses injected Profiler (NULL_PROFILER by default)
- [ ] No `performance.now` calls

**Scope:**

- ✅ In scope: Full adapter implementation
- ❌ Not in scope: Post-processing, multi-scene

**Files:**

```text
packages/echo-renderer-three/src/adapter/ThreeSceneAdapter.ts
```text

---

### SPA.10.2 — Update index.ts Exports

**Status:** 🟡 Blocked by SPA.10.1

**Description:**
Update package exports to expose public API.

**Requirements:**

- [ ] Export `ThreeSceneAdapter` class
- [ ] Export `ThreeSceneAdapterOptions` interface
- [ ] Export all types from types/index.ts
- [ ] Do NOT export internal classes (renderers, SceneState)

**Acceptance Criteria:**

- [ ] `import { ThreeSceneAdapter } from 'echo-renderer-three'` works
- [ ] All public types importable
- [ ] Internal implementation details not exported

**Scope:**

- ✅ In scope: Public exports
- ❌ Not in scope: Sub-path exports

**Files:**

```text
packages/echo-renderer-three/src/index.ts
```text

---

## Phase 11: Integration Tests

### SPA.11.1 — Write Adapter Integration Tests

**Status:** 🟡 Blocked by SPA.10.1

**Description:**
Write integration tests for ThreeSceneAdapter.

**Requirements:**

- [ ] Test: apply UpsertNode → node in scene
- [ ] Test: apply RemoveNode → node removed
- [ ] Test: apply UpsertEdge with valid endpoints → edge in scene
- [ ] Test: apply UpsertEdge with missing endpoint → no error
- [ ] Test: apply Clear → scene empty
- [ ] Test: setCamera updates camera position
- [ ] Test: setHighlight changes colors
- [ ] Test: resize updates renderer size

**Acceptance Criteria:**

- [ ] Tests run in jsdom or headless environment
- [ ] May need WebGL mock or skip GPU-dependent assertions
- [ ] No flaky tests

**Scope:**

- ✅ In scope: Adapter behavior tests
- ❌ Not in scope: Visual regression tests

**Files:**

```text
packages/echo-renderer-three/src/__tests__/adapter.test.ts
```text

---

### SPA.11.2 — Write Determinism Tests

**Status:** 🟡 Blocked by SPA.10.1, SPA.2.5

**Description:**
Write tests verifying TS adapter matches MockAdapter behavior.

**Requirements:**

- [ ] Apply same delta sequence to both MockAdapter and SceneState
- [ ] Compare node/edge/label counts
- [ ] Test epoch idempotency in both
- [ ] Test resetCursor allows epoch restart in both

**Acceptance Criteria:**

- [ ] Counts match between Rust MockAdapter and TS SceneState
- [ ] Epoch semantics match
- [ ] Document any intentional differences

**Scope:**

- ✅ In scope: Cross-check state consistency
- ❌ Not in scope: Byte-level CBOR comparison (different implementations)

**Files:**

```text
packages/echo-renderer-three/src/__tests__/determinism.test.ts
```text

---

## Phase 12: Documentation

### SPA.12.1 — Write README.md

**Status:** 🟡 Blocked by SPA.10.2

**Description:**
Write package README with usage examples.

**Requirements:**

- [ ] Installation instructions
- [ ] Basic usage example (create adapter, apply delta, render)
- [ ] API reference (methods, options)
- [ ] Profiler injection example
- [ ] Document what's NOT supported (no RAF, no time ownership)

**Acceptance Criteria:**

- [ ] Code examples compile
- [ ] Clear "Getting Started" section
- [ ] Documents determinism guarantees

**Scope:**

- ✅ In scope: Package documentation
- ❌ Not in scope: API docs generation, tutorials

**Files:**

```text
packages/echo-renderer-three/README.md
```text

---

### SPA.12.2 — Finalize and Verify Build

**Status:** 🟡 Blocked by SPA.11.1, SPA.11.2, SPA.12.1

**Description:**
Final verification that package is shippable.

**Requirements:**

- [ ] Run full test suite
- [ ] Run build
- [ ] Verify dist/ output is valid
- [ ] Run grep for banned patterns
- [ ] Review exports

**Acceptance Criteria:**

- [ ] `pnpm test` passes
- [ ] `pnpm build` produces dist/
- [ ] No `performance.now`, `Date.now`, `Math.random` in src/
- [ ] Package can be imported and used

**Scope:**

- ✅ In scope: Final verification
- ❌ Not in scope: Publishing to npm

**Files:**

```text
(verification only, no new files)
```text

---

## Milestones

### M1 — Rust Crates Complete

**Reached when:** SPA.2.5 complete
**Deliverables:** `echo-scene-port` and `echo-scene-codec` crates building and tested

### M2 — TS Types Complete

**Reached when:** SPA.4.2 complete
**Deliverables:** All TypeScript types implemented and tested

### M3 — Renderers Complete

**Reached when:** SPA.9.1 complete
**Deliverables:** All renderer classes implemented

### M4 — Shippable Package

**Reached when:** SPA.12.2 complete
**Deliverables:** Full package ready for integration

---

## Dependency Summary

```text
SPA.1.1 ─┬─► SPA.1.2 ─┬─► SPA.1.4 ─┬─► SPA.1.6 ─► SPA.2.1 ─► SPA.2.2 ─┬─► SPA.2.3 ─► SPA.2.5 ─► M1
         │           │            │                                    │
         ├─► SPA.1.3 ┘            │                                    └─► SPA.2.4
         │                        │
         └─► SPA.1.5              │
                                  │
SPA.3.1 ─┬─► SPA.3.2 ─┬─► SPA.3.4 ┴─► SPA.3.5 ─────────────────────────────────────┐
         │           │                                                              │
         │           └─► SPA.4.1 ─► SPA.4.2 ─► M2                                  │
         │                  │                                                       │
         ├─► SPA.3.3 ───────┼──► SPA.7.2 ──────────────────────────────────────┐   │
         │                  │                                                   │   │
         ├─► SPA.5.1        ├─► SPA.8.1 ─┬─► SPA.9.1 ─► M3 ────────────────┐   │   │
         ├─► SPA.6.1        ├─► SPA.8.2 ─┤                                  │   │   │
         │                  └─► SPA.8.3  │                                  │   │   │
         │                               │                                  │   │   │
         └─► SPA.7.1 ────────────────────┴──────────────────────────────────┴───┴───┴─► SPA.10.1 ─► SPA.10.2 ─► SPA.12.1 ─┐
                                                                                          │                               │
                                                                                          ├─► SPA.11.1 ──────────────────┼─► SPA.12.2 ─► M4
                                                                                          └─► SPA.11.2 ──────────────────┘
```text
````
