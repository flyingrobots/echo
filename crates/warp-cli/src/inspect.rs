// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! `echo-cli inspect` — display WSC snapshot metadata and graph statistics.
//!
//! Displays metadata (tick count, schema hash, warp count), graph statistics
//! (node/edge counts, type breakdown, connected components), and an optional
//! ASCII tree rendering of the graph structure.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::path::Path;

use anyhow::{Context, Result};
use bytes::Bytes;
use serde::Serialize;

use warp_core::wsc::types::AttRow;
use warp_core::wsc::view::WarpView;
use warp_core::wsc::{validate_wsc, WscFile};
use warp_core::{decode_motion_atom_payload, AtomPayload, TypeId};

use crate::cli::OutputFormat;
use crate::output::{emit, hex_hash, short_hex};
use crate::wsc_loader::graph_store_from_warp_view;

/// Metadata section of the inspect report.
#[derive(Debug, Serialize)]
pub(crate) struct Metadata {
    pub(crate) file: String,
    pub(crate) tick: u64,
    pub(crate) schema_hash: String,
    pub(crate) warp_count: usize,
}

/// Per-warp statistics.
#[derive(Debug, Serialize)]
pub(crate) struct WarpStats {
    pub(crate) warp_id: String,
    pub(crate) root_node_id: String,
    pub(crate) state_root: String,
    pub(crate) total_nodes: usize,
    pub(crate) total_edges: usize,
    pub(crate) node_types: BTreeMap<String, usize>,
    pub(crate) edge_types: BTreeMap<String, usize>,
    pub(crate) connected_components: usize,
    pub(crate) attachments: Vec<AttachmentSummary>,
}

/// Attachment payload display row.
#[derive(Debug, Serialize)]
pub(crate) struct AttachmentSummary {
    pub(crate) owner: String,
    pub(crate) owner_id: String,
    pub(crate) plane: String,
    pub(crate) kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) type_id: Option<String>,
    pub(crate) payload: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) warning: Option<String>,
}

/// Full inspect report.
#[derive(Debug, Serialize)]
pub(crate) struct InspectReport {
    pub(crate) metadata: Metadata,
    pub(crate) warps: Vec<WarpStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tree: Option<Vec<TreeNode>>,
}

/// A node in the ASCII tree rendering.
#[derive(Debug, Serialize)]
pub(crate) struct TreeNode {
    pub(crate) warp_index: usize,
    pub(crate) depth: usize,
    pub(crate) node_id: String,
    pub(crate) node_type: String,
    pub(crate) children: Vec<TreeNode>,
}

/// Limits tree rendering depth to prevent excessive output for wide/deep graphs.
const TREE_MAX_DEPTH: usize = 5;

/// Runs the inspect subcommand.
pub(crate) fn run(
    snapshot: &Path,
    show_tree: bool,
    raw_payloads: bool,
    format: &OutputFormat,
) -> Result<()> {
    let file = WscFile::open(snapshot)
        .with_context(|| format!("failed to open WSC file: {}", snapshot.display()))?;

    validate_wsc(&file)
        .with_context(|| format!("WSC validation failed: {}", snapshot.display()))?;

    let metadata = Metadata {
        file: snapshot.display().to_string(),
        tick: file.tick(),
        schema_hash: hex_hash(file.schema_hash()),
        warp_count: file.warp_count(),
    };

    let mut warp_stats = Vec::with_capacity(file.warp_count());
    let mut trees = if show_tree { Some(Vec::new()) } else { None };

    for i in 0..file.warp_count() {
        let view = file
            .warp_view(i)
            .with_context(|| format!("failed to read warp {i}"))?;

        let store = graph_store_from_warp_view(&view);
        let state_root = store.canonical_state_hash();

        let stats = compute_stats(&view, &state_root, raw_payloads);
        warp_stats.push(stats);

        if let Some(ref mut tree_list) = trees {
            let tree = build_tree(&view, i, TREE_MAX_DEPTH);
            tree_list.push(tree);
        }
    }

    let report = InspectReport {
        metadata,
        warps: warp_stats,
        tree: trees.map(|t| t.into_iter().flatten().collect()),
    };

    let text = format_text_report(&report);
    let json = serde_json::to_value(&report).context("failed to serialize inspect report")?;
    emit(format, &text, &json)?;

    Ok(())
}

