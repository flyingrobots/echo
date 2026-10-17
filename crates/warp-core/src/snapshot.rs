// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! Snapshot type and hash computation.
//!
//! See the high-level spec in `docs/spec/merkle-commit.md` for precise
//! definitions of `state_root` (graph-only hash) and `commit hash` (aka
//! `commit_id`: `state_root` + metadata + parents).
//!
//! Determinism contract
//! - The graph state hash (`state_root`) is a BLAKE3 digest over a canonical
//!   byte stream that encodes the entire reachable graph state for the current
//!   root.
//! - Ordering is explicit and stable: nodes are visited in ascending `NodeId`
//!   order (lexicographic over 32-byte ids). For each node, outbound edges are
//!   sorted by ascending `EdgeId` before being encoded.
//! - Encoding is fixed-size and architecture-independent:
//!   - All ids (`NodeId`, `TypeId`, `EdgeId`) are raw 32-byte values.
//!   - Payloads are encoded as:
//!     - 1 byte presence tag (`0` = None, `1` = Some)
//!     - when present: payload `type_id` (32 bytes), then 8-byte little-endian
//!       length, then the exact payload bytes.
//! - The root id is included first to bind the subgraph identity.
//!
//! Notes
//! - Little-endian was chosen for length fields to match the rest of the code
//!   base; changing endianness would change hash values and must be treated as a
//!   breaking change. If we decide to adopt big-endian, update the encoding
//!   here and add a migration note in the determinism spec.
//! - The in-memory store uses `BTreeMap`, which guarantees deterministic key
//!   iteration. For vectors (edge lists), we sort explicitly by `EdgeId`.
#[cfg(test)]
use std::collections::{BTreeSet, VecDeque};

use blake3::Hasher;

use crate::attachment::{AtomPayload, AttachmentKey, AttachmentOwner, AttachmentValue};
use crate::domain;
use crate::ident::{Hash, NodeKey, WarpId};
#[cfg(test)]
use crate::record::EdgeRecord;
use crate::tx::TxId;
use crate::warp_state::WarpState;

/// Snapshot returned after a successful commit.
///
/// The `hash` field is a deterministic commit hash (`commit_id`) computed from
/// `state_root` (graph-only hash) and commit metadata (parents, digests,
/// policy). Parents are explicit to support merges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// Node identifier that serves as the root of the snapshot.
    pub root: NodeKey,
    /// Canonical commit hash derived from `state_root` + metadata (see below).
    pub hash: Hash,
    /// Graph-only state hash (excludes commit metadata).
    pub state_root: Hash,
    /// Parent snapshot hashes (empty for initial commit, 1 for linear history, 2+ for merges).
    pub parents: Vec<Hash>,
    /// Deterministic digest of the candidate ready set and its canonical ordering.
    pub plan_digest: Hash,
    /// Deterministic digest of tick decisions (and later Aion tie‑break inputs).
    ///
    /// Today, `warp-core` uses this field to commit to the tick receipt decisions
    /// (accepted vs rejected candidates). Future Aion integration will extend
    /// this digest with additional agency inputs.
    pub decision_digest: Hash,
    /// Deterministic digest of the ordered rewrites applied during this commit.
    pub rewrites_digest: Hash,
    /// Deterministic digest of the tick patch boundary artifact.
    ///
    /// For commit hash v2, this is the sole committed “what happened” delta: the
    /// commit id commits to `state_root` and `patch_digest` (plus parents/policy),
    /// while plan/decision/rewrites digests remain diagnostics.
    pub patch_digest: Hash,
    /// Aion policy identifier (version pin for agency decisions).
    pub policy_id: u32,
    /// Transaction identifier associated with the snapshot.
    pub tx: TxId,
}

/// Reusable working memory for [`compute_state_root_with`].
///
/// An engine keeps one and passes it to every commit, so computing the state
/// root allocates nothing once these buffers have grown to the reachable
/// graph's size. Its contents never outlive a call and never affect the hash.
///
/// Reached nodes are kept by the logarithmic method: `levels[i]` is empty or
/// one sorted run of length in `[2^i, 2^(i+1))`. A new batch merges up through
/// the occupied levels like a binary-counter carry, so each node is merged
/// O(log n) times and reachability costs O(n log n) whatever the graph's
/// depth. Membership is a binary search in each of the O(log n) runs.
#[derive(Default)]
pub(crate) struct StateRootScratch {
    /// Sorted runs of reached nodes, one per size class.
    levels: Vec<Vec<NodeKey>>,
    /// The run being carried up through the levels.
    carry: Vec<NodeKey>,
    /// Merge target, swapped with `carry` or `reachable`.
    merged: Vec<NodeKey>,
    /// Every reachable node, sorted, once traversal ends.
    reachable: Vec<NodeKey>,
    /// The nodes found in the previous BFS level.
    frontier: Vec<NodeKey>,
    /// The nodes found in the current BFS level, not yet reached.
    next: Vec<NodeKey>,
    /// Every reachable warp, sorted once traversal ends.
    warps: Vec<WarpId>,
    /// One bucket's reachable edges, as positions in the bucket.
    edge_order: Vec<usize>,
    /// Keys copied by merges, so a test can bound the traversal's work.
    #[cfg(test)]
    moved: usize,
}

impl std::fmt::Debug for StateRootScratch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("StateRootScratch")
    }
}

/// Computes the canonical state root hash (graph only), with temporary working
/// memory. Hot paths call [`compute_state_root_with`] and keep the scratch.
pub(crate) fn compute_state_root(state: &WarpState, root: &NodeKey) -> Hash {
    compute_state_root_with(state, root, &mut StateRootScratch::default())
}

