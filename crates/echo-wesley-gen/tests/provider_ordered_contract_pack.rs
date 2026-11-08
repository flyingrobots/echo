// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used)]
//! Current publication structural conformance; not runtime execution evidence.
use echo_wesley_gen::provider_canonical::{encode_canonical_cbor_v1, CanonicalValueV1 as Value};
use echo_wesley_gen::provider_contract_pack::{
    admit_provider_contract_pack_v1, ProviderContractValidationErrorKind,
};
const SCHEMA: &[u8] =
    include_bytes!("../assets/v1/edict-provider/contracts/v1/edict-provider-contracts.cddl");
const MANIFEST: &[u8] = include_bytes!("../assets/v1/edict-provider/contracts/v1/manifest.json");
fn text(value: &str) -> Value {
    Value::Text(value.to_owned())
}
fn map(entries: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(
        entries
            .into_iter()
            .map(|(key, value)| (text(key), value))
            .collect(),
    )
}

// A structural CDDL specimen, deliberately not a compiler or runtime witness.
fn ordered_envelope(order: Option<Value>) -> Value {
    let mut intent = vec![
        ("operationProfile", text("continuum.profile.read/v1")),
        ("inputConstraints", Value::Array(vec![])),
        (
            "coreEvaluationBudget",
            map([
                ("maxSteps", Value::Integer(1)),
                ("maxAllocatedBytes", Value::Integer(1)),
                ("maxOutputBytes", Value::Integer(1)),
            ]),
        ),
        ("requirements", Value::Array(vec![])),
        ("steps", Value::Array(vec![])),
        (
            "result",
            map([("kind", text("record")), ("fields", map([]))]),
        ),
    ];
    if let Some(order) = order {
        intent.push(("executionOrder", order));
    }
    map([
        ("kind", text("orderedTargetIrArtifact")),
        ("domain", text("echo.span-ir/v2")),
        (
            "targetProfile",
            map([
                ("id", text("echo.dpo@1")),
                (
                    "digest",
                    Value::Array(vec![text("sha256"), Value::Bytes(vec![0; 32])]),
                ),
            ]),
        ),
        ("sourceCoreCoordinate", text("conformance.ordered@1")),
        ("intents", map([("read", map(intent))])),
    ])
}

#[test]
fn ordered_schema_validates_shape_and_requires_the_order_table() {
    let ordered = admit_provider_contract_pack_v1(SCHEMA, MANIFEST).expect("ordered admission");
    let valid = encode_canonical_cbor_v1(&ordered_envelope(Some(Value::Array(vec![]))))
        .expect("canonical specimen");
    ordered
        .validate_contract_bytes("target-ir-artifact", &valid)
        .expect("ordered shape");
    for order in [None, Some(text("not an order array"))] {
        let malformed = encode_canonical_cbor_v1(&ordered_envelope(order)).expect("canonical");
        let error = ordered
            .validate_contract_bytes("target-ir-artifact", &malformed)
            .expect_err("malformed order cannot pass the selected schema");
        assert_eq!(
            error.kind(),
            ProviderContractValidationErrorKind::SchemaMismatch
        );
    }
}

fn put(value: &mut Value, key: &str, next: Value) {
    let Value::Map(entries) = value else {
        unreachable!()
    };
    let (_, target) = entries
        .iter_mut()
        .find(|(k, _)| k == &text(key))
        .expect("field");
    *target = next;
}

#[test]
fn structured_target_order_and_phased_domains_have_distinct_schema_boundaries() {
    let pack = admit_provider_contract_pack_v1(SCHEMA, MANIFEST).expect("current pack");
    let mut structured = ordered_envelope(Some(Value::Array(vec![])));
    put(&mut structured, "kind", text("structuredTargetIrArtifact"));
    put(&mut structured, "domain", text("echo.span-ir/v3"));
    pack.validate_contract_bytes(
        "target-ir-artifact",
        &encode_canonical_cbor_v1(&structured).expect("structured bytes"),
    )
    .expect("structured order shape");
    for domain in ["echo.span-ir/v2", "echo.span-ir/v3"] {
        let mut phased = ordered_envelope(None);
        put(&mut phased, "kind", text("targetIrArtifact"));
        put(&mut phased, "domain", text(domain));
        let bytes = encode_canonical_cbor_v1(&phased).expect("phased bytes");
        assert_eq!(
            pack.validate_contract_bytes("target-ir-artifact", &bytes)
                .expect_err("reserved domain cannot omit execution order")
                .kind(),
            ProviderContractValidationErrorKind::SchemaMismatch
        );
        put(&mut phased, "domain", text("echo.span-ir/v1"));
        pack.validate_contract_bytes(
            "target-ir-artifact",
            &encode_canonical_cbor_v1(&phased).expect("control bytes"),
        )
        .expect("phased control");
    }
}