fn compute_stats(view: &WarpView<'_>, state_root: &[u8; 32], raw_payloads: bool) -> WarpStats {
    let nodes = view.nodes();
    let edges = view.edges();

    // Type breakdown.
    let mut node_types: BTreeMap<String, usize> = BTreeMap::new();
    for n in nodes {
        *node_types.entry(short_hex(&n.node_type)).or_default() += 1;
    }

    let mut edge_types: BTreeMap<String, usize> = BTreeMap::new();
    for e in edges {
        *edge_types.entry(short_hex(&e.edge_type)).or_default() += 1;
    }

    // Connected components via BFS.
    let connected_components = count_connected_components(view);
    let attachments = collect_attachments(view, raw_payloads);

    WarpStats {
        warp_id: hex_hash(view.warp_id()),
        root_node_id: hex_hash(view.root_node_id()),
        state_root: hex_hash(state_root),
        total_nodes: nodes.len(),
        total_edges: edges.len(),
        node_types,
        edge_types,
        connected_components,
        attachments,
    }
}

fn collect_attachments(view: &WarpView<'_>, raw_payloads: bool) -> Vec<AttachmentSummary> {
    let mut attachments = Vec::new();
    for (node_ix, node) in view.nodes().iter().enumerate() {
        for att in view.node_attachments(node_ix) {
            attachments.push(format_attachment(
                "node",
                &node.node_id,
                "alpha",
                att,
                view,
                raw_payloads,
            ));
        }
    }
    for (edge_ix, edge) in view.edges().iter().enumerate() {
        for att in view.edge_attachments(edge_ix) {
            attachments.push(format_attachment(
                "edge",
                &edge.edge_id,
                "beta",
                att,
                view,
                raw_payloads,
            ));
        }
    }
    attachments
}

fn format_attachment(
    owner: &str,
    owner_id: &[u8; 32],
    plane: &str,
    att: &AttRow,
    view: &WarpView<'_>,
    raw_payloads: bool,
) -> AttachmentSummary {
    if att.is_descend() {
        return AttachmentSummary {
            owner: owner.to_string(),
            owner_id: short_hex(owner_id),
            plane: plane.to_string(),
            kind: "descend".to_string(),
            type_id: None,
            payload: format!("warp:{}", hex_hash(&att.type_or_warp)),
            warning: None,
        };
    }

    let type_id = TypeId(att.type_or_warp);
    let type_id_hex = hex_hash(&att.type_or_warp);
    let (bytes, missing_blob) = match view.blob_for_attachment(att) {
        Some(bytes) => (bytes, false),
        None => (&[][..], true),
    };
    let atom = AtomPayload::new(type_id, Bytes::copy_from_slice(bytes));
    let mut warning = missing_blob.then(|| "warning: missing attachment blob".to_string());
    let payload = if raw_payloads {
        hex_payload(&type_id_hex, bytes)
    } else if let Some((position, velocity)) = decode_motion_atom_payload(&atom) {
        format!(
            "position: ({}, {}, {}), velocity: ({}, {}, {})",
            decimal(position[0]),
            decimal(position[1]),
            decimal(position[2]),
            decimal(velocity[0]),
            decimal(velocity[1]),
            decimal(velocity[2])
        )
    } else {
        if is_motion_type(type_id) && warning.is_none() {
            warning = Some("warning: truncated or invalid motion payload".to_string());
        }
        hex_payload(&type_id_hex, bytes)
    };

    AttachmentSummary {
        owner: owner.to_string(),
        owner_id: short_hex(owner_id),
        plane: plane.to_string(),
        kind: "atom".to_string(),
        type_id: Some(type_id_hex),
        payload,
        warning,
    }
}

fn is_motion_type(type_id: TypeId) -> bool {
    type_id == warp_core::motion_payload_type_id()
        || type_id == warp_core::motion_payload_type_id_v0()
}

