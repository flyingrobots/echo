// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Real compiler packages read stored atoms through one bounded frontier view.
#![cfg(feature = "trusted_runtime")]
#![allow(clippy::unwrap_used, clippy::panic)]

use echo_edict_canonical::{
    decode_canonical_cbor_v1 as decode, digest_canonical_value_bytes_v1 as digest,
    encode_canonical_cbor_v1 as encode, CanonicalValueV1 as Value,
};
use warp_core::edict_pure::{EvaluationError, EvaluationLimits};
use warp_core::edict_read::{
    evaluate, ReadBasis, ReadError, ReadLimits, ReadObstruction, ReadResult, ReadView,
};
use warp_core::{
    make_node_id, make_type_id, AtomPayload, AttachmentValue, GraphStore, NodeKey, NodeRecord,
    WarpId, WorldlineFrontier, WorldlineId, WorldlineState, WorldlineTick,
};

const SINGLE: &str =
    include_str!("fixtures/edict-node-read/single-executable-operation-package.hex");
const PAIR: &str = include_str!("fixtures/edict-node-read/pair-executable-operation-package.hex");

fn limits() -> ReadLimits {
    ReadLimits {
        evaluation: EvaluationLimits {
            max_package_bytes: 65_536,
            max_input_bytes: 2_097_152,
            max_steps: 1_048_576,
            max_allocated_bytes: 16_777_216,
            max_output_bytes: 8_388_608,
        },
        max_reads: 64,
        max_read_bytes: 1_048_576,
    }
}

fn record(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(
        fields
            .into_iter()
            .map(|(key, value)| (Value::Text(key.into()), value))
            .collect(),
    )
}

fn frontier() -> (WorldlineFrontier, Vec<NodeKey>) {
    let mut store = GraphStore::default();
    let mut keys = Vec::new();
    for (name, payload) in [
        ("first", b"alpha".as_slice()),
        ("second", b"bravo".as_slice()),
    ] {
        let id = make_node_id(name);
        store.insert_node(
            id,
            NodeRecord {
                ty: make_type_id("node"),
            },
        );
        store.set_node_attachment(
            id,
            Some(AttachmentValue::Atom(AtomPayload {
                type_id: make_type_id("document"),
                bytes: payload.to_vec().into(),
            })),
        );
        keys.push(NodeKey {
            warp_id: store.warp_id(),
            local_id: id,
        });
    }
    let state = WorldlineState::from_root_store(store, keys[0].local_id).unwrap();
    (
        WorldlineFrontier::new(WorldlineId::from_bytes([7; 32]), state),
        keys,
    )
}

fn address(key: NodeKey) -> Value {
    record([
        ("warpId", Value::Bytes(key.warp_id.0.to_vec())),
        ("nodeId", Value::Bytes(key.local_id.0.to_vec())),
        ("typeId", Value::Bytes(make_type_id("document").0.to_vec())),
    ])
}

#[test]
fn public_compiler_pair_reads_independent_stored_atoms_in_order() {
    let (frontier, keys) = frontier();
    let before = frontier.state().state_root();
    let basis = ReadBasis::at(&frontier);
    let mut aperture = keys.clone();
    aperture.sort();
    let view = ReadView::new(&frontier, basis, &aperture).unwrap();
    let (package, pin) = package(true);
    let input = encode(&record([
        ("firstAddress", address(keys[0])),
        ("firstExpected", Value::Bytes(b"alpha".to_vec())),
        ("secondAddress", address(keys[1])),
        ("secondExpected", Value::Bytes(b"bravo".to_vec())),
    ]))
    .unwrap();
    let result = evaluate(&package, pin, &input, &view, limits()).unwrap();
    assert_eq!(
        decode(&result.output).unwrap(),
        Value::Bytes(b"bravo".to_vec())
    );
    assert_eq!(result.reads, 2);
    assert_eq!(result.read_bytes, 10);
    assert_eq!(result.basis, basis);
    assert_eq!(
        result,
        evaluate(&package, pin, &input, &view, limits()).unwrap()
    );
    assert_eq!(frontier.state().state_root(), before);
}

