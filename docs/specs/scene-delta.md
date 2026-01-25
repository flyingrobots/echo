<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Scene Delta Specification

**Status:** Draft
**Created:** 2026-01-25
**Scope:** Contract between TTD Controller and renderer adapters

---

## 1. Overview

This specification defines **SceneDelta**, the message format used by the TTD Controller to communicate scene changes to renderer adapters. The design follows Echo's hexagonal architecture: the domain logic (TTD Controller) emits deltas to MBUS, and renderer adapters (wgpu native, Three.js web) subscribe and apply them.

### Key Principles

1. **Renderers are dumb** — They receive deltas and render. No domain logic.
2. **Deltas are canonical** — Same input produces identical byte representation.
3. **Keys are content-addressed** — 32-byte hashes derived from WARP graph identifiers.
4. **Operations are ordered** — Within a delta, ops follow a deterministic sort order.

---

## 2. Type Definitions

### 2.1 Primitive Types

| Type          | Size     | Description                            |
| ------------- | -------- | -------------------------------------- |
| `Hash`        | 32 bytes | Blake3 or SHA-256 hash                 |
| `NodeKey`     | 32 bytes | Hash identifying a graph node          |
| `EdgeKey`     | 32 bytes | Hash identifying a graph edge          |
| `LabelKey`    | 32 bytes | Hash identifying a text label          |
| `MaterialKey` | 32 bytes | Hash identifying a material definition |
| `f32`         | 4 bytes  | IEEE 754 single-precision float        |
| `u8`          | 1 byte   | Unsigned 8-bit integer                 |
| `u64`         | 8 bytes  | Unsigned 64-bit integer (epoch)        |

### 2.2 Color Encoding

Colors are RGBA with each channel as `u8` (0-255):

```text
[R, G, B, A] where each ∈ [0, 255]
```

Alpha = 255 is fully opaque, Alpha = 0 is fully transparent.

### 2.3 Position and Scale

All spatial values are in **meters** using a right-handed coordinate system:

- +X = right
- +Y = up
- +Z = toward viewer (out of screen)

Floats are truncated to 6 decimal places before hashing for canonicalization.

---

## 3. Material Definition

Materials define visual appearance and are referenced by key.

```typescript
interface MaterialDef {
    key: MaterialKey; // Content hash of this definition
    baseColor: [u8, u8, u8, u8]; // RGBA
    emissive?: [u8, u8, u8]; // RGB glow color (optional)
    metallic?: f32; // 0.0 = dielectric, 1.0 = metal
    roughness?: f32; // 0.0 = mirror, 1.0 = diffuse
    wireframe: boolean; // Render as wireframe
}
```

**Key derivation:**

```text
MaterialKey = hash(canonical_cbor(MaterialDef without key field))
```

**Default values** (when optional fields omitted):

- `emissive`: `[0, 0, 0]` (no glow)
- `metallic`: `0.0`
- `roughness`: `0.5`
- `wireframe`: `false`

---

## 4. Node Definition

Nodes are graph vertices rendered as 3D shapes.

```typescript
enum NodeShape {
    Sphere = 0,
    Cube = 1,
    Cylinder = 2,
    Octahedron = 3, // For "special" nodes (warps, portals)
    Ring = 4, // Torus for portal visualization
}

interface NodeDef {
    key: NodeKey; // From WARP graph NodeId
    position: [f32, f32, f32]; // World position (meters)
    radius: f32; // Bounding sphere radius (meters)
    shape: NodeShape;
    material: MaterialKey;
    label?: LabelKey; // Optional attached label
    layer: u8; // Render layer (0 = default)
}
```

**Layers:**

| Layer | Purpose                                                    |
| ----- | ---------------------------------------------------------- |
| 0     | Default scene                                              |
| 1     | Overlay (always on top)                                    |
| 2     | Ghost (semi-transparent, for deleted nodes in time-travel) |
| 3     | Debug (wireframe helpers)                                  |

---

## 5. Edge Definition

Edges connect nodes and represent relationships.

