// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Runtime-owner V2 package admission under independent policy.
#![cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
#![allow(clippy::expect_used, clippy::panic)]
#[path = "common/edict_effect_publication.rs"]
mod fixture;
use fixture::*;
use warp_core::edict_effects::{EffectAdmissionError, EffectPackageAdmissionPolicyV2};
use warp_core::{
    make_node_id, make_type_id, EchoOperationBudgetV1, EngineBuilder, GraphStore, NodeRecord,
    TrustedRuntimeHost, WorldlineRuntime,
};
fn host() -> TrustedRuntimeHost {
    let mut graph = GraphStore::default();
    let root = make_node_id("effect-admission-root");
    graph.insert_node(
        root,
        NodeRecord {
            ty: make_type_id("effect-admission-node"),
        },
    );
    let engine = EngineBuilder::new(graph, root).build();
    TrustedRuntimeHost::new(WorldlineRuntime::new(), engine).expect("runtime owner")
}
fn policy(
    bytes: &[u8],
    operation: &str,
    authority: [u8; 32],
    budget: EchoOperationBudgetV1,
) -> EffectPackageAdmissionPolicyV2 {
    EffectPackageAdmissionPolicyV2::exact(
        hash(b"echo:operation-package:v2\0", bytes),
        operation,
        authority,
        budget,
    )
}
#[test]
fn runtime_owner_admits_exact_publication_and_retains_separate_policy_evidence() {
    let bytes = encoded(&package(&program()));
    let policy = policy(
        &bytes,
        "app.update@1",
        [8; 32],
        EchoOperationBudgetV1::new(1, 0, 0),
    );
    let host = host();
    let admitted = host
        .admit_echo_operation_package_v2(&policy, bytes.clone())
        .expect("exact policy admits");
    assert_eq!(admitted.package().canonical_bytes(), bytes);
    assert_eq!(
        admitted.package().identity(),
        hash(b"echo:operation-package:v2\0", &bytes)
    );
    let mut oracle = blake3::Hasher::new();
    oracle.update(b"echo:operation-admission-policy:v1\0");
    oracle.update(&hash(b"echo:operation-package:v2\0", &bytes));
    oracle.update(&12u64.to_le_bytes());
    oracle.update(b"app.update@1");
    oracle.update(&[8; 32]);
    oracle.update(&1u64.to_le_bytes());
    oracle.update(&0u64.to_le_bytes());
    oracle.update(&0u64.to_le_bytes());
    assert_eq!(
        admitted.admission_policy_identity(),
        *oracle.finalize().as_bytes()
    );
    let mut id = blake3::Hasher::new();
    id.update(b"echo:operation-package-admission:v2\0");
    id.update(&hash(b"echo:operation-package:v2\0", &bytes));
    id.update(oracle.finalize().as_bytes());
    assert_eq!(admitted.identity(), *id.finalize().as_bytes());
}
#[test]
fn runtime_policy_refuses_operation_authority_and_budget_substitution() {
    let bytes = encoded(&package(&program()));
    let host = host();
    for (operation, authority, budget, want) in [
        (
            "app.other@1",
            [8; 32],
            EchoOperationBudgetV1::new(1, 0, 0),
            EffectAdmissionError::OperationCoordinateMismatch,
        ),
        (
            "app.update@1",
            [9; 32],
            EchoOperationBudgetV1::new(1, 0, 0),
            EffectAdmissionError::AuthorityProfileMismatch,
        ),
        (
            "app.update@1",
            [8; 32],
            EchoOperationBudgetV1::new(0, 0, 0),
            EffectAdmissionError::BudgetExceedsPolicy,
        ),
    ] {
        assert_eq!(
            host.admit_echo_operation_package_v2(
                &policy(&bytes, operation, authority, budget),
                bytes.clone()
            )
            .expect_err("independent policy refuses"),
            want
        );
    }
}
#[test]
fn pin_identity_does_not_substitute_for_supported_artifact_validation() {
    let host = host();
    let mut value = package(&program());
    set(&mut value, "schema", text("echo.operation-package/v1"));
    let bytes = encoded(&value);
    assert_eq!(
        host.admit_echo_operation_package_v2(
            &policy(
                &bytes,
                "app.update@1",
                [8; 32],
                EchoOperationBudgetV1::new(1, 0, 0)
            ),
            bytes
        )
        .expect_err("pinned but unsupported"),
        EffectAdmissionError::ArtifactInvalid
    );
    let bytes = encoded(&package(&program()));
    let pin = policy(
        b"wrong",
        "app.update@1",
        [8; 32],
        EchoOperationBudgetV1::new(1, 0, 0),
    );
    assert_eq!(
        host.admit_echo_operation_package_v2(&pin, bytes)
            .expect_err("wrong pin"),
        EffectAdmissionError::PackageIdentityMismatch
    );
}
#[test]
fn each_package_budget_axis_is_subject_to_the_independent_policy() {
    let host = host();
    for (steps, read, write) in [(2, 0, 0), (1, 1, 0), (1, 0, 1)] {
        let mut value = package(&program());
        set(
            &mut value,
            "budget_ceiling",
            map([
                (
                    "steps",
                    echo_edict_canonical::CanonicalValueV1::Integer(steps),
                ),
                (
                    "read_bytes",
                    echo_edict_canonical::CanonicalValueV1::Integer(read),
                ),
                (
                    "write_bytes",
                    echo_edict_canonical::CanonicalValueV1::Integer(write),
                ),
            ]),
        );
        let bytes = encoded(&value);
        assert_eq!(
            host.admit_echo_operation_package_v2(
                &policy(
                    &bytes,
                    "app.update@1",
                    [8; 32],
                    EchoOperationBudgetV1::new(1, 0, 0)
                ),
                bytes
            )
            .expect_err("ceiling exceeds policy"),
            EffectAdmissionError::BudgetExceedsPolicy
        );
    }
}
#[test]
fn admission_evidence_changes_with_policy_even_when_exact_program_bytes_do_not() {
    let bytes = encoded(&package(&program()));
    let host = host();
    let narrow = policy(
        &bytes,
        "app.update@1",
        [8; 32],
        EchoOperationBudgetV1::new(1, 0, 0),
    );
    let wide = policy(
        &bytes,
        "app.update@1",
        [8; 32],
        EchoOperationBudgetV1::new(10, 20, 30),
    );
    let first = host
        .admit_echo_operation_package_v2(&narrow, bytes.clone())
        .expect("narrow policy");
    let second = host
        .admit_echo_operation_package_v2(&wide, bytes)
        .expect("wide policy");
    assert_eq!(first.package(), second.package());
    assert_eq!(first.admission_policy(), &narrow);
    assert_eq!(second.admission_policy(), &wide);
    assert_ne!(
        first.admission_policy_identity(),
        second.admission_policy_identity()
    );
    assert_ne!(first.identity(), second.identity());
    let mut wrong_domain = blake3::Hasher::new();
    wrong_domain.update(b"echo:operation-package-admission:v1\0");
    wrong_domain.update(&first.package().identity());
    wrong_domain.update(&first.admission_policy_identity());
    assert_ne!(first.identity(), *wrong_domain.finalize().as_bytes());
}
#[path = "edict_effect_admission_tests/installation.rs"]
mod installation;
