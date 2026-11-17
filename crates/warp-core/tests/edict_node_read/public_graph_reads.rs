// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Public compiler graph-read packages evaluated by the generic runtime.
//!
//! The packages are unchanged public compiler output retained under
//! `fixtures/edict-graph-read/`; their pins are recorded here independently
//! of the bytes. Oracle: every expected row is written by hand from the
//! fixture tree, never computed by the code under test.
#![allow(clippy::unwrap_used, clippy::panic)]

use super::graph_reads::support::{
    boundary_guide, canonical, closure_request, edge_request, edge_row, guide, limits, link, node,
    node_row, occurrence, put, record, sorted, tree,
};
use echo_edict_canonical::{
    decode_canonical_cbor_v1 as decode, digest_canonical_value_bytes_v1 as digest,
    encode_canonical_cbor_v1 as encode, CanonicalValueV1 as Value,
};
use warp_core::edict_read::{
    evaluate, GraphReadAperture, GraphReadObstruction, ReadError, ReadResult, ReadView,
};
use warp_core::{
    make_edge_id, GraphStore, NodeKey, WorldlineFrontier, WorldlineId, WorldlineState,
};

const EDGE: &str =
    include_str!("../fixtures/edict-graph-read/edge-executable-operation-package.hex");
const CLOSURE: &str =
    include_str!("../fixtures/edict-graph-read/closure-executable-operation-package.hex");
const EDGE_PIN: &str = "1f2ef3ce45822995465e9250f8a98083e23dbf2c1cd682f5372826099580b926";
const CLOSURE_PIN: &str = "401680675b8364415b2d6e0768ba386ccc7085ad40b3c9e0021ed87bb1b90e2c";

fn package(hex_bytes: &str, pin: &str) -> (Vec<u8>, [u8; 32]) {
    let bytes = hex::decode(hex_bytes.trim()).unwrap();
    // The recorded pin is the package's domain-framed identity.
    let identity = digest("echo.operation-package/v1", &decode(&bytes).unwrap()).unwrap();
    assert_eq!(hex::encode(identity), pin, "retained package identity");
    (bytes, identity)
}

fn run(
    frontier: &WorldlineFrontier,
    start: NodeKey,
    (bytes, pin): &(Vec<u8>, [u8; 32]),
    read: Value,
) -> Result<ReadResult, ReadError> {
    let aperture = [start];
    let (edge_types, node_types) = (
        sorted(&["left", "right", "decoy"]),
        sorted(&["branch", "leaf"]),
    );
    let view = ReadView::at(frontier, &aperture)
        .unwrap()
        .with_graph_aperture(GraphReadAperture {
            edge_types: &edge_types,
            node_types: &node_types,
            max_edge_scans: 64,
        })
        .unwrap();
    let input = encode(&record([("read", read)])).unwrap();
    evaluate(bytes, *pin, &input, &view, limits())
}

fn output(result: Result<ReadResult, ReadError>) -> Value {
    decode(&result.unwrap().output).unwrap()
}

fn cause(result: Result<ReadResult, ReadError>) -> GraphReadObstruction {
    match result {
        Err(ReadError::GraphObstructed { cause, .. }) => cause,
        other => panic!("expected an authored graph obstruction, got {other:?}"),
    }
}

#[test]
fn the_public_edge_program_reads_present_and_absent_typed_edges() {
    let edge = package(EDGE, EDGE_PIN);
    let (frontier, root) = tree(|_| {});
    let edges =
        |edge_type: &str| output(run(&frontier, root, &edge, edge_request(root, edge_type)));
    // Expected rows are compared in canonical form, as the runtime emits them.
    let row = |name: &str, to: &str| {
        decode(&canonical(&record([
            ("edgeId", Value::Bytes(make_edge_id(name).0.to_vec())),
            ("toId", Value::Bytes(node(to).0.to_vec())),
        ])))
        .unwrap()
    };
    assert_eq!(
        edges("left"),
        Value::Array(vec![row("root-left-lower", "lower")])
    );
    assert_eq!(
        edges("decoy"),
        Value::Array(vec![row("root-decoy-l1", "l1")])
    );
    // A present source with no edge of the requested type reads nothing.
    let leaf = NodeKey {
        local_id: node("l1"),
        ..root
    };
    assert_eq!(
        output(run(&frontier, leaf, &edge, edge_request(leaf, "left"))),
        Value::Array(vec![])
    );
    let absent = NodeKey {
        local_id: node("never-stored"),
        ..root
    };
    assert_eq!(
        cause(run(&frontier, absent, &edge, edge_request(absent, "left"))),
        GraphReadObstruction::EdgeSourceMissing
    );
}

