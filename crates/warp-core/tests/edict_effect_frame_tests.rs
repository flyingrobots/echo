// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! One resource envelope across real compiled expressions and graph reads.
#![cfg(feature = "trusted_runtime")]
#![allow(clippy::expect_used)]
// Coherent budget mutants test runtime defense, not verifier approval.
#[allow(dead_code, clippy::unwrap_used, clippy::panic)]
#[path = "support/edict_byte_concat.rs"]
mod concat;

use echo_edict_canonical::{encode_canonical_cbor_v1 as encode, CanonicalValueV1 as Value};
use warp_core::edict_effects::{
    EffectEvaluationFrame, EffectFrameError, EffectRangeReadError, EffectRangeReadSession,
};
use warp_core::edict_pure::{evaluate, EvaluationError, EvaluationLimits};
use warp_core::{
    make_node_id, make_type_id, EchoOperationBudgetV1, GraphStore, NodeKey, NodeRecord,
    OutgoingEdgeRange, WorldlineFrontier, WorldlineId, WorldlineState,
};

fn package() -> Vec<u8> {
    hex::decode(
        include_str!("fixtures/edict-byte-concat/executable-operation-package.cbor.hex").trim(),
    )
    .expect("retained package")
}
fn pin() -> [u8; 32] {
    hex::decode("ed480a332a9d5f84f7425e46ab0b08cca886c8a040e048145b88fba1e6e0415f")
        .expect("pin")
        .try_into()
        .expect("digest")
}
fn input() -> Vec<u8> {
    encode(&Value::Map(vec![
        (
            Value::Text("firstFragment".into()),
            Value::Bytes(vec![b'a']),
        ),
        (
            Value::Text("secondFragment".into()),
            Value::Bytes(vec![b'b']),
        ),
    ]))
    .expect("input")
}
fn expected() -> Vec<u8> {
    encode(&Value::Map(vec![(
        Value::Text("bytes".into()),
        Value::Bytes(b"ab".to_vec()),
    )]))
    .expect("literal result")
}
fn limits() -> EvaluationLimits {
    EvaluationLimits {
        max_package_bytes: 32768,
        max_input_bytes: 1024,
        max_steps: 10000,
        max_allocated_bytes: 16777216,
        max_output_bytes: 1024,
    }
}
fn fixture() -> (WorldlineFrontier, OutgoingEdgeRange) {
    let mut store = GraphStore::default();
    let root = make_node_id("frame/root");
    store.insert_node(
        root,
        NodeRecord {
            ty: make_type_id("node"),
        },
    );
    let key = OutgoingEdgeRange {
        source: NodeKey {
            warp_id: store.warp_id(),
            local_id: root,
        },
        edge_type: make_type_id("edge"),
    };
    let state = WorldlineState::from_root_store(store, root).expect("state");
    (
        WorldlineFrontier::new(WorldlineId::from_bytes([7; 32]), state),
        key,
    )
}
fn session(frontier: &WorldlineFrontier, key: OutgoingEdgeRange) -> EffectRangeReadSession<'_> {
    EffectRangeReadSession::new(
        frontier,
        &[key.edge_type],
        0,
        EchoOperationBudgetV1::new(10000, 1024, 0),
    )
}

#[test]
fn read_then_real_compiler_package_spend_one_allocation_limit() {
    let control = evaluate(&package(), pin(), &input(), limits()).expect("standalone control");
    assert_eq!(control.output, expected());
    let (frontier, key) = fixture();
    for (extra, succeeds) in [(255, false), (256, true)] {
        let mut frame = EffectEvaluationFrame::new(
            session(&frontier, key),
            EvaluationLimits {
                max_allocated_bytes: control.allocated_bytes + extra,
                ..limits()
            },
        )
        .expect("frame");
        assert_eq!(frame.read_range(key).expect("read"), Some(Vec::new()));
        match frame.evaluate_pure(&package(), pin(), &input()) {
            Ok(output) => {
                assert!(succeeds);
                assert_eq!(output, expected());
                assert_eq!(frame.allocated_bytes(), control.allocated_bytes + 256);
            }
            Err(error) => {
                assert!(!succeeds);
                assert_eq!(
                    error,
                    EffectFrameError::Evaluation(EvaluationError::AllocationBudgetExceeded)
                );
            }
        }
        assert!(frame.allocated_bytes() <= control.allocated_bytes + extra);
    }
}

