// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! Deterministic primary artifact generation for the Echo Edict provider.

use echo_registry_api::{
    stable_semantic_operation_id_v1, OpKind, LITTLE_ENDIAN_CODEC_V1_ID,
    SEMANTIC_OPERATION_ID_LAW_V1,
};
use echo_wesley_gen::provider_artifacts::{
    generate_provider_primary_artifacts_v1, ProviderArtifactGenerationErrorKind,
    ProviderPrimaryArtifactsV1, SchemaValidatedCanonicalProviderOutputV1,
};
use echo_wesley_gen::provider_canonical::{
    digest_canonical_value_v1, encode_canonical_cbor_v1, CanonicalValueErrorKind, CanonicalValueV1,
};
use echo_wesley_gen::provider_contract_pack::{
    admit_provider_contract_pack_v1, AdmittedProviderContractPackV1,
};
use echo_wesley_gen::provider_generation::{
    build_provider_generation_input_v1, ProviderGenerationInputV1,
};
use serde_json::{json, Value};
use wesley_core::{compute_generation_artifact_digest_v1, GenerationContractErrorKind};

const SOURCE: &[u8] = include_bytes!("../assets/v1/edict-provider/echo-provider-semantics-v1.json");
const SETTINGS: &[u8] = include_bytes!("../assets/v1/edict-provider/generation-settings-v1.json");
const CONTRACT_CDDL: &[u8] =
    include_bytes!("../assets/v1/edict-provider/contracts/v1/edict-provider-contracts.cddl");
const CONTRACT_MANIFEST: &[u8] =
    include_bytes!("../assets/v1/edict-provider/contracts/v1/manifest.json");

const PRIMARY_ROLES: [&str; 6] = [
    "authority-facts.echo-dpo",
    "authority-facts.echo-lawpack",
    "generated-artifact-profile.echo-dpo-registration",
    "lawpack.echo-dpo",
    "schema.echo-provider-artifacts",
    "target-profile.echo-dpo",
];

const GENERATED_RESOURCE_ROLES: [&str; 14] = [
    "resource.conformance-corpus",
    "resource.lawpack-compatibility",
    "resource.lawpack-exports",
    "resource.lawpack-target-adapter",
    "resource.lawpack-verifier",
    "resource.target-bundle-profile",
    "resource.target-cost-algebra",
    "resource.target-footprint-algebra",
    "resource.target-intrinsics",
    "resource.target-ir",
    "resource.target-lowerer-contract",
    "resource.target-obstruction-taxonomy",
    "resource.target-operation-profiles",
    "resource.target-verifier-contract",
];

fn admitted_pack() -> AdmittedProviderContractPackV1 {
    admit_provider_contract_pack_v1(CONTRACT_CDDL, CONTRACT_MANIFEST)
        .expect("checked Edict provider contract pack is admitted")
}

fn build_input(source: &[u8], pack: &AdmittedProviderContractPackV1) -> ProviderGenerationInputV1 {
    build_provider_generation_input_v1(source, pack, SETTINGS)
        .expect("checked provider generation input builds")
}

fn generate(
    source: &[u8],
    pack: &AdmittedProviderContractPackV1,
) -> (ProviderGenerationInputV1, ProviderPrimaryArtifactsV1) {
    let input = build_input(source, pack);
    let artifacts = generate_provider_primary_artifacts_v1(&input, pack)
        .expect("checked primary provider artifacts generate");
    (input, artifacts)
}

fn reordered_source() -> Vec<u8> {
    let mut source = serde_json::from_slice::<Value>(SOURCE).expect("checked source is JSON");
    for pointer in [
        "/types/0/shape/fields",
        "/types/1/shape/fields",
        "/types/2/shape/fields",
        "/effects/0/guardKinds",
        "/effects/0/failures",
        "/profiles/0/sourceNames",
        "/profiles/0/allowedWriteClasses",
        "/profiles/0/guardKinds",
        "/operations/0/obstructionMappings",
        "/lawpackProjection/acceptedCoreAbis",
        "/lawpackProjection/dependencies",
        "/lawpackProjection/targetAdapters",
        "/lawpackProjection/targetAdapters/0/effects",
        "/targetProfileProjection/acceptedCoreAbis",
        "/targetProfileProjection/generatedArtifactProfileRoles",
        "/targetProfileProjection/acceptedLawpackAdapterAbis",
    ] {
        source
            .pointer_mut(pointer)
            .and_then(Value::as_array_mut)
            .expect("nested set-like field is an array")
            .reverse();
    }
    for key in [
        "authoritySources",
        "types",
        "writeClasses",
        "obstructions",
        "effects",
        "profiles",
        "budgets",
        "capabilities",
        "directAdapters",
        "operations",
        "artifactResources",
        "generatedArtifacts",
        "invocationInputs",
        "invocationOutputs",
        "schemaBindings",
    ] {
        source[key]
            .as_array_mut()
            .expect("set-like declaration family is an array")
            .reverse();
    }
    serde_json::to_vec(&source).expect("reordered source serializes")
}

fn map_field<'a>(value: &'a CanonicalValueV1, field: &str) -> &'a CanonicalValueV1 {
    let CanonicalValueV1::Map(entries) = value else {
        panic!("{field} parent is not a map");
    };
    entries
        .iter()
        .find_map(|(key, value)| {
            (key == &CanonicalValueV1::Text(field.to_owned())).then_some(value)
        })
        .unwrap_or_else(|| panic!("missing map field {field}"))
}