```typescript
enum EdgeStyle {
    Solid = 0,
    Dashed = 1,
    Dotted = 2,
    Flow = 3, // Animated particles along edge
}

interface EdgeDef {
    key: EdgeKey; // From WARP graph EdgeId
    a: NodeKey; // Source node
    b: NodeKey; // Target node
    width: f32; // Line width (meters)
    style: EdgeStyle;
    material: MaterialKey;
    directed: boolean; // Show arrowhead at target
    curvature?: f32; // 0 = straight, >0 = bezier bulge factor
    flowSpeed?: f32; // For Flow style: particles/sec, negative = reverse
}
```

**Edge rendering notes:**

- `Flow` style shows animated particles moving from `a` to `b`
- `flowSpeed` controls particle animation rate; negative reverses direction; default 1.0
- `curvature` controls bezier control point offset perpendicular to edge
- If referenced node doesn't exist, edge is not rendered (no error)

---

## 6. Label Definition

Labels are text billboards anchored to nodes or world positions.

```typescript
type LabelAnchor =
    | { kind: "Node"; key: NodeKey }
    | { kind: "World"; position: [f32, f32, f32] };

interface LabelDef {
    key: LabelKey;
    text: string; // UTF-8, max 64 characters recommended
    fontSize: f32; // Height in world units (meters)
    color: [u8, u8, u8, u8]; // RGBA
    anchor: LabelAnchor;
    offset?: [f32, f32, f32]; // Offset from anchor position
}
```

**Rendering behavior:**

- Labels always face camera (billboard)
- If anchored to non-existent node, label is not rendered
- Long text may be truncated by renderer

---

## 7. Scene Operations

### 7.1 Operation Enum

```typescript
type SceneOp =
    // Materials
    | { op: "DefineMaterial"; def: MaterialDef }
    | { op: "RemoveMaterial"; key: MaterialKey }

    // Nodes
    | { op: "UpsertNode"; def: NodeDef }
    | { op: "RemoveNode"; key: NodeKey }
    | { op: "MoveNode"; key: NodeKey; position: [f32, f32, f32] }

    // Edges
    | { op: "UpsertEdge"; def: EdgeDef }
    | { op: "RemoveEdge"; key: EdgeKey }

    // Labels
    | { op: "UpsertLabel"; def: LabelDef }
    | { op: "RemoveLabel"; key: LabelKey }

    // Bulk
    | { op: "Clear" }
    | { op: "ClearLayer"; layer: u8 }

    // Visual state
    | { op: "SetHighlight"; highlight: HighlightState };
```

### 7.2 Operation Semantics

| Operation        | Behavior                                             |
| ---------------- | ---------------------------------------------------- |
| `DefineMaterial` | Create or replace material with given key            |
| `RemoveMaterial` | Delete material; nodes using it render with fallback |
| `UpsertNode`     | Create node or update all fields                     |
| `RemoveNode`     | Delete node and any attached labels                  |
| `MoveNode`       | Update only position (fast path for animation)       |
| `UpsertEdge`     | Create edge or update all fields                     |
| `RemoveEdge`     | Delete edge                                          |
| `UpsertLabel`    | Create label or update all fields                    |
| `RemoveLabel`    | Delete label                                         |
| `Clear`          | Remove all nodes, edges, labels (keep materials)     |
| `ClearLayer`     | Remove all nodes/edges/labels in specific layer      |
| `SetHighlight`   | Update selection/hover state                         |

### 7.3 MoveNode Fast Path

`MoveNode` exists as an optimization for smooth animation and timeline scrubbing. Instead of sending a full `UpsertNode` with all fields, only the position changes.

```typescript
// Instead of this (verbose):
{ op: "UpsertNode", def: { key, position: newPos, radius, shape, material, label, layer } }

// Use this (minimal):
{ op: "MoveNode", key, position: newPos }
```

If the node doesn't exist, `MoveNode` is a no-op (no error).

---

## 8. Highlight State

Highlight state is ephemeral visual feedback, not persisted in provenance.

