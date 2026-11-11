// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! The static read cost bounds what the runtime meter actually spends.
//!
//! The retained public closure package is judged statically, then evaluated
//! by the generic runtime over graphs that drive it toward its declared
//! capacities. The oracle is the runtime meter, not the cost formula.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
use super::read_intent_costs;
use echo_edict_canonical::{
    decode_canonical_cbor_v1 as decode, digest_canonical_value_bytes_v1 as digest,
    encode_canonical_cbor_v1 as encode, CanonicalValueV1 as Value,
};
use warp_core::edict_pure::EvaluationLimits;
use warp_core::edict_read::{evaluate, GraphReadAperture, ReadLimits, ReadView};
use warp_core::{
    make_edge_id, make_node_id, make_type_id, AtomPayload, AttachmentValue, EdgeRecord, GraphStore,
    NodeKey, NodeRecord, TypeId, WorldlineFrontier, WorldlineId, WorldlineState,
};

const CLOSURE: &str = include_str!(
    "../../../../warp-core/tests/fixtures/edict-graph-read/closure-executable-operation-package.hex"
);

fn text(value: &str) -> Value {
    Value::Text(value.to_owned())
}
fn field<'a>(value: &'a Value, name: &str) -> &'a Value {
    let Value::Map(fields) = value else {
        panic!("map")
    };
    &fields
        .iter()
        .find(|(key, _)| key == &text(name))
        .expect(name)
        .1
}
fn embedded(program: &Value, name: &str) -> Value {
    let Value::Bytes(bytes) = field(program, name) else {
        panic!("artifact")
    };
    decode(bytes).expect("canonical artifact")
}
fn record<const N: usize>(fields: [(&str, Value); N]) -> Value {
    Value::Map(fields.into_iter().map(|(k, v)| (text(k), v)).collect())
}
fn word(value: u64) -> Value {
    Value::Integer(value.into())
}
fn id(bytes: [u8; 32]) -> Value {
    Value::Bytes(bytes.to_vec())
}
fn kind(name: &str) -> TypeId {
    make_type_id(name)
}

fn put(store: &mut GraphStore, name: &str, node_type: &str, atom: &[u8]) {
    store.insert_node(
        make_node_id(name),
        NodeRecord {
            ty: kind(node_type),
        },
    );
    store.set_node_attachment(
        make_node_id(name),
        Some(AttachmentValue::Atom(AtomPayload {
            type_id: kind("text"),
            bytes: atom.to_vec().into(),
        })),
    );
}
fn link(store: &mut GraphStore, from: &str, edge_type: &str, to: &str) {
    store.insert_edge(
        make_node_id(from),
        EdgeRecord {
            id: make_edge_id(&format!("{from}-{edge_type}-{to}")),
            from: make_node_id(from),
            to: make_node_id(to),
            ty: kind(edge_type),
        },
    );
}

/// Static cost of the retained closure program's only intent.
fn static_cost() -> (u64, u64) {
    let package = decode(&hex::decode(CLOSURE.trim()).unwrap()).unwrap();
    let Value::Bytes(program) = field(&package, "program") else {
        panic!("program")
    };
    let program = decode(program).unwrap();
    let configuration = embedded(&program, "target_configuration_artifact");
    let Value::Integer(scans) = field(field(&configuration, "graphReads"), "maxEdgeScans") else {
        panic!("scans")
    };
    let costs = read_intent_costs(
        &embedded(&program, "core_artifact"),
        &embedded(&program, "lawpack_exports_artifact"),
        u64::try_from(*scans).unwrap(),
    )
    .expect("the public closure program is judged");
    assert_eq!(costs.len(), 1);
    (costs[0].steps, costs[0].bytes)
}