fn map_field_mut<'a>(value: &'a mut CanonicalValueV1, field: &str) -> &'a mut CanonicalValueV1 {
    let CanonicalValueV1::Map(entries) = value else {
        panic!("{field} parent is not a map");
    };
    entries
        .iter_mut()
        .find_map(|(key, value)| {
            (key == &CanonicalValueV1::Text(field.to_owned())).then_some(value)
        })
        .unwrap_or_else(|| panic!("missing map field {field}"))
}

fn map_keys(value: &CanonicalValueV1) -> Vec<&str> {
    let CanonicalValueV1::Map(entries) = value else {
        panic!("value is not a map");
    };
    entries
        .iter()
        .map(|(key, _)| {
            let CanonicalValueV1::Text(key) = key else {
                panic!("map key is not text");
            };
            key.as_str()
        })
        .collect()
}

fn text_value(value: &CanonicalValueV1) -> &str {
    let CanonicalValueV1::Text(value) = value else {
        panic!("value is not text");
    };
    value
}

fn integer_value(value: &CanonicalValueV1) -> i128 {
    let CanonicalValueV1::Integer(value) = value else {
        panic!("value is not an integer");
    };
    *value
}

fn typed_digest(value: &CanonicalValueV1) -> String {
    let CanonicalValueV1::Array(parts) = value else {
        panic!("digest is not an array");
    };
    assert_eq!(parts.len(), 2);
    assert_eq!(text_value(&parts[0]), "sha256");
    let CanonicalValueV1::Bytes(bytes) = &parts[1] else {
        panic!("digest payload is not bytes");
    };
    assert_eq!(bytes.len(), 32);
    format!("sha256:{}", hex::encode(bytes))
}

fn resource_ref_digest(value: &CanonicalValueV1) -> String {
    typed_digest(map_field(value, "digest"))
}

fn changed_artifact_roles<'a>(
    baseline: &'a ProviderPrimaryArtifactsV1,
    changed: &'a ProviderPrimaryArtifactsV1,
) -> Vec<&'a str> {
    baseline
        .artifacts()
        .iter()
        .filter_map(|artifact| {
            let changed = changed
                .artifact(artifact.role())
                .expect("changed closure retains every primary role");
            (artifact.canonical_bytes() != changed.canonical_bytes()).then_some(artifact.role())
        })
        .collect()
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

#[test]
fn primary_generation_is_byte_identical_digest_closed_and_root_validated() {
    let pack = admitted_pack();
    let (_, first) = generate(SOURCE, &pack);
    let (_, second) = generate(SOURCE, &pack);

    assert_eq!(first, second);
    assert_eq!(
        first
            .projection_roles()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        PRIMARY_ROLES
    );
    assert_eq!(first.artifacts().len(), 5);
    assert_eq!(first.resources().len(), 14);
    assert_eq!(
        first
            .resources()
            .iter()
            .map(SchemaValidatedCanonicalProviderOutputV1::role)
            .collect::<Vec<_>>(),
        GENERATED_RESOURCE_ROLES
    );
    assert_eq!(first.schema().role(), "schema.echo-provider-artifacts");
    assert_eq!(
        first.schema().coordinate(),
        "echo.provider-artifacts.cddl@1"
    );
    assert!(first.schema().bytes().starts_with(CONTRACT_CDDL));
    assert!(std::str::from_utf8(first.schema().bytes())
        .expect("generated provider CDDL is UTF-8")
        .contains("generated-artifact-profile ="));
    assert_eq!(
        first.schema().content_reference().digest,
        compute_generation_artifact_digest_v1(first.schema().bytes())
    );
    assert_eq!(
        first.schema().content_reference().coordinate,
        first.schema().coordinate()
    );

    for output in first.artifacts().iter().chain(first.resources()) {
        let admitted = first
            .schema()
            .validate_output(output)
            .expect("output satisfies its owning root");
        assert_eq!(
            encode_canonical_cbor_v1(&admitted).expect("admitted output re-encodes"),
            output.canonical_bytes()
        );
        assert_eq!(
            digest_canonical_value_v1(output.digest_domain(), &admitted)
                .expect("admitted output domain-digests"),
            output.domain_framed_digest()
        );
        assert_eq!(
            output.content_reference().digest,
            compute_generation_artifact_digest_v1(output.canonical_bytes())
        );
        assert_eq!(output.content_reference().coordinate, output.coordinate());
        assert_ne!(
            output.domain_framed_digest(),
            output.content_reference().digest
        );
    }

    for (role, contract) in [
        ("lawpack.echo-dpo", "lawpack-manifest"),
        ("target-profile.echo-dpo", "target-profile-manifest"),
        ("authority-facts.echo-dpo", "authority-facts"),
        ("authority-facts.echo-lawpack", "authority-facts"),
    ] {
        pack.validate_contract_bytes(
            contract,
            first
                .artifact(role)
                .expect("Edict-owned primary role exists")
                .canonical_bytes(),
        )
        .expect("primary output satisfies the independently admitted Edict root");
    }
    for (role, contract) in [
        ("resource.lawpack-exports", "lawpack-exports"),
        ("resource.target-intrinsics", "target-profile-intrinsics"),
        (
            "resource.target-operation-profiles",
            "target-profile-operation-profiles",
        ),
    ] {
        pack.validate_contract_bytes(
            contract,
            first
                .resource(role)
                .expect("Edict-owned generated resource exists")
                .canonical_bytes(),
        )
        .expect("generated resource satisfies the independently admitted Edict root");
    }
}

