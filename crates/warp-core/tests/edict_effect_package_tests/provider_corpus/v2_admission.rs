// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! The public compiler's effect programs run at exactly the compiler's own
//! shared-read minimum (echo#1016).
//!
//! Each package is unchanged public compiler output retained under
//! `fixtures/edict-effect-v2-admission/`. The budget is the minimum that
//! Amblicit e306ecc9 reports with Echo's `/v3` templates (unchanged since
//! 80b52f95 for the read and the mutations) and that
//! `effect_programs_are_admitted_at_exactly_the_compiler_minimum` in
//! `echo-edict-provider-lowerer` pins. Oracles: the copying executor on the
//! same package, input and graph (a differential), and that static bound,
//! which the runtime's charge must not exceed on either axis.
use super::closure_witness::repinned;
use super::*;
use warp_core::edict_effects::{EffectMutation, TypedNodeRead};
use warp_core::{
    AtomPayload, AttachmentValue, GraphStore, NodeId, NodeKey, NodeRecord, TypeId, WarpId,
};

const READ: &str =
    include_str!("../../fixtures/edict-effect-v2-admission/read-executable-operation-package.hex");
const WRITE: &str =
    include_str!("../../fixtures/edict-effect-v2-admission/write-executable-operation-package.hex");
/// The compiler's shared-read minimums, (steps, bytes).
const READ_MINIMUM: (u64, u64) = (100, 1_631);
const WRITE_MINIMUM: (u64, u64) = (176, 6_796);

/// The node and atom types the read and write programs' schemas name.
const NODE_TYPE: TypeId = TypeId([0x5b; 32]);
const ATOM_TYPE: TypeId = TypeId([0x5c; 32]);
const EDGE_TYPE: TypeId = TypeId([0x5d; 32]);
const ROOT: NodeKey = NodeKey {
    warp_id: WarpId([0x31; 32]),
    local_id: NodeId([0x32; 32]),
};
/// The atom the read returns: 64 bytes, the largest `NodeRow` admits.
const FACT: [u8; 64] = [0x61; 64];

fn package(hex_text: &str) -> V {
    echo_edict_canonical::decode_canonical_cbor_v1(&hex::decode(hex_text.trim()).expect("hex"))
        .expect("canonical package")
}

/// What one run returned, buffered and was charged.
struct Charged {
    result: Result<Vec<u8>, EffectFrameError>,
    mutations: Vec<EffectMutation>,
    steps: u64,
    bytes: u64,
}

/// A root node carrying `FACT`, the only state either program sees.
fn store() -> GraphStore {
    let mut store = GraphStore::new(ROOT.warp_id);
    store.insert_node(ROOT.local_id, NodeRecord { ty: NODE_TYPE });
    store.set_node_attachment(
        ROOT.local_id,
        Some(AttachmentValue::Atom(AtomPayload {
            type_id: ATOM_TYPE,
            bytes: FACT.to_vec().into(),
        })),
    );
    store
}

/// Runs `package` once over `store()` in the widest frame, so only the
/// package's own Core budget can refuse it.
fn run_charged(package: &V, input: &[u8], reads: &[TypedNodeRead]) -> Charged {
    let prepared = prepare(package).expect("package prepares");
    let program = prepared.program().as_effect_program().expect("effect");
    let (host, head, _) = super::super::execution::host_with_store(store(), ROOT);
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
            &[],
            0,
            EchoOperationBudgetV1::new(frame.steps, 64 * 1024 * 1024, frame.write_bytes),
            frame.limits,
        )
        .expect("frame")
        .with_footprint_law(program.footprint_law().clone())
        .expect("law");
    let result = evaluation
        .evaluate_effect(program, input, reads)
        .map(<[u8]>::to_vec);
    Charged {
        mutations: evaluation.buffered_mutations().to_vec(),
        result,
        steps: evaluation.consumed().steps(),
        bytes: evaluation.allocated_bytes(),
    }
}

