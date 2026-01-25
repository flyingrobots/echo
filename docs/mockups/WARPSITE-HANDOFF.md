<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# WARPSITE Demo - Handoff Document

**Session Date:** 2026-01-24
**Status:** Planning Complete, Mockups Created

---

## Executive Summary

WARPSITE is a demo website that **IS** a WARP graph — everything that happens on the website is powered by Echo ticking. The goal is to prove determinism live by letting visitors fork reality, step multiple forks, and watch the hashes match.

---

## Key Demo Features

### 1. "Split Reality" (Fork Determinism Demo)

- Pause at a "cliff tick" (right before a collision/event)
- Fork into 3 lanes: Baseline (1 worker), Max Cores (16 workers), Chaos Mode (shuffled)
- Step all forks simultaneously
- **Reveal:** All three produce identical `state_root` → determinism proven

### 2. "Show Me Why" (Provenance View)

- Click any atom to see its change history
- Trace causality: Intent → Rule → Atom mutation
- 4D visualization: mouse cursor "worm" through spacetime + atom "pillar" showing value over time

### 3. Time Travel Debugger UX

- Stepper: Play/Pause, Step ±1, Seek slider
- Timeline: Visual markers for intents, forks, rule fires
- Tick Inspector: What rules fired, what patches applied
- Atoms View: Table of all atoms with values, types, provenance

---

## Architecture Decision: Pure WASM

**Chosen approach:** Run Echo entirely in-browser via WASM. No server needed.

```text
Browser
├── WASM Instance A (Fork 1, 1 worker)
├── WASM Instance B (Fork 2, 16 workers)
├── WASM Instance C (Fork 3, shuffled)
├── JS Glue (fork, step, compare hashes)
└── Three.js (4D Provenance View)
```

**Why:**

- No server = no trust issues ("it's running on YOUR machine")
- Static hosting (Vercel/Cloudflare)
- More convincing proof of determinism

---

## What's Already Implemented ✅

| Component               | Location                                  | Status             |
| ----------------------- | ----------------------------------------- | ------------------ |
| MaterializationBus      | `warp-core/src/materialization/bus.rs`    | Done               |
| MBUS Frame V1/V2        | `warp-core/src/materialization/frame*.rs` | Done               |
| PlaybackCursor          | `warp-core/src/playback.rs`               | Done               |
| ViewSession + TruthSink | `warp-core/src/playback.rs`               | Done               |
| Provenance Store        | `warp-core/src/provenance_store.rs`       | Done               |
| JS-ABI Wire Protocol    | `echo-session-proto/src/wire.rs`          | Done               |
| warp-viewer (3D)        | `crates/warp-viewer/`                     | Done (WGPU + egui) |
| WS Gateway              | `echo-session-ws-gateway/`                | Done               |

---

## What Needs to Be Built 🔨

### Phase 1: Wire WASM Bindings (~2 days)

**File:** `crates/warp-wasm/src/lib.rs`

Currently all frozen ABI functions are **placeholders**. Need to wire:

```rust
// Transaction control
#[wasm_bindgen] pub fn begin() -> JsValue;
#[wasm_bindgen] pub fn commit(tx_id: JsValue) -> Uint8Array;

// Playback
#[wasm_bindgen] pub fn seek_to(tick: u64) -> bool;
#[wasm_bindgen] pub fn step() -> Uint8Array; // StepResult

// Provenance
#[wasm_bindgen] pub fn get_state_root() -> Uint8Array;
#[wasm_bindgen] pub fn get_tick() -> u64;

// Fork support
#[wasm_bindgen] pub fn snapshot() -> Uint8Array;
#[wasm_bindgen] pub fn fork_from_snapshot(snapshot: &[u8]) -> JsValue; // new engine handle
```

### Phase 2: Atom Write Attribution (~1 day)

**Gap:** `decision_digest` is just a hash. Need structured "which rule wrote which atom."

Add to provenance:

```rust
pub struct AtomWrite {
    atom_id: NodeId,
    rule_id: RuleId,
    tick: u64,
    old_value: Option<Vec<u8>>,
    new_value: Vec<u8>,
}

// Store alongside each tick
fn append(..., atom_writes: Vec<AtomWrite>)
```

### Phase 3: Three.js Provenance View (~1 week)

| Element       | Three.js                                |
| ------------- | --------------------------------------- |
| Mouse worm    | `TubeGeometry` from `CatmullRomCurve3`  |
| Atom pillar   | Stacked `BoxGeometry`, colored by value |
| Causal arrows | `ArrowHelper`                           |
| Time plane    | `PlaneGeometry` at current tick Z       |
| Scrubber      | Raycaster → jump cursor                 |

