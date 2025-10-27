use rmg_core::math::{Mat4, Vec3};

/// Axis-aligned bounding box in world coordinates.
///
/// Invariants:
/// - `min` components are less than or equal to `max` components.
/// - Values are `f32` and represent meters in world space.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Aabb {
    min: Vec3,
    max: Vec3,
}

impl Aabb {
    /// Constructs an AABB from its minimum and maximum corners.
    ///
    /// # Panics
    /// Panics if any component of `min` is greater than its counterpart in `max`.
    #[must_use]
    pub fn new(min: Vec3, max: Vec3) -> Self {
        let a = min.to_array();
        let b = max.to_array();
        assert!(a[0] <= b[0] && a[1] <= b[1] && a[2] <= b[2], "invalid AABB: min > max");
        Self { min, max }
    }

    /// Returns the minimum corner.
    #[must_use]
    pub fn min(&self) -> Vec3 { self.min }

    /// Returns the maximum corner.
    #[must_use]
    pub fn max(&self) -> Vec3 { self.max }

    /// Builds an AABB centered at `center` with half-extents `hx, hy, hz`.
    #[must_use]
    pub fn from_center_half_extents(center: Vec3, hx: f32, hy: f32, hz: f32) -> Self {
        let he = Vec3::new(hx, hy, hz);
        Self::new(center.sub(&he), center.add(&he))
    }

    /// Returns `true` if this AABB overlaps another (inclusive on faces).
    #[must_use]
    pub fn overlaps(&self, other: &Self) -> bool {
        let a_min = self.min.to_array();
        let a_max = self.max.to_array();
        let b_min = other.min.to_array();
        let b_max = other.max.to_array();
        // Inclusive to treat touching faces as overlap for broad-phase pairing.
        !(a_max[0] < b_min[0]
            || a_min[0] > b_max[0]
            || a_max[1] < b_min[1]
            || a_min[1] > b_max[1]
            || a_max[2] < b_min[2]
            || a_min[2] > b_max[2])
    }

    /// Returns the union of two AABBs.
    #[must_use]
    pub fn union(&self, other: &Self) -> Self {
        let a = self.min.to_array();
        let b = self.max.to_array();
        let c = other.min.to_array();
        let d = other.max.to_array();
        Self {
            min: Vec3::new(a[0].min(c[0]), a[1].min(c[1]), a[2].min(c[2])),
            max: Vec3::new(b[0].max(d[0]), b[1].max(d[1]), b[2].max(d[2])),
        }
    }

    /// Inflates the box by a uniform margin `m` in all directions.
    #[must_use]
    pub fn inflate(&self, m: f32) -> Self {
        let delta = Vec3::new(m, m, m);
        Self { min: self.min.sub(&delta), max: self.max.add(&delta) }
    }

    /// Computes the AABB that bounds this box after transformation by `mat`.
    ///
    /// This evaluates the eight corners under the affine transform and builds a
    /// new axis-aligned box containing them.
    #[must_use]
    pub fn transformed(&self, mat: &Mat4) -> Self {
        let [minx, miny, minz] = self.min.to_array();
        let [maxx, maxy, maxz] = self.max.to_array();
        let corners = [
            Vec3::new(minx, miny, minz),
            Vec3::new(minx, miny, maxz),
            Vec3::new(minx, maxy, minz),
            Vec3::new(minx, maxy, maxz),
            Vec3::new(maxx, miny, minz),
            Vec3::new(maxx, miny, maxz),
            Vec3::new(maxx, maxy, minz),
            Vec3::new(maxx, maxy, maxz),
        ];
        // Compute bounds without allocating an intermediate Vec to avoid needless collects.
        let mut min = mat.transform_point(&corners[0]);
        let mut max = min;
        for c in &corners[1..] {
            let p = mat.transform_point(c);
            let pa = p.to_array();
            let mi = min.to_array();
            let ma = max.to_array();
            min = Vec3::new(mi[0].min(pa[0]), mi[1].min(pa[1]), mi[2].min(pa[2]));
            max = Vec3::new(ma[0].max(pa[0]), ma[1].max(pa[1]), ma[2].max(pa[2]));
        }
        Self { min, max }
    }

    /// Builds the minimal AABB that contains all `points`.
    ///
    /// # Panics
    /// Panics if `points` is empty.
    #[must_use]
    pub fn from_points(points: &[Vec3]) -> Self {
        assert!(!points.is_empty(), "from_points requires at least one point");
        let mut min = points[0];
        let mut max = points[0];
        for p in &points[1..] {
            let a = p.to_array();
            let mi = min.to_array();
            let ma = max.to_array();
            min = Vec3::new(mi[0].min(a[0]), mi[1].min(a[1]), mi[2].min(a[2]));
            max = Vec3::new(ma[0].max(a[0]), ma[1].max(a[1]), ma[2].max(a[2]));
        }
        Self { min, max }
    }
}
