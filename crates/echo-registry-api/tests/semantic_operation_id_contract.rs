// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Executable contract for generic semantic operation identifiers.

use echo_registry_api::{
    is_reserved_operation_id, stable_semantic_operation_id_v1, OpKind,
    RESERVED_CONTROL_OPERATION_ID, RESERVED_IMPORT_SUFFIX_OPERATION_ID,
    SEMANTIC_OPERATION_ID_LAW_V1,
};

const MUTATION_ID: u32 = stable_semantic_operation_id_v1(OpKind::Mutation, "a.b@1.t");
const READ_ID: u32 = stable_semantic_operation_id_v1(OpKind::Query, "a.b@1.read");

fn legacy_wesley_fnv1_id(kind_rank: u8, field_name: &str) -> u32 {
    let mut hash = 2_166_136_261_u32;
    for byte in std::iter::once(kind_rank).chain(field_name.bytes()) {
        hash = hash.wrapping_mul(16_777_619) ^ u32::from(byte);
    }
    hash
}

#[test]
fn semantic_operation_id_v1_is_domain_separated_and_byte_exact() {
    assert_eq!(
        SEMANTIC_OPERATION_ID_LAW_V1,
        "echo.semantic-operation-id.fnv1-32/v1"
    );
    assert_eq!(MUTATION_ID, 3_389_142_194);
    assert_eq!(READ_ID, 2_012_636_359);
    assert_eq!(RESERVED_CONTROL_OPERATION_ID, u32::MAX);
    assert_eq!(RESERVED_IMPORT_SUFFIX_OPERATION_ID, u32::MAX - 1);
    assert!(is_reserved_operation_id(RESERVED_CONTROL_OPERATION_ID));
    assert!(is_reserved_operation_id(
        RESERVED_IMPORT_SUFFIX_OPERATION_ID
    ));
    assert!(!is_reserved_operation_id(u32::MAX - 2));
    assert_ne!(MUTATION_ID, READ_ID);

    assert_ne!(MUTATION_ID, legacy_wesley_fnv1_id(1, "a.b@1.t"));
    assert_ne!(
        stable_semantic_operation_id_v1(OpKind::Query, "same.coordinate"),
        stable_semantic_operation_id_v1(OpKind::Mutation, "same.coordinate")
    );
    assert_ne!(
        stable_semantic_operation_id_v1(OpKind::Mutation, "\u{00e9}"),
        stable_semantic_operation_id_v1(OpKind::Mutation, "e\u{0301}")
    );
}
