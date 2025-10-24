//! rmg-core: typed deterministic graph rewriting engine.

use std::collections::{BTreeMap, HashMap};

use blake3::Hasher;
use bytes::Bytes;
use thiserror::Error;

pub type Hash = [u8; 32];

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TypeId(pub Hash);

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
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
    pub nodes: HashMap<NodeId, NodeRecord>,
    pub edges_from: HashMap<NodeId, Vec<EdgeRecord>>,
}

impl GraphStore {
    pub fn node(&self, id: &NodeId) -> Option<&NodeRecord> {
        self.nodes.get(id)
    }

    pub fn edges_from(&self, id: &NodeId) -> impl Iterator<Item = &EdgeRecord> {
        self.edges_from.get(id).into_iter().flatten()
    }
}

#[derive(Debug, Clone)]
pub struct PatternGraph {
    pub nodes: Vec<TypeId>,
}

#[derive(Debug, Clone)]
pub struct RewriteRule {
    pub id: Hash,
    pub name: &'static str,
    pub left: PatternGraph,
}

#[derive(Debug, Clone, Copy)]
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
        // TODO: execute pending rewrites in deterministic order (placeholder flush)
        self.scheduler.pending.clear();

        let hash = hash_root(&self.current_root);
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
        let hash = hash_root(&self.current_root);
        Snapshot {
            root: self.current_root.clone(),
            hash,
            parent: self.last_snapshot.as_ref().map(|s| s.hash),
            tx: TxId(self.tx_counter),
        }
    }
}

fn scope_hash(rule: &RewriteRule, scope: &NodeId) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(&rule.id);
    hasher.update(&scope.0);
    hasher.finalize().into()
}

fn hash_root(root: &NodeId) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(&root.0);
    hasher.finalize().into()
}