/// Computes the canonical state root hash (graph only), using `scratch` for
/// all working memory.
///
/// The hash covers the reachable graph in a fixed order: reachable warps
/// ascending, then in each warp its instance header, its reachable nodes by
/// ascending `NodeId`, and the bucket of each reachable source by ascending
/// source, its edges to reachable targets sorted by `EdgeId`. Only the
/// reachable set enters the hash, never the order in which it was found.
pub(crate) fn compute_state_root_with(
    state: &WarpState,
    root: &NodeKey,
    scratch: &mut StateRootScratch,
) -> Hash {
    collect_reachable(state, root, scratch);
    let StateRootScratch {
        reachable,
        warps,
        edge_order,
        ..
    } = scratch;

    let mut hasher = Hasher::new();
    hasher.update(domain::STATE_ROOT_V1);
    hasher.update(&(root.warp_id).0);
    hasher.update(&(root.local_id).0);

    for warp_id in warps.iter() {
        let Some(instance) = state.instance(warp_id) else {
            debug_assert!(false, "missing warp instance metadata: {warp_id:?}");
            continue;
        };
        let Some(store) = state.store(warp_id) else {
            debug_assert!(false, "missing warp store for instance: {warp_id:?}");
            continue;
        };

        // Instance header: bind metadata into the deterministic boundary.
        hasher.update(&(instance.warp_id).0);
        hasher.update(&(instance.root_node).0);
        hash_attachment_key_opt(&mut hasher, instance.parent.as_ref());

        // `reachable` is sorted by warp, then local id, so this warp's nodes
        // are one ascending run.
        let start = reachable.partition_point(|key| key.warp_id < *warp_id);
        let end = reachable.partition_point(|key| key.warp_id <= *warp_id);
        let in_warp = &reachable[start..end];

        // Nodes: ascending NodeId order. A reachable key with no record (a
        // dangling edge target) contributes nothing.
        for key in in_warp {
            let Some(node) = store.nodes.get(&key.local_id) else {
                continue;
            };
            hasher.update(&key.local_id.0);
            hasher.update(&(node.ty).0);
            hash_attachment_value_opt(&mut hasher, store.node_attachment(&key.local_id));
        }

        // Edges: per reachable source bucket, ascending source, sorted by
        // EdgeId, filtered to reachable targets. Edge ids are unique, so an
        // unstable sort gives the same order without a merge buffer.
        for key in in_warp {
            let Some(edges) = store.edges_from.get(&key.local_id) else {
                continue;
            };
            edge_order.clear();
            edge_order.extend((0..edges.len()).filter(|&index| {
                reachable
                    .binary_search(&NodeKey {
                        warp_id: *warp_id,
                        local_id: edges[index].to,
                    })
                    .is_ok()
            }));
            edge_order.sort_unstable_by_key(|&index| edges[index].id.0);

            hasher.update(&key.local_id.0);
            hasher.update(&(edge_order.len() as u64).to_le_bytes());
            for &index in edge_order.iter() {
                let edge = &edges[index];
                hasher.update(&(edge.id).0);
                hasher.update(&(edge.ty).0);
                hasher.update(&(edge.to).0);
                hash_attachment_value_opt(&mut hasher, store.edge_attachment(&edge.id));
            }
        }
    }

    hasher.finalize().into()
}

/// Merges sorted `a` and `b`, which share no element, into `out`.
fn merge_sorted(a: &[NodeKey], b: &[NodeKey], out: &mut Vec<NodeKey>) {
    out.clear();
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if a[i] < b[j] {
            out.push(a[i]);
            i += 1;
        } else {
            out.push(b[j]);
            j += 1;
        }
    }
    out.extend_from_slice(&a[i..]);
    out.extend_from_slice(&b[j..]);
}

/// Whether `key` is in any run of `levels`.
fn is_reached(levels: &[Vec<NodeKey>], key: &NodeKey) -> bool {
    levels.iter().any(|run| run.binary_search(key).is_ok())
}

/// The size class of a non-empty run of `len` keys: `floor(log2(len))`.
fn level_of(len: usize) -> usize {
    (usize::BITS - 1 - len.leading_zeros()) as usize
}

/// Sorts and deduplicates `pending` once it has doubled since its last
/// compaction, so a level's batch stays proportional to the distinct nodes
/// (or warps) it holds, not to the edges that reached them.
fn compact_if_doubled<T: Ord>(pending: &mut Vec<T>, compacted_len: &mut usize) {
    if pending.len() >= (*compacted_len).max(32) * 2 {
        pending.sort_unstable();
        pending.dedup();
        *compacted_len = pending.len();
    }
}

/// Fills `scratch.reachable` and `scratch.warps` with the graph reachable from
/// `root`, following edges and `Descend` portals on nodes and edges, level by
/// level. Each level's new nodes are added to the reached runs as one batch.
fn collect_reachable(state: &WarpState, root: &NodeKey, scratch: &mut StateRootScratch) {
    let StateRootScratch {
        levels,
        carry,
        merged,
        reachable,
        frontier,
        next,
        warps,
        #[cfg(test)]
        moved,
        ..
    } = scratch;
    for run in levels.iter_mut() {
        run.clear();
    }
    frontier.clear();
    next.clear();
    warps.clear();
    warps.push(root.warp_id);
    next.push(*root);

    while !next.is_empty() {
        // `next` holds this level's new nodes; add them to the reached runs,
        // carrying up while the target size class is occupied.
        next.sort_unstable();
        next.dedup();
        carry.clear();
        carry.extend_from_slice(next);
        loop {
            let level = level_of(carry.len());
            if levels.len() <= level {
                levels.resize_with(level + 1, Vec::new);
            }
            if levels[level].is_empty() {
                levels[level].extend_from_slice(carry);
                #[cfg(test)]
                {
                    *moved += carry.len();
                }
                break;
            }
            merge_sorted(&levels[level], carry, merged);
            #[cfg(test)]
            {
                *moved += merged.len();
            }
            levels[level].clear();
            std::mem::swap(carry, merged);
        }
        std::mem::swap(frontier, next);
        next.clear();

        let mut next_compacted = 0;
        let mut warps_compacted = warps.len();
        for current in frontier.iter() {
            let Some(store) = state.store(&current.warp_id) else {
                debug_assert!(
                    false,
                    "reachable traversal referenced missing warp store: {:?}",
                    current.warp_id
                );
                continue;
            };
            for edge in store.edges_from(&current.local_id) {
                let to = NodeKey {
                    warp_id: current.warp_id,
                    local_id: edge.to,
                };
                if !is_reached(levels, &to) {
                    next.push(to);
                    compact_if_doubled(next, &mut next_compacted);
                }
                if let Some(AttachmentValue::Descend(child_warp)) = store.edge_attachment(&edge.id)
                {
                    reach_descend(state, *child_warp, warps, levels, next);
                    compact_if_doubled(warps, &mut warps_compacted);
                    compact_if_doubled(next, &mut next_compacted);
                }
            }
            if let Some(AttachmentValue::Descend(child_warp)) =
                store.node_attachment(&current.local_id)
            {
                reach_descend(state, *child_warp, warps, levels, next);
                compact_if_doubled(warps, &mut warps_compacted);
                compact_if_doubled(next, &mut next_compacted);
            }
        }
    }
    warps.sort_unstable();
    warps.dedup();

    // Merge the runs, smallest first, into one sorted `reachable`.
    reachable.clear();
    for run in levels.iter() {
        if run.is_empty() {
            continue;
        }
        merge_sorted(reachable, run, merged);
        #[cfg(test)]
        {
            *moved += merged.len();
        }
        std::mem::swap(reachable, merged);
    }

    // Buffers that trade places (the two frontiers; `carry`, `merged` and
    // `reachable`) each get the largest capacity of their group, so the next
    // traversal of a graph this size allocates nothing, whichever buffer each
    // step starts in.
    frontier.clear();
    next.clear();
    // `reserve_exact`, not `reserve`: amortized growth would overshoot the
    // larger capacity, leaving the other buffer smaller again, so the buffers
    // would keep growing commit after commit.
    let capacity = frontier.capacity().max(next.capacity());
    frontier.reserve_exact(capacity);
    next.reserve_exact(capacity);
    carry.clear();
    merged.clear();
    let capacity = carry
        .capacity()
        .max(merged.capacity())
        .max(reachable.capacity());
    carry.reserve_exact(capacity);
    merged.reserve_exact(capacity);
    reachable.reserve_exact(capacity - reachable.len());
}

