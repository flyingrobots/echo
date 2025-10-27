//! Core rewrite engine implementation.
use std::collections::HashMap;

use blake3::Hasher;
use thiserror::Error;

use crate::graph::GraphStore;
use crate::ident::{Hash, NodeId};
use crate::record::NodeRecord;
use crate::rule::RewriteRule;
use crate::scheduler::{DeterministicScheduler, PendingRewrite};
use crate::snapshot::{compute_snapshot_hash, Snapshot};
use crate::tx::TxId;

/// Result of calling [`Engine::apply`].
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
    /// A rule was requested that has not been registered with the engine.
    #[error("rule not registered: {0}")]
    UnknownRule(String),
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
    ///
    /// # Errors
    /// Returns [`EngineError::UnknownTx`] if the transaction is invalid, or
    /// [`EngineError::UnknownRule`] if the named rule is not registered.
    pub fn apply(
        &mut self,
        tx: TxId,
        rule_name: &str,
        scope: &NodeId,
    ) -> Result<ApplyResult, EngineError> {
        if tx.0 == 0 || tx.0 > self.tx_counter {
            return Err(EngineError::UnknownTx);
        }
        let Some(rule) = self.rules.get(rule_name) else {
            return Err(EngineError::UnknownRule(rule_name.to_owned()));
        };
        let matches = (rule.matcher)(&self.store, scope);
        if !matches {
            return Ok(ApplyResult::NoMatch);
        }

        let scope_hash = scope_hash(rule, scope);
        self.scheduler.pending.entry(tx).or_default().insert(
            (scope_hash, rule.id),
            PendingRewrite {
                tx,
                rule_id: rule.id,
                scope: *scope,
            },
        );

        Ok(ApplyResult::Applied)
    }

    /// Executes all pending rewrites for the transaction and produces a snapshot.
    ///
    /// # Errors
    /// Returns [`EngineError::UnknownTx`] if `tx` does not refer to a live transaction.
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
            root: self.current_root,
            hash,
            parent: self.last_snapshot.as_ref().map(|s| s.hash),
            tx,
        };
        self.last_snapshot = Some(snapshot.clone());
        Ok(snapshot)
    }

    /// Returns a snapshot for the current graph state without executing rewrites.
    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        let hash = compute_snapshot_hash(&self.store, &self.current_root);
        Snapshot {
            root: self.current_root,
            hash,
            parent: self.last_snapshot.as_ref().map(|s| s.hash),
            tx: TxId(self.tx_counter),
        }
    }

    /// Returns a shared view of a node when it exists.
    #[must_use]
    pub fn node(&self, id: &NodeId) -> Option<&NodeRecord> {
        self.store.node(id)
    }

    /// Inserts or replaces a node directly inside the store.
    ///
    /// The spike uses this to create motion entities prior to executing rewrites.
    pub fn insert_node(&mut self, id: NodeId, record: NodeRecord) {
        self.store.insert_node(id, record);
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
