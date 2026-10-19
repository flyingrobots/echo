// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Private authored instruction execution, without scheduler mutation authority.
use super::*;
use warp_core::edict_effects::{EffectFrameError, TypedNodeRead};
use warp_core::edict_pure::EvaluationLimits;
use warp_core::{
    make_head_id, AtomPayload, AttachmentValue, EchoOperationBudgetV1, EngineBuilder, GraphStore,
    InboxPolicy, NodeId, NodeKey, NodeRecord, PlaybackMode, TrustedRuntimeHost, WarpId,
    WorldlineId, WorldlineRuntime, WorldlineState, WriterHead, WriterHeadKey,
};
fn host() -> (TrustedRuntimeHost, WriterHeadKey, NodeKey) {
    let key = NodeKey {
        warp_id: WarpId([3; 32]),
        local_id: NodeId([4; 32]),
    };
    let mut store = GraphStore::new(key.warp_id);
    store.insert_node(
        key.local_id,
        NodeRecord {
            ty: TypeId([1; 32]),
        },
    );
    store.set_node_attachment(
        key.local_id,
        Some(AttachmentValue::Atom(AtomPayload {
            type_id: TypeId([2; 32]),
            bytes: b"before".to_vec().into(),
        })),
    );
    let state = WorldlineState::from_root_store(store.clone(), key.local_id).expect("state");
    let head = WriterHeadKey {
        worldline_id: WorldlineId::from_bytes([5; 32]),
        head_id: make_head_id("authored-execution"),
    };
    let mut runtime = WorldlineRuntime::new();
    runtime
        .register_worldline(head.worldline_id, state)
        .expect("worldline");
    runtime
        .register_writer_head(WriterHead::with_routing(
            head,
            PlaybackMode::Play,
            InboxPolicy::AcceptAll,
            None,
            true,
        ))
        .expect("head");
    let engine = EngineBuilder::new(store, key.local_id).build();
    (
        TrustedRuntimeHost::new(runtime, engine).expect("host"),
        head,
        key,
    )
}
fn input(node: NodeKey, expected: &[u8]) -> Vec<u8> {
    encoded(&map([
        (
            "address",
            map([
                ("nodeId", V::Bytes(node.local_id.0.to_vec())),
                ("warpId", V::Bytes(node.warp_id.0.to_vec())),
                ("typeId", id(2)),
            ]),
        ),
        ("expected", V::Bytes(expected.to_vec())),
    ]))
}
fn publication() -> V {
    let mut publication = publication_for_lowering(&repinned_lowering_artifacts());
    let V::Bytes(raw) = get(&publication, "program") else {
        panic!("program")
    };
    let mut program = echo_edict_canonical::decode_canonical_cbor_v1(raw).expect("program");
    let mut permissions = law();
    // A real typed atom read resolves both a node slot and its attachment slot.
    set(&mut permissions, "max_reads", V::Integer(2));
    set(
        &mut program,
        "footprint_law",
        V::Bytes(encoded(&permissions)),
    );
    set(&mut publication, "program", V::Bytes(encoded(&program)));
    set(
        &mut publication,
        "footprint_contract_identity",
        footprint(&permissions),
    );
    publication
}
fn limits() -> EvaluationLimits {
    EvaluationLimits {
        max_package_bytes: 1_048_576,
        max_input_bytes: 65536,
        max_steps: 10000,
        max_allocated_bytes: 1_048_576,
        max_output_bytes: 65536,
    }
}
#[test]
fn authored_instruction_execution_reads_the_real_atom_and_records_dependencies() {
    // Returning an input field or skipping the read/require must fail this witness.
    let publication = publication();
    let package = prepare(&publication).expect("synthetic bound program");
    let program = package.program().as_effect_program().expect("effect");
    let (host, head, node) = host();
    let basis = host
        .echo_operation_evaluation_basis_v1(
            head,
            warp_core::EchoOperationApplicationBasisV1::new([6; 32], [7; 32]),
        )
        .expect("basis");
    let mut frame = host
        .effect_evaluation_frame_v1(
            basis,
            &[],
            0,
            EchoOperationBudgetV1::new(10000, 4096, 0),
            limits(),
        )
        .expect("frame")
        .with_footprint_law(program.footprint_law().clone())
        .expect("law");
    let result = frame
        .evaluate_effect(
            program,
            &input(node, b"before"),
            &[TypedNodeRead {
                node,
                absent_node_type: TypeId([1; 32]),
            }],
        )
        .expect("authored evaluation");
    assert_eq!(result, encoded(&V::Bytes(b"before".to_vec())));
    assert_eq!(frame.readings().len(), 2);
    assert!(frame.footprint().n_read.iter().any(|key| *key == node));
    assert!(frame.consumed().steps() > 2);
    assert!(frame.consumed().read_bytes() >= 134);
}
#[test]
fn authored_require_uses_the_real_read_and_ends_the_frame_on_failure() {
    let package = prepare(&publication()).expect("program");
    let program = package.program().as_effect_program().expect("effect");
    let (host, head, node) = host();
    let basis = host
        .echo_operation_evaluation_basis_v1(
            head,
            warp_core::EchoOperationApplicationBasisV1::new([6; 32], [7; 32]),
        )
        .expect("basis");
    let mut frame = host
        .effect_evaluation_frame_v1(
            basis,
            &[],
            0,
            EchoOperationBudgetV1::new(10000, 4096, 0),
            limits(),
        )
        .expect("frame")
        .with_footprint_law(program.footprint_law().clone())
        .expect("law");
    let aperture = [TypedNodeRead {
        node,
        absent_node_type: TypeId([1; 32]),
    }];
    let error = frame
        .evaluate_effect(program, &input(node, b"different"), &aperture)
        .expect_err("require must run");
    assert!(matches!(
        error,
        EffectFrameError::AuthoredObstruction { .. }
    ));
    assert_eq!(frame.readings().len(), 2);
    let consumed = frame.consumed();
    assert_eq!(
        frame.evaluate_effect(program, &input(node, b"before"), &aperture),
        Err(EffectFrameError::Terminated)
    );
    assert_eq!(frame.consumed(), consumed);
}
#[test]
fn authored_execution_refuses_unbound_frames_and_mismatched_laws_before_reads() {
    use warp_core::edict_effects::{
        EffectEvaluationFrame, EffectFootprintLaw, EffectRangeReadError, EffectRangeReadSession,
    };
    let package = prepare(&publication()).expect("program");
    let program = package.program().as_effect_program().expect("effect");
    let (host, head, node) = host();
    let frontier = host
        .runtime()
        .worldlines()
        .get(&head.worldline_id)
        .expect("frontier");
    let mut unbound = EffectEvaluationFrame::new(
        EffectRangeReadSession::new(frontier, &[], 0, EchoOperationBudgetV1::new(10000, 4096, 0)),
        limits(),
    )
    .expect("unbound");
    assert_eq!(
        unbound.evaluate_effect(program, &input(node, b"before"), &[]),
        Err(EffectFrameError::UnboundParent)
    );
    assert!(unbound.readings().is_empty());
    let basis = host
        .echo_operation_evaluation_basis_v1(
            head,
            warp_core::EchoOperationApplicationBasisV1::new([6; 32], [7; 32]),
        )
        .expect("basis");
    let mut frame = host
        .effect_evaluation_frame_v1(
            basis,
            &[],
            0,
            EchoOperationBudgetV1::new(10000, 4096, 0),
            limits(),
        )
        .expect("frame")
        .with_footprint_law(
            EffectFootprintLaw::from_canonical_bytes(&encoded(&law())).expect("different law"),
        )
        .expect("bind");
    assert_eq!(
        frame.evaluate_effect(program, &input(node, b"before"), &[]),
        Err(EffectFrameError::Read(
            EffectRangeReadError::FootprintLawViolation
        ))
    );
    assert!(frame.readings().is_empty());
    assert_eq!(frame.consumed(), EchoOperationBudgetV1::new(0, 0, 0));
}
#[test]
fn authored_execution_enforces_aperture_and_shared_resource_limits() {
    use warp_core::edict_pure::EvaluationError;
    use warp_core::edict_read::ReadError;
    let package = prepare(&publication()).expect("program");
    let program = package.program().as_effect_program().expect("effect");
    let (host, head, node) = host();
    let basis = host
        .echo_operation_evaluation_basis_v1(
            head,
            warp_core::EchoOperationApplicationBasisV1::new([6; 32], [7; 32]),
        )
        .expect("basis");
    for (aperture, cap, want) in [
        (
            vec![],
            10000,
            EffectFrameError::BoundedRead(ReadError::OutsideAperture(node)),
        ),
        (
            vec![TypedNodeRead {
                node,
                absent_node_type: TypeId([1; 32]),
            }],
            1,
            EffectFrameError::BoundedRead(ReadError::Evaluation(
                EvaluationError::StepBudgetExceeded,
            )),
        ),
    ] {
        let mut frame = host
            .effect_evaluation_frame_v1(
                basis,
                &[],
                0,
                EchoOperationBudgetV1::new(cap, 4096, 0),
                limits(),
            )
            .expect("frame")
            .with_footprint_law(program.footprint_law().clone())
            .expect("law");
        assert_eq!(
            frame.evaluate_effect(program, &input(node, b"before"), &aperture),
            Err(want)
        );
        assert!(frame.readings().is_empty());
        assert!(frame.consumed().steps() <= cap);
        assert_eq!(
            frame.evaluate_effect(program, &input(node, b"before"), &aperture),
            Err(EffectFrameError::Terminated)
        );
    }
}
#[test]
fn authored_absence_maps_the_declared_failure_without_returning_a_value() {
    let package = prepare(&publication()).expect("program");
    let program = package.program().as_effect_program().expect("effect");
    let (host, head, node) = host();
    let missing = NodeKey {
        local_id: NodeId([9; 32]),
        ..node
    };
    let basis = host
        .echo_operation_evaluation_basis_v1(
            head,
            warp_core::EchoOperationApplicationBasisV1::new([6; 32], [7; 32]),
        )
        .expect("basis");
    let mut frame = host
        .effect_evaluation_frame_v1(
            basis,
            &[],
            0,
            EchoOperationBudgetV1::new(10000, 4096, 0),
            limits(),
        )
        .expect("frame")
        .with_footprint_law(program.footprint_law().clone())
        .expect("law");
    assert_eq!(
        frame.evaluate_effect(
            program,
            &input(missing, b"before"),
            &[TypedNodeRead {
                node: missing,
                absent_node_type: TypeId([1; 32])
            }]
        ),
        Err(EffectFrameError::AuthoredObstruction {
            reason_kind: "text.FactMissing".into(),
            payload: encoded(&map([]))
        })
    );
    assert_eq!(frame.readings().len(), 1);
    assert!(frame.footprint().n_read.iter().any(|key| *key == missing));
}
#[test]
fn authored_execution_runs_ordered_pure_bindings_under_the_same_program_model() {
    let mut core = compiled_core();
    set(
        value_field_mut(
            value_field_mut(value_field_mut(&mut core, "intents"), "update@1"),
            "coreEvaluationBudget",
        ),
        "maxSteps",
        V::Integer(1000),
    );
    let binding = map([
        ("id", text("local.0")),
        ("type", text("U32")),
        ("alphaName", text("$value")),
    ]);
    let value = map([
        ("kind", text("const")),
        (
            "value",
            map([
                ("kind", text("int")),
                ("width", text("U32")),
                ("value", V::Integer(7)),
            ]),
        ),
    ]);
    let body = core_body_mut(&mut core);
    let V::Array(locals) = value_field_mut(body, "locals") else {
        panic!("locals")
    };
    locals.push(binding.clone());
    set(
        body,
        "nodes",
        V::Array(vec![map([
            ("kind", text("let")),
            ("binding", binding.clone()),
            ("value", value.clone()),
        ])]),
    );
    set(
        body,
        "result",
        map([("kind", text("local")), ("ref", binding.clone())]),
    );
    let mut target = compiled_target(&core);
    let selected = selected_target_mut(&mut target);
    set(
        selected,
        "pureBindings",
        V::Array(vec![map([
            ("id", text("binding.0")),
            ("binding", binding),
            ("value", value),
        ])]),
    );
    set(
        selected,
        "executionOrder",
        V::Array(vec![text("binding.0")]),
    );
    let package = prepare(&bound_compiler_package(
        &core,
        &target,
        &compiled_exports(),
        "update@1",
    ))
    .expect("pure bindings");
    let program = package.program().as_effect_program().expect("program");
    let (host, head, _) = host();
    let basis = host
        .echo_operation_evaluation_basis_v1(
            head,
            warp_core::EchoOperationApplicationBasisV1::new([6; 32], [7; 32]),
        )
        .expect("basis");
    let mut frame = host
        .effect_evaluation_frame_v1(
            basis,
            &[],
            0,
            EchoOperationBudgetV1::new(10000, 4096, 0),
            limits(),
        )
        .expect("frame")
        .with_footprint_law(program.footprint_law().clone())
        .expect("law");
    assert_eq!(
        frame
            .evaluate_effect(program, &encoded(&V::Integer(123)), &[])
            .expect("let/result"),
        encoded(&V::Integer(7))
    );
    assert!(frame.readings().is_empty());
}
#[test]
fn authored_reads_enforce_the_prepared_graph_schema() {
    let mut publication = publication();
    let V::Bytes(raw) = get(&publication, "program") else {
        panic!("program")
    };
    let mut program = echo_edict_canonical::decode_canonical_cbor_v1(raw).expect("program");
    let schema = map([
        (
            "node_types",
            V::Map(vec![(
                id(1),
                map([
                    ("addressing", V::Integer(1)),
                    ("atom_types", V::Array(vec![])),
                ]),
            )]),
        ),
        ("edge_types", V::Map(vec![])),
    ]);
    set(&mut program, "graph_schema", V::Bytes(encoded(&schema)));
    set(&mut publication, "program", V::Bytes(encoded(&program)));
    let package = prepare(&publication).expect("schema");
    let program = package.program().as_effect_program().expect("program");
    let (host, head, node) = host();
    let basis = host
        .echo_operation_evaluation_basis_v1(
            head,
            warp_core::EchoOperationApplicationBasisV1::new([6; 32], [7; 32]),
        )
        .expect("basis");
    let mut frame = host
        .effect_evaluation_frame_v1(
            basis,
            &[],
            0,
            EchoOperationBudgetV1::new(10000, 4096, 0),
            limits(),
        )
        .expect("frame")
        .with_footprint_law(program.footprint_law().clone())
        .expect("law");
    assert_eq!(
        frame.evaluate_effect(
            program,
            &input(node, b"before"),
            &[TypedNodeRead {
                node,
                absent_node_type: TypeId([1; 32])
            }]
        ),
        Err(EffectFrameError::GraphSchemaViolation)
    );
    assert_eq!(frame.readings().len(), 2);
}
