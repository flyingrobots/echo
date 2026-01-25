<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# ScenePort Adapter Implementation Plan

**Status:** ✅ APPROVED (v2.1)
**Created:** 2026-01-25
**Author:** ARCHY
**Scope:** Migration of James website renderer to hexagonal ScenePort adapter for TTD

---

## 1. Executive Summary

This plan migrates an existing Three.js renderer into a **hexagonal rendering port** for Echo's Time Travel Debugger. The goal: **renderers become dumb projection surfaces**—they receive `SceneDelta` messages and render. No domain logic. No time ownership.

### Design Doctrine

> **Option C port is correct.**
> **Phase 1 is "dumb, minimal" SceneDelta → Three.js render.**
> **Anything fancy is Phase 2.**

### Why Hexagonal?

The TTD requires **deterministic replay**. If the renderer owns time, drives RAF loops, or makes decisions about scene state, determinism breaks. By inverting control:

- **TTD Controller** owns time, produces deltas, controls playback
- **ScenePort adapter** consumes deltas, renders on demand
- **Testing becomes trivial**—MockAdapter validates delta sequences without GPU

### What We're Building

1. **`echo-scene-port`** — Pure Rust crate with types + trait. **No serde. No runtime plumbing.**
2. **`packages/echo-renderer-three`** — TypeScript package implementing `ScenePort` with Three.js
3. **MockAdapter** — Rust-side test harness applying deltas to maps (codec-aware, not in port crate)

### Layer Separation

| Layer            | Crate/Package                    | Responsibility                                |
| ---------------- | -------------------------------- | --------------------------------------------- |
| Domain Contract  | `echo-scene-port`                | Types + `ScenePort` trait. No serialization.  |
| Codec            | MBUS / `echo-scene-codec`        | CBOR encode/decode. Lives outside port crate. |
| Adapter (TS)     | `echo-renderer-three`            | Three.js implementation of port               |
| Adapter (Native) | `warp-viewer` (existing)         | wgpu implementation (future alignment)        |
| App Plumbing     | `echo-app-core` or adapter-local | RenderContext, profiling, timing              |

---

## 2. File Tree After Refactor

```text
echo/
├── crates/
│   └── echo-scene-port/               # NEW: Pure Rust types + trait
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs                 # Re-exports
│           ├── types.rs               # SceneDelta, NodeDef, EdgeDef (NO serde)
│           ├── camera.rs              # CameraState, ProjectionKind (NO serde)
│           ├── highlight.rs           # HighlightState (NO serde)
│           ├── port.rs                # ScenePort trait
│           └── canon.rs               # Float canonicalization helpers
│
│   └── echo-scene-codec/              # NEW: Serialization layer (optional)
│       ├── Cargo.toml                 # Depends on echo-scene-port + minicbor
│       └── src/
│           ├── lib.rs
│           ├── cbor.rs                # CBOR encode/decode
│           └── mock_adapter.rs        # MockAdapter (needs codec for testing)
│
└── packages/
    └── echo-renderer-three/           # NEW: Three.js ScenePort implementation
        ├── package.json
        ├── tsconfig.json
        ├── vitest.config.ts
        └── src/
            ├── index.ts               # Public exports
            ├── types/
            │   ├── index.ts           # Type re-exports
            │   ├── SceneDelta.ts      # TS mirror of Rust types (MVP ops only)
            │   ├── CameraState.ts
            │   └── HighlightState.ts
            ├── adapter/
            │   ├── ThreeSceneAdapter.ts    # Implements ScenePort
            │   └── SceneState.ts           # Internal node/edge/label maps
            ├── core/
            │   ├── ThreeRenderCore.ts      # WebGLRenderer wrapper
            │   └── CameraController.ts     # Applies CameraState to THREE.Camera
            ├── objects/
            │   ├── NodeRenderer.ts         # Basic sphere/box rendering (MVP)
            │   ├── EdgeRenderer.ts         # Basic line rendering (MVP)
            │   ├── LabelRenderer.ts        # CanvasTexture sprites (MVP)
            │   └── HighlightRenderer.ts    # Color tint (MVP, no outline)
            ├── shaders/
            │   └── ShaderManager.ts        # GLSL chunk registry (ported, simplified)
            ├── assets/
            │   └── AssetManager.ts         # URL-keyed texture cache
            └── __tests__/
                ├── adapter.test.ts
                └── sceneState.test.ts
```

### Notable Decisions

| Item                | Decision                 | Rationale                             |
| ------------------- | ------------------------ | ------------------------------------- |
| `RenderContext`     | Adapter-local TS type    | Not part of domain contract           |
| `FrameResult`       | Adapter-local TS type    | Profiling is app concern              |
| `performance.now()` | Banned in adapter        | Inject `Profiler` interface if needed |
| Serde               | Not in `echo-scene-port` | Codec layer handles serialization     |
| InputSystem         | Removed entirely         | App concern, not renderer             |
| Instanced rendering | Phase 2                  | MVP uses basic meshes                 |
| SDF fonts           | Phase 2                  | MVP uses CanvasTexture                |
| Tubes for edges     | Phase 2                  | MVP uses `LineSegments`               |
| Outline highlights  | Phase 2                  | MVP uses color tint                   |

---

## 3. MVP Type Definitions

### 3.1 Rust Types (echo-scene-port) — NO SERDE

```rust
// crates/echo-scene-port/src/lib.rs

//! Scene port contract for Echo renderers.
//!
//! This crate defines the domain contract between TTD Controller and renderers.
//! It contains NO serialization logic—that lives in echo-scene-codec or MBUS.
//!
//! This crate is `no_std`-capable but defaults to `std` for ergonomics.
//! Disable default features for embedded/WASM contexts.

#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

mod types;
mod camera;
mod highlight;
mod port;
mod canon;

pub use types::*;
pub use camera::*;
pub use highlight::*;
pub use port::*;
pub use canon::*;
```

