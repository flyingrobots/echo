// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Installed contract package boundary.
//!
//! This module binds generated registry metadata to installed mutation handlers
//! and read-only query observers without importing application nouns into core.

#[cfg(feature = "native_rule_bootstrap")]
use std::collections::BTreeSet;

use echo_registry_api::{
    ContractArtifactRejection, ContractArtifactTrustPosture, ContractArtifactVerificationPolicy,
    OpKind, RegistryInfo, RegistryProvider,
};
use thiserror::Error;

#[cfg(feature = "native_rule_bootstrap")]
use blake3::Hasher;
#[cfg(feature = "native_rule_bootstrap")]
use echo_registry_api::verify_contract_artifact;

use crate::ident::Hash;
use crate::observation::ContractQueryObserver;
use crate::rule::RewriteRule;

#[cfg(feature = "native_rule_bootstrap")]
const INSTALLED_CONTRACT_PACKAGE_ID_DOMAIN: &[u8] = b"echo:installed-contract-package-id:v1\0";

/// Deterministic identity for an installed generated contract package.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InstalledContractPackageId(Hash);

impl InstalledContractPackageId {
    /// Reconstructs the id from canonical bytes.
    #[must_use]
    pub const fn from_bytes(bytes: Hash) -> Self {
        Self(bytes)
    }

    /// Returns the canonical byte representation.
    #[must_use]
    pub const fn as_bytes(&self) -> &Hash {
        &self.0
    }
}

/// Host-owned package identity supplied when installing generated contract code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContractPackageIdentity<'a> {
    /// Runtime package name chosen by the host.
    pub package_name: &'a str,
    /// Runtime package version chosen by the host.
    pub package_version: &'a str,
    /// Hex-encoded generated package artifact hash.
    pub artifact_hash_hex: &'a str,
}

/// Generated mutation handler bound to a registry operation id.
pub struct ContractMutationHandler {
    /// Generated mutation operation id this handler supports.
    pub op_id: u32,
    /// Scheduler-owned rewrite rule that handles materialized runtime ingress.
    pub rule: RewriteRule,
}

/// Generated contract package ready for runtime-owner installation.
pub struct InstalledContractPackage<'a> {
    /// Host-owned package identity.
    pub identity: ContractPackageIdentity<'a>,
    /// Generated registry provider compiled by Wesley.
    pub registry: &'a dyn RegistryProvider,
    /// Host verification policy for the generated registry artifact.
    pub verification_policy: ContractArtifactVerificationPolicy<'a>,
    /// Generated mutation handlers to install.
    pub mutation_handlers: Vec<ContractMutationHandler>,
    /// Generated read-only query observers to install.
    pub query_observers: Vec<ContractQueryObserver>,
}

/// Installed package metadata retained by Echo core.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstalledContractPackageRecord {
    /// Deterministic package identity.
    pub package_id: InstalledContractPackageId,
    /// Runtime package name.
    pub package_name: String,
    /// Runtime package version.
    pub package_version: String,
    /// Hex-encoded generated package artifact hash.
    pub artifact_hash_hex: String,
    /// Verified generated registry metadata.
    pub registry_info: RegistryInfo,
    /// Verified artifact trust posture.
    pub trust_posture: ContractArtifactTrustPosture,
    /// Installed mutation operation ids.
    pub mutation_op_ids: Vec<u32>,
    /// Installed query operation ids.
    pub query_op_ids: Vec<u32>,
}

