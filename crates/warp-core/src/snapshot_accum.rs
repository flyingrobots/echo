// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

#![allow(dead_code)]

//! Columnar snapshot accumulator for building WSC directly from base + ops.
//!
//! # Overview
//!
//! `SnapshotAccumulator` is a lightweight data structure that:
//! 1. Captures an immutable base state from `WarpState`
//! 2. Applies `WarpOp` operations to produce a new state
//! 3. Builds WSC bytes and computes `state_root` directly from tables
//!
//! Unlike [`GraphStore`], this accumulator:
//! - Has NO reverse indexes (`edge_index`, `edge_to_index`, `edges_to`)
//! - Stores only what WSC rows need (no extra fields)
//! - Computes adjacency (`edges_from`) at build time, not during op application
//!
//! # Phase 4 (ADR-0007)
//!
//! This module implements the `SnapshotBuilder` described in ADR-0007 Section 12.
//! The key invariant: `base_view + ops → WSC` without rebuilding [`GraphStore`].

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::attachment::{AttachmentKey, AttachmentOwner, AttachmentPlane, AttachmentValue};
use crate::ident::{EdgeId, Hash, NodeId, NodeKey, TypeId, WarpId};
use crate::tick_patch::WarpOp;
use crate::warp_state::{WarpInstance, WarpState};
use crate::wsc::types::{AttRow, EdgeRow, NodeRow, OutEdgeRef, Range};
use crate::wsc::write::{write_wsc_one_warp, OneWarpInput};

/// Minimal node data needed for WSC rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeRowParts {
    /// Node identifier within the instance.
    pub node_id: NodeId,
    /// Type identifier for the node.
    pub node_type: TypeId,
}

/// Minimal edge data needed for WSC rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EdgeRowParts {
    /// Edge identifier.
    pub edge_id: EdgeId,
    /// Source node identifier.
    pub from: NodeId,
    /// Target node identifier.
    pub to: NodeId,
    /// Type identifier for the edge.
    pub edge_type: TypeId,
}

/// Output from building a snapshot.
#[derive(Debug)]
pub struct SnapshotOutput {
    /// Serialized WSC bytes.
    #[allow(dead_code)]
    pub wsc_bytes: Vec<u8>,
    /// Canonical hash of materialized reachable state.
    pub state_root: Hash,
}

/// Columnar snapshot accumulator.
///
/// This is NOT [`GraphStore`]. It stores exactly what's needed to:
/// - Apply ops via key lookup
/// - Iterate in canonical order
/// - Build WSC output
///
/// No reverse indexes, no delete-by-scan helpers.
#[derive(Debug, Default)]
pub struct SnapshotAccumulator {
    /// Instance metadata keyed by `WarpId`.
    instances: BTreeMap<WarpId, WarpInstance>,

    /// Nodes keyed by (`WarpId`, `NodeId`) for efficient lookup and canonical iteration.
    nodes: BTreeMap<NodeKey, NodeRowParts>,

    /// Edges keyed by (`WarpId`, `EdgeId`) for efficient lookup.
    /// Adjacency (`edges_from`) is computed at build time.
    edges: BTreeMap<(WarpId, EdgeId), EdgeRowParts>,

    /// Node attachments keyed by `AttachmentKey`.
    node_attachments: BTreeMap<AttachmentKey, AttachmentValue>,

    /// Edge attachments keyed by `AttachmentKey`.
    edge_attachments: BTreeMap<AttachmentKey, AttachmentValue>,
}

impl SnapshotAccumulator {
    /// Create an empty accumulator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Initialize accumulator from an existing `WarpState`.
    ///
    /// This captures the base state by copying the minimal data needed.
    /// The original `WarpState` is not modified and can be safely mutated
    /// after this call returns.
    pub fn from_warp_state(state: &WarpState) -> Self {
        let mut acc = Self::new();

        // Copy instance metadata
        for (warp_id, instance) in state.iter_instances() {
            acc.instances.insert(*warp_id, instance.clone());
        }

        // Copy nodes and edges from each store
        for (warp_id, store) in state.iter_stores() {
            // Nodes
            for (node_id, record) in store.iter_nodes() {
                let key = NodeKey {
                    warp_id: *warp_id,
                    local_id: *node_id,
                };
                acc.nodes.insert(
                    key,
                    NodeRowParts {
                        node_id: *node_id,
                        node_type: record.ty,
                    },
                );
            }

            // Edges (iterate all edges from the store)
            for (_from_node, edge_list) in store.iter_edges() {
                for edge in edge_list {
                    acc.edges.insert(
                        (*warp_id, edge.id),
                        EdgeRowParts {
                            edge_id: edge.id,
                            from: edge.from,
                            to: edge.to,
                            edge_type: edge.ty,
                        },
                    );
                }
            }

            // Node attachments
            for (node_id, value) in store.iter_node_attachments() {
                let key = AttachmentKey {
                    owner: AttachmentOwner::Node(NodeKey {
                        warp_id: *warp_id,
                        local_id: *node_id,
                    }),
                    plane: AttachmentPlane::Alpha,
                };
                acc.node_attachments.insert(key, value.clone());
            }

            // Edge attachments
            for (edge_id, value) in store.iter_edge_attachments() {
                let key = AttachmentKey {
                    owner: AttachmentOwner::Edge(crate::ident::EdgeKey {
                        warp_id: *warp_id,
                        local_id: *edge_id,
                    }),
                    plane: AttachmentPlane::Beta,
                };
                acc.edge_attachments.insert(key, value.clone());
            }
        }

        acc
    }

