// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! This module owns the stable reason codes that name which check refused an
//! authored effect program when Echo admits its package (echo#1023).
//!
//! Codes follow the provider's `effect-program.*` scheme (echo#1018): they
//! never change meaning, and the graph-schema and footprint-law codes are
//! exactly the ones Echo's provider lowerer and verifier report for the same
//! check, so the three judges' refusals can be compared by code.
use super::EffectLawError;

/// The authored-effect program check that refused at admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum EffectProgramRefusal {
    /// The program's fields, schema, kind, identity law or profile pins.
    #[error("effect-program.program-envelope")]
    ProgramEnvelope,
    /// The graph schema is not an exact canonical schema.
    #[error("effect-program.graph-schema")]
    GraphSchema,
    /// The footprint law is not an exact canonical law.
    #[error("effect-program.footprint-law")]
    FootprintLaw,
    /// The footprint law's `max_reads` is above 4,096.
    #[error("effect-program.footprint-law.max-reads")]
    FootprintMaxReads,
    /// The footprint law's `max_writes` is above 256.
    #[error("effect-program.footprint-law.max-writes")]
    FootprintMaxWrites,
    /// The footprint law's `max_range_edges` is above 65,536.
    #[error("effect-program.footprint-law.max-range-edges")]
    FootprintMaxRangeEdges,
    /// A declared input, result or allocation limit or the output type is
    /// outside the effect profile.
    #[error("effect-program.resource-limits")]
    ResourceLimits,
    /// The compiled program bundle is outside the runtime's contract.
    #[error("effect-program.compiled-program")]
    CompiledProgram,
    /// The graph-read declaration does not fit the compiled program.
    #[error("effect-program.graph-read-declaration")]
    GraphReadDeclaration,
    /// The compiled program, its selected operation and the package's
    /// semantic closure disagree, or the program is outside the runtime's
    /// own effect-program judgment.
    #[error("effect-program.compiled-binding")]
    CompiledBinding,
}

impl EffectProgramRefusal {
    /// The stable machine-readable code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ProgramEnvelope => "effect-program.program-envelope",
            Self::GraphSchema => "effect-program.graph-schema",
            Self::FootprintLaw => "effect-program.footprint-law",
            Self::FootprintMaxReads => "effect-program.footprint-law.max-reads",
            Self::FootprintMaxWrites => "effect-program.footprint-law.max-writes",
            Self::FootprintMaxRangeEdges => "effect-program.footprint-law.max-range-edges",
            Self::ResourceLimits => "effect-program.resource-limits",
            Self::CompiledProgram => "effect-program.compiled-program",
            Self::GraphReadDeclaration => "effect-program.graph-read-declaration",
            Self::CompiledBinding => "effect-program.compiled-binding",
        }
    }
}

impl From<EffectLawError> for EffectProgramRefusal {
    fn from(error: EffectLawError) -> Self {
        match error {
            EffectLawError::InvalidLaw => Self::FootprintLaw,
            EffectLawError::MaxReads => Self::FootprintMaxReads,
            EffectLawError::MaxWrites => Self::FootprintMaxWrites,
            EffectLawError::MaxRangeEdges => Self::FootprintMaxRangeEdges,
        }
    }
}