/// Marks `child_warp` reachable and queues its root node unless already
/// reached. `warps` may hold duplicates until traversal ends.
fn reach_descend(
    state: &WarpState,
    child_warp: WarpId,
    warps: &mut Vec<WarpId>,
    levels: &[Vec<NodeKey>],
    next: &mut Vec<NodeKey>,
) {
    warps.push(child_warp);
    let Some(child) = state.instance(&child_warp) else {
        debug_assert!(
            false,
            "descend referenced missing warp instance metadata: {child_warp:?}"
        );
        return;
    };
    let child_root = NodeKey {
        warp_id: child_warp,
        local_id: child.root_node,
    };
    if !is_reached(levels, &child_root) {
        next.push(child_root);
    }
}

/// The state root as first written, kept to prove [`compute_state_root_with`]
/// computes the same bytes.
#[cfg(test)]
fn compute_state_root_reference(state: &WarpState, root: &NodeKey) -> Hash {
    // 1) Determine reachable nodes across instances via deterministic BFS.
    let (reachable_warps, reachable_nodes) = collect_reachable_graph(state, root);

    // 2) Hash reachable instance content in canonical order.
    let mut hasher = Hasher::new();
    hasher.update(domain::STATE_ROOT_V1);
    hasher.update(&(root.warp_id).0);
    hasher.update(&(root.local_id).0);

    for warp_id in &reachable_warps {
        let Some(instance) = state.instance(warp_id) else {
            debug_assert!(false, "missing warp instance metadata: {warp_id:?}");
            continue;
        };
        let Some(store) = state.store(warp_id) else {
            debug_assert!(false, "missing warp store for instance: {warp_id:?}");
            continue;
        };

        // Instance header: bind metadata into the deterministic boundary.
        hasher.update(&(instance.warp_id).0);
        hasher.update(&(instance.root_node).0);
        hash_attachment_key_opt(&mut hasher, instance.parent.as_ref());

        // Nodes: ascending NodeId order, filtered to reachable nodes in this warp.
        for (node_id, node) in &store.nodes {
            let key = NodeKey {
                warp_id: *warp_id,
                local_id: *node_id,
            };
            if !reachable_nodes.contains(&key) {
                continue;
            }
            hasher.update(&node_id.0);
            hasher.update(&(node.ty).0);
            hash_attachment_value_opt(&mut hasher, store.node_attachment(node_id));
        }

        // Edges: per reachable source node bucket, sorted by EdgeId, filtered to reachable targets.
        for (from, edges) in &store.edges_from {
            let from_key = NodeKey {
                warp_id: *warp_id,
                local_id: *from,
            };
            if !reachable_nodes.contains(&from_key) {
                continue;
            }

            let mut sorted_edges: Vec<&EdgeRecord> = edges
                .iter()
                .filter(|e| {
                    reachable_nodes.contains(&NodeKey {
                        warp_id: *warp_id,
                        local_id: e.to,
                    })
                })
                .collect();
            sorted_edges.sort_by_key(|a| a.id.0);

            hasher.update(&from.0);
            hasher.update(&(sorted_edges.len() as u64).to_le_bytes());
            for edge in sorted_edges {
                hasher.update(&(edge.id).0);
                hasher.update(&(edge.ty).0);
                hasher.update(&(edge.to).0);
                hash_attachment_value_opt(&mut hasher, store.edge_attachment(&edge.id));
            }
        }
    }

    hasher.finalize().into()
}

/// Computes the canonical state root for a full [`WarpState`] rooted at `root`.
#[must_use]
pub(crate) fn compute_state_root_for_warp_state(state: &WarpState, root: &NodeKey) -> Hash {
    compute_state_root(state, root)
}

#[cfg(test)]
fn collect_reachable_graph(
    state: &WarpState,
    root: &NodeKey,
) -> (BTreeSet<WarpId>, BTreeSet<NodeKey>) {
    let mut reachable_nodes: BTreeSet<NodeKey> = BTreeSet::new();
    let mut reachable_warps: BTreeSet<WarpId> = BTreeSet::new();
    let mut queue: VecDeque<NodeKey> = VecDeque::new();

    reachable_nodes.insert(*root);
    reachable_warps.insert(root.warp_id);
    queue.push_back(*root);

    while let Some(current) = queue.pop_front() {
        let Some(store) = state.store(&current.warp_id) else {
            debug_assert!(
                false,
                "reachable traversal referenced missing warp store: {:?}",
                current.warp_id
            );
            continue;
        };

        for edge in store.edges_from(&current.local_id) {
            let to = NodeKey {
                warp_id: current.warp_id,
                local_id: edge.to,
            };
            if reachable_nodes.insert(to) {
                queue.push_back(to);
            }

            if let Some(AttachmentValue::Descend(child_warp)) = store.edge_attachment(&edge.id) {
                enqueue_descend(
                    state,
                    *child_warp,
                    &mut reachable_warps,
                    &mut reachable_nodes,
                    &mut queue,
                );
            }
        }

        if let Some(AttachmentValue::Descend(child_warp)) = store.node_attachment(&current.local_id)
        {
            enqueue_descend(
                state,
                *child_warp,
                &mut reachable_warps,
                &mut reachable_nodes,
                &mut queue,
            );
        }
    }

    (reachable_warps, reachable_nodes)
}

/// Computes the final commit hash from the state root and metadata digests.
///
/// This is the legacy v1 commit header hash (plan/decision/rewrites digests).
/// It is retained for reference and potential migration tooling.
pub(crate) fn _compute_commit_hash(
    state_root: &Hash,
    parents: &[Hash],
    plan_digest: &Hash,
    decision_digest: &Hash,
    rewrites_digest: &Hash,
    policy_id: u32,
) -> Hash {
    let mut h = Hasher::new();
    // Version tag for future evolution.
    h.update(&1u16.to_le_bytes());
    // Parents (length + raw bytes)
    h.update(&(parents.len() as u64).to_le_bytes());
    for p in parents {
        h.update(p);
    }
    // State root and metadata digests
    h.update(state_root);
    h.update(plan_digest);
    h.update(decision_digest);
    h.update(rewrites_digest);
    h.update(&policy_id.to_le_bytes());
    h.finalize().into()
}