    /// Apply a sequence of operations to the accumulator.
    ///
    /// Operations should be canonically sorted (via `WarpOp::sort_key()`).
    pub fn apply_ops(&mut self, ops: Vec<WarpOp>) {
        for op in ops {
            self.apply_op(op);
        }
    }

    /// Apply a single operation to the accumulator.
    fn apply_op(&mut self, op: WarpOp) {
        match op {
            WarpOp::OpenPortal {
                key,
                child_warp,
                child_root,
                init,
            } => {
                // Create the child instance
                let instance = WarpInstance {
                    warp_id: child_warp,
                    root_node: child_root,
                    parent: Some(key),
                };
                self.instances.insert(child_warp, instance);

                // Create the root node if init specifies
                if let crate::tick_patch::PortalInit::Empty { root_record } = init {
                    let node_key = NodeKey {
                        warp_id: child_warp,
                        local_id: child_root,
                    };
                    self.nodes.insert(
                        node_key,
                        NodeRowParts {
                            node_id: child_root,
                            node_type: root_record.ty,
                        },
                    );
                }

                // Set the parent attachment to Descend
                self.set_attachment_internal(key, Some(AttachmentValue::Descend(child_warp)));
            }

            WarpOp::UpsertWarpInstance { instance } => {
                self.instances.insert(instance.warp_id, instance);
            }

            WarpOp::DeleteWarpInstance { warp_id } => {
                self.instances.remove(&warp_id);
                // Cascade: remove all nodes, edges, and attachments for this instance
                self.nodes.retain(|k, _| k.warp_id != warp_id);
                self.edges.retain(|(w, _), _| *w != warp_id);
                self.node_attachments.retain(|k, _| match k.owner {
                    AttachmentOwner::Node(nk) => nk.warp_id != warp_id,
                    AttachmentOwner::Edge(ek) => ek.warp_id != warp_id,
                });
                self.edge_attachments.retain(|k, _| match k.owner {
                    AttachmentOwner::Node(nk) => nk.warp_id != warp_id,
                    AttachmentOwner::Edge(ek) => ek.warp_id != warp_id,
                });
            }

            WarpOp::UpsertNode { node, record } => {
                self.nodes.insert(
                    node,
                    NodeRowParts {
                        node_id: node.local_id,
                        node_type: record.ty,
                    },
                );
            }

            WarpOp::DeleteNode { node } => {
                self.nodes.remove(&node);
                // Remove node's attachments
                let att_key = AttachmentKey {
                    owner: AttachmentOwner::Node(node),
                    plane: AttachmentPlane::Alpha,
                };
                self.node_attachments.remove(&att_key);
                // Cascade: remove incident edges (GraphStore semantics)
                self.remove_incident_edges(node);
            }

            WarpOp::UpsertEdge { warp_id, record } => {
                self.edges.insert(
                    (warp_id, record.id),
                    EdgeRowParts {
                        edge_id: record.id,
                        from: record.from,
                        to: record.to,
                        edge_type: record.ty,
                    },
                );
            }

            WarpOp::DeleteEdge {
                warp_id,
                from: _,
                edge_id,
            } => {
                self.edges.remove(&(warp_id, edge_id));
                // Remove edge's attachments
                let att_key = AttachmentKey {
                    owner: AttachmentOwner::Edge(crate::ident::EdgeKey {
                        warp_id,
                        local_id: edge_id,
                    }),
                    plane: AttachmentPlane::Beta,
                };
                self.edge_attachments.remove(&att_key);
            }

            WarpOp::SetAttachment { key, value } => {
                self.set_attachment_internal(key, value);
            }
        }
    }

    /// Internal helper for setting/clearing attachments.
    fn set_attachment_internal(&mut self, key: AttachmentKey, value: Option<AttachmentValue>) {
        let map = match key.owner {
            AttachmentOwner::Node(_) => &mut self.node_attachments,
            AttachmentOwner::Edge(_) => &mut self.edge_attachments,
        };

        match value {
            Some(v) => {
                map.insert(key, v);
            }
            None => {
                map.remove(&key);
            }
        }
    }

