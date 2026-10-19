// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! The authored-effect payload of the single current operation program.
use super::{EffectFootprintLaw, EffectGraphSchema, EFFECT_READ_STORAGE_CEILING_V1};
use crate::edict_pure::{
    values::{bytes, exact_fields, field, number, require_text, text},
    EvaluationError,
};
use echo_edict_canonical::CanonicalValueV1 as Value;

/// Refusal while preparing a pinned effect publication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EffectPackageError {
    /// The exact bytes differ from the requested current package identity.
    #[error("effect package identity mismatch")]
    IdentityMismatch,
    /// The publication has malformed or inconsistent fields or profiles.
    #[error("invalid effect package")]
    InvalidPackage,
    /// The invocation has malformed or inconsistent package, input or basis fields.
    #[error("invalid effect invocation")]
    InvalidInvocation,
}
/// Prepared authored-effect program payload, without execution authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectProgram {
    value: Value,
    compiled: Vec<u8>,
    schema: EffectGraphSchema,
    law: EffectFootprintLaw,
    target_ir: [u8; 32],
    footprint: [u8; 32],
    output_type: String,
    max_input_bytes: u64,
    max_result_bytes: u64,
    max_allocated_bytes: u64,
}
impl EffectProgram {
    pub(crate) fn from_value(value: &Value) -> Result<Self, EvaluationError> {
        exact_fields(
            value,
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
        require_text(value, "schema", "echo.operation-program/v1")?;
        require_text(value, "kind", "edict-effect-program/v1")?;
        if number(field(value, "identity_law")?)? != 1 {
            return Err(EvaluationError::InvalidArtifact);
        }
        for (key, label) in [
            (
                "interpreter_profile_identity",
                "echo.operation-interpreter.edict-effect/v1",
            ),
            (
                "intrinsic_profile_identity",
                "echo.operation-intrinsics.edict-effect/v1",
            ),
        ] {
            if hash(field(value, key)?)?
                != domain_hash(b"echo:operation-profile:v1\0", label.as_bytes())
            {
                return Err(EvaluationError::InvalidArtifact);
            }
        }
        let schema = EffectGraphSchema::from_canonical_bytes(bytes(field(value, "graph_schema")?)?)
            .map_err(|_| EvaluationError::InvalidArtifact)?;
        let law_bytes = bytes(field(value, "footprint_law")?)?;
        let law = EffectFootprintLaw::from_canonical_bytes(law_bytes)
            .map_err(|_| EvaluationError::InvalidArtifact)?;
        let mut footprint = blake3::Hasher::new();
        footprint.update(b"echo:operation-footprint-law:v1\0");
        footprint.update(law_bytes);
        let max_input_bytes = number(field(value, "max_input_bytes")?)?;
        let max_result_bytes = number(field(value, "max_result_bytes")?)?;
        let max_allocated_bytes = number(field(value, "max_allocated_bytes")?)?;
        let output_type = coordinate(field(value, "output_type")?)?;
        if max_input_bytes > 65536
            || max_result_bytes > 65536
            || max_allocated_bytes > EFFECT_READ_STORAGE_CEILING_V1
            || output_type.len() > 1024
        {
            return Err(EvaluationError::InvalidArtifact);
        }
        Ok(Self {
            value: value.clone(),
            compiled: bytes(field(value, "compiled_program")?)?.to_vec(),
            schema,
            law,
            target_ir: hash(field(value, "target_ir_identity")?)?,
            footprint: footprint.finalize().into(),
            output_type: output_type.to_owned(),
            max_input_bytes,
            max_result_bytes,
            max_allocated_bytes,
        })
    }
    pub(crate) fn to_value(&self) -> Value {
        self.value.clone()
    }
    pub(crate) const fn target_ir_identity(&self) -> [u8; 32] {
        self.target_ir
    }
    pub(crate) const fn footprint_identity(&self) -> [u8; 32] {
        self.footprint
    }
    /// Compiler bytes whose instruction language still requires validation.
    #[must_use]
    pub fn compiled_program(&self) -> &[u8] {
        &self.compiled
    }
    /// Executable graph schema from the exact program.
    #[must_use]
    pub const fn graph_schema(&self) -> &EffectGraphSchema {
        &self.schema
    }
    /// Exact footprint law paired with the executable schema.
    #[must_use]
    pub const fn footprint_law(&self) -> &EffectFootprintLaw {
        &self.law
    }
    /// Compiler-declared output type coordinate.
    #[must_use]
    pub fn output_type(&self) -> &str {
        &self.output_type
    }
    /// Maximum canonical input bytes, including framing.
    #[must_use]
    pub const fn max_input_bytes(&self) -> u64 {
        self.max_input_bytes
    }
    /// Maximum canonical result bytes.
    #[must_use]
    pub const fn max_result_bytes(&self) -> u64 {
        self.max_result_bytes
    }
    /// Maximum declared evaluation allocation bytes.
    #[must_use]
    pub const fn max_allocated_bytes(&self) -> u64 {
        self.max_allocated_bytes
    }
}
pub(super) fn hash(value: &Value) -> Result<[u8; 32], EvaluationError> {
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
pub(super) fn domain_hash(domain: &[u8], bytes: &[u8]) -> [u8; 32] {
    let mut h = blake3::Hasher::new();
    h.update(domain);
    h.update(&(bytes.len() as u64).to_le_bytes());
    h.update(bytes);
    h.finalize().into()
}
