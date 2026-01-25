// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! Time-Travel Debugger compliance engine for Echo.
//!
//! This crate validates that recorded tick emissions conform to declared
//! channel policies, rule contracts, and determinism constraints.
//!
//! # Architecture
//!
//! The compliance engine operates on finalized tick data:
//! - Channel emissions from the materialization bus
//! - Receipts (TTDR) from the session protocol
//! - Schema metadata declaring policies and contracts
//!
//! # Modules
//!
//! - [`compliance`]: Channel policy validation and violation tracking

#![deny(missing_docs)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::cargo)]
#![allow(clippy::module_name_repetitions)]

pub mod compliance;

pub use compliance::{check_channel_policies, PolicyChecker, Severity, Violation, ViolationCode};
