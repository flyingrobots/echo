// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Warp-scoped footprint tests for BOAW Phase 6.
//!
//! These tests verify that footprints correctly distinguish resources by warp,
//! preventing false conflicts when different warps touch resources with the
//! same local identifier.

use warp_core::{
    make_warp_id, pack_port_key, EdgeId, EdgeSet, Footprint, NodeId, NodeSet, PortSet,
};

// =============================================================================
// T2.1: Multi-warp same local ID no false conflict (nodes)
// =============================================================================

/// T2.1: Two footprints in different warps reading/writing the same local NodeId
/// should be independent (no false conflict).
///
/// This test exercises the warp-scoping mechanism that prevents false positives
/// when rewrites in different warps touch resources with identical local IDs.
#[test]
fn multiwarp_same_local_id_no_false_conflict_nodes() {
    // Two different warps: simulation and time-travel debugging
    let warp_sim = make_warp_id("sim");
    let warp_ttd = make_warp_id("ttd");

    // Shared local node ID (same bytes in both warps)
    let shared_node_id = NodeId(blake3::hash(b"shared-node").into());

    // Footprint A: writes the shared node in W_sim
    let mut fp_a = Footprint {
        factor_mask: 1, // Non-zero to avoid O(1) fast path
        ..Default::default()
    };
    fp_a.n_write.insert_with_warp(warp_sim, shared_node_id);

    // Footprint B: reads the shared node in W_ttd
    let mut fp_b = Footprint {
        factor_mask: 1,
        ..Default::default()
    };
    fp_b.n_read.insert_with_warp(warp_ttd, shared_node_id);

    // These should be INDEPENDENT despite having the same local node ID,
    // because they are in different warps.
    assert!(
        fp_a.independent(&fp_b),
        "T2.1 FAIL: footprints in different warps should be independent \
         even with same local node ID"
    );

    // Verify symmetry
    assert!(
        fp_b.independent(&fp_a),
        "T2.1 FAIL: independence check must be symmetric"
    );
}

// =============================================================================
// T2.2: Multi-warp same local ID no false conflict (ports)
// =============================================================================

/// T2.2: Two footprints in different warps touching the same packed port key
/// should be independent (no false conflict).
///
/// This test verifies that boundary port sets are warp-scoped, preventing
/// false conflicts when different warps use identical port keys.
#[test]
fn multiwarp_same_local_id_no_false_conflict_ports() {
    // Two different warps
    let warp_sim = make_warp_id("sim");
    let warp_ttd = make_warp_id("ttd");

    // Shared port key (same packed value in both warps)
    let node_for_port = NodeId(blake3::hash(b"port-owner-node").into());
    let shared_port_key = pack_port_key(&node_for_port, 0, true); // input port 0

    // Footprint A: touches the port in W_sim
    let mut fp_a = Footprint {
        factor_mask: 1,
        ..Default::default()
    };
    fp_a.b_in.insert(warp_sim, shared_port_key);

    // Footprint B: touches the same port key in W_ttd
    let mut fp_b = Footprint {
        factor_mask: 1,
        ..Default::default()
    };
    fp_b.b_in.insert(warp_ttd, shared_port_key);

    // These should be INDEPENDENT despite having the same packed port key,
    // because they are in different warps.
    assert!(
        fp_a.independent(&fp_b),
        "T2.2 FAIL: footprints in different warps should be independent \
         even with same packed port key"
    );

    // Verify symmetry
    assert!(
        fp_b.independent(&fp_a),
        "T2.2 FAIL: independence check must be symmetric"
    );

    // Also test b_out ports
    let mut fp_c = Footprint {
        factor_mask: 1,
        ..Default::default()
    };
    fp_c.b_out.insert(warp_sim, shared_port_key);

    let mut fp_d = Footprint {
        factor_mask: 1,
        ..Default::default()
    };
    fp_d.b_out.insert(warp_ttd, shared_port_key);

    assert!(
        fp_c.independent(&fp_d),
        "T2.2 FAIL: b_out ports in different warps should be independent"
    );
}

// =============================================================================
// Sanity check: Same warp, same ID DOES conflict (nodes)
// =============================================================================

