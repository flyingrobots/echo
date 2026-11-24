// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! The public compiler's closure-read effect witness under the shared-read
//! schedule (echo#1016).
//!
//! Under that schedule the lowerer charges a closure read its declared
//! selector only. The witness program reads one guided closure through
//! `echo.dpo@2.closure-read` and returns it; its package is unchanged public
//! compiler output retained under `fixtures/edict-effect-closure-read/`.
//! Oracles: the copying executor on the same input and graph (a
//! differential), and the lowerer minimum that
//! `closure_read_witness_minimums_under_each_schedule` in
//! `echo-edict-provider-lowerer` measures and pins.
use super::*;
use warp_core::edict_effects::TypedNodeRead;
use warp_core::{
    AtomPayload, AttachmentValue, EdgeId, EdgeRecord, GraphStore, NodeId, NodeKey, NodeRecord,
    TypeId, WarpId,
};

const PACKAGE: &str = include_str!(
    "../../fixtures/edict-effect-closure-read/closure-executable-operation-package.hex"
);
/// The lowerer's shared-schedule minimum for the witness: (steps, bytes).
const LOWERER_SHARED_MINIMUM: (u64, u64) = (8666, 504_387);

const WARP: WarpId = WarpId([3; 32]);
/// The node and edge types the witness's graph schema and footprint law name.
const NODE_TYPE: TypeId = TypeId([0x6b; 32]);
const EDGE_TYPE: TypeId = TypeId([0x6d; 32]);
/// Nodes in the tree; the read's node bound.
const NODES: u8 = 8;
/// Each parent's `E` edges in slot order. The root follows both slots; the
/// selector then keeps a child on the slot that reached it, so a slot-0 chain
/// (0, 1, 3, 5, 7) reaches depth four, the read's depth bound, and a slot-1
/// chain (0, 2, 4, 6) passes each node's unfollowed slot-0 edge to node 7.
const EDGES: [(u8, u8); 10] = [
    (0, 1),
    (0, 2),
    (1, 3),
    (2, 7),
    (2, 4),
    (3, 5),
    (4, 7),
    (4, 6),
    (5, 7),
    (6, 7),
];
/// Reads the widened footprint law allows: each node, its atom and its range
/// fit well within it.
const WIDE_READS: u64 = 64;

fn witness() -> V {
    echo_edict_canonical::decode_canonical_cbor_v1(&hex::decode(PACKAGE.trim()).expect("hex"))
        .expect("canonical package")
}

fn node(index: u8) -> NodeId {
    NodeId([0x70 + index; 32])
}

/// The graph of `EDGES`, whose every node carries a 64-byte atom, the largest
/// `NodeRow` admits; each parent orders its `E` edges as slots 0 and 1.
fn tree() -> GraphStore {
    let mut store = GraphStore::new(WARP);
    for index in 0..NODES {
        store.insert_node(node(index), NodeRecord { ty: NODE_TYPE });
        store.set_node_attachment(
            node(index),
            Some(AttachmentValue::Atom(AtomPayload {
                type_id: TypeId([2; 32]),
                bytes: vec![0x61; 64].into(),
            })),
        );
    }
    for (index, (parent, child)) in EDGES.iter().enumerate() {
        store.insert_edge(
            node(*parent),
            EdgeRecord {
                id: EdgeId([0x90 + index as u8; 32]),
                from: node(*parent),
                to: node(*child),
                ty: EDGE_TYPE,
            },
        );
    }
    store
}

/// Read the whole tree from its root, following both slots everywhere.
fn input() -> Vec<u8> {
    let id = |bytes: &[u8]| V::Bytes(bytes.to_vec());
    encoded(&map([
        ("anchor", V::Bytes(vec![0x5a; 32])),
        (
            "read",
            map([
                ("edgeTypes", V::Array(vec![id(&EDGE_TYPE.0)])),
                (
                    "guide",
                    map([
                        ("a", V::Integer(0)),
                        ("b", V::Integer(0)),
                        ("followA", V::Bool(true)),
                        ("followB", V::Bool(true)),
                        ("reach", V::Integer(4)),
                    ]),
                ),
                ("maxDepth", V::Integer(4)),
                ("maxNodes", V::Integer(8)),
                ("nodeId", id(&node(0).0)),
                ("warpId", id(&WARP.0)),
            ]),
        ),
    ]))
}

