// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! Exact-material provenance for deterministic Echo provider generation.

use echo_wesley_gen::provider_artifacts::{
    generate_provider_primary_artifacts_v1, ProviderPrimaryArtifactsV1,
};
use echo_wesley_gen::provider_canonical::CanonicalValueV1;
use echo_wesley_gen::provider_contract_pack::{
    admit_provider_contract_pack_v1, AdmittedProviderContractPackV1,
};
use echo_wesley_gen::provider_generation::{
    build_provider_generation_input_v1, ProviderGenerationInputV1,
};
use echo_wesley_gen::provider_provenance::{
    generate_provider_generation_provenance_v1, ProviderGeneratorMaterialV1,
    ProviderProvenanceErrorKind,
};
use serde_json::{json, Value};
use wesley_core::{
    compute_generation_artifact_digest_v1, GenerationContractErrorKind,
    GenerationProvenanceManifestV1,
};

const SOURCE: &[u8] =
    include_bytes!("../../../schemas/edict-provider/echo-provider-semantics-v1.json");
const SETTINGS: &[u8] =
    include_bytes!("../../../schemas/edict-provider/generation-settings-v1.json");
const CONTRACT_CDDL: &[u8] =
    include_bytes!("../../../schemas/edict-provider/contracts/v1/edict-provider-contracts.cddl");
const CONTRACT_MANIFEST: &[u8] =
    include_bytes!("../../../schemas/edict-provider/contracts/v1/manifest.json");

const GENERATOR_COORDINATE: &str = "echo-wesley-gen.provider-artifact-generator@1";
const GENERATOR_VERSION: &str = "0.1.0";
const GENERATOR_BYTES: &[u8] = b"echo-wesley-gen provider generator test material v1";

fn admitted_pack() -> AdmittedProviderContractPackV1 {
    admit_provider_contract_pack_v1(CONTRACT_CDDL, CONTRACT_MANIFEST)
        .expect("checked Edict provider contract pack is admitted")
}

fn generate() -> (ProviderGenerationInputV1, ProviderPrimaryArtifactsV1) {
    let pack = admitted_pack();
    let input = build_provider_generation_input_v1(SOURCE, &pack, SETTINGS)
        .expect("checked provider generation input builds");
    let primary = generate_provider_primary_artifacts_v1(&input, &pack)
        .expect("checked primary provider artifacts generate");
    (input, primary)
}

fn generator() -> ProviderGeneratorMaterialV1 {
    ProviderGeneratorMaterialV1::new(GENERATOR_COORDINATE, GENERATOR_VERSION, GENERATOR_BYTES)
        .expect("explicit test generator material is valid")
}

fn map_value<'a>(value: &'a CanonicalValueV1, field: &str) -> Option<&'a CanonicalValueV1> {
    let CanonicalValueV1::Map(entries) = value else {
        return None;
    };
    entries.iter().find_map(|(key, value)| match key {
        CanonicalValueV1::Text(key) if key == field => Some(value),
        _ => None,
    })
}

fn typed_digest(value: &CanonicalValueV1) -> Option<String> {
    let CanonicalValueV1::Array(parts) = value else {
        return None;
    };
    match parts.as_slice() {
        [CanonicalValueV1::Text(algorithm), CanonicalValueV1::Bytes(bytes)] => {
            Some(format!("{algorithm}:{}", hex::encode(bytes)))
        }
        _ => None,
    }
}

fn contains_resource_reference(value: &CanonicalValueV1, coordinate: &str, digest: &str) -> bool {
    let is_reference = matches!(
        map_value(value, "id"),
        Some(CanonicalValueV1::Text(id)) if id == coordinate
    ) && map_value(value, "digest").and_then(typed_digest).as_deref()
        == Some(digest);
    if is_reference {
        return true;
    }
    match value {
        CanonicalValueV1::Array(values) => values
            .iter()
            .any(|value| contains_resource_reference(value, coordinate, digest)),
        CanonicalValueV1::Map(entries) => entries
            .iter()
            .any(|(_, value)| contains_resource_reference(value, coordinate, digest)),
        _ => false,
    }
}

