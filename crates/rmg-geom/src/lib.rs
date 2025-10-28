#![deny(
    warnings,
    clippy::all,
    clippy::pedantic,
    rust_2018_idioms,
    missing_docs,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]
#![doc = r"Geometry primitives for Echo.

This crate provides:
- Axis-aligned bounding boxes (`Aabb`).
- Rigid transforms (`Transform`).
- Temporal utilities (`Tick`, `TemporalTransform`, `TemporalProxy`).
- A minimal broad-phase trait and an AABB-based pairing structure.

Design notes:
- Deterministic: no ambient RNG; ordering of pair outputs is canonical.
- Float32 throughout; operations favor clarity and reproducibility.
- Rustdoc is treated as part of the contract; public items are documented.
"]

/// Time-aware utilities for broad-phase and motion.
pub mod temporal;
/// Foundational geometric types.
pub mod types;
// Broad-phase will land in a follow-up PR.
// pub mod broad;

pub use types::aabb::Aabb;
pub use types::transform::Transform;
