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
    /// Policy (deterministic): unions the AABBs at three sample poses — start (t=0),
    /// midpoint (t=0.5), and end (t=1). This strictly contains pure translations
    /// and captures protrusions that can occur at `t≈0.5` during rotations about
    /// an off‑centre pivot, which a start/end‑only union can miss.
    ///
    /// Sampling count is fixed (3) for determinism; future work may make the
    /// sampling policy configurable while keeping results identical across peers.
    #[must_use]
    pub fn fat_aabb(&self, shape: &Aabb) -> Aabb {
        let a0 = shape.transformed(&self.start.to_mat4());
        let a1 = shape.transformed(&self.end.to_mat4());

        // Midpoint transform via linear interp of translation/scale and
        // normalized-linear blend of rotation (nlerp), then convert to Mat4.
        let t0 = self.start.translation().to_array();
        let t1 = self.end.translation().to_array();
        let tm = rmg_core::math::Vec3::new(
            0.5 * (t0[0] + t1[0]),
            0.5 * (t0[1] + t1[1]),
            0.5 * (t0[2] + t1[2]),
        );

        let s0 = self.start.scale().to_array();
        let s1 = self.end.scale().to_array();
        let sm = rmg_core::math::Vec3::new(
            0.5 * (s0[0] + s1[0]),
            0.5 * (s0[1] + s1[1]),
            0.5 * (s0[2] + s1[2]),
        );

        let q0 = self.start.rotation().to_array();
        let q1 = self.end.rotation().to_array();
        let qm = rmg_core::math::Quat::new(
            0.5 * (q0[0] + q1[0]),
            0.5 * (q0[1] + q1[1]),
            0.5 * (q0[2] + q1[2]),
            0.5 * (q0[3] + q1[3]),
        )
        .normalize();

        let mid_tf = crate::types::transform::Transform::new(tm, qm, sm);
        let am = shape.transformed(&mid_tf.to_mat4());

        a0.union(&a1).union(&am)
    }
}
