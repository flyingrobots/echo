#![allow(missing_docs)]
use rmg_core::{make_node_id, Footprint, NodeId, PortKey};

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
    let mut a = Footprint {
        factor_mask: 0b0001,
        ..Default::default()
    };
    let n = make_node_id("n");
    a.n_write.insert_node(&n);

    let mut b = Footprint {
        factor_mask: 0b0001,
        ..Default::default()
    };
    b.n_write.insert_node(&n);

    assert!(!a.independent(&b));
}

#[test]
fn write_read_conflict() {
    let mut a = Footprint {
        factor_mask: 0b0001,
        ..Default::default()
    };
    let n = make_node_id("n");
    a.n_write.insert_node(&n);

    let mut b = Footprint {
        factor_mask: 0b0001,
        ..Default::default()
    };
    b.n_read.insert_node(&n);

    assert!(!a.independent(&b));
}

#[test]
fn independent_nodes_no_conflict() {
    let mut a = Footprint {
        factor_mask: 0b0001,
        ..Default::default()
    };
    a.n_write.insert_node(&make_node_id("a"));

    let mut b = Footprint {
        factor_mask: 0b0001,
        ..Default::default()
    };
    b.n_write.insert_node(&make_node_id("b"));

    assert!(a.independent(&b));
}

#[test]
fn port_conflict_detected() {
    let node = make_node_id("p");
    let mut a = Footprint {
        factor_mask: 0b0001,
        ..Default::default()
    };
    a.b_in.insert(pack_port(&node, 0, true));

    let mut b = Footprint {
        factor_mask: 0b0001,
        ..Default::default()
    };
    b.b_in.insert(pack_port(&node, 0, true));

    assert!(!a.independent(&b));
}
