// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! SYNTHETIC graph-read packages and the stored tree they are evaluated over.
//!
//! Each package is the retained public single-read package with its Core
//! intent, Target IR step, result projection and lowering configuration
//! coherently replaced and repinned here. None is public compiler output and
//! none carries provider or verifier approval.
use echo_edict_canonical::{
    decode_canonical_cbor_v1 as decode, digest_canonical_value_bytes_v1 as digest,
    encode_canonical_cbor_v1 as encode, CanonicalValueV1 as Value,
};
use warp_core::edict_pure::EvaluationLimits;
use warp_core::edict_read::{
    evaluate, GraphReadAperture, GraphReadObstruction as Obstruction, ReadError, ReadLimits,
    ReadResult, ReadView,
};
use warp_core::{
    make_edge_id, make_node_id, make_type_id, AtomPayload, AttachmentValue, EdgeRecord, GraphStore,
    NodeId, NodeKey, NodeRecord, TypeId, WorldlineFrontier, WorldlineId, WorldlineState,
};

pub const SINGLE: &str =
    include_str!("../fixtures/edict-node-read/single-executable-operation-package.hex");
pub const NODE: &str = "Record<atomBytes:Bytes<max=64>,atomType:Bytes<max=32>,\
nodeId:Bytes<exact=32>,nodeType:Bytes<exact=32>>";
pub const GUIDE: &str = "Record<a:U64,b:U64,followA:Bool,followB:Bool,reach:U64>";
pub const GRAPH_FOOTPRINT: &str = "echo.operation.footprint.typed-graph-read-aperture/v1";
pub const NODE_FOOTPRINT: &str = "echo.operation.footprint.explicit-node-read-aperture/v1";
pub const EDGE_READ: &str = "echo.dpo@1.outgoing-edge-read";
pub const CLOSURE_READ: &str = "echo.dpo@1.closure-read";

pub fn text(value: &str) -> Value {
    Value::Text(value.into())
}
pub fn record<const N: usize>(fields: [(&str, Value); N]) -> Value {
    Value::Map(fields.into_iter().map(|(k, v)| (text(k), v)).collect())
}
pub fn word(value: u64) -> Value {
    Value::Integer(value.into())
}
pub fn id(bytes: [u8; 32]) -> Value {
    Value::Bytes(bytes.to_vec())
}
pub fn field<'a>(value: &'a Value, name: &str) -> &'a Value {
    let Value::Map(fields) = value else {
        panic!("map")
    };
    &fields.iter().find(|(key, _)| key == &text(name)).unwrap().1
}
pub fn field_mut<'a>(value: &'a mut Value, name: &str) -> &'a mut Value {
    let Value::Map(fields) = value else {
        panic!("map")
    };
    &mut fields
        .iter_mut()
        .find(|(key, _)| key == &text(name))
        .unwrap()
        .1
}
pub fn set(value: &mut Value, name: &str, replacement: Value) {
    let Value::Map(fields) = value else {
        panic!("map")
    };
    fields.retain(|(key, _)| key != &text(name));
    fields.push((text(name), replacement));
}
pub fn embedded(value: &Value, name: &str) -> Value {
    let Value::Bytes(bytes) = field(value, name) else {
        panic!("embedded artifact")
    };
    decode(bytes).unwrap()
}

// --- authored Core fragments, in the shapes the compiler emits ---------------

pub fn local(name: &str, ty: &str) -> Value {
    record([
        ("id", text(&format!("local.{name}"))),
        ("type", text(ty)),
        ("alphaName", text(name)),
    ])
}
pub fn reference(declaration: &Value) -> Value {
    record([("kind", text("local")), ("ref", declaration.clone())])
}
pub fn select(base: Value, name: &str) -> Value {
    record([
        ("kind", text("field")),
        ("base", base),
        ("field", text(name)),
    ])
}
pub fn boolean(value: bool) -> Value {
    record([
        ("kind", text("const")),
        (
            "value",
            record([("kind", text("bool")), ("value", Value::Bool(value))]),
        ),
    ])
}
pub fn literal(value: u64) -> Value {
    record([
        ("kind", text("const")),
        (
            "value",
            record([
                ("kind", text("int")),
                ("width", text("U64")),
                ("value", word(value)),
            ]),
        ),
    ])
}
pub fn compare(op: &str, left: Value, right: Value) -> Value {
    record([
        ("kind", text("compare")),
        ("op", text(op)),
        ("left", left),
        ("right", right),
    ])
}
pub fn all(values: Vec<Value>) -> Value {
    record([("kind", text("all")), ("values", Value::Array(values))])
}
pub fn any(values: Vec<Value>) -> Value {
    record([("kind", text("any")), ("values", Value::Array(values))])
}
pub fn truth(predicate: Value, when_true: bool) -> Value {
    record([
        ("kind", text("if")),
        ("predicate", predicate),
        ("then", boolean(when_true)),
        ("else", boolean(!when_true)),
    ])
}
pub fn length(bytes: Value) -> Value {
    record([
        ("kind", text("call")),
        ("callee", text("core.bytes.length")),
        ("args", Value::Array(vec![bytes])),
        ("typeArgs", Value::Array(vec![text("Bytes<max=64>")])),
    ])
}
pub fn binding(declaration: &Value, value: Value) -> Value {
    record([
        ("kind", text("let")),
        ("binding", declaration.clone()),
        ("value", value),
    ])
}

