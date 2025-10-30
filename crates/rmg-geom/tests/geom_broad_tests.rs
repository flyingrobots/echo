#![allow(missing_docs)]
//! Integration tests for rmg-geom broad-phase (AABB tree).

use rmg_core::math::{Quat, Vec3};
use rmg_geom::broad::aabb_tree::{AabbTree, BroadPhase};
use rmg_geom::temporal::timespan::Timespan;
use rmg_geom::types::{aabb::Aabb, transform::Transform};

#[test]
fn fat_aabb_covers_start_and_end_poses() {
    // Local shape: unit cube centered at origin with half-extents 1
    let local = Aabb::from_center_half_extents(Vec3::ZERO, 1.0, 1.0, 1.0);
    // Start at origin; end translated +10 on X
    let t0 = Transform::new(Vec3::ZERO, Quat::identity(), Vec3::new(1.0, 1.0, 1.0));
    let t1 = Transform::new(
        Vec3::new(10.0, 0.0, 0.0),
        Quat::identity(),
        Vec3::new(1.0, 1.0, 1.0),
    );
    let tt = Timespan::new(t0, t1);
    let fat = tt.fat_aabb(&local);
    assert_eq!(fat.min().to_array(), [-1.0, -1.0, -1.0]);
    assert_eq!(fat.max().to_array(), [11.0, 1.0, 1.0]);
}

#[test]
fn broad_phase_pair_order_is_deterministic() {
    let mut bp = AabbTree::new();
    // Two overlapping boxes and one far-away
    let a = Aabb::from_center_half_extents(Vec3::new(0.0, 0.0, 0.0), 1.0, 1.0, 1.0); // id 0
    let b = Aabb::from_center_half_extents(Vec3::new(1.0, 0.0, 0.0), 1.0, 1.0, 1.0); // id 1, overlaps with 0
    let c = Aabb::from_center_half_extents(Vec3::new(100.0, 0.0, 0.0), 1.0, 1.0, 1.0); // id 2

    // Insert out of order to test determinism
    bp.upsert(2, c);
    bp.upsert(1, b);
    bp.upsert(0, a);

    let pairs = bp.pairs();
    assert_eq!(pairs, vec![(0, 1)]);

    // Add another overlapping box to create multiple pairs
    let d = Aabb::from_center_half_extents(Vec3::new(0.5, 0.0, 0.0), 1.0, 1.0, 1.0); // id 3
    bp.upsert(3, d);
    let pairs = bp.pairs();
    // Expected canonical order: (0,1), (0,3), (1,3)
    assert_eq!(pairs, vec![(0, 1), (0, 3), (1, 3)]);
}

#[test]
fn fat_aabb_covers_mid_rotation_with_offset() {
    use core::f32::consts::FRAC_PI_2;
    // Local shape: rod from x=0..2 (center at (1,0,0)) with small thickness
    let local =
        Aabb::from_center_half_extents(rmg_core::math::Vec3::new(1.0, 0.0, 0.0), 1.0, 0.1, 0.1);

    let t0 = Transform::new(
        rmg_core::math::Vec3::new(0.0, 0.0, 0.0),
        rmg_core::math::Quat::identity(),
        rmg_core::math::Vec3::new(1.0, 1.0, 1.0),
    );
    let t1 = Transform::new(
        rmg_core::math::Vec3::new(0.0, 0.0, 0.0),
        rmg_core::math::Quat::from_axis_angle(rmg_core::math::Vec3::new(0.0, 0.0, 1.0), FRAC_PI_2),
        rmg_core::math::Vec3::new(1.0, 1.0, 1.0),
    );
    let span = Timespan::new(t0, t1);

    // Compute mid pose explicitly (45°); this can protrude beyond both endpoints.
    let mid_rot = rmg_core::math::Quat::from_axis_angle(
        rmg_core::math::Vec3::new(0.0, 0.0, 1.0),
        FRAC_PI_2 * 0.5,
    );
    let mid = Transform::new(
        rmg_core::math::Vec3::new(0.0, 0.0, 0.0),
        mid_rot,
        rmg_core::math::Vec3::new(1.0, 1.0, 1.0),
    );
    let mid_aabb = local.transformed(&mid.to_mat4());

    let fat = span.fat_aabb(&local);
    let fmin = fat.min().to_array();
    let fmax = fat.max().to_array();
    let mmin = mid_aabb.min().to_array();
    let mmax = mid_aabb.max().to_array();

    assert!(
        fmin[0] <= mmin[0] && fmin[1] <= mmin[1] && fmin[2] <= mmin[2],
        "fat min must enclose mid min: fat={fmin:?} mid={mmin:?}"
    );
    assert!(
        fmax[0] >= mmax[0] && fmax[1] >= mmax[1] && fmax[2] >= mmax[2],
        "fat max must enclose mid max: fat={fmax:?} mid={mmax:?}"
    );
}
