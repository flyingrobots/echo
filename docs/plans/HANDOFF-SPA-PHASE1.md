<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# HANDOFF: Scene Port Adapter — Phase 1 (Rust Crates)

**Date:** 2026-01-25
**From:** ARCHY (Architecture Planning)
**To:** Implementation Agent
**Status:** Ready to Begin

---

## TL;DR

You are implementing **Phase 1** of the Scene Port Adapter plan: two Rust crates that define the rendering port contract for Echo's Time Travel Debugger.

**Your deliverables:**

1. `crates/echo-scene-port/` — Pure types + trait (NO serde, NO external deps)
2. `crates/echo-scene-codec/` — CBOR codec + MockAdapter for testing

**Start with:** Task SPA.1.1 (create echo-scene-port crate structure)

---

## Context

### What is this?

Echo's TTD (Time Travel Debugger) needs a hexagonal rendering architecture where:

- **Domain logic** (TTD Controller) emits `SceneDelta` messages
- **Renderers** (Three.js, wgpu) implement `ScenePort` trait and just render

This decoupling enables deterministic replay—renderers don't own time or make decisions.

### Why two crates?

| Crate              | Purpose                      | Dependencies              |
| ------------------ | ---------------------------- | ------------------------- |
| `echo-scene-port`  | Pure domain contract         | None (no_std capable)     |
| `echo-scene-codec` | Serialization + test harness | minicbor, echo-scene-port |

Serialization is explicitly separated from the contract to keep the port crate pristine.

---

## Reference Documents

Read these before starting:

1. **Plan:** `docs/plans/scene-port-adapter-plan.md`
    - Full architectural context
    - Type definitions (copy from here)
    - Epoch semantics (cursor-scoped)

2. **Tasks:** `docs/plans/scene-port-adapter-tasks.md`
    - Detailed requirements per task
    - Acceptance criteria checklists
    - Scope boundaries

3. **DAG:** `docs/assets/dags/scene-port-adapter-dag-elk.svg`
    - Visual dependency graph
    - Shows what can be parallelized

4. **Scene Delta Spec:** `docs/specs/scene-delta.md`
    - Original specification (some parts superseded by plan)
    - Good for understanding intent

---

## Your Tasks (Phase 1)

### Task Order

```text
SPA.1.1 ──┬──► SPA.1.2 ──┬──► SPA.1.4 ──┬──► SPA.1.6 ──► SPA.2.1 ──► SPA.2.2 ──┬──► SPA.2.3 ──► SPA.2.5
          │              │              │                                       │
          ├──► SPA.1.3 ──┘              │                                       └──► SPA.2.4
          │                             │
          └──► SPA.1.5 ─────────────────┘
```

**Parallelizable after SPA.1.1:**

- SPA.1.2, SPA.1.3, SPA.1.5 can run in parallel

**Parallelizable after SPA.2.2:**

- SPA.2.3, SPA.2.4 can run in parallel

---

## Task SPA.1.1 — Create Crate Structure

### Files to Create

```text
crates/echo-scene-port/
├── Cargo.toml
└── src/
    └── lib.rs
```

### Cargo.toml

```toml
[package]
name = "echo-scene-port"
version = "0.1.0"
edition = "2021"
description = "Scene rendering port contract for Echo TTD"
license = "Apache-2.0 OR MIND-UCAL-1.0"

[dependencies]
# NO dependencies. Pure types only.

[features]
default = ["std"]
std = []
```

### src/lib.rs

```rust
//! Scene port contract for Echo renderers.
//!
//! This crate defines the domain contract between TTD Controller and renderers.
//! It contains NO serialization logic—that lives in echo-scene-codec.
//!
//! # Design Principles
//!
//! - **Renderers are dumb** — They receive deltas and render. No domain logic.
//! - **No time ownership** — All timing comes from the app, not the renderer.
//! - **Cursor-scoped epochs** — Deltas are idempotent per (cursor_id, epoch).
//!
//! # Crate Features
//!
//! - `std` (default): Enables std library. Disable for no_std contexts.

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

### Update Workspace Cargo.toml

Add to `members` in the root `Cargo.toml`:

```toml
members = [
    # ... existing members ...
    "crates/echo-scene-port",
]
```

### Acceptance Criteria

- [ ] `cargo build -p echo-scene-port` passes (will have warnings about missing modules)
- [ ] `cargo build -p echo-scene-port --no-default-features` passes
- [ ] Crate appears in workspace

---

## Task SPA.1.2 — Implement types.rs

### File: `crates/echo-scene-port/src/types.rs`

Copy types from the plan. Key points:

```rust
use alloc::string::String;
use alloc::vec::Vec;

