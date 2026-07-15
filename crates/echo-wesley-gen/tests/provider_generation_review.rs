// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! Deterministic non-authoritative review of Echo provider generation.

use echo_wesley_gen::provider_artifacts::{
    generate_provider_primary_artifacts_v1, ProviderPrimaryArtifactsV1,
};
use echo_wesley_gen::provider_contract_pack::{
    admit_provider_contract_pack_v1, AdmittedProviderContractPackV1,
};
use echo_wesley_gen::provider_generation::{
    build_provider_generation_input_v1, ProviderGenerationInputV1,
};
use echo_wesley_gen::provider_provenance::{
    generate_provider_generation_provenance_v1, ProviderGenerationProvenanceV1,
    ProviderGeneratorMaterialV1,
};
use echo_wesley_gen::provider_review::{
    generate_provider_generation_review_v1, ProviderReviewErrorKind,
};
use serde_json::{json, Value};
use wesley_core::{
    compute_generation_artifact_digest_v1, GenerationContractErrorKind, GenerationReviewV1,
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

fn generate() -> (
    ProviderGenerationInputV1,
    ProviderPrimaryArtifactsV1,
    ProviderGenerationProvenanceV1,
) {
    let pack = admitted_pack();
    let input = build_provider_generation_input_v1(SOURCE, &pack, SETTINGS)
        .expect("checked provider generation input builds");
    let primary = generate_provider_primary_artifacts_v1(&input, &pack)
        .expect("checked primary provider artifacts generate");
    let generator =
        ProviderGeneratorMaterialV1::new(GENERATOR_COORDINATE, GENERATOR_VERSION, GENERATOR_BYTES)
            .expect("explicit test generator material is valid");
    let provenance = generate_provider_generation_provenance_v1(&input, &primary, &generator)
        .expect("checked provider provenance generates");
    (input, primary, provenance)
}

#[test]
fn verified_provenance_produces_one_canonical_non_authoritative_review() {
    let (input, _primary, provenance) = generate();
    let first = generate_provider_generation_review_v1(&input, &provenance)
        .expect("generation review derives from verified provenance");
    let second = generate_provider_generation_review_v1(&input, &provenance)
        .expect("repeated generation review derives from verified provenance");

    assert_eq!(first, second);
    assert_eq!(first.role(), "review.provider-generation");
    assert_eq!(
        first.coordinate(),
        "echo.edict-provider-generation-review@1"
    );
    assert_eq!(first.schema_contract(), "wesley:GenerationReviewV1");

    let review = first.review();
    assert_eq!(review.api_version, "wesley.generation-review/v1");
    assert!(!review.authoritative());
    assert_eq!(review.generation_input_digest, input.digest());
    assert_eq!(
        review.provenance_manifest_digest,
        provenance
            .manifest()
            .digest()
            .expect("checked provenance manifest digests")
    );
    assert_ne!(
        review.provenance_manifest_digest,
        provenance.content_reference().digest,
        "review binds Wesley's domain-framed manifest identity, not its raw JSON digest"
    );
    assert_eq!(review.generator, provenance.manifest().generator);
    assert_eq!(
        review.projection_roles,
        input.wesley_input().projection_roles
    );
    assert_eq!(
        review.source_artifacts,
        provenance.manifest().source_artifacts
    );
    assert_eq!(
        review.emitted_artifacts,
        provenance.manifest().emitted_artifacts
    );
    assert_eq!(review.source_artifacts.len(), 3);
    assert_eq!(review.emitted_artifacts.len(), 6);
    assert!(review.emitted_artifacts.iter().all(|artifact| {
        artifact.coordinate != first.coordinate() && artifact.coordinate != provenance.coordinate()
    }));

    assert_eq!(
        first.canonical_bytes(),
        review.canonical_bytes().expect("review canonicalizes")
    );
    assert_eq!(first.content_reference().coordinate, first.coordinate());
    assert_eq!(
        first.content_reference().digest,
        compute_generation_artifact_digest_v1(first.canonical_bytes())
    );
    let decoded: GenerationReviewV1 =
        serde_json::from_slice(first.canonical_bytes()).expect("canonical review decodes");
    assert_eq!(&decoded, review);
    assert_eq!(
        decoded
            .canonical_bytes()
            .expect("decoded review canonicalizes"),
        first.canonical_bytes()
    );
}

#[test]
fn review_rejects_an_authoritative_claim_during_deserialization() {
    let (input, _primary, provenance) = generate();
    let review = generate_provider_generation_review_v1(&input, &provenance)
        .expect("generation review derives from verified provenance");
    let mut claimed = serde_json::from_slice::<Value>(review.canonical_bytes())
        .expect("canonical review is JSON");
    claimed["authoritative"] = json!(true);
    let claimed = serde_json::to_vec(&claimed).expect("tampered review serializes");

    let error = serde_json::from_slice::<GenerationReviewV1>(&claimed)
        .expect_err("a review cannot claim authority");
    assert!(
        error
            .to_string()
            .contains(GenerationContractErrorKind::AuthoritativeReviewRejected.as_str()),
        "Wesley exposes a stable rejection code"
    );
}

#[test]
fn review_preserves_typed_wesley_input_mismatch_failures() {
    let (_input, _primary, provenance) = generate();
    let pack = admitted_pack();
    let mut changed_source =
        serde_json::from_slice::<Value>(SOURCE).expect("checked source is JSON");
    changed_source["budgets"][0]["maxSteps"] = json!(9);
    let changed_source = serde_json::to_vec(&changed_source).expect("changed source serializes");
    let changed_input = build_provider_generation_input_v1(&changed_source, &pack, SETTINGS)
        .expect("changed provider generation input builds");

    let error = generate_provider_generation_review_v1(&changed_input, &provenance)
        .expect_err("review input must match the provenance input");
    assert_eq!(
        error.kind(),
        ProviderReviewErrorKind::WesleyContractRejected
    );
    assert_eq!(
        error.wesley_contract_kind(),
        Some(GenerationContractErrorKind::GenerationInputDigestMismatch)
    );
    assert_eq!(error.subject(), "generationInputDigest");
    assert_eq!(
        error.reference(),
        GenerationContractErrorKind::GenerationInputDigestMismatch.as_str()
    );
}
