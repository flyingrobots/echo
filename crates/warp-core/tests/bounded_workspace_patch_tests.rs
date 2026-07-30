// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! RED contract for compiler-authored basis-bound workspace patches.

#![allow(clippy::panic)]

use echo_edict_canonical::{encode_canonical_cbor_v1, CanonicalValueV1};
use warp_core::external_action_adapter::admit_edict_external_action_request_v1;
use warp_core::{Hash, WorldlineId};

const CORE_BYTES: &[u8] =
    include_bytes!("fixtures/external_action_patch/apply-validated-patch.core.cbor");
const TARGET_IR_BYTES: &[u8] =
    include_bytes!("fixtures/external_action_patch/apply-validated-patch.target-ir.cbor");
const CORE_DIGEST: &str =
    include_str!("fixtures/external_action_patch/apply-validated-patch.core.sha256");
const TARGET_IR_DIGEST: &str =
    include_str!("fixtures/external_action_patch/apply-validated-patch.target-ir.sha256");

fn digest(label: &str) -> Hash {
    blake3::hash(label.as_bytes()).into()
}

fn must_ok<T, E: core::fmt::Debug>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("expected Ok(..), got {error:?}"),
    }
}

fn text(value: &str) -> CanonicalValueV1 {
    CanonicalValueV1::Text(value.to_owned())
}

fn map(entries: impl IntoIterator<Item = (&'static str, CanonicalValueV1)>) -> CanonicalValueV1 {
    CanonicalValueV1::Map(
        entries
            .into_iter()
            .map(|(key, value)| (text(key), value))
            .collect(),
    )
}

fn application_input(
    patch: Vec<u8>,
    authority: Hash,
    basis: Hash,
    max_settlement_bytes: u64,
) -> Vec<u8> {
    must_ok(encode_canonical_cbor_v1(&map([
        ("patch", CanonicalValueV1::Bytes(patch)),
        ("authority", CanonicalValueV1::Bytes(authority.to_vec())),
        ("basis", CanonicalValueV1::Bytes(basis.to_vec())),
        (
            "maxSettlementBytes",
            CanonicalValueV1::Integer(i128::from(max_settlement_bytes)),
        ),
        ("maxAttempts", CanonicalValueV1::Integer(1)),
    ])))
}

#[test]
fn exact_compiler_artifacts_admit_one_noncallable_patch_request() {
    let authority = digest("authority:exact");
    let basis = digest("basis:exact");
    let patch = must_ok(encode_canonical_cbor_v1(&map([])));
    let admitted = must_ok(admit_edict_external_action_request_v1(
        WorldlineId::from_bytes([23; 32]),
        CORE_BYTES,
        TARGET_IR_BYTES,
        "applyValidated",
        &application_input(patch.clone(), authority, basis, 65_536),
    ));

    assert_eq!(admitted.source_core_digest(), CORE_DIGEST.trim());
    assert_eq!(admitted.target_ir_digest(), TARGET_IR_DIGEST.trim());
    assert_eq!(
        admitted.operation_coordinate(),
        "workspace.patch.applyValidated@1"
    );
    assert_eq!(admitted.canonical_operation_input(), patch);
    assert_eq!(admitted.request().authority_scope_digest, authority);
    assert_eq!(admitted.request().basis_digest, basis);
    assert_eq!(admitted.request().budget.max_settlement_bytes, 65_536);
    assert_eq!(admitted.request().budget.max_attempts, 1);
}
