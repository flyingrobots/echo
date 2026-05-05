// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! METHOD workspace operations.
//!
//! This crate reads a METHOD workspace from the filesystem and answers
//! questions about it: backlog lane counts, active cycles, legend load.
//!
//! It has no dependency on Echo or any other project. It could live in
//! its own repo.

pub mod graph;
pub mod inbox;
pub mod status;
pub mod workspace;