/// The authored selector. A branch atom's length is the absolute offset where
/// its right child begins. Offset `a` (and `b`) is followed left while it is
/// below that split and right otherwise; a child is expanded while it lies on
/// a followed path. The selector declares a child malformed when its atom is
/// empty, and itself insufficient when a followed path continues below the
/// `reach` its guide was given.
pub fn selector() -> Value {
    let candidate_type =
        format!("Record<child:{NODE},depth:U64,guide:{GUIDE},parent:{NODE},slot:U64>");
    let decision = format!("Record<expand:Bool,guide:{GUIDE},sufficient:Bool,wellFormed:Bool>");
    let candidate = record([
        ("id", text("param.0")),
        ("type", text(&candidate_type)),
        ("alphaName", text("candidate")),
    ]);
    let (split, follow_a, follow_b) = (
        local("split", "U64"),
        local("followA", "Bool"),
        local("followB", "Bool"),
    );
    let input = || reference(&candidate);
    let guide = |name: &str| select(select(input(), "guide"), name);
    let follows = |flag: &str, offset: &str| {
        truth(
            all(vec![
                compare("==", guide(flag), boolean(true)),
                any(vec![
                    all(vec![
                        compare("==", select(input(), "slot"), literal(0)),
                        compare("<", guide(offset), reference(&split)),
                    ]),
                    all(vec![
                        compare("==", select(input(), "slot"), literal(1)),
                        compare(">=", guide(offset), reference(&split)),
                    ]),
                ]),
            ]),
            true,
        )
    };
    let wanted = || {
        any(vec![
            compare("==", reference(&follow_a), boolean(true)),
            compare("==", reference(&follow_b), boolean(true)),
        ])
    };
    let result = record([
        ("kind", text("record")),
        (
            "fields",
            record([
                ("expand", truth(wanted(), true)),
                (
                    "guide",
                    record([
                        ("kind", text("record")),
                        (
                            "fields",
                            record([
                                ("a", guide("a")),
                                ("b", guide("b")),
                                ("followA", reference(&follow_a)),
                                ("followB", reference(&follow_b)),
                                ("reach", guide("reach")),
                            ]),
                        ),
                    ]),
                ),
                (
                    "sufficient",
                    truth(
                        all(vec![
                            wanted(),
                            compare(">", select(input(), "depth"), guide("reach")),
                        ]),
                        false,
                    ),
                ),
                (
                    "wellFormed",
                    truth(
                        compare(
                            ">",
                            length(select(select(input(), "child"), "atomBytes")),
                            literal(0),
                        ),
                        true,
                    ),
                ),
            ]),
        ),
    ]);
    record([
        ("params", Value::Array(vec![candidate.clone()])),
        ("returnType", text(&decision)),
        (
            "body",
            record([
                (
                    "locals",
                    Value::Array(vec![split.clone(), follow_a.clone(), follow_b.clone()]),
                ),
                (
                    "bindings",
                    Value::Array(vec![
                        binding(
                            &split,
                            length(select(select(input(), "parent"), "atomBytes")),
                        ),
                        binding(&follow_a, follows("followA", "a")),
                        binding(&follow_b, follows("followB", "b")),
                    ]),
                ),
                ("result", result),
            ]),
        ),
    ])
}

// --- synthetic package --------------------------------------------------------

/// One coherently repinned synthetic read program.
#[derive(Clone)]
pub struct Build {
    pub intrinsic: &'static str,
    /// Declared result capacities: edges for an edge read; nodes, edges and
    /// occurrences for a closure read.
    pub capacity: [u64; 3],
    pub graph_reads: Option<Value>,
    pub footprint: &'static str,
    pub selector: Option<Value>,
}