/// Computes the commit hash (commit id) for commit header v2.
///
/// Commit hash v2 commits only to the replay boundary artifact: `state_root`
/// and the tick `patch_digest` (plus explicit parents and policy id).
///
/// # Parent Ordering
///
/// `parents` MUST be supplied in a deterministic, canonical order
/// (e.g., lexicographic by hash bytes). The slice is hashed exactly as
/// provided—reordering parents produces a different commit hash.
pub fn compute_commit_hash_v2(
    state_root: &Hash,
    parents: &[Hash],
    patch_digest: &Hash,
    policy_id: u32,
) -> Hash {
    let mut h = Hasher::new();
    h.update(domain::COMMIT_ID_V2);
    // Version tag for future evolution.
    h.update(&2u16.to_le_bytes());
    // Parents (length + raw bytes)
    h.update(&(parents.len() as u64).to_le_bytes());
    for p in parents {
        h.update(p);
    }
    // State root + patch digest + policy id.
    h.update(state_root);
    h.update(patch_digest);
    h.update(&policy_id.to_le_bytes());
    h.finalize().into()
}

/// Computes commit hash v3: v2's inputs plus the tick receipt digest.
///
/// A tick whose receipt carries an attribution section (#873) uses v3, so the
/// commit identity transitively binds which submissions the tick decided:
/// commit hash → receipt digest → attribution. v3 has its own domain tag and
/// version, so its bytes can never equal a v2 preimage.
pub fn compute_commit_hash_v3(
    state_root: &Hash,
    parents: &[Hash],
    patch_digest: &Hash,
    receipt_digest: &Hash,
    policy_id: u32,
) -> Hash {
    let mut h = Hasher::new();
    h.update(domain::COMMIT_ID_V3);
    h.update(&3u16.to_le_bytes());
    h.update(&(parents.len() as u64).to_le_bytes());
    for p in parents {
        h.update(p);
    }
    h.update(state_root);
    h.update(patch_digest);
    h.update(receipt_digest);
    h.update(&policy_id.to_le_bytes());
    h.finalize().into()
}

/// The commit hash of a tick, given its retained receipt.
///
/// The receipt's own format decides the version: a commit-bound receipt
/// (format 4, retained as `EPRV0003`) is committed with v3, which binds its
/// digest; any other tick keeps v2, including legacy ticks (format 2,
/// `EPRV0001`), ticks attributed before commit binding existed (format 3,
/// `EPRV0002`) and commits with no receipt. A receipt stripped of its binding
/// therefore selects v2 and cannot reproduce a v3 commit hash.
pub fn compute_commit_hash_for_receipt(
    state_root: &Hash,
    parents: &[Hash],
    patch_digest: &Hash,
    policy_id: u32,
    receipt: Option<&crate::receipt::TickReceipt>,
) -> Hash {
    match receipt.filter(|receipt| receipt.binds_commit()) {
        Some(receipt) => compute_commit_hash_v3(
            state_root,
            parents,
            patch_digest,
            &receipt.digest(),
            policy_id,
        ),
        None => compute_commit_hash_v2(state_root, parents, patch_digest, policy_id),
    }
}

// Tests for commit header encoding and hashing live under PR-09
// (branch: echo/pr-09-blake3-header-tests). Intentionally omitted here
// to keep PR-10 scope to README/docs/CI and avoid duplicate content.

#[cfg(test)]
fn enqueue_descend(
    state: &WarpState,
    child_warp: WarpId,
    reachable_warps: &mut BTreeSet<WarpId>,
    reachable_nodes: &mut BTreeSet<NodeKey>,
    queue: &mut VecDeque<NodeKey>,
) {
    reachable_warps.insert(child_warp);
    let Some(child) = state.instance(&child_warp) else {
        debug_assert!(
            false,
            "descend referenced missing warp instance metadata: {child_warp:?}"
        );
        return;
    };
    let child_root = NodeKey {
        warp_id: child_warp,
        local_id: child.root_node,
    };
    if reachable_nodes.insert(child_root) {
        queue.push_back(child_root);
    }
}

fn hash_attachment_key_opt(hasher: &mut Hasher, key: Option<&AttachmentKey>) {
    match key {
        None => {
            hasher.update(&[0u8]);
        }
        Some(key) => {
            hasher.update(&[1u8]);
            hash_attachment_key(hasher, key);
        }
    }
}

fn hash_attachment_key(hasher: &mut Hasher, key: &AttachmentKey) {
    let (owner_tag, plane_tag) = key.tag();
    hasher.update(&[owner_tag]);
    hasher.update(&[plane_tag]);
    match key.owner {
        AttachmentOwner::Node(node) => {
            hasher.update(&(node.warp_id).0);
            hasher.update(&(node.local_id).0);
        }
        AttachmentOwner::Edge(edge) => {
            hasher.update(&(edge.warp_id).0);
            hasher.update(&(edge.local_id).0);
        }
    }
}

fn hash_attachment_value_opt(hasher: &mut Hasher, value: Option<&AttachmentValue>) {
    match value {
        None => {
            hasher.update(&[0u8]);
        }
        Some(value) => {
            hasher.update(&[1u8]);
            hash_attachment_value(hasher, value);
        }
    }
}

fn hash_attachment_value(hasher: &mut Hasher, value: &AttachmentValue) {
    match value {
        AttachmentValue::Atom(atom) => {
            hasher.update(&[1u8]);
            hash_atom_payload(hasher, atom);
        }
        AttachmentValue::Descend(warp_id) => {
            hasher.update(&[2u8]);
            hasher.update(&warp_id.0);
        }
    }
}

fn hash_atom_payload(hasher: &mut Hasher, atom: &AtomPayload) {
    hasher.update(&(atom.type_id).0);
    hasher.update(&(atom.bytes.len() as u64).to_le_bytes());
    hasher.update(&atom.bytes);
}

// ─── TTD Tick Commit Hash (v2) ────────────────────────────────────────────────

use crate::worldline::WorldlineId;

