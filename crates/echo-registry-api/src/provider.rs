// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! Provider-generic semantic registry vocabulary.
//!
//! These borrowed values describe exact provider artifact claims. Constructing
//! or inspecting them does not authenticate, admit, install, register, invoke,
//! or execute any operation.

use crate::OpKind;

/// Exact identity of one digest-framed provider artifact.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProviderDigestIdentityV1<'a> {
    /// Semantic or artifact coordinate named by the digest claim.
    pub coordinate: &'a str,
    /// Proposition domain used to frame the digest.
    pub digest_domain: &'a str,
    /// Exact encoded digest value, including its algorithm prefix.
    pub digest: &'a str,
}

/// Exact identity of the admitted provider schema used by generated helpers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProviderSchemaIdentityV1<'a> {
    /// Coordinate of the provider schema artifact.
    pub coordinate: &'a str,
    /// Lowercase hexadecimal SHA-256 of the exact raw schema bytes.
    pub raw_sha256_hex: &'a str,
}

/// Exact coordinate and semantic domain of one provider-owned meaning.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProviderSemanticIdentityV1<'a> {
    /// Provider-owned semantic coordinate.
    pub coordinate: &'a str,
    /// Domain that defines the coordinate's meaning.
    pub semantic_domain: &'a str,
}

/// Schema and codec claims for one operation value boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProviderValueContractV1<'a> {
    /// Semantic schema coordinate for the value.
    pub schema_coordinate: &'a str,
    /// Semantic domain that defines the value schema.
    pub schema_domain: &'a str,
    /// Exact codec identifier selected for the value bytes.
    pub codec_id: &'a str,
}

/// Independent semantic-layer and release-layer provider bundle identities.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProviderBundleIdentityV1<'a> {
    /// Proposition domain used to frame the semantic bundle digest.
    pub semantic_digest_domain: &'a str,
    /// Exact semantic bundle digest.
    pub semantic_digest: &'a str,
    /// Proposition domain used to frame the release bundle digest.
    pub release_digest_domain: &'a str,
    /// Exact release bundle digest.
    pub release_digest: &'a str,
}

/// Exact abstract footprint obligation and its owning algebra artifact.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProviderFootprintIdentityV1<'a> {
    /// Abstract footprint obligation authored for the operation.
    pub obligation: &'a str,
    /// Coordinate of the footprint algebra artifact.
    pub algebra_coordinate: &'a str,
    /// Proposition domain used to frame the footprint algebra digest.
    pub algebra_digest_domain: &'a str,
    /// Exact footprint algebra digest.
    pub algebra_digest: &'a str,
}

/// Exact semantic, generated, and target claims for one provider operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProviderOperationV1<'a> {
    /// Semantic operation coordinate.
    pub coordinate: &'a str,
    /// Domain that defines the operation coordinate.
    pub semantic_domain: &'a str,
    /// Whether the operation mutates state or performs a bounded read.
    pub kind: OpKind,
    /// Law used to derive the persisted operation identifier.
    pub operation_id_law: &'a str,
    /// Exact persisted operation identifier.
    pub operation_id: u32,
    /// Input schema and codec claims.
    pub input: ProviderValueContractV1<'a>,
    /// Output schema and codec claims.
    pub output: ProviderValueContractV1<'a>,
    /// Target-owned schema for a failure before obstruction mapping.
    pub target_failure_schema: &'a str,
    /// Provider-owned typed obstruction produced by failure mapping.
    pub obstruction: ProviderSemanticIdentityV1<'a>,
    /// Provider-owned payload schema for the typed obstruction.
    pub obstruction_payload_schema: &'a str,
    /// Exact Target IR artifact identity.
    pub target_ir: ProviderDigestIdentityV1<'a>,
    /// Exact target-profile artifact identity.
    pub target_profile: ProviderDigestIdentityV1<'a>,
    /// Exact generated-artifact-profile identity.
    pub generated_artifact_profile: ProviderDigestIdentityV1<'a>,
    /// Semantic operation profile selected for this operation.
    pub operation_profile: ProviderSemanticIdentityV1<'a>,
    /// Exact operation-profiles document identity.
    pub operation_profiles: ProviderDigestIdentityV1<'a>,
    /// Exact footprint obligation and algebra claims.
    pub footprint: ProviderFootprintIdentityV1<'a>,
}

/// Borrowed provider-generic registry retained by a package proposal.
///
/// This registry intentionally contains no GraphQL facade, Wesley generator
/// metadata, runtime installation token, or executor authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProviderRegistryV1<'a> {
    /// Echo contract ABI version targeted by generated helpers.
    pub echo_contract_abi_version: u32,
    /// Contract-host helper API version targeted by generated helpers.
    pub helper_api_version: u32,
    /// Exact raw provider schema identity.
    pub provider_schema: ProviderSchemaIdentityV1<'a>,
    /// Exact target-bundle-profile artifact identity.
    pub target_bundle_profile: ProviderDigestIdentityV1<'a>,
    /// Independent semantic-layer and release-layer bundle identities.
    pub bundle: ProviderBundleIdentityV1<'a>,
    /// Provider operations described by this registry.
    pub operations: &'a [ProviderOperationV1<'a>],
}

impl<'a> ProviderRegistryV1<'a> {
    /// Look up one operation by its exact persisted identifier.
    #[must_use]
    pub fn operation_by_id(&self, operation_id: u32) -> Option<&ProviderOperationV1<'a>> {
        let mut found = None;
        for operation in self.operations {
            if operation.operation_id != operation_id {
                continue;
            }
            if found.is_some() {
                return None;
            }
            found = Some(operation);
        }
        found
    }
}
