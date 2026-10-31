// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Native capability admission with coherently rebound Core, Target and package carriers.
use super::*;
fn edit(core: &mut Value, capability: Option<Value>, variant: bool) {
    if variant {
        insert(
            field_mut(core, "types"),
            "UnusedChoice",
            map([
                ("kind", text("Variant")),
                ("cases", map([("Empty", map([]))])),
            ]),
        );
    }
    let Value::Map(fields) = core else {
        panic!("Core map")
    };
    fields.retain(|(key, _)| key != &text("requiredCoreCapabilities"));
    if let Some(capability) = capability {
        fields.push((text("requiredCoreCapabilities"), capability));
    }
}
fn source(capability: Option<Value>, variant: bool, helper: bool) -> LoweringRequestV1 {
    let mut source = read_with_budget(4_194_304, helper);
    let mut core = decode(&source.core.artifact.bytes).expect("Core");
    edit(&mut core, capability, variant);
    source.core = bound(
        &source.core.reference.coordinate,
        "edict.core.module/v1",
        encode(&core).expect("Core"),
    );
    let artifact = &mut source.semantic_inputs[5].artifact;
    let mut target = decode(&artifact.artifact.bytes).expect("Target");
    *field_mut(
        field_mut(field_mut(&mut target, "semanticClosure"), "sourceCore"),
        "digest",
    ) = Value::Array(vec![
        text("sha256"),
        Value::Bytes(source.core.reference.digest.bytes.clone()),
    ]);
    *artifact = bound(
        &artifact.reference.coordinate,
        &artifact.artifact.domain,
        encode(&target).expect("Target"),
    );
    source
}
fn invalid() -> Vec<(Option<Value>, bool)> {
    vec![
        (Some(Value::Array(vec![])), true),
        (None, false),
        (Some(Value::Null), false),
        (Some(Value::Array(vec![text("core.variant/v2")])), false),
        (Some(Value::Array(vec![text("core.unknown/v1")])), false),
        (
            Some(Value::Array(vec![
                text("core.variant/v1"),
                text("core.variant/v1"),
            ])),
            true,
        ),
        (Some(Value::Array(vec![Value::Null])), false),
    ]
}
#[test]
fn read_lowerer_admits_the_known_variant_capability_with_or_without_helpers() {
    for helper in [false, true] {
        for variant in [false, true] {
            lower(source(
                Some(Value::Array(vec![text("core.variant/v1")])),
                variant,
                helper,
            ))
            .expect("known capability lowers");
        }
    }
}
#[test]
fn read_verifier_independently_admits_the_known_variant_capability() {
    for helper in [false, true] {
        for variant in [false, true] {
            // Only the unmodified scalar control is lowered. The candidate is
            // independently rebound, so lowerer refusal cannot mask this check.
            let mut candidate = request(read_with_budget(4_194_304, helper));
            changed_core(&mut candidate, |core| {
                edit(
                    core,
                    Some(Value::Array(vec![text("core.variant/v1")])),
                    variant,
                );
            });
            let verified = verifier::verify(candidate).expect("known capability verifies");
            let report = decode(&verified.outputs[0].artifact.bytes).expect("report");
            assert_eq!(field(&report, "outcome"), &text("accepted"));
        }
    }
}
#[test]
fn read_lowerer_refuses_missing_unknown_or_malformed_required_capabilities() {
    for (capability, variant) in invalid() {
        let expected = if capability.is_none() {
            echo_edict_provider_lowerer::ProviderRefusalKind::InvalidSemanticArtifact
        } else {
            echo_edict_provider_lowerer::ProviderRefusalKind::UnsupportedSemantics
        };
        let refusal =
            lower(source(capability, variant, false)).expect_err("unsupported capability metadata");
        assert_eq!(refusal.kind, expected);
    }
}
#[test]
fn read_verifier_independently_refuses_missing_unknown_or_malformed_required_capabilities() {
    for (capability, variant) in invalid() {
        let mut candidate = request(read_with_budget(4_194_304, false));
        changed_core(&mut candidate, |core| edit(core, capability, variant));
        let response = verifier::verify(candidate).expect("well-bound candidate receives a report");
        let report = decode(&response.outputs[0].artifact.bytes).expect("report");
        assert_eq!(field(&report, "outcome"), &text("rejected"));
    }
}
