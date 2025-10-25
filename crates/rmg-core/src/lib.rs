//! rmg-core: typed deterministic graph rewriting engine.
#![deny(missing_docs)]

use std::collections::{BTreeMap, HashMap};

use blake3::Hasher;
use bytes::Bytes;
use thiserror::Error;

const POSITION_VELOCITY_BYTES: usize = 24;

/// Canonical 256-bit hash used throughout the engine for addressing nodes,
/// types, snapshots, and rewrite rules.
pub type Hash = [u8; 32];

/// Strongly typed identifier for a registered entity or structural node.
///
/// `NodeId` values are obtained from `make_node_id` and remain stable across
/// runs because they are derived from a BLAKE3 hash of a string label.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct NodeId(pub Hash);

/// Strongly typed identifier for the logical kind of a node or component.
///
/// `TypeId` values are produced by `make_type_id` which hashes a label; using
/// a dedicated wrapper prevents accidental mixing of node and type identifiers.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TypeId(pub Hash);

/// Identifier for a directed edge within the graph.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct EdgeId(pub Hash);

/// Materialised record for a single node stored in the graph.
///
/// The optional `payload` carries domain-specific bytes (component data,
/// attachments, etc) and is interpreted by higher layers.
#[derive(Clone, Debug)]
pub struct NodeRecord {
    /// Stable identifier for the node.
    pub id: NodeId,
    /// Type identifier describing the node.
    pub ty: TypeId,
    /// Optional payload owned by the node (component data, attachments, etc.).
    pub payload: Option<Bytes>,
}

/// Materialised record for a single edge stored in the graph.
#[derive(Clone, Debug)]
pub struct EdgeRecord {
    /// Stable identifier for the edge.
    pub id: EdgeId,
    /// Source node identifier.
    pub from: NodeId,
    /// Destination node identifier.
    pub to: NodeId,
    /// Type identifier describing the edge.
    pub ty: TypeId,
    /// Optional payload owned by the edge.
    pub payload: Option<Bytes>,
}

/// Minimal in-memory graph store used by the rewrite executor tests.
///
/// The production engine will eventually swap in a content-addressed store,
/// but this structure keeps the motion rewrite spike self-contained.
#[derive(Default)]
pub struct GraphStore {
    /// Mapping from node identifiers to their materialised records.
    pub nodes: BTreeMap<NodeId, NodeRecord>,
    /// Mapping from source node to outbound edge records.
    pub edges_from: BTreeMap<NodeId, Vec<EdgeRecord>>,
}

impl GraphStore {
    /// Returns a shared reference to a node when it exists.
    pub fn node(&self, id: &NodeId) -> Option<&NodeRecord> {
        self.nodes.get(id)
    }

    /// Returns an iterator over edges that originate from the provided node.
    pub fn edges_from(&self, id: &NodeId) -> impl Iterator<Item = &EdgeRecord> {
        self.edges_from.get(id).into_iter().flatten()
    }

    /// Returns a mutable reference to a node when it exists.
    pub fn node_mut(&mut self, id: &NodeId) -> Option<&mut NodeRecord> {
        self.nodes.get_mut(id)
    }

    /// Inserts or replaces a node in the store.
    pub fn insert_node(&mut self, record: NodeRecord) {
        self.nodes.insert(record.id.clone(), record);
    }
}

/// Pattern metadata used by a rewrite rule to describe the input graph shape.
#[derive(Debug, Clone)]
pub struct PatternGraph {
    /// Ordered list of type identifiers that make up the pattern.
    pub nodes: Vec<TypeId>,
}

/// Function pointer used to determine whether a rule matches the provided scope.
pub type MatchFn = fn(&GraphStore, &NodeId) -> bool;

/// Function pointer that applies a rewrite to the given scope.
pub type ExecuteFn = fn(&mut GraphStore, &NodeId);

/// Descriptor for a rewrite rule registered with the engine.
///
/// Each rule owns:
/// * a deterministic identifier (`id`)
/// * a human-readable name
/// * a left pattern (currently unused by the spike)
/// * callbacks for matching and execution
pub struct RewriteRule {
    /// Deterministic identifier for the rewrite rule.
    pub id: Hash,
    /// Human-readable name for logs and debugging.
    pub name: &'static str,
    /// Pattern used to describe the left-hand side of the rule.
    pub left: PatternGraph,
    /// Callback that determines whether the rule matches a given scope.
    pub matcher: MatchFn,
    /// Callback that applies the rewrite to the given scope.
    pub executor: ExecuteFn,
}

/// Thin wrapper around an auto-incrementing transaction identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TxId(pub u64);

/// Snapshot returned after a successful commit.
///
/// The `hash` value is deterministic and reflects the entire canonicalised
/// graph state (root + payloads).
#[derive(Debug, Clone)]
pub struct Snapshot {
    /// Node identifier that serves as the root of the snapshot.
    pub root: NodeId,
    /// Canonical hash derived from the entire graph state.
    pub hash: Hash,
    /// Optional parent snapshot hash (if one exists).
    pub parent: Option<Hash>,
    /// Transaction identifier associated with the snapshot.
    pub tx: TxId,
}