    /// Remove all edges incident to a node and their attachments.
    ///
    /// This implements cascade delete semantics matching `GraphStore`.
    fn remove_incident_edges(&mut self, node: NodeKey) {
        let edges_to_remove: Vec<_> = self
            .edges
            .iter()
            .filter(|((w, _), e)| {
                *w == node.warp_id && (e.from == node.local_id || e.to == node.local_id)
            })
            .map(|((w, eid), _)| (*w, *eid))
            .collect();

        for (warp_id, edge_id) in edges_to_remove {
            self.edges.remove(&(warp_id, edge_id));
            let edge_att_key = AttachmentKey {
                owner: AttachmentOwner::Edge(crate::ident::EdgeKey {
                    warp_id,
                    local_id: edge_id,
                }),
                plane: AttachmentPlane::Beta,
            };
            self.edge_attachments.remove(&edge_att_key);
        }
    }

    /// Build WSC bytes and compute `state_root`.
    ///
    /// This method:
    /// 1. Computes reachability via BFS from the root
    /// 2. Filters to reachable-only nodes/edges/instances
    /// 3. Builds columnar structures (`OneWarpInput` per instance)
    /// 4. Writes WSC bytes
    /// 5. Computes `state_root` from the tables
    #[allow(clippy::panic)] // Vec-backed Write is infallible; panic indicates a bug
    pub fn build(&self, root: &NodeKey, schema_hash: Hash, tick: u64) -> SnapshotOutput {
        // Phase 1: Compute reachability
        let (reachable_nodes, reachable_warps) = self.compute_reachability(root);

        // Phase 2: Build OneWarpInput for each reachable instance
        let mut warp_inputs: Vec<OneWarpInput> = Vec::new();

        for warp_id in &reachable_warps {
            if let Some(input) = self.build_one_warp_input(*warp_id, &reachable_nodes) {
                warp_inputs.push(input);
            }
        }

        // Phase 3: Write WSC bytes
        // For now, we only support single-instance (the warp_inputs should have one entry)
        // Multi-instance WSC writing will be needed for full Stage B1
        let wsc_bytes = if warp_inputs.is_empty() {
            Vec::new()
        } else {
            // Use the first (root) warp's input
            // TODO: Support multi-warp WSC files
            write_wsc_one_warp(&warp_inputs[0], schema_hash, tick).unwrap_or_else(|e| {
                panic!("write_wsc_one_warp failed (should be infallible for Vec): {e}")
            })
        };

        // Phase 4: Compute state_root
        let state_root = self.compute_state_root(root, &reachable_nodes, &reachable_warps);

        SnapshotOutput {
            wsc_bytes,
            state_root,
        }
    }

    /// Compute reachability via BFS from the root node.
    ///
    /// Returns (`reachable_nodes`, `reachable_warps`).
    fn compute_reachability(&self, root: &NodeKey) -> (BTreeSet<NodeKey>, BTreeSet<WarpId>) {
        let mut reachable_nodes: BTreeSet<NodeKey> = BTreeSet::new();
        let mut reachable_warps: BTreeSet<WarpId> = BTreeSet::new();
        let mut queue: VecDeque<NodeKey> = VecDeque::new();

        // Seed with root
        reachable_nodes.insert(*root);
        reachable_warps.insert(root.warp_id);
        queue.push_back(*root);

        while let Some(current) = queue.pop_front() {
            // Follow edges from this node (within same instance)
            for ((warp_id, _edge_id), edge) in &self.edges {
                if *warp_id != current.warp_id || edge.from != current.local_id {
                    continue;
                }

                let target = NodeKey {
                    warp_id: current.warp_id,
                    local_id: edge.to,
                };
                if reachable_nodes.insert(target) {
                    queue.push_back(target);
                }

                // Check edge attachment for Descend
                let edge_att_key = AttachmentKey {
                    owner: AttachmentOwner::Edge(crate::ident::EdgeKey {
                        warp_id: *warp_id,
                        local_id: edge.edge_id,
                    }),
                    plane: AttachmentPlane::Beta,
                };
                if let Some(AttachmentValue::Descend(child_warp)) =
                    self.edge_attachments.get(&edge_att_key)
                {
                    self.enqueue_descend(
                        *child_warp,
                        &mut reachable_warps,
                        &mut reachable_nodes,
                        &mut queue,
                    );
                }
            }

            // Check node attachment for Descend
            let node_att_key = AttachmentKey {
                owner: AttachmentOwner::Node(current),
                plane: AttachmentPlane::Alpha,
            };
            if let Some(AttachmentValue::Descend(child_warp)) =
                self.node_attachments.get(&node_att_key)
            {
                self.enqueue_descend(
                    *child_warp,
                    &mut reachable_warps,
                    &mut reachable_nodes,
                    &mut queue,
                );
            }
        }

        (reachable_nodes, reachable_warps)
    }

