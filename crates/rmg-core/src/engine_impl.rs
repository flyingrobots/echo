// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Core rewrite engine implementation.
use std::collections::{HashMap, HashSet};

use blake3::Hasher;
use thiserror::Error;

use crate::graph::GraphStore;
use crate::ident::{CompactRuleId, Hash, NodeId};
use crate::record::NodeRecord;
use crate::rule::{ConflictPolicy, RewriteRule};
use crate::scheduler::{DeterministicScheduler, PendingRewrite, RewritePhase, SchedulerKind};
use crate::snapshot::{compute_commit_hash, compute_state_root, Snapshot};
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
    #[error("transaction not active")]
    UnknownTx,
    /// A rule was requested that has not been registered with the engine.
    #[error("rule not registered: {0}")]
    UnknownRule(String),
    /// Attempted to register a rule with a duplicate name.
    #[error("duplicate rule name: {0}")]
    DuplicateRuleName(&'static str),
    /// Attempted to register a rule with a duplicate ID.
    #[error("duplicate rule id: {0:?}")]
    DuplicateRuleId(Hash),
    /// Conflict policy Join requires a join function.
    #[error("missing join function for ConflictPolicy::Join")]
    MissingJoinFn,
    /// Internal invariant violated (engine state corruption).
    #[error("internal invariant violated: {0}")]
    InternalCorruption(&'static str),
}

/// Core rewrite engine used by the spike.
///
/// It owns a `GraphStore`, the registered rules, and the deterministic
/// scheduler. Snapshot determinism is provided by the snapshot hashing routine:
/// includes the root id, all nodes in ascending `NodeId` order, and all
/// outbound edges per node sorted by `EdgeId`. All length prefixes are 8-byte
/// little-endian and ids are raw 32-byte values. Changing any of these rules is
/// a breaking change to snapshot identity and must be recorded in the
/// determinism spec and tests.
pub struct Engine {
    store: GraphStore,
    rules: HashMap<&'static str, RewriteRule>,
    rules_by_id: HashMap<Hash, &'static str>,
    compact_rule_ids: HashMap<Hash, CompactRuleId>,
    rules_by_compact: HashMap<CompactRuleId, &'static str>,
    scheduler: DeterministicScheduler,
    tx_counter: u64,
    live_txs: HashSet<u64>,
    current_root: NodeId,
    last_snapshot: Option<Snapshot>,
}

impl Engine {
    /// Constructs a new engine with the supplied backing store and root node id.
    pub fn new(store: GraphStore, root: NodeId) -> Self {
        Self::with_scheduler(store, root, SchedulerKind::Radix)
    }

    /// Constructs a new engine with an explicit scheduler kind (radix vs. legacy).
    pub fn with_scheduler(store: GraphStore, root: NodeId, kind: SchedulerKind) -> Self {
        Self {
            store,
            rules: HashMap::new(),
            rules_by_id: HashMap::new(),
            compact_rule_ids: HashMap::new(),
            rules_by_compact: HashMap::new(),
            scheduler: DeterministicScheduler::new(kind),
            tx_counter: 0,
            live_txs: HashSet::new(),
            current_root: root,
            last_snapshot: None,
        }
    }

    /// Registers a rewrite rule so it can be referenced by name.
    ///
    /// # Errors
    /// Returns [`EngineError::DuplicateRuleName`] if a rule with the same
    /// name has already been registered, or [`EngineError::DuplicateRuleId`]
    /// if a rule with the same id was previously registered.
    pub fn register_rule(&mut self, rule: RewriteRule) -> Result<(), EngineError> {
        if self.rules.contains_key(rule.name) {
            return Err(EngineError::DuplicateRuleName(rule.name));
        }
        if self.rules_by_id.contains_key(&rule.id) {
            return Err(EngineError::DuplicateRuleId(rule.id));
        }
        if matches!(rule.conflict_policy, ConflictPolicy::Join) && rule.join_fn.is_none() {
            return Err(EngineError::MissingJoinFn);
        }
        self.rules_by_id.insert(rule.id, rule.name);
        debug_assert!(
            self.compact_rule_ids.len() < u32::MAX as usize,
            "too many rules to assign a compact id"
        );
        #[allow(clippy::cast_possible_truncation)]
        let next = CompactRuleId(self.compact_rule_ids.len() as u32);
        let compact = *self.compact_rule_ids.entry(rule.id).or_insert(next);
        self.rules_by_compact.insert(compact, rule.name);
        self.rules.insert(rule.name, rule);
        Ok(())
    }

    /// Begins a new transaction and returns its identifier.
    #[must_use]
    pub fn begin(&mut self) -> TxId {
        // Increment with wrap and ensure we never produce 0 (reserved invalid).
        self.tx_counter = self.tx_counter.wrapping_add(1);
        if self.tx_counter == 0 {
            self.tx_counter = 1;
        }
        self.live_txs.insert(self.tx_counter);
        TxId::from_raw(self.tx_counter)
    }

    /// Queues a rewrite for execution if it matches the provided scope.
    ///
    /// # Errors
    /// Returns [`EngineError::UnknownTx`] if the transaction is invalid, or
    /// [`EngineError::UnknownRule`] if the named rule is not registered.
    ///
    /// # Panics
    /// Panics only if internal rule tables are corrupted (should not happen
    /// when rules are registered via `register_rule`).
    pub fn apply(
        &mut self,
        tx: TxId,
        rule_name: &str,
        scope: &NodeId,
    ) -> Result<ApplyResult, EngineError> {
        if tx.value() == 0 || !self.live_txs.contains(&tx.value()) {
            return Err(EngineError::UnknownTx);
        }
        let Some(rule) = self.rules.get(rule_name) else {
            return Err(EngineError::UnknownRule(rule_name.to_owned()));
        };
        let matches = (rule.matcher)(&self.store, scope);
        if !matches {
            return Ok(ApplyResult::NoMatch);
        }

        let scope_fp = scope_hash(rule, scope);
        let footprint = (rule.compute_footprint)(&self.store, scope);
        let Some(&compact_rule) = self.compact_rule_ids.get(&rule.id) else {
            return Err(EngineError::InternalCorruption(
                "missing compact rule id for a registered rule",
            ));
        };
        self.scheduler.enqueue(
            tx,
            PendingRewrite {
                rule_id: rule.id,
                compact_rule,
                scope_hash: scope_fp,
                scope: *scope,
                footprint,
                phase: RewritePhase::Matched,
            },
        );

        Ok(ApplyResult::Applied)
    }

    /// Executes all pending rewrites for the transaction and produces a snapshot.
    ///
    /// # Errors
    /// - Returns [`EngineError::UnknownTx`] if `tx` does not refer to a live transaction.
    /// - Returns [`EngineError::InternalCorruption`] if internal rule tables are
    ///   corrupted (e.g., a reserved rewrite references a missing rule).
    pub fn commit(&mut self, tx: TxId) -> Result<Snapshot, EngineError> {
        if tx.value() == 0 || !self.live_txs.contains(&tx.value()) {
            return Err(EngineError::UnknownTx);
        }
        // Drain pending to form the ready set and compute a plan digest over its canonical order.
        let drained = self.scheduler.drain_for_tx(tx);
        let plan_digest = {
            let mut hasher = blake3::Hasher::new();
            hasher.update(&(drained.len() as u64).to_le_bytes());
            for pr in &drained {
                hasher.update(&pr.scope_hash);
                hasher.update(&pr.rule_id);
            }
            hasher.finalize().into()
        };

        // Reserve phase: enforce independence against active frontier.
        let mut reserved: Vec<PendingRewrite> = Vec::new();
        for mut rewrite in drained {
            if self.scheduler.reserve(tx, &mut rewrite) {
                reserved.push(rewrite);
            }
        }
        // Deterministic digest of the ordered rewrites we will apply.
        let rewrites_digest = {
            let mut hasher = blake3::Hasher::new();
            hasher.update(&(reserved.len() as u64).to_le_bytes());
            for r in &reserved {
                hasher.update(&r.rule_id);
                hasher.update(&r.scope_hash);
                hasher.update(&(r.scope).0);
            }
            hasher.finalize().into()
        };

        for rewrite in reserved {
            let id = rewrite.compact_rule;
            let Some(rule) = self.rule_by_compact(id) else {
                debug_assert!(false, "missing rule for compact id: {id:?}");
                return Err(EngineError::InternalCorruption(
                    "missing rule for compact id during commit",
                ));
            };
            (rule.executor)(&mut self.store, &rewrite.scope);
        }

        let state_root = crate::snapshot::compute_state_root(&self.store, &self.current_root);
        let parents: Vec<Hash> = self
            .last_snapshot
            .as_ref()
            .map(|s| vec![s.hash])
            .unwrap_or_default();
        // Canonical empty digest (0-length list) for decisions until Aion lands.
        let decision_digest: Hash = *crate::constants::DIGEST_LEN0_U64;
        let hash = crate::snapshot::compute_commit_hash(
            &state_root,
            &parents,
            &plan_digest,
            &decision_digest,
            &rewrites_digest,
            0,
        );
        let snapshot = Snapshot {
            root: self.current_root,
            hash,
            parents,
            plan_digest,
            decision_digest,
            rewrites_digest,
            policy_id: 0,
            tx,
        };
        self.last_snapshot = Some(snapshot.clone());
        // Mark transaction as closed/inactive and finalize scheduler accounting.
        self.live_txs.remove(&tx.value());
        self.scheduler.finalize_tx(tx);
        Ok(snapshot)
    }

    /// Returns a snapshot for the current graph state without executing rewrites.
    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        // Build a lightweight snapshot view of the current state using the
        // same commit header shape but with zeroed metadata digests. This
        // ensures callers see the same stable structure as real commits while
        // making it clear that no rewrites were applied.
        let state_root = compute_state_root(&self.store, &self.current_root);
        let parents: Vec<Hash> = self
            .last_snapshot
            .as_ref()
            .map(|s| vec![s.hash])
            .unwrap_or_default();
        // Canonical empty digests match commit() behaviour when no rewrites are pending.
        let empty_digest: Hash = {
            let mut h = blake3::Hasher::new();
            h.update(&0u64.to_le_bytes());
            h.finalize().into()
        };
        let decision_empty: Hash = *crate::constants::DIGEST_LEN0_U64;
        let hash = compute_commit_hash(
            &state_root,
            &parents,
            &empty_digest,
            &decision_empty,
            &empty_digest,
            0,
        );
        Snapshot {
            root: self.current_root,
            hash,
            parents,
            plan_digest: empty_digest,
            decision_digest: decision_empty,
            rewrites_digest: empty_digest,
            policy_id: 0,
            tx: TxId::from_raw(self.tx_counter),
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
    fn rule_by_compact(&self, id: CompactRuleId) -> Option<&RewriteRule> {
        let name = self.rules_by_compact.get(&id)?;
        self.rules.get(name)
    }
}

fn scope_hash(rule: &RewriteRule, scope: &NodeId) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(&rule.id);
    hasher.update(&scope.0);
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{demo::motion::motion_rule, ident::make_node_id};

    #[test]
    fn scope_hash_stable_for_rule_and_scope() {
        let rule = motion_rule();
        let scope = make_node_id("scope-hash-entity");
        let h1 = super::scope_hash(&rule, &scope);
        // Recompute expected value manually using the same inputs.
        let mut hasher = blake3::Hasher::new();
        hasher.update(&rule.id);
        hasher.update(&scope.0);
        let expected: Hash = hasher.finalize().into();
        assert_eq!(h1, expected);
    }

    #[test]
    fn register_rule_join_requires_join_fn() {
        // Build a rule that declares Join but provides no join_fn.
        let bad = RewriteRule {
            id: [0u8; 32],
            name: "bad/join",
            left: crate::rule::PatternGraph { nodes: vec![] },
            matcher: |_s, _n| true,
            executor: |_s, _n| {},
            compute_footprint: |_s, _n| crate::footprint::Footprint::default(),
            factor_mask: 0,
            conflict_policy: crate::rule::ConflictPolicy::Join,
            join_fn: None,
        };
        let mut engine = Engine::new(GraphStore::default(), make_node_id("r"));
        let res = engine.register_rule(bad);
        assert!(
            matches!(res, Err(EngineError::MissingJoinFn)),
            "expected MissingJoinFn, got {res:?}"
        );
    }
}
