// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Bounded, effect-free evaluation of exactly pinned compiler packages.
//!
//! This trusted-host computation is not package installation, admission,
//! scheduler settlement, or causal evidence. Its caller must obtain the expected
//! package identity from an independently verified, authorized release.
//!
//! Compiler-produced byte constants preserve raw octets. Materializing a
//! constant uses the same deterministic copy charges as other runtime values;
//! package-size limits bound decoding before evaluation.

mod decode;
pub(crate) mod evaluate;
#[cfg(test)]
mod fold_tests;
pub(crate) mod functions;
#[cfg(test)]
mod list_tests;
mod matches;
pub(crate) mod model;
pub(crate) mod proven_order;
pub(crate) mod syntax;
pub(crate) mod values;
#[cfg(test)]
mod variant_tests;
mod variants;

use echo_edict_canonical::decode_canonical_cbor_v1;

/// Host ceilings, intersected with the compiler's declared evaluation budget.
#[derive(Clone, Copy, Debug)]
#[allow(
    clippy::struct_field_names,
    reason = "Each field is an explicit resource ceiling, including in the private compiler checker."
)]
pub struct EvaluationLimits {
    /// Maximum encoded package size before canonical decoding.
    pub max_package_bytes: usize,
    /// Maximum encoded application input size before canonical decoding.
    pub max_input_bytes: usize,
    /// Maximum deterministic work units, including byte comparisons, selected slice bytes, and concatenated bytes.
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
    /// Deterministic work units, including byte comparisons, selected slice bytes, and concatenated bytes.
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
    /// Reserved shared-frame storage could not be provisioned by the host.
    /// This is a host fault and must never become an Action obstruction.
    HostAllocationFailed,
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
    let (program, input) = prepare(package_bytes, verified_package_digest, input_bytes, limits)?;
    evaluate::run(&program, input)
}

pub(crate) fn prepare(
    package_bytes: &[u8],
    verified_package_digest: [u8; 32],
    input_bytes: &[u8],
    limits: EvaluationLimits,
) -> Result<(model::Program, echo_edict_canonical::CanonicalValueV1), EvaluationError> {
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
    Ok((program, input))
}

#[cfg(test)]
mod predicate_tests;

#[cfg(test)]
mod byte_count_tests;
#[cfg(test)]
mod byte_utf8_tests;
#[cfg(test)]
mod proven_range_tests;

#[cfg(test)]
mod byte_at_tests;
