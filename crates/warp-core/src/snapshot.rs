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
use crate::domain;
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

// Tests for commit header encoding and hashing live under PR-09
// (branch: echo/pr-09-blake3-header-tests). Intentionally omitted here
// to keep PR-10 scope to README/docs/CI and avoid duplicate content.

/// Computes the state root hash for a single warp's [`GraphStore`].
///
/// This is a **low-level** function intended for cursor replay verification and
/// provenance checkpoint validation. Most callers should use higher-level APIs
/// such as [`PlaybackCursor::seek_to`] which invokes this internally.
///
/// # When to Use
///
/// Use this function directly only when:
/// - Building a custom provenance store that records per-tick state roots
/// - Implementing checkpoint-based fast-seek outside the standard cursor flow
/// - Validating graph state integrity in test harnesses
///
/// # Determinism
///
/// The hash uses the same canonical ordering scheme as the full state root:
/// - Nodes are visited in ascending `NodeId` order
/// - Edges are sorted by `EdgeId` before hashing
/// - Attachments are hashed alongside their owners
///
/// The `warp_id` parameter is accepted for API forward-compatibility but is not
/// currently incorporated into the hash. The returned hash is purely a function
/// of the store's graph content.
///
/// # Relationship to `compute_state_root`
///
/// This is a simpler, single-warp variant of the full `compute_state_root` which
/// operates on the multi-warp [`WarpState`]. It is used for warp-local cursor
/// verification where multi-warp traversal is not required.
///
/// [`GraphStore`]: crate::graph::GraphStore
/// [`PlaybackCursor::seek_to`]: crate::playback::PlaybackCursor::seek_to
/// [`WarpState`]: crate::warp_state::WarpState
pub fn compute_state_root_for_warp_store(
    store: &crate::graph::GraphStore,
    _warp_id: WarpId,
) -> Hash {
    // Use the existing canonical_state_hash implementation from GraphStore,
    // which already provides deterministic hashing with proper ordering.
    // This ensures consistency with the existing hash scheme.
    //
    // Note: warp_id is kept as a parameter for API consistency and future use
    // when we need to incorporate warp identity into the hash computation.
    store.canonical_state_hash()
}

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
// Allow many arguments: this signature matches the TTD spec (docs/plans/ttd-app.md §3.3)
// exactly. A builder pattern would obscure the wire format correspondence.
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
    sorted.sort_by(|a, b| a.channel.0.cmp(&b.channel.0));

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
    sorted.sort_by(|a, b| a.op_id.cmp(&b.op_id));

    // Number of ops
    h.update(&(sorted.len() as u64).to_le_bytes());

    // Hash each op's emission index
    for entry in sorted {
        h.update(&entry.op_id);

        // Sort channels for this op
        let mut channels: Vec<_> = entry.channels.iter().collect();
        channels.sort_by(|a, b| a.0.cmp(&b.0));

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
        WorldlineId(make_hash(n))
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
        let worldline_id = WorldlineId([0xCDu8; 32]);
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