```rust
// crates/echo-scene-port/src/types.rs

use alloc::string::String;
use alloc::vec::Vec;

/// 32-byte content-addressed key.
pub type Hash = [u8; 32];
pub type NodeKey = Hash;
pub type EdgeKey = Hash;
pub type LabelKey = Hash;

/// Node shape for rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum NodeShape {
    Sphere = 0,
    Cube = 1,
}

/// Edge visual style.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum EdgeStyle {
    Solid = 0,
    Dashed = 1,
}

/// RGBA color as 4 bytes (0-255 each).
pub type ColorRgba8 = [u8; 4];

/// Node definition.
#[derive(Clone, Debug, PartialEq)]
pub struct NodeDef {
    pub key: NodeKey,
    pub position: [f32; 3],
    pub radius: f32,
    pub shape: NodeShape,
    pub color: ColorRgba8,
}

/// Edge definition.
#[derive(Clone, Debug, PartialEq)]
pub struct EdgeDef {
    pub key: EdgeKey,
    pub a: NodeKey,
    pub b: NodeKey,
    pub width: f32,
    pub style: EdgeStyle,
    pub color: ColorRgba8,
}

/// Label anchor type.
#[derive(Clone, Debug, PartialEq)]
pub enum LabelAnchor {
    Node { key: NodeKey },
    World { position: [f32; 3] },
}

/// Label definition.
#[derive(Clone, Debug, PartialEq)]
pub struct LabelDef {
    pub key: LabelKey,
    pub text: String,
    pub font_size: f32,
    pub color: ColorRgba8,
    pub anchor: LabelAnchor,
    pub offset: [f32; 3],
}

/// Scene operation (MVP set).
///
/// Minimal ops for Phase 1. No MoveNode, no ClearLayer, no SetHighlight.
/// Highlight is a separate channel/method.
#[derive(Clone, Debug, PartialEq)]
pub enum SceneOp {
    UpsertNode(NodeDef),
    RemoveNode { key: NodeKey },
    UpsertEdge(EdgeDef),
    RemoveEdge { key: EdgeKey },
    UpsertLabel(LabelDef),
    RemoveLabel { key: LabelKey },
    Clear,
}

/// Scene delta: a batch of operations.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneDelta {
    /// Session ID for cursor scoping.
    pub session_id: Hash,
    /// Cursor ID within session.
    pub cursor_id: Hash,
    /// Monotonic epoch within this cursor.
    pub epoch: u64,
    /// Operations to apply.
    pub ops: Vec<SceneOp>,
}
```

```rust
// crates/echo-scene-port/src/camera.rs

/// Camera projection type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectionKind {
    Perspective,
    Orthographic,
}

/// Camera state for rendering.
#[derive(Clone, Debug, PartialEq)]
pub struct CameraState {
    pub position: [f32; 3],
    pub target: [f32; 3],
    pub up: [f32; 3],
    pub projection: ProjectionKind,
    /// Vertical FOV in radians (perspective only).
    pub fov_y_radians: f32,
    /// Vertical extent in world units (orthographic only).
    pub ortho_scale: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 5.0],
            target: [0.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            projection: ProjectionKind::Perspective,
            fov_y_radians: 1.0472, // 60 degrees
            ortho_scale: 10.0,
            near: 0.01,
            far: 10000.0,
        }
    }
}
```

```rust
// crates/echo-scene-port/src/highlight.rs

use alloc::vec::Vec;
use crate::types::{NodeKey, EdgeKey};

/// Highlight state for selection/hover feedback.
///
/// This is UI state, NOT part of scene ops. Set via separate method.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HighlightState {
    pub selected_nodes: Vec<NodeKey>,
    pub selected_edges: Vec<EdgeKey>,
    pub hovered_node: Option<NodeKey>,
    pub hovered_edge: Option<EdgeKey>,
}
```

```rust
// crates/echo-scene-port/src/port.rs

use crate::{SceneDelta, CameraState, HighlightState};

/// Scene rendering port trait.
///
/// Implementors receive deltas and render. No time ownership.
/// RenderContext/FrameResult are adapter-local concerns.
pub trait ScenePort {
    /// Apply a scene delta. Idempotent per (cursor_id, epoch).
    fn apply_scene_delta(&mut self, delta: &SceneDelta);

    /// Set camera state.
    fn set_camera(&mut self, camera: &CameraState);

    /// Set highlight state (selection/hover).
    fn set_highlight(&mut self, highlight: &HighlightState);

    /// Render the current scene. Returns nothing—profiling is adapter concern.
    fn render(&mut self);

    /// Resize viewport.
    fn resize(&mut self, width: u32, height: u32, dpr: f32);

    /// Reset epoch tracking for a cursor (allows epoch to restart from 0).
    ///
    /// This ONLY clears epoch tracking. Scene state is NOT cleared.
    /// Use `SceneOp::Clear` to clear the scene.
    fn reset_cursor(&mut self, cursor_id: &[u8; 32]);

    /// Dispose all resources.
    fn dispose(&mut self);
}
```

```rust
// crates/echo-scene-port/src/canon.rs

/// Canonicalize a float for deterministic comparison and hashing.
///
/// **Purpose:** Used by codec layer when computing content hashes.
/// Do NOT use this to mutate stored positions—it's a projection for comparison.
///
/// - Truncate to 6 decimal places
/// - Convert -0.0 to 0.0
/// - Panic on NaN/Infinity (validation should catch earlier)
pub fn canonicalize_f32(x: f32) -> f32 {
    assert!(x.is_finite(), "NaN/Infinity not allowed in scene data");
    let truncated = (x * 1_000_000.0).trunc() / 1_000_000.0;
    if truncated == 0.0 { 0.0 } else { truncated }
}

/// Canonicalize a position vector for comparison/hashing.
pub fn canonicalize_position(p: [f32; 3]) -> [f32; 3] {
    [
        canonicalize_f32(p[0]),
        canonicalize_f32(p[1]),
        canonicalize_f32(p[2]),
    ]
}
```

### 3.2 TypeScript Types (MVP)