const SINGLE_REPORT: &str = include_str!("fixtures/edict-node-read/single-verification-report.hex");
const PAIR_REPORT: &str = include_str!("fixtures/edict-node-read/pair-verification-report.hex");

fn artifact(pair: bool, name: &str, retained: &str) -> Vec<u8> {
    // The end-to-end Docker witness substitutes fresh compiler output only.
    // Expected pins stay independent of this input selection.
    if let Some(root) = std::env::var_os("EDICT_READ_OUTPUT_ROOT") {
        std::fs::read(
            std::path::PathBuf::from(root)
                .join(if pair { "pair" } else { "single" })
                .join(".build/application")
                .join(format!("{name}.cbor")),
        )
        .unwrap()
    } else {
        hex::decode(retained.trim()).unwrap()
    }
}

fn package(pair: bool) -> (Vec<u8>, [u8; 32]) {
    let raw = artifact(
        pair,
        "executable-operation-package",
        if pair { PAIR } else { SINGLE },
    );
    let pin = if pair {
        "359f1ff5d5fd82ebbbb8e3e3e2d454014ae66bbf77184f5cc497edfc57abddb5"
    } else {
        "a2ae35d39ae74a26da916544464529bc6c7af32a76b6e6f354ef79c5063ab109"
    };
    (raw, hex::decode(pin).unwrap().try_into().unwrap())
}

fn single_input(key: NodeKey, expected: &[u8]) -> Vec<u8> {
    encode(&record([
        ("address", address(key)),
        ("expected", Value::Bytes(expected.to_vec())),
    ]))
    .unwrap()
}

fn pair_input(keys: &[NodeKey], first: &[u8]) -> Vec<u8> {
    encode(&record([
        ("firstAddress", address(keys[0])),
        ("firstExpected", Value::Bytes(first.to_vec())),
        ("secondAddress", address(keys[1])),
        ("secondExpected", Value::Bytes(b"bravo".to_vec())),
    ]))
    .unwrap()
}

fn single(
    frontier: &WorldlineFrontier,
    key: NodeKey,
    expected: &[u8],
    limits: ReadLimits,
) -> Result<ReadResult, ReadError> {
    let aperture = [key];
    let view = ReadView::new(frontier, ReadBasis::at(frontier), &aperture).unwrap();
    let (package, pin) = package(false);
    evaluate(&package, pin, &single_input(key, expected), &view, limits)
}

fn field<'a>(value: &'a Value, name: &str) -> &'a Value {
    let Value::Map(fields) = value else {
        panic!("map");
    };
    &fields
        .iter()
        .find(|(key, _)| key == &Value::Text(name.into()))
        .unwrap()
        .1
}

#[test]
fn compiler_outputs_have_exact_independently_recorded_pins_and_accepted_reports() {
    for pair in [false, true] {
        let (raw, pin) = package(pair);
        assert_eq!(
            digest("echo.operation-package/v1", &decode(&raw).unwrap()).unwrap(),
            pin
        );
        let retained_report = if pair { PAIR_REPORT } else { SINGLE_REPORT };
        let report_bytes = artifact(pair, "verification-report", retained_report);
        assert_eq!(
            report_bytes,
            hex::decode(retained_report.trim()).unwrap(),
            "fresh verifier report differs from the independently retained report"
        );
        let report = decode(&report_bytes).unwrap();
        assert_eq!(field(&report, "outcome"), &Value::Text("accepted".into()));
        assert_eq!(
            field(field(&report, "package"), "digest"),
            &Value::Array(vec![
                Value::Text("sha256".into()),
                Value::Bytes(pin.to_vec())
            ])
        );
    }
}