    /// Helper to enqueue a descended instance's root node.
    fn enqueue_descend(
        &self,
        child_warp: WarpId,
        reachable_warps: &mut BTreeSet<WarpId>,
        reachable_nodes: &mut BTreeSet<NodeKey>,
        queue: &mut VecDeque<NodeKey>,
    ) {
        reachable_warps.insert(child_warp);
        if let Some(instance) = self.instances.get(&child_warp) {
            let child_root = NodeKey {
                warp_id: child_warp,
                local_id: instance.root_node,
            };
            if reachable_nodes.insert(child_root) {
                queue.push_back(child_root);
            }
        }
    }

    /// Build `OneWarpInput` for a single instance.
    #[allow(clippy::too_many_lines)]
    fn build_one_warp_input(
        &self,
        warp_id: WarpId,
        reachable_nodes: &BTreeSet<NodeKey>,
    ) -> Option<OneWarpInput> {
        let instance = self.instances.get(&warp_id)?;

        // Collect nodes for this instance (filtered to reachable, sorted by NodeId)
        let mut nodes: Vec<NodeRow> = Vec::new();

        for (key, parts) in &self.nodes {
            if key.warp_id != warp_id || !reachable_nodes.contains(key) {
                continue;
            }
            nodes.push(NodeRow {
                node_id: parts.node_id.0,
                node_type: parts.node_type.0,
            });
        }

        // Collect edges for this instance (sorted by EdgeId)
        let mut edges: Vec<EdgeRow> = Vec::new();
        let mut edge_id_to_ix: BTreeMap<EdgeId, usize> = BTreeMap::new();

        // Also build edges_from for out_index/out_edges
        let mut edges_from: BTreeMap<NodeId, Vec<(EdgeId, usize)>> = BTreeMap::new();

        for ((w, _), parts) in &self.edges {
            if *w != warp_id {
                continue;
            }
            // Only include edges whose source is reachable
            let from_key = NodeKey {
                warp_id,
                local_id: parts.from,
            };
            if !reachable_nodes.contains(&from_key) {
                continue;
            }
            // Only include edges whose target is reachable
            let to_key = NodeKey {
                warp_id,
                local_id: parts.to,
            };
            if !reachable_nodes.contains(&to_key) {
                continue;
            }

            let edge_ix = edges.len();
            edge_id_to_ix.insert(parts.edge_id, edge_ix);
            edges.push(EdgeRow {
                edge_id: parts.edge_id.0,
                from_node_id: parts.from.0,
                to_node_id: parts.to.0,
                edge_type: parts.edge_type.0,
            });

            edges_from
                .entry(parts.from)
                .or_default()
                .push((parts.edge_id, edge_ix));
        }

        // Build out_index and out_edges (parallel to nodes)
        let mut out_index: Vec<Range> = Vec::with_capacity(nodes.len());
        let mut out_edges: Vec<OutEdgeRef> = Vec::new();

        for key in self.nodes.keys() {
            if key.warp_id != warp_id || !reachable_nodes.contains(key) {
                continue;
            }
            let node_id = key.local_id;

            let start = out_edges.len() as u64;
            if let Some(edge_list) = edges_from.get(&node_id) {
                // Sort by EdgeId for canonical ordering
                let mut sorted: Vec<_> = edge_list.clone();
                sorted.sort_by_key(|(eid, _)| *eid);

                for (edge_id, edge_ix) in sorted {
                    out_edges.push(OutEdgeRef {
                        edge_ix_le: (edge_ix as u64).to_le(),
                        edge_id: edge_id.0,
                    });
                }
            }
            let len = out_edges.len() as u64 - start;
            out_index.push(Range {
                start_le: start.to_le(),
                len_le: len.to_le(),
            });
        }

        // Build node attachments (parallel to nodes)
        let mut node_atts_index: Vec<Range> = Vec::with_capacity(nodes.len());
        let mut node_atts: Vec<AttRow> = Vec::new();
        let mut blobs: Vec<u8> = Vec::new();

        for key in self.nodes.keys() {
            if key.warp_id != warp_id || !reachable_nodes.contains(key) {
                continue;
            }

            let att_key = AttachmentKey {
                owner: AttachmentOwner::Node(*key),
                plane: AttachmentPlane::Alpha,
            };

            let start = node_atts.len() as u64;
            if let Some(value) = self.node_attachments.get(&att_key) {
                let row = att_value_to_row(value, &mut blobs);
                node_atts.push(row);
            }
            let len = node_atts.len() as u64 - start;
            node_atts_index.push(Range {
                start_le: start.to_le(),
                len_le: len.to_le(),
            });
        }

        // Build edge attachments (parallel to edges)
        let mut edge_atts_index: Vec<Range> = Vec::with_capacity(edges.len());
        let mut edge_atts: Vec<AttRow> = Vec::new();

        for (w, edge_id) in self.edges.keys() {
            if *w != warp_id {
                continue;
            }
            // Check if this edge is included (both endpoints reachable)
            if !edge_id_to_ix.contains_key(edge_id) {
                continue;
            }

            let att_key = AttachmentKey {
                owner: AttachmentOwner::Edge(crate::ident::EdgeKey {
                    warp_id,
                    local_id: *edge_id,
                }),
                plane: AttachmentPlane::Beta,
            };

            let start = edge_atts.len() as u64;
            if let Some(value) = self.edge_attachments.get(&att_key) {
                let row = att_value_to_row(value, &mut blobs);
                edge_atts.push(row);
            }
            let len = edge_atts.len() as u64 - start;
            edge_atts_index.push(Range {
                start_le: start.to_le(),
                len_le: len.to_le(),
            });
        }

        Some(OneWarpInput {
            warp_id: warp_id.0,
            root_node_id: instance.root_node.0,
            nodes,
            edges,
            out_index,
            out_edges,
            node_atts_index,
            node_atts,
            edge_atts_index,
            edge_atts,
            blobs,
        })
    }