```typescript
// packages/echo-renderer-three/src/types/SceneDelta.ts

/** 32-byte key as Uint8Array. Matches Rust [u8; 32]. */
export type Hash = Uint8Array; // length 32
export type NodeKey = Hash;
export type EdgeKey = Hash;
export type LabelKey = Hash;

/** Convert Hash to hex string for use as Map key. */
export function hashToHex(h: Hash): string {
    return Array.from(h)
        .map((b) => b.toString(16).padStart(2, "0"))
        .join("");
}

/** Parse hex string back to Hash. */
export function hexToHash(hex: string): Hash {
    const bytes = new Uint8Array(32);
    for (let i = 0; i < 32; i++) {
        bytes[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
    }
    return bytes;
}

export enum NodeShape {
    Sphere = 0,
    Cube = 1,
}

export enum EdgeStyle {
    Solid = 0,
    Dashed = 1,
}

/** RGBA color as [r, g, b, a] where each is 0-255. */
export type ColorRgba8 = [number, number, number, number];

export interface NodeDef {
    key: NodeKey;
    position: [number, number, number];
    radius: number;
    shape: NodeShape;
    color: ColorRgba8;
}

export interface EdgeDef {
    key: EdgeKey;
    a: NodeKey;
    b: NodeKey;
    width: number;
    style: EdgeStyle;
    color: ColorRgba8;
}

export type LabelAnchor =
    | { kind: "Node"; key: NodeKey }
    | { kind: "World"; position: [number, number, number] };

export interface LabelDef {
    key: LabelKey;
    text: string;
    fontSize: number;
    color: ColorRgba8;
    anchor: LabelAnchor;
    offset: [number, number, number];
}

/** MVP scene operations. */
export type SceneOp =
    | { op: "UpsertNode"; def: NodeDef }
    | { op: "RemoveNode"; key: NodeKey }
    | { op: "UpsertEdge"; def: EdgeDef }
    | { op: "RemoveEdge"; key: EdgeKey }
    | { op: "UpsertLabel"; def: LabelDef }
    | { op: "RemoveLabel"; key: LabelKey }
    | { op: "Clear" };

export interface SceneDelta {
    sessionId: Hash;
    cursorId: Hash;
    epoch: number;
    ops: SceneOp[];
}
```

```typescript
// packages/echo-renderer-three/src/types/CameraState.ts

export type ProjectionKind = "Perspective" | "Orthographic";

export interface CameraState {
    position: [number, number, number];
    target: [number, number, number];
    up: [number, number, number];
    projection: ProjectionKind;
    fovYRadians: number;
    orthoScale: number;
    near: number;
    far: number;
}

export const DEFAULT_CAMERA: CameraState = {
    position: [0, 0, 5],
    target: [0, 0, 0],
    up: [0, 1, 0],
    projection: "Perspective",
    fovYRadians: 1.0472,
    orthoScale: 10,
    near: 0.01,
    far: 10000,
};
```

```typescript
// packages/echo-renderer-three/src/types/HighlightState.ts

import type { NodeKey, EdgeKey } from "./SceneDelta";

export interface HighlightState {
    selectedNodes: NodeKey[];
    selectedEdges: EdgeKey[];
    hoveredNode?: NodeKey;
    hoveredEdge?: EdgeKey;
}

export const EMPTY_HIGHLIGHT: HighlightState = {
    selectedNodes: [],
    selectedEdges: [],
};
```

```typescript
// packages/echo-renderer-three/src/types/index.ts

// Domain types (mirror Rust)
export * from "./SceneDelta";
export * from "./CameraState";
export * from "./HighlightState";

// Adapter-local types (NOT in Rust port crate)

/** Render timing context. Adapter-local, not part of domain contract. */
export interface RenderContext {
    /** Monotonic frame counter from app. */
    frameIndex: number;
    /** App-controlled time in seconds. */
    timeSeconds: number;
    /** Delta time since last frame. */
    dtSeconds: number;
    /** Viewport width in pixels. */
    width: number;
    /** Viewport height in pixels. */
    height: number;
    /** Device pixel ratio. */
    dpr: number;
}

/** Optional profiler interface. Inject to enable timing. */
export interface Profiler {
    markStart(label: string): void;
    markEnd(label: string): number; // returns ms
}

/** No-op profiler for production. */
export const NULL_PROFILER: Profiler = {
    markStart: () => {},
    markEnd: () => 0,
};

/** ScenePort interface (TypeScript version). */
export interface ScenePort {
    applySceneDelta(delta: SceneDelta): void;
    setCamera(camera: CameraState): void;
    setHighlight(highlight: HighlightState): void;
    render(ctx: RenderContext): void;
    resize(width: number, height: number, dpr: number): void;
    /** Reset epoch tracking only. Scene state is NOT cleared. Use SceneOp.Clear for that. */
    resetCursor(cursorId: Hash): void;
    dispose(): void;
}
```

---

## 4. Epoch Semantics: Cursor-Scoped Monotonic

### The Problem

Epochs must handle:

- Out-of-order delta arrival
- Timeline rewind (TTD scrubbing)
- Multiple worldlines (fork comparison)
- Replay after reset

### The Solution: Strategy A (Cursor-Scoped)

Each `SceneDelta` includes `(session_id, cursor_id, epoch)`:

```typescript
interface SceneDelta {
    sessionId: Hash; // Identifies the session
    cursorId: Hash; // Identifies the playback cursor within session
    epoch: number; // Monotonic within this cursor
    ops: SceneOp[];
}
```

**Adapter maintains:**

```typescript
class ThreeSceneAdapter {
    private lastEpochByCursor: Map<Hash, number> = new Map();

    applySceneDelta(delta: SceneDelta): void {
        const cursorKey = hashToHex(delta.cursorId); // Uint8Array → string for Map key
        const lastEpoch = this.lastEpochByCursor.get(cursorKey) ?? -1;

        if (delta.epoch <= lastEpoch) {
            // Idempotent: already applied or stale
            return;
        }

        this.state.apply(delta.ops);
        this.lastEpochByCursor.set(cursorKey, delta.epoch);
    }

    resetCursor(cursorId: Hash): void {
        // Clears epoch tracking ONLY. Scene state is NOT cleared.
        // Use SceneOp::Clear to clear the scene.
        this.lastEpochByCursor.delete(hashToHex(cursorId));
    }
}
```

**When rewind happens:**

1. TTD Controller sends `SceneOp::Clear` to reset scene state
2. TTD Controller calls `resetCursor(cursorId)` to reset epoch tracking
3. Controller can now send deltas starting from epoch 0

**Important:** `resetCursor` only clears epoch tracking. It does NOT clear the scene. Use `SceneOp::Clear` to clear scene state. This separation allows the controller to decide whether a rewind should preserve or discard existing scene content.