#[test]
fn real_compiler_package_then_read_uses_the_same_remaining_storage() {
    let control = evaluate(&package(), pin(), &input(), limits()).expect("control");
    let (frontier, key) = fixture();
    let mut frame = EffectEvaluationFrame::new(
        session(&frontier, key),
        EvaluationLimits {
            max_allocated_bytes: control.allocated_bytes + 255,
            ..limits()
        },
    )
    .expect("frame");
    assert_eq!(
        frame
            .evaluate_pure(&package(), pin(), &input())
            .expect("pure"),
        expected()
    );
    assert_eq!(
        frame.read_range(key),
        Err(EffectFrameError::Read(EffectRangeReadError::BudgetExceeded))
    );
    assert_eq!(frame.allocated_bytes(), control.allocated_bytes + 128);
    assert_eq!(frame.readings().len(), 1);
    assert_eq!(frame.read_range(key), Err(EffectFrameError::Terminated));
}

#[test]
fn expressions_and_reads_share_step_capacity_too() {
    let control = evaluate(&package(), pin(), &input(), limits()).expect("control");
    let (frontier, key) = fixture();
    let mut frame = EffectEvaluationFrame::new(
        session(&frontier, key),
        EvaluationLimits {
            max_steps: control.steps,
            ..limits()
        },
    )
    .expect("frame");
    frame.read_range(key).expect("read");
    assert_eq!(
        frame.evaluate_pure(&package(), pin(), &input()),
        Err(EffectFrameError::Evaluation(
            EvaluationError::StepBudgetExceeded
        ))
    );
    assert!(frame.consumed().steps() <= control.steps);
    assert_eq!(frame.consumed().read_bytes(), 64);
    assert_eq!(
        frame.evaluate_pure(&package(), pin(), &input()),
        Err(EffectFrameError::Terminated)
    );
}

#[test]
fn another_package_evaluation_cannot_reset_the_frame_budget() {
    let control = evaluate(&package(), pin(), &input(), limits()).expect("control");
    let (frontier, key) = fixture();
    let mut frame = EffectEvaluationFrame::new(
        session(&frontier, key),
        EvaluationLimits {
            max_allocated_bytes: 2 * control.allocated_bytes - 1,
            ..limits()
        },
    )
    .expect("frame");
    assert_eq!(
        frame
            .evaluate_pure(&package(), pin(), &input())
            .expect("first"),
        expected()
    );
    assert_eq!(
        frame.evaluate_pure(&package(), pin(), &input()),
        Err(EffectFrameError::Evaluation(
            EvaluationError::AllocationBudgetExceeded
        ))
    );
    assert!(frame.allocated_bytes() < 2 * control.allocated_bytes);
}

#[test]
fn a_failed_package_identity_ends_the_frame_before_any_graph_read() {
    let (frontier, key) = fixture();
    let mut frame = EffectEvaluationFrame::new(session(&frontier, key), limits()).expect("frame");
    assert_eq!(
        frame.evaluate_pure(&package(), [0; 32], &input()),
        Err(EffectFrameError::Evaluation(
            EvaluationError::PackageIdentityMismatch
        ))
    );
    assert_eq!(frame.read_range(key), Err(EffectFrameError::Terminated));
    assert_eq!(frame.allocated_bytes(), 0);
    assert_eq!(frame.consumed(), EchoOperationBudgetV1::new(0, 0, 0));
    assert!(frame.readings().is_empty());
}

#[test]
fn preexisting_reads_cannot_be_reintroduced_as_an_unspent_frame() {
    let (frontier, key) = fixture();
    let mut reads = session(&frontier, key);
    reads.read(key).expect("preexisting read");
    assert!(matches!(
        EffectEvaluationFrame::new(reads, limits()),
        Err(warp_core::edict_effects::EffectFrameSetupError::ReadSessionAlreadyUsed)
    ));
}