/// Actual meter usage of one evaluation that must succeed.
fn spent(store: GraphStore, guide: Value, max_nodes: u64, max_depth: u64) -> (u64, u64) {
    let root = NodeKey {
        warp_id: store.warp_id(),
        local_id: make_node_id("root"),
    };
    let state = WorldlineState::from_root_store(store, root.local_id).unwrap();
    let frontier = WorldlineFrontier::new(WorldlineId::from_bytes([7; 32]), state);
    let aperture = [root];
    let mut edge_types = vec![kind("left"), kind("right")];
    edge_types.sort();
    let mut node_types = vec![kind("branch"), kind("leaf")];
    node_types.sort();
    let view = ReadView::at(&frontier, &aperture)
        .unwrap()
        .with_graph_aperture(GraphReadAperture {
            edge_types: &edge_types,
            node_types: &node_types,
            max_edge_scans: 64,
        })
        .unwrap();
    let bytes = hex::decode(CLOSURE.trim()).unwrap();
    let pin = digest("echo.operation-package/v1", &decode(&bytes).unwrap()).unwrap();
    let read = record([
        ("warpId", id(root.warp_id.0)),
        ("nodeId", id(root.local_id.0)),
        (
            "edgeTypes",
            Value::Array(vec![id(kind("left").0), id(kind("right").0)]),
        ),
        ("maxNodes", word(max_nodes)),
        ("maxDepth", word(max_depth)),
        ("guide", guide),
    ]);
    let input = encode(&record([("read", read)])).unwrap();
    let limits = ReadLimits {
        evaluation: EvaluationLimits {
            max_package_bytes: 1 << 20,
            max_input_bytes: 1 << 16,
            max_steps: 1 << 30,
            max_allocated_bytes: 1 << 30,
            max_output_bytes: 1 << 23,
        },
        max_reads: 64,
        max_read_bytes: 1 << 20,
    };
    let result = evaluate(&bytes, pin, &input, &view, limits).expect("evaluation succeeds");
    (result.steps, result.allocated_bytes)
}

fn guide(a: u64, b: u64, follow_a: bool, follow_b: bool, reach: u64) -> Value {
    record([
        ("a", word(a)),
        ("b", word(b)),
        ("followA", Value::Bool(follow_a)),
        ("followB", Value::Bool(follow_b)),
        ("reach", word(reach)),
    ])
}

fn assert_bounded(name: &str, spent: (u64, u64)) {
    let (steps, bytes) = static_cost();
    assert!(
        spent.0 <= steps && spent.1 <= bytes,
        "{name}: runtime spent {spent:?}, static cost ({steps}, {bytes})"
    );
}

#[test]
fn the_static_cost_bounds_the_boundary_path_closure() {
    let mut store = GraphStore::default();
    for (name, atom) in [("root", 4), ("lower", 2), ("upper", 6)] {
        put(&mut store, name, "branch", &vec![b'.'; atom]);
    }
    for (name, atom) in [("l1", "ab"), ("l2", "cd"), ("l3", "ef"), ("l4", "gh")] {
        put(&mut store, name, "leaf", atom.as_bytes());
    }
    for (from, left, right) in [
        ("root", "lower", "upper"),
        ("lower", "l1", "l2"),
        ("upper", "l3", "l4"),
    ] {
        link(&mut store, from, "left", left);
        link(&mut store, from, "right", right);
    }
    assert_bounded("boundary", spent(store, guide(1, 6, true, true, 2), 7, 2));
}

/// Every node row, eleven of twelve edge rows, ten of twelve occurrences,
/// maximal atoms and exactly the 64-edge scan allowance.
#[test]
fn the_static_cost_bounds_a_closure_near_every_declared_capacity() {
    let mut store = GraphStore::default();
    let atom = [b'x'; 64];
    put(&mut store, "root", "branch", &atom);
    for name in ["a", "b", "c", "d", "e", "g", "h"] {
        put(&mut store, name, "leaf", &atom);
    }
    for child in ["a", "b", "c"] {
        link(&mut store, "root", "left", child);
        for grandchild in ["d", "e"] {
            link(&mut store, child, "left", grandchild);
        }
    }
    // Declined right children are still materialized and offered.
    link(&mut store, "root", "right", "g");
    link(&mut store, "a", "right", "h");
    // Scans: root 2*(4+21), a 2*3, b and c 2*2 each = 64.
    for index in 0..21 {
        link(&mut store, "root", "decoy", &format!("unread-{index}"));
    }
    assert_bounded(
        "near capacity",
        spent(store, guide(0, 0, true, false, 10), 8, 4),
    );
}