**When comparing forks:**

- Each fork has its own `cursorId`
- Epochs are independent per cursor
- Adapter can maintain multiple scene states (future Phase 2)

---

## 5. Commit-by-Commit Plan

### Commit 1: Bootstrap `echo-scene-port` Crate (Types Only)

**Title:** `feat(scene-port): bootstrap echo-scene-port crate with MVP types`

**Rationale:** Create the pure domain contract. No serde, no runtime deps.

**Files Touched:**

- `crates/echo-scene-port/Cargo.toml` (new)
- `crates/echo-scene-port/src/lib.rs` (new)
- `crates/echo-scene-port/src/types.rs` (new)
- `crates/echo-scene-port/src/camera.rs` (new)
- `crates/echo-scene-port/src/highlight.rs` (new)
- `crates/echo-scene-port/src/port.rs` (new)
- `crates/echo-scene-port/src/canon.rs` (new)
- `Cargo.toml` (workspace member)

**Cargo.toml:**

```toml
[package]
name = "echo-scene-port"
version = "0.1.0"
edition = "2021"

[dependencies]
# NO serde. NO external deps.

[features]
default = ["std"]  # std enabled by default for ergonomics
std = []           # disable for no_std contexts (WASM, embedded)
```

**Acceptance Tests:**

- [ ] `cargo build -p echo-scene-port` passes
- [ ] `cargo build -p echo-scene-port --no-default-features` passes (no_std capable)
- [ ] No serde dependency (verify with `cargo tree`)
- [ ] `ScenePort` trait compiles
- [ ] `canonicalize_f32(-0.0) == 0.0`
- [ ] `canonicalize_f32(1.23456789) == 1.234567`

---

### Commit 2: Add `echo-scene-codec` with MockAdapter

**Title:** `feat(scene-codec): add CBOR codec and MockAdapter`

**Rationale:** Serialization lives outside port crate. MockAdapter needs codec for testing.

**Files Touched:**

- `crates/echo-scene-codec/Cargo.toml` (new)
- `crates/echo-scene-codec/src/lib.rs` (new)
- `crates/echo-scene-codec/src/cbor.rs` (new)
- `crates/echo-scene-codec/src/mock_adapter.rs` (new)
- `crates/echo-scene-codec/tests/roundtrip_tests.rs` (new)
- `crates/echo-scene-codec/tests/mock_adapter_tests.rs` (new)

**Cargo.toml:**

```toml
[package]
name = "echo-scene-codec"
version = "0.1.0"
edition = "2021"

[dependencies]
echo-scene-port = { path = "../echo-scene-port" }
minicbor = { version = "0.24", features = ["alloc"] }

[dev-dependencies]
# test deps
```

**Acceptance Tests:**

- [ ] CBOR roundtrip: encode → decode → original
- [ ] MockAdapter implements `ScenePort`
- [ ] MockAdapter tracks node/edge/label counts correctly
- [ ] Two MockAdapters given same deltas produce identical state
- [ ] Epoch idempotency works per cursor
- [ ] `resetCursor` allows epoch restart

---

### Commit 3: Bootstrap `echo-renderer-three` Package

**Title:** `feat(renderer-three): bootstrap TypeScript package with MVP types`

**Rationale:** Create TS package with mirrored types. No Three.js yet.

**Files Touched:**

- `packages/echo-renderer-three/package.json` (new)
- `packages/echo-renderer-three/tsconfig.json` (new)
- `packages/echo-renderer-three/vitest.config.ts` (new)
- `packages/echo-renderer-three/src/index.ts` (new)
- `packages/echo-renderer-three/src/types/*.ts` (new)

**package.json:**

```json
{
    "name": "echo-renderer-three",
    "version": "0.1.0",
    "type": "module",
    "main": "./dist/index.js",
    "types": "./dist/index.d.ts",
    "exports": {
        ".": {
            "import": "./dist/index.js",
            "types": "./dist/index.d.ts"
        }
    },
    "scripts": {
        "build": "tsc",
        "test": "vitest run",
        "test:watch": "vitest"
    },
    "peerDependencies": {
        "three": ">=0.150.0"
    },
    "devDependencies": {
        "@types/three": "^0.160.0",
        "three": "^0.160.0",
        "typescript": "^5.3.0",
        "vitest": "^1.0.0"
    }
}
```

**Acceptance Tests:**

- [ ] `pnpm install` succeeds
- [ ] `pnpm tsc --noEmit` passes
- [ ] Types match Rust definitions (manual checklist)
- [ ] `ScenePort` interface has all methods
- [ ] No `performance.now` or `Date.now` in types/

---

### Commit 4: Implement SceneState (Pure State Machine)

**Title:** `feat(renderer-three): implement SceneState for delta application`

**Rationale:** Pure TS class managing node/edge/label maps. Testable without Three.js.

**Files Touched:**

- `packages/echo-renderer-three/src/adapter/SceneState.ts` (new)
- `packages/echo-renderer-three/src/__tests__/sceneState.test.ts` (new)

**Implementation:**

```typescript
import { hashToHex } from "../types";

export class SceneState {
    // Maps use hex strings as keys (Uint8Array can't be Map keys directly)
    readonly nodes = new Map<string, NodeDef>();
    readonly edges = new Map<string, EdgeDef>();
    readonly labels = new Map<string, LabelDef>();

    apply(ops: SceneOp[]): void {
        for (const op of ops) {
            switch (op.op) {
                case "UpsertNode":
                    this.nodes.set(hashToHex(op.def.key), op.def);
                    break;
                case "RemoveNode": {
                    const keyHex = hashToHex(op.key);
                    this.nodes.delete(keyHex);
                    // Remove labels anchored to this node
                    for (const [k, label] of this.labels) {
                        if (
                            label.anchor.kind === "Node" &&
                            hashToHex(label.anchor.key) === keyHex
                        ) {
                            this.labels.delete(k);
                        }
                    }
                    break;
                }
                case "UpsertEdge":
                    this.edges.set(hashToHex(op.def.key), op.def);
                    break;
                case "RemoveEdge":
                    this.edges.delete(hashToHex(op.key));
                    break;
                case "UpsertLabel":
                    this.labels.set(hashToHex(op.def.key), op.def);
                    break;
                case "RemoveLabel":
                    this.labels.delete(hashToHex(op.key));
                    break;
                case "Clear":
                    this.nodes.clear();
                    this.edges.clear();
                    this.labels.clear();
                    break;
            }
        }
    }

    /** Check if edge endpoints exist. */
    isEdgeValid(edge: EdgeDef): boolean {
        return (
            this.nodes.has(hashToHex(edge.a)) &&
            this.nodes.has(hashToHex(edge.b))
        );
    }
}
```

