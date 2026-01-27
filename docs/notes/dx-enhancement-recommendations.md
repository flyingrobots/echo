<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Developer Experience (DX) Enhancement Recommendations

This document identifies pain points in the current Echo developer experience and proposes actionable improvements organized by implementation effort.

## Executive Summary

Echo has a strong technical foundation with excellent determinism guarantees, but the developer onboarding path has friction. The README's "How to Use Echo" section is comprehensive but overwhelming for newcomers. Key opportunities exist in:

1. **Reducing boilerplate** for common patterns (rule creation, engine setup)
2. **Providing interactive examples** that run out of the box
3. **Better error messages** with actionable guidance
4. **CLI tooling** to scaffold projects and debug simulations

---

## Quick Wins (Easy to Implement, High Impact)

### 1. Add a "Hello World" Example Binary

**Current Pain Point:** The README's "How to Use Echo" section jumps straight into complex concepts (graph model, rewrite rules, footprints) without a runnable "hello world" that newcomers can execute immediately.

**Proposed Enhancement:** Create `crates/warp-core/examples/hello_world.rs`:

```rust
//! Minimal Echo example: a ball that moves deterministically.
//! Run with: cargo run --example hello_world -p warp-core

use warp_core::{
    make_node_id, make_type_id, Engine, EngineBuilder, GraphStore, NodeRecord,
};

fn main() {
    let mut store = GraphStore::default();
    let world = make_node_id("world");
    store.insert_node(world, NodeRecord { ty: make_type_id("world") });

    let mut engine = EngineBuilder::new(store, world).build();

    for tick in 0..10 {
        let receipt = engine.commit();
        println!("Tick {}: hash={:x?}", tick, &receipt.commit_id[..4]);
    }
}
```

**Expected Impact:** Developers can run `cargo run --example hello_world` within 30 seconds of cloning the repo.

---

### 2. Create a Rule Builder / Macro to Reduce Boilerplate

**Current Pain Point:** Defining a `RewriteRule` requires filling out 9 fields including a manual rule ID hash, pattern graph, matcher fn, executor fn, footprint fn, and conflict policy. This is shown in the README but is intimidating for newcomers.

**Proposed Enhancement:** Add a `rule!` macro or builder in `echo-dry-tests` (or a new `warp-macros` crate):

```rust
// Before: 40+ lines of boilerplate
let rule = RewriteRule {
    id: motion_rule_id(),
    name: "motion/update",
    left: PatternGraph { nodes: vec![] },
    matcher: motion_matcher,
    executor: motion_executor,
    compute_footprint: compute_motion_footprint,
    factor_mask: 0,
    conflict_policy: ConflictPolicy::Abort,
    join_fn: None,
};

// After: Declarative macro
let rule = rule! {
    name: "motion/update",
    matcher: |view, scope| view.node_attachment(scope).is_some(),
    executor: |view, scope, delta| {
        // Update logic
    },
    reads: [node(scope), attachment(scope)],
    writes: [attachment(scope)],
};
```

**Expected Impact:** Rule definitions shrink from 40+ lines to ~10 lines; footprint declarations become explicit and readable.

---

### 3. Improve Error Messages with Actionable Hints

**Current Pain Point:** Error variants like `EngineError::UnknownRule("missing-rule")` tell you what happened but not how to fix it.

**Proposed Enhancement:** Extend error messages to include hints:

```rust
// Current
#[error("rule not registered: {0}")]
UnknownRule(String),

// Proposed
#[error("rule not registered: '{0}'\n  Hint: Register rules before applying them:\n    engine.register_rule(my_rule())?;\n  Available rules: {1}")]
UnknownRule(String, String), // second arg = comma-separated registered rule names
```

Similarly for `DuplicateRuleName`:

```rust
#[error("duplicate rule name: '{0}'\n  Hint: Each rule must have a unique name. Check if you're registering the same rule twice.")]
```

**Expected Impact:** Developers self-diagnose common mistakes without consulting docs or source code.

---

### 4. Add `cargo xtask new-rule` Scaffolding Command

**Current Pain Point:** Creating a new rule requires copying ~60 lines from an existing rule and modifying it.

**Proposed Enhancement:** Add a scaffolding command:

```bash
cargo xtask new-rule my-game/player-move
# Creates: crates/my-game/src/rules/player_move.rs with:
# - Rule skeleton with TODOs
# - Matcher stub
# - Executor stub
# - Footprint stub
# - Test file with determinism check
```

