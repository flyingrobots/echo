// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

#![allow(missing_docs)]
use warp_core::{make_node_id, make_warp_id, Footprint, NodeId, PortKey, WarpId};

/// Test warp ID used for footprint independence tests.
fn test_warp_id() -> WarpId {
    make_warp_id("test")
}

fn pack_port(node: &NodeId, port_id: u32, dir_in: bool) -> PortKey {
    // Test-only packer: use the leading 8 bytes of NodeId for a stable key.
    let mut node_hi = [0u8; 8];
    node_hi.copy_from_slice(&node.0[0..8]);
    let node_bits = u64::from_le_bytes(node_hi);
    let dir_bit = if dir_in { 1u64 } else { 0u64 };
    (node_bits << 32) | ((port_id as u64) << 2) | dir_bit
}

#[test]
fn disjoint_factors_are_independent() {
    let a = Footprint {
        factor_mask: 0b0001,
        ..Default::default()
    };
    let b = Footprint {
        factor_mask: 0b0010,
        ..Default::default()
    };
    assert!(a.independent(&b));
}

#[test]
fn overlapping_node_writes_conflict() {
    let warp = test_warp_id();
    let mut a = Footprint {
        factor_mask: 0b0001,
        ..Default::default()
    };
    let n = make_node_id("n");
    a.n_write.insert_with_warp(warp, n);

    let mut b = Footprint {
        factor_mask: 0b0001,
        ..Default::default()
    };
    b.n_write.insert_with_warp(warp, n);

    assert!(!a.independent(&b));
}

#[test]
fn write_read_conflict() {
    let warp = test_warp_id();
    let mut a = Footprint {
        factor_mask: 0b0001,
        ..Default::default()
    };
    let n = make_node_id("n");
    a.n_write.insert_with_warp(warp, n);

    let mut b = Footprint {
        factor_mask: 0b0001,
        ..Default::default()
    };
    b.n_read.insert_with_warp(warp, n);

    assert!(!a.independent(&b));
}

#[test]
fn independent_nodes_no_conflict() {
    let warp = test_warp_id();
    let mut a = Footprint {
        factor_mask: 0b0001,
        ..Default::default()
    };
    a.n_write.insert_with_warp(warp, make_node_id("a"));

    let mut b = Footprint {
        factor_mask: 0b0001,
        ..Default::default()
    };
    b.n_write.insert_with_warp(warp, make_node_id("b"));

    assert!(a.independent(&b));
}

#[test]
fn port_conflict_detected() {
    let warp = test_warp_id();
    let node = make_node_id("p");
    let mut a = Footprint {
        factor_mask: 0b0001,
        ..Default::default()
    };
    a.b_in.insert(warp, pack_port(&node, 0, true));

    let mut b = Footprint {
        factor_mask: 0b0001,
        ..Default::default()
    };
    b.b_in.insert(warp, pack_port(&node, 0, true));

    assert!(!a.independent(&b));
}