#[test]
fn exact_materials_produce_one_verified_canonical_provenance_manifest() {
    let (input, primary) = generate();
    let generator = generator();
    let first = generate_provider_generation_provenance_v1(&input, &primary, &generator)
        .expect("provenance generates from exact materials");
    let second = generate_provider_generation_provenance_v1(&input, &primary, &generator)
        .expect("repeated provenance generates from exact materials");

    assert_eq!(first, second);
    assert_eq!(first.role(), "provenance.provider-generation");
    assert_eq!(
        first.coordinate(),
        "echo.edict-provider-generation-provenance@1"
    );
    assert_eq!(
        first.schema_contract(),
        "wesley:GenerationProvenanceManifestV1"
    );

    let manifest = first.manifest();
    assert_eq!(
        manifest.api_version,
        "wesley.generation-provenance-manifest/v1"
    );
    assert_eq!(manifest.generation_input_digest, input.digest());
    assert_eq!(
        manifest.settings_digest,
        compute_generation_artifact_digest_v1(SETTINGS)
    );
    assert_eq!(manifest.generator.coordinate, GENERATOR_COORDINATE);
    assert_eq!(manifest.generator.version, GENERATOR_VERSION);
    assert_eq!(
        manifest.generator.digest,
        compute_generation_artifact_digest_v1(GENERATOR_BYTES)
    );
    assert_eq!(
        manifest.contract_versions.input_schema,
        "wesley.extension-generation-input/v1"
    );
    assert_eq!(
        manifest.contract_versions.provenance_schema,
        "wesley.generation-provenance-manifest/v1"
    );
    assert_eq!(
        manifest.contract_versions.generator_abi,
        "wesley.extension-generator/v1"
    );

    assert_eq!(
        manifest.source_artifacts,
        input.wesley_input().owner_declarations
    );
    assert_eq!(
        manifest
            .source_artifacts
            .iter()
            .map(|artifact| artifact.coordinate.as_str())
            .collect::<Vec<_>>(),
        vec![
            "echo.semantic-schema@1",
            "edict.provider-contract-pack.cddl@1",
            "edict.provider-contract-pack.manifest@1",
        ]
    );

    let mut expected_emitted = primary
        .artifacts()
        .iter()
        .map(|artifact| artifact.content_reference().clone())
        .chain(std::iter::once(
            primary.schema().content_reference().clone(),
        ))
        .collect::<Vec<_>>();
    expected_emitted.sort_by(|left, right| left.coordinate.cmp(&right.coordinate));
    assert_eq!(manifest.emitted_artifacts, expected_emitted);
    assert_eq!(manifest.emitted_artifacts.len(), 6);

    for resource in primary.resources() {
        assert!(
            manifest
                .emitted_artifacts
                .iter()
                .all(|artifact| artifact.coordinate != resource.coordinate()),
            "generated resources are transitively bound, not primary emits"
        );
        assert!(
            primary.artifacts().iter().any(|artifact| {
                contains_resource_reference(
                    artifact.canonical_value(),
                    resource.coordinate(),
                    resource.domain_framed_digest(),
                )
            }),
            "every generated resource is digest-bound by a primary artifact"
        );
    }
    assert!(manifest.emitted_artifacts.iter().all(|artifact| {
        artifact.coordinate != "echo.edict-provider-generation-provenance@1"
            && artifact.coordinate != "echo.edict-provider-generation-review@1"
    }));
    for artifact in primary.artifacts() {
        let emitted = manifest
            .emitted_artifacts
            .iter()
            .find(|emitted| emitted.coordinate == artifact.coordinate())
            .expect("each primary canonical artifact is emitted");
        assert_eq!(emitted, artifact.content_reference());
        assert_ne!(emitted.digest, artifact.domain_framed_digest());
    }

    assert_eq!(first.verification().generation_input_digest, input.digest());
    assert_eq!(first.verification().verified_source_count, 3);
    assert_eq!(first.verification().verified_output_count, 6);
    assert_eq!(
        first.canonical_bytes(),
        first
            .manifest()
            .canonical_bytes()
            .expect("manifest canonicalizes")
    );
    assert_eq!(first.content_reference().coordinate, first.coordinate());
    assert_eq!(
        first.content_reference().digest,
        compute_generation_artifact_digest_v1(first.canonical_bytes())
    );

    let decoded: GenerationProvenanceManifestV1 =
        serde_json::from_slice(first.canonical_bytes()).expect("canonical provenance decodes");
    assert_eq!(&decoded, first.manifest());
    assert_eq!(
        decoded
            .canonical_bytes()
            .expect("decoded provenance canonicalizes"),
        first.canonical_bytes()
    );
}