```typescript
interface HighlightState {
    selectedNodes: NodeKey[]; // Multi-select supported
    selectedEdges: EdgeKey[];
    hoveredNode?: NodeKey; // At most one
    hoveredEdge?: EdgeKey; // At most one
    focusNode?: NodeKey; // Camera should frame this node
}
```

**Rendering behavior:**

- Selected items get outline/glow effect
- Hovered items get subtle highlight
- Focus node triggers camera animation (if camera controller supports it)

---

## 9. Scene Delta

A delta is a batch of operations applied atomically.

```typescript
interface SceneDelta {
    sceneId: Hash; // Which scene (main, minimap, etc.)
    epoch: u64; // Monotonically increasing sequence number
    ops: SceneOp[]; // Ordered list of operations
}
```

### 9.1 Scene IDs

| Scene ID               | Purpose                   |
| ---------------------- | ------------------------- |
| `hash("ttd.main")`     | Primary 3D graph view     |
| `hash("ttd.timeline")` | 2D timeline visualization |
| `hash("ttd.minimap")`  | Overview minimap          |

### 9.2 Epoch Semantics

- Deltas with `epoch ≤ lastAppliedEpoch` are dropped (idempotent)
- Epochs are per-scene (each scene tracks its own)
- Gap in epochs is allowed (missed deltas are lost, not an error)

---

## 10. Canonicalization Rules

For deterministic hashing and replay, deltas must be canonical.

### 10.1 Operation Ordering

Within a delta, operations MUST be sorted in this order:

1. `Clear` (if present, must be first)
2. `ClearLayer` (sorted by layer number)
3. `DefineMaterial` (sorted by key, lexicographic byte order)
4. `RemoveMaterial` (sorted by key)
5. `UpsertNode` (sorted by key)
6. `RemoveNode` (sorted by key)
7. `MoveNode` (sorted by key)
8. `UpsertEdge` (sorted by key)
9. `RemoveEdge` (sorted by key)
10. `UpsertLabel` (sorted by key)
11. `RemoveLabel` (sorted by key)
12. `SetHighlight` (at most one, always last)

### 10.2 Float Canonicalization

Before hashing or serializing:

1. Truncate to 6 decimal places: `floor(x * 1_000_000) / 1_000_000`
2. Negative zero becomes positive zero: `-0.0` → `0.0`
3. NaN and Infinity are forbidden (validation error)

### 10.3 String Canonicalization

- UTF-8 NFC normalization
- No trailing whitespace
- No control characters except newline

### 10.4 CBOR Encoding

Deltas are encoded as **canonical CBOR** (RFC 8949 deterministic encoding):

- Map keys sorted lexicographically
- Integers use smallest encoding
- No indefinite-length arrays/maps

---

## 11. Camera State

Camera state is set separately from scene deltas (different update frequency).

```typescript
type ProjectionKind = "Perspective" | "Orthographic";

interface CameraState {
    position: [f32, f32, f32];
    target: [f32, f32, f32]; // LookAt point
    up: [f32, f32, f32]; // Up vector, typically [0, 1, 0]
    projection: ProjectionKind;
    fovYRadians?: f32; // Perspective only, vertical FOV
    orthoScale?: f32; // Orthographic only, vertical extent
    near: f32; // Near clip plane
    far: f32; // Far clip plane
}
```

**Default values:**

- `up`: `[0, 1, 0]`
- `fovYRadians`: `1.0472` (60°)
- `near`: `0.01`
- `far`: `10000.0`

---

## 12. Render Context

Provided by the application to the renderer each frame.

```typescript
interface RenderContext {
    frameIndex: number; // Monotonic frame counter
    timeSeconds: number; // App-controlled time (for determinism)
    dtSeconds: number; // Delta since last frame
    width: number; // Viewport width (pixels)
    height: number; // Viewport height (pixels)
    dpr: number; // Device pixel ratio
}
```

**Important:** Renderers MUST NOT call `performance.now()` or `Date.now()`. All time comes from `RenderContext`.

---

## 13. Frame Result

Returned by `render()` for diagnostics.