    /// Compute `state_root` directly from accumulator tables.
    ///
    /// Same algorithm as `crate::snapshot::compute_state_root()`, but operates
    /// on the accumulator's internal structures instead of `WarpState`.
    fn compute_state_root(
        &self,
        root: &NodeKey,
        reachable_nodes: &BTreeSet<NodeKey>,
        reachable_warps: &BTreeSet<WarpId>,
    ) -> Hash {
        use blake3::Hasher;

        let mut hasher = Hasher::new();

        // Root binding
        hasher.update(&root.warp_id.0);
        hasher.update(&root.local_id.0);

        // Process instances in canonical order (BTreeSet iteration is sorted)
        for warp_id in reachable_warps {
            let Some(instance) = self.instances.get(warp_id) else {
                continue;
            };

            // Instance header
            hasher.update(&instance.warp_id.0);
            hasher.update(&instance.root_node.0);

            // Parent attachment key (if any)
            if let Some(ref parent_key) = instance.parent {
                hasher.update(&[1u8]); // Present
                hash_attachment_key(&mut hasher, parent_key);
            } else {
                hasher.update(&[0u8]); // Absent
            }

            // Nodes in this instance (sorted by NodeId, filtered to reachable)
            for (key, parts) in &self.nodes {
                if key.warp_id != *warp_id || !reachable_nodes.contains(key) {
                    continue;
                }

                hasher.update(&parts.node_id.0);
                hasher.update(&parts.node_type.0);

                // Node attachment
                let att_key = AttachmentKey {
                    owner: AttachmentOwner::Node(*key),
                    plane: AttachmentPlane::Alpha,
                };
                hash_optional_attachment(&mut hasher, self.node_attachments.get(&att_key));
            }

            // Edges in this instance, grouped by source node
            // Collect edges by source, then iterate sources in NodeId order
            let mut edges_by_source: BTreeMap<NodeId, Vec<&EdgeRowParts>> = BTreeMap::new();
            for ((w, _), parts) in &self.edges {
                if *w != *warp_id {
                    continue;
                }
                let from_key = NodeKey {
                    warp_id: *warp_id,
                    local_id: parts.from,
                };
                let to_key = NodeKey {
                    warp_id: *warp_id,
                    local_id: parts.to,
                };
                if !reachable_nodes.contains(&from_key) || !reachable_nodes.contains(&to_key) {
                    continue;
                }
                edges_by_source.entry(parts.from).or_default().push(parts);
            }

            for (from_node, edge_list) in &mut edges_by_source {
                // Sort edges by EdgeId
                edge_list.sort_by_key(|e| e.edge_id);

                hasher.update(&from_node.0);
                hasher.update(&(edge_list.len() as u64).to_le_bytes());

                for edge in edge_list {
                    hasher.update(&edge.edge_id.0);
                    hasher.update(&edge.edge_type.0);
                    hasher.update(&edge.to.0);

                    // Edge attachment
                    let att_key = AttachmentKey {
                        owner: AttachmentOwner::Edge(crate::ident::EdgeKey {
                            warp_id: *warp_id,
                            local_id: edge.edge_id,
                        }),
                        plane: AttachmentPlane::Beta,
                    };
                    hash_optional_attachment(&mut hasher, self.edge_attachments.get(&att_key));
                }
            }

            // Hash nodes with zero outgoing edges to maintain parity with legacy
            for (key, parts) in &self.nodes {
                if key.warp_id != *warp_id || !reachable_nodes.contains(key) {
                    continue;
                }
                if !edges_by_source.contains_key(&parts.node_id) {
                    hasher.update(&parts.node_id.0);
                    hasher.update(&0u64.to_le_bytes()); // Zero edges
                }
            }
        }

        hasher.finalize().into()
    }
}

