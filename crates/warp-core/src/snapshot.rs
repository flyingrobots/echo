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
/// **Invariant: the state root is independent of scratch history.** For the
/// same state and root, every history of this scratch yields the root the
/// ordered reference walk yields: fresh, warmed, after a larger graph, after
/// collisions or a fallback, and across generation rollover. Nothing in the
/// scratch is an input to state identity.
///
/// **Memory.** The buffers are reused, not bounded: they keep the peak size
/// they reach (about 144 bytes per reachable node: the discovery list, the
/// sorted list, and two table slots per node) for the engine's lifetime, and
/// are reclaimed only when the engine is dropped. Once warm, a commit's state
/// root allocates nothing, except when the membership table falls back to the
/// ordered walk (ids that collide pathologically, or more reachable nodes than
/// a `u32` position can name), which allocates as the walk always has.
///
/// Traversal finds each node once: `found` lists reached nodes in discovery
/// order and doubles as the BFS queue, and `index` answers membership in one
/// probe. `index` is an open-addressing table of positions in `found`, keyed
/// by a deterministic mix of every byte of the node key. It is never iterated, so its layout
/// cannot reach any output; the canonical order comes from sorting `found`
/// into `reachable` once traversal ends. Traversal is O(edges), and the sort
/// O(n log n), whatever the graph's depth.
#[derive(Default)]
pub(crate) struct StateRootScratch {
    /// Reached nodes, in discovery order; also the BFS queue.
    found: Vec<NodeKey>,
    /// Membership table over positions in `found`.
    index: MembershipTable,
    /// Every reachable node, sorted, once traversal ends.
    reachable: Vec<NodeKey>,
    /// Every reachable warp, sorted once traversal ends.
    warps: Vec<WarpId>,
    /// One bucket's reachable edges, as positions in the bucket.
    edge_order: Vec<usize>,
    /// Membership probes made, so a test can bound the traversal's work.
    #[cfg(test)]
    probes: usize,
    /// Table slots cleared to start traversals, so a test can bound it.
    #[cfg(test)]
    cleared: usize,
    /// Whether the last traversal fell back to the ordered walk.
    #[cfg(test)]
    fell_back: bool,
    /// Warps listed during traversal, before deduplication.
    #[cfg(test)]
    warp_pushes: usize,
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
    let table_complete = collect_reachable(state, root, scratch);
    if !table_complete {
        // Ids that collide pathologically in the membership table: take the
        // ordered walk instead, which reaches the same set (and allocates).
        let (ordered_warps, ordered_nodes) = collect_reachable_graph(state, root);
        scratch.warps.clear();
        scratch.warps.extend(ordered_warps);
        scratch.reachable.clear();
        scratch.reachable.extend(ordered_nodes);
    }
    #[cfg(test)]
    {
        scratch.fell_back = !table_complete;
    }
    let StateRootScratch {
        found,
        index: membership,
        reachable,
        warps,
        edge_order,
        ..
    } = scratch;
    // One probe per edge target when the table holds this traversal, a
    // binary search in the sorted set after a fallback.
    let is_reachable = |key: &NodeKey| {
        if table_complete {
            membership.contains(found, key)
        } else {
            reachable.binary_search(key).is_ok()
        }
    };

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
            edge_order.extend((0..edges.len()).filter(|&position| {
                is_reachable(&NodeKey {
                    warp_id: *warp_id,
                    local_id: edges[position].to,
                })
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

/// The state root's membership accelerator, kept in its own module so the
/// compiler enforces what the determinism waiver promises: its slot arrays
/// are private here, and the only operations outside it are starting a
/// traversal, inserting a key, and testing membership. There is no iterator
/// and no slice accessor, so no caller can observe the table's layout.
mod membership {
    use crate::ident::NodeKey;

    /// The table slot where `key`'s probe sequence starts. Every byte of the key
    /// is mixed in: node ids are conventionally BLAKE3 output, but nothing
    /// requires it, and ids that share leading bytes must not share a slot.
    fn home_slot(key: &NodeKey, mask: usize) -> usize {
        let mut hash: u64 = 0x9E37_79B9_7F4A_7C15;
        for word in key
            .warp_id
            .0
            .chunks_exact(8)
            .chain(key.local_id.0.chunks_exact(8))
        {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(word);
            hash = (hash ^ u64::from_le_bytes(bytes)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            hash ^= hash >> 31;
        }
        hash = (hash ^ (hash >> 30)).wrapping_mul(0x94D0_49BB_1331_11EB);
        hash ^= hash >> 31;
        // Truncation is the point: only the low bits select a slot.
        #[allow(clippy::cast_possible_truncation)]
        let slot = hash as usize;
        slot & mask
    }

    /// Open-addressing membership over positions in `found`. Each slot carries
    /// the generation of the traversal that wrote it, and a slot from an earlier
    /// generation counts as empty. Starting a traversal is one increment, never a
    /// pass over the table, so a commit pays for the graph it reaches now, not
    /// for the largest graph the table ever held. Its length is a power of two,
    /// at least twice `found`'s.
    #[derive(Default)]
    pub(super) struct MembershipTable {
        /// The generation that wrote each slot.
        generations: Vec<u32>,
        /// One plus a position in `found`, valid when the slot's generation is
        /// current.
        tags: Vec<u32>,
        /// The current traversal's generation; never `0`, which marks a slot
        /// never written.
        generation: u32,
        /// Sends every key to slot `0`, so a test can drive the collision
        /// fallback.
        #[cfg(test)]
        force_collisions: bool,
        /// Lowers the position limit, so a test can drive the representability
        /// fallback without billions of nodes.
        #[cfg(test)]
        position_limit: Option<usize>,
    }

    /// The longest probe run the membership table accepts. Honest ids, BLAKE3
    /// output by convention, almost never run past a handful of slots at load
    /// one half. Ids chosen to collide can, and then the traversal falls back to
    /// the ordered walk rather than probing quadratically.
    pub(super) const MAX_PROBE_RUN: usize = 64;

    impl MembershipTable {
        /// Starts a traversal: every slot written before reads as empty. On the
        /// counter's wraparound, once in `2^32` traversals, the table is cleared.
        /// Returns the number of slots cleared.
        pub(super) fn begin(&mut self) -> usize {
            if let Some(generation) = self.generation.checked_add(1) {
                self.generation = generation;
                return 0;
            }
            self.generations.fill(0);
            self.generation = 1;
            self.generations.len()
        }

        // `self` matters only to the test-only collision switch.
        #[cfg_attr(not(test), allow(clippy::unused_self))]
        fn home(&self, key: &NodeKey, mask: usize) -> usize {
            #[cfg(test)]
            if self.force_collisions {
                return 0;
            }
            home_slot(key, mask)
        }

        /// Whether `key` was inserted in this traversal.
        pub(super) fn contains(&self, found: &[NodeKey], key: &NodeKey) -> bool {
            if self.tags.is_empty() {
                return false;
            }
            let mask = self.tags.len() - 1;
            let mut slot = self.home(key, mask);
            while self.is_current(slot) {
                if found[(self.tags[slot] - 1) as usize] == *key {
                    return true;
                }
                slot = (slot + 1) & mask;
            }
            false
        }

        fn is_current(&self, slot: usize) -> bool {
            self.generations[slot] == self.generation
        }

        /// The table's encoding of `position` (one plus it), or `None` when the
        /// position cannot be named: the table never aliases two positions, and
        /// the caller falls back to the ordered walk instead.
        // `self` matters only to the test-only position limit.
        #[cfg_attr(not(test), allow(clippy::unused_self))]
        fn tag_for(&self, position: usize) -> Option<u32> {
            #[cfg(test)]
            if self.position_limit.is_some_and(|limit| position >= limit) {
                return None;
            }
            u32::try_from(position.checked_add(1)?).ok()
        }

        fn set(&mut self, slot: usize, tag: u32) {
            self.generations[slot] = self.generation;
            self.tags[slot] = tag;
        }

        /// Adds `key` to `found` unless it is already there, and reports whether
        /// it was new. One probe sequence per call; the table grows, rehashing
        /// from `found`, only when `found` would exceed half its length. Returns
        /// `None` when a probe run passes [`MAX_PROBE_RUN`] (the keys collide
        /// pathologically) or a position cannot be named: the caller must then
        /// fall back to the ordered walk.
        pub(super) fn insert(
            &mut self,
            found: &mut Vec<NodeKey>,
            key: NodeKey,
            #[cfg(test)] probes: &mut usize,
        ) -> Option<bool> {
            if (found.len() + 1) * 2 > self.tags.len() {
                let length = ((found.len() + 1) * 2).next_power_of_two().max(64);
                self.generations.clear();
                self.generations.resize(length, 0);
                self.tags.clear();
                self.tags.resize(length, 0);
                let mask = length - 1;
                for (position, existing) in found.iter().enumerate() {
                    let mut slot = self.home(existing, mask);
                    let mut run = 0;
                    while self.is_current(slot) {
                        run += 1;
                        if run > MAX_PROBE_RUN {
                            return None;
                        }
                        slot = (slot + 1) & mask;
                    }
                    let tag = self.tag_for(position)?;
                    self.set(slot, tag);
                }
            }
            let mask = self.tags.len() - 1;
            let mut slot = self.home(&key, mask);
            for _ in 0..=MAX_PROBE_RUN {
                #[cfg(test)]
                {
                    *probes += 1;
                }
                if !self.is_current(slot) {
                    let tag = self.tag_for(found.len())?;
                    self.set(slot, tag);
                    found.push(key);
                    return Some(true);
                }
                if found[(self.tags[slot] - 1) as usize] == key {
                    return Some(false);
                }
                slot = (slot + 1) & mask;
            }
            None
        }

        /// Sends every key to slot `0`, to drive the collision fallback.
        #[cfg(test)]
        pub(super) fn force_collisions(&mut self, on: bool) {
            self.force_collisions = on;
        }

        /// Lowers the position limit, to drive the representability fallback.
        #[cfg(test)]
        pub(super) fn limit_positions(&mut self, limit: Option<usize>) {
            self.position_limit = limit;
        }

        /// The current generation, and a way to set it near the wrap.
        #[cfg(test)]
        pub(super) fn generation(&self) -> u32 {
            self.generation
        }

        #[cfg(test)]
        pub(super) fn set_generation(&mut self, generation: u32) {
            self.generation = generation;
        }
    }
}

use membership::MembershipTable;

/// Fills `scratch.reachable` and `scratch.warps` with the graph reachable from
/// `root`, following edges and `Descend` portals on nodes and edges, breadth
/// first. Returns `false`, with the scratch unfinished, when the membership
/// table meets a pathological probe run; the caller then uses the ordered
/// walk.
fn collect_reachable(state: &WarpState, root: &NodeKey, scratch: &mut StateRootScratch) -> bool {
    let StateRootScratch {
        found,
        index,
        reachable,
        warps,
        #[cfg(test)]
        probes,
        #[cfg(test)]
        cleared,
        #[cfg(test)]
        warp_pushes,
        ..
    } = scratch;
    found.clear();
    #[cfg(test)]
    {
        *cleared += index.begin();
    }
    #[cfg(not(test))]
    index.begin();
    warps.clear();
    warps.push(root.warp_id);
    let walked = (|| -> Option<()> {
        index.insert(
            found,
            *root,
            #[cfg(test)]
            probes,
        )?;
        let mut head = 0;
        while head < found.len() {
            let current = found[head];
            head += 1;
            let Some(store) = state.store(&current.warp_id) else {
                debug_assert!(
                    false,
                    "reachable traversal referenced missing warp store: {:?}",
                    current.warp_id
                );
                continue;
            };
            let mut descend = |child_warp: WarpId,
                               found: &mut Vec<NodeKey>,
                               index: &mut MembershipTable,
                               #[cfg(test)] probes: &mut usize|
             -> Option<()> {
                let Some(child) = state.instance(&child_warp) else {
                    // A missing instance contributes nothing to the hash.
                    debug_assert!(
                        false,
                        "descend referenced missing warp instance metadata: {child_warp:?}"
                    );
                    return Some(());
                };
                // A warp is listed when its root is first reached, which
                // happens once however many portals name it.
                if index.insert(
                    found,
                    NodeKey {
                        warp_id: child_warp,
                        local_id: child.root_node,
                    },
                    #[cfg(test)]
                    probes,
                )? {
                    warps.push(child_warp);
                    #[cfg(test)]
                    {
                        *warp_pushes += 1;
                    }
                }
                Some(())
            };
            for edge in store.edges_from(&current.local_id) {
                index.insert(
                    found,
                    NodeKey {
                        warp_id: current.warp_id,
                        local_id: edge.to,
                    },
                    #[cfg(test)]
                    probes,
                )?;
                if let Some(AttachmentValue::Descend(child_warp)) = store.edge_attachment(&edge.id)
                {
                    descend(
                        *child_warp,
                        found,
                        index,
                        #[cfg(test)]
                        probes,
                    )?;
                }
            }
            if let Some(AttachmentValue::Descend(child_warp)) =
                store.node_attachment(&current.local_id)
            {
                descend(
                    *child_warp,
                    found,
                    index,
                    #[cfg(test)]
                    probes,
                )?;
            }
        }
        Some(())
    })();
    if walked.is_none() {
        return false;
    }
    warps.sort_unstable();
    warps.dedup();
    reachable.clear();
    reachable.extend_from_slice(found);
    reachable.sort_unstable();
    true
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

/// The reachable warps and nodes, by an ordered breadth-first walk. It
/// allocates its sets, so the engine uses it only as the fallback when the
/// membership table meets a pathological probe run (see
/// [`membership::MembershipTable::insert`]); it also backs the test-only reference.
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

        /// The state root is independent of scratch history: one scratch,
        /// driven through any sequence of states and table conditions (the
        /// collision fallback, the representability fallback, generation
        /// rollover, and back to normal), always yields the reference root.
        #[test]
        fn state_root_is_independent_of_scratch_history(
            steps in proptest::collection::vec(
                (proptest::collection::vec(write(), 0..32), 0u8..4),
                1..8,
            ),
        ) {
            let root = NodeKey { warp_id: warp(0), local_id: node(0, 0) };
            let mut scratch = StateRootScratch::default();
            for (writes, condition) in &steps {
                scratch.index.force_collisions(*condition == 1);
                scratch.index.limit_positions((*condition == 2).then_some(2));
                if *condition == 3 {
                    // The next traversal wraps the generation counter.
                    scratch.index.set_generation(u32::MAX);
                }
                let state = state(writes);
                prop_assert_eq!(
                    compute_state_root_with(&state, &root, &mut scratch),
                    compute_state_root_reference(&state, &root)
                );
            }
        }
    }

    /// Generation rollover clears the table once and changes no root: the
    /// state roots before and after the wrap equal the reference.
    #[test]
    fn generation_rollover_changes_no_root() {
        let root = NodeKey {
            warp_id: warp(0),
            local_id: node(0, 0),
        };
        let first = state(&[
            Write::Node { warp: 0, node: 1 },
            Write::Edge {
                warp: 0,
                id: 1,
                from: 0,
                to: 1,
            },
        ]);
        let second = state(&[Write::Node { warp: 0, node: 2 }]);
        let mut scratch = StateRootScratch::default();
        assert_eq!(
            compute_state_root_with(&first, &root, &mut scratch),
            compute_state_root_reference(&first, &root)
        );
        scratch.index.set_generation(u32::MAX - 1);
        for state in [&first, &second, &first, &second] {
            assert_eq!(
                compute_state_root_with(state, &root, &mut scratch),
                compute_state_root_reference(state, &root)
            );
        }
        assert!(
            scratch.index.generation() < 4,
            "the generation counter did not wrap"
        );
    }

    /// A position the table cannot name falls back to the ordered walk
    /// rather than aliasing an existing position.
    #[test]
    fn unrepresentable_positions_fall_back() {
        let root = NodeKey {
            warp_id: warp(0),
            local_id: node(0, 0),
        };
        let writes: Vec<Write> = (1..10)
            .flat_map(|n| {
                [
                    Write::Node { warp: 0, node: n },
                    Write::Edge {
                        warp: 0,
                        id: n,
                        from: 0,
                        to: n,
                    },
                ]
            })
            .collect();
        let state = state(&writes);
        let mut scratch = StateRootScratch::default();
        scratch.index.limit_positions(Some(4));
        assert_eq!(
            compute_state_root_with(&state, &root, &mut scratch),
            compute_state_root_reference(&state, &root)
        );
        assert!(
            scratch.fell_back,
            "an unnameable position did not fall back"
        );
    }
}

#[cfg(test)]
mod reachability_work {
    use super::{compute_state_root_with, StateRootScratch};
    use crate::graph::GraphStore;
    use crate::ident::{make_edge_id, make_node_id, make_type_id, make_warp_id, NodeKey};
    use crate::record::{EdgeRecord, NodeRecord};
    use crate::warp_state::{WarpInstance, WarpState};

    /// Reaching a chain of `n` nodes costs O(n) membership probes: one probe
    /// sequence per edge, never a search of everything reached so far.
    #[test]
    fn reaching_a_deep_chain_probes_linearly() {
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
        let bound = 4 * N;
        assert!(
            scratch.probes <= bound,
            "reaching a {N}-node chain made {} membership probes; O(n) allows {bound}",
            scratch.probes
        );
    }

    /// Many portals naming the same few warps list each warp once, so their
    /// number never adds sorting work.
    #[test]
    fn repeated_portals_list_each_warp_once() {
        use crate::attachment::AttachmentValue;
        const EDGES: usize = 4_096;
        let ty = make_type_id("reachability-work/ty");
        let root_warp = make_warp_id("reachability-work/portals/root");
        let child = |w: usize| make_warp_id(&format!("reachability-work/portals/child/{w}"));
        let node = |i: usize| make_node_id(&format!("reachability-work/portals/node/{i}"));
        let mut root_store = GraphStore::new(root_warp);
        root_store.insert_node(node(0), NodeRecord { ty });
        for i in 1..=EDGES {
            root_store.insert_node(node(i), NodeRecord { ty });
            let edge = make_edge_id(&format!("reachability-work/portals/edge/{i}"));
            root_store.insert_edge(
                node(0),
                EdgeRecord {
                    id: edge,
                    from: node(0),
                    to: node(i),
                    ty,
                },
            );
            root_store.set_edge_attachment(edge, Some(AttachmentValue::Descend(child(i % 2))));
        }
        let mut state = WarpState::new();
        state.upsert_instance(
            WarpInstance {
                warp_id: root_warp,
                root_node: node(0),
                parent: None,
            },
            root_store,
        );
        for w in 0..2 {
            let child_root = make_node_id(&format!("reachability-work/portals/child-root/{w}"));
            let mut store = GraphStore::new(child(w));
            store.insert_node(child_root, NodeRecord { ty });
            state.upsert_instance(
                WarpInstance {
                    warp_id: child(w),
                    root_node: child_root,
                    parent: None,
                },
                store,
            );
        }
        let root = NodeKey {
            warp_id: root_warp,
            local_id: node(0),
        };
        let mut scratch = StateRootScratch::default();
        let hash = compute_state_root_with(&state, &root, &mut scratch);
        assert!(
            scratch.warp_pushes <= 3,
            "{EDGES} portals to 2 warps listed {} warps before deduplication",
            scratch.warp_pushes
        );
        assert_eq!(hash, super::compute_state_root_reference(&state, &root));
    }

    /// Ids chosen to collide in the membership table cannot make a commit
    /// quadratic: past [`super::membership::MAX_PROBE_RUN`] the traversal falls back to the
    /// ordered walk, and the state root is unchanged.
    #[test]
    fn colliding_ids_fall_back_to_the_ordered_walk() {
        const N: usize = 4_096;
        let warp_id = make_warp_id("reachability-work/collide");
        let ty = make_type_id("reachability-work/ty");
        let node = |i: usize| make_node_id(&format!("reachability-work/collide/{i}"));
        let mut store = GraphStore::new(warp_id);
        for i in 0..N {
            store.insert_node(node(i), NodeRecord { ty });
        }
        for i in 1..N {
            store.insert_edge(
                node(0),
                EdgeRecord {
                    id: make_edge_id(&format!("reachability-work/collide/edge/{i}")),
                    from: node(0),
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
        scratch.index.force_collisions(true);
        let hash = compute_state_root_with(&state, &root, &mut scratch);
        assert!(scratch.fell_back, "colliding ids did not take the fallback");
        // The table gives up within one probe run per key, before N matters.
        let bound = (super::membership::MAX_PROBE_RUN + 1) * (super::membership::MAX_PROBE_RUN + 2);
        assert!(
            scratch.probes <= bound,
            "{N} colliding ids made {} probes; the fallback allows {bound}, independent of N",
            scratch.probes
        );
        assert_eq!(hash, super::compute_state_root_reference(&state, &root));
    }

    /// A walk of a small graph after a large one clears nothing: the
    /// membership table keeps its peak size, but starting a walk is one
    /// generation increment, so a commit pays for the graph it reaches now.
    #[test]
    fn a_small_walk_after_a_large_one_clears_nothing() {
        let star = |n: usize| {
            let warp_id = make_warp_id("reachability-work/shrink");
            let ty = make_type_id("reachability-work/ty");
            let node = |i: usize| make_node_id(&format!("reachability-work/shrink/{i}"));
            let mut store = GraphStore::new(warp_id);
            for i in 0..n {
                store.insert_node(node(i), NodeRecord { ty });
            }
            for i in 1..n {
                store.insert_edge(
                    node(0),
                    EdgeRecord {
                        id: make_edge_id(&format!("reachability-work/shrink/edge/{i}")),
                        from: node(0),
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
            (
                state,
                NodeKey {
                    warp_id,
                    local_id: node(0),
                },
            )
        };
        let mut scratch = StateRootScratch::default();
        let (large, root) = star(4_096);
        let _ = compute_state_root_with(&large, &root, &mut scratch);
        let (small, root) = star(1);
        let before = scratch.cleared;
        let _ = compute_state_root_with(&small, &root, &mut scratch);
        assert_eq!(
            scratch.cleared - before,
            0,
            "a one-node walk after a 4096-node walk cleared {} table slots",
            scratch.cleared - before
        );
    }

    /// Ids that share their leading bytes, as caller-authored ids may, still
    /// spread across the membership table: probing stays linear.
    #[test]
    fn ids_sharing_leading_bytes_probe_linearly() {
        const N: usize = 4_096;
        let warp_id = make_warp_id("reachability-work/shared-prefix");
        let ty = make_type_id("reachability-work/ty");
        let node = |i: usize| {
            let mut id = [0u8; 32];
            id[24..].copy_from_slice(&(i as u64).to_le_bytes());
            crate::ident::NodeId(id)
        };
        let mut store = GraphStore::new(warp_id);
        for i in 0..N {
            store.insert_node(node(i), NodeRecord { ty });
        }
        for i in 1..N {
            store.insert_edge(
                node(0),
                EdgeRecord {
                    id: make_edge_id(&format!("reachability-work/shared-prefix/{i}")),
                    from: node(0),
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
        let bound = 4 * N;
        assert!(
            scratch.probes <= bound,
            "{N} ids sharing leading bytes made {} membership probes; O(n) allows {bound}",
            scratch.probes
        );
    }
}