**Expected Impact:** New rules are correctly structured from the start; developers focus on logic instead of boilerplate.

---

### 5. Document the "Why" Behind Footprints

**Current Pain Point:** The README shows footprint code but doesn't explain why it matters. Newcomers copy-paste without understanding, leading to subtle bugs.

**Proposed Enhancement:** Add a conceptual section before the code example:

```markdown
### Why Footprints Matter

Footprints declare what your rule reads and writes. This enables:

1. **Parallel safety** - Rules with disjoint footprints run concurrently
2. **Conflict detection** - Overlapping writes are detected and resolved
3. **Slicing** - History replay only includes relevant rules

**The footprint contract:** Your executor must not access anything outside its declared footprint.
If you read a node, declare it in `n_read`. If you write an attachment, declare it in `a_write`.
```

**Expected Impact:** Developers understand footprints conceptually before seeing the implementation.

---

## Medium Effort (Valuable but Requires More Work)

### 6. Create an Interactive Tutorial Crate

**Current Pain Point:** Learning Echo requires reading a lot of documentation. There's no guided, hands-on tutorial that builds understanding incrementally.

**Proposed Enhancement:** Create `crates/echo-tutorial/` with progressive lessons:

```
echo-tutorial/
  lessons/
    01_hello_tick.rs      # Create engine, commit, verify hash
    02_first_node.rs      # Add nodes, understand NodeId/TypeId
    03_attachments.rs     # Add data to nodes
    04_first_rule.rs      # Simple matcher/executor
    05_determinism.rs     # Run two engines, verify hashes match
    06_parallel.rs        # Worker count, footprint importance
```

Each lesson is a runnable binary with inline comments and exercises:

```rust
// Lesson 1: Your First Tick
//
// Run this: cargo run -p echo-tutorial --bin lesson_01
//
// EXERCISE: Modify the code to run 100 ticks instead of 10.
// Verify the final hash is: 0x7f3a...
```

**Expected Impact:** Structured learning path; developers gain intuition before tackling real projects.

---

### 7. Add `cargo xtask doctor` Diagnostic Command

**Current Pain Point:** When something doesn't work, developers must manually check:

- Rust toolchain version
- Node.js installation
- Wesley repo location
- Pre-commit hooks installed
- Lockfile format

**Proposed Enhancement:**

```bash
$ cargo xtask doctor

Echo Development Environment Check
==================================
[ok] Rust toolchain: 1.90.0 (matches rust-toolchain.toml)
[ok] Cargo.lock format: v4
[ok] Git hooks installed: .githooks/pre-commit
[warn] Wesley repo not found at ~/git/Wesley
       Run: git clone https://github.com/flyingrobots/Wesley ~/git/Wesley
[ok] Node.js: v18.17.0
[ok] pnpm: 8.6.0

1 warning(s) found. See above for details.
```

**Expected Impact:** Environment issues are diagnosed in seconds; CI failures become reproducible locally.

---

### 8. Create a WASM Playground for Browser-Based Experimentation

**Current Pain Point:** Trying Echo requires cloning the repo and building locally. The "WARPSITE" roadmap item hints at this but isn't available yet.

**Proposed Enhancement:** Expand `ttd-browser` or create a new `echo-playground` crate:

- Run simple Echo simulations in the browser
- Edit rules via a Monaco editor
- Visualize state graph changes
- Compare hashes across ticks

**Expected Impact:** Zero-install experimentation; shareable playground links for bug reports.

---

### 9. Add Determinism Debugging CLI

**Current Pain Point:** When determinism fails, the error is "hashes diverged" with no insight into where or why.

**Proposed Enhancement:**

```bash
cargo xtask debug-divergence --trace-a trace_a.json --trace-b trace_b.json

Divergence detected at tick 47:
  Engine A: state_root = 0x1234...
  Engine B: state_root = 0x5678...

First differing operation:
  A: SetAttachment { key: node(player:1)/alpha, value: pos=[100,200,0] }
  B: SetAttachment { key: node(player:1)/alpha, value: pos=[100,201,0] }

Likely cause: Non-deterministic calculation in rule "physics/gravity"
  - Check for float operations without det_fixed feature
  - Verify PRNG seed is consistent
```