#[test]
fn source_session_ceilings_bind_computation_as_well_as_reads() {
    let (frontier, key) = fixture();
    let step_limited = EffectRangeReadSession::new(
        &frontier,
        &[key.edge_type],
        0,
        EchoOperationBudgetV1::new(1, 1024, 0),
    );
    let mut frame = EffectEvaluationFrame::new(step_limited, limits()).expect("frame");
    assert_eq!(
        frame.evaluate_pure(&package(), pin(), &input()),
        Err(EffectFrameError::Evaluation(
            EvaluationError::StepBudgetExceeded
        ))
    );
    assert!(frame.consumed().steps() <= 1);
    let storage_limited = EffectRangeReadSession::with_allocation_limit(
        &frontier,
        &[key.edge_type],
        0,
        EchoOperationBudgetV1::new(10000, 1024, 0),
        127,
    )
    .expect("read view");
    let mut frame = EffectEvaluationFrame::new(storage_limited, limits()).expect("frame");
    assert_eq!(
        frame.evaluate_pure(&package(), pin(), &input()),
        Err(EffectFrameError::Evaluation(
            EvaluationError::AllocationBudgetExceeded
        ))
    );
    assert!(frame.allocated_bytes() <= 127);
}

#[test]
fn the_fixed_effect_ceilings_are_checked_before_a_frame_starts() {
    let (frontier, key) = fixture();
    for budget in [
        EchoOperationBudgetV1::new(1_048_577, 1024, 0),
        EchoOperationBudgetV1::new(10000, 67_108_865, 0),
        EchoOperationBudgetV1::new(10000, 1024, 16_777_217),
    ] {
        let reads = EffectRangeReadSession::new(&frontier, &[key.edge_type], 0, budget);
        assert!(matches!(
            EffectEvaluationFrame::new(reads, limits()),
            Err(warp_core::edict_effects::EffectFrameSetupError::ResourceCeilingExceeded)
        ));
    }
    assert!(matches!(
        EffectEvaluationFrame::new(
            session(&frontier, key),
            EvaluationLimits {
                max_allocated_bytes: 16_777_217,
                ..limits()
            }
        ),
        Err(warp_core::edict_effects::EffectFrameSetupError::ResourceCeilingExceeded)
    ));
    let exact = EffectRangeReadSession::new(
        &frontier,
        &[key.edge_type],
        0,
        EchoOperationBudgetV1::new(1_048_576, 67_108_864, 16_777_216),
    );
    let mut frame = EffectEvaluationFrame::new(exact, limits()).expect("exact ceilings");
    assert_eq!(
        frame
            .evaluate_pure(&package(), pin(), &input())
            .expect("computation"),
        expected()
    );
}

#[test]
fn each_component_keeps_its_declared_cap_inside_a_larger_frame() {
    let control = evaluate(&package(), pin(), &input(), limits()).expect("control");
    let (frontier, key) = fixture();
    for (package_key, core_key, amount, expected_error) in [
        (
            "max_steps",
            "maxSteps",
            control.steps - 1,
            EvaluationError::StepBudgetExceeded,
        ),
        (
            "max_allocated_bytes",
            "maxAllocatedBytes",
            control.allocated_bytes - 1,
            EvaluationError::AllocationBudgetExceeded,
        ),
    ] {
        let (limited, test_pin) = concat::budget(package_key, core_key, amount);
        let mut frame =
            EffectEvaluationFrame::new(session(&frontier, key), limits()).expect("frame");
        assert_eq!(
            frame.evaluate_pure(&limited, test_pin, &input()),
            Err(EffectFrameError::Evaluation(expected_error))
        );
    }
}

#[test]
fn component_caps_are_relative_and_the_global_cap_is_restored_between_calls() {
    let control = evaluate(&package(), pin(), &input(), limits()).expect("control");
    let (exact, test_pin) = concat::budget(
        "max_allocated_bytes",
        "maxAllocatedBytes",
        control.allocated_bytes,
    );
    let (frontier, key) = fixture();
    let mut frame = EffectEvaluationFrame::new(session(&frontier, key), limits()).expect("frame");
    for count in 1..=2 {
        assert_eq!(
            frame
                .evaluate_pure(&exact, test_pin, &input())
                .expect("component"),
            expected()
        );
        assert_eq!(frame.allocated_bytes(), count * control.allocated_bytes);
    }
}

#[test]
fn a_low_level_frame_does_not_claim_a_runtime_parent() {
    let (frontier, key) = fixture();
    let frame =
        EffectEvaluationFrame::new(session(&frontier, key), limits()).expect("private frame");
    assert_eq!(frame.evaluation_basis(), None);
}