#[test]
fn tampered_generator_material_preserves_the_typed_wesley_failure() {
    let (input, primary) = generate();
    let provenance = generate_provider_generation_provenance_v1(&input, &primary, &generator())
        .expect("provenance generates from exact materials");
    let tampered = ProviderGeneratorMaterialV1::new(
        GENERATOR_COORDINATE,
        GENERATOR_VERSION,
        b"tampered generator material",
    )
    .expect("tampered generator material remains structurally valid");

    let error = provenance
        .verify_exact_materials(&input, &primary, &tampered)
        .expect_err("changed generator bytes must not verify");
    assert_eq!(
        error.kind(),
        ProviderProvenanceErrorKind::WesleyContractRejected
    );
    assert_eq!(
        error.wesley_contract_kind(),
        Some(GenerationContractErrorKind::ArtifactDigestMismatch)
    );
    assert_eq!(error.subject(), GENERATOR_COORDINATE);
    assert_eq!(
        error.reference(),
        GenerationContractErrorKind::ArtifactDigestMismatch.as_str()
    );
}

#[test]
fn provenance_rejects_primary_outputs_from_another_generation_input() {
    let (input, _) = generate();
    let pack = admitted_pack();
    let mut changed_source =
        serde_json::from_slice::<Value>(SOURCE).expect("checked source is JSON");
    changed_source["budgets"][0]["maxSteps"] = json!(9);
    let changed_source = serde_json::to_vec(&changed_source).expect("changed source serializes");
    let changed_input = build_provider_generation_input_v1(&changed_source, &pack, SETTINGS)
        .expect("changed provider generation input builds");
    let changed_primary = generate_provider_primary_artifacts_v1(&changed_input, &pack)
        .expect("changed primary provider artifacts generate");

    let error = generate_provider_generation_provenance_v1(&input, &changed_primary, &generator())
        .expect_err("primary outputs from a different input must not be attributed");
    assert_eq!(
        error.kind(),
        ProviderProvenanceErrorKind::PrimaryOutputClosureMismatch
    );
    assert_eq!(error.subject(), "generationInputDigest");
    assert_eq!(error.reference(), input.digest());
}

#[test]
fn generator_identity_mismatches_name_the_exact_stable_field() {
    let (input, primary) = generate();
    let provenance = generate_provider_generation_provenance_v1(&input, &primary, &generator())
        .expect("provenance generates from exact materials");
    let changed_coordinate = ProviderGeneratorMaterialV1::new(
        "echo-wesley-gen.other-generator@1",
        GENERATOR_VERSION,
        GENERATOR_BYTES,
    )
    .expect("alternate generator coordinate is structurally valid");

    let coordinate_error = provenance
        .verify_exact_materials(&input, &primary, &changed_coordinate)
        .expect_err("a different generator coordinate must not verify");
    assert_eq!(
        coordinate_error.kind(),
        ProviderProvenanceErrorKind::GeneratorIdentityMismatch
    );
    assert_eq!(coordinate_error.subject(), "generator.coordinate");
    assert_eq!(coordinate_error.reference(), GENERATOR_COORDINATE);
    assert_eq!(coordinate_error.wesley_contract_kind(), None);

    let changed_version =
        ProviderGeneratorMaterialV1::new(GENERATOR_COORDINATE, "0.2.0", GENERATOR_BYTES)
            .expect("alternate generator version is structurally valid");
    let version_error = provenance
        .verify_exact_materials(&input, &primary, &changed_version)
        .expect_err("a different generator version must not verify");
    assert_eq!(
        version_error.kind(),
        ProviderProvenanceErrorKind::GeneratorIdentityMismatch
    );
    assert_eq!(version_error.subject(), "generator.version");
    assert_eq!(version_error.reference(), GENERATOR_VERSION);
    assert_eq!(version_error.wesley_contract_kind(), None);
}