### Phase 4: UI Shell (~3 days)

- Stepper bar (play/pause/step/seek)
- Timeline with event markers
- Graph View (reuse warp-viewer's scene graph approach)
- Atoms View (data table)
- Provenance Drawer (slide-out panel)
- Fork Manager panel
- Hash verification display

---

## Mockups Created

All in `docs/mockups/`:

| File                             | Description                                                           |
| -------------------------------- | --------------------------------------------------------------------- |
| `warpsite-main-layout.svg`       | Full debugger layout: stepper, timeline, graph, atoms, tick inspector |
| `warpsite-provenance-drawer.svg` | Slide-out panel showing atom history + causal chain                   |
| `warpsite-fork-comparison.svg`   | Side-by-side fork view with hash verification                         |
| `warpsite-4d-provenance.svg`     | Spacetime visualization: mouse worm, atom pillar, causal arrows       |

**To view:** Open in browser or VS Code preview.

---

## Estimated Timeline

| Phase                  | Work                            | Time   |
| ---------------------- | ------------------------------- | ------ |
| Wire WASM bindings     | Expose engine APIs to JS        | 2 days |
| Atom write attribution | Track rule→atom in provenance   | 1 day  |
| JS glue + fork logic   | `fork()`, `step_all()`, compare | 2 days |
| Three.js 4D view       | Worm, pillar, arrows, scrubber  | 5 days |
| UI shell               | React/Svelte panels, tables     | 3 days |
| Polish + demo script   | "Cliff tick" scenario, copy     | 2 days |

**Total: ~3 weeks** for full demo

---

## Demo Script (Theater)

### Scene: Cliff Tick

Physics sim paused at Tick 47, right before collision.

### Act 1: Split Reality

1. User clicks **"Fork"** twice → 3 lanes appear
2. Lane A: "Baseline (1 worker)"
3. Lane B: "Max Cores (16 workers)"
4. Lane C: "Chaos Mode (shuffled)"
5. User clicks **"Step All"**
6. All three advance, hashes computed
7. **Reveal:** `✅ VERIFIED: A = B = C`

### Act 2: Show Me Why

1. User clicks an atom (e.g., `counter_1`)
2. Provenance drawer slides in
3. Shows: value 47, last write at T45, rule `update_counter`, intent `ClickIntent`
4. User clicks **"View in 4D"**
5. 4D view opens: mouse worm, atom pillar, causal arrow from click to rule to atom

### Act 3: Counterfactual

1. Fork again at T47
2. In Fork C, nudge one input (don't click)
3. Step both
4. **Reveal:** A = B ✅, C diverged ✅ (but still deterministic)

---

## Key Insight

> "The browser becomes the prover."

With server-side: "Trust our infrastructure."
With WASM: "Don't trust us — run it yourself and check the hashes."

This is the same reason blockchain demos run in-browser: **client-side verification is more convincing than server-side claims.**

---

## Knowledge Graph Entities Created

This session added to the persistent knowledge graph:

- `MaterializationBus`, `MBUS_FrameV1`, `MBUS_FrameV2`
- `PlaybackCursor`, `PlaybackMode`, `ViewSession`, `TruthFrame`, `CursorReceipt`, `TruthSink`
- `JS_ABI_WireProtocol`, `echo-session-proto`
- `warp-viewer`, `warp-wasm`, `echo-session-ws-gateway`
- `LocalProvenanceStore`, `WorldlineTickPatchV1`
- `SPEC-0004`

Relations mapped: `IMPLEMENTS`, `CONTAINS`, `SHOULD_EXPOSE`, `STORES`, `FEEDS`, etc.

---

## Questions for Next Session

1. **Cliff tick scenario:** Physics sim? Counter? Domino stack?
2. **Rendering stack for browser:** Three.js? Babylon? Plain Canvas 2D?
3. **State model:** What atoms/nodes should the demo website have?
4. **Deployment target:** Vercel? Cloudflare Pages? Self-hosted?

---

## Files Modified This Session

- Created: `docs/mockups/warpsite-main-layout.svg`
- Created: `docs/mockups/warpsite-provenance-drawer.svg`
- Created: `docs/mockups/warpsite-fork-comparison.svg`
- Created: `docs/mockups/warpsite-4d-provenance.svg`
- Created: `docs/mockups/WARPSITE-HANDOFF.md` (this file)