/// Error returned when installing a generated contract package fails.
#[derive(Debug, Error)]
pub enum InstalledContractPackageError<'a> {
    /// Package name was empty.
    #[error("installed contract package name is empty")]
    EmptyPackageName,
    /// Package version was empty.
    #[error("installed contract package version is empty")]
    EmptyPackageVersion,
    /// Package artifact hash was empty.
    #[error("installed contract package artifact hash is empty")]
    EmptyArtifactHash,
    /// Generated registry failed host verification.
    #[error("contract artifact verification failed: {0:?}")]
    ArtifactRejected(ContractArtifactRejection<'a>),
    /// Mutation handler named an operation not present in the generated registry.
    #[error("unknown mutation operation id: {op_id}")]
    UnknownMutationOperation {
        /// Unsupported operation id.
        op_id: u32,
    },
    /// Mutation handler named a non-mutation registry operation.
    #[error("operation id {op_id} is not a mutation: {actual:?}")]
    MutationOperationKindMismatch {
        /// Operation id.
        op_id: u32,
        /// Actual registry operation kind.
        actual: OpKind,
    },
    /// Query observer named an operation not present in the generated registry.
    #[error("unknown query operation id: {op_id}")]
    UnknownQueryOperation {
        /// Unsupported operation id.
        op_id: u32,
    },
    /// Query observer named a non-query registry operation.
    #[error("operation id {op_id} is not a query: {actual:?}")]
    QueryOperationKindMismatch {
        /// Operation id.
        op_id: u32,
        /// Actual registry operation kind.
        actual: OpKind,
    },
    /// Package repeated a mutation handler operation id.
    #[error("duplicate mutation handler operation id in package: {op_id}")]
    DuplicateMutationHandlerInPackage {
        /// Duplicated operation id.
        op_id: u32,
    },
    /// Package repeated a query observer operation id.
    #[error("duplicate query observer operation id in package: {op_id}")]
    DuplicateQueryObserverInPackage {
        /// Duplicated operation id.
        op_id: u32,
    },
    /// Package id is already installed.
    #[error("installed contract package already exists: {package_id:?}")]
    DuplicatePackage {
        /// Duplicated package id.
        package_id: InstalledContractPackageId,
    },
    /// Mutation operation id is already installed by another package.
    #[error("contract mutation operation id already installed: {op_id}")]
    DuplicateInstalledMutationOperation {
        /// Duplicated operation id.
        op_id: u32,
    },
    /// Query operation id is already installed by another package.
    #[error("contract query operation id already installed: {op_id}")]
    DuplicateInstalledQueryOperation {
        /// Duplicated operation id.
        op_id: u32,
    },
    /// Rewrite rule name is already installed.
    #[error("duplicate rewrite rule name: {name}")]
    DuplicateRuleName {
        /// Duplicated rule name.
        name: &'static str,
    },
    /// Rewrite rule id is already installed.
    #[error("duplicate rewrite rule id: {rule_id:?}")]
    DuplicateRuleId {
        /// Duplicated rule id.
        rule_id: Hash,
    },
    /// Rule requested Join conflict policy without a join function.
    #[error("missing join function for installed contract rule")]
    MissingJoinFn,
    /// Engine registration failed after package preflight.
    #[error("internal installed contract registration failure: {reason}")]
    InternalRegistrationFailure {
        /// Static failure reason.
        reason: &'static str,
    },
}

/// Validated package installation plan.
#[cfg(feature = "native_rule_bootstrap")]
pub(crate) struct PreparedInstalledContractPackage {
    pub(crate) record: InstalledContractPackageRecord,
    pub(crate) mutation_handlers: Vec<ContractMutationHandler>,
    pub(crate) query_observers: Vec<ContractQueryObserver>,
}