/// Computes the TTD tick commit hash (v2 format).
///
/// This is the canonical commit hash for TTDR receipts. It commits to:
/// - Schema identity (binds this commit to a specific protocol version)
/// - Worldline identity (binds to a specific history branch)
/// - Tick number (ordering within the worldline)
/// - Parent commits (explicit history linkage)
/// - Patch digest (what changed)
/// - State root (resulting state, if available)
/// - Emissions digest (what was output)
/// - Op emission index digest (which ops emitted what, if tracked)
///
/// # Wire Format
///
/// ```text
/// tick_commit:v2 = BLAKE3(
///     "tick_commit:v2"           (14-byte domain separator)
///     schema_hash: [u8; 32]
///     worldline_id: [u8; 32]
///     tick: u64 (LE)
///     num_parents: u64 (LE)
///     parent_hashes[]: [u8; 32]... (pre-sorted)
///     patch_digest: [u8; 32]
///     has_state_root: u8 (0 or 1)
///     state_root?: [u8; 32]       (if present)
///     emissions_digest: [u8; 32]
///     has_op_emission_idx: u8 (0 or 1)
///     op_emission_index_digest?: [u8; 32] (if present)
/// )
/// ```
///
/// # Parent Ordering
///
/// The `parent_hashes` slice MUST be pre-sorted in ascending lexicographic order.
/// This function hashes parents exactly as provided—it does NOT re-sort internally.
/// Callers are responsible for sorting to ensure determinism.
///
/// # Relationship to `compute_commit_hash_v2`
///
/// [`compute_commit_hash_v2`] is the simpler internal commit hash used by
/// [`Snapshot`]. This function is the full TTD-aware version that includes
/// `schema_hash`, `worldline_id`, and emission digests required by TTDR receipts.
///
/// # Example
///
/// ```ignore
/// let commit_hash = compute_tick_commit_hash_v2(
///     &schema_hash,
///     &worldline_id,
///     tick,
///     &sorted_parents,
///     &patch_digest,
///     Some(&state_root),
///     &emissions_digest,
///     Some(&op_emission_index_digest),
/// );
/// ```
// Allow many arguments: this signature mirrors the current TTD receipt hash
// contract exactly. A builder pattern would obscure the wire-format
// correspondence.
#[allow(clippy::too_many_arguments)]
pub fn compute_tick_commit_hash_v2(
    schema_hash: &Hash,
    worldline_id: &WorldlineId,
    tick: u64,
    parent_hashes: &[Hash],
    patch_digest: &Hash,
    state_root: Option<&Hash>,
    emissions_digest: &Hash,
    op_emission_index_digest: Option<&Hash>,
) -> Hash {
    let mut h = Hasher::new();

    // Domain separator (matches spec: "tick_commit:v2")
    h.update(b"tick_commit:v2");

    // Schema and worldline identity
    h.update(schema_hash);
    h.update(worldline_id.as_bytes());

    // Tick number (little-endian)
    h.update(&tick.to_le_bytes());

    // Parent commits (count + hashes, pre-sorted by caller)
    h.update(&(parent_hashes.len() as u64).to_le_bytes());
    for p in parent_hashes {
        h.update(p);
    }

    // Patch digest
    h.update(patch_digest);

    // State root (optional)
    match state_root {
        Some(sr) => {
            h.update(&[1u8]);
            h.update(sr);
        }
        None => {
            h.update(&[0u8]);
        }
    }

    // Emissions digest (required)
    h.update(emissions_digest);

    // Op emission index digest (optional)
    match op_emission_index_digest {
        Some(oeid) => {
            h.update(&[1u8]);
            h.update(oeid);
        }
        None => {
            h.update(&[0u8]);
        }
    }

    h.finalize().into()
}

// ─── Emission Digests ────────────────────────────────────────────────────────

use crate::materialization::{ChannelId, FinalizedChannel};

/// Computes a deterministic digest over all finalized channel emissions.
///
/// This captures the complete set of materialized outputs for a tick in a single
/// hash that can be included in commit verification.
///
/// # Ordering
///
/// Emissions are hashed in canonical order:
/// 1. Channels sorted by [`ChannelId`] (lexicographic over bytes)
/// 2. For each channel: the complete finalized data blob
///
/// # Wire Format
///
/// ```text
/// emissions_digest = BLAKE3(
///     version: u16 (LE)
///     num_channels: u64 (LE)
///     for each channel (sorted by channel_id):
///         channel_id: [u8; 32]
///         data_len: u64 (LE)
///         data: [u8; data_len]
/// )
/// ```
///
/// # Usage
///
/// ```ignore
/// let report = bus.finalize();
/// let digest = compute_emissions_digest(&report.channels);
/// ```
pub fn compute_emissions_digest(channels: &[FinalizedChannel]) -> Hash {
    let mut h = Hasher::new();

    // Version tag for future evolution
    h.update(&1u16.to_le_bytes());

    // Sort channels by ChannelId for deterministic ordering
    let mut sorted: Vec<_> = channels.iter().collect();
    sorted.sort_by_key(|a| a.channel.0);

    // Number of channels
    h.update(&(sorted.len() as u64).to_le_bytes());

    // Hash each channel's emissions
    for fc in sorted {
        h.update(&fc.channel.0);
        h.update(&(fc.data.len() as u64).to_le_bytes());
        h.update(&fc.data);
    }

    h.finalize().into()
}

/// Entry mapping an operation to its emission indices.
///
/// This is used by [`compute_op_emission_index_digest`] to track which
/// operations triggered which channel emissions within a tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpEmissionEntry {
    /// The operation ID (opcode hash) that triggered emissions.
    pub op_id: Hash,
    /// Channel IDs that received emissions from this op.
    pub channels: Vec<ChannelId>,
}

