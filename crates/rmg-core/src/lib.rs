//! rmg-core: typed deterministic graph rewriting engine.

use std::collections::{BTreeMap, HashMap};

use blake3::Hasher;
use bytes::Bytes;
use thiserror::Error;

const POSITION_VELOCITY_BYTES: usize = 24;

pub type Hash = [u8; 32];

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TypeId(pub Hash);

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct NodeId(pub Hash);

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct EdgeId(pub Hash);

#[derive(Clone, Debug)]
pub struct NodeRecord {
    pub id: NodeId,
    pub ty: TypeId,
    pub payload: Option<Bytes>,
}

#[derive(Clone, Debug)]
pub struct EdgeRecord {
    pub id: EdgeId,
    pub from: NodeId,
    pub to: NodeId,
    pub ty: TypeId,
    pub payload: Option<Bytes>,
}

#[derive(Default)]
pub struct GraphStore {
    pub nodes: BTreeMap<NodeId, NodeRecord>,
    pub edges_from: BTreeMap<NodeId, Vec<EdgeRecord>>,
}

impl GraphStore {
    pub fn node(&self, id: &NodeId) -> Option<&NodeRecord> {
        self.nodes.get(id)
    }

    pub fn edges_from(&self, id: &NodeId) -> impl Iterator<Item = &EdgeRecord> {
        self.edges_from.get(id).into_iter().flatten()
    }

    pub fn node_mut(&mut self, id: &NodeId) -> Option<&mut NodeRecord> {
        self.nodes.get_mut(id)
    }

    pub fn insert_node(&mut self, record: NodeRecord) {
        self.nodes.insert(record.id.clone(), record);
    }
}

#[derive(Debug, Clone)]
pub struct PatternGraph {
    pub nodes: Vec<TypeId>,
}

pub type MatchFn = fn(&GraphStore, &NodeId) -> bool;
pub type ExecuteFn = fn(&mut GraphStore, &NodeId);

pub struct RewriteRule {
    pub id: Hash,
    pub name: &'static str,
    pub left: PatternGraph,
    pub matcher: MatchFn,
    pub executor: ExecuteFn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TxId(pub u64);

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub root: NodeId,
    pub hash: Hash,
    pub parent: Option<Hash>,
    pub tx: TxId,
}

#[derive(Debug, Default)]
pub struct DeterministicScheduler {
    pending: BTreeMap<(Hash, Hash), PendingRewrite>,
}

#[derive(Debug, Clone)]
pub struct PendingRewrite {
    pub tx: TxId,
    pub rule_id: Hash,
    pub scope: NodeId,
}

#[derive(Debug)]
pub enum ApplyResult {
    Applied,
    NoMatch,
}

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("transaction not found")]
    UnknownTx,
}

pub struct Engine {
    store: GraphStore,
    rules: HashMap<&'static str, RewriteRule>,
    scheduler: DeterministicScheduler,
    tx_counter: u64,
    current_root: NodeId,
    last_snapshot: Option<Snapshot>,
}

impl Engine {
    pub fn new(store: GraphStore, root: NodeId) -> Self {
        Self {
            store,
            rules: HashMap::new(),
            scheduler: DeterministicScheduler::default(),
            tx_counter: 0,
            current_root: root,
            last_snapshot: None,
        }
    }

    pub fn register_rule(&mut self, rule: RewriteRule) {
        self.rules.insert(rule.name, rule);
    }

    pub fn begin(&mut self) -> TxId {
        self.tx_counter += 1;
        TxId(self.tx_counter)
    }

    pub fn apply(
        &mut self,
        tx: TxId,
        rule_name: &str,
        scope: &NodeId,
    ) -> Result<ApplyResult, EngineError> {
        if tx.0 == 0 || tx.0 > self.tx_counter {
            return Err(EngineError::UnknownTx);
        }
        let rule = match self.rules.get(rule_name) {
            Some(rule) => rule,
            None => return Ok(ApplyResult::NoMatch),
        };
        if !(rule.matcher)(&self.store, scope) {
            return Ok(ApplyResult::NoMatch);
        }

        let scope_hash = scope_hash(rule, scope);
        self.scheduler.pending.insert(
            (scope_hash, rule.id),
            PendingRewrite {
                tx,
                rule_id: rule.id,
                scope: scope.clone(),
            },
        );

        Ok(ApplyResult::Applied)
    }

