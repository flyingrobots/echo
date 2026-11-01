// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Rebound native end-to-end provider relation, not public compiler provenance.
use super::*;
fn folded_fixture(bound: u64, capability: bool) -> (FixtureNames<'static>, RawFixture, Vec<u8>) {
    called_fixture(|core, target| {
        let coordinate = text_field(core, "coordinate")
            .expect("coordinate")
            .to_owned();
        *map_field_mut(core, "requiredCoreCapabilities") = list(if capability {
            vec![text("core.fold/v1")]
        } else {
            vec![]
        });
        for (artifact, key) in [(&mut *core, "nodes"), (&mut *target, "pureBindings")] {
            let body = if key == "nodes" {
                map_field_mut(intent(artifact), "body")
            } else {
                intent(artifact)
            };
            let CanonicalValueV1::Array(bindings) = map_field_mut(body, key) else {
                unreachable!()
            };
            let initial = map_field(&bindings[0], "value").clone();
            *map_field_mut(&mut bindings[0], "value") = map([
                ("kind", text("fold")),
                ("initial", initial),
                ("step", text(format!("{coordinate}.passNext"))),
                ("bound", CanonicalValueV1::Integer(bound.into())),
                ("stateType", text("U64")),
            ]);
        }
    })
}
#[test]
fn native_providers_independently_accept_executed_fold_and_exact_carriers() {
    for bound in [0, 1, 2, 3] {
        let (names, fixture, package) = folded_fixture(bound, true);
        let lowered = lowerer::lower(lowering_request(names, &fixture)).expect("fold lowers");
        assert_eq!(lowered.outputs[0].artifact.bytes, package);
        assert!(
            verifier_accepts(names, &fixture, package),
            "fold verifies independently at bound {bound}"
        );
    }
}
#[test]
fn native_providers_independently_refuse_fold_without_capability_or_with_cost_overflow() {
    for (bound, capability) in [(0, false), (3, false), (u64::MAX, true)] {
        let (names, fixture, package) = folded_fixture(bound, capability);
        both_refuse(names, &fixture, package, "fold capability or cost");
    }
}

#[test]
fn native_providers_refuse_functionless_core_fold_instead_of_skipping_judgment() {
    let (names, fixture, package) = edited_fixture(|_, _| {});
    assert!(
        lowerer::lower(lowering_request(names, &fixture)).is_ok(),
        "functionless non-fold control"
    );
    assert!(
        verifier_accepts(names, &fixture, package),
        "independent control"
    );
    for (bound, capability) in [(0, false), (3, false), (0, true), (3, true)] {
        let (names, fixture, package) = edited_fixture(|core, target| {
            let CanonicalValueV1::Map(fields) = &*core else {
                unreachable!()
            };
            assert!(
                !fields.iter().any(|(key, _)| key == &text("functions")),
                "fixture has no function authority"
            );
            *map_field_mut(core, "requiredCoreCapabilities") = list(if capability {
                vec![text("core.fold/v1")]
            } else {
                vec![]
            });
            let coordinate = text_field(core, "coordinate")
                .expect("coordinate")
                .to_owned();
            for (artifact, key) in [(&mut *core, "nodes"), (&mut *target, "pureBindings")] {
                let body = if key == "nodes" {
                    map_field_mut(intent(artifact), "body")
                } else {
                    intent(artifact)
                };
                let CanonicalValueV1::Array(bindings) = map_field_mut(body, key) else {
                    unreachable!()
                };
                *map_field_mut(&mut bindings[0], "value") = map([
                    ("kind", text("fold")),
                    ("initial", literal("U64", 0)),
                    ("step", text(format!("{coordinate}.missing"))),
                    ("bound", CanonicalValueV1::Integer(bound.into())),
                    ("stateType", text("U64")),
                ]);
            }
        });
        both_refuse(
            names,
            &fixture,
            package,
            "functionless fold must not skip judgment",
        );
    }
}