/// Runs `package` under the copying schedule and again under the shared-read
/// schedule at exactly `minimum`; the copying run returns or obstructs with an
/// authored reason, both must agree, and the shared run's charge
/// must stay within `minimum` on both axes.
fn assert_runs_within(
    name: &str,
    package: &V,
    minimum: (u64, u64),
    input: &[u8],
    reads: &[TypedNodeRead],
) -> Charged {
    let copying = run_charged(package, input, reads);
    assert!(
        matches!(
            copying.result,
            Ok(_) | Err(EffectFrameError::AuthoredObstruction { .. })
        ),
        "{name}: copying run {:?}",
        copying.result
    );
    let exact = repinned(package, "core.cost.shared-reads/v1", Some(minimum));
    let sharing = run_charged(&exact, input, reads);
    assert_eq!(sharing.result, copying.result, "{name}: result");
    assert_eq!(sharing.mutations, copying.mutations, "{name}: mutations");
    let (steps, bytes) = minimum;
    assert!(
        sharing.steps <= steps && sharing.bytes <= bytes,
        "{name}: runtime charge ({}, {}) exceeds the static bound ({steps}, {bytes})",
        sharing.steps,
        sharing.bytes
    );
    sharing
}

/// The node-atom read returns its 64-byte fact at exactly the compiler's
/// minimum.
#[test]
fn the_node_atom_read_runs_within_the_compiler_minimum() {
    let input = encoded(&map([
        ("anchor", V::Bytes(vec![0x5a; 32])),
        (
            "address",
            map([
                ("nodeId", V::Bytes(ROOT.local_id.0.to_vec())),
                ("typeId", V::Bytes(ATOM_TYPE.0.to_vec())),
                ("warpId", V::Bytes(ROOT.warp_id.0.to_vec())),
            ]),
        ),
    ]));
    let reads = [TypedNodeRead {
        node: ROOT,
        absent_node_type: NODE_TYPE,
    }];
    let run = assert_runs_within(
        "node-atom-read",
        &package(READ),
        READ_MINIMUM,
        &input,
        &reads,
    );
    assert_eq!(
        run.result,
        Ok(encoded(&V::Bytes(FACT.to_vec()))),
        "the fact"
    );
}

/// The five mutation intrinsics emit their five intents at exactly the
/// compiler's minimum, on the input of
/// `the_public_mutation_package_emits_each_intent_in_source_order` (64-byte
/// atoms exceed the program's Core input limit).
#[test]
fn the_mutation_intrinsics_run_within_the_compiler_minimum() {
    let warp = V::Bytes(ROOT.warp_id.0.to_vec());
    let id = |byte: u8| V::Bytes(vec![byte; 32]);
    let node_write = |node: u8, atom: &[u8]| {
        map([
            ("warpId", warp.clone()),
            ("nodeId", id(node)),
            ("nodeType", V::Bytes(NODE_TYPE.0.to_vec())),
            ("atomType", V::Bytes(ATOM_TYPE.0.to_vec())),
            ("atomBytes", V::Bytes(atom.to_vec())),
        ])
    };
    let input = encoded(&map([
        ("anchor", id(0x40)),
        ("node", node_write(0x41, b"ensured")),
        ("created", node_write(0x42, b"created")),
        (
            "atom",
            map([
                ("warpId", warp.clone()),
                ("nodeId", id(0x41)),
                ("expectedAtomDigest", id(0x43)),
                ("atomType", V::Bytes(ATOM_TYPE.0.to_vec())),
                ("atomBytes", V::Bytes(b"replaced".to_vec())),
            ]),
        ),
        (
            "edge",
            map([
                ("warpId", warp.clone()),
                ("edgeId", id(0x44)),
                ("fromId", id(0x41)),
                ("toId", id(0x42)),
                ("edgeType", V::Bytes(EDGE_TYPE.0.to_vec())),
            ]),
        ),
        (
            "retarget",
            map([
                ("warpId", warp),
                ("edgeId", id(0x44)),
                ("expectedToId", id(0x42)),
                ("toId", id(0x41)),
            ]),
        ),
    ]));
    let run = assert_runs_within("mutations", &package(WRITE), WRITE_MINIMUM, &input, &[]);
    assert_eq!(run.mutations.len(), 5, "five intents");
}