#[test]
fn computed_digest_edges_and_authority_partition_are_exact() {
    let pack = admitted_pack();
    let (_, generated) = generate(SOURCE, &pack);
    let registration = output(
        &generated,
        "generated-artifact-profile.echo-dpo-registration",
    );
    let target = output(&generated, "target-profile.echo-dpo");
    let lawpack = output(&generated, "lawpack.echo-dpo");

    let generated_profiles = map_field(target.canonical_value(), "generatedArtifactProfiles");
    let CanonicalValueV1::Array(generated_profiles) = generated_profiles else {
        panic!("generated artifact profiles is not an array");
    };
    assert_eq!(generated_profiles.len(), 1);
    assert_eq!(
        resource_ref_digest(&generated_profiles[0]),
        registration.domain_framed_digest()
    );

    let CanonicalValueV1::Array(adapters) = map_field(lawpack.canonical_value(), "targetAdapters")
    else {
        panic!("target adapters is not an array");
    };
    assert_eq!(adapters.len(), 1);
    let accepted_target = map_field(&adapters[0], "acceptedTargetProfile");
    assert_eq!(text_value(map_field(accepted_target, "id")), "echo.dpo@1");
    assert_eq!(
        resource_ref_digest(accepted_target),
        target.domain_framed_digest()
    );

    for (field, coordinate) in [
        ("sandbox", "edict.wasm-component/v1"),
        ("fuelModel", "edict.fuel/v1"),
        ("canonicalEncodingRules", "edict.canonical-cbor/v1"),
        ("diagnosticAbi", "edict.diagnostics/v1"),
        ("deterministicExecution", "edict.determinism/v1"),
    ] {
        let reference = map_field(target.canonical_value(), field);
        let resource = pack
            .resource(coordinate)
            .expect("external contract-pack resource is admitted");
        assert_eq!(text_value(map_field(reference, "id")), coordinate);
        assert_eq!(
            typed_digest(map_field(reference, "digest")),
            resource.domain_framed_digest()
        );
        assert_ne!(
            typed_digest(map_field(reference, "digest")),
            format!("sha256:{}", resource.raw_sha256())
        );
    }

    for (role, source_kind, source_artifact) in [
        (
            "authority-facts.echo-dpo",
            "targetProfile",
            "target-profile.echo-dpo",
        ),
        (
            "authority-facts.echo-lawpack",
            "lawpack",
            "lawpack.echo-dpo",
        ),
    ] {
        let facts = output(&generated, role);
        let source = map_field(facts.canonical_value(), "source");
        assert_eq!(text_value(map_field(source, "kind")), source_kind);
        assert_eq!(
            typed_digest(map_field(source, "digest")),
            output(&generated, source_artifact).domain_framed_digest()
        );
    }

    let target_facts = output(&generated, "authority-facts.echo-dpo").canonical_value();
    assert_eq!(
        map_keys(map_field(target_facts, "operationProfiles")),
        ["p.effectful"]
    );
    assert_eq!(
        map_keys(map_field(target_facts, "effectWriteClasses")),
        ["target.replace"]
    );
    assert!(map_keys(map_field(target_facts, "budgets")).is_empty());

    let lawpack_facts = output(&generated, "authority-facts.echo-lawpack").canonical_value();
    assert!(map_keys(map_field(lawpack_facts, "operationProfiles")).is_empty());
    assert!(map_keys(map_field(lawpack_facts, "effectWriteClasses")).is_empty());
    assert_eq!(map_keys(map_field(lawpack_facts, "budgets")), ["p.tiny"]);
    let budget = map_field(map_field(lawpack_facts, "budgets"), "p.tiny");
    assert_eq!(integer_value(map_field(budget, "maxSteps")), 8);
}

#[test]
fn source_set_reordering_moves_input_evidence_not_primary_semantics() {
    let pack = admitted_pack();
    let (baseline_input, baseline) = generate(SOURCE, &pack);
    let reordered_bytes = reordered_source();
    let (reordered_input, reordered) = generate(&reordered_bytes, &pack);

    assert_eq!(
        baseline_input.semantic_source(),
        reordered_input.semantic_source()
    );
    assert_ne!(baseline_input.digest(), reordered_input.digest());
    assert_eq!(baseline.generation_input_digest(), baseline_input.digest());
    assert_eq!(
        reordered.generation_input_digest(),
        reordered_input.digest()
    );
    assert_ne!(
        baseline.generation_input_digest(),
        reordered.generation_input_digest()
    );
    assert_eq!(baseline.projection_roles(), reordered.projection_roles());
    assert_eq!(baseline.artifacts(), reordered.artifacts());
    assert_eq!(baseline.resources(), reordered.resources());
    assert_eq!(baseline.schema(), reordered.schema());
}

#[test]
fn budget_change_moves_only_lawpack_authority_facts() {
    let pack = admitted_pack();
    let (_, baseline) = generate(SOURCE, &pack);
    let mut source = serde_json::from_slice::<Value>(SOURCE).expect("checked source is JSON");
    source["budgets"][0]["maxSteps"] = json!(9);
    let changed_source = serde_json::to_vec(&source).expect("changed source serializes");
    let (_, changed) = generate(&changed_source, &pack);

    assert_eq!(
        changed_artifact_roles(&baseline, &changed),
        ["authority-facts.echo-lawpack"]
    );
    assert_eq!(baseline.resources(), changed.resources());
    assert_eq!(baseline.schema().bytes(), changed.schema().bytes());
    assert_ne!(
        baseline
            .artifact("authority-facts.echo-lawpack")
            .expect("baseline lawpack facts exist")
            .domain_framed_digest(),
        changed
            .artifact("authority-facts.echo-lawpack")
            .expect("changed lawpack facts exist")
            .domain_framed_digest()
    );
}

