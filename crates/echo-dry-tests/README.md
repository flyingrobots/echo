<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# echo-dry-tests

Shared test doubles and fixtures for Echo crates.

## Purpose

This crate provides reusable test utilities including:

- In-memory configuration store for testing without filesystem
- Demo rules (motion, port) for integration tests
- Engine and GraphStore builder utilities
- WarpSnapshot and WarpDiff builders
- Hash ID generation helpers
- Motion payload encoding helpers
- Synthetic rule builders (noop matchers/executors)

## Public API

### Config Store

- **`InMemoryConfigStore`** - An in-memory configuration store implementing the
  config trait, useful for testing without touching the filesystem.

### Demo Rules

- **`MOTION_RULE_NAME`** - Constant string `"motion/update"` identifying the
  built-in motion update rule.
- **`motion_rule()`** - Returns a `RewriteRule` that updates entity positions
  based on velocity (position += velocity each tick).
- **`build_motion_demo_engine()`** - Constructs a demo `Engine` with a
  world-root node and motion rule pre-registered.
- **`PORT_RULE_NAME`** - Constant string `"demo/port_nop"` identifying the port
  demo rule.
- **`port_rule()`** - Returns a demo `RewriteRule` that reserves a boundary
  input port.
- **`build_port_demo_engine()`** - Builds an engine with a world root for
  port-rule tests.

### Engine Builders

- **`EngineTestBuilder`** - Fluent builder for constructing test engines with
  custom configuration.
- **`build_engine_with_root(name)`** - Quickly build an engine with a named root
  node.
- **`build_engine_with_typed_root(name, type_name)`** - Build an engine with a
  named and typed root node.

### Frame Builders

- **`SnapshotBuilder`** - Builder for constructing `WarpSnapshot` test fixtures.
- **`DiffBuilder`** - Builder for constructing `WarpDiff` test fixtures.

### Hash Helpers

- **`make_rule_id(name)`** - Generate a deterministic rule ID hash from a name.
- **`make_intent_id(name)`** - Generate a deterministic intent ID hash from a
  name.

### Motion Helpers

- **`MotionPayloadBuilder`** - Builder for constructing motion payloads.
- **`DEFAULT_MOTION_POSITION`** - Default position `[0.0, 0.0, 0.0]` for motion
  payloads.
- **`DEFAULT_MOTION_VELOCITY`** - Default velocity `[0.0, 0.0, 0.0]` for motion
  payloads.

### Synthetic Rules

- **`NoOpRule`** - A rule that matches everything but does nothing, useful for
  testing rule registration and scheduling.
- **`SyntheticRuleBuilder`** - Builder for creating custom synthetic rules with
  configurable matchers and executors.

## Usage

Add as a dev-dependency in your crate's `Cargo.toml`:

```toml
[dev-dependencies]
echo-dry-tests = { workspace = true }
```

### Example: Using the Motion Rule

```rust
use echo_dry_tests::{motion_rule, MOTION_RULE_NAME};
use warp_core::Engine;

// Create an engine and register the motion rule
let mut engine = Engine::default();
engine.register_rule(motion_rule()).unwrap();

// The rule is now registered under MOTION_RULE_NAME ("motion/update")
// and will update position += velocity for matching nodes each tick.
```

### Example: Quick Engine Setup

```rust
use echo_dry_tests::build_motion_demo_engine;

// Get a fully configured engine with world-root and motion rule
let engine = build_motion_demo_engine();
```
