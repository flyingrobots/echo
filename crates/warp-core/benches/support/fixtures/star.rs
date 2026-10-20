// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Control C1: the star scene `benches/support/action_scene.rs` builds.
//!
//! A root node and `cells` cell nodes, each with a 7-byte attachment and an
//! edge from the root. This is built independently of `action_scene`; a test
//! checks that the two produce the same state root, so results measured on
//! `action_scene` stay comparable with the fixture suite.

use bytes::Bytes;
use sha2::{Digest, Sha256};
use warp_core::{
    make_edge_id, make_node_id, make_type_id, make_warp_id, AtomPayload, AttachmentValue,
    EdgeRecord, GraphStore, NodeId, NodeRecord, TypeId, WorldlineState,
};

use super::manifest::{Construction, EvidenceClass, Expectation, WorkloadSpec};

/// Bumped whenever this generator's logic changes.
pub const FIXTURE_VERSION: u32 = 1;

/// The value every cell holds, as in `action_scene`.
const INITIAL: &[u8] = b"initial";

fn profile(label: &str) -> [u8; 32] {
    let mut hash = blake3::Hasher::new();
    hash.update(b"echo:operation-profile:v1\0");
    hash.update(&(label.len() as u64).to_le_bytes());
    hash.update(label.as_bytes());
    hash.finalize().into()
}

/// The C1 workload for `cells` cells.
pub fn workload(cells: u64) -> WorkloadSpec {
    WorkloadSpec {
        fixture: "C1",
        fixture_version: FIXTURE_VERSION,
        seed: 0,
        construction: Construction::Direct,
        evidence: EvidenceClass::Exploratory,
        dimensions: vec![("cells", cells), ("warps", 1)],
        settings: vec![("reachability", "linked")],
        history_operations: vec![],
    }
}

/// Builds the C1 store; returns it with its root node.
pub fn build(cells: u64) -> (GraphStore, NodeId) {
    let warp_id = make_warp_id("edict-cas-public-witness");
    let root = make_node_id("edict-cas-public-witness-root");
    let mut store = GraphStore::new(warp_id);
    store.insert_node(
        root,
        NodeRecord {
            ty: make_type_id("cas-witness-root"),
        },
    );
    let attachment_type = TypeId(profile("cas.echo.attachment.greeting-message/v1"));
    let node_type = TypeId(profile("cas.echo.node.greeting/v1"));
    let edge_type = make_type_id("cas-witness-cell");
    for cell in 0..cells {
        let key = format!("cell-{cell:06}");
        let node = NodeId(Sha256::digest(key.as_bytes()).into());
        store.insert_node(node, NodeRecord { ty: node_type });
        store.set_node_attachment(
            node,
            Some(AttachmentValue::Atom(AtomPayload::new(
                attachment_type,
                Bytes::from_static(INITIAL),
            ))),
        );
        store.insert_edge(
            root,
            EdgeRecord {
                id: make_edge_id(&key),
                from: root,
                to: node,
                ty: edge_type,
            },
        );
    }
    (store, root)
}

/// The C1 state root for `cells` cells.
pub fn state_root(cells: u64) -> [u8; 32] {
    let (store, root) = build(cells);
    WorldlineState::from_root_store(store, root)
        .expect("C1 root state")
        .state_root()
}

/// Declared outcomes for C1 at `cells` cells, with an optional pinned root.
pub fn expectation(cells: u64, pinned_root: Option<&str>) -> Expectation {
    let mut assertions = vec![
        ("reachable_nodes", (cells + 1).to_string()),
        ("reachable_edges", cells.to_string()),
        ("matches_action_scene_state_root", "true".to_owned()),
    ];
    if let Some(root) = pinned_root {
        assertions.push(("state_root_v1", root.to_owned()));
    }
    Expectation {
        assertions,
        output_formats: vec![("state_root", 1)],
    }
}