#[test]
fn generated_profile_carries_the_echo_owned_semantic_operation_id() {
    let pack = admitted_pack();
    let (_, generated) = generate(SOURCE, &pack);
    let registration = generated
        .artifact("generated-artifact-profile.echo-dpo-registration")
        .expect("registration profile exists")
        .canonical_value();

    assert_eq!(
        text_value(map_field(registration, "operationIdLaw")),
        SEMANTIC_OPERATION_ID_LAW_V1
    );
    let operation = map_field(map_field(registration, "operations"), "a.b@1.t");
    assert_eq!(
        integer_value(map_field(operation, "operationId")),
        i128::from(stable_semantic_operation_id_v1(OpKind::Mutation, "a.b@1.t"))
    );
}

#[test]
fn generated_profile_binds_the_exact_echo_value_codec() {
    let pack = admitted_pack();
    let (_, generated) = generate(SOURCE, &pack);
    let registration = generated
        .artifact("generated-artifact-profile.echo-dpo-registration")
        .expect("registration profile exists")
        .canonical_value();
    let operation = map_field(map_field(registration, "operations"), "a.b@1.t");

    assert_eq!(
        text_value(map_field(operation, "valueCodec")),
        LITTLE_ENDIAN_CODEC_V1_ID
    );
}

#[test]
fn semantic_operation_ids_fail_closed_on_reserved_and_colliding_results() {
    let pack = admitted_pack();

    for coordinate in ["a.b@1.cs2esoh", "a.b@1.0ctdaa8i"] {
        let mut reserved = serde_json::from_slice::<Value>(SOURCE).expect("checked source is JSON");
        reserved["operations"][0]["identity"]["coordinate"] = json!(coordinate);
        let reserved_bytes = serde_json::to_vec(&reserved).expect("reserved-id source serializes");
        let reserved_input = build_input(&reserved_bytes, &pack);
        let reserved_error = generate_provider_primary_artifacts_v1(&reserved_input, &pack)
            .expect_err("every Echo-reserved protocol id must refuse");
        assert_eq!(
            (
                reserved_error.kind(),
                reserved_error.subject(),
                reserved_error.reference(),
            ),
            (
                ProviderArtifactGenerationErrorKind::ReservedOperationId,
                coordinate,
                SEMANTIC_OPERATION_ID_LAW_V1,
            )
        );
    }

    let mut colliding = serde_json::from_slice::<Value>(SOURCE).expect("checked source is JSON");
    let mut second = colliding["operations"][0].clone();
    colliding["operations"][0]["identity"]["coordinate"] = json!("a.b@1.3n8plpiea7");
    second["identity"]["coordinate"] = json!("a.b@1.lmdfcgrl2h");
    colliding["operations"]
        .as_array_mut()
        .expect("operations are an array")
        .push(second);
    let colliding_bytes = serde_json::to_vec(&colliding).expect("colliding-id source serializes");
    let colliding_input = build_input(&colliding_bytes, &pack);
    let collision_error = generate_provider_primary_artifacts_v1(&colliding_input, &pack)
        .expect_err("package-local operation-id collisions must refuse");
    assert_eq!(
        (
            collision_error.kind(),
            collision_error.subject(),
            collision_error.reference(),
        ),
        (
            ProviderArtifactGenerationErrorKind::OperationIdCollision,
            "a.b@1.lmdfcgrl2h",
            "a.b@1.3n8plpiea7",
        )
    );
}

#[test]
fn revelation_profiles_generate_bounded_observer_invocations() {
    let pack = admitted_pack();
    let mut source = serde_json::from_slice::<Value>(SOURCE).expect("checked source is JSON");
    source["writeClasses"][0]["identity"]["coordinate"] = json!("read");
    source["profiles"][0]["allowedWriteClasses"][0] = json!("read");
    source["profiles"][0]["opticTemplate"]["opticKind"] = json!("revelation");
    source["profiles"][0]["opticTemplate"]["boundaryKind"] = json!("projection");
    source["effects"][0]["effectKindHint"] = json!("read");
    source["capabilities"][0]["effectKind"] = json!("read");
    source["capabilities"][0]["writeClass"] = json!("read");
    source["capabilities"][0]["semanticDischarge"]["effectKindHint"] = json!("read");
    let observer_source = serde_json::to_vec(&source).expect("observer semantic source serializes");
    let (_, generated) = generate(&observer_source, &pack);

    let registration = generated
        .artifact("generated-artifact-profile.echo-dpo-registration")
        .expect("registration profile exists")
        .canonical_value();
    let operation = map_field(map_field(registration, "operations"), "a.b@1.t");
    assert_eq!(
        text_value(map_field(operation, "invocationKind")),
        "observer"
    );
    assert_eq!(
        integer_value(map_field(operation, "operationId")),
        i128::from(stable_semantic_operation_id_v1(OpKind::Query, "a.b@1.t"))
    );
    assert_ne!(
        integer_value(map_field(operation, "operationId")),
        i128::from(stable_semantic_operation_id_v1(OpKind::Mutation, "a.b@1.t"))
    );

    let profiles = map_field(
        generated
            .resource("resource.target-operation-profiles")
            .expect("target operation profiles exist")
            .canonical_value(),
        "profiles",
    );
    let optic = map_field(
        map_field(profiles, "continuum.profile.write/v1"),
        "opticTemplate",
    );
    assert_eq!(text_value(map_field(optic, "opticKind")), "revelation");
    assert_eq!(text_value(map_field(optic, "boundaryKind")), "projection");

    let target_facts = generated
        .artifact("authority-facts.echo-dpo")
        .expect("target facts exist")
        .canonical_value();
    assert_eq!(
        text_value(map_field(
            map_field(target_facts, "effectWriteClasses"),
            "target.replace"
        )),
        "read"
    );
}

