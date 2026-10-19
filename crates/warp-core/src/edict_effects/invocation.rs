// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Static V2 input/basis binding, without installed authority or freshness.
use super::package::{domain_hash, hash};
use super::{EffectPackageError, ExecutableOperationPackageV2};
use crate::edict_pure::{
    values::{bytes, exact_fields, field, number, require_text, text},
    EvaluationError,
};
use crate::{EchoOperationBudgetV1, EchoOperationEvaluationBasisV1};
use echo_edict_canonical::{decode_canonical_cbor_v1, CanonicalValueV1 as Value};
/// Prepared invocation bytes, not an admitted Action or verified grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectInvocationV2 {
    canonical: Vec<u8>,
    input: Vec<u8>,
    basis: EchoOperationEvaluationBasisV1,
    grant: [u8; 32],
    identity: [u8; 32],
    budget: EchoOperationBudgetV1,
}
impl EffectInvocationV2 {
    /// Checks canonical invocation input against its pinned package contract.
    ///
    /// This checks static byte/basis consistency. Scheduler admission must still
    /// verify the authority grant and installation, and evaluation must resolve
    /// the retained parent against the actual runtime frontier.
    /// # Errors
    /// Refuses malformed or inconsistent input, budget, package and basis fields.
    pub fn prepare(
        bytes: &[u8],
        package: &ExecutableOperationPackageV2,
    ) -> Result<Self, EffectPackageError> {
        let value =
            decode_canonical_cbor_v1(bytes).map_err(|_| EffectPackageError::InvalidInvocation)?;
        Self::parse(&value, bytes, package).map_err(|_| EffectPackageError::InvalidInvocation)
    }
    fn parse(
        value: &Value,
        canonical: &[u8],
        package: &ExecutableOperationPackageV2,
    ) -> Result<Self, EvaluationError> {
        exact_fields(
            value,
            &[
                "schema",
                "authority_grant_identity",
                "delegated_budget",
                "evaluation_basis",
                "operation_coordinate",
                "package_id",
                "application_input_bytes",
            ],
        )?;
        require_text(value, "schema", "echo.operation-invocation/v2")?;
        if hash(field(value, "package_id")?)? != package.identity()
            || text(field(value, "operation_coordinate")?)? != package.operation_coordinate()
        {
            return Err(EvaluationError::InvalidArtifact);
        }
        let input = bytes(field(value, "application_input_bytes")?)?;
        if u64::try_from(input.len()).map_err(|_| EvaluationError::InvalidArtifact)?
            > package.max_input_bytes()
        {
            return Err(EvaluationError::InvalidArtifact);
        }
        decode_canonical_cbor_v1(input).map_err(|_| EvaluationError::InvalidArtifact)?;
        let basis =
            EchoOperationEvaluationBasisV1::from_value(field(value, "evaluation_basis")?.clone())
                .map_err(|_| EvaluationError::InvalidArtifact)?;
        if basis.application_basis().schema_identity()
            != domain_hash(
                b"echo:operation-profile:v1\0",
                b"echo.operation.basis.edict-effect/v1",
            )
            || basis.application_basis().value_identity()
                != domain_hash(b"echo:operation-edict-effect-basis:v1\0", input)
        {
            return Err(EvaluationError::InvalidArtifact);
        }
        let budget = field(value, "delegated_budget")?;
        exact_fields(budget, &["steps", "read_bytes", "write_bytes"])?;
        let budget = EchoOperationBudgetV1::new(
            number(field(budget, "steps")?)?,
            number(field(budget, "read_bytes")?)?,
            number(field(budget, "write_bytes")?)?,
        );
        let ceiling = package.budget_ceiling();
        if budget.steps() == 0
            || budget.steps() > ceiling.steps()
            || budget.read_bytes() > ceiling.read_bytes()
            || budget.write_bytes() > ceiling.write_bytes()
        {
            return Err(EvaluationError::InvalidArtifact);
        }
        Ok(Self {
            canonical: canonical.to_vec(),
            input: input.to_vec(),
            basis,
            grant: hash(field(value, "authority_grant_identity")?)?,
            identity: domain_hash(b"echo:operation-invocation:v2\0", canonical),
            budget,
        })
    }
    /// Requested resource allowance, checked only against the publication ceiling.
    #[must_use]
    pub const fn delegated_budget(&self) -> EchoOperationBudgetV1 {
        self.budget
    }
    /// Exact canonical invocation bytes.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical
    }
    /// Exact application input whose digest is retained in the basis.
    #[must_use]
    pub fn application_input_bytes(&self) -> &[u8] {
        &self.input
    }
    /// Claimed exact V1 parent basis; freshness has not been established.
    #[must_use]
    pub const fn evaluation_basis(&self) -> EchoOperationEvaluationBasisV1 {
        self.basis
    }
    /// Claimed grant identity; no grant authority is conferred by decoding it.
    #[must_use]
    pub const fn authority_grant_identity(&self) -> [u8; 32] {
        self.grant
    }
    /// Identity of exact invocation bytes under the distinct V2 domain.
    #[must_use]
    pub const fn identity(&self) -> [u8; 32] {
        self.identity
    }
}