**Acceptance Tests:**

- [ ] `UpsertNode` creates or replaces
- [ ] `RemoveNode` deletes node and anchored labels
- [ ] `UpsertEdge` creates or replaces
- [ ] `RemoveEdge` deletes edge
- [ ] `Clear` empties all maps
- [ ] Edge with missing endpoint: `isEdgeValid` returns false
- [ ] No external dependencies (pure TS)

---

### Commit 5: Port ShaderManager (Simplified)

**Title:** `feat(renderer-three): port ShaderManager without time dependencies`

**Rationale:** Keep shader composition system but remove all time-dependent code.

**Files Touched:**

- `packages/echo-renderer-three/src/shaders/ShaderManager.ts` (new)

**Changes from Original:**

- Remove singleton `getInstance()` → constructor
- Remove `uTime` uniform
- Remove film grain
- Keep basic fresnel/noise as pure functions with explicit seed input

**Acceptance Tests:**

- [ ] `new ShaderManager()` works (no singleton)
- [ ] `#include <fresnel>` resolves
- [ ] No `performance.now` or `Math.random` in module
- [ ] Grep returns zero matches for banned patterns

---

### Commit 6: Port AssetManager (URL-Keyed)

**Title:** `feat(renderer-three): port AssetManager with URL keys`

**Rationale:** Simple texture cache. Content-addressing is future work.

**Files Touched:**

- `packages/echo-renderer-three/src/assets/AssetManager.ts` (new)

**Implementation:**

```typescript
export class AssetManager {
    private textures = new Map<string, THREE.Texture>();
    private loadingManager: THREE.LoadingManager;

    constructor(loadingManager?: THREE.LoadingManager) {
        this.loadingManager = loadingManager ?? new THREE.LoadingManager();
    }

    getTexture(url: string): THREE.Texture {
        let tex = this.textures.get(url);
        if (!tex) {
            tex = new THREE.TextureLoader(this.loadingManager).load(url);
            this.textures.set(url, tex);
        }
        return tex;
    }

    dispose(): void {
        for (const tex of this.textures.values()) {
            tex.dispose();
        }
        this.textures.clear();
    }
}
```

**Acceptance Tests:**

- [ ] Two instances don't share state
- [ ] `dispose()` clears all textures
- [ ] Same URL returns same texture instance
- [ ] No singleton pattern

---

### Commit 7: Implement ThreeRenderCore (Minimal)

**Title:** `feat(renderer-three): implement ThreeRenderCore without task scheduler`

**Rationale:** WebGLRenderer wrapper. Single scene. No RAF ownership. No timing calls.

**Files Touched:**

- `packages/echo-renderer-three/src/core/ThreeRenderCore.ts` (new)
- `packages/echo-renderer-three/src/core/CameraController.ts` (new)

**ThreeRenderCore:**

```typescript
export interface RenderCoreOptions {
    antialias?: boolean;
    pixelRatio?: number;
}

export class ThreeRenderCore {
    readonly renderer: THREE.WebGLRenderer;

    constructor(canvas: HTMLCanvasElement, options: RenderCoreOptions = {}) {
        this.renderer = new THREE.WebGLRenderer({
            canvas,
            antialias: options.antialias ?? true,
            powerPreference: "high-performance",
        });
        this.renderer.setPixelRatio(options.pixelRatio ?? 1);
    }

    render(scene: THREE.Scene, camera: THREE.Camera): void {
        this.renderer.render(scene, camera);
    }

    resize(width: number, height: number, dpr: number): void {
        this.renderer.setSize(width, height, false);
        this.renderer.setPixelRatio(dpr);
    }

    dispose(): void {
        this.renderer.dispose();
    }
}
```

**CameraController:**

```typescript
export class CameraController {
    private perspCamera: THREE.PerspectiveCamera;
    private orthoCamera: THREE.OrthographicCamera;
    private current: THREE.Camera;

    constructor(aspect: number) {
        this.perspCamera = new THREE.PerspectiveCamera(60, aspect, 0.01, 10000);
        this.orthoCamera = new THREE.OrthographicCamera(
            -5,
            5,
            5,
            -5,
            0.01,
            10000,
        );
        this.current = this.perspCamera;
    }

    apply(state: CameraState, aspect: number): THREE.Camera {
        if (state.projection === "Perspective") {
            this.perspCamera.fov = (state.fovYRadians * 180) / Math.PI;
            this.perspCamera.aspect = aspect;
            this.perspCamera.near = state.near;
            this.perspCamera.far = state.far;
            this.perspCamera.position.fromArray(state.position);
            this.perspCamera.lookAt(...state.target);
            this.perspCamera.up.fromArray(state.up);
            this.perspCamera.updateProjectionMatrix();
            this.current = this.perspCamera;
        } else {
            const scale = state.orthoScale;
            this.orthoCamera.left = -scale * aspect;
            this.orthoCamera.right = scale * aspect;
            this.orthoCamera.top = scale;
            this.orthoCamera.bottom = -scale;
            this.orthoCamera.near = state.near;
            this.orthoCamera.far = state.far;
            this.orthoCamera.position.fromArray(state.position);
            this.orthoCamera.lookAt(...state.target);
            this.orthoCamera.up.fromArray(state.up);
            this.orthoCamera.updateProjectionMatrix();
            this.current = this.orthoCamera;
        }
        return this.current;
    }

    get camera(): THREE.Camera {
        return this.current;
    }
}
```

**Acceptance Tests:**

- [ ] No `requestAnimationFrame` calls
- [ ] No `performance.now` calls
- [ ] `render()` takes scene + camera, nothing else
- [ ] `resize()` updates renderer correctly
- [ ] Grep for banned patterns returns zero