/// Convert `AttachmentValue` to `AttRow`, appending blob data.
fn att_value_to_row(value: &AttachmentValue, blobs: &mut Vec<u8>) -> AttRow {
    match value {
        AttachmentValue::Atom(payload) => {
            // Align to 8 bytes
            let padding = (8 - (blobs.len() % 8)) % 8;
            blobs.extend(std::iter::repeat_n(0u8, padding));

            let offset = blobs.len() as u64;
            blobs.extend_from_slice(&payload.bytes);
            let len = payload.bytes.len() as u64;

            AttRow {
                tag: AttRow::TAG_ATOM,
                reserved0: [0u8; 7],
                type_or_warp: payload.type_id.0,
                blob_off_le: offset.to_le(),
                blob_len_le: len.to_le(),
            }
        }
        AttachmentValue::Descend(warp_id) => AttRow {
            tag: AttRow::TAG_DESCEND,
            reserved0: [0u8; 7],
            type_or_warp: warp_id.0,
            blob_off_le: 0,
            blob_len_le: 0,
        },
    }
}

/// Hash an `AttachmentKey` into the hasher.
/// Must match the legacy implementation in snapshot.rs exactly.
fn hash_attachment_key(hasher: &mut blake3::Hasher, key: &AttachmentKey) {
    // Get tags via the same method as legacy
    let (owner_tag, plane_tag) = key.tag();
    hasher.update(&[owner_tag]);
    hasher.update(&[plane_tag]);
    match &key.owner {
        AttachmentOwner::Node(node) => {
            hasher.update(&node.warp_id.0);
            hasher.update(&node.local_id.0);
        }
        AttachmentOwner::Edge(edge) => {
            hasher.update(&edge.warp_id.0);
            hasher.update(&edge.local_id.0);
        }
    }
}

/// Hash an optional `AttachmentValue` into the hasher.
/// Must match the legacy implementation in snapshot.rs exactly.
fn hash_optional_attachment(hasher: &mut blake3::Hasher, value: Option<&AttachmentValue>) {
    match value {
        None => {
            hasher.update(&[0u8]);
        }
        Some(v) => {
            hasher.update(&[1u8]); // Some tag
            hash_attachment_value(hasher, v);
        }
    }
}