pub fn graph_reads(scans: u64, nodes: u64, depth: u64, selector: bool) -> Value {
    let mut value = record([
        ("maxEdgeScans", word(scans)),
        ("maxClosureNodes", word(nodes)),
        ("maxClosureDepth", word(depth)),
    ]);
    if selector {
        set(&mut value, "closureSelector", text("select"));
    }
    value
}

pub fn edge_build(max: u64) -> Build {
    Build {
        intrinsic: EDGE_READ,
        capacity: [max, 0, 0],
        graph_reads: Some(graph_reads(64, 0, 0, false)),
        footprint: GRAPH_FOOTPRINT,
        selector: None,
    }
}

pub fn closure_build(nodes: u64, edges: u64, occurrences: u64) -> Build {
    Build {
        intrinsic: CLOSURE_READ,
        capacity: [nodes, edges, occurrences],
        graph_reads: Some(graph_reads(64, 64, 8, true)),
        footprint: GRAPH_FOOTPRINT,
        selector: Some(selector()),
    }
}

pub fn failures(intrinsic: &str) -> Vec<Obstruction> {
    if intrinsic == EDGE_READ {
        vec![
            Obstruction::EdgeSourceMissing,
            Obstruction::EdgeBoundExceeded,
            Obstruction::EdgeScanBoundExceeded,
        ]
    } else {
        vec![
            Obstruction::ClosureStartMissing,
            Obstruction::ClosureNodeBoundExceeded,
            Obstruction::ClosureDepthBoundExceeded,
            Obstruction::ClosureScanBoundExceeded,
            Obstruction::ClosureWorkBoundExceeded,
            Obstruction::ClosureSelectorInsufficient,
            Obstruction::ClosureStorageMalformed,
        ]
    }
}

/// Each failure gets its own authored arm, named after the failure.
pub fn arm(failure: Obstruction) -> String {
    format!("text.{failure:?}")
}

pub fn types(build: &Build) -> (String, String) {
    let [first, second, third] = build.capacity;
    if build.intrinsic == EDGE_READ {
        (
            "Record<edgeType:Bytes<exact=32>,nodeId:Bytes<exact=32>,warpId:Bytes<exact=32>>".into(),
            format!("List<Record<edgeId:Bytes<exact=32>,toId:Bytes<exact=32>>,max={first}>"),
        )
    } else {
        (
            format!(
                "Record<edgeTypes:List<Bytes<exact=32>,max=2>,guide:{GUIDE},maxDepth:U64,\
maxNodes:U64,nodeId:Bytes<exact=32>,warpId:Bytes<exact=32>>"
            ),
            format!(
                "Record<edges:List<Record<edgeId:Bytes<exact=32>,from:U64,node:U64,slot:U64>,\
max={second}>,nodes:List<{NODE},max={first}>,occurrences:List<Record<depth:U64,edge:U64,\
guide:{GUIDE},node:U64>,max={third}>>"
            ),
        )
    }
}