#[test]
fn the_public_closure_program_reads_both_boundary_paths() {
    let closure = package(CLOSURE, CLOSURE_PIN);
    let (frontier, root) = tree(|_| {});
    let expected = record([
        (
            "nodes",
            Value::Array(vec![
                node_row("root", "branch", b"...."),
                node_row("lower", "branch", b".."),
                node_row("upper", "branch", b"......"),
                node_row("l1", "leaf", b"ab"),
                node_row("l2", "leaf", b"cd"),
                node_row("l3", "leaf", b"ef"),
                node_row("l4", "leaf", b"gh"),
            ]),
        ),
        (
            "edges",
            Value::Array(vec![
                edge_row(0, 0, "root-left-lower", 1),
                edge_row(0, 1, "root-right-upper", 2),
                edge_row(1, 0, "lower-left-l1", 3),
                edge_row(1, 1, "lower-right-l2", 4),
                edge_row(2, 0, "upper-left-l3", 5),
                edge_row(2, 1, "upper-right-l4", 6),
            ]),
        ),
        (
            "occurrences",
            Value::Array(vec![
                occurrence(0, 0, 0, guide(1, 6, true, true, 2)),
                occurrence(1, 0, 1, guide(1, 6, true, false, 2)),
                occurrence(2, 1, 1, guide(1, 6, false, true, 2)),
                occurrence(3, 2, 2, guide(1, 6, true, false, 2)),
                occurrence(6, 5, 2, guide(1, 6, false, true, 2)),
            ]),
        ),
    ]);
    let read = closure_request(root, boundary_guide(), 7, 2);
    assert_eq!(
        output(run(&frontier, root, &closure, read)),
        decode(&canonical(&expected)).unwrap()
    );
}

#[test]
fn the_public_closure_program_keeps_shared_content_at_two_occurrences() {
    let closure = package(CLOSURE, CLOSURE_PIN);
    let mut store = GraphStore::default();
    put(&mut store, "root", "branch", b"....");
    put(&mut store, "shared", "leaf", b"abcd");
    link(&mut store, "root", "left", "shared");
    link(&mut store, "root", "right", "shared");
    let root = NodeKey {
        warp_id: store.warp_id(),
        local_id: node("root"),
    };
    let frontier = WorldlineFrontier::new(
        WorldlineId::from_bytes([7; 32]),
        WorldlineState::from_root_store(store, root.local_id).unwrap(),
    );
    let read = closure_request(root, guide(1, 6, true, true, 1), 2, 1);
    let expected = record([
        (
            "nodes",
            Value::Array(vec![
                node_row("root", "branch", b"...."),
                node_row("shared", "leaf", b"abcd"),
            ]),
        ),
        (
            "edges",
            Value::Array(vec![
                edge_row(0, 0, "root-left-shared", 1),
                edge_row(0, 1, "root-right-shared", 1),
            ]),
        ),
        (
            "occurrences",
            Value::Array(vec![
                occurrence(0, 0, 0, guide(1, 6, true, true, 1)),
                occurrence(1, 0, 1, guide(1, 6, true, false, 1)),
                occurrence(1, 1, 1, guide(1, 6, false, true, 1)),
            ]),
        ),
    ]);
    assert_eq!(
        output(run(&frontier, root, &closure, read)),
        decode(&canonical(&expected)).unwrap()
    );
}

#[test]
fn the_public_closure_program_keeps_limit_coverage_and_storage_outcomes_distinct() {
    let closure = package(CLOSURE, CLOSURE_PIN);
    let (frontier, root) = tree(|_| {});
    let read = |guide, depth| closure_request(root, guide, 7, depth);
    assert_eq!(
        cause(run(&frontier, root, &closure, read(boundary_guide(), 1))),
        GraphReadObstruction::ClosureDepthBoundExceeded
    );
    assert_eq!(
        cause(run(
            &frontier,
            root,
            &closure,
            read(guide(1, 6, true, true, 1), 2)
        )),
        GraphReadObstruction::ClosureSelectorInsufficient
    );
    let (empty, start) = tree(|store| put(store, "l1", "leaf", b""));
    assert_eq!(
        cause(run(&empty, start, &closure, read(boundary_guide(), 2))),
        GraphReadObstruction::ClosureStorageMalformed
    );
}