/// Sanity check: Two footprints in the SAME warp touching the same node
/// should conflict (write-write conflict).
#[test]
fn multiwarp_same_warp_same_id_does_conflict_nodes() {
    let warp_sim = make_warp_id("sim");
    let shared_node_id = NodeId(blake3::hash(b"shared-node").into());

    // Footprint A: writes the node in W_sim
    let mut fp_a = Footprint {
        factor_mask: 1,
        ..Default::default()
    };
    fp_a.n_write.insert_with_warp(warp_sim, shared_node_id);

    // Footprint B: also writes the same node in the SAME warp W_sim
    let mut fp_b = Footprint {
        factor_mask: 1,
        ..Default::default()
    };
    fp_b.n_write.insert_with_warp(warp_sim, shared_node_id);

    // These should CONFLICT (not independent) - write-write on same warp+node
    assert!(
        !fp_a.independent(&fp_b),
        "SANITY FAIL: write-write on same warp+node must conflict"
    );

    // Also test write-read conflict in same warp
    let mut fp_c = Footprint {
        factor_mask: 1,
        ..Default::default()
    };
    fp_c.n_read.insert_with_warp(warp_sim, shared_node_id);

    assert!(
        !fp_a.independent(&fp_c),
        "SANITY FAIL: write-read on same warp+node must conflict"
    );
}

// =============================================================================
// Sanity check: Same warp, same ID DOES conflict (ports)
// =============================================================================

/// Sanity check: Two footprints in the SAME warp touching the same port
/// should conflict.
#[test]
fn multiwarp_same_warp_same_id_does_conflict_ports() {
    let warp_sim = make_warp_id("sim");
    let node_for_port = NodeId(blake3::hash(b"port-owner-node").into());
    let shared_port_key = pack_port_key(&node_for_port, 0, true);

    // Footprint A: touches the port in W_sim
    let mut fp_a = Footprint {
        factor_mask: 1,
        ..Default::default()
    };
    fp_a.b_in.insert(warp_sim, shared_port_key);

    // Footprint B: also touches the same port in the SAME warp W_sim
    let mut fp_b = Footprint {
        factor_mask: 1,
        ..Default::default()
    };
    fp_b.b_in.insert(warp_sim, shared_port_key);

    // These should CONFLICT (not independent)
    assert!(
        !fp_a.independent(&fp_b),
        "SANITY FAIL: same port in same warp must conflict"
    );

    // Test b_in vs b_out cross-conflict in same warp
    let mut fp_c = Footprint {
        factor_mask: 1,
        ..Default::default()
    };
    fp_c.b_out.insert(warp_sim, shared_port_key);

    assert!(
        !fp_a.independent(&fp_c),
        "SANITY FAIL: b_in vs b_out on same port in same warp must conflict"
    );
}

// =============================================================================
// Multi-warp edges are warp-scoped
// =============================================================================

/// Edges should also be warp-scoped: same local EdgeId in different warps
/// should not conflict.
#[test]
fn multiwarp_edges_warp_scoped() {
    let warp_sim = make_warp_id("sim");
    let warp_ttd = make_warp_id("ttd");

    // Shared local edge ID
    let shared_edge_id = EdgeId(blake3::hash(b"shared-edge").into());

    // Footprint A: writes the edge in W_sim
    let mut fp_a = Footprint {
        factor_mask: 1,
        ..Default::default()
    };
    fp_a.e_write.insert_with_warp(warp_sim, shared_edge_id);

    // Footprint B: reads the same edge ID in W_ttd
    let mut fp_b = Footprint {
        factor_mask: 1,
        ..Default::default()
    };
    fp_b.e_read.insert_with_warp(warp_ttd, shared_edge_id);

    // Different warps -> independent
    assert!(
        fp_a.independent(&fp_b),
        "edges in different warps should be independent even with same local ID"
    );

    // Same warp -> conflict
    let mut fp_c = Footprint {
        factor_mask: 1,
        ..Default::default()
    };
    fp_c.e_read.insert_with_warp(warp_sim, shared_edge_id);

    assert!(
        !fp_a.independent(&fp_c),
        "write-read on same edge in same warp must conflict"
    );

    // Write-write in same warp
    let mut fp_d = Footprint {
        factor_mask: 1,
        ..Default::default()
    };
    fp_d.e_write.insert_with_warp(warp_sim, shared_edge_id);

    assert!(
        !fp_a.independent(&fp_d),
        "write-write on same edge in same warp must conflict"
    );
}

// =============================================================================
// Additional edge cases
// =============================================================================