/// Computes a digest mapping operations to their emission channels.
///
/// This enables compliance verification: proving that an operation emitted
/// exactly the channels it was supposed to (no more, no less).
///
/// # Ordering
///
/// Entries are hashed in canonical order:
/// 1. Operations sorted by `op_id` (lexicographic over bytes)
/// 2. For each op: channels sorted by `ChannelId`
///
/// # Wire Format
///
/// ```text
/// op_emission_index_digest = BLAKE3(
///     version: u16 (LE)
///     num_ops: u64 (LE)
///     for each op (sorted by op_id):
///         op_id: [u8; 32]
///         num_channels: u64 (LE)
///         for each channel (sorted by channel_id):
///             channel_id: [u8; 32]
/// )
/// ```
///
/// # Usage
///
/// ```ignore
/// let entries = vec![
///     OpEmissionEntry {
///         op_id: op_hash,
///         channels: vec![channel_a, channel_b],
///     },
/// ];
/// let digest = compute_op_emission_index_digest(&entries);
/// ```
pub fn compute_op_emission_index_digest(entries: &[OpEmissionEntry]) -> Hash {
    let mut h = Hasher::new();

    // Version tag for future evolution
    h.update(&1u16.to_le_bytes());

    // Sort entries by op_id for deterministic ordering
    let mut sorted: Vec<_> = entries.iter().collect();
    sorted.sort_by_key(|a| a.op_id);

    // Number of ops
    h.update(&(sorted.len() as u64).to_le_bytes());

    // Hash each op's emission index
    for entry in sorted {
        h.update(&entry.op_id);

        // Sort channels for this op
        let mut channels: Vec<_> = entry.channels.iter().collect();
        channels.sort_by_key(|a| a.0);

        h.update(&(channels.len() as u64).to_le_bytes());
        for ch in channels {
            h.update(&ch.0);
        }
    }

    h.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::materialization::make_channel_id;

    fn make_hash(n: u8) -> Hash {
        let mut h = [0u8; 32];
        h[0] = n;
        h
    }

    #[test]
    fn emissions_digest_deterministic_ordering() {
        // Same emissions in different order should produce same digest
        let ch_a = make_channel_id("channel:a");
        let ch_b = make_channel_id("channel:b");

        let channels_order1 = vec![
            FinalizedChannel {
                channel: ch_a,
                data: vec![1, 2, 3],
            },
            FinalizedChannel {
                channel: ch_b,
                data: vec![4, 5],
            },
        ];

        let channels_order2 = vec![
            FinalizedChannel {
                channel: ch_b,
                data: vec![4, 5],
            },
            FinalizedChannel {
                channel: ch_a,
                data: vec![1, 2, 3],
            },
        ];

        let digest1 = compute_emissions_digest(&channels_order1);
        let digest2 = compute_emissions_digest(&channels_order2);

        assert_eq!(
            digest1, digest2,
            "emissions_digest should be order-independent"
        );
    }

    #[test]
    fn emissions_digest_empty() {
        let digest = compute_emissions_digest(&[]);
        // Should still produce a valid (non-zero) digest
        assert_ne!(digest, [0u8; 32]);
    }

    #[test]
    fn emissions_digest_content_sensitive() {
        let ch = make_channel_id("test:channel");

        let channels_a = vec![FinalizedChannel {
            channel: ch,
            data: vec![1, 2, 3],
        }];

        let channels_b = vec![FinalizedChannel {
            channel: ch,
            data: vec![1, 2, 4], // Different data
        }];

        let digest_a = compute_emissions_digest(&channels_a);
        let digest_b = compute_emissions_digest(&channels_b);

        assert_ne!(
            digest_a, digest_b,
            "different data should produce different digest"
        );
    }

    #[test]
    fn op_emission_index_digest_deterministic() {
        let op_a = make_hash(1);
        let op_b = make_hash(2);
        let ch_x = make_channel_id("channel:x");
        let ch_y = make_channel_id("channel:y");

        let entries_order1 = vec![
            OpEmissionEntry {
                op_id: op_a,
                channels: vec![ch_x, ch_y],
            },
            OpEmissionEntry {
                op_id: op_b,
                channels: vec![ch_x],
            },
        ];

        let entries_order2 = vec![
            OpEmissionEntry {
                op_id: op_b,
                channels: vec![ch_x],
            },
            OpEmissionEntry {
                op_id: op_a,
                channels: vec![ch_y, ch_x], // Channels also reordered
            },
        ];

        let digest1 = compute_op_emission_index_digest(&entries_order1);
        let digest2 = compute_op_emission_index_digest(&entries_order2);

        assert_eq!(
            digest1, digest2,
            "op_emission_index_digest should be order-independent"
        );
    }

    #[test]
    fn op_emission_index_digest_empty() {
        let digest = compute_op_emission_index_digest(&[]);
        assert_ne!(digest, [0u8; 32]);
    }

    #[test]
    fn op_emission_index_different_channels_different_digest() {
        let op = make_hash(1);
        let ch_x = make_channel_id("channel:x");
        let ch_y = make_channel_id("channel:y");

        let entries_a = vec![OpEmissionEntry {
            op_id: op,
            channels: vec![ch_x],
        }];

        let entries_b = vec![OpEmissionEntry {
            op_id: op,
            channels: vec![ch_y], // Different channel
        }];

        let digest_a = compute_op_emission_index_digest(&entries_a);
        let digest_b = compute_op_emission_index_digest(&entries_b);

        assert_ne!(
            digest_a, digest_b,
            "different channels should produce different digest"
        );
    }

    // ─── Tick Commit Hash v2 Tests ───────────────────────────────────────────────

    use crate::worldline::WorldlineId;

    fn make_worldline_id(n: u8) -> WorldlineId {
        WorldlineId::from_bytes(make_hash(n))
    }

    #[test]
    fn tick_commit_hash_v2_basic() {
        let schema_hash = make_hash(1);
        let worldline_id = make_worldline_id(2);
        let tick = 42u64;
        let parents = vec![make_hash(10), make_hash(11)];
        let patch_digest = make_hash(20);
        let state_root = make_hash(30);
        let emissions_digest = make_hash(40);
        let op_emission_index = make_hash(50);

        let hash = compute_tick_commit_hash_v2(
            &schema_hash,
            &worldline_id,
            tick,
            &parents,
            &patch_digest,
            Some(&state_root),
            &emissions_digest,
            Some(&op_emission_index),
        );

        // Should produce a valid non-zero hash
        assert_ne!(hash, [0u8; 32]);
    }

    #[test]
    fn tick_commit_hash_v2_deterministic() {
        let schema_hash = make_hash(1);
        let worldline_id = make_worldline_id(2);
        let tick = 100u64;
        let parents = vec![make_hash(10)];
        let patch_digest = make_hash(20);
        let emissions_digest = make_hash(40);

        // Compute twice with identical inputs
        let hash1 = compute_tick_commit_hash_v2(
            &schema_hash,
            &worldline_id,
            tick,
            &parents,
            &patch_digest,
            None,
            &emissions_digest,
            None,
        );

        let hash2 = compute_tick_commit_hash_v2(
            &schema_hash,
            &worldline_id,
            tick,
            &parents,
            &patch_digest,
            None,
            &emissions_digest,
            None,
        );

        assert_eq!(
            hash1, hash2,
            "identical inputs should produce identical hash"
        );
    }

    #[test]
    fn tick_commit_hash_v2_schema_sensitive() {
        let worldline_id = make_worldline_id(2);
        let tick = 1u64;
        let parents: Vec<Hash> = vec![];
        let patch_digest = make_hash(20);
        let emissions_digest = make_hash(40);

        let hash_a = compute_tick_commit_hash_v2(
            &make_hash(1), // schema A
            &worldline_id,
            tick,
            &parents,
            &patch_digest,
            None,
            &emissions_digest,
            None,
        );

        let hash_b = compute_tick_commit_hash_v2(
            &make_hash(2), // schema B (different)
            &worldline_id,
            tick,
            &parents,
            &patch_digest,
            None,
            &emissions_digest,
            None,
        );

        assert_ne!(
            hash_a, hash_b,
            "different schema should produce different hash"
        );
    }

    #[test]
    fn tick_commit_hash_v2_worldline_sensitive() {
        let schema_hash = make_hash(1);
        let tick = 1u64;
        let parents: Vec<Hash> = vec![];
        let patch_digest = make_hash(20);
        let emissions_digest = make_hash(40);

        let hash_a = compute_tick_commit_hash_v2(
            &schema_hash,
            &make_worldline_id(1), // worldline A
            tick,
            &parents,
            &patch_digest,
            None,
            &emissions_digest,
            None,
        );

        let hash_b = compute_tick_commit_hash_v2(
            &schema_hash,
            &make_worldline_id(2), // worldline B (different)
            tick,
            &parents,
            &patch_digest,
            None,
            &emissions_digest,
            None,
        );

        assert_ne!(
            hash_a, hash_b,
            "different worldline should produce different hash"
        );
    }

    #[test]
    fn tick_commit_hash_v2_tick_sensitive() {
        let schema_hash = make_hash(1);
        let worldline_id = make_worldline_id(2);
        let parents: Vec<Hash> = vec![];
        let patch_digest = make_hash(20);
        let emissions_digest = make_hash(40);

        let hash_a = compute_tick_commit_hash_v2(
            &schema_hash,
            &worldline_id,
            1, // tick 1
            &parents,
            &patch_digest,
            None,
            &emissions_digest,
            None,
        );

        let hash_b = compute_tick_commit_hash_v2(
            &schema_hash,
            &worldline_id,
            2, // tick 2 (different)
            &parents,
            &patch_digest,
            None,
            &emissions_digest,
            None,
        );

        assert_ne!(
            hash_a, hash_b,
            "different tick should produce different hash"
        );
    }

    #[test]
    fn tick_commit_hash_v2_parent_order_matters() {
        let schema_hash = make_hash(1);
        let worldline_id = make_worldline_id(2);
        let tick = 1u64;
        let patch_digest = make_hash(20);
        let emissions_digest = make_hash(40);

        // Parents in order [A, B]
        let parents_ab = vec![make_hash(10), make_hash(11)];
        // Parents in order [B, A] (reversed)
        let parents_ba = vec![make_hash(11), make_hash(10)];

        let hash_ab = compute_tick_commit_hash_v2(
            &schema_hash,
            &worldline_id,
            tick,
            &parents_ab,
            &patch_digest,
            None,
            &emissions_digest,
            None,
        );

        let hash_ba = compute_tick_commit_hash_v2(
            &schema_hash,
            &worldline_id,
            tick,
            &parents_ba,
            &patch_digest,
            None,
            &emissions_digest,
            None,
        );

        // Parents are NOT re-sorted internally, so order matters
        // Caller is responsible for canonical ordering
        assert_ne!(hash_ab, hash_ba, "parent order matters (caller must sort)");
    }

    #[test]
    fn tick_commit_hash_v2_state_root_presence_matters() {
        let schema_hash = make_hash(1);
        let worldline_id = make_worldline_id(2);
        let tick = 1u64;
        let parents: Vec<Hash> = vec![];
        let patch_digest = make_hash(20);
        let emissions_digest = make_hash(40);
        let state_root = make_hash(30);

        let hash_with_sr = compute_tick_commit_hash_v2(
            &schema_hash,
            &worldline_id,
            tick,
            &parents,
            &patch_digest,
            Some(&state_root),
            &emissions_digest,
            None,
        );

        let hash_without_sr = compute_tick_commit_hash_v2(
            &schema_hash,
            &worldline_id,
            tick,
            &parents,
            &patch_digest,
            None, // no state root
            &emissions_digest,
            None,
        );

        assert_ne!(
            hash_with_sr, hash_without_sr,
            "state_root presence should affect hash"
        );
    }

    #[test]
    fn tick_commit_hash_v2_emissions_digest_sensitive() {
        let schema_hash = make_hash(1);
        let worldline_id = make_worldline_id(2);
        let tick = 1u64;
        let parents: Vec<Hash> = vec![];
        let patch_digest = make_hash(20);

        let hash_a = compute_tick_commit_hash_v2(
            &schema_hash,
            &worldline_id,
            tick,
            &parents,
            &patch_digest,
            None,
            &make_hash(40), // emissions A
            None,
        );

        let hash_b = compute_tick_commit_hash_v2(
            &schema_hash,
            &worldline_id,
            tick,
            &parents,
            &patch_digest,
            None,
            &make_hash(41), // emissions B (different)
            None,
        );

        assert_ne!(
            hash_a, hash_b,
            "different emissions_digest should produce different hash"
        );
    }

    #[test]
    fn tick_commit_hash_v2_op_emission_index_presence_matters() {
        let schema_hash = make_hash(1);
        let worldline_id = make_worldline_id(2);
        let tick = 1u64;
        let parents: Vec<Hash> = vec![];
        let patch_digest = make_hash(20);
        let emissions_digest = make_hash(40);
        let op_emission_index = make_hash(50);

        let hash_with_oei = compute_tick_commit_hash_v2(
            &schema_hash,
            &worldline_id,
            tick,
            &parents,
            &patch_digest,
            None,
            &emissions_digest,
            Some(&op_emission_index),
        );

        let hash_without_oei = compute_tick_commit_hash_v2(
            &schema_hash,
            &worldline_id,
            tick,
            &parents,
            &patch_digest,
            None,
            &emissions_digest,
            None, // no op emission index
        );

        assert_ne!(
            hash_with_oei, hash_without_oei,
            "op_emission_index_digest presence should affect hash"
        );
    }

    #[test]
    fn tick_commit_hash_v2_empty_parents_valid() {
        let schema_hash = make_hash(1);
        let worldline_id = make_worldline_id(2);
        let tick = 0u64; // genesis tick
        let parents: Vec<Hash> = vec![]; // no parents (genesis)
        let patch_digest = make_hash(20);
        let emissions_digest = make_hash(40);

        let hash = compute_tick_commit_hash_v2(
            &schema_hash,
            &worldline_id,
            tick,
            &parents,
            &patch_digest,
            None,
            &emissions_digest,
            None,
        );

        // Should produce a valid non-zero hash
        assert_ne!(hash, [0u8; 32]);
    }

    #[test]
    fn tick_commit_hash_v2_golden_vector() {
        // Golden vector test: ensures the hash algorithm doesn't change accidentally.
        // If this test fails after code changes, the wire format has changed and
        // requires a version bump or migration plan.

        let schema_hash = [0xABu8; 32];
        let worldline_id = WorldlineId::from_bytes([0xCDu8; 32]);
        let tick = 42u64;
        let parent = [0x11u8; 32];
        let patch_digest = [0x22u8; 32];
        let state_root = [0x33u8; 32];
        let emissions_digest = [0x44u8; 32];
        let op_emission_index = [0x55u8; 32];

        let hash = compute_tick_commit_hash_v2(
            &schema_hash,
            &worldline_id,
            tick,
            &[parent],
            &patch_digest,
            Some(&state_root),
            &emissions_digest,
            Some(&op_emission_index),
        );

        // This is the expected hash for the above inputs.
        // Computed once and recorded here. If this changes, the wire format changed.
        let expected_hex = "a83bc43c4c35757493c95eaa14ba1c08403f94f02a3955b0c05b73a1af3618bf";
        let actual_hex = hex::encode(hash);

        assert_eq!(
            actual_hex, expected_hex,
            "golden vector mismatch - wire format may have changed!\nexpected: {expected_hex}\nactual:   {actual_hex}"
        );
    }
}

