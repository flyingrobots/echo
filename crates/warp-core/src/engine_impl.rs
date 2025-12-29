// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Core rewrite engine implementation.
use std::collections::{HashMap, HashSet};

use blake3::Hasher;
use thiserror::Error;

use crate::graph::GraphStore;
use crate::ident::{CompactRuleId, EdgeId, Hash, NodeId};
use crate::receipt::{TickReceipt, TickReceiptDisposition, TickReceiptEntry, TickReceiptRejection};
use crate::record::NodeRecord;
use crate::rule::{ConflictPolicy, RewriteRule};
use crate::scheduler::{DeterministicScheduler, PendingRewrite, RewritePhase, SchedulerKind};
use crate::snapshot::{compute_commit_hash_v2, compute_state_root, Snapshot};
use crate::tick_patch::{diff_store, SlotId, TickCommitStatus, WarpTickPatchV1};
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
    policy_id: u32,
    tx_counter: u64,
    live_txs: HashSet<u64>,
    current_root: NodeId,
    last_snapshot: Option<Snapshot>,
}

struct ReserveOutcome {
    receipt: TickReceipt,
    reserved: Vec<PendingRewrite>,
    in_slots: std::collections::BTreeSet<SlotId>,
    out_slots: std::collections::BTreeSet<SlotId>,
}

impl Engine {
    /// Constructs a new engine with the supplied backing store and root node id.
    ///
    /// Uses the default scheduler (Radix) and the default policy id
    /// [`crate::POLICY_ID_NO_POLICY_V0`].
    pub fn new(store: GraphStore, root: NodeId) -> Self {
        Self::with_scheduler_and_policy_id(
            store,
            root,
            SchedulerKind::Radix,
            crate::POLICY_ID_NO_POLICY_V0,
        )
    }

    /// Constructs a new engine with an explicit scheduler kind (radix vs. legacy).
    ///
    /// Uses the default policy id [`crate::POLICY_ID_NO_POLICY_V0`].
    pub fn with_scheduler(store: GraphStore, root: NodeId, kind: SchedulerKind) -> Self {
        Self::with_scheduler_and_policy_id(store, root, kind, crate::POLICY_ID_NO_POLICY_V0)
    }

    /// Constructs a new engine with an explicit policy identifier.
    ///
    /// `policy_id` is committed into both `patch_digest` (tick patches) and
    /// `commit_id` (commit hash v2). Callers must treat it as part of the
    /// deterministic boundary.
    pub fn with_policy_id(store: GraphStore, root: NodeId, policy_id: u32) -> Self {
        Self::with_scheduler_and_policy_id(store, root, SchedulerKind::Radix, policy_id)
    }

