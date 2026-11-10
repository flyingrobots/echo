// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Typed outgoing-edge reads and guided closure reads over real frontier storage.
//!
//! These packages are SYNTHETIC. Each is the retained public single-read
//! package with its Core intent, Target IR step, result projection and
//! lowering configuration coherently replaced and repinned by
//! `tests/support/edict_graph_read.rs`. They
//! witness the runtime's graph-read semantics only. They are not public
//! compiler output and carry no provider or verifier approval.
//!
//! Oracle: every expected row below is written by hand from the fixture tree
//! in `tree`, never computed by the code under test.
#![cfg(feature = "trusted_runtime")]
#![allow(clippy::unwrap_used, clippy::panic)]

#[allow(dead_code, clippy::unwrap_used, clippy::panic)]
#[path = "../support/edict_graph_read.rs"]
mod support;

use echo_edict_canonical::{
    decode_canonical_cbor_v1 as decode, encode_canonical_cbor_v1 as encode,
    CanonicalValueV1 as Value,
};
use support::{
    boundary_guide, canonical, closure_build, closure_request, edge_build, edge_request, edge_row,
    graph_reads, guide, id, kind, limits, link, node, node_row, obstruction, occurrence, package,
    put, record, run, text, tree, Build, NODE, NODE_FOOTPRINT, SINGLE,
};
use warp_core::edict_pure::EvaluationError;
use warp_core::edict_read::{
    evaluate, GraphReadObstruction as Obstruction, ReadBasis, ReadError, ReadLimits, ReadResult,
    ReadView,
};
use warp_core::{
    make_edge_id, AtomPayload, AttachmentValue, GraphStore, NodeKey, NodeRecord, WorldlineFrontier,
    WorldlineId, WorldlineState,
};

// --- typed outgoing-edge reads ------------------------------------------------

#[test]
fn a_typed_edge_read_returns_only_the_matching_edge_of_a_present_source() {
    let (frontier, root) = tree(|_| {});
    let before = frontier.state().state_root();
    let result = run(
        &frontier,
        root,
        &edge_build(2),
        edge_request(root, "left"),
        3,
        limits(),
    )
    .unwrap();
    assert_eq!(
        result.output,
        canonical(&Value::Array(vec![record([
            ("edgeId", id(make_edge_id("root-left-lower").0)),
            ("toId", id(node("lower").0)),
        ])]))
    );
    // The bucket holds left, right and decoy: all three were inspected.
    assert_eq!((result.reads, result.edge_scans), (1, 3));
    assert_eq!(result.basis, ReadBasis::at(&frontier));
    assert_eq!(frontier.state().state_root(), before);
}

#[test]
fn an_absent_typed_edge_is_an_empty_result_and_an_absent_source_is_a_failure() {
    let (frontier, root) = tree(|_| {});
    let leaf = NodeKey {
        local_id: node("l1"),
        ..root
    };
    let empty = run(
        &frontier,
        leaf,
        &edge_build(2),
        edge_request(leaf, "left"),
        3,
        limits(),
    )
    .unwrap();
    assert_eq!(empty.output, canonical(&Value::Array(vec![])));
    assert_eq!((empty.reads, empty.edge_scans), (1, 0));

    let absent = NodeKey {
        local_id: node("never-stored"),
        ..root
    };
    assert_eq!(
        obstruction(run(
            &frontier,
            absent,
            &edge_build(2),
            edge_request(absent, "left"),
            3,
            limits(),
        )),
        Obstruction::EdgeSourceMissing
    );
}

#[test]
fn an_edge_read_scan_bound_counts_excluded_edges_and_rejects_one_unit_short() {
    let (frontier, root) = tree(|_| {});
    // One edge matches; the bucket's three edges are what the bound counts.
    assert!(run(
        &frontier,
        root,
        &edge_build(2),
        edge_request(root, "left"),
        3,
        limits()
    )
    .is_ok());
    assert_eq!(
        obstruction(run(
            &frontier,
            root,
            &edge_build(2),
            edge_request(root, "left"),
            2,
            limits(),
        )),
        Obstruction::EdgeScanBoundExceeded
    );
    // The package's own declared ceiling rejects at the same boundary.
    for (declared, accepted) in [(3, true), (2, false)] {
        let build = Build {
            graph_reads: Some(graph_reads(declared, 0, 0, false)),
            ..edge_build(2)
        };
        let result = run(
            &frontier,
            root,
            &build,
            edge_request(root, "left"),
            64,
            limits(),
        );
        assert_eq!(result.is_ok(), accepted, "declared scan ceiling {declared}");
        if !accepted {
            assert_eq!(obstruction(result), Obstruction::EdgeScanBoundExceeded);
        }
    }
}