#[test]
fn primary_closure_contains_only_declared_generic_provider_semantics() {
    let pack = admitted_pack();
    let (_, generated) = generate(SOURCE, &pack);

    assert_eq!(
        generated
            .projection_roles()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        PRIMARY_ROLES
    );
    assert!(generated
        .projection_roles()
        .iter()
        .all(|role| role != "provider-manifest.echo" && !role.contains("component")));

    let exports = generated
        .resource("resource.lawpack-exports")
        .expect("lawpack exports exist")
        .canonical_value();
    let CanonicalValueV1::Array(effects) = map_field(exports, "effects") else {
        panic!("effects is not an array");
    };
    assert_eq!(
        effects
            .iter()
            .map(|effect| text_value(map_field(effect, "coordinate")))
            .collect::<Vec<_>>(),
        ["target.replace"]
    );

    let intrinsics = map_field(
        generated
            .resource("resource.target-intrinsics")
            .expect("target intrinsics exist")
            .canonical_value(),
        "intrinsics",
    );
    assert_eq!(map_keys(intrinsics), ["echo.dpo@1.replace"]);

    let profiles = map_field(
        generated
            .resource("resource.target-operation-profiles")
            .expect("operation profiles exist")
            .canonical_value(),
        "profiles",
    );
    assert_eq!(map_keys(profiles), ["continuum.profile.write/v1"]);

    let registration = generated
        .artifact("generated-artifact-profile.echo-dpo-registration")
        .expect("generated registration profile exists")
        .canonical_value();
    let operations = map_field(registration, "operations");
    assert_eq!(map_keys(operations), ["a.b@1.t"]);
    assert_eq!(
        text_value(map_field(
            map_field(operations, "a.b@1.t"),
            "invocationKind"
        )),
        "mutation"
    );
    let operation = map_field(operations, "a.b@1.t");
    for (field, expected) in [
        ("inputType", "a.b@1.Input"),
        ("outputType", "a.b@1.Output"),
        ("effect", "target.replace"),
        ("operationProfile", "continuum.profile.write/v1"),
        ("opticContract", "replace-point"),
        ("budget", "p.tiny"),
    ] {
        assert_eq!(text_value(map_field(operation, field)), expected);
    }
    let implementation = map_field(operation, "implementation");
    assert_eq!(text_value(map_field(implementation, "kind")), "native");
    assert_eq!(
        text_value(map_field(implementation, "coordinate")),
        "echo.dpo@1.replace"
    );
    assert_eq!(
        text_value(map_field(
            map_field(operation, "obstructionMappings"),
            "rejected"
        )),
        "domain.WriteRejected"
    );

    for role in [
        "resource.lawpack-verifier",
        "resource.target-lowerer-contract",
        "resource.target-verifier-contract",
    ] {
        assert_eq!(
            text_value(map_field(
                output(&generated, role).canonical_value(),
                "class"
            )),
            "declarative"
        );
    }
}

#[test]
fn echo_owned_outputs_emit_their_declared_schema_api() {
    let pack = admitted_pack();
    let (_, generated) = generate(SOURCE, &pack);

    for role in [
        "generated-artifact-profile.echo-dpo-registration",
        "resource.conformance-corpus",
        "resource.lawpack-compatibility",
        "resource.lawpack-target-adapter",
        "resource.lawpack-verifier",
        "resource.target-bundle-profile",
        "resource.target-cost-algebra",
        "resource.target-footprint-algebra",
        "resource.target-ir",
        "resource.target-lowerer-contract",
        "resource.target-obstruction-taxonomy",
        "resource.target-verifier-contract",
    ] {
        let output = output(&generated, role);
        assert_eq!(
            text_value(map_field(output.canonical_value(), "apiVersion")),
            output.schema_contract(),
            "{role} must not validate against a generator-invented API marker"
        );
    }
}