#[test]
fn single_read_reports_real_basis_application_proposition_and_exact_accounting() {
    let (frontier, keys) = frontier();
    let result = single(&frontier, keys[0], b"alpha", limits()).unwrap();
    assert_eq!(
        decode(&result.output).unwrap(),
        Value::Bytes(b"alpha".to_vec())
    );
    assert_eq!(result.basis.worldline_id(), frontier.worldline_id());
    assert_eq!(result.basis.tick(), frontier.frontier_tick());
    assert_eq!(result.basis.state_root(), frontier.state().state_root());
    assert_eq!(result.application_basis, keys[0].local_id.0);
    assert_eq!(result.reads, 1);
    assert_eq!(result.read_bytes, 5);
    assert!(result.steps > 5 && result.allocated_bytes > 5);
    let exact = ReadLimits {
        evaluation: EvaluationLimits {
            max_steps: result.steps,
            max_allocated_bytes: result.allocated_bytes,
            max_output_bytes: result.output.len() as u64,
            ..limits().evaluation
        },
        max_reads: 1,
        max_read_bytes: 5,
    };
    assert_eq!(result, single(&frontier, keys[0], b"alpha", exact).unwrap());
}

#[test]
fn current_frontier_view_preserves_basis_aperture_and_evaluation() {
    let (frontier, keys) = frontier();
    let aperture = [keys[0]];
    let view = ReadView::at(&frontier, &aperture).unwrap();
    assert_eq!(view.basis(), ReadBasis::at(&frontier));
    let (package, pin) = package(false);
    assert_eq!(
        evaluate(
            &package,
            pin,
            &single_input(keys[0], b"alpha"),
            &view,
            limits()
        ),
        single(&frontier, keys[0], b"alpha", limits())
    );
    let mut descending = keys.clone();
    descending.sort_by(|left, right| right.cmp(left));
    for invalid in [vec![keys[0]; 2], descending, vec![keys[0]; 65_537]] {
        assert!(matches!(
            ReadView::at(&frontier, &invalid),
            Err(ReadError::InvalidAperture)
        ));
    }
    let empty = ReadView::at(&frontier, &[]).unwrap();
    assert_eq!(
        evaluate(
            &package,
            pin,
            &single_input(keys[0], b"alpha"),
            &empty,
            limits()
        ),
        Err(ReadError::OutsideAperture(keys[0]))
    );
}

#[test]
fn host_budgets_refuse_one_unit_below_the_exact_success_boundary() {
    let (frontier, keys) = frontier();
    let result = single(&frontier, keys[0], b"alpha", limits()).unwrap();
    let mut constrained = limits();
    constrained.max_reads = 0;
    assert_eq!(
        single(&frontier, keys[0], b"alpha", constrained),
        Err(ReadError::ReadCountExceeded)
    );
    constrained = limits();
    constrained.max_read_bytes = 4;
    assert_eq!(
        single(&frontier, keys[0], b"alpha", constrained),
        Err(ReadError::ReadBytesExceeded)
    );
    for (kind, expected) in [
        ("steps", EvaluationError::StepBudgetExceeded),
        ("allocated", EvaluationError::AllocationBudgetExceeded),
        ("output", EvaluationError::OutputBudgetExceeded),
    ] {
        constrained = limits();
        match kind {
            "steps" => constrained.evaluation.max_steps = result.steps - 1,
            "allocated" => constrained.evaluation.max_allocated_bytes = result.allocated_bytes - 1,
            "output" => constrained.evaluation.max_output_bytes = result.output.len() as u64 - 1,
            _ => panic!("known limit"),
        }
        assert_eq!(
            single(&frontier, keys[0], b"alpha", constrained),
            Err(ReadError::Evaluation(expected))
        );
    }
}

#[test]
fn pair_reads_charge_aggregate_bytes_and_attempts() {
    let (frontier, keys) = frontier();
    let mut aperture = keys.clone();
    aperture.sort();
    let view = ReadView::new(&frontier, ReadBasis::at(&frontier), &aperture).unwrap();
    let (raw, pin) = package(true);
    let input = pair_input(&keys, b"alpha");
    let mut constrained = limits();
    constrained.max_reads = 1;
    assert_eq!(
        evaluate(&raw, pin, &input, &view, constrained),
        Err(ReadError::ReadCountExceeded)
    );
    constrained = limits();
    constrained.max_read_bytes = 9;
    assert_eq!(
        evaluate(&raw, pin, &input, &view, constrained),
        Err(ReadError::ReadBytesExceeded)
    );
}

