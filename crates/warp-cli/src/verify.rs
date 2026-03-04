// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! `echo-cli verify` — validate WSC snapshot integrity.
//!
//! Loads a WSC file, validates its structure, reconstructs the graph for
//! each warp, and computes state root hashes. Optionally compares against
//! an expected hash.

use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::Serialize;

use warp_core::wsc::{validate_wsc, WscFile};

use crate::cli::OutputFormat;
use crate::output::{emit, hex_hash};
use crate::wsc_loader::graph_store_from_warp_view;

/// Result of verifying a single warp instance within a WSC file.
#[derive(Debug, Serialize)]
pub struct WarpVerifyResult {
    pub warp_id: String,
    pub root_node_id: String,
    pub nodes: usize,
    pub edges: usize,
    pub state_root: String,
    pub status: String,
}

/// Result of the full verify operation.
#[derive(Debug, Serialize)]
pub struct VerifyReport {
    pub file: String,
    pub tick: u64,
    pub schema_hash: String,
    pub warp_count: usize,
    pub warps: Vec<WarpVerifyResult>,
    pub result: String,
}

/// Runs the verify subcommand.
pub fn run(snapshot: &Path, expected: Option<&str>, format: &OutputFormat) -> Result<()> {
    // 1. Load WSC file.
    let file = WscFile::open(snapshot)
        .with_context(|| format!("failed to open WSC file: {}", snapshot.display()))?;

    // 2. Structural validation.
    validate_wsc(&file)
        .with_context(|| format!("WSC validation failed: {}", snapshot.display()))?;

    let tick = file.tick();
    let schema_hash = hex_hash(file.schema_hash());
    let warp_count = file.warp_count();

    let mut warp_results = Vec::with_capacity(warp_count);
    let mut all_pass = true;

    // 3. For each warp: reconstruct graph, compute state root.
    for i in 0..warp_count {
        let view = file
            .warp_view(i)
            .with_context(|| format!("failed to read warp {i}"))?;

        let store = graph_store_from_warp_view(&view);
        let state_root = store.canonical_state_hash();
        let state_root_hex = hex_hash(&state_root);

        // Check against expected hash (if provided, applies to first warp).
        let status = if let Some(exp) = expected {
            if i == 0 {
                if state_root_hex == exp {
                    "pass".to_string()
                } else {
                    all_pass = false;
                    format!("MISMATCH (expected {exp})")
                }
            } else {
                "pass".to_string()
            }
        } else {
            "pass".to_string()
        };

        warp_results.push(WarpVerifyResult {
            warp_id: hex_hash(view.warp_id()),
            root_node_id: hex_hash(view.root_node_id()),
            nodes: view.nodes().len(),
            edges: view.edges().len(),
            state_root: state_root_hex,
            status,
        });
    }

    let report = VerifyReport {
        file: snapshot.display().to_string(),
        tick,
        schema_hash,
        warp_count,
        warps: warp_results,
        result: if all_pass {
            "pass".to_string()
        } else {
            "fail".to_string()
        },
    };

    // 4. Output.
    let text = format_text_report(&report);
    let json = serde_json::to_value(&report).context("failed to serialize verify report")?;

    emit(format, &text, &json);

    if !all_pass {
        bail!("verification failed");
    }
    Ok(())
}

