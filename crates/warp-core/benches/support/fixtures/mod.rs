// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Benchmark scene fixtures, first slice.
//!
//! See `docs/benchmarks/scene-fixtures.md` for the contract. This slice
//! provides:
//!
//! - [`prng`]: SplitMix64 and its bounded-integer mapping, pinned by vectors.
//! - [`manifest`]: workload and expectation identities, digested separately.
//! - [`outcome`]: result classes and work accounting.
//! - [`star`]: control C1, the star scene `action_scene` builds.
//! - [`think_append`]: fixture F5, append and replay through the trusted host.

pub mod manifest;
pub mod outcome;
pub mod prng;
pub mod star;
pub mod think_append;
