// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Public compiler output for an authored bounded loop with both statement
//! branch arms. The oracle is the source's exact mutation sequence, before
//! proposal validation or application to the graph.
use super::current_effect_publication_fixture as publication;
use super::public_read_effect::{basis, host, limits};
use echo_edict_canonical::{decode_canonical_cbor_v1 as decode, CanonicalValueV1 as Value};
use warp_core::edict_effects::{EffectCompletionError, EffectFrameError, EffectMutation};
use warp_core::edict_pure::EvaluationError;
use warp_core::{
    AtomPayload, EchoOperationAdmissionErrorKind, EchoOperationAdmissionPolicy,
    EchoOperationBudgetV1, NodeId, NodeKey, TypeId, WarpId,
};

const PACKAGE: &str =
    include_str!("../fixtures/edict-effect-structured/structured-executable-operation-package.hex");
/// `echo:operation-package:v1` identity of the retained package bytes.
const PACKAGE_PIN: &str = "4c85550661f0ab6071e62bc7eab33bf5adf24d096554ad0f2e229477a7e6cb1b";
const OPERATION: &str = "consumer.structured.program@1.structured";
const AUTHORITY: &[u8] = b"consumer.structured.authority/v1";
const NODE_TYPE: TypeId = TypeId([0x5b; 32]);
const ATOM_TYPE: TypeId = TypeId([0x5c; 32]);

fn embedded(value: &Value, key: &str) -> Value {
    let Value::Bytes(bytes) = publication::get(value, key) else {
        panic!("embedded public artifact bytes")
    };
    decode(bytes).expect("canonical public artifact")
}

fn retained() -> (Vec<u8>, [u8; 32]) {
    let bytes = hex::decode(PACKAGE.trim()).expect("retained public hex");
    let package = decode(&bytes).expect("public package");
    let program = embedded(&package, "program");
    let bundle = embedded(&program, "compiled_program");
    let target = embedded(&bundle, "target_ir_artifact");
    assert_eq!(
        publication::get(&target, "domain"),
        &Value::Text("echo.span-ir/v3".to_owned()),
        "the source witness was compiled to v3"
    );
    assert_eq!(
        publication::get(&target, "kind"),
        &Value::Text("structuredTargetIrArtifact".to_owned())
    );
    let identity = publication::hash(b"echo:operation-package:v1\0", &bytes);
    assert_eq!(
        hex::encode(identity),
        PACKAGE_PIN,
        "retained package identity"
    );
    (bytes, identity)
}

fn policy(identity: [u8; 32]) -> EchoOperationAdmissionPolicy {
    EchoOperationAdmissionPolicy::exact(
        identity,
        OPERATION,
        publication::hash(b"echo:operation-profile:v1\0", AUTHORITY),
        EchoOperationBudgetV1::new(4096, 4096, 4096),
    )
}

fn input(warp: WarpId, branches: &[bool]) -> Vec<u8> {
    let node = |create: bool| {
        publication::map([
            ("warpId", Value::Bytes(warp.0.to_vec())),
            ("nodeId", publication::id(if create { 0x42 } else { 0x41 })),
            ("nodeType", Value::Bytes(NODE_TYPE.0.to_vec())),
            ("atomType", Value::Bytes(ATOM_TYPE.0.to_vec())),
            (
                "atomBytes",
                Value::Bytes(if create { b"created" } else { b"ensured" }.to_vec()),
            ),
        ])
    };
    publication::encoded(&publication::map([
        ("anchor", publication::id(0x40)),
        ("initial", node(false)),
        (
            "items",
            Value::Array(
                branches
                    .iter()
                    .map(|create| {
                        publication::map([
                            ("create", Value::Bool(*create)),
                            ("node", node(*create)),
                        ])
                    })
                    .collect(),
            ),
        ),
    ]))
}

fn run(branches: &[bool]) -> Result<Vec<EffectMutation>, EffectFrameError> {
    let (bytes, identity) = retained();
    let (mut host, head, root) = host();
    let admitted = host
        .admit_echo_operation_package(&policy(identity), bytes)
        .expect("exact public package admission");
    let installed = host
        .install_admitted_echo_operation_package(admitted)
        .expect("public package installation");
    let program = installed
        .program()
        .as_effect_program()
        .expect("effect program");
    let input = input(root.warp_id, branches);
    let parent = host
        .echo_operation_evaluation_basis_v1(head, basis(&input))
        .expect("basis");
    let mut frame = host
        .effect_evaluation_frame_v1(
            parent,
            &[],
            0,
            EchoOperationBudgetV1::new(4096, 4096, 4096),
            limits(),
        )
        .expect("frame")
        .with_footprint_law(program.footprint_law().clone())
        .expect("law");
    let output = match frame.evaluate_effect(program, &input, &[]) {
        Ok(output) => output.to_vec(),
        Err(error) => {
            assert!(frame.buffered_mutations().is_empty(), "no partial buffer");
            assert_eq!(
                frame
                    .into_effect_evaluation(4096)
                    .expect_err("no completion"),
                EffectCompletionError::NoCompletedOperation
            );
            return Err(error);
        }
    };
    assert_eq!(output, publication::encoded(&publication::map([])));
    let buffered = frame.buffered_mutations().to_vec();
    let completed = frame
        .into_effect_evaluation(4096)
        .expect("completed evidence");
    assert_eq!(completed.mutations(), buffered.as_slice());
    assert_eq!(completed.output(), output.as_slice());
    assert_eq!(completed.input_basis(), basis(&input));
    Ok(buffered)
}

#[test]
fn the_public_structured_package_preserves_loop_and_branch_mutation_order() {
    let ensured = EffectMutation::EnsureNode {
        node: NodeKey {
            warp_id: WarpId([0x31; 32]),
            local_id: NodeId([0x41; 32]),
        },
        ty: NODE_TYPE,
        atom: AtomPayload {
            type_id: ATOM_TYPE,
            bytes: b"ensured".to_vec().into(),
        },
    };
    let created = EffectMutation::CreateNode {
        node: NodeKey {
            warp_id: WarpId([0x31; 32]),
            local_id: NodeId([0x42; 32]),
        },
        ty: NODE_TYPE,
        atom: AtomPayload {
            type_id: ATOM_TYPE,
            bytes: b"created".to_vec().into(),
        },
    };
    // Source oracle: seed once, then visit each item and exactly its chosen arm.
    for (branches, expected) in [
        (vec![], vec![ensured.clone()]),
        (vec![true], vec![ensured.clone(), created.clone()]),
        (vec![true, false], vec![ensured.clone(), created, ensured]),
    ] {
        assert_eq!(
            run(&branches).expect("public structured execution"),
            expected
        );
    }
}

#[test]
fn the_public_structured_package_refuses_a_list_above_its_authored_bound() {
    assert_eq!(
        run(&[false, false, false]),
        Err(EffectFrameError::from(EvaluationError::InvalidInput))
    );
}

#[test]
fn another_public_package_is_refused_under_the_structured_package_policy() {
    let (_, identity) = retained();
    let (host, _, _) = host();
    let other = hex::decode(
        include_str!("../fixtures/edict-effect-program/flat-executable-operation-package.hex")
            .trim(),
    )
    .expect("flat public package");
    assert_eq!(
        host.admit_echo_operation_package(&policy(identity), other)
            .expect_err("wrong package refused")
            .kind(),
        EchoOperationAdmissionErrorKind::PackageIdentityMismatch
    );
}
