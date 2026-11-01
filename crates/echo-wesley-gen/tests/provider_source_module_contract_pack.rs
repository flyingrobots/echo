// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used)]
//! Current publication structural conformance; not runtime execution evidence.
use echo_wesley_gen::provider_contract_pack::admit_provider_contract_pack_v1;
const SCHEMA: &[u8] =
    include_bytes!("../assets/v1/edict-provider/contracts/v1/edict-provider-contracts.cddl");
const MANIFEST: &[u8] = include_bytes!("../assets/v1/edict-provider/contracts/v1/manifest.json");
fn closure_value(
    count: usize,
    algorithm: &str,
    digest_bytes: usize,
) -> echo_wesley_gen::provider_canonical::CanonicalValueV1 {
    use echo_wesley_gen::provider_canonical::CanonicalValueV1 as Value;
    Value::Map(vec![
        (
            Value::Text("apiVersion".into()),
            Value::Text("edict.source-closure/v1".into()),
        ),
        (
            Value::Text("root".into()),
            Value::Text("library.m0@1".into()),
        ),
        (
            Value::Text("modules".into()),
            Value::Map(
                (0..count)
                    .map(|i| {
                        (
                            Value::Text(format!("library.m{i}@1")),
                            Value::Map(vec![
                                (
                                    Value::Text("source".into()),
                                    Value::Bytes(format!("package library.m{i}@1;").into_bytes()),
                                ),
                                (
                                    Value::Text("digest".into()),
                                    Value::Array(vec![
                                        Value::Text(algorithm.into()),
                                        Value::Bytes(vec![0; digest_bytes]),
                                    ]),
                                ),
                            ]),
                        )
                    })
                    .collect(),
            ),
        ),
    ])
}

#[test]
fn source_module_schema_enforces_closed_envelope_module_counts_and_typed_digests() {
    use echo_wesley_gen::provider_canonical::{
        encode_canonical_cbor_v1, CanonicalValueV1 as Value,
    };
    use echo_wesley_gen::provider_contract_pack::ProviderContractValidationErrorKind;
    let pack = admit_provider_contract_pack_v1(SCHEMA, MANIFEST).expect("current pack");
    assert_eq!(
        pack.root_for_domain("edict.source-closure/v1"),
        Some("source-closure")
    );
    // Structural admission cannot authenticate these placeholder digest values.
    for count in [1, 64] {
        let value = closure_value(count, "sha256", 32);
        let bytes = encode_canonical_cbor_v1(&value).expect("canonical envelope");
        pack.validate_contract_bytes("source-closure", &bytes)
            .expect("bounded structural closure");
    }
    let mut extra_field = closure_value(1, "sha256", 32);
    let Value::Map(ref mut fields) = extra_field else {
        unreachable!()
    };
    fields.push((Value::Text("ignored".into()), Value::Bool(true)));
    for value in [
        Value::Bytes(b"package library.m0@1;".to_vec()),
        closure_value(0, "sha256", 32),
        closure_value(65, "sha256", 32),
        closure_value(1, "sha256", 31),
        closure_value(1, "sha256", 33),
        closure_value(1, "blake3", 32),
        extra_field,
    ] {
        let bytes = encode_canonical_cbor_v1(&value).expect("canonical invalid closure");
        assert_eq!(
            pack.validate_contract_bytes("source-closure", &bytes)
                .expect_err("must reject schema violation")
                .kind(),
            ProviderContractValidationErrorKind::SchemaMismatch
        );
    }
}