#[cfg(test)]
mod state_root_equivalence {
    use bytes::Bytes;
    use proptest::prelude::*;

    use super::{compute_state_root_reference, compute_state_root_with, StateRootScratch};
    use crate::attachment::{AtomPayload, AttachmentValue};
    use crate::graph::GraphStore;
    use crate::ident::{make_edge_id, make_node_id, make_type_id, make_warp_id, NodeKey, WarpId};
    use crate::record::{EdgeRecord, NodeRecord};
    use crate::warp_state::{WarpInstance, WarpState};

    const WARPS: u8 = 3;

    #[derive(Debug, Clone)]
    enum Write {
        Node { warp: u8, node: u8 },
        Edge { warp: u8, id: u8, from: u8, to: u8 },
        Atom { warp: u8, node: u8, bytes: u8 },
        NodeDescend { warp: u8, node: u8, child: u8 },
        EdgeDescend { warp: u8, id: u8, child: u8 },
    }

    fn warp(w: u8) -> WarpId {
        make_warp_id(&format!("state-root-eq/warp/{w}"))
    }

    fn node(w: u8, n: u8) -> crate::ident::NodeId {
        make_node_id(&format!("state-root-eq/{w}/node/{n}"))
    }

    fn edge(w: u8, e: u8) -> crate::ident::EdgeId {
        make_edge_id(&format!("state-root-eq/{w}/edge/{e}"))
    }

