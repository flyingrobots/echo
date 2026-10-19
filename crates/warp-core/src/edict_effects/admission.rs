// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Runtime-owner package policy evidence, separate from installation.
use super::{EffectPackageError, ExecutableOperationPackageV2};
use crate::EchoOperationBudgetV1;
/// Independently selected package-admission pins and budget ceiling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectPackageAdmissionPolicyV2 {
    pin: [u8; 32],
    operation: String,
    authority: [u8; 32],
    maximum: EchoOperationBudgetV1,
}
impl EffectPackageAdmissionPolicyV2 {
    /// Pins exact publication bytes, operation, authority profile and budget.
    #[must_use]
    pub fn exact(
        pin: [u8; 32],
        operation: impl Into<String>,
        authority: [u8; 32],
        maximum: EchoOperationBudgetV1,
    ) -> Self {
        Self {
            pin,
            operation: operation.into(),
            authority,
            maximum,
        }
    }
    /// Policy identity using the existing V1 policy recipe and V2 package pin.
    #[must_use]
    pub fn identity(&self) -> [u8; 32] {
        let mut h = blake3::Hasher::new();
        h.update(b"echo:operation-admission-policy:v1\0");
        h.update(&self.pin);
        h.update(&(self.operation.len() as u64).to_le_bytes());
        h.update(self.operation.as_bytes());
        h.update(&self.authority);
        h.update(&self.maximum.steps().to_le_bytes());
        h.update(&self.maximum.read_bytes().to_le_bytes());
        h.update(&self.maximum.write_bytes().to_le_bytes());
        h.finalize().into()
    }
}
/// Stable V2 package-admission refusal category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EffectAdmissionError {
    /// Exact bytes differ from the independent package pin.
    #[error("effect package identity differs from policy")]
    PackageIdentityMismatch,
    /// Canonical artifact structure or profile bindings are invalid.
    #[error("invalid effect publication")]
    ArtifactInvalid,
    /// The package operation coordinate differs from policy.
    #[error("effect operation differs from policy")]
    OperationCoordinateMismatch,
    /// The package authority profile differs from policy.
    #[error("effect authority profile differs from policy")]
    AuthorityProfileMismatch,
    /// The publication requires more resources than policy permits.
    #[error("effect package budget exceeds policy")]
    BudgetExceedsPolicy,
}
/// Opaque evidence of runtime-owner admission, without installation or a grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedEffectPackageV2 {
    package: ExecutableOperationPackageV2,
    policy_id: [u8; 32],
    id: [u8; 32],
    policy: EffectPackageAdmissionPolicyV2,
}
impl AdmittedEffectPackageV2 {
    /// Independent policy retained alongside its identity.
    #[must_use]
    pub const fn admission_policy(&self) -> &EffectPackageAdmissionPolicyV2 {
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
    policy: &EffectPackageAdmissionPolicyV2,
    bytes: Vec<u8>,
) -> Result<AdmittedEffectPackageV2, EffectAdmissionError> {
    let package =
        ExecutableOperationPackageV2::prepare(&bytes, policy.pin).map_err(|error| match error {
            EffectPackageError::IdentityMismatch => EffectAdmissionError::PackageIdentityMismatch,
            _ => EffectAdmissionError::ArtifactInvalid,
        })?;
    if package.operation_coordinate() != policy.operation {
        return Err(EffectAdmissionError::OperationCoordinateMismatch);
    }
    if package.authority_profile_identity() != policy.authority {
        return Err(EffectAdmissionError::AuthorityProfileMismatch);
    }
    let budget = package.budget_ceiling();
    if budget.steps() > policy.maximum.steps()
        || budget.read_bytes() > policy.maximum.read_bytes()
        || budget.write_bytes() > policy.maximum.write_bytes()
    {
        return Err(EffectAdmissionError::BudgetExceedsPolicy);
    }
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