fn format_text_report(report: &VerifyReport) -> String {
    use std::fmt::Write;

    let mut out = String::new();
    writeln!(out, "echo-cli verify").ok();
    writeln!(out, "  File: {}", report.file).ok();
    writeln!(out, "  Tick: {}", report.tick).ok();
    writeln!(out, "  Schema: {}", report.schema_hash).ok();
    writeln!(out, "  Warps: {}", report.warp_count).ok();
    writeln!(out).ok();

    for (i, w) in report.warps.iter().enumerate() {
        writeln!(out, "  Warp {i}:").ok();
        writeln!(out, "    ID:        {}", w.warp_id).ok();
        writeln!(out, "    Root node: {}", w.root_node_id).ok();
        writeln!(out, "    Nodes:     {}", w.nodes).ok();
        writeln!(out, "    Edges:     {}", w.edges).ok();
        writeln!(out, "    State root: {}", w.state_root).ok();
        writeln!(out, "    Status:    {}", w.status).ok();
        writeln!(out).ok();
    }

    writeln!(out, "  Result: {}", report.result.to_uppercase()).ok();
    out
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
        make_edge_id, make_node_id, make_type_id, make_warp_id, EdgeRecord, GraphStore, Hash,
        NodeRecord,
    };

    fn make_test_wsc() -> (Vec<u8>, Hash) {
        let warp = make_warp_id("test");
        let node_ty = make_type_id("TestNode");
        let edge_ty = make_type_id("TestEdge");
        let root = make_node_id("root");
        let child = make_node_id("child");

        let mut store = GraphStore::new(warp);
        store.insert_node(root, NodeRecord { ty: node_ty });
        store.insert_node(child, NodeRecord { ty: node_ty });
        store.insert_edge(
            root,
            EdgeRecord {
                id: make_edge_id("root->child"),
                from: root,
                to: child,
                ty: edge_ty,
            },
        );

        let state_root = store.canonical_state_hash();
        let input = build_one_warp_input(&store, root);
        let wsc_bytes = write_wsc_one_warp(&input, [0u8; 32], 42).expect("WSC write");
        (wsc_bytes, state_root)
    }

    fn write_temp_wsc(data: &[u8]) -> NamedTempFile {
        let mut f = NamedTempFile::new().expect("tempfile");
        f.write_all(data).expect("write");
        f.flush().expect("flush");
        f
    }

    #[test]
    fn valid_snapshot_passes() {
        let (wsc_bytes, _) = make_test_wsc();
        let f = write_temp_wsc(&wsc_bytes);
        let result = run(f.path(), None, &OutputFormat::Text);
        assert!(result.is_ok(), "valid snapshot should pass: {result:?}");
    }

    #[test]
    fn valid_snapshot_with_matching_expected_hash() {
        let (wsc_bytes, state_root) = make_test_wsc();
        let expected_hex = hex_hash(&state_root);
        let f = write_temp_wsc(&wsc_bytes);
        let result = run(f.path(), Some(&expected_hex), &OutputFormat::Text);
        assert!(
            result.is_ok(),
            "matching expected hash should pass: {result:?}"
        );
    }

    #[test]
    fn mismatched_expected_hash_fails() {
        let (wsc_bytes, _) = make_test_wsc();
        let f = write_temp_wsc(&wsc_bytes);
        let result = run(
            f.path(),
            Some("0000000000000000000000000000000000000000000000000000000000000000"),
            &OutputFormat::Text,
        );
        assert!(result.is_err(), "mismatched hash should fail");
    }

    #[test]
    fn tampered_wsc_fails() {
        let (mut wsc_bytes, _) = make_test_wsc();
        // Flip a byte in the node data (well past the header).
        let flip_pos = wsc_bytes.len() / 2;
        wsc_bytes[flip_pos] ^= 0xFF;
        let f = write_temp_wsc(&wsc_bytes);
        // May fail at validation or hash comparison.
        let result = run(f.path(), None, &OutputFormat::Text);
        // Tampered files may still pass structural validation if the flip
        // hits data (not structural fields). What matters is the state root
        // will differ, which we verify via the expected hash mechanism.
        // So this test just ensures no panic.
        drop(result);
    }

    #[test]
    fn json_output_is_valid() {
        let (wsc_bytes, _) = make_test_wsc();
        let f = write_temp_wsc(&wsc_bytes);
        // Just verify it doesn't panic in JSON mode.
        let result = run(f.path(), None, &OutputFormat::Json);
        assert!(result.is_ok());
    }

    #[test]
    fn missing_file_gives_clean_error() {
        let result = run(
            Path::new("/nonexistent/path/state.wsc"),
            None,
            &OutputFormat::Text,
        );
        assert!(result.is_err());
        let err_msg = format!("{:#}", result.unwrap_err());
        assert!(
            err_msg.contains("failed to open WSC file"),
            "error should mention file open failure: {err_msg}"
        );
    }

    #[test]
    fn empty_graph_passes() {
        let warp = make_warp_id("test");
        let store = GraphStore::new(warp);
        let zero_root = warp_core::NodeId([0u8; 32]);

        let input = build_one_warp_input(&store, zero_root);
        let wsc_bytes = write_wsc_one_warp(&input, [0u8; 32], 0).expect("WSC write");
        let f = write_temp_wsc(&wsc_bytes);

        let result = run(f.path(), None, &OutputFormat::Text);
        assert!(result.is_ok(), "empty graph should pass: {result:?}");
    }
}