    pub fn commit(&mut self, tx: TxId) -> Result<Snapshot, EngineError> {
        if tx.0 == 0 || tx.0 > self.tx_counter {
            return Err(EngineError::UnknownTx);
        }
        let pending = self.scheduler.drain_for_tx(tx);
        for rewrite in pending {
            if let Some(rule) = self.rule_by_id(&rewrite.rule_id) {
                (rule.executor)(&mut self.store, &rewrite.scope);
            }
        }

        let hash = compute_snapshot_hash(&self.store, &self.current_root);
        let snapshot = Snapshot {
            root: self.current_root.clone(),
            hash,
            parent: self.last_snapshot.as_ref().map(|s| s.hash),
            tx,
        };
        self.last_snapshot = Some(snapshot.clone());
        Ok(snapshot)
    }

    pub fn snapshot(&self) -> Snapshot {
        let hash = compute_snapshot_hash(&self.store, &self.current_root);
        Snapshot {
            root: self.current_root.clone(),
            hash,
            parent: self.last_snapshot.as_ref().map(|s| s.hash),
            tx: TxId(self.tx_counter),
        }
    }

    pub fn node(&self, id: &NodeId) -> Option<&NodeRecord> {
        self.store.node(id)
    }
}

impl Engine {
    fn rule_by_id(&self, id: &Hash) -> Option<&RewriteRule> {
        self.rules.values().find(|rule| &rule.id == id)
    }
}

fn scope_hash(rule: &RewriteRule, scope: &NodeId) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(&rule.id);
    hasher.update(&scope.0);
    hasher.finalize().into()
}

fn compute_snapshot_hash(store: &GraphStore, root: &NodeId) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(&root.0);
    for (node_id, node) in &store.nodes {
        hasher.update(&node_id.0);
        hasher.update(&(node.ty).0);
        match &node.payload {
            Some(payload) => {
                hasher.update(&(payload.len() as u64).to_le_bytes());
                hasher.update(payload);
            }
            None => {
                hasher.update(&0u64.to_le_bytes());
            }
        }
    }
    for (from, edges) in &store.edges_from {
        hasher.update(&from.0);
        hasher.update(&(edges.len() as u64).to_le_bytes());
        for edge in edges {
            hasher.update(&(edge.id).0);
            hasher.update(&(edge.ty).0);
            hasher.update(&(edge.to).0);
            match &edge.payload {
                Some(payload) => {
                    hasher.update(&(payload.len() as u64).to_le_bytes());
                    hasher.update(payload);
                }
                None => {
                    hasher.update(&0u64.to_le_bytes());
                }
            }
        }
    }
    hasher.update(&root.0);
    hasher.finalize().into()
}

impl DeterministicScheduler {
    fn drain_for_tx(&mut self, tx: TxId) -> Vec<PendingRewrite> {
        let mut ready = Vec::new();
        let pending = std::mem::take(&mut self.pending);
        for (key, rewrite) in pending {
            if rewrite.tx == tx {
                ready.push(rewrite);
            } else {
                self.pending.insert(key, rewrite);
            }
        }
        ready
    }
}

fn encode_position_velocity(position: [f32; 3], velocity: [f32; 3]) -> Bytes {
    let mut buf = Vec::with_capacity(POSITION_VELOCITY_BYTES);
    for value in position.into_iter().chain(velocity.into_iter()) {
        buf.extend_from_slice(&value.to_le_bytes());
    }
    Bytes::from(buf)
}