/// Verify that NodeSet, EdgeSet, and PortSet collections themselves
/// correctly implement warp-scoped intersection.
#[test]
fn warp_scoped_sets_intersection_behavior() {
    let warp_a = make_warp_id("warp-a");
    let warp_b = make_warp_id("warp-b");

    // NodeSet
    {
        let node_id = NodeId(blake3::hash(b"test-node").into());
        let mut set_a = NodeSet::default();
        let mut set_b = NodeSet::default();

        set_a.insert_with_warp(warp_a, node_id);
        set_b.insert_with_warp(warp_b, node_id);

        assert!(
            !set_a.intersects(&set_b),
            "NodeSet: different warps should not intersect"
        );

        let mut set_same = NodeSet::default();
        set_same.insert_with_warp(warp_a, node_id);
        assert!(
            set_a.intersects(&set_same),
            "NodeSet: same warp should intersect"
        );
    }

    // EdgeSet
    {
        let edge_id = EdgeId(blake3::hash(b"test-edge").into());
        let mut set_a = EdgeSet::default();
        let mut set_b = EdgeSet::default();

        set_a.insert_with_warp(warp_a, edge_id);
        set_b.insert_with_warp(warp_b, edge_id);

        assert!(
            !set_a.intersects(&set_b),
            "EdgeSet: different warps should not intersect"
        );
    }

    // PortSet
    {
        let node_for_port = NodeId(blake3::hash(b"port-node").into());
        let port_key = pack_port_key(&node_for_port, 42, false);
        let mut set_a = PortSet::default();
        let mut set_b = PortSet::default();

        set_a.insert(warp_a, port_key);
        set_b.insert(warp_b, port_key);

        assert!(
            !set_a.intersects(&set_b),
            "PortSet: different warps should not intersect"
        );
    }
}

/// Test that the factor_mask fast path still works correctly with warp-scoped
/// resources (disjoint masks should short-circuit before checking sets).
#[test]
fn factor_mask_fast_path_with_warp_scoping() {
    let warp_sim = make_warp_id("sim");
    let shared_node_id = NodeId(blake3::hash(b"shared-node").into());

    // Footprint A with factor_mask = 0b0001
    let mut fp_a = Footprint {
        factor_mask: 0b0001,
        ..Default::default()
    };
    fp_a.n_write.insert_with_warp(warp_sim, shared_node_id);

    // Footprint B with DISJOINT factor_mask = 0b0010
    // Even in the same warp with same node, disjoint masks -> independent
    let mut fp_b = Footprint {
        factor_mask: 0b0010,
        ..Default::default()
    };
    fp_b.n_write.insert_with_warp(warp_sim, shared_node_id);

    // Disjoint factor_mask should make them independent (fast path)
    assert!(
        fp_a.independent(&fp_b),
        "disjoint factor_mask should short-circuit to independent"
    );

    // Now with overlapping factor_mask, they should conflict
    let mut fp_c = Footprint {
        factor_mask: 0b0001, // overlaps with fp_a
        ..Default::default()
    };
    fp_c.n_write.insert_with_warp(warp_sim, shared_node_id);

    assert!(
        !fp_a.independent(&fp_c),
        "overlapping factor_mask with same warp+node should conflict"
    );
}

/// Test multiple resources across different warps in a single footprint.
#[test]
fn multiwarp_multiple_resources_in_single_footprint() {
    let warp_sim = make_warp_id("sim");
    let warp_ttd = make_warp_id("ttd");
    let warp_replay = make_warp_id("replay");

    let node_a = NodeId(blake3::hash(b"node-a").into());
    let node_b = NodeId(blake3::hash(b"node-b").into());
    let edge_x = EdgeId(blake3::hash(b"edge-x").into());

    // Footprint that touches resources in multiple warps
    let mut fp_multi = Footprint {
        factor_mask: 1,
        ..Default::default()
    };
    fp_multi.n_write.insert_with_warp(warp_sim, node_a);
    fp_multi.n_read.insert_with_warp(warp_ttd, node_b);
    fp_multi.e_write.insert_with_warp(warp_replay, edge_x);

    // Footprint that only touches W_sim resources
    let mut fp_sim_only = Footprint {
        factor_mask: 1,
        ..Default::default()
    };
    fp_sim_only.n_read.insert_with_warp(warp_sim, node_a);

    // Should conflict on node_a in W_sim (write-read)
    assert!(
        !fp_multi.independent(&fp_sim_only),
        "should conflict on shared warp+node"
    );

    // Footprint that touches different nodes in W_sim
    let mut fp_sim_different = Footprint {
        factor_mask: 1,
        ..Default::default()
    };
    fp_sim_different
        .n_write
        .insert_with_warp(warp_sim, NodeId(blake3::hash(b"other-node").into()));

    // Should be independent (different nodes)
    assert!(
        fp_multi.independent(&fp_sim_different),
        "different nodes in same warp should be independent"
    );
}