/// The loop programs' checked packages and the compiler's shared-read
/// minimums, which Amblicit e306ecc9 reports and
/// `effect_programs_are_admitted_at_exactly_the_compiler_minimum` pins.
const LOOPS: [(&str, &str, (u64, u64)); 6] = [
    (
        "loop-scalar",
        include_str!("../../fixtures/edict-effect-v2-admission/payload-loop-scalar-executable-operation-package.hex"),
        (300, 3_200),
    ),
    (
        "loop-record",
        include_str!("../../fixtures/edict-effect-v2-admission/payload-loop-record-executable-operation-package.hex"),
        (318, 4_190),
    ),
    (
        "loop-nested",
        include_str!("../../fixtures/edict-effect-v2-admission/payload-loop-nested-executable-operation-package.hex"),
        (783, 5_144),
    ),
    (
        "loop-entry",
        include_str!("../../fixtures/edict-effect-v2-admission/payload-loop-entry-executable-operation-package.hex"),
        (336, 3_998),
    ),
    (
        "loop-bytes",
        include_str!("../../fixtures/edict-effect-v2-admission/payload-loop-bytes-executable-operation-package.hex"),
        (1_134, 3_608),
    ),
    (
        "loop-mixed",
        include_str!("../../fixtures/edict-effect-v2-admission/payload-loop-mixed-executable-operation-package.hex"),
        (339, 4_232),
    ),
];

/// The typed-failure input with `end` 10 and three loop items, the one at
/// `failing` (if any) out of range: an integer or a bytes item longer than
/// `end`, or an entry whose value exceeds its limit.
fn loop_input(name: &str, failing: Option<usize>) -> Vec<u8> {
    let int = |value: u64| V::Integer(i128::from(value));
    let items = (0..3)
        .map(|index| {
            let bad = failing == Some(index);
            match name {
                "loop-entry" => map([("value", int(if bad { 11 } else { 4 })), ("limit", int(10))]),
                "loop-bytes" => V::Bytes(vec![0x62; if bad { 8 } else { 3 }]),
                _ => int(if bad { 11 } else { 4 }),
            }
        })
        .collect();
    let end = if name == "loop-bytes" { 5 } else { 10 };
    encoded(&map([
        ("anchor", V::Bytes(vec![0x40; 32])),
        ("start", int(0)),
        ("end", int(end)),
        ("items", V::Array(items)),
        (
            "node",
            map([
                ("warpId", V::Bytes(ROOT.warp_id.0.to_vec())),
                ("nodeId", V::Bytes(vec![0x41; 32])),
                ("nodeType", V::Bytes(NODE_TYPE.0.to_vec())),
                ("atomType", V::Bytes(ATOM_TYPE.0.to_vec())),
                ("atomBytes", V::Bytes(b"seeded".to_vec())),
            ]),
        ),
    ]))
}

/// Every loop program runs at exactly the compiler's minimum on every path:
/// all three items pass, or the first, second or third fails its `require`
/// and the program obstructs with `RangeInverted` (echo#1016).
#[test]
fn the_loop_programs_run_within_the_compiler_minimum_on_every_path() {
    for (name, hex_text, minimum) in LOOPS {
        let package = package(hex_text);
        for failing in [None, Some(0), Some(1), Some(2)] {
            let label = format!("{name}, failing item {failing:?}");
            let run =
                assert_runs_within(&label, &package, minimum, &loop_input(name, failing), &[]);
            match failing {
                None => assert!(run.result.is_ok(), "{label}: {:?}", run.result),
                Some(_) => assert!(
                    matches!(&run.result,
                        Err(EffectFrameError::AuthoredObstruction { reason_kind, .. })
                            if reason_kind == "fx.RangeInverted"),
                    "{label}: {:?}",
                    run.result
                ),
            }
        }
    }
}