#[test]
fn declarative_conformance_resource_requires_at_least_one_case_contract() {
    let pack = admitted_pack();
    let (_, generated) = generate(SOURCE, &pack);
    let corpus = generated
        .resource("resource.conformance-corpus")
        .expect("conformance resource exists");
    let assert_root_rejected = |value: &CanonicalValueV1, reason: &str| {
        let bytes = encode_canonical_cbor_v1(value).expect("mutated value remains canonical CBOR");
        let error = generated
            .schema()
            .validate_root_bytes(corpus.owning_root(), &bytes)
            .expect_err(reason);
        assert_eq!(
            (error.kind(), error.subject(), error.reference()),
            (
                ProviderArtifactGenerationErrorKind::OwningRootRejected,
                "echo-provider-conformance-corpus",
                "generated-provider-schema"
            )
        );
    };

    let mut empty = corpus.canonical_value().clone();
    *map_field_mut(&mut empty, "cases") = CanonicalValueV1::Map(Vec::new());
    assert_root_rejected(
        &empty,
        "the declarative corpus cannot omit every required case contract",
    );

    let mut malformed_outcome = corpus.canonical_value().clone();
    *map_field_mut(
        map_field_mut(
            map_field_mut(
                map_field_mut(&mut malformed_outcome, "cases"),
                "package-parity",
            ),
            "requiredOutcome",
        ),
        "disposition",
    ) = CanonicalValueV1::Text("passed".to_owned());
    assert_root_rejected(
        &malformed_outcome,
        "a result-shaped disposition is not a declared conformance obligation",
    );

    let mut fabricated_evidence = corpus.canonical_value().clone();
    let CanonicalValueV1::Map(case_entries) = map_field_mut(
        map_field_mut(&mut fabricated_evidence, "cases"),
        "package-parity",
    ) else {
        panic!("package-parity case is a map");
    };
    case_entries.push((
        CanonicalValueV1::Text("evidence".to_owned()),
        CanonicalValueV1::Null,
    ));
    assert_root_rejected(
        &fabricated_evidence,
        "the declarative resource cannot claim execution evidence",
    );
}

#[test]
fn declarative_conformance_resource_names_the_reviewed_case_contracts() {
    let pack = admitted_pack();
    let (_, generated) = generate(SOURCE, &pack);
    let corpus = generated
        .resource("resource.conformance-corpus")
        .expect("conformance resource exists");
    let cases = map_field(corpus.canonical_value(), "cases");
    let mut case_ids = map_keys(cases);
    case_ids.sort_unstable();

    assert_eq!(case_ids, ["package-parity"]);

    let case = map_field(cases, "package-parity");
    assert_eq!(map_keys(case), ["crossing", "stimulus", "requiredOutcome"]);
    assert_eq!(text_value(map_field(case, "crossing")), "pipeline");
    assert_eq!(text_value(map_field(case, "stimulus")), "baseline");
    let required_outcome = map_field(case, "requiredOutcome");
    assert_eq!(
        text_value(map_field(required_outcome, "disposition")),
        "accepted"
    );
    assert_eq!(
        text_value(map_field(required_outcome, "contract")),
        "completed-package-parity"
    );
}

#[test]
fn generated_profile_type_catalog_is_structurally_admitted() {
    let pack = admitted_pack();
    let (_, generated) = generate(SOURCE, &pack);
    let profile = generated
        .artifact("generated-artifact-profile.echo-dpo-registration")
        .expect("generated artifact profile exists");
    let mut malformed = profile.canonical_value().clone();
    *map_field_mut(map_field_mut(&mut malformed, "types"), "a.b@1.Id") = CanonicalValueV1::Null;
    let malformed_bytes =
        encode_canonical_cbor_v1(&malformed).expect("malformed value remains canonical CBOR");

    let error = generated
        .schema()
        .validate_root_bytes(profile.owning_root(), &malformed_bytes)
        .expect_err("an untyped catalog value cannot pass the generated profile root");
    assert_eq!(
        (error.kind(), error.subject(), error.reference()),
        (
            ProviderArtifactGenerationErrorKind::OwningRootRejected,
            "generated-artifact-profile",
            "generated-provider-schema"
        )
    );
}

#[test]
fn direct_adapter_routes_are_complete_generic_semantics() {
    let pack = admitted_pack();
    let mut source = serde_json::from_slice::<Value>(SOURCE).expect("checked source is JSON");
    let mut native_effect = source["effects"][0].clone();
    native_effect["identity"]["coordinate"] = json!("target.native-replace");
    source["effects"]
        .as_array_mut()
        .expect("effects are an array")
        .push(native_effect);
    source["capabilities"][0]["effect"] = json!("target.native-replace");
    source["directAdapters"] = json!([{
        "identity": {
            "coordinate": "echo.dpo@1.replace-adapter",
            "domain": "echo.edict-provider/direct-adapter/v1",
            "authority": "echo.provider-target-metadata@1"
        },
        "consumesEffect": "target.replace",
        "capability": "echo.dpo@1.replace",
        "emitsEffects": []
    }]);
    source["operations"][0]["implementation"] = json!({
        "kind": "directAdapter",
        "adapter": "echo.dpo@1.replace-adapter"
    });
    source["lawpackProjection"]["targetAdapters"][0]["effects"] =
        json!(["target.native-replace", "target.replace"]);
    let adapter_source =
        serde_json::to_vec(&source).expect("direct-adapter semantic source serializes");
    let (_, generated) = generate(&adapter_source, &pack);

    for role in [
        "resource.lawpack-target-adapter",
        "resource.target-lowerer-contract",
    ] {
        let implementations = map_field(
            generated
                .resource(role)
                .expect("implementation resource exists")
                .canonical_value(),
            "effectImplementations",
        );
        let direct = map_field(implementations, "target.replace");
        assert_eq!(text_value(map_field(direct, "kind")), "directAdapter");
        assert_eq!(
            text_value(map_field(direct, "adapter")),
            "echo.dpo@1.replace-adapter"
        );
        assert_eq!(
            text_value(map_field(direct, "capability")),
            "echo.dpo@1.replace"
        );
        assert_eq!(text_value(map_field(direct, "writeClass")), "replace");

        let native = map_field(implementations, "target.native-replace");
        assert_eq!(text_value(map_field(native, "kind")), "native");
        assert_eq!(
            text_value(map_field(native, "capability")),
            "echo.dpo@1.replace"
        );
        assert_eq!(text_value(map_field(native, "writeClass")), "replace");
    }

    let write_classes = map_field(
        generated
            .artifact("authority-facts.echo-dpo")
            .expect("target facts exist")
            .canonical_value(),
        "effectWriteClasses",
    );
    assert_eq!(
        text_value(map_field(write_classes, "target.replace")),
        "replace"
    );
    assert_eq!(
        text_value(map_field(write_classes, "target.native-replace")),
        "replace"
    );
}

