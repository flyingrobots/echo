// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Trusted-host private evaluation of exactly pinned bounded-read packages.
//!
//! A view borrows real frontier storage under an independently selected aperture.
//! This is not installation, authorization of an application, or causal evidence.
//! The host owns the package pin and view selection. No callbacks or mutations
//! are available to the interpreted program.

mod decode;
mod evaluate;
mod instructions;
mod model;
mod view;

pub use view::{ReadBasis, ReadView};

use crate::edict_pure::{EvaluationError, EvaluationLimits};
use crate::NodeKey;

/// Host ceilings, intersected with the package's declared budgets.
#[derive(Clone, Copy, Debug)]
pub struct ReadLimits {
    /// Package/input sizes and deterministic expression evaluation ceilings.
    pub evaluation: EvaluationLimits,
    /// Maximum number of node-atom read attempts.
    pub max_reads: u64,
    /// Maximum aggregate atom bytes delivered to the interpreter.
    pub max_read_bytes: u64,
}

/// Result of private computation over a borrowed frontier, never a receipt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadResult {
    /// Canonical application output.
    pub output: Vec<u8>,
    /// Exact independently selected worldline basis.
    pub basis: ReadBasis,
    /// Authored opaque application basis value; no canonical-head claim is made.
    pub application_basis: [u8; 32],
    /// Expression work units plus one per delivered atom byte.
    pub steps: u64,
    /// Cumulative charged value storage, including copied atom bytes.
    pub allocated_bytes: u64,
    /// Successful node-atom reads.
    pub reads: u64,
    /// Aggregate delivered atom bytes.
    pub read_bytes: u64,
}

/// Underlying cause of an authored obstruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadObstruction {
    /// Node or attachment is absent.
    Missing,
    /// The attachment is a descent rather than an atom.
    AtomRequired,
    /// Stored and requested atom types disagree.
    TypeMismatch,
    /// Atom bytes exceed the authored result type bound.
    AtomTooLarge,
    /// An authored terminal guard is false.
    Guard,
}

/// Typed failure; no partial output or mutation is published.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReadError {
    /// Canonical parsing, expression evaluation, or ordinary evaluation budget failure.
    Evaluation(EvaluationError),
    /// Selected worldline, tick, or real state root differs from the expected basis.
    BasisMismatch,
    /// Aperture is oversized, unsorted, or contains duplicates.
    InvalidAperture,
    /// Requested node is outside the host-selected aperture.
    OutsideAperture(NodeKey),
    /// The read-attempt ceiling is exhausted.
    ReadCountExceeded,
    /// The aggregate read-byte ceiling is exhausted.
    ReadBytesExceeded,
    /// A generic read failure or guard maps to this authored obstruction.
    Obstructed {
        /// Application-owned source obstruction coordinate.
        coordinate: String,
        /// Generic runtime cause.
        cause: ReadObstruction,
    },
}

impl From<EvaluationError> for ReadError {
    fn from(value: EvaluationError) -> Self {
        Self::Evaluation(value)
    }
}

/// Evaluates an unchanged compiler package against an explicit immutable view.
///
/// The expected digest must come from independently verified host policy, not
/// from untrusted package bytes. View establishment (including full-state basis
/// hashing) precedes this metered evaluation. Parsing is bounded by package/input
/// byte ceilings; expression and read work is charged by the interpreter.
pub fn evaluate(
    package_bytes: &[u8],
    verified_package_digest: [u8; 32],
    input_bytes: &[u8],
    view: &ReadView<'_>,
    limits: ReadLimits,
) -> Result<ReadResult, ReadError> {
    let program = decode::package(package_bytes, verified_package_digest, limits)?;
    if input_bytes.len()
        > limits
            .evaluation
            .max_input_bytes
            .min(model::MAX_ARTIFACT_BYTES)
    {
        return Err(EvaluationError::InputTooLarge.into());
    }
    let input = echo_edict_canonical::decode_canonical_cbor_v1(input_bytes)
        .map_err(|_| EvaluationError::InvalidInput)?;
    evaluate::run(&program, input, view)
}
