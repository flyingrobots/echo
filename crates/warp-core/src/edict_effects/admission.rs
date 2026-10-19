// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Runtime-owner package policy evidence, separate from installation.
use super::{EffectPackageError, ExecutableOperationPackageV2};
use crate::echo_operation::admission_error;
use crate::{
    EchoOperationAdmissionError, EchoOperationAdmissionErrorKind, EchoOperationAdmissionPolicy,
};
/// Opaque evidence of runtime-owner admission, without installation or a grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedEffectPackageV2 {
    package: ExecutableOperationPackageV2,
    policy_id: [u8; 32],
    id: [u8; 32],
    policy: EchoOperationAdmissionPolicy,
}
impl AdmittedEffectPackageV2 {
    /// Independent policy retained alongside its identity.
    #[must_use]
    pub const fn admission_policy(&self) -> &EchoOperationAdmissionPolicy {
        &self.policy
    }
    /// Exact publication admitted by the runtime owner.
    #[must_use]
    pub const fn package(&self) -> &ExecutableOperationPackageV2 {
        &self.package
    }
    /// Identity of the independently supplied admission policy.
    #[must_use]
    pub const fn admission_policy_identity(&self) -> [u8; 32] {
        self.policy_id
    }
    /// Admission identity under the distinct V2 domain.
    #[must_use]
    pub const fn identity(&self) -> [u8; 32] {
        self.id
    }
}
pub(crate) fn admit(
    policy: &EchoOperationAdmissionPolicy,
    bytes: Vec<u8>,
) -> Result<AdmittedEffectPackageV2, EchoOperationAdmissionError> {
    let package = ExecutableOperationPackageV2::prepare(&bytes, policy.expected_package_identity())
        .map_err(|error| {
            let kind = match error {
                EffectPackageError::IdentityMismatch => {
                    EchoOperationAdmissionErrorKind::PackageIdentityMismatch
                }
                _ => EchoOperationAdmissionErrorKind::ArtifactInvalid,
            };
            admission_error(
                kind,
                "effect publication differs from its pin or canonical contract",
            )
        })?;
    policy.validate_contract(
        package.operation_coordinate(),
        package.authority_profile_identity(),
        package.budget_ceiling(),
    )?;
    let policy_id = policy.identity();
    let mut h = blake3::Hasher::new();
    h.update(b"echo:operation-package-admission:v2\0");
    h.update(&package.identity());
    h.update(&policy_id);
    Ok(AdmittedEffectPackageV2 {
        package,
        policy_id,
        id: h.finalize().into(),
        policy: policy.clone(),
    })
}
