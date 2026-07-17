// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Contract-host footprint soundness tests.

use warp_core::{
    make_node_id, make_warp_id, runtime_ingress_eint_read_footprint, AttachmentKey,
    AttachmentValue, Footprint, GraphStore, GraphView, NodeId, NodeKey,
};

fn assert_ingress_scope_reads(footprint: &Footprint, warp_id: warp_core::WarpId, scope: NodeId) {
    let node = NodeKey {
        warp_id,
        local_id: scope,
    };
    assert_eq!(
        footprint.n_read.iter().copied().collect::<Vec<_>>(),
        vec![node]
    );
    assert_eq!(
        footprint.a_read.iter().copied().collect::<Vec<_>>(),
        vec![AttachmentKey::node_alpha(node)]
    );
}

#[test]
fn runtime_ingress_footprint_declares_absent_scope_reads() {
    let warp_id = make_warp_id("test/contract-host/absent-scope");
    let scope = make_node_id("test/contract-host/absent-scope/event");
    let store = GraphStore::new(warp_id);

    let footprint = runtime_ingress_eint_read_footprint(GraphView::new(&store), &scope);

    assert_ingress_scope_reads(&footprint, warp_id, scope);
}

#[test]
fn runtime_ingress_footprint_declares_orphan_attachment_reads() {
    let warp_id = make_warp_id("test/contract-host/orphan-attachment");
    let scope = make_node_id("test/contract-host/orphan-attachment/event");
    let mut store = GraphStore::new(warp_id);
    store.set_node_attachment(
        scope,
        Some(AttachmentValue::Descend(make_warp_id(
            "test/contract-host/orphan-attachment/target",
        ))),
    );

    let footprint = runtime_ingress_eint_read_footprint(GraphView::new(&store), &scope);

    assert_ingress_scope_reads(&footprint, warp_id, scope);
}
