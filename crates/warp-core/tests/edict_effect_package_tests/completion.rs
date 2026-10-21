// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Whole authored-operation origin, distinct from private component composition.
use super::execution::{host, limits};
use super::mutations::fixture;
use super::*;
use warp_core::{EchoOperationApplicationBasisV1, EchoOperationBudgetV1};
fn basis(input: &[u8]) -> EchoOperationApplicationBasisV1 {
    EchoOperationApplicationBasisV1::new(
        hash(
            b"echo:operation-profile:v1\0",
            b"echo.operation.basis.edict-effect/v1",
        ),
        hash(b"echo:operation-edict-effect-basis:v1\0", input),
    )
}
#[test]
fn authored_operation_cannot_adopt_work_from_another_private_component() {
    let (publication, input) = fixture("create-node", "continuum.profile.graph-effects/v1");
    let package = prepare(&publication).expect("program");
    let program = package.program().as_effect_program().expect("effect");
    let (host, head, node) = host();
    let parent = host
        .echo_operation_evaluation_basis_v1(head, basis(&input))
        .expect("parent");
    let mut frame = host
        .effect_evaluation_frame_v1(
            parent,
            &[],
            0,
            EchoOperationBudgetV1::new(10000, 4096, 4096),
            limits(),
        )
        .expect("frame")
        .with_footprint_law(program.footprint_law().clone())
        .expect("law");
    frame
        .read_typed_node(node, TypeId([1; 32]))
        .expect("prior component read");
    let usage = frame.consumed();
    assert_eq!(
        frame.evaluate_effect(program, &input, &[]),
        Err(warp_core::edict_effects::EffectFrameError::UsedFrame)
    );
    assert_eq!(frame.consumed(), usage);
    assert!(frame.buffered_mutations().is_empty());
}
#[test]
fn completed_authored_evidence_binds_the_exact_program_input_parent_and_output() {
    use warp_core::edict_effects::EffectFrameError;
    for name in [
        "ensure-node",
        "create-node",
        "replace-atom",
        "ensure-edge",
        "replace-edge",
    ] {
        let (publication, input) = fixture(name, "continuum.profile.graph-effects/v1");
        let package = prepare(&publication).expect("program");
        let program = package.program().as_effect_program().expect("effect");
        let V::Bytes(raw_program) = get(&publication, "program") else {
            panic!("program")
        };
        let expected_id = hash(b"echo:operation-program:v1\0", raw_program);
        assert_eq!(
            package.program().identity().expect("identity").as_hash(),
            expected_id
        );
        let (host, head, _) = host();
        let parent = host
            .echo_operation_evaluation_basis_v1(head, basis(&input))
            .expect("parent");
        let mut frame = host
            .effect_evaluation_frame_v1(
                parent,
                &[],
                0,
                EchoOperationBudgetV1::new(10000, 4096, 4096),
                limits(),
            )
            .expect("frame")
            .with_footprint_law(program.footprint_law().clone())
            .expect("law");
        assert_eq!(
            frame
                .evaluate_effect(program, &input, &[])
                .expect("execute"),
            encoded(&map([]))
        );
        let usage = frame.consumed();
        let allocated = frame.allocated_bytes();
        assert_eq!(
            frame.evaluate_effect(program, &input, &[]),
            Err(EffectFrameError::Terminated)
        );
        assert_eq!(frame.consumed(), usage);
        let completed = frame
            .into_effect_evaluation(1024)
            .expect("completed evidence");
        assert_eq!(completed.program_identity(), expected_id);
        assert_eq!(completed.input_basis(), basis(&input));
        assert_eq!(completed.evaluation_basis(), parent);
        assert_eq!(completed.output(), encoded(&map([])));
        assert_eq!(completed.mutations().len(), 1);
        assert_eq!(completed.read_set_image().as_bytes(), &[0; 16]);
        assert_eq!(completed.read_set_image().consumed(), usage);
        assert_eq!(completed.read_set_image().allocated_bytes(), allocated);
    }
}
#[test]
fn input_substitution_cannot_be_sealed_under_the_original_application_basis() {
    use warp_core::edict_effects::EffectCompletionError;
    let (publication, input) = fixture("create-node", "continuum.profile.graph-effects/v1");
    let package = prepare(&publication).expect("program");
    let program = package.program().as_effect_program().expect("effect");
    let mut changed = echo_edict_canonical::decode_canonical_cbor_v1(&input).expect("input");
    set(&mut changed, "nodeId", id(99));
    let changed = encoded(&changed);
    let (host, head, _) = host();
    let parent = host
        .echo_operation_evaluation_basis_v1(head, basis(&input))
        .expect("original basis");
    let mut frame = host
        .effect_evaluation_frame_v1(
            parent,
            &[],
            0,
            EchoOperationBudgetV1::new(10000, 4096, 4096),
            limits(),
        )
        .expect("frame")
        .with_footprint_law(program.footprint_law().clone())
        .expect("law");
    frame
        .evaluate_effect(program, &changed, &[])
        .expect("valid private computation");
    assert!(matches!(
        frame.into_effect_evaluation(1024),
        Err(EffectCompletionError::InputBasisMismatch)
    ));
}
#[test]
fn unused_failed_and_export_limited_frames_cannot_deliver_completed_evidence() {
    use warp_core::edict_effects::{EffectCompletionError, EffectReadImageError};
    let (publication, input) = fixture("create-node", "continuum.profile.graph-effects/v1");
    let package = prepare(&publication).expect("program");
    let program = package.program().as_effect_program().expect("effect");
    let (host, head, _) = host();
    let parent = host
        .echo_operation_evaluation_basis_v1(head, basis(&input))
        .expect("basis");
    for mode in 0..3 {
        let mut frame = host
            .effect_evaluation_frame_v1(
                parent,
                &[],
                0,
                EchoOperationBudgetV1::new(10000, 4096, 4096),
                limits(),
            )
            .expect("frame")
            .with_footprint_law(program.footprint_law().clone())
            .expect("law");
        let expected = match mode {
            0 => EffectCompletionError::NoCompletedOperation,
            1 => {
                assert!(frame.evaluate_effect(program, &[0xff], &[]).is_err());
                EffectCompletionError::NoCompletedOperation
            }
            _ => {
                frame
                    .evaluate_effect(program, &input, &[])
                    .expect("completed");
                EffectCompletionError::ReadImage(EffectReadImageError::ExportLimitExceeded)
            }
        };
        assert!(matches!(frame.into_effect_evaluation(0),Err(error) if error==expected));
    }
}
#[test]
fn completed_authored_read_retains_the_actual_read_image_and_result() {
    use warp_core::edict_effects::{EffectReadSetView, TypedNodeRead};
    let package = prepare(&super::execution::publication()).expect("read program");
    let program = package.program().as_effect_program().expect("effect");
    let (host, head, node) = host();
    let input = super::execution::input(node, b"before");
    let parent = host
        .echo_operation_evaluation_basis_v1(head, basis(&input))
        .expect("basis");
    let mut frame = host
        .effect_evaluation_frame_v1(
            parent,
            &[],
            0,
            EchoOperationBudgetV1::new(10000, 4096, 4096),
            limits(),
        )
        .expect("frame")
        .with_footprint_law(program.footprint_law().clone())
        .expect("law");
    frame
        .evaluate_effect(
            program,
            &input,
            &[TypedNodeRead {
                node,
                absent_node_type: TypeId([1; 32]),
            }],
        )
        .expect("read");
    let completed = frame.into_effect_evaluation(4096).expect("completed read");
    assert_eq!(completed.output(), encoded(&V::Bytes(b"before".to_vec())));
    assert_eq!(completed.evaluation_basis(), parent);
    assert!(completed.mutations().is_empty());
    let reads =
        EffectReadSetView::parse(completed.read_set_image().as_bytes()).expect("read image");
    assert_eq!(reads.rows().len(), 2);
    assert_eq!(reads.rows()[0].slot.class, 0);
    assert_eq!(reads.rows()[0].slot.warp_id, node.warp_id.0);
    assert_eq!(reads.rows()[0].slot.local_id, node.local_id.0);
    assert_eq!(reads.rows()[0].read_type, [1; 32]);
    assert_eq!(reads.rows()[1].slot.class, 1);
    assert_eq!(reads.rows()[1].read_type, [2; 32]);
    assert_eq!(completed.read_set_image().consumed().read_bytes(), 134);
}