/// Runs `package` over the tree, returning its result and the steps and
/// bytes the runtime charged.
fn run_closure(package: &V) -> (Result<Vec<u8>, EffectFrameError>, u64, u64) {
    let prepared = prepare(package).expect("package prepares");
    let program = prepared.program().as_effect_program().expect("effect");
    let root = NodeKey {
        warp_id: WARP,
        local_id: node(0),
    };
    let (host, head, root) = super::super::execution::host_with_store(tree(), root);
    let basis = host
        .echo_operation_evaluation_basis_v1(
            head,
            EchoOperationApplicationBasisV1::new([6; 32], [7; 32]),
        )
        .expect("basis");
    let frame = Frame::widest(None);
    let mut evaluation = host
        .effect_evaluation_frame_v1(
            basis,
            &[EDGE_TYPE],
            64,
            EchoOperationBudgetV1::new(frame.steps, 64 * 1024 * 1024, frame.write_bytes),
            frame.limits,
        )
        .expect("frame")
        .with_footprint_law(program.footprint_law().clone())
        .expect("law");
    let result = evaluation
        .evaluate_effect(
            program,
            &input(),
            &[TypedNodeRead {
                node: root,
                absent_node_type: NODE_TYPE,
            }],
        )
        .map(<[u8]>::to_vec);
    (
        result,
        evaluation.consumed().steps(),
        evaluation.allocated_bytes(),
    )
}

/// The witness with its Core requiring the shared-read schedule and, when
/// given, a Core budget of (steps, bytes), repinned in place.
///
/// Unlike `rebuild`, every other part of the public package (its graph
/// schema, footprint law, graph-read declaration and lowering artifacts) is
/// kept, since a closure read depends on them.
pub(super) fn repinned(package: &V, capability: &str, budget: Option<(u64, u64)>) -> V {
    let mut program = decode(get(package, "program"));
    let mut bundle = decode(get(&program, "compiled_program"));
    let mut core = decode(get(&bundle, "core_artifact"));
    let mut target = decode(get(&bundle, "target_ir_artifact"));
    let V::Array(capabilities) = value_field_mut(&mut core, "requiredCoreCapabilities") else {
        panic!("capabilities are a list")
    };
    capabilities.push(text(capability));
    if let Some((steps, bytes)) = budget {
        for (axis, value) in [("maxSteps", steps), ("maxAllocatedBytes", bytes)] {
            let value = V::Integer(i128::from(value));
            set(
                value_field_mut(first_intent_mut(&mut core), "coreEvaluationBudget"),
                axis,
                value.clone(),
            );
            set(
                value_field_mut(selected_target_mut(&mut target), "coreEvaluationBudget"),
                axis,
                value,
            );
        }
    }
    set(
        value_field_mut(
            value_field_mut(&mut target, "semanticClosure"),
            "sourceCore",
        ),
        "digest",
        V::Array(vec![
            text("sha256"),
            artifact_identity("edict.core.module/v1", &core),
        ]),
    );
    set(&mut bundle, "core_artifact", V::Bytes(encoded(&core)));
    set(
        &mut bundle,
        "target_ir_artifact",
        V::Bytes(encoded(&target)),
    );
    set(&mut program, "compiled_program", V::Bytes(encoded(&bundle)));
    set(
        &mut program,
        "target_ir_identity",
        artifact_identity("edict.target-ir.artifact/v1", &target),
    );
    let mut repinned = package.clone();
    let closure = value_field_mut(&mut repinned, "semantic_closure");
    set(closure, "core_identity", core_identity(&program));
    set(closure, "target_ir_identity", target_identity(&program));
    set(&mut repinned, "program", V::Bytes(encoded(&program)));
    repinned
}

