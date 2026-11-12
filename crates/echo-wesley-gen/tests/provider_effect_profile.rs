// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! The generated `echo.dpo@2` effect target profile (flyingrobots/echo#985).
//!
//! Oracles: the intrinsic table is written by hand from the runtime ABI in
//! `docs/topics/GeneratedRules.md`; the `echo.dpo@1` identity is the digest
//! every adapter pinned before this profile existed.

use echo_wesley_gen::provider_artifacts::{
    generate_provider_primary_artifacts_v1, ProviderPrimaryArtifactsV1,
    SchemaValidatedCanonicalProviderOutputV1,
};
use echo_wesley_gen::provider_canonical::CanonicalValueV1 as Value;
use echo_wesley_gen::provider_contract_pack::admit_provider_contract_pack_v1;
use echo_wesley_gen::provider_generation::build_provider_generation_input_v1;

const SOURCE: &[u8] = include_bytes!("../assets/v1/edict-provider/echo-provider-semantics-v1.json");
const SETTINGS: &[u8] = include_bytes!("../assets/v1/edict-provider/generation-settings-v1.json");
const CONTRACT_CDDL: &[u8] =
    include_bytes!("../assets/v1/edict-provider/contracts/v1/edict-provider-contracts.cddl");
const CONTRACT_MANIFEST: &[u8] =
    include_bytes!("../assets/v1/edict-provider/contracts/v1/manifest.json");

/// The identity every existing adapter, lawpack and fixture pins.
const ECHO_DPO_1: &str = "sha256:2e2494121aecf5e6a2d920f5fb85408825d394765fad41484c416397c920fb04";

/// The runtime's effect intrinsics: operation, effect kind (also the write
/// class), and the read's named failures with their authority classes.
/// One runtime intrinsic: operation, effect kind and named failures.
type RuntimeIntrinsic = (
    &'static str,
    &'static str,
    &'static [(&'static str, &'static str)],
);

const RUNTIME_INTRINSICS: [RuntimeIntrinsic; 6] = [
    (
        "node-atom-read",
        "read",
        &[
            ("atomMissing", "resourceFault"),
            ("atomRequired", "integrityFault"),
            ("atomTypeMismatch", "integrityFault"),
            ("atomByteBoundExceeded", "integrityFault"),
        ],
    ),
    ("ensure-node", "ensure", &[]),
    ("create-node", "create", &[]),
    ("replace-atom", "replace", &[]),
    ("ensure-edge", "ensure", &[]),
    ("replace-edge", "replace", &[]),
];

fn generate() -> ProviderPrimaryArtifactsV1 {
    let pack = admit_provider_contract_pack_v1(CONTRACT_CDDL, CONTRACT_MANIFEST)
        .expect("checked contract pack");
    let input = build_provider_generation_input_v1(SOURCE, &pack, SETTINGS).expect("input");
    generate_provider_primary_artifacts_v1(&input, &pack).expect("primary artifacts")
}

fn output<'a>(
    artifacts: &'a ProviderPrimaryArtifactsV1,
    role: &str,
) -> &'a SchemaValidatedCanonicalProviderOutputV1 {
    artifacts
        .artifact(role)
        .or_else(|| artifacts.resource(role))
        .unwrap_or_else(|| panic!("missing provider output role {role}"))
}

fn field<'a>(value: &'a Value, name: &str) -> &'a Value {
    let Value::Map(entries) = value else {
        panic!("{name} parent is not a map")
    };
    entries
        .iter()
        .find_map(|(key, value)| (key == &Value::Text(name.to_owned())).then_some(value))
        .unwrap_or_else(|| panic!("missing field {name}"))
}

fn text(value: &Value) -> &str {
    let Value::Text(value) = value else {
        panic!("not text")
    };
    value
}

fn entries(value: &Value) -> &[(Value, Value)] {
    let Value::Map(entries) = value else {
        panic!("not a map")
    };
    entries
}

/// `{id, digest}` naming `output` under its own digest domain.
fn reference_to(output: &SchemaValidatedCanonicalProviderOutputV1) -> (String, String) {
    (
        output.coordinate().to_owned(),
        output.domain_framed_digest().to_owned(),
    )
}

fn reference(value: &Value) -> (String, String) {
    let Value::Array(parts) = field(value, "digest") else {
        panic!("digest")
    };
    let Value::Bytes(bytes) = &parts[1] else {
        panic!("digest bytes")
    };
    (
        text(field(value, "id")).to_owned(),
        format!("{}:{}", text(&parts[0]), hex::encode(bytes)),
    )
}