#[cfg(feature = "native_rule_bootstrap")]
pub(crate) fn prepare_installed_contract_package(
    package: InstalledContractPackage<'_>,
) -> Result<PreparedInstalledContractPackage, InstalledContractPackageError<'_>> {
    validate_identity(package.identity)?;
    let verified = verify_contract_artifact(package.registry, &package.verification_policy)
        .map_err(InstalledContractPackageError::ArtifactRejected)?;

    let mut mutation_op_ids = Vec::with_capacity(package.mutation_handlers.len());
    let mut seen_mutations = BTreeSet::new();
    for handler in &package.mutation_handlers {
        if !seen_mutations.insert(handler.op_id) {
            return Err(
                InstalledContractPackageError::DuplicateMutationHandlerInPackage {
                    op_id: handler.op_id,
                },
            );
        }
        let Some(op) = package.registry.op_by_id(handler.op_id) else {
            return Err(InstalledContractPackageError::UnknownMutationOperation {
                op_id: handler.op_id,
            });
        };
        if op.kind != OpKind::Mutation {
            return Err(
                InstalledContractPackageError::MutationOperationKindMismatch {
                    op_id: handler.op_id,
                    actual: op.kind,
                },
            );
        }
        mutation_op_ids.push(handler.op_id);
    }

    let mut query_op_ids = Vec::with_capacity(package.query_observers.len());
    let mut seen_queries = BTreeSet::new();
    for observer in &package.query_observers {
        if !seen_queries.insert(observer.query_id) {
            return Err(
                InstalledContractPackageError::DuplicateQueryObserverInPackage {
                    op_id: observer.query_id,
                },
            );
        }
        let Some(op) = package.registry.op_by_id(observer.query_id) else {
            return Err(InstalledContractPackageError::UnknownQueryOperation {
                op_id: observer.query_id,
            });
        };
        if op.kind != OpKind::Query {
            return Err(InstalledContractPackageError::QueryOperationKindMismatch {
                op_id: observer.query_id,
                actual: op.kind,
            });
        }
        query_op_ids.push(observer.query_id);
    }

    mutation_op_ids.sort_unstable();
    query_op_ids.sort_unstable();

    let package_id = installed_contract_package_id(package.identity, verified.info);
    let record = InstalledContractPackageRecord {
        package_id,
        package_name: package.identity.package_name.to_owned(),
        package_version: package.identity.package_version.to_owned(),
        artifact_hash_hex: package.identity.artifact_hash_hex.to_owned(),
        registry_info: verified.info,
        trust_posture: verified.posture,
        mutation_op_ids,
        query_op_ids,
    };

    Ok(PreparedInstalledContractPackage {
        record,
        mutation_handlers: package.mutation_handlers,
        query_observers: package.query_observers,
    })
}

#[cfg(feature = "native_rule_bootstrap")]
fn validate_identity(
    identity: ContractPackageIdentity<'_>,
) -> Result<(), InstalledContractPackageError<'_>> {
    if identity.package_name.is_empty() {
        return Err(InstalledContractPackageError::EmptyPackageName);
    }
    if identity.package_version.is_empty() {
        return Err(InstalledContractPackageError::EmptyPackageVersion);
    }
    if identity.artifact_hash_hex.is_empty() {
        return Err(InstalledContractPackageError::EmptyArtifactHash);
    }
    Ok(())
}

#[cfg(feature = "native_rule_bootstrap")]
fn installed_contract_package_id(
    identity: ContractPackageIdentity<'_>,
    registry_info: RegistryInfo,
) -> InstalledContractPackageId {
    let mut hasher = Hasher::new();
    hasher.update(INSTALLED_CONTRACT_PACKAGE_ID_DOMAIN);
    push_len_prefixed(&mut hasher, identity.package_name.as_bytes());
    push_len_prefixed(&mut hasher, identity.package_version.as_bytes());
    push_len_prefixed(&mut hasher, identity.artifact_hash_hex.as_bytes());
    push_len_prefixed(&mut hasher, registry_info.codec_id.as_bytes());
    hasher.update(&registry_info.registry_version.to_le_bytes());
    push_len_prefixed(&mut hasher, registry_info.schema_sha256_hex.as_bytes());
    InstalledContractPackageId::from_bytes(hasher.finalize().into())
}

#[cfg(feature = "native_rule_bootstrap")]
fn push_len_prefixed(hasher: &mut Hasher, bytes: &[u8]) {
    hasher.update(&(bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
}
