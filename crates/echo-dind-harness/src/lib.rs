// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::print_stdout, clippy::print_stderr)]
//! Echo DIND (Deterministic Ironclad Nightmare Drills) harness.
//!
//! This crate provides tooling for running determinism verification scenarios
//! against the Echo kernel, ensuring bit-identical state evolution across runs.

pub mod dind;
