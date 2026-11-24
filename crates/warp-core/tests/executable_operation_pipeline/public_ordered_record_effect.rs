// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Public compiler output for a helper that returns a fallible ordered record.
//! The oracle is the source: `{ z: fail First, a: fail Second }` stops at its
//! first field, so the intent obstructs with `FirstRaised`; the same record
//! authored `{ a: fail Second, z: fail First }` obstructs with `SecondRaised`.
use super::current_effect_publication_fixture as publication;
use super::public_read_effect::{basis, host, limits};
use echo_edict_canonical::{decode_canonical_cbor_v1 as decode, CanonicalValueV1 as Value};
use warp_core::edict_effects::EffectFrameError;
use warp_core::{EchoOperationAdmissionPolicy, EchoOperationBudgetV1, TypeId, WarpId};

const Z_FIRST: &str = include_str!(
    "../fixtures/edict-effect-ordered-record-z/ordered-z-executable-operation-package.hex"
);
const A_FIRST: &str = include_str!(
    "../fixtures/edict-effect-ordered-record-a/ordered-a-executable-operation-package.hex"
);
const OPERATION: &str = "consumer.ordered.program@1.order";
const AUTHORITY: &[u8] = b"consumer.ordered.authority/v1";
const NODE_TYPE: TypeId = TypeId([0x5b; 32]);
const ATOM_TYPE: TypeId = TypeId([0x5c; 32]);

fn embedded(value: &Value, key: &str) -> Value {
    let Value::Bytes(bytes) = publication::get(value, key) else {
        panic!("embedded public artifact bytes")
    };
    decode(bytes).expect("canonical public artifact")
}

/// The retained package, after checking that the compiler emitted the
/// helper's record as an `orderedRecord` whose entries keep `first_field`
/// first.
fn retained(hex: &str, first_field: &str) -> (Vec<u8>, [u8; 32]) {
    let bytes = hex::decode(hex.trim()).expect("retained public hex");
    let package = decode(&bytes).expect("public package");
    let program = embedded(&package, "program");
    let bundle = embedded(&program, "compiled_program");
    let core = embedded(&bundle, "core_artifact");
    let result = publication::get(
        publication::get(
            publication::get(publication::get(&core, "functions"), "pair"),
            "body",
        ),
        "result",
    );
    assert_eq!(
        publication::get(result, "kind"),
        &Value::Text("orderedRecord".to_owned()),
        "the compiler kept the fallible record's order"
    );
    let Value::Array(entries) = publication::get(result, "fields") else {
        panic!("ordered entries")
    };
    assert_eq!(
        publication::get(&entries[0], "name"),
        &Value::Text(first_field.to_owned()),
        "authored first field"
    );
    let identity = publication::hash(b"echo:operation-package:v1\0", &bytes);
    (bytes, identity)
}

fn run(hex: &str, first_field: &str) -> Result<(), EffectFrameError> {
    let (bytes, identity) = retained(hex, first_field);
    let (mut host, head, root) = host();
    let policy = EchoOperationAdmissionPolicy::exact(
        identity,
        OPERATION,
        publication::hash(b"echo:operation-profile:v1\0", AUTHORITY),
        EchoOperationBudgetV1::new(4096, 4096, 4096),
    );
    let admitted = host
        .admit_echo_operation_package(&policy, bytes)
        .expect("exact public package admission");
    let installed = host
        .install_admitted_echo_operation_package(admitted)
        .expect("public package installation");
    let program = installed
        .program()
        .as_effect_program()
        .expect("effect program");
    let input = input(root.warp_id);
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
    let result = frame.evaluate_effect(program, &input, &[]).map(|_| ());
    assert!(frame.buffered_mutations().is_empty(), "no partial buffer");
    result
}

fn input(warp: WarpId) -> Vec<u8> {
    publication::encoded(&publication::map([
        ("anchor", publication::id(0x40)),
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

fn obstruction(reason: &str) -> Result<(), EffectFrameError> {
    Err(EffectFrameError::AuthoredObstruction {
        reason_kind: reason.to_owned(),
        payload: publication::encoded(&publication::map([])),
    })
}

#[test]
fn the_public_ordered_record_obstructs_with_its_first_authored_failure() {
    assert_eq!(
        run(Z_FIRST, "z"),
        obstruction("fx.FirstRaised"),
        "{{ z: fail First, a: fail Second }} stops at First"
    );
    assert_eq!(
        run(A_FIRST, "a"),
        obstruction("fx.SecondRaised"),
        "{{ a: fail Second, z: fail First }} stops at Second"
    );
}
