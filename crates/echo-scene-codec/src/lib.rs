// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
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
mod mock_adapter;

pub use cbor::*;
pub use mock_adapter::*;
