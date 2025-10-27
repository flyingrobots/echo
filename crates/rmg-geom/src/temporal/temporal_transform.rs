use crate::types::{aabb::Aabb, transform::Transform};

/// Transform at two adjacent ticks used to bound motion in the broad-phase.
///
/// - `start` corresponds to tick `n`.
/// - `end` corresponds to tick `n+1`.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct TemporalTransform {
    start: Transform,
    end: Transform,
}

impl TemporalTransform {
    /// Creates a new `TemporalTransform` from start and end transforms.
    #[must_use]
    pub const fn new(start: Transform, end: Transform) -> Self { Self { start, end } }

    /// Returns the start transform.
    #[must_use]
    pub const fn start(&self) -> Transform { self.start }

    /// Returns the end transform.
    #[must_use]
    pub const fn end(&self) -> Transform { self.end }

    /// Computes a conservative fat AABB for a collider with local-space `shape` AABB.
    ///
    /// The fat box is defined as the union of the shape’s AABBs at the start and
    /// end transforms. This is conservative for linear motion and suffices for
    /// broad-phase pairing and CCD triggering.
    #[must_use]
    pub fn fat_aabb(&self, shape: &Aabb) -> Aabb {
        let a0 = shape.transformed(&self.start.to_mat4());
        let a1 = shape.transformed(&self.end.to_mat4());
        a0.union(&a1)
    }
}