#[test]
fn more_matching_edges_than_the_declared_list_holds_is_its_own_failure() {
    let (frontier, root) = tree(|store| link(store, "root", "left", "l2"));
    let request = || edge_request(root, "left");
    let both = run(&frontier, root, &edge_build(2), request(), 4, limits()).unwrap();
    let Value::Array(rows) = decode(&both.output).unwrap() else {
        panic!("edge list")
    };
    assert_eq!(rows.len(), 2);
    assert_eq!(
        obstruction(run(&frontier, root, &edge_build(1), request(), 4, limits())),
        Obstruction::EdgeBoundExceeded
    );
}

#[test]
fn graph_reads_stay_inside_the_host_selected_apertures() {
    let (frontier, root) = tree(|_| {});
    let lower = NodeKey {
        local_id: node("lower"),
        ..root
    };
    // The decoy type is stored but was never granted.
    assert_eq!(
        run(
            &frontier,
            root,
            &edge_build(2),
            edge_request(root, "decoy"),
            8,
            limits()
        ),
        Err(ReadError::OutsideGraphAperture(kind("decoy")))
    );
    // A source the host did not select is refused before any lookup.
    assert_eq!(
        run(
            &frontier,
            root,
            &edge_build(2),
            edge_request(lower, "left"),
            8,
            limits()
        ),
        Err(ReadError::OutsideAperture(lower))
    );
    // Without a graph aperture a node-only view grants no graph read.
    let aperture = [root];
    let view = ReadView::at(&frontier, &aperture).unwrap();
    let (package, pin) = package(&edge_build(2));
    let input = encode(&record([("read", edge_request(root, "left"))])).unwrap();
    assert_eq!(
        evaluate(&package, pin, &input, &view, limits()),
        Err(ReadError::OutsideGraphAperture(kind("left")))
    );
    // A discovered node of an ungranted type is refused, not delivered.
    let (typed, typed_root) = tree(|store| {
        store.insert_node(node("l1"), NodeRecord { ty: kind("secret") });
    });
    assert_eq!(
        run(
            &typed,
            typed_root,
            &closure_build(7, 6, 5),
            closure_request(typed_root, boundary_guide(), 7, 2),
            64,
            limits()
        ),
        Err(ReadError::OutsideGraphAperture(kind("secret")))
    );
}

// --- guided closure reads -----------------------------------------------------

#[test]
fn a_guided_closure_reads_both_boundary_paths_and_their_sibling_material() {
    let (frontier, root) = tree(|_| {});
    let before = frontier.state().state_root();
    let result = run(
        &frontier,
        root,
        &closure_build(7, 6, 5),
        closure_request(root, boundary_guide(), 7, 2),
        14,
        limits(),
    )
    .unwrap();
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
    assert_eq!(
        decode(&result.output).unwrap(),
        decode(&canonical(&expected)).unwrap()
    );
    // Seven materializations and two typed ranges for each of five occurrences.
    assert_eq!(result.reads, 17);
    // root 3 edges, lower 2, upper 2, the two leaves 0; each scanned per type.
    assert_eq!(result.edge_scans, 14);
    assert_eq!(result.read_bytes, 4 + 2 + 6 + 2 + 2 + 2 + 2);
    assert_eq!(frontier.state().state_root(), before);
    assert_eq!(
        result,
        run(
            &frontier,
            root,
            &closure_build(7, 6, 5),
            closure_request(root, boundary_guide(), 7, 2),
            14,
            limits(),
        )
        .unwrap()
    );
}

#[test]
fn a_leaf_shared_by_both_children_is_one_node_and_two_occurrences() {
    // One branch whose left and right edges both lead to the same leaf.
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
    let result = run(
        &frontier,
        root,
        &closure_build(2, 2, 3),
        closure_request(root, guide(1, 6, true, true, 1), 2, 1),
        4,
        limits(),
    )
    .unwrap();
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
                // Same node, two positions: distinct edge paths and guides.
                occurrence(1, 0, 1, guide(1, 6, true, false, 1)),
                occurrence(1, 1, 1, guide(1, 6, false, true, 1)),
            ]),
        ),
    ]);
    assert_eq!(
        decode(&result.output).unwrap(),
        decode(&canonical(&expected)).unwrap()
    );
    // The shared leaf's four bytes were delivered once.
    assert_eq!(result.read_bytes, 4 + 4);
}

