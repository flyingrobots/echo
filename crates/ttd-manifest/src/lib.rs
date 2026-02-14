// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Vendored TTD protocol manifest and IR.

/// The TTD Intermediate Representation (IR).
pub const TTD_IR: &str = include_str!("../ttd-ir.json");

/// The TTD protocol manifest.
pub const MANIFEST: &str = include_str!("../manifest.json");

/// The TTD behavioral contracts.
pub const CONTRACTS: &str = include_str!("../contracts.json");

/// The TTD protocol schema (JSON).
pub const SCHEMA: &str = include_str!("../schema.json");
