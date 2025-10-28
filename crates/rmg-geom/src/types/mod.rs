//! Core geometry types used by the engine (transform, AABB).
//!
//! Determinism notes:
//! - Overlap semantics are inclusive on faces to avoid pair churn on contact
//!   boundaries.
//! - Affine math uses `f32` without fused multiply-add to preserve identical
//!   results across platforms.
//! - Temporal fattening/quantization policy will be documented at the
//!   `temporal` layer so that identical inputs yield identical proxy bounds.

#[doc = "Axis-aligned bounding boxes (world space)."]
pub mod aabb;
#[doc = "Rigid transforms with non-uniform scale."]
pub mod transform;
