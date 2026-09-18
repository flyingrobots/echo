// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Bounded, effect-free evaluation of exactly pinned compiler packages.
//!
//! This trusted-host computation is not package installation, admission,
//! scheduler settlement, or causal evidence. Its caller must obtain the expected
//! package identity from an independently verified, authorized release.

mod decode;
mod evaluate;
mod model;
mod syntax;
mod values;

use echo_edict_canonical::decode_canonical_cbor_v1;

/// Host ceilings, intersected with the compiler's declared evaluation budget.
#[derive(Clone, Copy, Debug)]
pub struct EvaluationLimits {
    /// Maximum encoded package size before canonical decoding.
    pub max_package_bytes: usize,
    /// Maximum encoded application input size before canonical decoding.
    pub max_input_bytes: usize,
    /// Maximum number of interpreted operations.
    pub max_steps: u64,
    /// Maximum cumulative materialized value bytes, including copies.
    pub max_allocated_bytes: u64,
    /// Maximum canonical result size.
    pub max_output_bytes: u64,
}

/// A pure result and deterministic resource accounting, never a Tick or Receipt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvaluationResult {
    /// Canonical application result bytes.
    pub output: Vec<u8>,
    /// Interpreted operation count.
    pub steps: u64,
    /// Cumulative charged value storage.
    pub allocated_bytes: u64,
}

/// Typed failure before a pure result is returned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvaluationError {
    /// The package exceeds the host's decode aperture.
    PackageTooLarge,
    /// The input exceeds the host's decode aperture.
    InputTooLarge,
    /// Package bytes do not match the trusted host's verified pin.
    PackageIdentityMismatch,
    /// Canonical bytes or required package structure are invalid.
    InvalidArtifact,
    /// The selected program contains unsupported semantics.
    UnsupportedProgram,
    /// Input does not inhabit the authored runtime type.
    InvalidInput,
    /// An authored input predicate was false at the named coordinate.
    InputConstraintFailed(String),
    /// The interpreted operation budget was exhausted.
    StepBudgetExceeded,
    /// The cumulative value storage budget was exhausted.
    AllocationBudgetExceeded,
    /// The canonical result exceeds its output budget.
    OutputBudgetExceeded,
}

/// Evaluates exact compiler-produced package bytes under a verified host pin.
///
/// The pin is trusted-host input, not an authentication claim supplied by an
/// application. A matching digest alone does not establish verifier approval.
/// No graph, runtime host, callbacks, clock, filesystem, or WAL is accessible.
pub fn evaluate(
    package_bytes: &[u8],
    verified_package_digest: [u8; 32],
    input_bytes: &[u8],
    limits: EvaluationLimits,
) -> Result<EvaluationResult, EvaluationError> {
    if package_bytes.len() > limits.max_package_bytes.min(model::MAX_ARTIFACT_BYTES) {
        return Err(EvaluationError::PackageTooLarge);
    }
    if input_bytes.len() > limits.max_input_bytes.min(model::MAX_ARTIFACT_BYTES) {
        return Err(EvaluationError::InputTooLarge);
    }
    let package =
        decode_canonical_cbor_v1(package_bytes).map_err(|_| EvaluationError::InvalidArtifact)?;
    let program = decode::package(&package, verified_package_digest, limits)?;
    let input = decode_canonical_cbor_v1(input_bytes).map_err(|_| EvaluationError::InvalidInput)?;
    evaluate::run(&program, input)
}