/// Given exactly the lowerer's shared minimum as its Core budget, the
/// witness reads the whole tree to the copying schedule's result, and the
/// runtime's charge stays at or below that static bound on both axes.
#[test]
fn the_closure_witness_runs_within_the_lowerer_minimum() {
    let package = with_max_reads(&witness(), WIDE_READS, true);
    let (copying, _, _) = run_closure(&package);
    let closure = copying.unwrap_or_else(|error| panic!("the copying schedule reads the tree: {error:?}"));
    assert_eq!(listed(&closure, "nodes"), Some(usize::from(NODES)), "every node");
    // Control for the repinning: an unused capability added the same way.
    let (control, _, _) = run_closure(&repinned(&package, "core.variant/v1", None));
    assert_eq!(control.as_deref(), Ok(closure.as_slice()), "control");
    let (steps, bytes) = LOWERER_SHARED_MINIMUM;
    let exact = repinned(&package, "core.cost.shared-reads/v1", Some((steps, bytes)));
    let (sharing, charged_steps, charged_bytes) = run_closure(&exact);
    assert_eq!(sharing.as_deref(), Ok(closure.as_slice()), "result");
    assert!(
        charged_steps <= steps && charged_bytes <= bytes,
        "runtime charge ({charged_steps}, {charged_bytes}) exceeds the static bound ({steps}, {bytes})"
    );
}

/// The witness with its footprint law allowing `max_reads` reads and as many
/// range edges. The law's
/// bytes are bound twice: inside the program and, as
/// `footprint_contract_identity`, in the package. With `repin` false only the
/// program changes, so the two disagree.
fn with_max_reads(package: &V, max_reads: u64, repin: bool) -> V {
    let mut program = decode(get(package, "program"));
    let mut law = decode(get(&program, "footprint_law"));
    set(&mut law, "max_reads", V::Integer(i128::from(max_reads)));
    // The bucket edges every read range may hold, widened alike.
    set(&mut law, "max_range_edges", V::Integer(i128::from(max_reads)));
    let law = encoded(&law);
    set(&mut program, "footprint_law", V::Bytes(law.clone()));
    let mut widened = package.clone();
    set(&mut widened, "program", V::Bytes(encoded(&program)));
    if repin {
        let mut identity = blake3::Hasher::new();
        identity.update(b"echo:operation-footprint-law:v1\0");
        identity.update(&law);
        set(
            &mut widened,
            "footprint_contract_identity",
            V::Bytes(identity.finalize().as_bytes().to_vec()),
        );
    }
    widened
}

/// A wider law is refused only while the package still pins the old law's
/// identity: preparation's footprint-contract check, not a reads ceiling
/// (the law admits up to 4096 reads).
#[test]
fn a_wider_footprint_law_prepares_once_its_identity_is_repinned() {
    let package = witness();
    for max_reads in [17, 64, 4096] {
        assert!(
            prepare(&with_max_reads(&package, max_reads, false)).is_err(),
            "stale identity, max_reads {max_reads}"
        );
        assert!(
            prepare(&with_max_reads(&package, max_reads, true)).is_ok(),
            "repinned identity, max_reads {max_reads}"
        );
    }
}

/// The length of the first list under `name` in a canonical result.
fn listed(result: &[u8], name: &str) -> Option<usize> {
    fn find(value: &V, name: &str) -> Option<usize> {
        match value {
            V::Map(entries) => entries.iter().find_map(|(key, inner)| match (key, inner) {
                (V::Text(key), V::Array(items)) if key == name => Some(items.len()),
                _ => find(inner, name),
            }),
            V::Array(items) => items.iter().find_map(|item| find(item, name)),
            _ => None,
        }
    }
    find(&echo_edict_canonical::decode_canonical_cbor_v1(result).ok()?, name)
}