pub fn package(build: &Build) -> (Vec<u8>, [u8; 32]) {
    let mut package = decode(&hex::decode(SINGLE.trim()).unwrap()).unwrap();
    let mut program = embedded(&package, "program");
    let mut core = embedded(&program, "core_artifact");
    let mut target = embedded(&program, "target_ir_artifact");
    let mut projection = embedded(&program, "result_projection_artifact");
    let mut configuration = embedded(&program, "target_configuration_artifact");

    let (read_type, result_type) = types(build);
    let input_type = format!("Record<read:{read_type}>");
    let argument = record([
        ("id", text("arg.0")),
        ("type", text(&input_type)),
        ("alphaName", text("$arg0")),
    ]);
    let observed = local("observed", &result_type);
    let read = select(reference(&argument), "read");
    let basis = select(read.clone(), "nodeId");
    let result = reference(&observed);

    set(
        &mut core,
        "requiredCoreCapabilities",
        Value::Array(vec![text("core.list/v1")]),
    );
    if let Some(selector) = &build.selector {
        set(
            &mut core,
            "functions",
            record([("select", selector.clone())]),
        );
    }
    let intent = field_mut(field_mut(&mut core, "intents"), "replaceRange");
    set(intent, "input", text(&input_type));
    set(intent, "output", text(&result_type));
    set(intent, "basis", basis.clone());
    let body = field_mut(intent, "body");
    set(
        body,
        "locals",
        Value::Array(vec![argument, observed.clone()]),
    );
    set(body, "result", result.clone());

    let declared = failures(build.intrinsic);
    let step = record([
        ("id", text("step.0")),
        ("binding", observed),
        ("effect", text("text.readGraph")),
        ("input", read),
        ("targetIntrinsic", text(build.intrinsic)),
        (
            "obstructionArms",
            Value::Map(
                declared
                    .iter()
                    .map(|failure| {
                        (
                            text(failure.runtime_tag()),
                            record([
                                ("binder", local("failed", &read_type)),
                                (
                                    "value",
                                    record([
                                        ("kind", text("call")),
                                        ("callee", text(&arm(*failure))),
                                        ("args", Value::Array(vec![])),
                                        ("typeArgs", Value::Array(vec![])),
                                    ]),
                                ),
                            ]),
                        )
                    })
                    .collect(),
            ),
        ),
        (
            "obstructionFailures",
            Value::Array(
                declared
                    .iter()
                    .map(|failure| text(failure.runtime_tag()))
                    .collect(),
            ),
        ),
    ]);
    let core_identity = digest("edict.core.module/v1", &core).unwrap();
    let lowered = field_mut(field_mut(&mut target, "intents"), "replaceRange");
    set(lowered, "steps", Value::Array(vec![step]));
    set(lowered, "requirements", Value::Array(vec![]));
    set(
        lowered,
        "executionOrder",
        Value::Array(vec![text("step.0")]),
    );
    set(lowered, "basis", basis);
    set(lowered, "result", result);
    set(
        field_mut(field_mut(&mut target, "semanticClosure"), "sourceCore"),
        "digest",
        Value::Array(vec![text("sha256"), Value::Bytes(core_identity.to_vec())]),
    );
    let target_identity = digest("edict.target-ir.artifact/v1", &target).unwrap();

    set(
        &mut projection,
        "expression",
        record([
            ("kind", text("source")),
            ("path", Value::Array(vec![])),
            (
                "source",
                record([
                    ("kind", text("capabilityResult")),
                    ("stepId", text("step.0")),
                ]),
            ),
        ]),
    );
    set(&mut projection, "outputType", text(&result_type));
    if let Some(graph_reads) = &build.graph_reads {
        set(&mut configuration, "graphReads", graph_reads.clone());
    }

    for (name, artifact) in [
        ("core_artifact", &core),
        ("target_ir_artifact", &target),
        ("result_projection_artifact", &projection),
        ("target_configuration_artifact", &configuration),
    ] {
        set(&mut program, name, Value::Bytes(encode(artifact).unwrap()));
    }
    set(
        &mut package,
        "program",
        Value::Bytes(encode(&program).unwrap()),
    );
    let closure = field_mut(&mut package, "semantic_closure");
    for key in ["core_identity", "canonical_meaning_identity"] {
        set(closure, key, Value::Bytes(core_identity.to_vec()));
    }
    set(
        closure,
        "target_ir_identity",
        Value::Bytes(target_identity.to_vec()),
    );
    let mut profile = blake3::Hasher::new();
    profile.update(b"echo:operation-profile:v1\0");
    profile.update(&(build.footprint.len() as u64).to_le_bytes());
    profile.update(build.footprint.as_bytes());
    set(
        &mut package,
        "footprint_contract_identity",
        Value::Bytes(profile.finalize().as_bytes().to_vec()),
    );
    let pin = digest("echo.operation-package/v1", &package).unwrap();
    (encode(&package).unwrap(), pin)
}

// --- stored graph -------------------------------------------------------------

pub fn node(name: &str) -> NodeId {
    make_node_id(name)
}
pub fn kind(name: &str) -> TypeId {
    make_type_id(name)
}
pub fn put(store: &mut GraphStore, name: &str, node_type: &str, atom: &[u8]) {
    store.insert_node(
        node(name),
        NodeRecord {
            ty: kind(node_type),
        },
    );
    store.set_node_attachment(
        node(name),
        Some(AttachmentValue::Atom(AtomPayload {
            type_id: kind("text"),
            bytes: atom.to_vec().into(),
        })),
    );
}
pub fn link(store: &mut GraphStore, from: &str, edge_type: &str, to: &str) {
    store.insert_edge(
        node(from),
        EdgeRecord {
            id: make_edge_id(&format!("{from}-{edge_type}-{to}")),
            from: node(from),
            to: node(to),
            ty: kind(edge_type),
        },
    );
}

/// ```text
///                 root (split 4)            + one `decoy` edge root -> l1
///        lower (split 2)    upper (split 6)
///      l1 "ab"   l2 "cd"   l3 "ef"   l4 "gh"
/// ```
pub fn tree(change: impl FnOnce(&mut GraphStore)) -> (WorldlineFrontier, NodeKey) {
    let (state, root) = tree_state(change);
    (
        WorldlineFrontier::new(WorldlineId::from_bytes([7; 32]), state),
        root,
    )
}

