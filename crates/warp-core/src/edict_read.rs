// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Trusted-host private evaluation of exactly pinned bounded-read packages.
//!
//! A view borrows real frontier storage under an independently selected aperture.
//! This is not installation, authorization of an application, or causal evidence.
//! The host owns the package pin and view selection. No callbacks or mutations
//! are available to the interpreted program.

mod access;
mod decode;
mod evaluate;
mod graph;
mod instructions;
mod model;
mod view;

pub(crate) use access::{Material, NodeAtomAccess, Outgoing};
pub use view::{GraphReadAperture, ReadBasis, ReadView};

use crate::edict_pure::{EvaluationError, EvaluationLimits};
use crate::{NodeKey, TypeId};

/// Host ceilings, intersected with the package's declared budgets.
#[derive(Clone, Copy, Debug)]
pub struct ReadLimits {
    /// Package/input sizes and deterministic expression evaluation ceilings.
    pub evaluation: EvaluationLimits,
    /// Maximum number of read attempts: node-atom reads, typed outgoing-range
    /// reads, and closure node materializations.
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
    /// Read attempts: node-atom reads, typed outgoing-range reads, and closure
    /// node materializations.
    pub reads: u64,
    /// Aggregate delivered atom bytes.
    pub read_bytes: u64,
    /// Bucket edges inspected by graph reads, matching or not.
    pub edge_scans: u64,
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

/// Underlying cause of an authored graph-read obstruction.
///
/// Resource bounds, a selector that asked for too little and storage that
/// violates the declared invariants stay distinct: each is its own declared
/// failure with its own authored arm.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum GraphReadObstruction {
    /// The edge-read source node is absent.
    EdgeSourceMissing,
    /// More edges match than the declared result list holds.
    EdgeBoundExceeded,
    /// The source bucket exceeds the remaining scan allowance.
    EdgeScanBoundExceeded,
    /// The closure start node is absent.
    ClosureStartMissing,
    /// Another distinct node would exceed the node bound.
    ClosureNodeBoundExceeded,
    /// A selected child lies deeper than the depth bound.
    ClosureDepthBoundExceeded,
    /// A bucket exceeds the remaining scan allowance.
    ClosureScanBoundExceeded,
    /// The declared edge or occurrence capacity is exhausted.
    ClosureWorkBoundExceeded,
    /// The selector declared that the closure it can ask for is insufficient.
    ClosureSelectorInsufficient,
    /// Stored material violates a declared or structural invariant.
    ClosureStorageMalformed,
}

impl GraphReadObstruction {
    /// The versioned tag the runtime records for this failure.
    #[must_use]
    pub const fn runtime_tag(self) -> &'static str {
        match self {
            Self::EdgeSourceMissing => "echo.edge-source-missing/v1",
            Self::EdgeBoundExceeded => "echo.edge-bound-exceeded/v1",
            Self::EdgeScanBoundExceeded => "echo.edge-scan-bound-exceeded/v1",
            Self::ClosureStartMissing => "echo.closure-start-missing/v1",
            Self::ClosureNodeBoundExceeded => "echo.closure-node-bound-exceeded/v1",
            Self::ClosureDepthBoundExceeded => "echo.closure-depth-bound-exceeded/v1",
            Self::ClosureScanBoundExceeded => "echo.closure-scan-bound-exceeded/v1",
            Self::ClosureWorkBoundExceeded => "echo.closure-work-bound-exceeded/v1",
            Self::ClosureSelectorInsufficient => "echo.closure-selector-insufficient/v1",
            Self::ClosureStorageMalformed => "echo.closure-storage-malformed/v1",
        }
    }
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
    /// A graph read named an edge type, or discovered a node type, outside the
    /// host-selected graph aperture.
    OutsideGraphAperture(TypeId),
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
    /// A graph-read failure maps to this authored obstruction.
    GraphObstructed {
        /// Application-owned source obstruction coordinate.
        coordinate: String,
        /// Generic runtime cause.
        cause: GraphReadObstruction,
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
    let (program, input) = prepare(package_bytes, verified_package_digest, input_bytes, limits)?;
    evaluate::run(&program, input, view)
}

fn prepare(
    package_bytes: &[u8],
    verified_package_digest: [u8; 32],
    input_bytes: &[u8],
    limits: ReadLimits,
) -> Result<(model::Program, echo_edict_canonical::CanonicalValueV1), ReadError> {
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
    Ok((program, input))
}

pub(crate) fn evaluate_recorded(
    package: &[u8],
    pin: [u8; 32],
    input: &[u8],
    reads: &mut crate::edict_effects::EffectRangeReadSession<'_>,
    aperture: crate::edict_effects::NodeAperture<'_>,
    meter: &mut crate::edict_pure::evaluate::Meter,
    limits: ReadLimits,
) -> Result<ReadResult, crate::edict_effects::EffectFrameError> {
    aperture.validate()?;
    let (program, input) = prepare(package, pin, input, limits)?;
    let mut access = crate::edict_effects::RecordedNodeAccess { reads, aperture };
    evaluate::run_with_meter(&program, input, &mut access, meter)
}
