// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Coherently rebound native requests; the verifier does not use altered lowerer output.
use super::*;

fn edit(core: &mut CanonicalValueV1, capability: Option<CanonicalValueV1>, variant: bool) {
    if variant {
        add_field(
            map_field_mut(core, "types"),
            "UnusedChoice",
            map([
                ("kind", text("Variant")),
                ("cases", map([("Empty", map([]))])),
            ]),
        );
    }
    let CanonicalValueV1::Map(fields) = core else {
        panic!("Core map")
    };
    fields.retain(|(key, _)| key != &text("requiredCoreCapabilities"));
    if let Some(capability) = capability {
        fields.push((text("requiredCoreCapabilities"), capability));
    }
}
fn invalid() -> Vec<(Option<CanonicalValueV1>, bool)> {
    vec![
        (Some(list(vec![])), true),
        (None, false),
        (Some(CanonicalValueV1::Null), false),
        (Some(list(vec![text("core.variant/v2")])), false),
        (Some(list(vec![text("core.unknown/v1")])), false),
        (
            Some(list(vec![text("core.variant/v1"), text("core.variant/v1")])),
            true,
        ),
        (Some(list(vec![CanonicalValueV1::Null])), false),
    ]
}
#[test]
fn pure_providers_admit_the_known_variant_capability_with_unused_declarations() {
    for variant in [false, true] {
        let (names, fixture, package) = edited_fixture(|core, _| {
            edit(core, Some(list(vec![text("core.variant/v1")])), variant);
        });
        let lowered =
            lowerer::lower(lowering_request(names, &fixture)).expect("known capability lowers");
        assert_eq!(lowered.outputs[0].artifact.bytes, package);
        let verified = verifier::verify(verification_request(names, &fixture, package))
            .expect("independent known capability verification");
        let report = decode_canonical_cbor_v1(&verified.outputs[0].artifact.bytes).expect("report");
        assert_eq!(text_field(&report, "outcome"), Some("accepted"));
    }
}
#[test]
fn pure_lowerer_refuses_missing_unknown_or_malformed_required_capabilities() {
    for (capability, variant) in invalid() {
        let (names, fixture, _) = edited_fixture(|core, _| edit(core, capability, variant));
        let refusal = lowerer::lower(lowering_request(names, &fixture))
            .expect_err("unsupported capability metadata");
        assert_eq!(
            refusal.kind,
            lowerer::ProviderRefusalKind::UnsupportedSemantics
        );
    }
}
#[test]
fn pure_verifier_independently_refuses_missing_unknown_or_malformed_required_capabilities() {
    for (capability, variant) in invalid() {
        let (names, fixture, package) = edited_fixture(|core, _| edit(core, capability, variant));
        let refusal = verifier::verify(verification_request(names, &fixture, package))
            .expect_err("independent capability refusal");
        assert_eq!(
            refusal.kind,
            verifier::ProviderRefusalKind::UnsupportedSemantics
        );
    }
}