#[test]
fn generator_coordinate_cannot_alias_provider_closure_coordinates() {
    let (input, primary) = generate();
    let source = input.semantic_source().source();
    let resource = primary
        .resources()
        .first()
        .expect("checked primary closure has generated resources");
    let conflicts = [
        (
            "echo.edict-provider-generation-provenance@1",
            "provenance.provider-generation",
        ),
        (resource.coordinate(), resource.role()),
        (
            source.package_manifest.coordinate.as_str(),
            source.package_manifest.role.as_str(),
        ),
        (
            source.package_manifest.provider_coordinate.as_str(),
            "packageManifest.providerCoordinate",
        ),
    ];

    for (coordinate, role) in conflicts {
        let conflicting =
            ProviderGeneratorMaterialV1::new(coordinate, GENERATOR_VERSION, GENERATOR_BYTES)
                .expect("provider coordinate is structurally valid as a Wesley coordinate");
        let error = generate_provider_generation_provenance_v1(&input, &primary, &conflicting)
            .expect_err("generator coordinate must be unique in the provider closure");
        assert_eq!(
            error.kind(),
            ProviderProvenanceErrorKind::GeneratorCoordinateConflict
        );
        assert_eq!(error.subject(), "generator.coordinate");
        assert_eq!(error.reference(), role);
        assert_eq!(error.wesley_contract_kind(), None);
    }
}

#[test]
fn exact_source_reordering_moves_provenance_but_not_primary_emitted_bytes() {
    let pack = admitted_pack();
    let baseline_input = build_provider_generation_input_v1(SOURCE, &pack, SETTINGS)
        .expect("baseline provider generation input builds");
    let baseline_primary = generate_provider_primary_artifacts_v1(&baseline_input, &pack)
        .expect("baseline primary provider artifacts generate");
    let baseline = generate_provider_generation_provenance_v1(
        &baseline_input,
        &baseline_primary,
        &generator(),
    )
    .expect("baseline provenance generates");

    let mut reordered_source =
        serde_json::from_slice::<Value>(SOURCE).expect("checked source is JSON");
    reordered_source["types"]
        .as_array_mut()
        .expect("types are an array")
        .reverse();
    let reordered_source =
        serde_json::to_vec(&reordered_source).expect("reordered source serializes");
    let reordered_input = build_provider_generation_input_v1(&reordered_source, &pack, SETTINGS)
        .expect("reordered provider generation input builds");
    let reordered_primary = generate_provider_primary_artifacts_v1(&reordered_input, &pack)
        .expect("reordered primary provider artifacts generate");
    let reordered = generate_provider_generation_provenance_v1(
        &reordered_input,
        &reordered_primary,
        &generator(),
    )
    .expect("reordered provenance generates");

    assert_eq!(
        baseline.manifest().emitted_artifacts,
        reordered.manifest().emitted_artifacts
    );
    assert_ne!(
        baseline.manifest().generation_input_digest,
        reordered.manifest().generation_input_digest
    );
    assert_ne!(baseline.canonical_bytes(), reordered.canonical_bytes());
    assert_ne!(
        baseline.content_reference().digest,
        reordered.content_reference().digest
    );
}
