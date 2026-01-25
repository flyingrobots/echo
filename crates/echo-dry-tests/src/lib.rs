// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Shared test doubles and fixtures for Echo crates.
#![forbid(unsafe_code)]
//!
//! This crate provides commonly used test utilities to reduce duplication
//! across the Echo test suite and improve test maintainability.
//!
//! # Modules
//!
//! - [`config`] - In-memory config store fake for testing without filesystem
//! - [`demo_rules`] - Demo rules (motion, port) for integration tests
//! - [`engine`] - Engine and GraphStore builder utilities
//! - [`footprint`] - Ergonomic footprint construction via builder pattern
//! - [`frames`] - WarpSnapshot and WarpDiff builders
//! - [`hashes`] - Hash ID generation helpers (rule_id, intent_id, etc.)
//! - [`motion`] - Motion payload encoding helpers
//! - [`rules`] - Synthetic rule builders (noop matchers/executors)

pub mod config;
pub mod demo_rules;
pub mod engine;
pub mod footprint;
pub mod frames;
pub mod hashes;
pub mod motion;
pub mod rules;

// Re-export commonly used items at crate root for convenience
pub use config::InMemoryConfigStore;
pub use demo_rules::{
    build_motion_demo_engine, build_port_demo_engine, motion_rule, port_rule, MOTION_RULE_NAME,
    PORT_RULE_NAME,
};
pub use engine::{build_engine_with_root, build_engine_with_typed_root, EngineTestBuilder};
pub use footprint::FootprintBuilder;
pub use frames::{DiffBuilder, SnapshotBuilder};
pub use hashes::{make_intent_id, make_rule_id};
pub use motion::{MotionPayloadBuilder, DEFAULT_MOTION_POSITION, DEFAULT_MOTION_VELOCITY};
pub use rules::{NoOpRule, SyntheticRuleBuilder};