#[test]
fn false_first_guard_precedes_the_second_read_and_publishes_no_result() {
    let (frontier, keys) = frontier();
    let before = frontier.state().state_root();
    let aperture = [keys[0]];
    let view = ReadView::new(&frontier, ReadBasis::at(&frontier), &aperture).unwrap();
    let (raw, pin) = package(true);
    assert_eq!(
        evaluate(&raw, pin, &pair_input(&keys, b"wrong"), &view, limits()),
        Err(ReadError::Obstructed {
            coordinate: "text.BasisNotCanonical".into(),
            cause: ReadObstruction::Guard
        })
    );
    assert_eq!(
        evaluate(&raw, pin, &pair_input(&keys, b"alpha"), &view, limits()),
        Err(ReadError::OutsideAperture(keys[1]))
    );
    assert_eq!(frontier.state().state_root(), before);
}

#[test]
fn aperture_checks_full_warp_and_node_identity_before_storage_lookup() {
    let (frontier, keys) = frontier();
    let aperture = [keys[0]];
    let view = ReadView::new(&frontier, ReadBasis::at(&frontier), &aperture).unwrap();
    let (raw, pin) = package(false);
    for key in [
        keys[1],
        NodeKey {
            warp_id: WarpId([9; 32]),
            ..keys[0]
        },
    ] {
        assert_eq!(
            evaluate(&raw, pin, &single_input(key, b"alpha"), &view, limits()),
            Err(ReadError::OutsideAperture(key))
        );
    }
    let empty = ReadView::new(&frontier, ReadBasis::at(&frontier), &[]).unwrap();
    assert_eq!(
        evaluate(
            &raw,
            pin,
            &single_input(keys[0], b"alpha"),
            &empty,
            limits()
        ),
        Err(ReadError::OutsideAperture(keys[0]))
    );
}

#[test]
fn view_refuses_wrong_worldline_tick_state_or_noncanonical_aperture() {
    let (frontier, keys) = frontier();
    let basis = ReadBasis::at(&frontier);
    let wrong_world =
        WorldlineFrontier::new(WorldlineId::from_bytes([8; 32]), frontier.state().clone());
    let wrong_tick = WorldlineFrontier::at_tick(
        frontier.worldline_id(),
        frontier.state().clone(),
        WorldlineTick::from_raw(1),
    );
    let (different, _) = with_attachment(Some(AttachmentValue::Atom(AtomPayload {
        type_id: make_type_id("document"),
        bytes: b"changed".to_vec().into(),
    })));
    for other in [&wrong_world, &wrong_tick, &different] {
        assert!(matches!(
            ReadView::new(other, basis, &[]),
            Err(ReadError::BasisMismatch)
        ));
    }
    assert!(matches!(
        ReadView::new(&frontier, basis, &[keys[0], keys[0]]),
        Err(ReadError::InvalidAperture)
    ));
    let mut unsorted = keys;
    unsorted.sort();
    unsorted.reverse();
    assert!(matches!(
        ReadView::new(&frontier, basis, &unsorted),
        Err(ReadError::InvalidAperture)
    ));
    assert!(matches!(
        ReadView::new(&frontier, basis, &vec![unsorted[0]; 65_537]),
        Err(ReadError::InvalidAperture)
    ));
}

fn with_attachment(attachment: Option<AttachmentValue>) -> (WorldlineFrontier, NodeKey) {
    let mut store = GraphStore::default();
    let node = make_node_id("first");
    store.insert_node(
        node,
        NodeRecord {
            ty: make_type_id("node"),
        },
    );
    store.set_node_attachment(node, attachment);
    let key = NodeKey {
        warp_id: store.warp_id(),
        local_id: node,
    };
    (
        WorldlineFrontier::new(
            WorldlineId::from_bytes([7; 32]),
            WorldlineState::from_root_store(store, node).unwrap(),
        ),
        key,
    )
}