#[test]
fn lawpack_target_adapter_does_not_overclaim_proof_only_capabilities() {
    let pack = admitted_pack();
    let mut source = serde_json::from_slice::<Value>(SOURCE).expect("checked source is JSON");
    source["writeClasses"]
        .as_array_mut()
        .expect("write classes are an array")
        .push(json!({
            "identity": {
                "coordinate": "read",
                "domain": "echo.edict-provider/write-class/v1",
                "authority": "echo.provider-target-metadata@1"
            }
        }));
    let mut proof_effect = source["effects"][0].clone();
    proof_effect["identity"]["coordinate"] = json!("target.proof");
    proof_effect["executionClass"] = json!("proofOnly");
    proof_effect["effectKindHint"] = json!("read");
    proof_effect["guardKinds"] = json!([]);
    proof_effect["footprintObligation"] = json!("target.proof.footprint");
    proof_effect["costObligation"] = json!("target.proof.cost");
    proof_effect["guardSupport"] = json!(false);
    source["effects"]
        .as_array_mut()
        .expect("effects are an array")
        .push(proof_effect);
    let mut proof_capability = source["capabilities"][0].clone();
    proof_capability["identity"]["coordinate"] = json!("echo.dpo@1.proof");
    proof_capability["effect"] = json!("target.proof");
    proof_capability["effectKind"] = json!("read");
    proof_capability["writeClass"] = json!("read");
    proof_capability["guardSupport"] = json!(false);
    proof_capability["footprintTemplate"] = json!("target.proof.footprint");
    proof_capability["costTemplate"] = json!("target.proof.cost");
    proof_capability["semanticDischarge"]["effectKindHint"] = json!("read");
    proof_capability["semanticDischarge"]["footprintObligation"] = json!("target.proof.footprint");
    proof_capability["semanticDischarge"]["costObligation"] = json!("target.proof.cost");
    proof_capability["canParticipateInAtomicGuard"] = json!(false);
    source["capabilities"]
        .as_array_mut()
        .expect("capabilities are an array")
        .push(proof_capability);
    let proof_source =
        serde_json::to_vec(&source).expect("proof-capability semantic source serializes");
    let (_, generated) = generate(&proof_source, &pack);

    let adapter_implementations = map_field(
        generated
            .resource("resource.lawpack-target-adapter")
            .expect("lawpack target adapter exists")
            .canonical_value(),
        "effectImplementations",
    );
    assert_eq!(map_keys(adapter_implementations), ["target.replace"]);

    let lowerer_implementations = map_field(
        generated
            .resource("resource.target-lowerer-contract")
            .expect("target lowerer exists")
            .canonical_value(),
        "effectImplementations",
    );
    assert_eq!(
        text_value(map_field(
            map_field(lowerer_implementations, "target.proof"),
            "kind"
        )),
        "native"
    );
}

#[test]
fn lawpack_verifier_preserves_operation_local_obstruction_mappings() {
    let pack = admitted_pack();
    let mut source = serde_json::from_slice::<Value>(SOURCE).expect("checked source is JSON");
    source["obstructions"]
        .as_array_mut()
        .expect("obstructions are an array")
        .push(json!({
            "identity": {
                "coordinate": "domain.WriteRejectedAlternate",
                "domain": "echo.edict-provider/obstruction/v1",
                "authority": "echo.provider-semantic-declaration@1"
            },
            "authorityClass": "domainMappable",
            "payloadSchema": "domain.WriteRejected.Payload"
        }));
    let mut operation = source["operations"][0].clone();
    operation["identity"]["coordinate"] = json!("a.b@1.u");
    operation["obstructionMappings"][0]["obstruction"] = json!("domain.WriteRejectedAlternate");
    source["operations"]
        .as_array_mut()
        .expect("operations are an array")
        .push(operation);
    let two_operation_source =
        serde_json::to_vec(&source).expect("two-operation semantic source serializes");
    let (_, generated) = generate(&two_operation_source, &pack);

    let verifier = generated
        .resource("resource.lawpack-verifier")
        .expect("lawpack verifier exists")
        .canonical_value();
    let operations = map_field(verifier, "operationObstructions");
    for (operation, obstruction) in [
        ("a.b@1.t", "domain.WriteRejected"),
        ("a.b@1.u", "domain.WriteRejectedAlternate"),
    ] {
        let mapping = map_field(operations, operation);
        assert_eq!(text_value(map_field(mapping, "effect")), "target.replace");
        assert_eq!(
            text_value(map_field(map_field(mapping, "failureMappings"), "rejected")),
            obstruction
        );
    }
}

