// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
// CBOR codec with intentional fixed-width casts for wire format compatibility.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::trivially_copy_pass_by_ref
)]
//! CBOR codec and test harness for echo-scene-port.
//!
//! This crate provides:
//! - CBOR encode/decode for all scene port types
//! - MockAdapter for headless testing of ScenePort implementations
//!
//! # Design
//!
//! Serialization is deliberately separated from the port contract.
//! This keeps echo-scene-port pure and dependency-free.

mod cbor;
#[cfg(feature = "test-utils")]
mod mock_adapter;

pub use cbor::*;
#[cfg(feature = "test-utils")]
pub use mock_adapter::*;