    fn write() -> impl Strategy<Value = Write> {
        let w = 0..WARPS;
        let n = 0u8..10;
        prop_oneof![
            (w.clone(), n.clone()).prop_map(|(warp, node)| Write::Node { warp, node }),
            (w.clone(), 0u8..16, n.clone(), n.clone())
                .prop_map(|(warp, id, from, to)| Write::Edge { warp, id, from, to }),
            (w.clone(), n.clone(), 0u8..3).prop_map(|(warp, node, bytes)| Write::Atom {
                warp,
                node,
                bytes
            }),
            (w.clone(), n, 1..WARPS).prop_map(|(warp, node, child)| Write::NodeDescend {
                warp,
                node,
                child
            }),
            (w, 0u8..16, 1..WARPS).prop_map(|(warp, id, child)| Write::EdgeDescend {
                warp,
                id,
                child
            }),
        ]
    }

    /// A state of `WARPS` instances, each rooted at its node 0, after
    /// `writes`. Edges may dangle, portals may form cycles, and warps may be
    /// unreachable.
    fn state(writes: &[Write]) -> WarpState {
        let ty = make_type_id("state-root-eq/ty");
        let mut stores: Vec<GraphStore> = (0..WARPS)
            .map(|w| {
                let mut store = GraphStore::new(warp(w));
                store.insert_node(node(w, 0), NodeRecord { ty });
                store
            })
            .collect();
        for write in writes {
            match *write {
                Write::Node { warp: w, node: n } => {
                    stores[usize::from(w)].insert_node(node(w, n), NodeRecord { ty });
                }
                Write::Edge {
                    warp: w,
                    id,
                    from,
                    to,
                } => stores[usize::from(w)].insert_edge(
                    node(w, from),
                    EdgeRecord {
                        id: edge(w, id),
                        from: node(w, from),
                        to: node(w, to),
                        ty,
                    },
                ),
                Write::Atom {
                    warp: w,
                    node: n,
                    bytes,
                } => stores[usize::from(w)].set_node_attachment(
                    node(w, n),
                    Some(AttachmentValue::Atom(AtomPayload::new(
                        ty,
                        Bytes::from(vec![bytes; usize::from(bytes)]),
                    ))),
                ),
                Write::NodeDescend {
                    warp: w,
                    node: n,
                    child,
                } => stores[usize::from(w)]
                    .set_node_attachment(node(w, n), Some(AttachmentValue::Descend(warp(child)))),
                Write::EdgeDescend { warp: w, id, child } => stores[usize::from(w)]
                    .set_edge_attachment(edge(w, id), Some(AttachmentValue::Descend(warp(child)))),
            }
        }
        let mut state = WarpState::new();
        for (w, store) in (0..WARPS).zip(stores) {
            state.upsert_instance(
                WarpInstance {
                    warp_id: warp(w),
                    root_node: node(w, 0),
                    parent: None,
                },
                store,
            );
        }
        state
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(2048))]

        /// The scratch-based state root hashes exactly the bytes the original
        /// does, including when one scratch is reused across different states.
        #[test]
        fn scratch_state_root_matches_the_reference(
            first in proptest::collection::vec(write(), 0..48),
            second in proptest::collection::vec(write(), 0..48),
        ) {
            let root = NodeKey { warp_id: warp(0), local_id: node(0, 0) };
            let mut scratch = StateRootScratch::default();
            for writes in [&first, &second, &first] {
                let state = state(writes);
                prop_assert_eq!(
                    compute_state_root_with(&state, &root, &mut scratch),
                    compute_state_root_reference(&state, &root)
                );
            }
        }
    }
}

#[cfg(test)]
mod reachability_work {
    use super::{compute_state_root_with, StateRootScratch};
    use crate::graph::GraphStore;
    use crate::ident::{make_edge_id, make_node_id, make_type_id, make_warp_id, NodeKey};
    use crate::record::{EdgeRecord, NodeRecord};
    use crate::warp_state::{WarpInstance, WarpState};

    /// Reaching a chain of `n` nodes moves O(n log n) keys, not O(n^2): the
    /// traversal never re-copies everything reached so far once per level.
    #[test]
    fn reaching_a_deep_chain_moves_n_log_n_keys() {
        const N: usize = 4_096;
        let warp_id = make_warp_id("reachability-work/warp");
        let ty = make_type_id("reachability-work/ty");
        let node = |i: usize| make_node_id(&format!("reachability-work/node/{i}"));
        let mut store = GraphStore::new(warp_id);
        for i in 0..N {
            store.insert_node(node(i), NodeRecord { ty });
        }
        for i in 1..N {
            store.insert_edge(
                node(i - 1),
                EdgeRecord {
                    id: make_edge_id(&format!("reachability-work/edge/{i}")),
                    from: node(i - 1),
                    to: node(i),
                    ty,
                },
            );
        }
        let mut state = WarpState::new();
        state.upsert_instance(
            WarpInstance {
                warp_id,
                root_node: node(0),
                parent: None,
            },
            store,
        );
        let root = NodeKey {
            warp_id,
            local_id: node(0),
        };
        let mut scratch = StateRootScratch::default();
        let _ = compute_state_root_with(&state, &root, &mut scratch);
        let bound = 4 * N * (usize::BITS - N.leading_zeros()) as usize;
        assert!(
            scratch.moved <= bound,
            "reaching a {N}-node chain moved {} keys; O(n log n) allows {bound}",
            scratch.moved
        );
    }
}