```typescript
interface FrameResult {
    frameIndex: number;
    nodeCount: number;
    edgeCount: number;
    labelCount: number;
    drawCalls: number;
    triangles: number;
    cpuMs: number;
    gpuMs?: number; // If GPU timing available
}
```

---

## 14. ScenePort Trait

The adapter interface implemented by each renderer.

### 14.1 TypeScript

```typescript
interface ScenePort {
    applySceneDelta(delta: SceneDelta): void;
    setCamera(camera: CameraState): void;
    render(ctx: RenderContext): FrameResult;
    resize(width: number, height: number, dpr: number): void;
    dispose(): void;
}
```

### 14.2 Rust

```rust
pub trait ScenePort {
    fn apply_scene_delta(&mut self, delta: &SceneDelta);
    fn set_camera(&mut self, camera: &CameraState);
    fn render(&mut self, ctx: &RenderContext) -> FrameResult;
    fn resize(&mut self, width: u32, height: u32, dpr: f32);
    fn dispose(&mut self);
}
```

### 14.3 Implementation Notes

- **Instancing:** Adapters SHOULD use instanced rendering for nodes and edges. One draw call per shape type (sphere, cube, etc.) with per-instance transforms and colors.
- **LOD:** For scenes with >1000 nodes, adapters MAY implement level-of-detail, rendering distant nodes as points.
- **Batching:** Scene deltas should be batched per frame. Applying multiple deltas between renders is valid.

---

## 15. MBUS Integration

The TTD Controller emits deltas to MBUS channels:

| Channel              | Payload       | Policy                           |
| -------------------- | ------------- | -------------------------------- |
| `ttd.scene.main`     | `SceneDelta`  | `STRICT_SINGLE` + `LAST` reducer |
| `ttd.scene.timeline` | `SceneDelta`  | `STRICT_SINGLE` + `LAST` reducer |
| `ttd.camera`         | `CameraState` | `STRICT_SINGLE` + `LAST` reducer |

Renderers subscribe to these channels and apply received payloads.

---

## 16. Error Handling

Renderers are **tolerant**:

| Condition                      | Behavior                          |
| ------------------------------ | --------------------------------- |
| Unknown material key           | Use fallback material (magenta)   |
| Edge references missing node   | Skip edge (no error)              |
| Label anchored to missing node | Skip label (no error)             |
| Duplicate epoch                | Drop delta (idempotent)           |
| Invalid float (NaN/Inf)        | Log warning, clamp to valid range |

Renderers MUST NOT throw/panic on malformed deltas. Log and continue.

---

## 17. Versioning

This spec is **version 1**. Future versions will be indicated by a version field in `SceneDelta`:

```typescript
interface SceneDelta {
    version: 1; // Added in future versions
    sceneId: Hash;
    epoch: u64;
    ops: SceneOp[];
}
```

Version 1 deltas without explicit version field are assumed.

---

## Appendix A: Rust Type Definitions