/// 32-byte content-addressed key.
pub type Hash = [u8; 32];
pub type NodeKey = Hash;
pub type EdgeKey = Hash;
pub type LabelKey = Hash;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum NodeShape {
    Sphere = 0,
    Cube = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum EdgeStyle {
    Solid = 0,
    Dashed = 1,
}

pub type ColorRgba8 = [u8; 4];

#[derive(Clone, Debug, PartialEq)]
pub struct NodeDef {
    pub key: NodeKey,
    pub position: [f32; 3],
    pub radius: f32,
    pub shape: NodeShape,
    pub color: ColorRgba8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EdgeDef {
    pub key: EdgeKey,
    pub a: NodeKey,
    pub b: NodeKey,
    pub width: f32,
    pub style: EdgeStyle,
    pub color: ColorRgba8,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LabelAnchor {
    Node { key: NodeKey },
    World { position: [f32; 3] },
}

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
    pub session_id: Hash,
    pub cursor_id: Hash,
    pub epoch: u64,
    pub ops: Vec<SceneOp>,
}
```

### DO NOT include

- `MoveNode` (use UpsertNode)
- `ClearLayer` (no layers in MVP)
- `SetHighlight` (separate method)
- Materials (color is inline)
- Serde derives

---

## Task SPA.1.3 — Implement camera.rs

```rust
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
    pub fov_y_radians: f32,
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

---

## Task SPA.1.4 — Implement highlight.rs

```rust
use alloc::vec::Vec;
use crate::types::{NodeKey, EdgeKey};

/// Highlight state for selection/hover feedback.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HighlightState {
    pub selected_nodes: Vec<NodeKey>,
    pub selected_edges: Vec<EdgeKey>,
    pub hovered_node: Option<NodeKey>,
    pub hovered_edge: Option<EdgeKey>,
}
```

---

## Task SPA.1.5 — Implement canon.rs

```rust
/// Canonicalize a float for deterministic comparison and hashing.
///
/// **Purpose:** Used by codec layer when computing content hashes.
/// Do NOT use this to mutate stored positions—it's a projection for comparison.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_negative_zero() {
        assert_eq!(canonicalize_f32(-0.0), 0.0);
    }

    #[test]
    fn test_truncation() {
        assert_eq!(canonicalize_f32(1.23456789), 1.234567);
    }

    #[test]
    #[should_panic(expected = "NaN")]
    fn test_nan_panics() {
        canonicalize_f32(f32::NAN);
    }

    #[test]
    #[should_panic(expected = "Infinity")]
    fn test_infinity_panics() {
        canonicalize_f32(f32::INFINITY);
    }
}
```

---

## Task SPA.1.6 — Implement port.rs

```rust
use crate::{SceneDelta, CameraState, HighlightState};

/// Scene rendering port trait.
///
/// Implementors receive deltas and render. No time ownership.
/// RenderContext/FrameResult are adapter-local concerns, not part of this contract.
pub trait ScenePort {
    /// Apply a scene delta. Idempotent per (cursor_id, epoch).
    fn apply_scene_delta(&mut self, delta: &SceneDelta);

    /// Set camera state.
    fn set_camera(&mut self, camera: &CameraState);

    /// Set highlight state (selection/hover).
    fn set_highlight(&mut self, highlight: &HighlightState);

    /// Render the current scene.
    ///
    /// Takes no parameters—profiling/timing is the adapter's concern.
    fn render(&mut self);

    /// Resize viewport.
    fn resize(&mut self, width: u32, height: u32, dpr: f32);

    /// Reset epoch tracking for a cursor.
    ///
    /// This ONLY clears epoch tracking. Scene state is NOT cleared.
    /// Use `SceneOp::Clear` to clear the scene.
    fn reset_cursor(&mut self, cursor_id: &[u8; 32]);

    /// Dispose all resources.
    fn dispose(&mut self);
}
```

---

## Phase 2 Tasks (After Phase 1 Complete)

Once SPA.1.6 is done, proceed to:

### SPA.2.1 — Create echo-scene-codec crate

```toml
[package]
name = "echo-scene-codec"
version = "0.1.0"
edition = "2021"

[dependencies]
echo-scene-port = { path = "../echo-scene-port" }
minicbor = { version = "0.24", features = ["alloc"] }
```

### SPA.2.2 — Implement CBOR codec

Implement `Encode` and `Decode` for all types using minicbor.

### SPA.2.3 — Implement MockAdapter

Headless adapter that tracks state in HashMaps.

### SPA.2.4 — Roundtrip tests

### SPA.2.5 — MockAdapter tests

---

## Banned Patterns

**DO NOT USE:**

- `serde` or any serialization in `echo-scene-port`
- `performance.now()` or any timing
- `HashMap` iteration order (use sorted keys)
- Global state / singletons
- Floating point operations that produce NaN/Infinity

**VERIFY:**

```bash
# No serde in port crate
cargo tree -p echo-scene-port | grep -i serde && echo "FAIL: serde found" || echo "PASS"

# Compiles without std
cargo build -p echo-scene-port --no-default-features
```

---

## Definition of Done (Phase 1)

Phase 1 is complete when:

- [ ] `cargo build -p echo-scene-port` passes
- [ ] `cargo build -p echo-scene-port --no-default-features` passes
- [ ] `cargo test -p echo-scene-port` passes (canon tests)
- [ ] `cargo build -p echo-scene-codec` passes
- [ ] `cargo test -p echo-scene-codec` passes (roundtrip + MockAdapter tests)
- [ ] No serde in echo-scene-port dependency tree
- [ ] All types match plan specification

---

## Questions?

If blocked or unclear:

1. Check `docs/plans/scene-port-adapter-plan.md` first
2. Check `docs/plans/scene-port-adapter-tasks.md` for detailed requirements
3. If still unclear, ask before proceeding

**Key architectural decisions are already made. Just implement the spec.**