/// Ordering queue that guarantees rewrites execute deterministically.
#[derive(Debug, Default)]
pub struct DeterministicScheduler {
    pending: BTreeMap<(Hash, Hash), PendingRewrite>,
}

/// Internal representation of a rewrite waiting to be applied.
#[derive(Debug, Clone)]
pub struct PendingRewrite {
    /// Transaction identifier that enqueued the rewrite.
    pub tx: TxId,
    /// Identifier of the rule to execute.
    pub rule_id: Hash,
    /// Scope node supplied when `apply` was invoked.
    pub scope: NodeId,
}

/// Result of calling `Engine::apply`.
#[derive(Debug)]
pub enum ApplyResult {
    /// The rewrite matched and was enqueued for execution.
    Applied,
    /// The rewrite did not match the provided scope.
    NoMatch,
}

/// Errors emitted by the engine.
#[derive(Debug, Error)]
pub enum EngineError {
    /// The supplied transaction identifier did not exist or was already closed.
    #[error("transaction not found")]
    UnknownTx,
}

/// Core rewrite engine used by the spike.
///
/// It owns a `GraphStore`, the registered rules, and the deterministic scheduler.
pub struct Engine {
    store: GraphStore,
    rules: HashMap<&'static str, RewriteRule>,
    scheduler: DeterministicScheduler,
    tx_counter: u64,
    current_root: NodeId,
    last_snapshot: Option<Snapshot>,
}

impl Engine {
    /// Constructs a new engine with the supplied backing store and root node id.
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

    /// Registers a rewrite rule so it can be referenced by name.
    pub fn register_rule(&mut self, rule: RewriteRule) {
        self.rules.insert(rule.name, rule);
    }

    /// Begins a new transaction and returns its identifier.
    pub fn begin(&mut self) -> TxId {
        self.tx_counter += 1;
        TxId(self.tx_counter)
    }

    /// Queues a rewrite for execution if it matches the provided scope.
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

    /// Executes all pending rewrites for the transaction and produces a snapshot.
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

    /// Returns a snapshot for the current graph state without executing rewrites.
    pub fn snapshot(&self) -> Snapshot {
        let hash = compute_snapshot_hash(&self.store, &self.current_root);
        Snapshot {
            root: self.current_root.clone(),
            hash,
            parent: self.last_snapshot.as_ref().map(|s| s.hash),
            tx: TxId(self.tx_counter),
        }
    }

    /// Returns a shared view of a node when it exists.
    pub fn node(&self, id: &NodeId) -> Option<&NodeRecord> {
        self.store.node(id)
    }

    /// Inserts or replaces a node directly inside the store.
    ///
    /// The spike uses this to create motion entities prior to executing rewrites.
    pub fn insert_node(&mut self, record: NodeRecord) {
        self.store.insert_node(record);
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

/// Serialises a 3D position + velocity vector pair into the canonical payload.
pub fn encode_motion_payload(position: [f32; 3], velocity: [f32; 3]) -> Bytes {
    let mut buf = Vec::with_capacity(POSITION_VELOCITY_BYTES);
    for value in position.into_iter().chain(velocity.into_iter()) {
        buf.extend_from_slice(&value.to_le_bytes());
    }
    Bytes::from(buf)
}

/// Deserialises a canonical motion payload into (position, velocity) slices.
pub fn decode_motion_payload(bytes: &Bytes) -> Option<([f32; 3], [f32; 3])> {
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

/// Convenience helper for deriving `TypeId` values from human-readable labels.
pub fn make_type_id(label: &str) -> TypeId {
    TypeId(hash_label(label))
}

/// Convenience helper for deriving `NodeId` values from human-readable labels.
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

/// Executor that updates the encoded position in the entity payload.
fn motion_executor(store: &mut GraphStore, scope: &NodeId) {
    if let Some(record) = store.node_mut(scope)
        && let Some(payload) = &record.payload
        && let Some((position, velocity)) = decode_motion_payload(payload)
    {
        let updated = encode_motion_payload(add_vec(position, velocity), velocity);
        record.payload = Some(updated);
    }
}

/// Matcher used by the motion rule to ensure the payload is well-formed.
fn motion_matcher(store: &GraphStore, scope: &NodeId) -> bool {
    store
        .node(scope)
        .and_then(|record| record.payload.as_ref())
        .and_then(decode_motion_payload)
        .is_some()
}

/// Returns the built-in motion rule used by the spike.
///
/// The rule advances an entity's position by its velocity; it is deliberately
/// deterministic so hash comparisons stay stable across independent executions.
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
        let payload = encode_motion_payload([1.0, 2.0, 3.0], [0.5, -1.0, 0.25]);

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
        let payload_b = encode_motion_payload([1.0, 2.0, 3.0], [0.5, -1.0, 0.25]);
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
            .and_then(decode_motion_payload)
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