fn decode_position_velocity(bytes: &Bytes) -> Option<([f32; 3], [f32; 3])> {
    if bytes.len() != POSITION_VELOCITY_BYTES {
        return None;
    }
    let mut floats = [0f32; 6];
    for (index, chunk) in bytes.chunks_exact(4).enumerate() {
        floats[index] = f32::from_le_bytes(chunk.try_into().ok()?);
    }
    let position = [floats[0], floats[1], floats[2]];
    let velocity = [floats[3], floats[4], floats[5]];
    Some((position, velocity))
}

pub fn make_type_id(label: &str) -> TypeId {
    TypeId(hash_label(label))
}

pub fn make_node_id(label: &str) -> NodeId {
    NodeId(hash_label(label))
}

fn hash_label(label: &str) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(label.as_bytes());
    hasher.finalize().into()
}

fn add_vec(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn motion_executor(store: &mut GraphStore, scope: &NodeId) {
    if let Some(record) = store.node_mut(scope) {
        if let Some(payload) = &record.payload {
            if let Some((position, velocity)) = decode_position_velocity(payload) {
                let updated = encode_position_velocity(add_vec(position, velocity), velocity);
                record.payload = Some(updated);
            }
        }
    }
}

fn motion_matcher(store: &GraphStore, scope: &NodeId) -> bool {
    store
        .node(scope)
        .and_then(|record| record.payload.as_ref())
        .and_then(decode_position_velocity)
        .is_some()
}

pub fn motion_rule() -> RewriteRule {
    let mut hasher = Hasher::new();
    hasher.update(b"motion/update");
    let id = hasher.finalize().into();
    RewriteRule {
        id,
        name: "motion/update",
        left: PatternGraph { nodes: vec![] },
        matcher: motion_matcher,
        executor: motion_executor,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motion_rule_updates_position_deterministically() {
        let entity = make_node_id("entity-1");
        let entity_type = make_type_id("entity");
        let payload = encode_position_velocity([1.0, 2.0, 3.0], [0.5, -1.0, 0.25]);

        let mut store = GraphStore::default();
        store.insert_node(NodeRecord {
            id: entity.clone(),
            ty: entity_type,
            payload: Some(payload),
        });

        let mut engine = Engine::new(store, entity.clone());
        engine.register_rule(motion_rule());

        let tx = engine.begin();
        let apply = engine.apply(tx, "motion/update", &entity).unwrap();
        assert!(matches!(apply, ApplyResult::Applied));

        let snap = engine.commit(tx).expect("commit");
        let hash_after_first_apply = snap.hash;

        // Run a second engine with identical initial state and ensure hashes match.
        let mut store_b = GraphStore::default();
        let payload_b = encode_position_velocity([1.0, 2.0, 3.0], [0.5, -1.0, 0.25]);
        store_b.insert_node(NodeRecord {
            id: entity.clone(),
            ty: entity_type,
            payload: Some(payload_b),
        });

        let mut engine_b = Engine::new(store_b, entity.clone());
        engine_b.register_rule(motion_rule());
        let tx_b = engine_b.begin();
        let apply_b = engine_b.apply(tx_b, "motion/update", &entity).unwrap();
        assert!(matches!(apply_b, ApplyResult::Applied));
        let snap_b = engine_b.commit(tx_b).expect("commit B");

        assert_eq!(hash_after_first_apply, snap_b.hash);

        // Ensure the position actually moved.
        let node = engine
            .node(&entity)
            .expect("entity exists")
            .payload
            .as_ref()
            .and_then(decode_position_velocity)
            .expect("payload decode");
        assert_eq!(node.0, [1.5, 1.0, 3.25]);
    }

    #[test]
    fn motion_rule_no_match_on_missing_payload() {
        let entity = make_node_id("entity-2");
        let entity_type = make_type_id("entity");

        let mut store = GraphStore::default();
        store.insert_node(NodeRecord {
            id: entity.clone(),
            ty: entity_type,
            payload: None,
        });

        let mut engine = Engine::new(store, entity.clone());
        engine.register_rule(motion_rule());

        let tx = engine.begin();
        let apply = engine.apply(tx, "motion/update", &entity).unwrap();
        assert!(matches!(apply, ApplyResult::NoMatch));
    }
}
