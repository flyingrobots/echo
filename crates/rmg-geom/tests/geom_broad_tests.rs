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
