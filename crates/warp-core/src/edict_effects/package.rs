// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Exact V2 publication binding, separate from interpreter and admission.
use super::{EffectFootprintLaw, EffectGraphSchema, EFFECT_READ_STORAGE_CEILING_V1};
use crate::edict_pure::{
    values::{bytes, exact_fields, field, number, require_text, text},
    EvaluationError,
};
use crate::EchoOperationBudgetV1;
use echo_edict_canonical::{decode_canonical_cbor_v1, CanonicalValueV1 as Value};

/// Refusal while preparing a pinned effect publication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EffectPackageError {
    /// The exact bytes differ from the requested V2 package identity.
    #[error("effect package identity mismatch")]
    IdentityMismatch,
    /// The publication has malformed or inconsistent fields or profiles.
    #[error("invalid effect package")]
    InvalidPackage,
}
/// Prepared publication, without installation or mutation authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutableOperationPackageV2 {
    canonical: Vec<u8>,
    program_identity: [u8; 32],
    compiled: Vec<u8>,
    schema: EffectGraphSchema,
    law: EffectFootprintLaw,
    output_type: String,
    max_input_bytes: u64,
    max_result_bytes: u64,
    max_allocated_bytes: u64,
    budget: EchoOperationBudgetV1,
}
impl ExecutableOperationPackageV2 {
    /// Checks exact pinned publication bytes and their executable contracts.
    /// # Errors
    /// Refuses substitutions, malformed maps and inconsistent contracts.
    pub fn prepare(bytes: &[u8], expected: [u8; 32]) -> Result<Self, EffectPackageError> {
        if domain_hash(b"echo:operation-package:v2\0", bytes) != expected {
            return Err(EffectPackageError::IdentityMismatch);
        }
        let value =
            decode_canonical_cbor_v1(bytes).map_err(|_| EffectPackageError::InvalidPackage)?;
        Self::parse(&value, bytes).map_err(|_| EffectPackageError::InvalidPackage)
    }
    /// Exact bytes bound by the requested V2 package identity.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical
    }
    /// Identity of the exact nested program bytes under the V2 domain.
    #[must_use]
    pub const fn program_identity(&self) -> [u8; 32] {
        self.program_identity
    }
    /// Compiler bytes, not yet validated by the effect interpreter.
    #[must_use]
    pub fn compiled_program(&self) -> &[u8] {
        &self.compiled
    }
    /// Executable schema decoded from this publication's exact program.
    #[must_use]
    pub const fn graph_schema(&self) -> &EffectGraphSchema {
        &self.schema
    }
    /// Footprint permissions decoded from the same pinned program as the schema.
    #[must_use]
    pub const fn footprint_law(&self) -> &EffectFootprintLaw {
        &self.law
    }
    /// Compiler-declared result coordinate, bounded to 1,024 UTF-8 bytes.
    #[must_use]
    pub fn output_type(&self) -> &str {
        &self.output_type
    }
    /// Declared application input ceiling, at most 65,536 bytes.
    #[must_use]
    pub const fn max_input_bytes(&self) -> u64 {
        self.max_input_bytes
    }
    /// Declared application result ceiling, at most 65,536 bytes.
    #[must_use]
    pub const fn max_result_bytes(&self) -> u64 {
        self.max_result_bytes
    }
    /// Declared evaluation allocation ceiling, at most 16 MiB.
    #[must_use]
    pub const fn max_allocated_bytes(&self) -> u64 {
        self.max_allocated_bytes
    }
    /// Publication's budget ceiling; delegation is checked separately at admission.
    #[must_use]
    pub const fn budget_ceiling(&self) -> EchoOperationBudgetV1 {
        self.budget
    }

    fn parse(value: &Value, canonical: &[u8]) -> Result<Self, EvaluationError> {
        exact_fields(
            value,
            &[
                "schema",
                "operation_coordinate",
                "obstruction_coordinate",
                "authority_profile_identity",
                "semantic_closure",
                "program",
                "budget_ceiling",
                "target_profile_identity",
                "application_basis_schema_identity",
                "input_schema_identity",
                "result_schema_identity",
                "obstruction_schema_identity",
                "result_interpretation_identity",
                "obstruction_interpretation_identity",
                "evaluation_basis_schema_identity",
                "interpreter_profile_identity",
                "intrinsic_profile_identity",
                "footprint_contract_identity",
            ],
        )?;
        require_text(value, "schema", "echo.operation-package/v2")?;
        coordinate(field(value, "operation_coordinate")?)?;
        coordinate(field(value, "obstruction_coordinate")?)?;
        hash(field(value, "authority_profile_identity")?)?;
        let closure = field(value, "semantic_closure")?;
        exact_fields(
            closure,
            &[
                "application_schema_coordinate",
                "application_schema_identity",
                "canonical_meaning_identity",
                "core_identity",
                "edict_source_identity",
                "target_ir_identity",
            ],
        )?;
        coordinate(field(closure, "application_schema_coordinate")?)?;
        for name in [
            "application_schema_identity",
            "canonical_meaning_identity",
            "core_identity",
            "edict_source_identity",
        ] {
            hash(field(closure, name)?)?;
        }
        let target_ir = hash(field(closure, "target_ir_identity")?)?;
        let program_bytes = bytes(field(value, "program")?)?;
        let program = decode_canonical_cbor_v1(program_bytes)
            .map_err(|_| EvaluationError::InvalidArtifact)?;
        exact_fields(
            &program,
            &[
                "schema",
                "kind",
                "interpreter_profile_identity",
                "intrinsic_profile_identity",
                "compiled_program",
                "target_ir_identity",
                "identity_law",
                "graph_schema",
                "footprint_law",
                "max_input_bytes",
                "max_result_bytes",
                "max_allocated_bytes",
                "output_type",
            ],
        )?;
        require_text(&program, "schema", "echo.operation-program/v2")?;
        require_text(&program, "kind", "edict-effect-program/v1")?;
        if number(field(&program, "identity_law")?)? != 1
            || hash(field(&program, "target_ir_identity")?)? != target_ir
        {
            return Err(EvaluationError::InvalidArtifact);
        }
        for (name, label) in [
            (
                "target_profile_identity",
                "echo.operation-target.edict-effect/v1",
            ),
            (
                "application_basis_schema_identity",
                "echo.operation.basis.edict-effect/v1",
            ),
            (
                "input_schema_identity",
                "echo.operation.input.edict-effect/v1",
            ),
            (
                "result_schema_identity",
                "echo.operation.result.edict-effect/v1",
            ),
            (
                "obstruction_schema_identity",
                "echo.operation.obstruction.edict-effect/v1",
            ),
            (
                "result_interpretation_identity",
                "echo.operation.result-interpretation.edict-effect/v1",
            ),
            (
                "obstruction_interpretation_identity",
                "echo.operation.obstruction-interpretation.edict-effect/v1",
            ),
            (
                "evaluation_basis_schema_identity",
                "echo.operation.evaluation-basis/v1",
            ),
            (
                "interpreter_profile_identity",
                "echo.operation-interpreter.edict-effect/v1",
            ),
            (
                "intrinsic_profile_identity",
                "echo.operation-intrinsics.edict-effect/v1",
            ),
        ] {
            let expected = domain_hash(b"echo:operation-profile:v1\0", label.as_bytes());
            if hash(field(value, name)?)? != expected {
                return Err(EvaluationError::InvalidArtifact);
            }
        }
        for name in ["interpreter_profile_identity", "intrinsic_profile_identity"] {
            if hash(field(&program, name)?)? != hash(field(value, name)?)? {
                return Err(EvaluationError::InvalidArtifact);
            }
        }
        let schema =
            EffectGraphSchema::from_canonical_bytes(bytes(field(&program, "graph_schema")?)?)
                .map_err(|_| EvaluationError::InvalidArtifact)?;
        let law_bytes = bytes(field(&program, "footprint_law")?)?;
        let law = EffectFootprintLaw::from_canonical_bytes(law_bytes)
            .map_err(|_| EvaluationError::InvalidArtifact)?;
        let mut footprint = blake3::Hasher::new();
        footprint.update(b"echo:operation-footprint-law:v2\0");
        footprint.update(law_bytes);
        if hash(field(value, "footprint_contract_identity")?)? != *footprint.finalize().as_bytes() {
            return Err(EvaluationError::InvalidArtifact);
        }
        let max_input_bytes = number(field(&program, "max_input_bytes")?)?;
        let max_result_bytes = number(field(&program, "max_result_bytes")?)?;
        let max_allocated_bytes = number(field(&program, "max_allocated_bytes")?)?;
        if max_input_bytes > 65_536
            || max_result_bytes > 65_536
            || max_allocated_bytes > EFFECT_READ_STORAGE_CEILING_V1
        {
            return Err(EvaluationError::InvalidArtifact);
        }
        if coordinate(field(&program, "output_type")?)?.len() > 1024 {
            return Err(EvaluationError::InvalidArtifact);
        }
        let budget = field(value, "budget_ceiling")?;
        exact_fields(budget, &["steps", "read_bytes", "write_bytes"])?;
        let budget = EchoOperationBudgetV1::new(
            number(field(budget, "steps")?)?,
            number(field(budget, "read_bytes")?)?,
            number(field(budget, "write_bytes")?)?,
        );
        if budget.steps() == 0 {
            return Err(EvaluationError::InvalidArtifact);
        }
        Ok(Self {
            canonical: canonical.to_vec(),
            program_identity: domain_hash(b"echo:operation-program:v2\0", program_bytes),
            compiled: bytes(field(&program, "compiled_program")?)?.to_vec(),
            schema,
            law,
            output_type: coordinate(field(&program, "output_type")?)?.to_owned(),
            max_input_bytes,
            max_result_bytes,
            max_allocated_bytes,
            budget,
        })
    }
}
fn hash(value: &Value) -> Result<[u8; 32], EvaluationError> {
    bytes(value)?
        .try_into()
        .map_err(|_| EvaluationError::InvalidArtifact)
}
fn coordinate(value: &Value) -> Result<&str, EvaluationError> {
    let value = text(value)?;
    if value.is_empty() {
        return Err(EvaluationError::InvalidArtifact);
    }
    Ok(value)
}
fn domain_hash(domain: &[u8], bytes: &[u8]) -> [u8; 32] {
    let mut h = blake3::Hasher::new();
    h.update(domain);
    h.update(&(bytes.len() as u64).to_le_bytes());
    h.update(bytes);
    h.finalize().into()
}
