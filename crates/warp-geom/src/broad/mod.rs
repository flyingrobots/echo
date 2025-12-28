// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! Broad-phase interfaces and a minimal reference implementation.
//!
//! Determinism contract (applies to all implementations used here):
//! - Pair identity is canonicalized as `(min_id, max_id)`.
//! - The emitted pair list is strictly sorted lexicographically by that tuple.
//! - Overlap is inclusive on faces (touching AABBs are considered overlapping).
//!
//! The current `AabbTree` is an `O(n^2)` all-pairs baseline intended only for
//! early tests. It will be replaced by a deterministic Sweep-and-Prune (and/or
//! a Dynamic AABB Tree) while preserving the ordering and overlap semantics.

#[doc = "Reference AABB-based broad-phase and trait definitions."]
pub mod aabb_tree;