#[test]
fn missing_atom_and_type_failures_map_to_authored_obstructions() {
    let (missing, key) = with_attachment(None);
    assert_eq!(
        single(&missing, key, b"", limits()),
        Err(ReadError::Obstructed {
            coordinate: "text.FactMissing".into(),
            cause: ReadObstruction::Missing
        })
    );
    let absent = NodeKey {
        local_id: make_node_id("absent"),
        ..key
    };
    assert_eq!(
        single(&missing, absent, b"", limits()),
        Err(ReadError::Obstructed {
            coordinate: "text.FactMissing".into(),
            cause: ReadObstruction::Missing
        })
    );
    let (wrong_type, key) = with_attachment(Some(AttachmentValue::Atom(AtomPayload {
        type_id: make_type_id("other"),
        bytes: b"alpha".to_vec().into(),
    })));
    assert_eq!(
        single(&wrong_type, key, b"alpha", limits()),
        Err(ReadError::Obstructed {
            coordinate: "text.FactMalformed".into(),
            cause: ReadObstruction::TypeMismatch
        })
    );
    let (large, key) = with_attachment(Some(AttachmentValue::Atom(AtomPayload {
        type_id: make_type_id("document"),
        bytes: vec![0; 1_048_577].into(),
    })));
    assert_eq!(
        single(&large, key, b"", limits()),
        Err(ReadError::Obstructed {
            coordinate: "text.FactMalformed".into(),
            cause: ReadObstruction::AtomTooLarge
        })
    );
}

#[test]
fn wrong_package_pin_and_malformed_input_refuse_before_reading() {
    let (frontier, keys) = frontier();
    let view = ReadView::new(&frontier, ReadBasis::at(&frontier), &[]).unwrap();
    let (raw, pin) = package(false);
    assert_eq!(
        evaluate(
            &raw,
            [0; 32],
            &single_input(keys[0], b"alpha"),
            &view,
            limits()
        ),
        Err(ReadError::Evaluation(
            EvaluationError::PackageIdentityMismatch
        ))
    );
    assert_eq!(
        evaluate(
            &raw,
            pin,
            &encode(&Value::Integer(1)).unwrap(),
            &view,
            limits()
        ),
        Err(ReadError::Evaluation(EvaluationError::InvalidInput))
    );
    let malformed = encode(&record([
        (
            "address",
            record([
                ("nodeId", Value::Bytes(vec![0; 31])),
                ("warpId", Value::Bytes(vec![0; 32])),
                ("typeId", Value::Bytes(vec![0; 32])),
            ]),
        ),
        ("expected", Value::Bytes(vec![])),
    ]))
    .unwrap();
    assert_eq!(
        evaluate(&raw, pin, &malformed, &view, limits()),
        Err(ReadError::Evaluation(EvaluationError::InvalidInput))
    );
    let mut bounded = limits();
    bounded.evaluation.max_package_bytes = raw.len() - 1;
    assert_eq!(
        evaluate(&raw, pin, &[], &view, bounded),
        Err(ReadError::Evaluation(EvaluationError::PackageTooLarge))
    );
    bounded = limits();
    bounded.evaluation.max_input_bytes = 0;
    assert_eq!(
        evaluate(&raw, pin, &[0], &view, bounded),
        Err(ReadError::Evaluation(EvaluationError::InputTooLarge))
    );
}

#[test]
fn a_valid_descent_is_refused_without_following_the_child() {
    use warp_core::{AttachmentKey, PortalInit, TickCommitStatus, WarpOp, WarpTickPatchV1};
    let (frontier, key) = with_attachment(None);
    let mut state = frontier.state().warp_state().clone();
    let patch = WarpTickPatchV1::new(
        0,
        [0; 32],
        TickCommitStatus::Committed,
        vec![],
        vec![],
        vec![WarpOp::OpenPortal {
            key: AttachmentKey::node_alpha(key),
            child_warp: WarpId([11; 32]),
            child_root: make_node_id("child"),
            init: PortalInit::Empty {
                root_record: NodeRecord {
                    ty: make_type_id("node"),
                },
            },
        }],
    );
    patch.apply_to_state(&mut state).unwrap();
    let state = WorldlineState::new(state, key).unwrap();
    let frontier = WorldlineFrontier::new(frontier.worldline_id(), state);
    let before = frontier.state().state_root();
    assert_eq!(
        single(&frontier, key, b"", limits()),
        Err(ReadError::Obstructed {
            coordinate: "text.FactMalformed".into(),
            cause: ReadObstruction::AtomRequired,
        })
    );
    assert_eq!(frontier.state().state_root(), before);
}