#[test]
fn echo_dpo_1_keeps_its_identity() {
    let artifacts = generate();
    let profile = output(&artifacts, "target-profile.echo-dpo");
    assert_eq!(profile.coordinate(), "echo.dpo@1");
    assert_eq!(profile.domain_framed_digest(), ECHO_DPO_1);
}

#[test]
fn echo_dpo_2_names_its_own_catalog_and_the_ordered_target_ir() {
    let artifacts = generate();
    let profile = output(&artifacts, "target-profile.echo-dpo-2");
    assert_eq!(profile.coordinate(), "echo.dpo@2");
    let value = profile.canonical_value();
    assert_eq!(text(field(value, "id")), "echo.dpo");
    assert_eq!(text(field(value, "version")), "2");
    assert_eq!(text(field(value, "intrinsicNamespace")), "echo.dpo@2");

    let catalog = output(&artifacts, "resource.target-intrinsics-2");
    assert_eq!(catalog.digest_domain(), "echo.dpo.intrinsics/v2");
    assert_eq!(
        reference(field(value, "intrinsics")),
        reference_to(catalog),
        "the profile pins its own catalog"
    );
    let target_ir = output(&artifacts, "resource.target-ir-2");
    assert_eq!(
        reference(field(value, "targetIr")),
        (
            "echo.span-ir/v2".to_owned(),
            target_ir.domain_framed_digest().to_owned()
        ),
    );
    assert_eq!(
        text(field(target_ir.canonical_value(), "domain")),
        "echo.span-ir/v2"
    );
    assert_eq!(
        text(field(target_ir.canonical_value(), "targetProfile")),
        "echo.dpo@2"
    );
}

#[test]
fn echo_dpo_2_shares_every_other_reference_and_doctrine_with_echo_dpo_1() {
    let artifacts = generate();
    let first = output(&artifacts, "target-profile.echo-dpo").canonical_value();
    let second = output(&artifacts, "target-profile.echo-dpo-2").canonical_value();
    let own = ["version", "intrinsicNamespace", "intrinsics", "targetIr"];
    let names: Vec<_> = entries(first).iter().map(|(key, _)| text(key)).collect();
    assert_eq!(
        names,
        entries(second)
            .iter()
            .map(|(key, _)| text(key))
            .collect::<Vec<_>>(),
        "the two profiles have the same fields"
    );
    let mut shared = 0;
    for name in names.iter().filter(|name| !own.contains(name)) {
        assert_eq!(field(first, name), field(second, name), "{name}");
        shared += 1;
    }
    assert_eq!(
        shared,
        names.len() - own.len(),
        "every other field is shared"
    );
}

#[test]
fn echo_dpo_2_catalog_declares_exactly_the_runtime_intrinsics() {
    let artifacts = generate();
    let catalog = output(&artifacts, "resource.target-intrinsics-2").canonical_value();
    assert_eq!(
        text(field(catalog, "apiVersion")),
        "edict.target-profile.intrinsics/v1"
    );
    let declared = entries(field(catalog, "intrinsics"));
    let mut expected: Vec<_> = RUNTIME_INTRINSICS
        .iter()
        .map(|(operation, _, _)| format!("echo.dpo@2.{operation}"))
        .collect();
    expected.sort();
    let mut names: Vec<_> = declared.iter().map(|(key, _)| text(key)).collect();
    names.sort_unstable();
    assert_eq!(names, expected);
    for (operation, kind, failures) in RUNTIME_INTRINSICS {
        let name = format!("echo.dpo@2.{operation}");
        let intrinsic = field(field(catalog, "intrinsics"), &name);
        assert_eq!(text(field(intrinsic, "intrinsicClass")), "effect", "{name}");
        assert_eq!(
            field(intrinsic, "argumentTypes"),
            &Value::Array(vec![Value::Text(format!("{name}.input"))]),
            "{name}"
        );
        assert_eq!(
            text(field(intrinsic, "returnType")),
            format!("{name}.result")
        );
        assert_eq!(field(intrinsic, "typeParameters"), &Value::Array(vec![]));
        assert_eq!(text(field(intrinsic, "effectKind")), kind, "{name}");
        assert_eq!(text(field(intrinsic, "writeClass")), kind, "{name}");
        for template in ["costTemplate", "footprintTemplate"] {
            assert!(
                !text(field(intrinsic, template)).is_empty(),
                "{name} {template}"
            );
        }
        let declared_failures = entries(field(intrinsic, "effectFailures"));
        assert_eq!(declared_failures.len(), failures.len(), "{name}");
        for (failure, authority) in failures {
            let entry = field(field(intrinsic, "effectFailures"), failure);
            assert_eq!(text(field(entry, "authorityClass")), *authority, "{name}");
            assert_eq!(text(field(entry, "payloadType")), format!("{name}.input"));
        }
    }
}