**Expected Impact:** Determinism bugs become tractable; root cause identified in minutes instead of hours.

---

### 10. Generate API Documentation with Examples

**Current Pain Point:** `cargo doc` generates reference docs, but they lack usage examples. The README has examples but they're not linked to the API docs.

**Proposed Enhancement:**

1. Add `#[doc = include_str!("../examples/...")]` to key types
2. Add doc-tests that run as part of CI
3. Generate a "cookbook" section in the docs site

````rust
/// Creates a deterministic hash from a string label.
///
/// # Example
///
/// ```rust
/// use warp_core::make_node_id;
///
/// let player = make_node_id("player:1");
/// let same_player = make_node_id("player:1");
/// assert_eq!(player, same_player); // Same label = same ID
/// ```
pub fn make_node_id(label: &str) -> NodeId { ... }
````

**Expected Impact:** API docs become self-teaching; examples are guaranteed to compile.

---

## Future Vision (Longer-Term Ideas)

### 11. "Echo Studio" - Visual Rule Editor

**Vision:** A desktop/web application for:

- Visual graph editing (drag nodes, connect edges)
- Rule authoring with live preview
- Step-through debugging with state inspection
- Export to Rust code

**Why It Matters:** Reduces the barrier from "I want to try Echo" to "I'm building something real."

---

### 12. Protocol Buffers / Cap'n Proto Alternative to CBOR

**Vision:** While CBOR is deterministic with the right encoder, some teams prefer established schemas like Protocol Buffers or Cap'n Proto for:

- Cross-language code generation
- Binary compatibility tooling
- Existing ecosystem integration

**Why It Matters:** Enterprise adoption often requires familiar wire formats.

---

### 13. Language Server Protocol (LSP) for Rules

**Vision:** IDE integration that provides:

- Autocomplete for rule names in `apply_by_name`
- Type checking for footprint declarations
- "Go to definition" for rule references
- Lint warnings for footprint violations

**Why It Matters:** IDE support dramatically improves productivity and catches errors early.

---

### 14. `echo init` Project Generator

**Vision:** Similar to `cargo init` but for Echo projects:

```bash
echo init my-game --template puzzle-game
# Creates:
# my-game/
#   Cargo.toml (with warp-core dependency)
#   src/
#     main.rs (engine setup)
#     rules/
#       mod.rs
#       player.rs (scaffold)
#     payloads/
#       mod.rs
#       position.rs (scaffold)
#   tests/
#     determinism.rs (template)
```

**Why It Matters:** Correct project structure from day one; best practices baked in.

---

### 15. Community Examples Gallery

**Vision:** A curated collection of complete, working examples:

- **Puzzle Game** - Sokoban-style box pushing
- **Physics Demo** - Bouncing balls with collision
- **Networked Game** - Two-player lockstep
- **AI Simulation** - Agent pathfinding

Each example includes:

- Source code with detailed comments
- Live demo (via WASM)
- "Extend this" exercises

**Why It Matters:** Learning by example is often faster than reading docs.

---

## Summary of Priorities

| Priority | Enhancement                   | Effort | Impact |
| -------- | ----------------------------- | ------ | ------ |
| 1        | Hello World example           | Low    | High   |
| 2        | Rule builder macro            | Low    | High   |
| 3        | Better error messages         | Low    | Medium |
| 4        | `xtask new-rule` scaffolding  | Low    | Medium |
| 5        | Footprint "why" documentation | Low    | Medium |
| 6        | Interactive tutorial crate    | Medium | High   |
| 7        | `xtask doctor` command        | Medium | Medium |
| 8        | WASM playground               | Medium | High   |
| 9        | Determinism debugging CLI     | Medium | High   |
| 10       | API docs with examples        | Medium | Medium |

---

## Appendix: Current Strengths to Preserve

The following aspects of Echo's DX are already strong and should be maintained:

1. **Comprehensive README** - The "How to Use Echo" section covers all major concepts
2. **Strong type system** - `NodeId`, `TypeId`, `WarpId` prevent ID confusion
3. **Determinism-first design** - The `det_fixed` feature and PRNG controls are excellent
4. **Git hooks** - Auto-formatting and Wesley checks catch issues early
5. **DIND testing** - Cross-platform determinism verification is sophisticated
6. **Wesley schema-first** - Single source of truth for protocol types
7. **Documentation variety** - ELI5, primer, specs cover different learning styles