```rust
// crates/echo-scene-port/src/lib.rs

use serde::{Deserialize, Serialize};

pub type Hash = [u8; 32];
pub type NodeKey = Hash;
pub type EdgeKey = Hash;
pub type LabelKey = Hash;
pub type MaterialKey = Hash;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum NodeShape {
    Sphere = 0,
    Cube = 1,
    Cylinder = 2,
    Octahedron = 3,
    Ring = 4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum EdgeStyle {
    Solid = 0,
    Dashed = 1,
    Dotted = 2,
    Flow = 3,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MaterialDef {
    pub key: MaterialKey,
    pub base_color: [u8; 4],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emissive: Option<[u8; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metallic: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roughness: Option<f32>,
    #[serde(default)]
    pub wireframe: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NodeDef {
    pub key: NodeKey,
    pub position: [f32; 3],
    pub radius: f32,
    pub shape: NodeShape,
    pub material: MaterialKey,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<LabelKey>,
    #[serde(default)]
    pub layer: u8,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EdgeDef {
    pub key: EdgeKey,
    pub a: NodeKey,
    pub b: NodeKey,
    pub width: f32,
    pub style: EdgeStyle,
    pub material: MaterialKey,
    #[serde(default)]
    pub directed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub curvature: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flow_speed: Option<f32>,  // For Flow style: particles/sec
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum LabelAnchor {
    Node { key: NodeKey },
    World { position: [f32; 3] },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LabelDef {
    pub key: LabelKey,
    pub text: String,
    pub font_size: f32,
    pub color: [u8; 4],
    pub anchor: LabelAnchor,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<[f32; 3]>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum SceneOp {
    DefineMaterial { def: MaterialDef },
    RemoveMaterial { key: MaterialKey },
    UpsertNode { def: NodeDef },
    RemoveNode { key: NodeKey },
    MoveNode { key: NodeKey, position: [f32; 3] },
    UpsertEdge { def: EdgeDef },
    RemoveEdge { key: EdgeKey },
    UpsertLabel { def: LabelDef },
    RemoveLabel { key: LabelKey },
    Clear,
    ClearLayer { layer: u8 },
    SetHighlight { highlight: HighlightState },
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HighlightState {
    #[serde(default)]
    pub selected_nodes: Vec<NodeKey>,
    #[serde(default)]
    pub selected_edges: Vec<EdgeKey>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hovered_node: Option<NodeKey>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hovered_edge: Option<EdgeKey>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focus_node: Option<NodeKey>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneDelta {
    pub scene_id: Hash,
    pub epoch: u64,
    pub ops: Vec<SceneOp>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectionKind {
    Perspective,
    Orthographic,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CameraState {
    pub position: [f32; 3],
    pub target: [f32; 3],
    #[serde(default = "default_up")]
    pub up: [f32; 3],
    pub projection: ProjectionKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fov_y_radians: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ortho_scale: Option<f32>,
    pub near: f32,
    pub far: f32,
}

fn default_up() -> [f32; 3] {
    [0.0, 1.0, 0.0]
}

#[derive(Clone, Debug)]
pub struct RenderContext {
    pub frame_index: u64,
    pub time_seconds: f64,
    pub dt_seconds: f64,
    pub width: u32,
    pub height: u32,
    pub dpr: f32,
}

#[derive(Clone, Debug, Default)]
pub struct FrameResult {
    pub frame_index: u64,
    pub node_count: usize,
    pub edge_count: usize,
    pub label_count: usize,
    pub draw_calls: usize,
    pub triangles: usize,
    pub cpu_ms: f64,
    pub gpu_ms: Option<f64>,
}

pub trait ScenePort {
    fn apply_scene_delta(&mut self, delta: &SceneDelta);
    fn set_camera(&mut self, camera: &CameraState);
    fn render(&mut self, ctx: &RenderContext) -> FrameResult;
    fn resize(&mut self, width: u32, height: u32, dpr: f32);
    fn dispose(&mut self);
}
```

---

## Appendix B: Example Delta

```json
{
    "scene_id": "746464...2e6d61696e",
    "epoch": 42,
    "ops": [
        {
            "op": "DefineMaterial",
            "def": {
                "key": "abc123...",
                "base_color": [66, 135, 245, 255],
                "roughness": 0.3
            }
        },
        {
            "op": "UpsertNode",
            "def": {
                "key": "node001...",
                "position": [0.0, 1.5, 0.0],
                "radius": 0.1,
                "shape": "Sphere",
                "material": "abc123...",
                "layer": 0
            }
        },
        {
            "op": "UpsertNode",
            "def": {
                "key": "node002...",
                "position": [2.0, 1.5, 0.0],
                "radius": 0.1,
                "shape": "Cube",
                "material": "abc123...",
                "layer": 0
            }
        },
        {
            "op": "UpsertEdge",
            "def": {
                "key": "edge001...",
                "a": "node001...",
                "b": "node002...",
                "width": 0.02,
                "style": "Solid",
                "material": "abc123...",
                "directed": true
            }
        },
        {
            "op": "SetHighlight",
            "highlight": {
                "selected_nodes": ["node001..."],
                "selected_edges": [],
                "hovered_node": "node002..."
            }
        }
    ]
}
```
