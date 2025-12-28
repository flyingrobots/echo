// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
use warp_core::{Engine, GraphStore, NodeId};

pub fn snapshot_hash_of(store: GraphStore, root: NodeId) -> [u8; 32] {
    let engine = Engine::new(store, root);
    engine.snapshot().hash
}