#[test]
fn each_closure_bound_accepts_its_exact_value_and_rejects_one_unit_short() {
    let (frontier, root) = tree(|_| {});
    let attempt = |build: Build, max_nodes, max_depth, scans, limits| {
        run(
            &frontier,
            root,
            &build,
            closure_request(root, boundary_guide(), max_nodes, max_depth),
            scans,
            limits,
        )
    };
    let exact = attempt(closure_build(7, 6, 5), 7, 2, 14, limits()).unwrap();
    let cases: [(&str, Result<ReadResult, ReadError>, Obstruction); 7] = [
        (
            "requested nodes",
            attempt(closure_build(7, 6, 5), 6, 2, 14, limits()),
            Obstruction::ClosureNodeBoundExceeded,
        ),
        (
            "declared node capacity",
            attempt(closure_build(6, 6, 5), 7, 2, 14, limits()),
            Obstruction::ClosureNodeBoundExceeded,
        ),
        (
            "requested depth",
            attempt(closure_build(7, 6, 5), 7, 1, 14, limits()),
            Obstruction::ClosureDepthBoundExceeded,
        ),
        (
            "host scan allowance",
            attempt(closure_build(7, 6, 5), 7, 2, 13, limits()),
            Obstruction::ClosureScanBoundExceeded,
        ),
        (
            "declared edge capacity",
            attempt(closure_build(7, 5, 5), 7, 2, 14, limits()),
            Obstruction::ClosureWorkBoundExceeded,
        ),
        (
            "declared occurrence capacity",
            attempt(closure_build(7, 6, 4), 7, 2, 14, limits()),
            Obstruction::ClosureWorkBoundExceeded,
        ),
        (
            "declared closure nodes",
            attempt(
                Build {
                    graph_reads: Some(graph_reads(64, 6, 8, true)),
                    ..closure_build(7, 6, 5)
                },
                7,
                2,
                14,
                limits(),
            ),
            Obstruction::ClosureNodeBoundExceeded,
        ),
    ];
    for (name, result, expected) in cases {
        assert_eq!(obstruction(result), expected, "{name}");
    }
    for (name, graph, expected) in [
        (
            "declared closure depth",
            graph_reads(64, 64, 1, true),
            Obstruction::ClosureDepthBoundExceeded,
        ),
        (
            "declared scan ceiling",
            graph_reads(13, 64, 8, true),
            Obstruction::ClosureScanBoundExceeded,
        ),
    ] {
        let build = Build {
            graph_reads: Some(graph),
            ..closure_build(7, 6, 5)
        };
        assert_eq!(
            obstruction(attempt(build, 7, 2, 64, limits())),
            expected,
            "{name}"
        );
    }
    // Host ceilings are generic evaluation refusals, never authored outcomes.
    let short = |change: fn(&mut ReadLimits, &ReadResult)| {
        let mut limits = limits();
        change(&mut limits, &exact);
        attempt(closure_build(7, 6, 5), 7, 2, 14, limits)
    };
    let fits = |change: fn(&mut ReadLimits, &ReadResult)| short(change).is_ok();
    assert!(fits(|limits, exact| limits.max_reads = exact.reads));
    assert_eq!(
        short(|limits, exact| limits.max_reads = exact.reads - 1),
        Err(ReadError::ReadCountExceeded)
    );
    assert!(fits(
        |limits, exact| limits.max_read_bytes = exact.read_bytes
    ));
    assert_eq!(
        short(|limits, exact| limits.max_read_bytes = exact.read_bytes - 1),
        Err(ReadError::ReadBytesExceeded)
    );
    assert!(fits(
        |limits, exact| limits.evaluation.max_steps = exact.steps
    ));
    assert_eq!(
        short(|limits, exact| limits.evaluation.max_steps = exact.steps - 1),
        Err(ReadError::Evaluation(EvaluationError::StepBudgetExceeded))
    );
    assert!(fits(|limits, exact| {
        limits.evaluation.max_allocated_bytes = exact.allocated_bytes;
    }));
    assert_eq!(
        short(|limits, exact| {
            limits.evaluation.max_allocated_bytes = exact.allocated_bytes - 1;
        }),
        Err(ReadError::Evaluation(
            EvaluationError::AllocationBudgetExceeded
        ))
    );
}

