// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used)]
//! The current upstream publication owns intrinsic catalog v2.
//!
//! Oracle: Amblicit `typed-failures-v1` CDDL, rule `intrinsics-document`
//! (catalog v2, EDICT-TARGET-PURE-INTRINSIC-001), and Amblicit's
//! `intrinsic_catalog_publication.rs` pure entry for `contentAddress`.
//! Structural conformance only; not admission or execution evidence.
use echo_wesley_gen::provider_canonical::{encode_canonical_cbor_v1, CanonicalValueV1 as Value};
use echo_wesley_gen::provider_contract_pack::{
    admit_provider_contract_pack_v1, ProviderContractValidationErrorKind as Kind,
};

const SCHEMA: &[u8] =
    include_bytes!("../assets/v1/edict-provider/contracts/v1/edict-provider-contracts.cddl");
const MANIFEST: &[u8] = include_bytes!("../assets/v1/edict-provider/contracts/v1/manifest.json");
const CATALOG: &str = "target-profile-intrinsics";
const ID: &str = "Bytes<exact=32>";

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

fn content_address(complete: bool) -> Value {
    let mut fields = vec![
        ("intrinsicClass", text("pure")),
        ("typeParameters", Value::Array(vec![])),
        (
            "argumentTypes",
            Value::Array(
                [
                    ID,
                    ID,
                    ID,
                    "Bytes<max=1048576>",
                    "List<echo.dpo@2.OutgoingEdge,max=16>",
                ]
                .map(text)
                .to_vec(),
            ),
        ),
        ("returnType", text(ID)),
        ("guardSupport", Value::Bool(false)),
        (
            "footprintTemplate",
            text("echo.dpo@2.contentAddress.footprint"),
        ),
        ("costTemplate", text("echo.dpo@2.contentAddress.cost")),
        ("writeClass", text("none")),
    ];
    if complete {
        fields.push(("contractVersion", text("echo.content-address/v1")));
        fields.push((
            "costRule",
            map([
                ("base", Value::Integer(71)),
                (
                    "perArgumentMax",
                    Value::Array([0, 0, 0, 1, 67].map(Value::Integer).to_vec()),
                ),
            ]),
        ));
    }
    map(fields)
}

fn catalog(api: &str, complete: bool) -> Vec<u8> {
    encode_canonical_cbor_v1(&map([
        ("apiVersion", text(api)),
        (
            "intrinsics",
            map([("echo.dpo@2.contentAddress", content_address(complete))]),
        ),
        (
            "types",
            map([(
                "echo.dpo@2.OutgoingEdge",
                text("Record<edgeType:Bytes<exact=32>,to:Bytes<exact=32>>"),
            )]),
        ),
    ]))
    .expect("specimen bytes")
}

#[test]
fn current_catalog_contract_admits_a_complete_v2_pure_contract() {
    let pack = admit_provider_contract_pack_v1(SCHEMA, MANIFEST).expect("current publication");
    pack.validate_contract_bytes(
        CATALOG,
        &catalog("edict.target-profile.intrinsics/v2", true),
    )
    .expect("a complete v2 pure contract is admitted");
}

#[test]
fn current_catalog_contract_refuses_v1_and_incomplete_pure_contracts() {
    let pack = admit_provider_contract_pack_v1(SCHEMA, MANIFEST).expect("current publication");
    for (bytes, why) in [
        (
            catalog("edict.target-profile.intrinsics/v1", true),
            "catalog v1 is not a value of the current root",
        ),
        (
            catalog("edict.target-profile.intrinsics/v2", false),
            "a v2 pure contract needs a version and cost rule",
        ),
    ] {
        assert_eq!(
            pack.validate_contract_bytes(CATALOG, &bytes)
                .expect_err(why)
                .kind(),
            Kind::SchemaMismatch,
            "{why}"
        );
    }
}