    /// Constructs a new engine with explicit scheduler kind and policy id.
    pub fn with_scheduler_and_policy_id(
        store: GraphStore,
        root: NodeId,
        kind: SchedulerKind,
        policy_id: u32,
    ) -> Self {
        Self {
            store,
            rules: HashMap::new(),
            rules_by_id: HashMap::new(),
            compact_rule_ids: HashMap::new(),
            rules_by_compact: HashMap::new(),
            scheduler: DeterministicScheduler::new(kind),
            policy_id,
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

        let scope_fp = scope_hash(&rule.id, scope);
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
        let (snapshot, _receipt, _patch) = self.commit_with_receipt(tx)?;
        Ok(snapshot)
    }

    /// Executes all pending rewrites for the transaction, producing both a snapshot and a tick receipt.
    ///
    /// The receipt records (in canonical plan order) which candidates were accepted vs rejected.
    /// For rejected candidates, it also records which earlier applied candidates blocked them
    /// (a minimal blocking-causality witness / poset edge list, per Paper II).
    ///
    /// This method also produces a delta tick patch (Paper III): a replayable boundary artifact
    /// whose digest is committed into the v2 commit hash.
    ///
    /// # Errors
    /// - Returns [`EngineError::UnknownTx`] if `tx` does not refer to a live transaction.
    /// - Returns [`EngineError::InternalCorruption`] if internal rule tables are
    ///   corrupted (e.g., a reserved rewrite references a missing rule).
    pub fn commit_with_receipt(
        &mut self,
        tx: TxId,
    ) -> Result<(Snapshot, TickReceipt, WarpTickPatchV1), EngineError> {
        if tx.value() == 0 || !self.live_txs.contains(&tx.value()) {
            return Err(EngineError::UnknownTx);
        }
        let policy_id = self.policy_id;
        let rule_pack_id = self.compute_rule_pack_id();
        // Drain pending to form the ready set and compute a plan digest over its canonical order.
        let drained = self.scheduler.drain_for_tx(tx);
        let plan_digest = compute_plan_digest(&drained);

        let ReserveOutcome {
            receipt,
            reserved: reserved_rewrites,
            in_slots,
            out_slots,
        } = self.reserve_for_receipt(tx, drained)?;

        // Deterministic digest of the ordered rewrites we will apply.
        let rewrites_digest = compute_rewrites_digest(&reserved_rewrites);

        // Capture pre-state for delta patch construction.
        let store_before = self.store.clone();

        self.apply_reserved_rewrites(reserved_rewrites)?;

        // Delta tick patch (Paper III boundary artifact).
        let ops = diff_store(&store_before, &self.store);
        let patch = WarpTickPatchV1::new(
            policy_id,
            rule_pack_id,
            TickCommitStatus::Committed,
            in_slots.into_iter().collect(),
            out_slots.into_iter().collect(),
            ops,
        );
        let patch_digest = patch.digest();

        let state_root = crate::snapshot::compute_state_root(&self.store, &self.current_root);
        let parents: Vec<Hash> = self
            .last_snapshot
            .as_ref()
            .map(|s| vec![s.hash])
            .unwrap_or_default();
        // `decision_digest` is reserved for Aion tie-breaks; in the spike we use the tick receipt digest
        // to commit to accepted/rejected decisions in a deterministic way.
        let decision_digest: Hash = receipt.digest();
        let hash = crate::snapshot::compute_commit_hash_v2(
            &state_root,
            &parents,
            &patch_digest,
            policy_id,
        );
        let snapshot = Snapshot {
            root: self.current_root,
            hash,
            parents,
            plan_digest,
            decision_digest,
            rewrites_digest,
            patch_digest,
            policy_id,
            tx,
        };
        self.last_snapshot = Some(snapshot.clone());
        // Mark transaction as closed/inactive and finalize scheduler accounting.
        self.live_txs.remove(&tx.value());
        self.scheduler.finalize_tx(tx);
        Ok((snapshot, receipt, patch))
    }

    fn reserve_for_receipt(
        &mut self,
        tx: TxId,
        drained: Vec<PendingRewrite>,
    ) -> Result<ReserveOutcome, EngineError> {
        // Reserve phase: enforce independence against active frontier.
        let mut receipt_entries: Vec<TickReceiptEntry> = Vec::with_capacity(drained.len());
        let mut in_slots: std::collections::BTreeSet<SlotId> = std::collections::BTreeSet::new();
        let mut out_slots: std::collections::BTreeSet<SlotId> = std::collections::BTreeSet::new();
        let mut blocked_by: Vec<Vec<u32>> = Vec::with_capacity(drained.len());
        let mut reserved: Vec<PendingRewrite> = Vec::new();
        let mut reserved_entry_indices: Vec<u32> = Vec::new();

        for (entry_idx, mut rewrite) in drained.into_iter().enumerate() {
            let entry_idx_u32 = u32::try_from(entry_idx).map_err(|_| {
                EngineError::InternalCorruption("too many receipt entries to index")
            })?;
            let accepted = self.scheduler.reserve(tx, &mut rewrite);
            let blockers = if accepted {
                Vec::new()
            } else {
                // O(n) scan over reserved rewrites. Acceptable for typical tick sizes;
                // consider spatial indexing if tick candidate counts grow large.
                let mut blockers: Vec<u32> = Vec::new();
                for (k, prior) in reserved.iter().enumerate() {
                    if footprints_conflict(&rewrite.footprint, &prior.footprint) {
                        blockers.push(reserved_entry_indices[k]);
                    }
                }
                if blockers.is_empty() {
                    // `reserve()` currently returns `false` exclusively on footprint
                    // conflicts (see scheduler reserve rustdoc). If additional rejection
                    // reasons are added, update the scheduler contract and this attribution
                    // logic accordingly.
                    return Err(EngineError::InternalCorruption(
                        "scheduler rejected rewrite but no blockers were found",
                    ));
                }
                blockers
            };
            receipt_entries.push(TickReceiptEntry {
                rule_id: rewrite.rule_id,
                scope_hash: rewrite.scope_hash,
                scope: rewrite.scope,
                disposition: if accepted {
                    TickReceiptDisposition::Applied
                } else {
                    // NOTE: reserve() currently returns `false` exclusively on
                    // footprint conflicts (see scheduler reserve rustdoc).
                    // If additional rejection reasons are added, update this mapping.
                    TickReceiptDisposition::Rejected(TickReceiptRejection::FootprintConflict)
                },
            });
            if accepted {
                extend_slots_from_footprint(&mut in_slots, &mut out_slots, &rewrite.footprint);
                reserved.push(rewrite);
                reserved_entry_indices.push(entry_idx_u32);
            }
            blocked_by.push(blockers);
        }

        Ok(ReserveOutcome {
            receipt: TickReceipt::new(tx, receipt_entries, blocked_by),
            reserved,
            in_slots,
            out_slots,
        })
    }

    fn apply_reserved_rewrites(
        &mut self,
        rewrites: Vec<PendingRewrite>,
    ) -> Result<(), EngineError> {
        for rewrite in rewrites {
            let id = rewrite.compact_rule;
            let Some(rule) = self.rule_by_compact(id) else {
                debug_assert!(false, "missing rule for compact id: {id:?}");
                return Err(EngineError::InternalCorruption(
                    "missing rule for compact id during commit",
                ));
            };
            (rule.executor)(&mut self.store, &rewrite.scope);
        }
        Ok(())
    }

    /// Returns a snapshot for the current graph state without executing rewrites.
    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        // Build a lightweight snapshot view of the current state using the
        // same v2 commit hash shape (parents + state_root + patch_digest) but
        // with empty diagnostic digests. This makes it explicit that no rewrites
        // were applied while keeping the structure stable for callers/tools.
        let state_root = compute_state_root(&self.store, &self.current_root);
        let parents: Vec<Hash> = self
            .last_snapshot
            .as_ref()
            .map(|s| vec![s.hash])
            .unwrap_or_default();
        // Canonical empty digests match commit() behaviour when no rewrites are pending.
        let empty_digest: Hash = *crate::constants::DIGEST_LEN0_U64;
        let decision_empty: Hash = *crate::constants::DIGEST_LEN0_U64;
        let policy_id = self.policy_id;
        let rule_pack_id = self.compute_rule_pack_id();
        let patch_digest = WarpTickPatchV1::new(
            policy_id,
            rule_pack_id,
            TickCommitStatus::Committed,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .digest();
        let hash = compute_commit_hash_v2(&state_root, &parents, &patch_digest, policy_id);
        Snapshot {
            root: self.current_root,
            hash,
            parents,
            plan_digest: empty_digest,
            decision_digest: decision_empty,
            rewrites_digest: empty_digest,
            patch_digest,
            policy_id,
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

fn footprints_conflict(a: &crate::footprint::Footprint, b: &crate::footprint::Footprint) -> bool {
    // IMPORTANT: do not use `Footprint::independent` here yet.
    //
    // This logic MUST remain consistent with the scheduler’s footprint conflict
    // predicate (`RadixScheduler::has_conflict` in `scheduler.rs`). If one
    // changes, the other must change too, or receipts will attribute blockers
    // differently than the scheduler rejects candidates.
    //
    // `Footprint::independent` includes a `factor_mask` fast-path that assumes
    // masks are correctly populated as a conservative superset. Many current
    // footprints in the engine spike use `factor_mask = 0` as a placeholder,
    // which would incorrectly classify conflicting rewrites as independent.
    //
    // The scheduler’s conflict logic is defined by explicit overlap checks on
    // nodes/edges/ports; this mirrors that behavior exactly and stays correct
    // while factor masks are still being wired through.
    if a.b_in.intersects(&b.b_in)
        || a.b_in.intersects(&b.b_out)
        || a.b_out.intersects(&b.b_in)
        || a.b_out.intersects(&b.b_out)
    {
        return true;
    }
    if a.e_write.intersects(&b.e_write)
        || a.e_write.intersects(&b.e_read)
        || b.e_write.intersects(&a.e_read)
    {
        return true;
    }
    a.n_write.intersects(&b.n_write)
        || a.n_write.intersects(&b.n_read)
        || b.n_write.intersects(&a.n_read)
}

fn compute_plan_digest(plan: &[PendingRewrite]) -> Hash {
    if plan.is_empty() {
        return *crate::constants::DIGEST_LEN0_U64;
    }
    let mut hasher = Hasher::new();
    hasher.update(&(plan.len() as u64).to_le_bytes());
    for pr in plan {
        hasher.update(&pr.scope_hash);
        hasher.update(&pr.rule_id);
    }
    hasher.finalize().into()
}

fn compute_rewrites_digest(rewrites: &[PendingRewrite]) -> Hash {
    if rewrites.is_empty() {
        return *crate::constants::DIGEST_LEN0_U64;
    }
    let mut hasher = Hasher::new();
    hasher.update(&(rewrites.len() as u64).to_le_bytes());
    for r in rewrites {
        hasher.update(&r.rule_id);
        hasher.update(&r.scope_hash);
        hasher.update(&(r.scope).0);
    }
    hasher.finalize().into()
}

impl Engine {
    fn rule_by_compact(&self, id: CompactRuleId) -> Option<&RewriteRule> {
        let name = self.rules_by_compact.get(&id)?;
        self.rules.get(name)
    }

    fn compute_rule_pack_id(&self) -> Hash {
        let mut ids: Vec<Hash> = self.rules.values().map(|r| r.id).collect();
        ids.sort_unstable();
        ids.dedup();

        let mut h = Hasher::new();
        // Version tag for future evolution.
        h.update(&1u16.to_le_bytes());
        h.update(&(ids.len() as u64).to_le_bytes());
        for id in ids {
            h.update(&id);
        }
        h.finalize().into()
    }
}

/// Computes the canonical scope hash used for deterministic scheduler ordering.
///
/// This value is the first component of the scheduler’s canonical ordering key
/// (`scope_hash`, then `rule_id`, then nonce), and is used to domain-separate
/// candidates by both the producing rule and the scoped node.
///
/// Stable definition (v0):
/// - `scope_hash := blake3(rule_id || scope_node_id)`
pub fn scope_hash(rule_id: &Hash, scope: &NodeId) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(rule_id);
    hasher.update(&scope.0);
    hasher.finalize().into()
}

fn extend_slots_from_footprint(
    in_slots: &mut std::collections::BTreeSet<SlotId>,
    out_slots: &mut std::collections::BTreeSet<SlotId>,
    fp: &crate::footprint::Footprint,
) {
    for node_hash in fp.n_read.iter() {
        in_slots.insert(SlotId::Node(NodeId(*node_hash)));
    }
    for node_hash in fp.n_write.iter() {
        let id = NodeId(*node_hash);
        in_slots.insert(SlotId::Node(id));
        out_slots.insert(SlotId::Node(id));
    }
    for edge_hash in fp.e_read.iter() {
        in_slots.insert(SlotId::Edge(EdgeId(*edge_hash)));
    }
    for edge_hash in fp.e_write.iter() {
        let id = EdgeId(*edge_hash);
        in_slots.insert(SlotId::Edge(id));
        out_slots.insert(SlotId::Edge(id));
    }
    for port_key in fp.b_in.keys() {
        in_slots.insert(SlotId::Port(*port_key));
    }
    for port_key in fp.b_out.keys() {
        in_slots.insert(SlotId::Port(*port_key));
        out_slots.insert(SlotId::Port(*port_key));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::demo::motion::{motion_rule, MOTION_RULE_NAME};
    use crate::ident::{make_node_id, make_type_id};
    use crate::payload::encode_motion_payload;
    use crate::record::NodeRecord;

    #[test]
    fn scope_hash_stable_for_rule_and_scope() {
        let rule = motion_rule();
        let scope = make_node_id("scope-hash-entity");
        let h1 = super::scope_hash(&rule.id, &scope);
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

    #[test]
    fn tick_patch_replay_matches_post_state() {
        let entity = make_node_id("tick-patch-entity");
        let entity_type = make_type_id("entity");
        let payload = encode_motion_payload([0.0, 0.0, 0.0], [1.0, 0.0, 0.0]);

        let mut store = GraphStore::default();
        store.insert_node(
            entity,
            NodeRecord {
                ty: entity_type,
                payload: Some(payload),
            },
        );

        let mut engine = Engine::new(store, entity);
        let register = engine.register_rule(motion_rule());
        assert!(register.is_ok(), "rule registration failed: {register:?}");

        let tx = engine.begin();
        let applied = engine.apply(tx, MOTION_RULE_NAME, &entity);
        assert!(
            matches!(applied, Ok(ApplyResult::Applied)),
            "expected ApplyResult::Applied, got {applied:?}"
        );

        let store_before = engine.store.clone();
        let committed = engine.commit_with_receipt(tx);
        assert!(
            committed.is_ok(),
            "commit_with_receipt failed: {committed:?}"
        );
        let Ok((snapshot, _receipt, patch)) = committed else {
            return;
        };
        let store_after = engine.store.clone();

        // Replay patch delta from the captured pre-state and compare the resulting state root.
        let mut store_replay = store_before;
        let replay = patch.apply_to_store(&mut store_replay);
        assert!(replay.is_ok(), "patch replay failed: {replay:?}");

        let root = entity;
        let state_after = compute_state_root(&store_after, &root);
        let state_replay = compute_state_root(&store_replay, &root);
        assert_eq!(
            state_after, state_replay,
            "patch replay must match post-state"
        );

        // Patch digest is the committed boundary artifact in commit hash v2.
        assert_eq!(snapshot.patch_digest, patch.digest());
        assert_eq!(
            snapshot.hash,
            compute_commit_hash_v2(
                &state_after,
                &snapshot.parents,
                &snapshot.patch_digest,
                snapshot.policy_id
            ),
            "commit hash v2 must commit to state_root + patch_digest (+ parents/policy)"
        );

        // Conservative slots from footprint: motion writes the scoped node record.
        assert!(patch.in_slots().contains(&SlotId::Node(entity)));
        assert!(patch.out_slots().contains(&SlotId::Node(entity)));
    }
}