#[test]
fn empty_atoms_are_values_and_missing_warps_are_obstructions() {
    let (frontier, key) = with_attachment(Some(AttachmentValue::Atom(AtomPayload {
        type_id: make_type_id("document"),
        bytes: Vec::new().into(),
    })));
    let result = single(&frontier, key, b"", limits()).unwrap();
    assert_eq!(decode(&result.output).unwrap(), Value::Bytes(vec![]));
    assert_eq!(result.read_bytes, 0);
    assert_eq!(result.reads, 1);
    let absent = NodeKey {
        warp_id: WarpId([12; 32]),
        ..key
    };
    assert_eq!(
        single(&frontier, absent, b"", limits()),
        Err(ReadError::Obstructed {
            coordinate: "text.FactMissing".into(),
            cause: ReadObstruction::Missing,
        })
    );
}

fn field_mut<'a>(value: &'a mut Value, name: &str) -> &'a mut Value {
    let Value::Map(fields) = value else {
        panic!("map")
    };
    &mut fields
        .iter_mut()
        .find(|(key, _)| key == &Value::Text(name.into()))
        .unwrap()
        .1
}

#[test]
fn input_identity_survives_coherently_reordered_local_declarations() {
    let (frontier, keys) = frontier();
    let aperture = [keys[0]];
    let view = ReadView::new(&frontier, ReadBasis::at(&frontier), &aperture).unwrap();
    let (raw, pin) = package(false);
    let input = single_input(keys[0], b"alpha");
    let expected = evaluate(&raw, pin, &input, &view, limits()).unwrap();
    let mut package = decode(&raw).unwrap();
    let Value::Bytes(program_bytes) = field(&package, "program") else {
        panic!("program")
    };
    let mut program = decode(program_bytes).unwrap();
    let Value::Bytes(core_bytes) = field(&program, "core_artifact") else {
        panic!("Core")
    };
    let mut core = decode(core_bytes).unwrap();
    let body = field_mut(
        field_mut(field_mut(&mut core, "intents"), "replaceRange"),
        "body",
    );
    let Value::Array(locals) = field_mut(body, "locals") else {
        panic!("locals")
    };
    assert_eq!(field(&locals[0], "id"), &Value::Text("arg.0".into()));
    locals.rotate_left(1);
    assert_ne!(field(&locals[0], "id"), &Value::Text("arg.0".into()));
    let core_identity = digest("edict.core.module/v1", &core).unwrap();
    *field_mut(&mut program, "core_artifact") = Value::Bytes(encode(&core).unwrap());
    let Value::Bytes(target_bytes) = field(&program, "target_ir_artifact") else {
        panic!("Target")
    };
    let mut target = decode(target_bytes).unwrap();
    let source_core = field_mut(field_mut(&mut target, "semanticClosure"), "sourceCore");
    *field_mut(source_core, "digest") = Value::Array(vec![
        Value::Text("sha256".into()),
        Value::Bytes(core_identity.to_vec()),
    ]);
    let target_identity = digest("edict.target-ir.artifact/v1", &target).unwrap();
    *field_mut(&mut program, "target_ir_artifact") = Value::Bytes(encode(&target).unwrap());
    *field_mut(&mut package, "program") = Value::Bytes(encode(&program).unwrap());
    let closure = field_mut(&mut package, "semantic_closure");
    for key in ["core_identity", "canonical_meaning_identity"] {
        *field_mut(closure, key) = Value::Bytes(core_identity.to_vec());
    }
    *field_mut(closure, "target_ir_identity") = Value::Bytes(target_identity.to_vec());
    let pin = digest("echo.operation-package/v1", &package).unwrap();
    let actual = evaluate(&encode(&package).unwrap(), pin, &input, &view, limits());
    assert_eq!(actual, Ok(expected));
}
