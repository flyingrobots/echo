// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Provider-generic registry contract witnesses.

use echo_registry_api::{
    OpKind, ProviderBundleIdentityV1, ProviderDigestIdentityV1, ProviderFootprintIdentityV1,
    ProviderOperationV1, ProviderRegistryV1, ProviderSchemaIdentityV1, ProviderSemanticIdentityV1,
    ProviderValueContractV1, CONTRACT_HOST_HELPER_API_VERSION, ECHO_CONTRACT_ABI_VERSION,
    LITTLE_ENDIAN_CODEC_V1_ID, SEMANTIC_OPERATION_ID_LAW_V1,
};

const TARGET_IR: ProviderDigestIdentityV1<'static> = ProviderDigestIdentityV1 {
    coordinate: "echo.span-ir/v1",
    digest_domain: "edict.target-ir.artifact/v1",
    digest: "sha256:target-ir",
};

const TARGET_PROFILE: ProviderDigestIdentityV1<'static> = ProviderDigestIdentityV1 {
    coordinate: "echo.dpo@1",
    digest_domain: "edict.target-profile/v1",
    digest: "sha256:target-profile",
};

const GENERATED_PROFILE: ProviderDigestIdentityV1<'static> = ProviderDigestIdentityV1 {
    coordinate: "echo.dpo.registration/v1",
    digest_domain: "echo.generated-artifact-profile/v1",
    digest: "sha256:generated-profile",
};

const OPERATION_PROFILES: ProviderDigestIdentityV1<'static> = ProviderDigestIdentityV1 {
    coordinate: "echo.dpo.operation-profiles/v1",
    digest_domain: "echo.dpo.operation-profiles/v1",
    digest: "sha256:operation-profiles",
};

const OPERATIONS: [ProviderOperationV1<'static>; 1] = [ProviderOperationV1 {
    coordinate: "a.b@1.t",
    semantic_domain: "echo.edict-provider/operation/v1",
    kind: OpKind::Mutation,
    operation_id_law: SEMANTIC_OPERATION_ID_LAW_V1,
    operation_id: 3_389_142_194,
    input: ProviderValueContractV1 {
        schema_coordinate: "a.b@1.Input",
        schema_domain: "echo.edict-provider/value/v1",
        codec_id: LITTLE_ENDIAN_CODEC_V1_ID,
    },
    output: ProviderValueContractV1 {
        schema_coordinate: "a.b@1.Output",
        schema_domain: "echo.edict-provider/value/v1",
        codec_id: LITTLE_ENDIAN_CODEC_V1_ID,
    },
    target_failure_schema: "target.replace.rejected",
    obstruction: ProviderSemanticIdentityV1 {
        coordinate: "domain.WriteRejected",
        semantic_domain: "echo.edict-provider/obstruction/v1",
    },
    obstruction_payload_schema: "domain.WriteRejected.Payload",
    target_ir: TARGET_IR,
    target_profile: TARGET_PROFILE,
    generated_artifact_profile: GENERATED_PROFILE,
    operation_profile: ProviderSemanticIdentityV1 {
        coordinate: "continuum.profile.write/v1",
        semantic_domain: "echo.edict-provider/operation-profile/v1",
    },
    operation_profiles: OPERATION_PROFILES,
    footprint: ProviderFootprintIdentityV1 {
        obligation: "target.replace.footprint",
        algebra_coordinate: "echo.dpo.footprint/v1",
        algebra_digest_domain: "echo.dpo.footprint/v1",
        algebra_digest: "sha256:footprint-algebra",
    },
}];

#[test]
fn provider_registry_represents_semantic_operation_without_graphql_facade() {
    let registry = ProviderRegistryV1 {
        echo_contract_abi_version: ECHO_CONTRACT_ABI_VERSION,
        helper_api_version: CONTRACT_HOST_HELPER_API_VERSION,
        provider_schema: ProviderSchemaIdentityV1 {
            coordinate: "echo.provider-artifacts.cddl@1",
            raw_sha256_hex: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        },
        target_bundle_profile: ProviderDigestIdentityV1 {
            coordinate: "echo.dpo.bundle/v1",
            digest_domain: "echo.dpo.bundle/v1",
            digest: "sha256:target-bundle-profile",
        },
        bundle: ProviderBundleIdentityV1 {
            semantic_digest_domain: "edict.bundle.semantic/v1",
            semantic_digest: "sha256:semantic-bundle",
            release_digest_domain: "edict.bundle.release/v1",
            release_digest: "sha256:release-bundle",
        },
        operations: &OPERATIONS,
    };

    assert_eq!(
        registry.operation_by_id(3_389_142_194),
        Some(&OPERATIONS[0])
    );
    let operation = &OPERATIONS[0];

    assert_eq!(operation.coordinate, "a.b@1.t");
    assert_eq!(operation.kind, OpKind::Mutation);
    assert_eq!(operation.input.codec_id, LITTLE_ENDIAN_CODEC_V1_ID);
    assert!(registry.operation_by_id(7).is_none());
}

#[test]
fn provider_registry_does_not_select_an_ambiguous_operation_id() {
    let operations = [
        OPERATIONS[0],
        ProviderOperationV1 {
            coordinate: "a.b@1.conflicting",
            ..OPERATIONS[0]
        },
    ];
    let registry = ProviderRegistryV1 {
        echo_contract_abi_version: ECHO_CONTRACT_ABI_VERSION,
        helper_api_version: CONTRACT_HOST_HELPER_API_VERSION,
        provider_schema: ProviderSchemaIdentityV1 {
            coordinate: "echo.provider-artifacts.cddl@1",
            raw_sha256_hex: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        },
        target_bundle_profile: ProviderDigestIdentityV1 {
            coordinate: "echo.dpo.bundle/v1",
            digest_domain: "echo.dpo.bundle/v1",
            digest: "sha256:target-bundle-profile",
        },
        bundle: ProviderBundleIdentityV1 {
            semantic_digest_domain: "edict.bundle.semantic/v1",
            semantic_digest: "sha256:semantic-bundle",
            release_digest_domain: "edict.bundle.release/v1",
            release_digest: "sha256:release-bundle",
        },
        operations: &operations,
    };

    assert!(registry.operation_by_id(3_389_142_194).is_none());
}