---

### Commit 8: Implement Basic Renderers (MVP)

**Title:** `feat(renderer-three): add basic node/edge/label renderers`

**Rationale:** Simple rendering. No instancing, no tubes, no SDF. Just get pixels on screen.

**Files Touched:**

- `packages/echo-renderer-three/src/objects/NodeRenderer.ts` (new)
- `packages/echo-renderer-three/src/objects/EdgeRenderer.ts` (new)
- `packages/echo-renderer-three/src/objects/LabelRenderer.ts` (new)

**NodeRenderer (MVP):**

```typescript
export class NodeRenderer {
    // Use string keys (hex) since Uint8Array can't be Map keys
    readonly meshes = new Map<string, THREE.Mesh>();
    private sphereGeom = new THREE.SphereGeometry(1, 16, 16);
    private boxGeom = new THREE.BoxGeometry(1, 1, 1);

    sync(nodes: Map<string, NodeDef>, scene: THREE.Scene): void {
        // Remove deleted
        for (const [key, mesh] of this.meshes) {
            if (!nodes.has(key)) {
                scene.remove(mesh);
                mesh.geometry.dispose();
                (mesh.material as THREE.Material).dispose();
                this.meshes.delete(key);
            }
        }

        // Upsert
        for (const [key, def] of nodes) {
            let mesh = this.meshes.get(key);
            if (!mesh) {
                const geom =
                    def.shape === NodeShape.Sphere
                        ? this.sphereGeom
                        : this.boxGeom;
                const mat = new THREE.MeshBasicMaterial();
                mesh = new THREE.Mesh(geom.clone(), mat);
                this.meshes.set(key, mesh);
                scene.add(mesh);
            }
            mesh.position.fromArray(def.position);
            mesh.scale.setScalar(def.radius);
            const mat = mesh.material as THREE.MeshBasicMaterial;
            mat.color.setRGB(
                def.color[0] / 255,
                def.color[1] / 255,
                def.color[2] / 255,
            );
            mat.opacity = def.color[3] / 255;
            mat.transparent = def.color[3] < 255;
        }
    }

    dispose(): void {
        for (const mesh of this.meshes.values()) {
            mesh.geometry.dispose();
            (mesh.material as THREE.Material).dispose();
        }
        this.meshes.clear();
        this.sphereGeom.dispose();
        this.boxGeom.dispose();
    }
}
```

**EdgeRenderer (MVP):**

```typescript
export class EdgeRenderer {
    readonly lines = new Map<string, THREE.Line>();

    sync(
        edges: Map<string, EdgeDef>,
        nodes: Map<string, NodeDef>,
        scene: THREE.Scene,
    ): void {
        // Remove deleted
        for (const [key, line] of this.lines) {
            if (!edges.has(key)) {
                scene.remove(line);
                line.geometry.dispose();
                (line.material as THREE.Material).dispose();
                this.lines.delete(key);
            }
        }

        // Upsert
        for (const [key, def] of edges) {
            const nodeA = nodes.get(hashToHex(def.a));
            const nodeB = nodes.get(hashToHex(def.b));
            if (!nodeA || !nodeB) {
                // Invalid edge - skip
                const existing = this.lines.get(key);
                if (existing) {
                    existing.visible = false;
                }
                continue;
            }

            let line = this.lines.get(key);
            if (!line) {
                const geom = new THREE.BufferGeometry();
                const mat = new THREE.LineBasicMaterial();
                line = new THREE.Line(geom, mat);
                this.lines.set(key, line);
                scene.add(line);
            }

            const positions = new Float32Array([
                ...nodeA.position,
                ...nodeB.position,
            ]);
            line.geometry.setAttribute(
                "position",
                new THREE.BufferAttribute(positions, 3),
            );
            line.visible = true;

            const mat = line.material as THREE.LineBasicMaterial;
            mat.color.setRGB(
                def.color[0] / 255,
                def.color[1] / 255,
                def.color[2] / 255,
            );
            mat.opacity = def.color[3] / 255;
            mat.transparent = def.color[3] < 255;
            // Note: linewidth only works in WebGL2 with custom shader; ignored for MVP
        }
    }

    dispose(): void {
        for (const line of this.lines.values()) {
            line.geometry.dispose();
            (line.material as THREE.Material).dispose();
        }
        this.lines.clear();
    }
}
```

**LabelRenderer (MVP with CanvasTexture):**

```typescript
export class LabelRenderer {
    private sprites = new Map<string, THREE.Sprite>();

    sync(
        labels: Map<string, LabelDef>,
        nodes: Map<string, NodeDef>,
        scene: THREE.Scene,
    ): void {
        // Remove deleted
        for (const [key, sprite] of this.sprites) {
            if (!labels.has(key)) {
                scene.remove(sprite);
                (sprite.material as THREE.SpriteMaterial).map?.dispose();
                sprite.material.dispose();
                this.sprites.delete(key);
            }
        }

        // Upsert
        for (const [key, def] of labels) {
            let position: [number, number, number];
            if (def.anchor.kind === "Node") {
                const node = nodes.get(hashToHex(def.anchor.key));
                if (!node) continue; // Anchor missing, skip
                position = [
                    node.position[0] + def.offset[0],
                    node.position[1] + def.offset[1],
                    node.position[2] + def.offset[2],
                ];
            } else {
                position = [
                    def.anchor.position[0] + def.offset[0],
                    def.anchor.position[1] + def.offset[1],
                    def.anchor.position[2] + def.offset[2],
                ];
            }

            let sprite = this.sprites.get(key);
            if (!sprite) {
                const canvas = this.createLabelCanvas(def);
                const texture = new THREE.CanvasTexture(canvas);
                const mat = new THREE.SpriteMaterial({
                    map: texture,
                    transparent: true,
                });
                sprite = new THREE.Sprite(mat);
                this.sprites.set(key, sprite);
                scene.add(sprite);
            } else {
                // Update texture
                const canvas = this.createLabelCanvas(def);
                const mat = sprite.material as THREE.SpriteMaterial;
                mat.map?.dispose();
                mat.map = new THREE.CanvasTexture(canvas);
            }

            sprite.position.fromArray(position);
            sprite.scale.set(
                def.fontSize * def.text.length * 0.5,
                def.fontSize,
                1,
            );
        }
    }

    private createLabelCanvas(def: LabelDef): HTMLCanvasElement {
        const canvas = document.createElement("canvas");
        const ctx = canvas.getContext("2d")!;
        const fontSize = 32; // Fixed canvas font size, scaled by sprite
        ctx.font = `${fontSize}px sans-serif`;
        const metrics = ctx.measureText(def.text);
        canvas.width = Math.ceil(metrics.width) + 4;
        canvas.height = fontSize + 4;
        ctx.font = `${fontSize}px sans-serif`;
        ctx.fillStyle = `rgba(${def.color[0]}, ${def.color[1]}, ${def.color[2]}, ${def.color[3] / 255})`;
        ctx.fillText(def.text, 2, fontSize);
        return canvas;
    }

    dispose(): void {
        for (const sprite of this.sprites.values()) {
            (sprite.material as THREE.SpriteMaterial).map?.dispose();
            sprite.material.dispose();
        }
        this.sprites.clear();
    }
}
```

