use rmg_core::{Engine, GraphStore, NodeId};

pub fn snapshot_hash_of(store: GraphStore, root: NodeId) -> [u8; 32] {
    let engine = Engine::new(store, root);
    engine.snapshot().hash
}
