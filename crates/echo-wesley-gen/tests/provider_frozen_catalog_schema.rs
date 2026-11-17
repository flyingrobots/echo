// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! The frozen `echo.dpo@1` catalog keeps its original owning schema.
//!
//! Oracle: the nine-rule closure of `intrinsics-document` in the outgoing
//! Amblicit `function-preconditions-v1` publication, retained as a historical
//! fixture. The generated schema carries it under the
//! `echo-frozen-catalog-v1-` namespace; only rule names may differ.
use cddl_cat::cbor::validate_cbor_bytes;
use cddl_cat::flatten::flatten_from_str;
use echo_wesley_gen::provider_canonical::{encode_canonical_cbor_v1, CanonicalValueV1 as Value};

const ORIGINAL: &str = include_str!(
    "fixtures/historical-contract-packs/function-preconditions-v1/edict-provider-contracts.cddl"
);
const GENERATED: &str = include_str!(
    "../assets/v1/edict-provider/package/v1/generated/primary/schema.echo-provider-artifacts.cddl"
);
const FROZEN_CATALOG: &[u8] = include_bytes!(
    "../assets/v1/edict-provider/package/v1/generated/resources/resource.target-intrinsics.cbor"
);
const PREFIX: &str = "echo-frozen-catalog-v1-";
const FROZEN_ROOT: &str = "echo-frozen-catalog-v1-intrinsics-document";
const CLOSURE: [&str; 9] = [
    "intrinsics-document",
    "intrinsic",
    "pure-intrinsic",
    "effect-intrinsic",
    "core-type-ref",
    "effect-kind",
    "effect-failure-body",
    "failure-ident",
    "authority-class",
];

#[test]
fn frozen_catalog_rules_equal_the_original_closure_up_to_rule_names() {
    let original = flatten_from_str(ORIGINAL).expect("original schema");
    let generated = flatten_from_str(GENERATED).expect("generated schema");
    let namespaced = generated
        .keys()
        .filter(|name| name.starts_with(PREFIX))
        .count();
    assert_eq!(namespaced, CLOSURE.len(), "no extra frozen rules");
    for name in CLOSURE {
        let old = format!("{:?}", original.get(name).expect("original rule"));
        let new = format!(
            "{:?}",
            generated
                .get(&format!("{PREFIX}{name}"))
                .unwrap_or_else(|| panic!("missing frozen {name}"))
        );
        // Rule references render as `name: "<rule>"`; literals never do.
        assert_eq!(
            new.replace(&format!("name: \"{PREFIX}"), "name: \""),
            old,
            "{name}"
        );
    }
}

#[test]
fn frozen_root_admits_the_frozen_catalog_and_the_current_root_refuses_it() {
    validate_cbor_bytes(FROZEN_ROOT, GENERATED, FROZEN_CATALOG)
        .expect("the frozen root owns the echo.dpo@1 catalog");
    assert!(
        validate_cbor_bytes("intrinsics-document", GENERATED, FROZEN_CATALOG).is_err(),
        "the current root admits only catalog v2"
    );
}

#[test]
fn frozen_root_refuses_catalog_v2() {
    let v2 = encode_canonical_cbor_v1(&Value::Map(vec![
        (
            Value::Text("apiVersion".into()),
            Value::Text("edict.target-profile.intrinsics/v2".into()),
        ),
        (Value::Text("intrinsics".into()), Value::Map(vec![])),
    ]))
    .expect("specimen");
    assert!(validate_cbor_bytes(FROZEN_ROOT, GENERATED, &v2).is_err());
    validate_cbor_bytes("intrinsics-document", GENERATED, &v2)
        .expect("an empty v2 catalog is a current value");
}