fn hex_payload(type_id: &str, bytes: &[u8]) -> String {
    format!("[type_id: {type_id}] 0x{}", hex::encode(bytes))
}

fn decimal(value: f32) -> String {
    let mut text = format!("{value:.6}");
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.push('0');
    }
    text
}

/// Counts connected components using BFS on the undirected graph.
fn count_connected_components(view: &WarpView<'_>) -> usize {
    let nodes = view.nodes();
    if nodes.is_empty() {
        return 0;
    }

    // Build adjacency from edges (undirected).
    // HashMap/HashSet: this is CLI-only code, not the deterministic engine.
    let mut adjacency: HashMap<[u8; 32], HashSet<[u8; 32]>> = HashMap::new();
    for n in nodes {
        adjacency.entry(n.node_id).or_default();
    }
    for e in view.edges() {
        adjacency
            .entry(e.from_node_id)
            .or_default()
            .insert(e.to_node_id);
        adjacency
            .entry(e.to_node_id)
            .or_default()
            .insert(e.from_node_id);
    }

    let mut visited: HashSet<[u8; 32]> = HashSet::new();
    let mut components = 0;

    for node in nodes {
        if visited.contains(&node.node_id) {
            continue;
        }

        // BFS from this node.
        let mut queue = VecDeque::new();
        queue.push_back(node.node_id);
        visited.insert(node.node_id);

        while let Some(current) = queue.pop_front() {
            if let Some(neighbors) = adjacency.get(&current) {
                for &neighbor in neighbors {
                    if visited.insert(neighbor) {
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        components += 1;
    }

    components
}

/// Builds an ASCII tree from the root node, depth-limited.
fn build_tree(view: &WarpView<'_>, warp_index: usize, max_depth: usize) -> Vec<TreeNode> {
    let root_id = *view.root_node_id();
    let root_ix = match view.node_ix(&root_id) {
        Some(ix) => ix,
        None => return vec![],
    };

    let root_node = &view.nodes()[root_ix];
    let mut visited = BTreeSet::new();
    visited.insert(root_id);

    vec![build_tree_node(
        view,
        warp_index,
        &root_id,
        &root_node.node_type,
        0,
        max_depth,
        &mut visited,
    )]
}

fn build_tree_node(
    view: &WarpView<'_>,
    warp_index: usize,
    node_id: &[u8; 32],
    node_type: &[u8; 32],
    depth: usize,
    max_depth: usize,
    visited: &mut BTreeSet<[u8; 32]>,
) -> TreeNode {
    let mut children = Vec::new();

    if depth < max_depth {
        if let Some(node_ix) = view.node_ix(node_id) {
            let out_edges = view.out_edges_for_node(node_ix);
            for out_edge in out_edges {
                if let Ok(edge_ix) = usize::try_from(out_edge.edge_ix()) {
                    if edge_ix < view.edges().len() {
                        let edge = &view.edges()[edge_ix];
                        let to_id = edge.to_node_id;

                        if visited.insert(to_id) {
                            if let Some(to_ix) = view.node_ix(&to_id) {
                                let to_node = &view.nodes()[to_ix];
                                children.push(build_tree_node(
                                    view,
                                    warp_index,
                                    &to_id,
                                    &to_node.node_type,
                                    depth + 1,
                                    max_depth,
                                    visited,
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    TreeNode {
        warp_index,
        depth,
        node_id: short_hex(node_id),
        node_type: short_hex(node_type),
        children,
    }
}

fn format_text_report(report: &InspectReport) -> String {
    use std::fmt::Write;

    let mut out = String::new();
    let _ = writeln!(out, "echo-cli inspect");
    let _ = writeln!(out, "  File: {}", report.metadata.file);
    let _ = writeln!(out, "  Tick: {}", report.metadata.tick);
    let _ = writeln!(out, "  Schema: {}", report.metadata.schema_hash);
    let _ = writeln!(out, "  Warps: {}", report.metadata.warp_count);
    let _ = writeln!(out);

    for (i, w) in report.warps.iter().enumerate() {
        let _ = writeln!(out, "  Warp {i}:");
        let _ = writeln!(out, "    ID:         {}", w.warp_id);
        let _ = writeln!(out, "    Root node:  {}", w.root_node_id);
        let _ = writeln!(out, "    State root: {}", w.state_root);
        let _ = writeln!(out, "    Nodes:      {}", w.total_nodes);
        let _ = writeln!(out, "    Edges:      {}", w.total_edges);
        let _ = writeln!(out, "    Components: {}", w.connected_components);

        if !w.node_types.is_empty() {
            let _ = writeln!(out, "    Node types:");
            for (ty, count) in &w.node_types {
                let _ = writeln!(out, "      {ty}: {count}");
            }
        }

        if !w.edge_types.is_empty() {
            let _ = writeln!(out, "    Edge types:");
            for (ty, count) in &w.edge_types {
                let _ = writeln!(out, "      {ty}: {count}");
            }
        }

        if !w.attachments.is_empty() {
            let _ = writeln!(out, "    Attachments:");
            for attachment in &w.attachments {
                let type_suffix = attachment
                    .type_id
                    .as_ref()
                    .map(|type_id| format!(" type_id={type_id}"))
                    .unwrap_or_default();
                let _ = writeln!(
                    out,
                    "      {} {} {} {}{}: {}",
                    attachment.owner,
                    attachment.owner_id,
                    attachment.plane,
                    attachment.kind,
                    type_suffix,
                    attachment.payload
                );
                if let Some(warning) = &attachment.warning {
                    let _ = writeln!(out, "        {warning}");
                }
            }
        }
        let _ = writeln!(out);
    }

    if let Some(ref tree) = report.tree {
        let multi_warp = report.metadata.warp_count > 1;
        let mut current_warp: Option<usize> = None;
        for node in tree {
            if multi_warp && (current_warp != Some(node.warp_index)) {
                let _ = writeln!(out, "  Tree (warp {}):", node.warp_index);
                current_warp = Some(node.warp_index);
            } else if !multi_warp && current_warp.is_none() {
                let _ = writeln!(out, "  Tree:");
                current_warp = Some(0);
            }
            format_tree_node(&mut out, node, "", true);
        }
        let _ = writeln!(out);
    }

    out
}

fn format_tree_node(out: &mut String, node: &TreeNode, prefix: &str, is_last: bool) {
    use std::fmt::Write;

    let connector = if node.depth == 0 {
        ""
    } else if is_last {
        "\u{2514}\u{2500}\u{2500} "
    } else {
        "\u{251c}\u{2500}\u{2500} "
    };

    let _ = writeln!(
        out,
        "    {prefix}{connector}[{}] type={}",
        node.node_id, node.node_type
    );

    let child_prefix = if node.depth == 0 {
        String::new()
    } else if is_last {
        format!("{prefix}    ")
    } else {
        format!("{prefix}\u{2502}   ")
    };

    for (i, child) in node.children.iter().enumerate() {
        let last = i + 1 == node.children.len();
        format_tree_node(out, child, &child_prefix, last);
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::io::Write as IoWrite;
    use tempfile::NamedTempFile;
    use warp_core::wsc::build::build_one_warp_input;
    use warp_core::wsc::write::write_wsc_one_warp;
    use warp_core::{
        encode_motion_atom_payload, make_edge_id, make_node_id, make_type_id, make_warp_id,
        motion_payload_type_id, AttachmentValue, EdgeRecord, GraphStore, NodeRecord,
    };

    fn make_test_graph() -> (GraphStore, warp_core::NodeId) {
        let warp = make_warp_id("test");
        let node_ty = make_type_id("Actor");
        let child_ty = make_type_id("Item");
        let edge_ty = make_type_id("HasItem");
        let root = make_node_id("root");
        let child1 = make_node_id("child1");
        let child2 = make_node_id("child2");

        let mut store = GraphStore::new(warp);
        store.insert_node(root, NodeRecord { ty: node_ty });
        store.insert_node(child1, NodeRecord { ty: child_ty });
        store.insert_node(child2, NodeRecord { ty: child_ty });
        store.insert_edge(
            root,
            EdgeRecord {
                id: make_edge_id("root->child1"),
                from: root,
                to: child1,
                ty: edge_ty,
            },
        );
        store.insert_edge(
            root,
            EdgeRecord {
                id: make_edge_id("root->child2"),
                from: root,
                to: child2,
                ty: edge_ty,
            },
        );

        (store, root)
    }

    fn make_test_wsc() -> Vec<u8> {
        let (store, root) = make_test_graph();
        let input = build_one_warp_input(&store, root);
        write_wsc_one_warp(&input, [0u8; 32], 42).expect("WSC write")
    }

    fn make_motion_attachment_wsc(raw_payload: Option<AtomPayload>) -> Vec<u8> {
        let warp = make_warp_id("test");
        let node_ty = make_type_id("Actor");
        let root = make_node_id("root");
        let mut store = GraphStore::new(warp);
        store.insert_node(root, NodeRecord { ty: node_ty });
        let payload = raw_payload
            .unwrap_or_else(|| encode_motion_atom_payload([1.0, 2.5, -3.0], [0.25, -0.5, 4.0]));
        store.set_node_attachment(root, Some(AttachmentValue::Atom(payload)));
        let input = build_one_warp_input(&store, root);
        write_wsc_one_warp(&input, [0u8; 32], 42).expect("WSC write")
    }

    fn write_temp_wsc(data: &[u8]) -> NamedTempFile {
        let mut f = NamedTempFile::new().expect("tempfile");
        f.write_all(data).expect("write");
        f.flush().expect("flush");
        f
    }

    #[test]
    fn metadata_fields_present() {
        let wsc = make_test_wsc();
        let f = write_temp_wsc(&wsc);
        let result = run(f.path(), false, false, &OutputFormat::Text);
        assert!(result.is_ok());
    }

    #[test]
    fn type_breakdown_sums_to_total() {
        let wsc = make_test_wsc();
        let file = WscFile::from_bytes(wsc).unwrap();
        let view = file.warp_view(0).unwrap();
        let store = graph_store_from_warp_view(&view);
        let state_root = store.canonical_state_hash();

        let stats = compute_stats(&view, &state_root, false);

        let node_type_sum: usize = stats.node_types.values().sum();
        assert_eq!(node_type_sum, stats.total_nodes);

        let edge_type_sum: usize = stats.edge_types.values().sum();
        assert_eq!(edge_type_sum, stats.total_edges);
    }

    #[test]
    fn tree_shows_root_at_depth_zero() {
        let wsc = make_test_wsc();
        let file = WscFile::from_bytes(wsc).unwrap();
        let view = file.warp_view(0).unwrap();

        let tree = build_tree(&view, 0, 5);
        assert!(!tree.is_empty());
        assert_eq!(tree[0].depth, 0);
        assert_eq!(tree[0].warp_index, 0);
    }

    #[test]
    fn tree_shows_children_indented() {
        let wsc = make_test_wsc();
        let file = WscFile::from_bytes(wsc).unwrap();
        let view = file.warp_view(0).unwrap();

        let tree = build_tree(&view, 0, 5);
        assert!(!tree.is_empty());
        // Root should have children from edges.
        assert!(!tree[0].children.is_empty(), "root should have children");
        for child in &tree[0].children {
            assert_eq!(child.depth, 1);
        }
    }

    #[test]
    fn json_includes_metadata_and_stats() {
        let wsc = make_test_wsc();
        let f = write_temp_wsc(&wsc);
        // Verify JSON mode doesn't panic.
        let result = run(f.path(), false, false, &OutputFormat::Json);
        assert!(result.is_ok());
    }

    #[test]
    fn motion_attachment_displays_decoded_fields() {
        let wsc = make_motion_attachment_wsc(None);
        let file = WscFile::from_bytes(wsc).unwrap();
        let view = file.warp_view(0).unwrap();
        let store = graph_store_from_warp_view(&view);
        let state_root = store.canonical_state_hash();

        let stats = compute_stats(&view, &state_root, false);

        assert_eq!(stats.attachments.len(), 1);
        assert_eq!(stats.attachments[0].kind, "atom");
        assert!(stats.attachments[0]
            .payload
            .contains("position: (1.0, 2.5, -3.0), velocity: (0.25, -0.5, 4.0)"));
        assert!(stats.attachments[0].warning.is_none());
    }

    #[test]
    fn raw_attachment_displays_hex_for_known_payload() {
        let wsc = make_motion_attachment_wsc(None);
        let file = WscFile::from_bytes(wsc).unwrap();
        let view = file.warp_view(0).unwrap();
        let store = graph_store_from_warp_view(&view);
        let state_root = store.canonical_state_hash();

        let stats = compute_stats(&view, &state_root, true);

        assert!(stats.attachments[0].payload.starts_with("[type_id: "));
        assert!(stats.attachments[0].payload.contains("] 0x"));
        assert!(!stats.attachments[0].payload.contains("position:"));
    }

    #[test]
    fn unknown_attachment_type_displays_type_id_and_hex() {
        let payload = AtomPayload::new(make_type_id("OtherPayload"), Bytes::from_static(b"Hello"));
        let wsc = make_motion_attachment_wsc(Some(payload));
        let file = WscFile::from_bytes(wsc).unwrap();
        let view = file.warp_view(0).unwrap();
        let store = graph_store_from_warp_view(&view);
        let state_root = store.canonical_state_hash();

        let stats = compute_stats(&view, &state_root, false);

        assert!(stats.attachments[0].payload.starts_with("[type_id: "));
        assert!(stats.attachments[0].payload.ends_with("0x48656c6c6f"));
        assert!(stats.attachments[0].warning.is_none());
    }

    #[test]
    fn truncated_motion_attachment_warns_and_falls_back_to_hex() {
        let payload = AtomPayload::new(motion_payload_type_id(), Bytes::from_static(&[1, 2, 3]));
        let wsc = make_motion_attachment_wsc(Some(payload));
        let file = WscFile::from_bytes(wsc).unwrap();
        let view = file.warp_view(0).unwrap();
        let store = graph_store_from_warp_view(&view);
        let state_root = store.canonical_state_hash();

        let stats = compute_stats(&view, &state_root, false);

        assert_eq!(
            stats.attachments[0].warning.as_deref(),
            Some("warning: truncated or invalid motion payload")
        );
        assert!(stats.attachments[0].payload.ends_with("0x010203"));
    }

    #[test]
    fn connected_components_single_graph() {
        let wsc = make_test_wsc();
        let file = WscFile::from_bytes(wsc).unwrap();
        let view = file.warp_view(0).unwrap();
        let components = count_connected_components(&view);
        assert_eq!(
            components, 1,
            "single connected graph should have 1 component"
        );
    }

    #[test]
    fn connected_components_empty_graph() {
        let warp = make_warp_id("test");
        let store = GraphStore::new(warp);
        let zero_root = warp_core::NodeId([0u8; 32]);
        let input = build_one_warp_input(&store, zero_root);
        let wsc = write_wsc_one_warp(&input, [0u8; 32], 0).unwrap();
        let file = WscFile::from_bytes(wsc).unwrap();
        let view = file.warp_view(0).unwrap();
        assert_eq!(count_connected_components(&view), 0);
    }

    #[test]
    fn connected_components_disconnected_nodes() {
        let warp = make_warp_id("test");
        let node_ty = make_type_id("Node");
        let a = make_node_id("a");
        let b = make_node_id("b");

        let mut store = GraphStore::new(warp);
        store.insert_node(a, NodeRecord { ty: node_ty });
        store.insert_node(b, NodeRecord { ty: node_ty });
        // No edges — two disconnected nodes.

        let input = build_one_warp_input(&store, a);
        let wsc = write_wsc_one_warp(&input, [0u8; 32], 0).unwrap();
        let file = WscFile::from_bytes(wsc).unwrap();
        let view = file.warp_view(0).unwrap();
        assert_eq!(count_connected_components(&view), 2);
    }
}