#[test]
fn an_empty_selection_still_pays_for_what_it_inspected() {
    let (frontier, root) = tree(|_| {});
    let nothing = || closure_request(root, guide(1, 6, false, false, 2), 3, 0);
    let result = run(
        &frontier,
        root,
        &closure_build(3, 2, 1),
        nothing(),
        6,
        limits(),
    )
    .unwrap();
    let expected = record([
        (
            "nodes",
            Value::Array(vec![
                node_row("root", "branch", b"...."),
                node_row("lower", "branch", b".."),
                node_row("upper", "branch", b"......"),
            ]),
        ),
        (
            "edges",
            Value::Array(vec![
                edge_row(0, 0, "root-left-lower", 1),
                edge_row(0, 1, "root-right-upper", 2),
            ]),
        ),
        (
            "occurrences",
            Value::Array(vec![occurrence(0, 0, 0, guide(1, 6, false, false, 2))]),
        ),
    ]);
    assert_eq!(
        decode(&result.output).unwrap(),
        decode(&canonical(&expected)).unwrap()
    );
    assert_eq!((result.reads, result.edge_scans), (5, 6));
    // Nothing was selected, yet the scan and the two offered children count.
    assert_eq!(
        obstruction(run(
            &frontier,
            root,
            &closure_build(3, 2, 1),
            nothing(),
            5,
            limits()
        )),
        Obstruction::ClosureScanBoundExceeded
    );
    let two_nodes = closure_request(root, guide(1, 6, false, false, 2), 2, 0);
    assert_eq!(
        obstruction(run(
            &frontier,
            root,
            &closure_build(3, 2, 1),
            two_nodes,
            6,
            limits()
        )),
        Obstruction::ClosureNodeBoundExceeded
    );
}

#[test]
fn limit_coverage_and_storage_failures_are_three_distinct_outcomes() {
    let (frontier, root) = tree(|_| {});
    let request = |guide, max_depth| closure_request(root, guide, 7, max_depth);
    let closure = |frontier: &WorldlineFrontier, start, read| {
        run(frontier, start, &closure_build(7, 6, 5), read, 64, limits())
    };
    // The declared envelope is too small for a path the selector may follow.
    assert_eq!(
        obstruction(closure(&frontier, root, request(boundary_guide(), 1))),
        Obstruction::ClosureDepthBoundExceeded
    );
    // The envelope is large enough; the selector's own reach is too short.
    assert_eq!(
        obstruction(closure(
            &frontier,
            root,
            request(guide(1, 6, true, true, 1), 2)
        )),
        Obstruction::ClosureSelectorInsufficient
    );
    // The selector judges observed material against its declared invariant.
    let (empty_atom, start) = tree(|store| put(store, "l1", "leaf", b""));
    assert_eq!(
        obstruction(closure(&empty_atom, start, request(boundary_guide(), 2))),
        Obstruction::ClosureStorageMalformed
    );
    // An edge to a node that is not stored violates referential integrity.
    let (dangling, start) = tree(|store| link(store, "lower", "left", "never-stored"));
    assert_eq!(
        obstruction(closure(&dangling, start, request(boundary_guide(), 2))),
        Obstruction::ClosureStorageMalformed
    );
    // A stored atom larger than the declared node bound is not truncated.
    let (oversized, start) = tree(|store| put(store, "l2", "leaf", &[b'x'; 65]));
    assert_eq!(
        obstruction(closure(&oversized, start, request(boundary_guide(), 2))),
        Obstruction::ClosureStorageMalformed
    );
    // An absent start node is ordinary absence, not malformed storage.
    let absent = NodeKey {
        local_id: node("never-stored"),
        ..root
    };
    assert_eq!(
        obstruction(closure(
            &frontier,
            absent,
            closure_request(absent, boundary_guide(), 7, 2)
        )),
        Obstruction::ClosureStartMissing
    );
}

#[test]
fn a_view_bound_to_a_stale_basis_refuses_before_any_graph_read() {
    let (frontier, root) = tree(|_| {});
    let (changed, _) = tree(|store| put(store, "l4", "leaf", b"zz"));
    let aperture = [root];
    assert_eq!(
        ReadView::new(&changed, ReadBasis::at(&frontier), &aperture).map(|_| ()),
        Err(ReadError::BasisMismatch)
    );
}

// --- declared capability and refusal -----------------------------------------

