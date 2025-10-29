use crate::types::{aabb::Aabb, transform::Transform};

/// Transform at two adjacent ticks used to bound motion in the broad-phase.
///
/// - `start` corresponds to tick `n`.
/// - `end` corresponds to tick `n+1`.
///
/// Determinism and plan:
/// - `fat_aabb` below currently uses a union of the start/end AABBs. This is
///   conservative for linear motion and sufficient for pairing/CCD triggers.
/// - Future: introduce a quantized margin policy (based on velocity, `dt`, and
///   shape scale) so that fat proxies are identical across peers/branches. The
///   policy and quantization will be recorded in the graph/spec.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Timespan {
    start: Transform,
    end: Transform,
}

impl Timespan {
    /// Creates a new `Timespan` from start and end transforms.
    #[must_use]
    pub const fn new(start: Transform, end: Transform) -> Self {
        Self { start, end }
    }

    /// Returns the start transform.
    #[must_use]
    pub const fn start(&self) -> Transform {
        self.start
    }

    /// Returns the end transform.
    #[must_use]
    pub const fn end(&self) -> Transform {
        self.end
    }

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
