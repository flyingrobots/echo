// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! `echo-cli inspect` — display WSC snapshot metadata and graph statistics.
//!
//! Prints metadata (tick count, schema hash, warp count), graph statistics
//! (node/edge counts, type breakdown, connected components), and an optional
//! ASCII tree rendering of the graph structure.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::Path;

use anyhow::{Context, Result};
use serde::Serialize;

use warp_core::wsc::view::WarpView;
use warp_core::wsc::{validate_wsc, WscFile};

use crate::cli::OutputFormat;
use crate::output::{emit, hex_hash, short_hex};
use crate::wsc_loader::graph_store_from_warp_view;

/// Metadata section of the inspect report.
#[derive(Debug, Serialize)]
pub struct Metadata {
    pub file: String,
    pub tick: u64,
    pub schema_hash: String,
    pub warp_count: usize,
}

/// Per-warp statistics.
#[derive(Debug, Serialize)]
pub struct WarpStats {
    pub warp_id: String,
    pub root_node_id: String,
    pub state_root: String,
    pub total_nodes: usize,
    pub total_edges: usize,
    pub node_types: BTreeMap<String, usize>,
    pub edge_types: BTreeMap<String, usize>,
    pub connected_components: usize,
}

/// Full inspect report.
#[derive(Debug, Serialize)]
pub struct InspectReport {
    pub metadata: Metadata,
    pub warps: Vec<WarpStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tree: Option<Vec<TreeNode>>,
}

/// A node in the ASCII tree rendering.
#[derive(Debug, Serialize)]
pub struct TreeNode {
    pub depth: usize,
    pub node_id: String,
    pub node_type: String,
    pub children: Vec<TreeNode>,
}

/// Runs the inspect subcommand.
pub fn run(snapshot: &Path, show_tree: bool, format: &OutputFormat) -> Result<()> {
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

        let stats = compute_stats(&view, &state_root);
        warp_stats.push(stats);

        if let Some(ref mut tree_list) = trees {
            let tree = build_tree(&view, 5);
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
    emit(format, &text, &json);

    Ok(())
}

fn compute_stats(view: &WarpView<'_>, state_root: &[u8; 32]) -> WarpStats {
    let nodes = view.nodes();
    let edges = view.edges();

    // Type breakdown.
    let mut node_types: BTreeMap<String, usize> = BTreeMap::new();
    for n in nodes {
        *node_types.entry(short_hex(&n.node_type)).or_insert(0) += 1;
    }

    let mut edge_types: BTreeMap<String, usize> = BTreeMap::new();
    for e in edges {
        *edge_types.entry(short_hex(&e.edge_type)).or_insert(0) += 1;
    }

    // Connected components via BFS.
    let connected_components = count_connected_components(view);

    WarpStats {
        warp_id: hex_hash(view.warp_id()),
        root_node_id: hex_hash(view.root_node_id()),
        state_root: hex_hash(state_root),
        total_nodes: nodes.len(),
        total_edges: edges.len(),
        node_types,
        edge_types,
        connected_components,
    }
}

/// Counts connected components using BFS on the undirected graph.
fn count_connected_components(view: &WarpView<'_>) -> usize {
    let nodes = view.nodes();
    if nodes.is_empty() {
        return 0;
    }

    // Build adjacency from edges (undirected).
    let mut adjacency: BTreeMap<[u8; 32], BTreeSet<[u8; 32]>> = BTreeMap::new();
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

    let mut visited: BTreeSet<[u8; 32]> = BTreeSet::new();
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
fn build_tree(view: &WarpView<'_>, max_depth: usize) -> Vec<TreeNode> {
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
        &root_id,
        &root_node.node_type,
        0,
        max_depth,
        &mut visited,
    )]
}

fn build_tree_node(
    view: &WarpView<'_>,
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
                let edge_ix = out_edge.edge_ix() as usize;
                if edge_ix < view.edges().len() {
                    let edge = &view.edges()[edge_ix];
                    let to_id = edge.to_node_id;

                    if visited.insert(to_id) {
                        if let Some(to_ix) = view.node_ix(&to_id) {
                            let to_node = &view.nodes()[to_ix];
                            children.push(build_tree_node(
                                view,
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

    TreeNode {
        depth,
        node_id: short_hex(node_id),
        node_type: short_hex(node_type),
        children,
    }
}

fn format_text_report(report: &InspectReport) -> String {
    use std::fmt::Write;

    let mut out = String::new();
    writeln!(out, "echo-cli inspect").ok();
    writeln!(out, "  File: {}", report.metadata.file).ok();
    writeln!(out, "  Tick: {}", report.metadata.tick).ok();
    writeln!(out, "  Schema: {}", report.metadata.schema_hash).ok();
    writeln!(out, "  Warps: {}", report.metadata.warp_count).ok();
    writeln!(out).ok();

    for (i, w) in report.warps.iter().enumerate() {
        writeln!(out, "  Warp {i}:").ok();
        writeln!(out, "    ID:         {}", w.warp_id).ok();
        writeln!(out, "    Root node:  {}", w.root_node_id).ok();
        writeln!(out, "    State root: {}", w.state_root).ok();
        writeln!(out, "    Nodes:      {}", w.total_nodes).ok();
        writeln!(out, "    Edges:      {}", w.total_edges).ok();
        writeln!(out, "    Components: {}", w.connected_components).ok();

        if !w.node_types.is_empty() {
            writeln!(out, "    Node types:").ok();
            for (ty, count) in &w.node_types {
                writeln!(out, "      {ty}: {count}").ok();
            }
        }

        if !w.edge_types.is_empty() {
            writeln!(out, "    Edge types:").ok();
            for (ty, count) in &w.edge_types {
                writeln!(out, "      {ty}: {count}").ok();
            }
        }
        writeln!(out).ok();
    }

    if let Some(ref tree) = report.tree {
        writeln!(out, "  Tree:").ok();
        for node in tree {
            format_tree_node(&mut out, node, "", true);
        }
        writeln!(out).ok();
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

    writeln!(
        out,
        "    {prefix}{connector}[{}] type={}",
        node.node_id, node.node_type
    )
    .ok();

    let child_prefix = if node.depth == 0 {
        String::new()
    } else if is_last {
        format!("{prefix}    ")
    } else {
        format!("{prefix}\u{2502}   ")
    };

    for (i, child) in node.children.iter().enumerate() {
        let last = i == node.children.len() - 1;
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
        make_edge_id, make_node_id, make_type_id, make_warp_id, EdgeRecord, GraphStore, NodeRecord,
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
        let result = run(f.path(), false, &OutputFormat::Text);
        assert!(result.is_ok());
    }

    #[test]
    fn type_breakdown_sums_to_total() {
        let wsc = make_test_wsc();
        let file = WscFile::from_bytes(wsc).unwrap();
        let view = file.warp_view(0).unwrap();
        let store = graph_store_from_warp_view(&view);
        let state_root = store.canonical_state_hash();

        let stats = compute_stats(&view, &state_root);

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

        let tree = build_tree(&view, 5);
        assert!(!tree.is_empty());
        assert_eq!(tree[0].depth, 0);
    }

    #[test]
    fn tree_shows_children_indented() {
        let wsc = make_test_wsc();
        let file = WscFile::from_bytes(wsc).unwrap();
        let view = file.warp_view(0).unwrap();

        let tree = build_tree(&view, 5);
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
        let result = run(f.path(), false, &OutputFormat::Json);
        assert!(result.is_ok());
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
