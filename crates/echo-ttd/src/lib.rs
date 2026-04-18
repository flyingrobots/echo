// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! Runtime-side compliance and receipt validation for Echo.
//!
//! This crate validates that recorded tick emissions conform to declared
//! channel policies, rule contracts, and determinism constraints.
//!
//! # Architecture
//!
//! The compliance engine operates on finalized runtime data:
//! - Channel emissions from the materialization bus
//! - Receipts (TTDR) from the session protocol
//! - Schema metadata declaring policies and contracts
//!
//! It is a runtime-side support crate, not the debugger product itself.
//!
//! # Modules
//!
//! - [`compliance`]: Channel policy validation and violation tracking

// Workspace lints apply via [lints] workspace = true.

pub mod compliance;

pub use compliance::{check_channel_policies, PolicyChecker, Severity, Violation, ViolationCode};