**Acceptance Tests:**

- [ ] Nodes render as spheres/boxes
- [ ] Edges render as lines between nodes
- [ ] Labels render as sprites
- [ ] Missing edge endpoints: edge hidden, no error
- [ ] Missing label anchor: label skipped, no error
- [ ] No `performance.now` or timing calls

---

### Commit 9: Implement HighlightRenderer (Color Tint Only)

**Title:** `feat(renderer-three): add HighlightRenderer with color tint`

**Rationale:** MVP highlight is just color change. No outline, no stencil.

**Files Touched:**

- `packages/echo-renderer-three/src/objects/HighlightRenderer.ts` (new)

**Implementation:**

```typescript
const SELECTED_COLOR = new THREE.Color(0xffff00); // Yellow tint
const HOVERED_COLOR = new THREE.Color(0x00ffff); // Cyan tint

export class HighlightRenderer {
    private originalColors = new Map<string, THREE.Color>();

    apply(
        highlight: HighlightState,
        nodeMeshes: Map<string, THREE.Mesh>,
        edgeLines: Map<string, THREE.Line>,
    ): void {
        // Reset all to original colors
        for (const [key, color] of this.originalColors) {
            const mesh = nodeMeshes.get(key) ?? edgeLines.get(key);
            if (mesh) {
                const mat = mesh.material as
                    | THREE.MeshBasicMaterial
                    | THREE.LineBasicMaterial;
                mat.color.copy(color);
            }
        }
        this.originalColors.clear();

        // Apply selection tint
        for (const key of highlight.selectedNodes) {
            const keyHex = hashToHex(key);
            const mesh = nodeMeshes.get(keyHex);
            if (mesh) {
                const mat = mesh.material as THREE.MeshBasicMaterial;
                this.originalColors.set(keyHex, mat.color.clone());
                mat.color.lerp(SELECTED_COLOR, 0.5);
            }
        }

        for (const key of highlight.selectedEdges) {
            const keyHex = hashToHex(key);
            const line = edgeLines.get(keyHex);
            if (line) {
                const mat = line.material as THREE.LineBasicMaterial;
                this.originalColors.set(keyHex, mat.color.clone());
                mat.color.lerp(SELECTED_COLOR, 0.5);
            }
        }

        // Apply hover tint (overwrites selection if both)
        if (highlight.hoveredNode) {
            const keyHex = hashToHex(highlight.hoveredNode);
            const mesh = nodeMeshes.get(keyHex);
            if (mesh) {
                const mat = mesh.material as THREE.MeshBasicMaterial;
                if (!this.originalColors.has(keyHex)) {
                    this.originalColors.set(keyHex, mat.color.clone());
                }
                mat.color.lerp(HOVERED_COLOR, 0.5);
            }
        }

        if (highlight.hoveredEdge) {
            const keyHex = hashToHex(highlight.hoveredEdge);
            const line = edgeLines.get(keyHex);
            if (line) {
                const mat = line.material as THREE.LineBasicMaterial;
                if (!this.originalColors.has(keyHex)) {
                    this.originalColors.set(keyHex, mat.color.clone());
                }
                mat.color.lerp(HOVERED_COLOR, 0.5);
            }
        }
    }
}
```

**Acceptance Tests:**

- [ ] Selected nodes get yellow tint
- [ ] Hovered node gets cyan tint
- [ ] Clearing highlight restores original colors
- [ ] No outline/stencil complexity

---

### Commit 10: Implement ThreeSceneAdapter

**Title:** `feat(renderer-three): implement ThreeSceneAdapter (ScenePort)`

**Rationale:** Wire everything together. Public API.

**Files Touched:**

- `packages/echo-renderer-three/src/adapter/ThreeSceneAdapter.ts` (new)
- `packages/echo-renderer-three/src/index.ts` (update exports)

**Implementation:**