/// Hash an `AttachmentValue` into the hasher.
fn hash_attachment_value(hasher: &mut blake3::Hasher, value: &AttachmentValue) {
    match value {
        AttachmentValue::Atom(atom) => {
            hasher.update(&[1u8]); // Atom tag
            hasher.update(&atom.type_id.0);
            hasher.update(&(atom.bytes.len() as u64).to_le_bytes());
            hasher.update(&atom.bytes);
        }
        AttachmentValue::Descend(warp_id) => {
            hasher.update(&[2u8]); // Descend tag
            hasher.update(&warp_id.0);
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_accumulator() {
        let acc = SnapshotAccumulator::new();
        assert!(acc.instances.is_empty());
        assert!(acc.nodes.is_empty());
        assert!(acc.edges.is_empty());
    }

    // More tests will be added as we integrate with the engine

    use crate::ident::{make_edge_id, make_node_id, make_type_id, make_warp_id};
    use crate::record::{EdgeRecord, NodeRecord};
    use crate::tick_patch::WarpOp;
    use crate::warp_state::WarpInstance;

    /// Helper to create a basic accumulator with a single instance and root node.
    fn setup_single_instance() -> (SnapshotAccumulator, WarpId, NodeId) {
        let warp_id = make_warp_id("test-warp");
        let root_id = make_node_id("root");

        let mut acc = SnapshotAccumulator::new();

        // Add instance metadata
        acc.instances.insert(
            warp_id,
            WarpInstance {
                warp_id,
                root_node: root_id,
                parent: None,
            },
        );

        // Add root node
        let root_key = NodeKey {
            warp_id,
            local_id: root_id,
        };
        acc.nodes.insert(
            root_key,
            NodeRowParts {
                node_id: root_id,
                node_type: make_type_id("RootType"),
            },
        );

        (acc, warp_id, root_id)
    }

    #[test]
    fn test_apply_ops_create_node() {
        let (mut acc, warp_id, _root_id) = setup_single_instance();

        let new_node_id = make_node_id("new-node");
        let new_node_key = NodeKey {
            warp_id,
            local_id: new_node_id,
        };
        let node_type = make_type_id("NewNodeType");

        // Apply CreateNode via UpsertNode op
        acc.apply_ops(vec![WarpOp::UpsertNode {
            node: new_node_key,
            record: NodeRecord { ty: node_type },
        }]);

        // Verify node exists
        assert!(acc.nodes.contains_key(&new_node_key));
        let node_parts = acc.nodes.get(&new_node_key).expect("node should exist");
        assert_eq!(node_parts.node_id, new_node_id);
        assert_eq!(node_parts.node_type, node_type);
    }

    #[test]
    fn test_apply_ops_update_node() {
        let (mut acc, warp_id, root_id) = setup_single_instance();

        let root_key = NodeKey {
            warp_id,
            local_id: root_id,
        };
        let new_type = make_type_id("UpdatedRootType");

        // Verify original type
        let original_type = acc
            .nodes
            .get(&root_key)
            .expect("root should exist")
            .node_type;
        assert_eq!(original_type, make_type_id("RootType"));

        // Apply update
        acc.apply_ops(vec![WarpOp::UpsertNode {
            node: root_key,
            record: NodeRecord { ty: new_type },
        }]);

        // Verify type changed
        let updated_type = acc
            .nodes
            .get(&root_key)
            .expect("root should exist")
            .node_type;
        assert_eq!(updated_type, new_type);
        assert_ne!(updated_type, original_type);
    }

    #[test]
    fn test_apply_ops_delete_node() {
        let (mut acc, warp_id, _root_id) = setup_single_instance();

        // First create a node to delete
        let temp_node_id = make_node_id("temp-node");
        let temp_node_key = NodeKey {
            warp_id,
            local_id: temp_node_id,
        };

        acc.apply_ops(vec![WarpOp::UpsertNode {
            node: temp_node_key,
            record: NodeRecord {
                ty: make_type_id("TempType"),
            },
        }]);

        assert!(acc.nodes.contains_key(&temp_node_key));

        // Now delete it
        acc.apply_ops(vec![WarpOp::DeleteNode {
            node: temp_node_key,
        }]);

        // Verify removal
        assert!(!acc.nodes.contains_key(&temp_node_key));
    }

    #[test]
    fn test_apply_ops_edges() {
        let (mut acc, warp_id, root_id) = setup_single_instance();

        // Create a second node to connect to
        let target_node_id = make_node_id("target-node");
        let target_node_key = NodeKey {
            warp_id,
            local_id: target_node_id,
        };

        acc.apply_ops(vec![WarpOp::UpsertNode {
            node: target_node_key,
            record: NodeRecord {
                ty: make_type_id("TargetType"),
            },
        }]);

        // Create an edge from root to target
        let edge_id = make_edge_id("root-to-target");
        let edge_type = make_type_id("ConnectsTo");

        acc.apply_ops(vec![WarpOp::UpsertEdge {
            warp_id,
            record: EdgeRecord {
                id: edge_id,
                from: root_id,
                to: target_node_id,
                ty: edge_type,
            },
        }]);

        // Verify edge exists
        let edge_key = (warp_id, edge_id);
        assert!(acc.edges.contains_key(&edge_key));

        let edge_parts = acc.edges.get(&edge_key).expect("edge should exist");
        assert_eq!(edge_parts.edge_id, edge_id);
        assert_eq!(edge_parts.from, root_id);
        assert_eq!(edge_parts.to, target_node_id);
        assert_eq!(edge_parts.edge_type, edge_type);
    }

    #[test]
    fn test_filter_reachable_removes_unreachable() {
        let (mut acc, warp_id, root_id) = setup_single_instance();

        // Create a reachable node connected from root
        let reachable_id = make_node_id("reachable");
        let reachable_key = NodeKey {
            warp_id,
            local_id: reachable_id,
        };

        acc.apply_ops(vec![
            WarpOp::UpsertNode {
                node: reachable_key,
                record: NodeRecord {
                    ty: make_type_id("ReachableType"),
                },
            },
            WarpOp::UpsertEdge {
                warp_id,
                record: EdgeRecord {
                    id: make_edge_id("root-to-reachable"),
                    from: root_id,
                    to: reachable_id,
                    ty: make_type_id("Link"),
                },
            },
        ]);

        // Create an unreachable node (no edge from root or reachable)
        let unreachable_id = make_node_id("unreachable");
        let unreachable_key = NodeKey {
            warp_id,
            local_id: unreachable_id,
        };

        acc.apply_ops(vec![WarpOp::UpsertNode {
            node: unreachable_key,
            record: NodeRecord {
                ty: make_type_id("UnreachableType"),
            },
        }]);

        // Verify both nodes exist in the accumulator
        assert!(acc.nodes.contains_key(&reachable_key));
        assert!(acc.nodes.contains_key(&unreachable_key));

        // Compute reachability from root
        let root_key = NodeKey {
            warp_id,
            local_id: root_id,
        };
        let (reachable_nodes, _reachable_warps) = acc.compute_reachability(&root_key);

        // Verify reachable set contains root and reachable node, but not unreachable
        assert!(reachable_nodes.contains(&root_key));
        assert!(reachable_nodes.contains(&reachable_key));
        assert!(!reachable_nodes.contains(&unreachable_key));
    }

    #[test]
    fn test_state_root_deterministic() {
        // Create two identical accumulators via the same ops
        let warp_id = make_warp_id("determinism-test");
        let root_id = make_node_id("root");
        let root_key = NodeKey {
            warp_id,
            local_id: root_id,
        };

        let ops = vec![
            WarpOp::UpsertWarpInstance {
                instance: WarpInstance {
                    warp_id,
                    root_node: root_id,
                    parent: None,
                },
            },
            WarpOp::UpsertNode {
                node: root_key,
                record: NodeRecord {
                    ty: make_type_id("RootType"),
                },
            },
            WarpOp::UpsertNode {
                node: NodeKey {
                    warp_id,
                    local_id: make_node_id("child"),
                },
                record: NodeRecord {
                    ty: make_type_id("ChildType"),
                },
            },
            WarpOp::UpsertEdge {
                warp_id,
                record: EdgeRecord {
                    id: make_edge_id("root-to-child"),
                    from: root_id,
                    to: make_node_id("child"),
                    ty: make_type_id("ParentOf"),
                },
            },
        ];

        // First accumulator
        let mut acc1 = SnapshotAccumulator::new();
        acc1.apply_ops(ops.clone());

        // Second accumulator
        let mut acc2 = SnapshotAccumulator::new();
        acc2.apply_ops(ops);

        // Build both with the same parameters
        let schema_hash = [0xABu8; 32];
        let tick = 42;

        let output1 = acc1.build(&root_key, schema_hash, tick);
        let output2 = acc2.build(&root_key, schema_hash, tick);

        // Verify state_root is deterministic
        assert_eq!(
            output1.state_root, output2.state_root,
            "state_root must be deterministic for identical ops"
        );
    }

    #[test]
    fn test_delete_node_cascades_edges() {
        let (mut acc, warp_id, root_id) = setup_single_instance();

        // Create a node with edges
        let node_id = make_node_id("connected-node");
        let node_key = NodeKey {
            warp_id,
            local_id: node_id,
        };

        let edge_from_root = make_edge_id("from-root");
        let edge_to_root = make_edge_id("to-root");

        acc.apply_ops(vec![
            WarpOp::UpsertNode {
                node: node_key,
                record: NodeRecord {
                    ty: make_type_id("Connected"),
                },
            },
            WarpOp::UpsertEdge {
                warp_id,
                record: EdgeRecord {
                    id: edge_from_root,
                    from: root_id,
                    to: node_id,
                    ty: make_type_id("Link"),
                },
            },
            WarpOp::UpsertEdge {
                warp_id,
                record: EdgeRecord {
                    id: edge_to_root,
                    from: node_id,
                    to: root_id,
                    ty: make_type_id("BackLink"),
                },
            },
        ]);

        // Verify setup
        assert!(acc.edges.contains_key(&(warp_id, edge_from_root)));
        assert!(acc.edges.contains_key(&(warp_id, edge_to_root)));

        // Delete the connected node
        acc.apply_ops(vec![WarpOp::DeleteNode { node: node_key }]);

        // Verify node is gone
        assert!(!acc.nodes.contains_key(&node_key));

        // Verify incident edges are cascade-deleted
        assert!(!acc.edges.contains_key(&(warp_id, edge_from_root)));
        assert!(!acc.edges.contains_key(&(warp_id, edge_to_root)));
    }

    #[test]
    fn test_delete_warp_instance_cascades() {
        let (mut acc, warp_id, root_id) = setup_single_instance();

        // Add another node and edge to the instance
        let extra_node_id = make_node_id("extra");
        let extra_node_key = NodeKey {
            warp_id,
            local_id: extra_node_id,
        };
        let edge_id = make_edge_id("test-edge");

        acc.apply_ops(vec![
            WarpOp::UpsertNode {
                node: extra_node_key,
                record: NodeRecord {
                    ty: make_type_id("Extra"),
                },
            },
            WarpOp::UpsertEdge {
                warp_id,
                record: EdgeRecord {
                    id: edge_id,
                    from: root_id,
                    to: extra_node_id,
                    ty: make_type_id("Link"),
                },
            },
        ]);

        // Verify instance exists with content
        assert!(acc.instances.contains_key(&warp_id));
        assert_eq!(acc.nodes.len(), 2);
        assert_eq!(acc.edges.len(), 1);

        // Delete the entire instance
        acc.apply_ops(vec![WarpOp::DeleteWarpInstance { warp_id }]);

        // Verify everything is gone
        assert!(!acc.instances.contains_key(&warp_id));
        assert!(acc.nodes.is_empty());
        assert!(acc.edges.is_empty());
    }
}