#[test]
fn a_package_that_does_not_declare_graph_reads_cannot_use_one() {
    let (frontier, root) = tree(|_| {});
    let refused = |build: Build, read| run(&frontier, root, &build, read, 64, limits());
    let unsupported = Err(ReadError::Evaluation(EvaluationError::UnsupportedProgram));
    for (name, build) in [
        (
            "no graphReads declaration",
            Build {
                graph_reads: None,
                footprint: NODE_FOOTPRINT,
                ..edge_build(2)
            },
        ),
        (
            "graph reads under the explicit node aperture contract",
            Build {
                footprint: NODE_FOOTPRINT,
                ..edge_build(2)
            },
        ),
        (
            "typed graph contract without a graphReads declaration",
            Build {
                graph_reads: None,
                ..edge_build(2)
            },
        ),
    ] {
        assert_eq!(
            refused(build, edge_request(root, "left")),
            unsupported,
            "{name}"
        );
    }
    let closure = || closure_request(root, boundary_guide(), 7, 2);
    for (name, build) in [
        (
            "closure read without a declared selector",
            Build {
                graph_reads: Some(graph_reads(64, 64, 8, false)),
                ..closure_build(7, 6, 5)
            },
        ),
        (
            "declared selector that the module does not define",
            Build {
                selector: None,
                ..closure_build(7, 6, 5)
            },
        ),
    ] {
        assert_eq!(refused(build, closure()), unsupported, "{name}");
    }
    // A well-typed helper over another guide type is not this closure's
    // selector: the runtime binds by the exact signature, never by name alone.
    let candidate = record([
        ("id", text("param.0")),
        (
            "type",
            text(&format!(
                "Record<child:{NODE},depth:U64,guide:U64,parent:{NODE},slot:U64>"
            )),
        ),
        ("alphaName", text("candidate")),
    ]);
    let constant = |value: bool| {
        record([
            ("kind", text("const")),
            (
                "value",
                record([("kind", text("bool")), ("value", Value::Bool(value))]),
            ),
        ])
    };
    let foreign = record([
        ("params", Value::Array(vec![candidate.clone()])),
        (
            "returnType",
            text("Record<expand:Bool,guide:U64,sufficient:Bool,wellFormed:Bool>"),
        ),
        (
            "body",
            record([
                ("locals", Value::Array(vec![])),
                ("bindings", Value::Array(vec![])),
                (
                    "result",
                    record([
                        ("kind", text("record")),
                        (
                            "fields",
                            record([
                                ("expand", constant(false)),
                                (
                                    "guide",
                                    record([
                                        ("kind", text("field")),
                                        (
                                            "base",
                                            record([("kind", text("local")), ("ref", candidate)]),
                                        ),
                                        ("field", text("guide")),
                                    ]),
                                ),
                                ("sufficient", constant(true)),
                                ("wellFormed", constant(true)),
                            ]),
                        ),
                    ]),
                ),
            ]),
        ),
    ]);
    assert_eq!(
        refused(
            Build {
                selector: Some(foreign),
                ..closure_build(7, 6, 5)
            },
            closure(),
        ),
        unsupported,
        "selector over another guide type"
    );
}

#[test]
fn the_retained_public_node_read_package_is_unchanged_by_graph_reads() {
    // Control: the unmodified public package still evaluates under its
    // explicit node aperture contract and reports no graph work.
    let mut store = GraphStore::default();
    store.insert_node(node("first"), NodeRecord { ty: kind("node") });
    store.set_node_attachment(
        node("first"),
        Some(AttachmentValue::Atom(AtomPayload {
            type_id: kind("document"),
            bytes: b"alpha".to_vec().into(),
        })),
    );
    let key = NodeKey {
        warp_id: store.warp_id(),
        local_id: node("first"),
    };
    let frontier = WorldlineFrontier::new(
        WorldlineId::from_bytes([7; 32]),
        WorldlineState::from_root_store(store, key.local_id).unwrap(),
    );
    let aperture = [key];
    let view = ReadView::at(&frontier, &aperture).unwrap();
    let package = hex::decode(SINGLE.trim()).unwrap();
    let pin: [u8; 32] =
        hex::decode("a2ae35d39ae74a26da916544464529bc6c7af32a76b6e6f354ef79c5063ab109")
            .unwrap()
            .try_into()
            .unwrap();
    let input = encode(&record([
        (
            "address",
            record([
                ("warpId", id(key.warp_id.0)),
                ("nodeId", id(key.local_id.0)),
                ("typeId", id(kind("document").0)),
            ]),
        ),
        ("expected", Value::Bytes(b"alpha".to_vec())),
    ]))
    .unwrap();
    let result = evaluate(&package, pin, &input, &view, limits()).unwrap();
    assert_eq!(
        decode(&result.output).unwrap(),
        Value::Bytes(b"alpha".to_vec())
    );
    assert_eq!(
        (result.reads, result.read_bytes, result.edge_scans),
        (1, 5, 0)
    );
}