```typescript
export interface ThreeSceneAdapterOptions {
    antialias?: boolean;
    pixelRatio?: number;
    profiler?: Profiler;
}

export class ThreeSceneAdapter implements ScenePort {
    private state = new SceneState();
    private core: ThreeRenderCore;
    private cameraController: CameraController;
    private scene = new THREE.Scene();

    private nodeRenderer = new NodeRenderer();
    private edgeRenderer = new EdgeRenderer();
    private labelRenderer = new LabelRenderer();
    private highlightRenderer = new HighlightRenderer();

    private lastEpochByCursor = new Map<string, number>(); // hex string keys
    private currentHighlight: HighlightState = EMPTY_HIGHLIGHT;
    private cameraState: CameraState = DEFAULT_CAMERA;
    private profiler: Profiler;

    constructor(
        canvas: HTMLCanvasElement,
        options: ThreeSceneAdapterOptions = {},
    ) {
        this.core = new ThreeRenderCore(canvas, options);
        this.cameraController = new CameraController(
            canvas.width / canvas.height,
        );
        this.profiler = options.profiler ?? NULL_PROFILER;
    }

    applySceneDelta(delta: SceneDelta): void {
        const cursorKey = hashToHex(delta.cursorId);
        const lastEpoch = this.lastEpochByCursor.get(cursorKey) ?? -1;
        if (delta.epoch <= lastEpoch) {
            return; // Idempotent
        }

        this.state.apply(delta.ops);
        this.syncObjects();
        this.lastEpochByCursor.set(cursorKey, delta.epoch);
    }

    setCamera(camera: CameraState): void {
        this.cameraState = camera;
    }

    setHighlight(highlight: HighlightState): void {
        this.currentHighlight = highlight;
    }

    render(ctx: RenderContext): void {
        this.profiler.markStart("render");

        // Apply highlight
        this.highlightRenderer.apply(
            this.currentHighlight,
            this.nodeRenderer.meshes,
            this.edgeRenderer.lines,
        );

        // Update camera
        const camera = this.cameraController.apply(
            this.cameraState,
            ctx.width / ctx.height,
        );

        // Render
        this.core.render(this.scene, camera);

        this.profiler.markEnd("render");
    }

    resize(width: number, height: number, dpr: number): void {
        this.core.resize(width, height, dpr);
    }

    resetCursor(cursorId: Hash): void {
        this.lastEpochByCursor.delete(hashToHex(cursorId));
    }

    dispose(): void {
        this.nodeRenderer.dispose();
        this.edgeRenderer.dispose();
        this.labelRenderer.dispose();
        this.core.dispose();
    }

    private syncObjects(): void {
        this.nodeRenderer.sync(this.state.nodes, this.scene);
        this.edgeRenderer.sync(this.state.edges, this.state.nodes, this.scene);
        this.labelRenderer.sync(
            this.state.labels,
            this.state.nodes,
            this.scene,
        );
    }
}
```

**Acceptance Tests:**

- [ ] Implements all `ScenePort` methods
- [ ] Duplicate epoch is no-op
- [ ] `resetCursor` allows epoch restart
- [ ] No `performance.now` calls (uses injected Profiler)
- [ ] Integration test: apply deltas, render, verify objects in scene

---

### Commit 11: Add Integration Tests

**Title:** `test(renderer-three): add integration and determinism tests`

**Rationale:** Verify adapter works end-to-end.

**Files Touched:**

- `packages/echo-renderer-three/src/__tests__/adapter.test.ts` (new)
- `packages/echo-renderer-three/src/__tests__/determinism.test.ts` (new)

**Test Cases:**

1. Apply UpsertNode → node appears in scene
2. Apply RemoveNode → node removed, attached labels removed
3. Apply UpsertEdge with valid endpoints → edge renders
4. Apply UpsertEdge with missing endpoint → edge hidden, no error
5. Apply Clear → all objects removed
6. Epoch idempotency: same epoch ignored
7. `resetCursor` allows lower epoch
8. Different cursors have independent epochs

**Acceptance Tests:**

- [ ] All tests pass
- [ ] No flaky timing-dependent tests
- [ ] Tests run without GPU (mock WebGL or headless)

---

### Commit 12: Documentation and Export Polish

**Title:** `docs(renderer-three): add README and finalize exports`

**Rationale:** Shippable package needs docs.

**Files Touched:**

- `packages/echo-renderer-three/README.md` (new)
- `packages/echo-renderer-three/src/index.ts` (finalize)

**README outline:**

- Installation
- Basic usage (create adapter, apply deltas, render)
- API reference
- Profiler injection
- What's NOT supported (no RAF, no time ownership)

**Acceptance Tests:**

- [ ] README has working code example
- [ ] All public types exported
- [ ] `pnpm build` produces valid dist/

---

## 6. Determinism Checklist

### 6.1 Banned Patterns

| Pattern             | Status | Enforcement       |
| ------------------- | ------ | ----------------- |
| `performance.now()` | BANNED | CI grep           |
| `Date.now()`        | BANNED | CI grep           |
| `Math.random()`     | BANNED | CI grep           |
| Serde in port crate | BANNED | Cargo.toml review |
| Singleton patterns  | BANNED | Code review       |

**CI grep command:**

```bash
! grep -rE "performance\.now\(\)|Date\.now\(\)|Math\.random\(\)" \
  packages/echo-renderer-three/src/ \
  --include="*.ts" \
  && echo "PASS: No banned patterns"
```

### 6.2 Allowed Non-Determinism

| Item                | Why Allowed                                  |
| ------------------- | -------------------------------------------- |
| Profiler timing     | Injected, opt-in, doesn't affect state       |
| Pixel rendering     | GPU variance is visual, state is canonical   |
| Canvas font metrics | May vary slightly, acceptable for MVP labels |

### 6.3 Epoch Invariants

- Epochs are scoped to `(sessionId, cursorId)`
- `epoch <= lastEpoch` → no-op (idempotent)
- `resetCursor(cursorId)` → clears epoch tracking, allows restart
- Different cursors are independent

---

## 7. What's Deferred to Phase 2

| Feature                | Rationale                                 |
| ---------------------- | ----------------------------------------- |
| Instanced rendering    | Optimization; MVP works without it        |
| TubeGeometry edges     | Complexity; LineSegments sufficient       |
| SDF fonts              | Build complexity; CanvasTexture works     |
| Outline highlights     | Stencil complexity; color tint sufficient |
| Grab pass / refraction | Not needed for graph viz                  |
| BokehPass / DOF        | Breaks timeline determinism               |
| Film grain             | Non-deterministic unless seeded           |
| `MoveNode` op          | UpsertNode replaces position              |
| `ClearLayer` op        | No use case yet                           |
| `SetHighlight` in ops  | Highlight is separate UI state            |
| Content-hash assets    | Pipeline complexity; URL keys work        |
| Multi-scene/worldline  | Future fork comparison feature            |

---

## 8. Success Criteria

The migration is complete when:

- [ ] `echo-scene-port` builds with zero dependencies (no serde)
- [ ] `echo-scene-codec` builds and passes roundtrip tests
- [ ] MockAdapter matches SceneState behavior
- [ ] `echo-renderer-three` builds and passes tests
- [ ] No banned patterns in codebase (CI enforced)
- [ ] Epoch semantics correctly scoped to cursor
- [ ] Integration test: 50 deltas → correct scene state
- [ ] README with working example
- [ ] Importable as `import { ThreeSceneAdapter } from 'echo-renderer-three'`
