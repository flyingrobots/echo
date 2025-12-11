// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Echo Tasks
//!
//! Implements the TASKS planning logic:
//! - SLAPS intent parsing
//! - HTN method expansion
//! - DAG generation and validation

pub mod method;
pub mod planner;
pub mod slaps;

pub use planner::Planner;
