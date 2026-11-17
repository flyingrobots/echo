// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Public compiler output for a helper that declares a typed failure and an
//! intent that maps it to an obstruction. The oracle is the source: the
//! helper returns `end - start` when `start <= end` and otherwise fails
//! `Underflow`, which the intent maps to `RangeInverted`.
use super::current_effect_publication_fixture as publication;
use super::public_read_effect::{basis, host, limits};
use echo_edict_canonical::{decode_canonical_cbor_v1 as decode, CanonicalValueV1 as Value};
use warp_core::edict_effects::{EffectFrameError, EffectMutation};
use warp_core::{
    AtomPayload, EchoOperationAdmissionPolicy, EchoOperationBudgetV1, NodeId, NodeKey, TypeId,
    WarpId,
};

const PACKAGE: &str =
    include_str!("../fixtures/edict-effect-typed-failure/typed-executable-operation-package.hex");
/// `echo:operation-package:v1` identity of the retained package bytes.
const PACKAGE_PIN: &str = "a29ebe80798166bedf630833c4a4b91d8cbd518f38da4f0d49e1263884cab5a2";
const OPERATION: &str = "consumer.typedfail.program@1.measure";
const AUTHORITY: &[u8] = b"consumer.typedfail.authority/v1";
/// The reason as Core names it, in the program's own lawpack alias, as for
/// every authored obstruction.
const REASON: &str = "fx.RangeInverted";
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
    let core = embedded(&bundle, "core_artifact");
    assert!(
        matches!(publication::get(&core, "requiredCoreCapabilities"),
            Value::Array(capabilities) if capabilities.contains(&Value::Text("core.failure/v1".to_owned()))),
        "the compiler declared core.failure/v1"
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

fn input(warp: WarpId, start: u64, end: u64) -> Vec<u8> {
    publication::encoded(&publication::map([
        ("anchor", publication::id(0x40)),
        ("start", Value::Integer(i128::from(start))),
        ("end", Value::Integer(i128::from(end))),
        (
            "node",
            publication::map([
                ("warpId", Value::Bytes(warp.0.to_vec())),
                ("nodeId", publication::id(0x41)),
                ("nodeType", Value::Bytes(NODE_TYPE.0.to_vec())),
                ("atomType", Value::Bytes(ATOM_TYPE.0.to_vec())),
                ("atomBytes", Value::Bytes(b"seeded".to_vec())),
            ]),
        ),
    ]))
}

fn run(start: u64, end: u64) -> Result<Vec<EffectMutation>, EffectFrameError> {
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
    let input = input(root.warp_id, start, end);
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
    match frame.evaluate_effect(program, &input, &[]) {
        Ok(_) => Ok(frame.buffered_mutations().to_vec()),
        Err(error) => {
            assert!(frame.buffered_mutations().is_empty(), "no partial buffer");
            Err(error)
        }
    }
}

#[test]
fn the_public_typed_failure_package_returns_when_its_helper_returns() {
    assert_eq!(
        run(1, 3).expect("distance(1, 3) returns"),
        vec![EffectMutation::EnsureNode {
            node: NodeKey {
                warp_id: WarpId([0x31; 32]),
                local_id: NodeId([0x41; 32]),
            },
            ty: NODE_TYPE,
            atom: AtomPayload {
                type_id: ATOM_TYPE,
                bytes: b"seeded".to_vec().into(),
            },
        }]
    );
}

#[test]
fn the_public_typed_failure_package_obstructs_with_exactly_its_mapped_reason() {
    assert_eq!(
        run(5, 3),
        Err(EffectFrameError::AuthoredObstruction {
            reason_kind: REASON.to_owned(),
            payload: publication::encoded(&publication::map([])),
        }),
        "distance(5, 3) fails Underflow, which the intent maps to RangeInverted"
    );
}