#[test]
fn optic_contract_changes_move_the_bound_generated_contracts() {
    let pack = admitted_pack();
    let (_, baseline) = generate(SOURCE, &pack);
    let mut source = serde_json::from_slice::<Value>(SOURCE).expect("checked source is JSON");
    source["profiles"][0]["opticContract"] = json!("replace-point-v2");
    let changed_source =
        serde_json::to_vec(&source).expect("changed optic-contract source serializes");
    let (_, changed) = generate(&changed_source, &pack);

    let operation = map_field(
        map_field(
            changed
                .artifact("generated-artifact-profile.echo-dpo-registration")
                .expect("changed registration profile exists")
                .canonical_value(),
            "operations",
        ),
        "a.b@1.t",
    );
    assert_eq!(
        text_value(map_field(operation, "opticContract")),
        "replace-point-v2"
    );
    for role in [
        "resource.target-lowerer-contract",
        "resource.target-verifier-contract",
    ] {
        let contracts = map_field(
            changed
                .resource(role)
                .expect("changed target contract exists")
                .canonical_value(),
            "opticContracts",
        );
        assert_eq!(
            text_value(map_field(contracts, "continuum.profile.write/v1")),
            "replace-point-v2"
        );
        assert_ne!(
            baseline
                .resource(role)
                .expect("baseline target contract exists")
                .domain_framed_digest(),
            changed
                .resource(role)
                .expect("changed target contract exists")
                .domain_framed_digest()
        );
    }
}

#[test]
fn emitted_content_references_reject_invalid_wesley_coordinates() {
    let pack = admitted_pack();
    let mut source = serde_json::from_slice::<Value>(SOURCE).expect("checked source is JSON");
    let declaration = source["generatedArtifacts"]
        .as_array_mut()
        .expect("generated artifacts are an array")
        .iter_mut()
        .find(|artifact| {
            artifact["role"] == json!("generated-artifact-profile.echo-dpo-registration")
        })
        .expect("registration profile declaration exists");
    declaration["coordinate"] = json!(" echo.dpo.registration/v1");
    let invalid_source =
        serde_json::to_vec(&source).expect("invalid-coordinate semantic source serializes");
    let input = build_input(&invalid_source, &pack);

    let error = generate_provider_primary_artifacts_v1(&input, &pack)
        .expect_err("Wesley must reject an invalid emitted content coordinate");
    assert_eq!(
        (
            error.kind(),
            error.wesley_contract_kind(),
            error.subject(),
            error.reference(),
        ),
        (
            ProviderArtifactGenerationErrorKind::WesleyContractRejected,
            Some(GenerationContractErrorKind::InvalidCoordinate),
            " echo.dpo.registration/v1",
            "WESLEY_GENERATION_INVALID_COORDINATE",
        )
    );
}

#[test]
fn generated_schema_distinguishes_canonical_wire_from_root_admission() {
    let pack = admitted_pack();
    let (_, generated) = generate(SOURCE, &pack);
    let profile = generated
        .artifact("generated-artifact-profile.echo-dpo-registration")
        .expect("generated artifact profile exists");

    let mut schema_invalid = profile.canonical_value().clone();
    *map_field_mut(
        map_field_mut(map_field_mut(&mut schema_invalid, "operations"), "a.b@1.t"),
        "invocationKind",
    ) = CanonicalValueV1::Text("unknown".to_owned());
    let schema_invalid_bytes = encode_canonical_cbor_v1(&schema_invalid)
        .expect("schema-invalid value remains canonical CBOR");
    let schema_error = generated
        .schema()
        .validate_root_bytes(profile.owning_root(), &schema_invalid_bytes)
        .expect_err("canonical decoding alone is not owning-root admission");
    assert_eq!(
        (
            schema_error.kind(),
            schema_error.subject(),
            schema_error.reference(),
            schema_error.canonical_value_kind(),
        ),
        (
            ProviderArtifactGenerationErrorKind::OwningRootRejected,
            "generated-artifact-profile",
            "generated-provider-schema",
            None,
        )
    );

    for invalid_operation_id in [
        i128::from(u32::MAX - 1),
        i128::from(u32::MAX),
        i128::from(u32::MAX) + 1,
    ] {
        let mut invalid_id = profile.canonical_value().clone();
        *map_field_mut(
            map_field_mut(map_field_mut(&mut invalid_id, "operations"), "a.b@1.t"),
            "operationId",
        ) = CanonicalValueV1::Integer(invalid_operation_id);
        let invalid_id_bytes = encode_canonical_cbor_v1(&invalid_id)
            .expect("invalid operation id remains canonical CBOR");
        let invalid_id_error = generated
            .schema()
            .validate_root_bytes(profile.owning_root(), &invalid_id_bytes)
            .expect_err("the owning root must constrain operation ids to application u32 values");
        assert_eq!(
            invalid_id_error.kind(),
            ProviderArtifactGenerationErrorKind::OwningRootRejected
        );
    }

    let mut trailing = profile.canonical_bytes().to_vec();
    trailing.push(0xf6);
    let canonical_error = generated
        .schema()
        .validate_root_bytes(profile.owning_root(), &trailing)
        .expect_err("trailing canonical data is rejected before schema admission");
    assert_eq!(
        (
            canonical_error.kind(),
            canonical_error.subject(),
            canonical_error.reference(),
            canonical_error.canonical_value_kind(),
        ),
        (
            ProviderArtifactGenerationErrorKind::CanonicalEncodingFailed,
            "generated-artifact-profile",
            "edict.canonical-cbor/v1",
            Some(CanonicalValueErrorKind::TrailingData),
        )
    );
}