/// The same stored tree as a state a runtime host can own.
pub fn tree_state(change: impl FnOnce(&mut GraphStore)) -> (WorldlineState, NodeKey) {
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
    link(&mut store, "root", "decoy", "l1");
    change(&mut store);
    let root = NodeKey {
        warp_id: store.warp_id(),
        local_id: node("root"),
    };
    (
        WorldlineState::from_root_store(store, root.local_id).unwrap(),
        root,
    )
}

pub fn sorted(names: &[&str]) -> Vec<TypeId> {
    let mut types: Vec<_> = names.iter().map(|name| kind(name)).collect();
    types.sort();
    types
}

pub fn limits() -> ReadLimits {
    ReadLimits {
        evaluation: EvaluationLimits {
            max_package_bytes: 65_536,
            max_input_bytes: 65_536,
            max_steps: 1_048_576,
            max_allocated_bytes: 16_777_216,
            max_output_bytes: 8_388_608,
        },
        max_reads: 64,
        max_read_bytes: 1_048_576,
    }
}

/// Evaluates one synthetic package from `start` under a typed graph aperture.
pub fn run(
    frontier: &WorldlineFrontier,
    start: NodeKey,
    build: &Build,
    read: Value,
    scans: u64,
    limits: ReadLimits,
) -> Result<ReadResult, ReadError> {
    let aperture = [start];
    let (edge_types, node_types) = (sorted(&["left", "right"]), sorted(&["branch", "leaf"]));
    let view = ReadView::at(frontier, &aperture)
        .unwrap()
        .with_graph_aperture(GraphReadAperture {
            edge_types: &edge_types,
            node_types: &node_types,
            max_edge_scans: scans,
        })
        .unwrap();
    let (package, pin) = package(build);
    let input = encode(&record([("read", read)])).unwrap();
    evaluate(&package, pin, &input, &view, limits)
}

pub fn edge_request(source: NodeKey, edge_type: &str) -> Value {
    record([
        ("warpId", id(source.warp_id.0)),
        ("nodeId", id(source.local_id.0)),
        ("edgeType", id(kind(edge_type).0)),
    ])
}

pub fn guide(a: u64, b: u64, follow_a: bool, follow_b: bool, reach: u64) -> Value {
    record([
        ("a", word(a)),
        ("b", word(b)),
        ("followA", Value::Bool(follow_a)),
        ("followB", Value::Bool(follow_b)),
        ("reach", word(reach)),
    ])
}

pub fn closure_request(start: NodeKey, guide: Value, max_nodes: u64, max_depth: u64) -> Value {
    record([
        ("warpId", id(start.warp_id.0)),
        ("nodeId", id(start.local_id.0)),
        (
            "edgeTypes",
            Value::Array(vec![id(kind("left").0), id(kind("right").0)]),
        ),
        ("maxNodes", word(max_nodes)),
        ("maxDepth", word(max_depth)),
        ("guide", guide),
    ])
}

pub fn node_row(name: &str, node_type: &str, atom: &[u8]) -> Value {
    record([
        ("nodeId", id(node(name).0)),
        ("nodeType", id(kind(node_type).0)),
        ("atomType", id(kind("text").0)),
        ("atomBytes", Value::Bytes(atom.to_vec())),
    ])
}
pub fn edge_row(from: u64, slot: u64, edge: &str, target: u64) -> Value {
    record([
        ("from", word(from)),
        ("slot", word(slot)),
        ("edgeId", id(make_edge_id(edge).0)),
        ("node", word(target)),
    ])
}
pub fn occurrence(node: u64, edge: u64, depth: u64, guide: Value) -> Value {
    record([
        ("node", word(node)),
        ("edge", word(edge)),
        ("depth", word(depth)),
        ("guide", guide),
    ])
}
pub fn canonical(value: &Value) -> Vec<u8> {
    encode(value).unwrap()
}
pub fn obstruction(result: Result<ReadResult, ReadError>) -> Obstruction {
    match result {
        Err(ReadError::GraphObstructed { coordinate, cause }) => {
            // The authored arm that fired is the one declared for this cause.
            assert_eq!(coordinate, arm(cause));
            cause
        }
        other => panic!("expected an authored graph obstruction, got {other:?}"),
    }
}

/// Both boundary paths of the fixture tree: offset 1 ends in `l1`, offset 6
/// ends in `l4`. Every child of an expanded occurrence is material.
pub fn boundary_guide() -> Value {
    guide(1, 6, true, true, 2)
}
