<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# echo-renderer-three

Three.js ScenePort adapter for Echo Time-Travel Debugger (TTD).

## Overview

This package implements the **hexagonal rendering port** pattern for Echo's TTD. Renderers are "dumb projection surfaces" — they receive `SceneDelta` messages and render. No domain logic. No time ownership.

### Design Principles

- **Renderers don't own time** — The app controls when `render()` is called
- **Deltas are idempotent** — Same `(cursorId, epoch)` is always a no-op
- **No banned APIs** — No `performance.now()`, `Date.now()`, or `Math.random()`
- **Testable without GPU** — `SceneState` is pure TypeScript with no Three.js dependency

## Installation

```bash
pnpm add echo-renderer-three three
# or
npm install echo-renderer-three three
```

Three.js is a peer dependency (>=0.150.0).

## Quick Start

```typescript
import {
    ThreeSceneAdapter,
    type SceneDelta,
    type RenderContext,
} from "echo-renderer-three";

// Create adapter with a canvas
const canvas = document.getElementById("canvas") as HTMLCanvasElement;
const adapter = new ThreeSceneAdapter(canvas, {
    antialias: true,
    pixelRatio: window.devicePixelRatio,
});

// Apply deltas from TTD Controller
const delta: SceneDelta = {
    sessionId: new Uint8Array(32), // 32-byte session ID
    cursorId: new Uint8Array(32), // 32-byte cursor ID
    epoch: 0, // Monotonic per cursor
    ops: [
        {
            op: "UpsertNode",
            def: {
                key: new Uint8Array(32), // 32-byte node key
                position: [0, 0, 0],
                radius: 1.0,
                shape: 0, // 0 = Sphere, 1 = Cube
                color: [255, 100, 50, 255], // RGBA 0-255
            },
        },
    ],
};
adapter.applySceneDelta(delta);

// Render loop (app controls timing)
function animate(time: number) {
    const ctx: RenderContext = {
        frameIndex: Math.floor(time / 16),
        timeSeconds: time / 1000,
        dtSeconds: 0.016,
        width: canvas.width,
        height: canvas.height,
        dpr: window.devicePixelRatio,
    };

    adapter.render(ctx);
    requestAnimationFrame(animate);
}
requestAnimationFrame(animate);
```

## API Reference

### `ThreeSceneAdapter`

The main ScenePort implementation.

```typescript
class ThreeSceneAdapter implements ScenePort {
    constructor(canvas: HTMLCanvasElement, options?: ThreeSceneAdapterOptions);

    // ScenePort interface
    applySceneDelta(delta: SceneDelta): void;
    setCamera(camera: CameraState): void;
    setHighlight(highlight: HighlightState): void;
    render(ctx: RenderContext): void;
    resize(width: number, height: number, dpr: number): void;
    resetCursor(cursorId: Hash): void;
    dispose(): void;

    // Accessors
    readonly nodeCount: number;
    readonly edgeCount: number;
    readonly labelCount: number;
    readonly threeScene: THREE.Scene;
}
```

#### Options

```typescript
interface ThreeSceneAdapterOptions {
    antialias?: boolean; // Default: true
    pixelRatio?: number; // Default: 1
    powerPreference?: "default" | "high-performance" | "low-power";
    profiler?: Profiler; // Optional performance profiler
    backgroundColor?: number; // Hex color, e.g. 0x000000
}
```

### Scene Operations

```typescript
type SceneOp =
    | { op: "UpsertNode"; def: NodeDef }
    | { op: "RemoveNode"; key: Hash }
    | { op: "UpsertEdge"; def: EdgeDef }
    | { op: "RemoveEdge"; key: Hash }
    | { op: "UpsertLabel"; def: LabelDef }
    | { op: "RemoveLabel"; key: Hash }
    | { op: "Clear" };
```

### Types

```typescript
// 32-byte content-addressed key
type Hash = Uint8Array; // length 32

// Node definition
interface NodeDef {
    key: Hash;
    position: [number, number, number];
    radius: number;
    shape: 0 | 1; // 0 = Sphere, 1 = Cube
    color: [number, number, number, number]; // RGBA 0-255
}

// Edge definition
interface EdgeDef {
    key: Hash;
    a: Hash; // Source node key
    b: Hash; // Target node key
    width: number;
    style: 0 | 1; // 0 = Solid, 1 = Dashed
    color: [number, number, number, number];
}

// Label definition
interface LabelDef {
    key: Hash;
    text: string;
    fontSize: number;
    color: [number, number, number, number];
    anchor:
        | { kind: "Node"; key: Hash }
        | { kind: "World"; position: [number, number, number] };
    offset: [number, number, number];
}

// Camera state
interface CameraState {
    position: [number, number, number];
    target: [number, number, number];
    up: [number, number, number];
    projection: "Perspective" | "Orthographic";
    fovYRadians: number; // For perspective
    orthoScale: number; // For orthographic
    near: number;
    far: number;
}

// Highlight state
interface HighlightState {
    selectedNodes: Hash[];
    selectedEdges: Hash[];
    hoveredNode?: Hash;
    hoveredEdge?: Hash;
}
```

## Epoch Semantics

Deltas are **idempotent per cursor**. Each delta has:

- `sessionId`: Identifies the TTD session
- `cursorId`: Identifies the playback cursor (enables parallel worldlines)
- `epoch`: Monotonically increasing per cursor

**Rules:**

1. If `epoch <= lastEpoch` for this cursor → no-op (already applied)
2. `resetCursor(cursorId)` clears epoch tracking (allows epoch restart)
3. Different cursors have independent epoch tracking

**For timeline rewind:**

```typescript
// Controller sends Clear + resetCursor before replaying
adapter.applySceneDelta({ ...delta, ops: [{ op: "Clear" }] });
adapter.resetCursor(cursorId);
// Now can send epoch 0 again
```

## Profiling

Inject a profiler to measure performance:

```typescript
const profiler: Profiler = {
    markStart(label: string) {
        performance.mark(`${label}-start`);
    },
    markEnd(label: string) {
        performance.mark(`${label}-end`);
        performance.measure(label, `${label}-start`, `${label}-end`);
        return performance.getEntriesByName(label).pop()?.duration ?? 0;
    },
};

const adapter = new ThreeSceneAdapter(canvas, { profiler });
```

Labels used: `render`, `applyDelta`, `syncObjects`, `highlight`, `draw`

## What's NOT Supported (MVP)

These are deferred to Phase 2:

| Feature               | MVP Alternative                |
| --------------------- | ------------------------------ |
| Instanced rendering   | Individual meshes              |
| TubeGeometry edges    | LineSegments                   |
| SDF fonts             | CanvasTexture sprites          |
| Outline highlights    | Color tint                     |
| MoveNode op           | UpsertNode (replaces position) |
| Multi-scene/worldline | Single scene                   |

## License

Apache-2.0
