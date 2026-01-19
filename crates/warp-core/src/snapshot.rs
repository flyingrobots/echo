// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! Snapshot type and hash computation.
//!
//! See the high-level spec in `docs/spec-merkle-commit.md` for precise
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
use std::collections::{BTreeSet, VecDeque};

use blake3::Hasher;

use crate::attachment::{AtomPayload, AttachmentKey, AttachmentOwner, AttachmentValue};
use crate::ident::{Hash, NodeKey, WarpId};
use crate::record::EdgeRecord;
use crate::tx::TxId;
use crate::warp_state::WarpState;

/// Snapshot returned after a successful commit.
///
/// The `hash` field is a deterministic commit hash (`commit_id`) computed from
/// `state_root` (graph-only hash) and commit metadata (parents, digests,
/// policy). Parents are explicit to support merges.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

#[cfg(feature = "serde")]
impl Snapshot {
    /// Returns the commit hash as a lowercase hex string.
    #[must_use]
    pub fn hash_hex(&self) -> String {
        hex::encode(self.hash)
    }
}

/// Computes the canonical state root hash (graph only).
pub(crate) fn compute_state_root(state: &WarpState, root: &NodeKey) -> Hash {
    // 1) Determine reachable nodes across instances via deterministic BFS:
    // - follow skeleton edges within an instance
    // - follow any `Descend(WarpId)` attachments on reachable nodes/edges
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

    // 2) Hash reachable instance content in canonical order.
    let mut hasher = Hasher::new();
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
            sorted_edges.sort_by(|a, b| a.id.0.cmp(&b.id.0));

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
pub(crate) fn compute_commit_hash_v2(
    state_root: &Hash,
    parents: &[Hash],
    patch_digest: &Hash,
    policy_id: u32,
) -> Hash {
    let mut h = Hasher::new();
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

// Tests for commit header encoding and hashing live under PR-09
// (branch: echo/pr-09-blake3-header-tests). Intentionally omitted here
// to keep PR-10 scope to README/docs/CI and avoid duplicate content.

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
