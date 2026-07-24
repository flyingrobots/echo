// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Echo-owned executable operation packages and bounded evaluation.
//!
//! This module is intentionally operation-oriented rather than a second
//! provider revision. Provider v1 binds ambient native callbacks and remains a
//! compatibility corridor. An executable operation package instead carries the
//! complete data-only program interpreted here by Echo.
//!
//! The first two earned program profiles are deliberately small. One anchors a
//! typed node and compares the digest of its typed alpha attachment before
//! replacing that attachment. The other requires the node and attachment to be
//! entirely absent and creates both atomically (ADR 0024). Each has one unique
//! match, a closed attachment algebra, explicit resource bounds, and no
//! callback, function pointer, matcher, executor, or application-specific
//! intrinsic.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use blake3::Hasher;
use bytes::Bytes;
use echo_edict_canonical::{
    decode_canonical_cbor_v1, encode_canonical_cbor_v1, CanonicalValueError,
    CanonicalValueErrorKind, CanonicalValueV1,
};
use thiserror::Error;

use crate::{
    attachment::{AtomPayload, AttachmentKey, AttachmentValue},
    clock::{GlobalTick, WorldlineTick},
    footprint::{Footprint, WarpScopedPortKey},
    head::WriterHeadKey,
    head_inbox::{make_intent_kind, IngressEnvelope, IngressPayload, IngressTarget, IntentKind},
    ident::{EdgeKey, Hash, NodeKey, TypeId},
    receipt::{TickReceipt, TickReceiptDisposition, TickReceiptEntry, TickReceiptRejection},
    record::NodeRecord,
    snapshot::{compute_commit_hash_v2, Snapshot},
    tick_patch::{SlotId, TickCommitStatus, TickPatchError, WarpOp, WarpTickPatchV1},
    tx::TxId,
    worldline_state::WorldlineState,
};

const PACKAGE_SCHEMA: &str = "echo.operation-package/v1";
const PROGRAM_SCHEMA: &str = "echo.operation-program/v1";
const INVOCATION_SCHEMA: &str = "echo.operation-invocation/v1";
const PROGRAM_KIND: &str = "anchored-node-attachment-compare-and-set/v1";
const FOOTPRINT_CONTRACT: &str = "anchored-node-alpha-exact/v1";
const INPUT_SCHEMA: &str = "echo.operation.input.anchored-node-alpha-cas/v1";
const RESULT_SCHEMA: &str = "echo.operation.result.anchored-node-alpha-cas/v1";
const OBSTRUCTION_SCHEMA: &str = "echo.operation.obstruction.anchored-node-alpha-cas/v1";
const RESULT_INTERPRETATION: &str =
    "echo.operation.result-interpretation.anchored-node-alpha-cas/v1";
const OBSTRUCTION_INTERPRETATION: &str =
    "echo.operation.obstruction-interpretation.anchored-node-alpha-cas/v1";
const BASIS_SCHEMA: &str = "echo.operation.evaluation-basis/v1";
const APPLICATION_BASIS_SCHEMA: &str = "echo.operation.basis.anchored-node-alpha/v1";
const TARGET_PROFILE: &str = "echo.operation-target.anchored-node-alpha-cas/v1";
const CREATE_INVOCATION_SCHEMA: &str =
    "echo.operation-invocation.anchored-node-alpha-create-if-absent/v1";
const CREATE_PROGRAM_KIND: &str = "anchored-node-attachment-create-if-absent/v1";
const CREATE_FOOTPRINT_CONTRACT: &str = "anchored-node-alpha-create-if-absent-exact/v1";
const CREATE_INPUT_SCHEMA: &str = "echo.operation.input.anchored-node-alpha-create-if-absent/v1";
const CREATE_RESULT_SCHEMA: &str = "echo.operation.result.anchored-node-alpha-create-if-absent/v1";
const CREATE_OBSTRUCTION_SCHEMA: &str =
    "echo.operation.obstruction.anchored-node-alpha-create-if-absent/v1";
const CREATE_RESULT_INTERPRETATION: &str =
    "echo.operation.result-interpretation.anchored-node-alpha-create-if-absent/v1";
const CREATE_OBSTRUCTION_INTERPRETATION: &str =
    "echo.operation.obstruction-interpretation.anchored-node-alpha-create-if-absent/v1";
const CREATE_APPLICATION_BASIS_SCHEMA: &str =
    "echo.operation.basis.anchored-node-alpha-create-if-absent/v1";
const CREATE_TARGET_PROFILE: &str = "echo.operation-target.anchored-node-alpha-create-if-absent/v1";
const CREATE_ABSENCE_PRECONDITION: &str = "node-and-alpha-attachment-absent/v1";
const INTERPRETER_PROFILE: &str = "echo.operation-interpreter/v1";
const INTRINSIC_PROFILE: &str = "echo.operation-attachment-algebra/v1";
const PACKAGE_ID_DOMAIN: &[u8] = b"echo:operation-package:v1\0";
const PROGRAM_ID_DOMAIN: &[u8] = b"echo:operation-program:v1\0";
const INVOCATION_ID_DOMAIN: &[u8] = b"echo:operation-invocation:v1\0";
const ACTION_INTENT_KIND: &str = "echo.executable-operation-action/v1";
const ACTION_BATCH_RULE_PACK_DOMAIN: &[u8] = b"echo:operation-action-batch-rule-pack:v1\0";
const ACTION_BATCH_PLAN_DOMAIN: &[u8] = b"echo:operation-action-batch-plan:v1\0";
const ACTION_BATCH_REWRITES_DOMAIN: &[u8] = b"echo:operation-action-batch-rewrites:v1\0";
const ACTION_BATCH_COMPOSITION_DOMAIN: &[u8] = b"echo:operation-action-batch-composition:v1\0";
const ACTION_OUTCOME_RECORD_MAGIC: &[u8; 8] = b"EOACT001";
const INVOCATION_BYTES_DIGEST_DOMAIN: &[u8] = b"echo:operation-invocation-bytes:v1\0";
const BASIS_ID_DOMAIN: &[u8] = b"echo:operation-evaluation-basis:v1\0";
const PACKAGE_ADMISSION_ID_DOMAIN: &[u8] = b"echo:operation-package-admission:v1\0";
const INSTALLATION_ID_DOMAIN: &[u8] = b"echo:operation-installation:v1\0";
const INVOCATION_ADMISSION_ID_DOMAIN: &[u8] = b"echo:operation-invocation-admission:v1\0";
const PRIVATE_EVALUATION_ID_DOMAIN: &[u8] = b"echo:operation-private-evaluation:v1\0";
const PREPARATION_ID_DOMAIN: &[u8] = b"echo:operation-preparation:v1\0";
const RESULT_ID_DOMAIN: &[u8] = b"echo:operation-result:v1\0";
const CREATE_RESULT_ID_DOMAIN: &[u8] =
    b"echo:operation-result-anchored-node-alpha-create-if-absent:v1\0";
const OBSTRUCTION_ID_DOMAIN: &[u8] = b"echo:operation-obstruction:v1\0";
const TERMINAL_OUTCOME_ID_DOMAIN: &[u8] = b"echo:operation-terminal-outcome:v1\0";
const ATOM_VALUE_DOMAIN: &[u8] = b"echo:operation-atom-value:v1\0";
const APPLICATION_BASIS_VALUE_DOMAIN: &[u8] = b"echo:operation-anchored-node-alpha-basis:v1\0";
const CREATE_APPLICATION_BASIS_DOMAIN: &[u8] =
    b"echo:operation-anchored-node-alpha-create-if-absent-basis:v1\0";
const CREATE_RESULT_ABSENCE_PROPOSITION_DOMAIN: &[u8] =
    b"echo:operation-result-anchored-node-alpha-absence-proposition:v1\0";
const FOOTPRINT_DIGEST_DOMAIN: &[u8] = b"echo:operation-footprint:v1\0";
const RECEIPT_DIGEST_DOMAIN: &[u8] = b"echo:operation-receipt:v1\0";
const COMPOSITION_DIGEST_DOMAIN: &[u8] = b"echo:operation-singleton-composition:v1\0";
const PLAN_DIGEST_DOMAIN: &[u8] = b"echo:operation-plan:v1\0";
const REWRITES_DIGEST_DOMAIN: &[u8] = b"echo:operation-rewrites:v1\0";
const GENESIS_COMMIT_DOMAIN: &[u8] = b"echo:operation-genesis-commit:v1\0";

/// Process-local capability proving that admission, evaluation, and commit are
/// owned by the same Echo runtime instance.
#[derive(Clone, Debug)]
pub(crate) struct EchoOperationEvaluationAuthorityV1(Arc<()>);

impl EchoOperationEvaluationAuthorityV1 {
    pub(crate) fn new() -> Self {
        Self(Arc::new(()))
    }

    fn same_owner(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

/// Stable content identity of exact executable-operation package bytes.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EchoOperationPackageIdV1(Hash);

impl EchoOperationPackageIdV1 {
    /// Returns the raw digest bytes.
    #[must_use]
    pub const fn as_hash(self) -> Hash {
        self.0
    }
}

/// Stable content identity of exact Echo operation-program bytes.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EchoOperationProgramIdV1(Hash);

impl EchoOperationProgramIdV1 {
    /// Returns the raw digest bytes.
    #[must_use]
    pub const fn as_hash(self) -> Hash {
        self.0
    }
}

/// Stable content identity of one canonical invocation.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EchoOperationInvocationIdV1(Hash);

impl EchoOperationInvocationIdV1 {
    /// Returns the raw digest bytes.
    #[must_use]
    pub const fn as_hash(self) -> Hash {
        self.0
    }
}

/// Stable content identity of one exact Echo evaluation basis.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EchoOperationEvaluationBasisIdV1(Hash);

impl EchoOperationEvaluationBasisIdV1 {
    /// Returns the raw digest bytes.
    #[must_use]
    pub const fn as_hash(self) -> Hash {
        self.0
    }
}

/// Stable identity of Echo's package-admission evidence.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EchoOperationPackageAdmissionIdV1(Hash);

impl EchoOperationPackageAdmissionIdV1 {
    /// Returns the raw digest bytes.
    #[must_use]
    pub const fn as_hash(self) -> Hash {
        self.0
    }
}

/// Stable identity of Echo-owned installed executable meaning.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstalledEchoOperationIdV1(Hash);

impl InstalledEchoOperationIdV1 {
    /// Returns the raw digest bytes.
    #[must_use]
    pub const fn as_hash(self) -> Hash {
        self.0
    }
}

/// Stable identity of Echo's invocation-admission evidence.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EchoOperationInvocationAdmissionIdV1(Hash);

impl EchoOperationInvocationAdmissionIdV1 {
    /// Returns the raw digest bytes.
    #[must_use]
    pub const fn as_hash(self) -> Hash {
        self.0
    }
}

/// Stable identity of one bounded private evaluation.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EchoOperationPrivateEvaluationIdV1(Hash);

impl EchoOperationPrivateEvaluationIdV1 {
    /// Returns the raw digest bytes.
    #[must_use]
    pub const fn as_hash(self) -> Hash {
        self.0
    }
}

/// Stable identity of one complete committable preparation.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PreparedEchoOperationIdV1(Hash);

impl PreparedEchoOperationIdV1 {
    /// Returns the raw digest bytes.
    #[must_use]
    pub const fn as_hash(self) -> Hash {
        self.0
    }
}

/// Stable identity of the typed result produced by private evaluation.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EchoOperationResultIdV1(Hash);

impl EchoOperationResultIdV1 {
    /// Returns the raw digest bytes.
    #[must_use]
    pub const fn as_hash(self) -> Hash {
        self.0
    }
}

/// Stable identity of one typed no-patch evaluation obstruction.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EchoOperationObstructionIdV1(Hash);

impl EchoOperationObstructionIdV1 {
    /// Returns the raw digest bytes.
    #[must_use]
    pub const fn as_hash(self) -> Hash {
        self.0
    }
}

/// Computes the identity of exact canonical package bytes.
///
/// This digest is substitution evidence only. It does not confer an operation
/// coordinate, installation, invocability, or authority.
#[must_use]
pub fn echo_operation_package_id_v1(bytes: &[u8]) -> EchoOperationPackageIdV1 {
    EchoOperationPackageIdV1(domain_hash(PACKAGE_ID_DOMAIN, bytes))
}

/// Returns the reserved stable kind for Echo-interpreted executable-operation
/// Actions.
#[must_use]
pub fn echo_operation_action_intent_kind_v1() -> IntentKind {
    make_intent_kind(ACTION_INTENT_KIND)
}

/// Wraps one canonical executable-operation invocation in the ordinary Echo
/// Action envelope.
///
/// The helper decodes and reproduces the invocation before constructing
/// ingress, so non-canonical or unsupported bytes never acquire the reserved
/// Action kind.
pub fn echo_operation_action_envelope_v1(
    target: IngressTarget,
    canonical_invocation_bytes: Vec<u8>,
) -> Result<IngressEnvelope, EchoOperationArtifactErrorV1> {
    let invocation = EchoOperationInvocationV1::from_canonical_bytes(&canonical_invocation_bytes)?;
    if invocation.to_canonical_bytes()? != canonical_invocation_bytes {
        return Err(artifact_error(
            EchoOperationArtifactErrorKindV1::NonCanonical,
            "executable-operation Action must carry exact canonical invocation bytes",
        ));
    }
    Ok(IngressEnvelope::local_intent(
        target,
        echo_operation_action_intent_kind_v1(),
        canonical_invocation_bytes,
    ))
}

pub(crate) fn echo_operation_action_invocation_bytes_v1(
    envelope: &IngressEnvelope,
) -> Option<&[u8]> {
    match envelope.payload() {
        IngressPayload::LocalIntent {
            intent_kind,
            intent_bytes,
        } if *intent_kind == echo_operation_action_intent_kind_v1() => Some(intent_bytes),
        IngressPayload::LocalIntent { .. } => None,
    }
}

/// Computes the digest of one typed attachment atom.
#[must_use]
pub fn echo_operation_atom_value_digest_v1(type_id: TypeId, bytes: &[u8]) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(ATOM_VALUE_DOMAIN);
    hasher.update(type_id.as_bytes());
    hasher.update(&(bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
    hasher.finalize().into()
}

/// Returns the first program profile's canonical application-basis proposition.
#[must_use]
pub fn echo_operation_anchored_node_application_basis_v1(
    node: NodeKey,
    attachment_type: TypeId,
    bytes: &[u8],
) -> EchoOperationApplicationBasisV1 {
    let atom_digest = echo_operation_atom_value_digest_v1(attachment_type, bytes);
    let mut hasher = Hasher::new();
    hasher.update(APPLICATION_BASIS_VALUE_DOMAIN);
    hasher.update(node.warp_id.as_bytes());
    hasher.update(node.local_id.as_bytes());
    hasher.update(&atom_digest);
    EchoOperationApplicationBasisV1::new(
        profile_digest(APPLICATION_BASIS_SCHEMA),
        hasher.finalize().into(),
    )
}

/// Closed occupancy proposition used by the create-if-absent application
/// basis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EchoOperationAnchoredNodeOccupancyV1 {
    /// Neither the node nor its alpha attachment exists.
    Absent,
    /// The node exists but its alpha attachment does not.
    NodeOnly,
    /// The alpha attachment exists without its owning node.
    AttachmentOnly,
    /// Both the node and its alpha attachment exist.
    NodeAndAttachment,
}

impl EchoOperationAnchoredNodeOccupancyV1 {
    const fn from_presence(node_present: bool, attachment_present: bool) -> Self {
        match (node_present, attachment_present) {
            (false, false) => Self::Absent,
            (true, false) => Self::NodeOnly,
            (false, true) => Self::AttachmentOnly,
            (true, true) => Self::NodeAndAttachment,
        }
    }

    const fn stable_code(self) -> u8 {
        match self {
            Self::Absent => 0,
            Self::NodeOnly => 1,
            Self::AttachmentOnly => 2,
            Self::NodeAndAttachment => 3,
        }
    }
}

/// Returns the create-if-absent profile's canonical application-basis
/// proposition for one exact occupancy state (ADR 0024).
///
/// This profile observes the node and attachment locations independently, so
/// a bare node and an orphan attachment cannot corroborate as total absence.
#[must_use]
pub fn echo_operation_anchored_node_creation_application_basis_v1(
    node: NodeKey,
    occupancy: EchoOperationAnchoredNodeOccupancyV1,
) -> EchoOperationApplicationBasisV1 {
    let mut hasher = Hasher::new();
    hasher.update(CREATE_APPLICATION_BASIS_DOMAIN);
    hasher.update(node.warp_id.as_bytes());
    hasher.update(node.local_id.as_bytes());
    hasher.update(&[occupancy.stable_code()]);
    EchoOperationApplicationBasisV1::new(
        profile_digest(CREATE_APPLICATION_BASIS_SCHEMA),
        hasher.finalize().into(),
    )
}

/// Returns the create-if-absent profile's total-absence application basis.
#[must_use]
pub fn echo_operation_anchored_node_absent_application_basis_v1(
    node: NodeKey,
) -> EchoOperationApplicationBasisV1 {
    echo_operation_anchored_node_creation_application_basis_v1(
        node,
        EchoOperationAnchoredNodeOccupancyV1::Absent,
    )
}

/// Returns the exact target-profile identity implemented by the v1 evaluator.
#[must_use]
pub fn echo_operation_target_profile_identity_v1() -> Hash {
    profile_digest(TARGET_PROFILE)
}

/// Returns the exact create-if-absent target-profile identity.
#[must_use]
pub fn echo_operation_create_if_absent_target_profile_identity_v1() -> Hash {
    profile_digest(CREATE_TARGET_PROFILE)
}

/// A three-axis resource budget for one bounded operation evaluation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EchoOperationBudgetV1 {
    steps: u64,
    read_bytes: u64,
    write_bytes: u64,
}

impl EchoOperationBudgetV1 {
    /// Creates one explicit budget.
    #[must_use]
    pub const fn new(steps: u64, read_bytes: u64, write_bytes: u64) -> Self {
        Self {
            steps,
            read_bytes,
            write_bytes,
        }
    }

    /// Returns the step allowance or consumption.
    #[must_use]
    pub const fn steps(self) -> u64 {
        self.steps
    }

    /// Returns the read-byte allowance or consumption.
    #[must_use]
    pub const fn read_bytes(self) -> u64 {
        self.read_bytes
    }

    /// Returns the write-byte allowance or consumption.
    #[must_use]
    pub const fn write_bytes(self) -> u64 {
        self.write_bytes
    }

    fn is_nonzero(self) -> bool {
        self.steps != 0
    }

    fn fits_within(self, ceiling: Self) -> bool {
        self.steps <= ceiling.steps
            && self.read_bytes <= ceiling.read_bytes
            && self.write_bytes <= ceiling.write_bytes
    }

    fn to_value(self) -> CanonicalValueV1 {
        map_value([
            ("read_bytes", uint_value(self.read_bytes)),
            ("steps", uint_value(self.steps)),
            ("write_bytes", uint_value(self.write_bytes)),
        ])
    }

    fn from_value(value: CanonicalValueV1) -> Result<Self, EchoOperationArtifactErrorV1> {
        let mut fields = exact_text_map(value, &["read_bytes", "steps", "write_bytes"])?;
        Ok(Self {
            steps: take_u64(&mut fields, "steps")?,
            read_bytes: take_u64(&mut fields, "read_bytes")?,
            write_bytes: take_u64(&mut fields, "write_bytes")?,
        })
    }
}

#[derive(Clone, Copy, Debug)]
struct EchoOperationBudgetMeterV1 {
    delegated: EchoOperationBudgetV1,
    consumed: EchoOperationBudgetV1,
}

impl EchoOperationBudgetMeterV1 {
    const fn new(delegated: EchoOperationBudgetV1) -> Self {
        Self {
            delegated,
            consumed: EchoOperationBudgetV1::new(0, 0, 0),
        }
    }

    fn charge(&mut self, steps: u64, read_bytes: u64, write_bytes: u64) -> bool {
        let Some(next_steps) = self.consumed.steps.checked_add(steps) else {
            return false;
        };
        let Some(next_read_bytes) = self.consumed.read_bytes.checked_add(read_bytes) else {
            return false;
        };
        let Some(next_write_bytes) = self.consumed.write_bytes.checked_add(write_bytes) else {
            return false;
        };
        let next = EchoOperationBudgetV1::new(next_steps, next_read_bytes, next_write_bytes);
        if !next.fits_within(self.delegated) {
            return false;
        }
        self.consumed = next;
        true
    }

    const fn consumed(self) -> EchoOperationBudgetV1 {
        self.consumed
    }
}

/// The declared footprint-derivation contract implemented by the first program profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EchoOperationFootprintContractV1 {
    /// Read one anchored node and alpha attachment; write that same attachment.
    AnchoredNodeAlphaExact,
    /// Read an absent anchored node and alpha attachment; create both.
    AnchoredNodeAlphaCreateIfAbsentExact,
}

impl EchoOperationFootprintContractV1 {
    const fn coordinate(self) -> &'static str {
        match self {
            Self::AnchoredNodeAlphaExact => FOOTPRINT_CONTRACT,
            Self::AnchoredNodeAlphaCreateIfAbsentExact => CREATE_FOOTPRINT_CONTRACT,
        }
    }
}

/// Data-only executable meaning interpreted by Echo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EchoOperationProgramV1 {
    /// Compare and replace one anchored typed node attachment.
    AnchoredNodeAttachmentCompareAndSet {
        /// Required skeleton node type.
        required_node_type: TypeId,
        /// Required alpha attachment atom type.
        required_attachment_type: TypeId,
        /// Maximum replacement byte count accepted by the program.
        max_replacement_bytes: u64,
    },
    /// Create one anchored typed node and alpha attachment only when both are absent.
    AnchoredNodeAttachmentCreateIfAbsent {
        /// Skeleton node type created by the program.
        required_node_type: TypeId,
        /// Alpha attachment atom type created by the program.
        required_attachment_type: TypeId,
        /// Maximum attachment byte count accepted by the program.
        max_replacement_bytes: u64,
    },
}

impl EchoOperationProgramV1 {
    /// Creates the first bounded, unique-match operation program.
    #[must_use]
    pub const fn anchored_node_attachment_compare_and_set(
        required_node_type: TypeId,
        required_attachment_type: TypeId,
        max_replacement_bytes: u64,
    ) -> Self {
        Self::AnchoredNodeAttachmentCompareAndSet {
            required_node_type,
            required_attachment_type,
            max_replacement_bytes,
        }
    }

    /// Creates the bounded create-if-absent operation program.
    #[must_use]
    pub const fn anchored_node_attachment_create_if_absent(
        required_node_type: TypeId,
        required_attachment_type: TypeId,
        max_replacement_bytes: u64,
    ) -> Self {
        Self::AnchoredNodeAttachmentCreateIfAbsent {
            required_node_type,
            required_attachment_type,
            max_replacement_bytes,
        }
    }

    /// Returns the exact program identity.
    pub fn identity(&self) -> Result<EchoOperationProgramIdV1, EchoOperationArtifactErrorV1> {
        Ok(EchoOperationProgramIdV1(domain_hash(
            PROGRAM_ID_DOMAIN,
            &self.to_canonical_bytes()?,
        )))
    }

    /// Encodes this program using Edict's canonical CBOR profile.
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, EchoOperationArtifactErrorV1> {
        self.validate_supported_profile()?;
        encode_canonical_cbor_v1(&self.to_value()).map_err(canonical_error)
    }

    fn footprint_contract(&self) -> EchoOperationFootprintContractV1 {
        match self {
            Self::AnchoredNodeAttachmentCompareAndSet { .. } => {
                EchoOperationFootprintContractV1::AnchoredNodeAlphaExact
            }
            Self::AnchoredNodeAttachmentCreateIfAbsent { .. } => {
                EchoOperationFootprintContractV1::AnchoredNodeAlphaCreateIfAbsentExact
            }
        }
    }

    const fn minimum_budget(&self) -> EchoOperationBudgetV1 {
        match self {
            Self::AnchoredNodeAttachmentCompareAndSet { .. } => {
                EchoOperationBudgetV1::new(4, 64, 32)
            }
            // Steps meter deterministic evaluator stages, not emitted
            // `WarpOp`s: node probe, attachment probe, and one atomic
            // create consequence. The consequence emits two operations but
            // remains one semantic rewrite step.
            Self::AnchoredNodeAttachmentCreateIfAbsent { .. } => {
                EchoOperationBudgetV1::new(3, 64, 64)
            }
        }
    }

    const fn input_schema(&self) -> &'static str {
        match self {
            Self::AnchoredNodeAttachmentCompareAndSet { .. } => INPUT_SCHEMA,
            Self::AnchoredNodeAttachmentCreateIfAbsent { .. } => CREATE_INPUT_SCHEMA,
        }
    }

    const fn result_schema(&self) -> &'static str {
        match self {
            Self::AnchoredNodeAttachmentCompareAndSet { .. } => RESULT_SCHEMA,
            Self::AnchoredNodeAttachmentCreateIfAbsent { .. } => CREATE_RESULT_SCHEMA,
        }
    }

    const fn obstruction_schema(&self) -> &'static str {
        match self {
            Self::AnchoredNodeAttachmentCompareAndSet { .. } => OBSTRUCTION_SCHEMA,
            Self::AnchoredNodeAttachmentCreateIfAbsent { .. } => CREATE_OBSTRUCTION_SCHEMA,
        }
    }

    const fn result_interpretation(&self) -> &'static str {
        match self {
            Self::AnchoredNodeAttachmentCompareAndSet { .. } => RESULT_INTERPRETATION,
            Self::AnchoredNodeAttachmentCreateIfAbsent { .. } => CREATE_RESULT_INTERPRETATION,
        }
    }

    const fn obstruction_interpretation(&self) -> &'static str {
        match self {
            Self::AnchoredNodeAttachmentCompareAndSet { .. } => OBSTRUCTION_INTERPRETATION,
            Self::AnchoredNodeAttachmentCreateIfAbsent { .. } => CREATE_OBSTRUCTION_INTERPRETATION,
        }
    }

    const fn application_basis_schema(&self) -> &'static str {
        match self {
            Self::AnchoredNodeAttachmentCompareAndSet { .. } => APPLICATION_BASIS_SCHEMA,
            Self::AnchoredNodeAttachmentCreateIfAbsent { .. } => CREATE_APPLICATION_BASIS_SCHEMA,
        }
    }

    const fn target_profile(&self) -> &'static str {
        match self {
            Self::AnchoredNodeAttachmentCompareAndSet { .. } => TARGET_PROFILE,
            Self::AnchoredNodeAttachmentCreateIfAbsent { .. } => CREATE_TARGET_PROFILE,
        }
    }

    const fn accepts_invocation_kind(&self, kind: EchoOperationInvocationKindV1) -> bool {
        matches!(
            (self, kind),
            (
                Self::AnchoredNodeAttachmentCompareAndSet { .. },
                EchoOperationInvocationKindV1::AnchoredNodeAttachmentCompareAndSet { .. }
            ) | (
                Self::AnchoredNodeAttachmentCreateIfAbsent { .. },
                EchoOperationInvocationKindV1::AnchoredNodeAttachmentCreateIfAbsent
            )
        )
    }

    fn validate_supported_profile(&self) -> Result<(), EchoOperationArtifactErrorV1> {
        match self {
            Self::AnchoredNodeAttachmentCompareAndSet {
                max_replacement_bytes,
                ..
            }
            | Self::AnchoredNodeAttachmentCreateIfAbsent {
                max_replacement_bytes,
                ..
            } if *max_replacement_bytes == 0 => Err(artifact_error(
                EchoOperationArtifactErrorKindV1::UnsupportedProgram,
                "program replacement bound must be nonzero",
            )),
            Self::AnchoredNodeAttachmentCompareAndSet { .. }
            | Self::AnchoredNodeAttachmentCreateIfAbsent { .. } => Ok(()),
        }
    }

    fn to_value(&self) -> CanonicalValueV1 {
        let (kind, required_node_type, required_attachment_type, max_replacement_bytes) = match self
        {
            Self::AnchoredNodeAttachmentCompareAndSet {
                required_node_type,
                required_attachment_type,
                max_replacement_bytes,
            } => (
                PROGRAM_KIND,
                required_node_type,
                required_attachment_type,
                max_replacement_bytes,
            ),
            Self::AnchoredNodeAttachmentCreateIfAbsent {
                required_node_type,
                required_attachment_type,
                max_replacement_bytes,
            } => (
                CREATE_PROGRAM_KIND,
                required_node_type,
                required_attachment_type,
                max_replacement_bytes,
            ),
        };
        map_value([
            (
                "interpreter_profile_identity",
                hash_value(profile_digest(INTERPRETER_PROFILE)),
            ),
            (
                "intrinsic_profile_identity",
                hash_value(profile_digest(INTRINSIC_PROFILE)),
            ),
            ("kind", text_value(kind)),
            ("max_replacement_bytes", uint_value(*max_replacement_bytes)),
            (
                "required_attachment_type",
                hash_value(required_attachment_type.0),
            ),
            ("required_node_type", hash_value(required_node_type.0)),
            ("schema", text_value(PROGRAM_SCHEMA)),
        ])
    }

    fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, EchoOperationArtifactErrorV1> {
        let value = decode_canonical_cbor_v1(bytes).map_err(canonical_error)?;
        let mut fields = exact_text_map(
            value,
            &[
                "interpreter_profile_identity",
                "intrinsic_profile_identity",
                "kind",
                "max_replacement_bytes",
                "required_attachment_type",
                "required_node_type",
                "schema",
            ],
        )?;
        require_text(&mut fields, "schema", PROGRAM_SCHEMA)?;
        let kind = take_text(&mut fields, "kind")?;
        let interpreter_profile_identity = take_hash(&mut fields, "interpreter_profile_identity")?;
        let intrinsic_profile_identity = take_hash(&mut fields, "intrinsic_profile_identity")?;
        if interpreter_profile_identity != profile_digest(INTERPRETER_PROFILE)
            || intrinsic_profile_identity != profile_digest(INTRINSIC_PROFILE)
        {
            return Err(artifact_error(
                EchoOperationArtifactErrorKindV1::UnsupportedTargetProfile,
                "operation program names an unsupported interpreter or intrinsic profile",
            ));
        }
        let required_node_type = TypeId(take_hash(&mut fields, "required_node_type")?);
        let required_attachment_type = TypeId(take_hash(&mut fields, "required_attachment_type")?);
        let max_replacement_bytes = take_u64(&mut fields, "max_replacement_bytes")?;
        let program = match kind.as_str() {
            PROGRAM_KIND => Self::anchored_node_attachment_compare_and_set(
                required_node_type,
                required_attachment_type,
                max_replacement_bytes,
            ),
            CREATE_PROGRAM_KIND => Self::anchored_node_attachment_create_if_absent(
                required_node_type,
                required_attachment_type,
                max_replacement_bytes,
            ),
            _ => {
                return Err(artifact_error(
                    EchoOperationArtifactErrorKindV1::UnsupportedProgram,
                    "operation program names an unsupported kind",
                ));
            }
        };
        program.validate_supported_profile()?;
        Ok(program)
    }
}

/// Application- and Edict-owned identities that close one executable meaning.
///
/// Echo treats these as opaque substitution evidence. It does not infer source,
/// Core, Target IR, application schema, or lawpack identity from program bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EchoOperationSemanticClosureV1 {
    edict_source_identity: Hash,
    canonical_meaning_identity: Hash,
    core_identity: Hash,
    target_ir_identity: Hash,
    application_schema_coordinate: String,
    application_schema_identity: Hash,
    lawpack_coordinate: String,
    lawpack_identity: Hash,
}

impl EchoOperationSemanticClosureV1 {
    /// Creates the exact upstream semantic closure bound by one package.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        edict_source_identity: Hash,
        canonical_meaning_identity: Hash,
        core_identity: Hash,
        target_ir_identity: Hash,
        application_schema_coordinate: impl Into<String>,
        application_schema_identity: Hash,
        lawpack_coordinate: impl Into<String>,
        lawpack_identity: Hash,
    ) -> Self {
        Self {
            edict_source_identity,
            canonical_meaning_identity,
            core_identity,
            target_ir_identity,
            application_schema_coordinate: application_schema_coordinate.into(),
            application_schema_identity,
            lawpack_coordinate: lawpack_coordinate.into(),
            lawpack_identity,
        }
    }

    /// Returns the canonical Edict meaning identity.
    #[must_use]
    pub const fn canonical_meaning_identity(&self) -> Hash {
        self.canonical_meaning_identity
    }

    /// Returns the application-owned lawpack identity.
    #[must_use]
    pub const fn lawpack_identity(&self) -> Hash {
        self.lawpack_identity
    }

    fn validate(&self) -> Result<(), EchoOperationArtifactErrorV1> {
        if self.application_schema_coordinate.is_empty() || self.lawpack_coordinate.is_empty() {
            return Err(invalid_structure(
                "application schema and lawpack coordinates must not be empty",
            ));
        }
        Ok(())
    }

    fn to_value(&self) -> CanonicalValueV1 {
        map_value([
            (
                "application_schema_coordinate",
                text_value(&self.application_schema_coordinate),
            ),
            (
                "application_schema_identity",
                hash_value(self.application_schema_identity),
            ),
            (
                "canonical_meaning_identity",
                hash_value(self.canonical_meaning_identity),
            ),
            ("core_identity", hash_value(self.core_identity)),
            (
                "edict_source_identity",
                hash_value(self.edict_source_identity),
            ),
            ("lawpack_coordinate", text_value(&self.lawpack_coordinate)),
            ("lawpack_identity", hash_value(self.lawpack_identity)),
            ("target_ir_identity", hash_value(self.target_ir_identity)),
        ])
    }

    fn from_value(value: CanonicalValueV1) -> Result<Self, EchoOperationArtifactErrorV1> {
        let mut fields = exact_text_map(
            value,
            &[
                "application_schema_coordinate",
                "application_schema_identity",
                "canonical_meaning_identity",
                "core_identity",
                "edict_source_identity",
                "lawpack_coordinate",
                "lawpack_identity",
                "target_ir_identity",
            ],
        )?;
        let closure = Self {
            edict_source_identity: take_hash(&mut fields, "edict_source_identity")?,
            canonical_meaning_identity: take_hash(&mut fields, "canonical_meaning_identity")?,
            core_identity: take_hash(&mut fields, "core_identity")?,
            target_ir_identity: take_hash(&mut fields, "target_ir_identity")?,
            application_schema_coordinate: take_text(&mut fields, "application_schema_coordinate")?,
            application_schema_identity: take_hash(&mut fields, "application_schema_identity")?,
            lawpack_coordinate: take_text(&mut fields, "lawpack_coordinate")?,
            lawpack_identity: take_hash(&mut fields, "lawpack_identity")?,
        };
        closure.validate()?;
        Ok(closure)
    }
}

/// Exact executable-operation publication material and provenance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutableOperationPackageV1 {
    operation_coordinate: String,
    semantic_closure: EchoOperationSemanticClosureV1,
    target_profile_identity: Hash,
    interpreter_profile_identity: Hash,
    intrinsic_profile_identity: Hash,
    authority_profile_identity: Hash,
    application_basis_schema_identity: Hash,
    input_schema_identity: Hash,
    result_schema_identity: Hash,
    obstruction_schema_identity: Hash,
    result_interpretation_identity: Hash,
    obstruction_interpretation_identity: Hash,
    evaluation_basis_schema_identity: Hash,
    footprint_contract_identity: Hash,
    budget_ceiling: EchoOperationBudgetV1,
    program: EchoOperationProgramV1,
}

impl ExecutableOperationPackageV1 {
    /// Creates publication material for the supported bounded operation profile.
    #[must_use]
    pub fn new(
        operation_coordinate: impl Into<String>,
        semantic_closure: EchoOperationSemanticClosureV1,
        target_profile_identity: Hash,
        authority_profile_identity: Hash,
        budget_ceiling: EchoOperationBudgetV1,
        program: EchoOperationProgramV1,
    ) -> Self {
        let input_schema_identity = profile_digest(program.input_schema());
        let result_schema_identity = profile_digest(program.result_schema());
        let obstruction_schema_identity = profile_digest(program.obstruction_schema());
        let result_interpretation_identity = profile_digest(program.result_interpretation());
        let obstruction_interpretation_identity =
            profile_digest(program.obstruction_interpretation());
        let application_basis_schema_identity = profile_digest(program.application_basis_schema());
        Self {
            operation_coordinate: operation_coordinate.into(),
            semantic_closure,
            target_profile_identity,
            interpreter_profile_identity: profile_digest(INTERPRETER_PROFILE),
            intrinsic_profile_identity: profile_digest(INTRINSIC_PROFILE),
            authority_profile_identity,
            input_schema_identity,
            result_schema_identity,
            obstruction_schema_identity,
            result_interpretation_identity,
            obstruction_interpretation_identity,
            application_basis_schema_identity,
            evaluation_basis_schema_identity: profile_digest(BASIS_SCHEMA),
            footprint_contract_identity: profile_digest(program.footprint_contract().coordinate()),
            budget_ceiling,
            program,
        }
    }

    /// Returns the public operation coordinate.
    #[must_use]
    pub fn operation_coordinate(&self) -> &str {
        &self.operation_coordinate
    }

    /// Returns the Edict semantic identity bound by this package.
    #[must_use]
    pub const fn semantic_identity(&self) -> Hash {
        self.semantic_closure.canonical_meaning_identity
    }

    /// Returns the application-owned lawpack identity bound by this package.
    #[must_use]
    pub const fn lawpack_identity(&self) -> Hash {
        self.semantic_closure.lawpack_identity
    }

    /// Returns the Echo target-profile identity bound by this package.
    #[must_use]
    pub const fn target_profile_identity(&self) -> Hash {
        self.target_profile_identity
    }

    /// Returns the required authority-profile identity.
    #[must_use]
    pub const fn authority_profile_identity(&self) -> Hash {
        self.authority_profile_identity
    }

    /// Returns the package budget ceiling.
    #[must_use]
    pub const fn budget_ceiling(&self) -> EchoOperationBudgetV1 {
        self.budget_ceiling
    }

    /// Returns the executable program.
    #[must_use]
    pub const fn program(&self) -> &EchoOperationProgramV1 {
        &self.program
    }

    /// Encodes exact package bytes using Edict's canonical CBOR profile.
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, EchoOperationArtifactErrorV1> {
        if self.operation_coordinate.is_empty() {
            return Err(artifact_error(
                EchoOperationArtifactErrorKindV1::EmptyOperationCoordinate,
                "operation coordinate must not be empty",
            ));
        }
        if !self.budget_ceiling.is_nonzero() {
            return Err(artifact_error(
                EchoOperationArtifactErrorKindV1::InvalidBudget,
                "package step budget must be nonzero",
            ));
        }
        if !self
            .program
            .minimum_budget()
            .fits_within(self.budget_ceiling)
        {
            return Err(artifact_error(
                EchoOperationArtifactErrorKindV1::InvalidBudget,
                "package budget cannot complete the program's smallest lawful evaluation",
            ));
        }
        self.semantic_closure.validate()?;
        let program_bytes = self.program.to_canonical_bytes()?;
        let value = map_value([
            (
                "application_basis_schema_identity",
                hash_value(self.application_basis_schema_identity),
            ),
            (
                "authority_profile_identity",
                hash_value(self.authority_profile_identity),
            ),
            ("budget_ceiling", self.budget_ceiling.to_value()),
            (
                "evaluation_basis_schema_identity",
                hash_value(self.evaluation_basis_schema_identity),
            ),
            (
                "footprint_contract_identity",
                hash_value(self.footprint_contract_identity),
            ),
            (
                "interpreter_profile_identity",
                hash_value(self.interpreter_profile_identity),
            ),
            (
                "input_schema_identity",
                hash_value(self.input_schema_identity),
            ),
            (
                "intrinsic_profile_identity",
                hash_value(self.intrinsic_profile_identity),
            ),
            (
                "obstruction_schema_identity",
                hash_value(self.obstruction_schema_identity),
            ),
            (
                "obstruction_interpretation_identity",
                hash_value(self.obstruction_interpretation_identity),
            ),
            (
                "operation_coordinate",
                text_value(&self.operation_coordinate),
            ),
            ("program", CanonicalValueV1::Bytes(program_bytes)),
            (
                "result_schema_identity",
                hash_value(self.result_schema_identity),
            ),
            (
                "result_interpretation_identity",
                hash_value(self.result_interpretation_identity),
            ),
            ("schema", text_value(PACKAGE_SCHEMA)),
            ("semantic_closure", self.semantic_closure.to_value()),
            (
                "target_profile_identity",
                hash_value(self.target_profile_identity),
            ),
        ]);
        encode_canonical_cbor_v1(&value).map_err(canonical_error)
    }

    fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, EchoOperationArtifactErrorV1> {
        let value = decode_canonical_cbor_v1(bytes).map_err(canonical_error)?;
        let mut fields = exact_text_map(
            value,
            &[
                "authority_profile_identity",
                "application_basis_schema_identity",
                "budget_ceiling",
                "evaluation_basis_schema_identity",
                "footprint_contract_identity",
                "interpreter_profile_identity",
                "input_schema_identity",
                "intrinsic_profile_identity",
                "obstruction_schema_identity",
                "obstruction_interpretation_identity",
                "operation_coordinate",
                "program",
                "result_schema_identity",
                "result_interpretation_identity",
                "schema",
                "semantic_closure",
                "target_profile_identity",
            ],
        )?;
        require_text(&mut fields, "schema", PACKAGE_SCHEMA)?;
        let operation_coordinate = take_text(&mut fields, "operation_coordinate")?;
        if operation_coordinate.is_empty() {
            return Err(artifact_error(
                EchoOperationArtifactErrorKindV1::EmptyOperationCoordinate,
                "operation coordinate must not be empty",
            ));
        }
        let program_bytes = take_bytes(&mut fields, "program")?;
        let program = EchoOperationProgramV1::from_canonical_bytes(&program_bytes)?;
        let package = Self {
            operation_coordinate,
            semantic_closure: EchoOperationSemanticClosureV1::from_value(take_field(
                &mut fields,
                "semantic_closure",
            )?)?,
            target_profile_identity: take_hash(&mut fields, "target_profile_identity")?,
            interpreter_profile_identity: take_hash(&mut fields, "interpreter_profile_identity")?,
            intrinsic_profile_identity: take_hash(&mut fields, "intrinsic_profile_identity")?,
            authority_profile_identity: take_hash(&mut fields, "authority_profile_identity")?,
            input_schema_identity: take_hash(&mut fields, "input_schema_identity")?,
            result_schema_identity: take_hash(&mut fields, "result_schema_identity")?,
            obstruction_schema_identity: take_hash(&mut fields, "obstruction_schema_identity")?,
            result_interpretation_identity: take_hash(
                &mut fields,
                "result_interpretation_identity",
            )?,
            obstruction_interpretation_identity: take_hash(
                &mut fields,
                "obstruction_interpretation_identity",
            )?,
            application_basis_schema_identity: take_hash(
                &mut fields,
                "application_basis_schema_identity",
            )?,
            evaluation_basis_schema_identity: take_hash(
                &mut fields,
                "evaluation_basis_schema_identity",
            )?,
            footprint_contract_identity: take_hash(&mut fields, "footprint_contract_identity")?,
            budget_ceiling: EchoOperationBudgetV1::from_value(take_field(
                &mut fields,
                "budget_ceiling",
            )?)?,
            program,
        };
        package.self_validate_supported_profile()?;
        if package.to_canonical_bytes()? != bytes {
            return Err(artifact_error(
                EchoOperationArtifactErrorKindV1::NonCanonical,
                "package did not reproduce the exact admitted bytes",
            ));
        }
        Ok(package)
    }

    fn self_validate_supported_profile(&self) -> Result<(), EchoOperationArtifactErrorV1> {
        if !self.budget_ceiling.is_nonzero() {
            return Err(artifact_error(
                EchoOperationArtifactErrorKindV1::InvalidBudget,
                "package step budget must be nonzero",
            ));
        }
        let expected = [
            (
                "input schema",
                self.input_schema_identity,
                profile_digest(self.program.input_schema()),
            ),
            (
                "result schema",
                self.result_schema_identity,
                profile_digest(self.program.result_schema()),
            ),
            (
                "obstruction schema",
                self.obstruction_schema_identity,
                profile_digest(self.program.obstruction_schema()),
            ),
            (
                "result interpretation",
                self.result_interpretation_identity,
                profile_digest(self.program.result_interpretation()),
            ),
            (
                "obstruction interpretation",
                self.obstruction_interpretation_identity,
                profile_digest(self.program.obstruction_interpretation()),
            ),
            (
                "application basis schema",
                self.application_basis_schema_identity,
                profile_digest(self.program.application_basis_schema()),
            ),
            (
                "evaluation basis schema",
                self.evaluation_basis_schema_identity,
                profile_digest(BASIS_SCHEMA),
            ),
            (
                "footprint contract",
                self.footprint_contract_identity,
                profile_digest(self.program.footprint_contract().coordinate()),
            ),
        ];
        if let Some((label, _, _)) = expected
            .iter()
            .find(|(_, actual, required)| actual != required)
        {
            return Err(artifact_error(
                EchoOperationArtifactErrorKindV1::UnsupportedSchema,
                format!("unsupported {label} identity"),
            ));
        }
        if self.target_profile_identity != profile_digest(self.program.target_profile()) {
            return Err(artifact_error(
                EchoOperationArtifactErrorKindV1::UnsupportedTargetProfile,
                "unsupported Echo target profile identity",
            ));
        }
        if self.interpreter_profile_identity != profile_digest(INTERPRETER_PROFILE)
            || self.intrinsic_profile_identity != profile_digest(INTRINSIC_PROFILE)
        {
            return Err(artifact_error(
                EchoOperationArtifactErrorKindV1::UnsupportedTargetProfile,
                "unsupported interpreter or intrinsic profile identity",
            ));
        }
        Ok(())
    }
}

/// Stable package decode/self-validation failure categories.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EchoOperationArtifactErrorKindV1 {
    /// Canonical bytes were malformed or outside the accepted value model.
    MalformedCanonicalBytes,
    /// The decoded value did not have the exact supported structure.
    InvalidStructure,
    /// Exact canonical bytes were not supplied.
    NonCanonical,
    /// The public operation coordinate was empty.
    EmptyOperationCoordinate,
    /// The target profile is not implemented by this Echo runtime.
    UnsupportedTargetProfile,
    /// A schema or footprint profile is not implemented by this runtime.
    UnsupportedSchema,
    /// The program kind or its values are unsupported.
    UnsupportedProgram,
    /// The budget is malformed or cannot execute any step.
    InvalidBudget,
}

/// One structured artifact failure.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("{kind:?}: {detail}")]
pub struct EchoOperationArtifactErrorV1 {
    kind: EchoOperationArtifactErrorKindV1,
    detail: String,
}

impl EchoOperationArtifactErrorV1 {
    /// Returns the stable failure category.
    #[must_use]
    pub const fn kind(&self) -> EchoOperationArtifactErrorKindV1 {
        self.kind
    }

    /// Returns deterministic diagnostic detail.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

/// Independently pinned policy for the package-admission crossing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EchoOperationAdmissionPolicyV1 {
    expected_package_id: EchoOperationPackageIdV1,
    expected_operation_coordinate: String,
    expected_authority_profile_identity: Hash,
    maximum_budget: EchoOperationBudgetV1,
}

impl EchoOperationAdmissionPolicyV1 {
    /// Pins the exact package, public operation, authority profile, and budget ceiling.
    #[must_use]
    pub fn exact(
        expected_package_id: EchoOperationPackageIdV1,
        expected_operation_coordinate: impl Into<String>,
        expected_authority_profile_identity: Hash,
        maximum_budget: EchoOperationBudgetV1,
    ) -> Self {
        Self {
            expected_package_id,
            expected_operation_coordinate: expected_operation_coordinate.into(),
            expected_authority_profile_identity,
            maximum_budget,
        }
    }

    fn identity(&self) -> Hash {
        let mut hasher = Hasher::new();
        hasher.update(b"echo:operation-admission-policy:v1\0");
        hasher.update(&self.expected_package_id.as_hash());
        hash_len_bytes(&mut hasher, self.expected_operation_coordinate.as_bytes());
        hasher.update(&self.expected_authority_profile_identity);
        hash_budget(&mut hasher, self.maximum_budget);
        hasher.finalize().into()
    }

    fn to_value(&self) -> CanonicalValueV1 {
        map_value([
            (
                "expected_authority_profile_identity",
                hash_value(self.expected_authority_profile_identity),
            ),
            (
                "expected_operation_coordinate",
                text_value(&self.expected_operation_coordinate),
            ),
            (
                "expected_package_id",
                hash_value(self.expected_package_id.as_hash()),
            ),
            ("maximum_budget", self.maximum_budget.to_value()),
        ])
    }

    fn from_value(value: CanonicalValueV1) -> Result<Self, EchoOperationArtifactErrorV1> {
        let mut fields = exact_text_map(
            value,
            &[
                "expected_authority_profile_identity",
                "expected_operation_coordinate",
                "expected_package_id",
                "maximum_budget",
            ],
        )?;
        let policy = Self {
            expected_package_id: EchoOperationPackageIdV1(take_hash(
                &mut fields,
                "expected_package_id",
            )?),
            expected_operation_coordinate: take_text(&mut fields, "expected_operation_coordinate")?,
            expected_authority_profile_identity: take_hash(
                &mut fields,
                "expected_authority_profile_identity",
            )?,
            maximum_budget: EchoOperationBudgetV1::from_value(take_field(
                &mut fields,
                "maximum_budget",
            )?)?,
        };
        if policy.expected_operation_coordinate.is_empty() || !policy.maximum_budget.is_nonzero() {
            return Err(invalid_structure(
                "package-admission policy coordinate and budget must be nonempty",
            ));
        }
        Ok(policy)
    }
}

/// Stable package-admission refusal categories.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EchoOperationAdmissionErrorKindV1 {
    /// Exact package bytes were malformed or unsupported.
    ArtifactInvalid,
    /// Exact package bytes did not match the independently pinned package id.
    PackageIdentityMismatch,
    /// The public operation coordinate differed from policy.
    OperationCoordinateMismatch,
    /// The authority profile differed from policy.
    AuthorityProfileMismatch,
    /// The package budget exceeded policy.
    BudgetExceedsPolicy,
}

/// One structured package-admission refusal.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("{kind:?}: {detail}")]
pub struct EchoOperationAdmissionErrorV1 {
    kind: EchoOperationAdmissionErrorKindV1,
    detail: String,
    artifact: Option<EchoOperationArtifactErrorV1>,
}

impl EchoOperationAdmissionErrorV1 {
    /// Returns the stable refusal category.
    #[must_use]
    pub const fn kind(&self) -> EchoOperationAdmissionErrorKindV1 {
        self.kind
    }

    /// Returns the artifact failure, when admission failed during decoding.
    #[must_use]
    pub const fn artifact(&self) -> Option<&EchoOperationArtifactErrorV1> {
        self.artifact.as_ref()
    }
}

/// Opaque evidence that Echo admitted exact package bytes under separate policy.
#[derive(Clone, Debug)]
pub struct AdmittedExecutableOperationPackageV1 {
    package: ExecutableOperationPackageV1,
    canonical_package_bytes: Vec<u8>,
    package_id: EchoOperationPackageIdV1,
    admission_policy_id: Hash,
    admission_id: EchoOperationPackageAdmissionIdV1,
    admission_policy: EchoOperationAdmissionPolicyV1,
}

/// Installed executable meaning. Installation does not itself authorize an invocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstalledEchoOperationV1 {
    installed_operation_id: InstalledEchoOperationIdV1,
    package_id: EchoOperationPackageIdV1,
    package_admission_id: EchoOperationPackageAdmissionIdV1,
    operation_coordinate: String,
    semantic_identity: Hash,
    lawpack_identity: Hash,
    target_profile_identity: Hash,
    interpreter_profile_identity: Hash,
    intrinsic_profile_identity: Hash,
    authority_profile_identity: Hash,
    application_basis_schema_identity: Hash,
    budget_ceiling: EchoOperationBudgetV1,
    program_id: EchoOperationProgramIdV1,
    program: EchoOperationProgramV1,
    canonical_package_bytes: Vec<u8>,
    admission_policy_id: Hash,
    admission_policy: EchoOperationAdmissionPolicyV1,
}

impl InstalledEchoOperationV1 {
    /// Returns Echo's installed-operation identity.
    #[must_use]
    pub const fn installed_operation_id(&self) -> InstalledEchoOperationIdV1 {
        self.installed_operation_id
    }

    /// Returns the public operation coordinate admitted by Echo.
    #[must_use]
    pub fn operation_coordinate(&self) -> &str {
        &self.operation_coordinate
    }

    /// Returns the exact package identity.
    #[must_use]
    pub const fn package_id(&self) -> EchoOperationPackageIdV1 {
        self.package_id
    }

    /// Returns the package-admission evidence identity consumed by installation.
    #[must_use]
    pub const fn package_admission_id(&self) -> EchoOperationPackageAdmissionIdV1 {
        self.package_admission_id
    }

    /// Returns the subordinate executable-program identity.
    #[must_use]
    pub const fn program_id(&self) -> EchoOperationProgramIdV1 {
        self.program_id
    }

    pub(crate) const fn program(&self) -> &EchoOperationProgramV1 {
        &self.program
    }

    /// Returns the semantic identity bound by the admitted package.
    #[must_use]
    pub const fn semantic_identity(&self) -> Hash {
        self.semantic_identity
    }

    /// Returns the lawpack identity bound by the admitted package.
    #[must_use]
    pub const fn lawpack_identity(&self) -> Hash {
        self.lawpack_identity
    }

    /// Returns the exact retained package bytes.
    #[must_use]
    pub fn canonical_package_bytes(&self) -> &[u8] {
        &self.canonical_package_bytes
    }
}

/// Stable installation refusal categories.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EchoOperationInstallationErrorKindV1 {
    /// The package id is already installed with different exact evidence.
    PackageIdentityConflict,
    /// The public operation coordinate is already bound to another package.
    OperationCoordinateConflict,
}

/// One structured installation refusal.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("{kind:?}: {detail}")]
pub struct EchoOperationInstallationErrorV1 {
    kind: EchoOperationInstallationErrorKindV1,
    detail: String,
}

impl EchoOperationInstallationErrorV1 {
    /// Returns the stable refusal category.
    #[must_use]
    pub const fn kind(&self) -> EchoOperationInstallationErrorKindV1 {
        self.kind
    }
}

pub(crate) fn admit_package_v1(
    policy: &EchoOperationAdmissionPolicyV1,
    canonical_package_bytes: Vec<u8>,
) -> Result<AdmittedExecutableOperationPackageV1, EchoOperationAdmissionErrorV1> {
    let package_id = echo_operation_package_id_v1(&canonical_package_bytes);
    if package_id != policy.expected_package_id {
        return Err(admission_error(
            EchoOperationAdmissionErrorKindV1::PackageIdentityMismatch,
            "exact package bytes differ from the independently pinned package id",
        ));
    }
    let package = ExecutableOperationPackageV1::from_canonical_bytes(&canonical_package_bytes)
        .map_err(|artifact| EchoOperationAdmissionErrorV1 {
            kind: EchoOperationAdmissionErrorKindV1::ArtifactInvalid,
            detail: artifact.to_string(),
            artifact: Some(artifact),
        })?;
    if package.operation_coordinate != policy.expected_operation_coordinate {
        return Err(admission_error(
            EchoOperationAdmissionErrorKindV1::OperationCoordinateMismatch,
            "package operation coordinate differs from admission policy",
        ));
    }
    if package.authority_profile_identity != policy.expected_authority_profile_identity {
        return Err(admission_error(
            EchoOperationAdmissionErrorKindV1::AuthorityProfileMismatch,
            "package authority profile differs from admission policy",
        ));
    }
    if !package.budget_ceiling.fits_within(policy.maximum_budget) {
        return Err(admission_error(
            EchoOperationAdmissionErrorKindV1::BudgetExceedsPolicy,
            "package budget ceiling exceeds admission policy",
        ));
    }
    let admission_policy_id = policy.identity();
    Ok(AdmittedExecutableOperationPackageV1 {
        package,
        canonical_package_bytes,
        package_id,
        admission_policy_id,
        admission_id: package_admission_id(package_id, admission_policy_id),
        admission_policy: policy.clone(),
    })
}

pub(crate) fn installed_from_admitted(
    admitted: AdmittedExecutableOperationPackageV1,
) -> Result<InstalledEchoOperationV1, EchoOperationArtifactErrorV1> {
    let package = admitted.package;
    let mut installed = InstalledEchoOperationV1 {
        installed_operation_id: InstalledEchoOperationIdV1([0; 32]),
        package_id: admitted.package_id,
        package_admission_id: admitted.admission_id,
        operation_coordinate: package.operation_coordinate,
        semantic_identity: package.semantic_closure.canonical_meaning_identity,
        lawpack_identity: package.semantic_closure.lawpack_identity,
        target_profile_identity: package.target_profile_identity,
        interpreter_profile_identity: package.interpreter_profile_identity,
        intrinsic_profile_identity: package.intrinsic_profile_identity,
        authority_profile_identity: package.authority_profile_identity,
        application_basis_schema_identity: package.application_basis_schema_identity,
        budget_ceiling: package.budget_ceiling,
        program_id: package.program.identity()?,
        program: package.program,
        canonical_package_bytes: admitted.canonical_package_bytes,
        admission_policy_id: admitted.admission_policy_id,
        admission_policy: admitted.admission_policy,
    };
    installed.installed_operation_id = installed_operation_id(&installed);
    Ok(installed)
}

pub(crate) fn retain_installation_v1(
    installed: &InstalledEchoOperationV1,
) -> Result<Vec<u8>, EchoOperationArtifactErrorV1> {
    encode_canonical_cbor_v1(&map_value([
        ("admission_policy", installed.admission_policy.to_value()),
        (
            "admission_policy_id",
            hash_value(installed.admission_policy_id),
        ),
        (
            "installed_operation_id",
            hash_value(installed.installed_operation_id.as_hash()),
        ),
        (
            "package_bytes",
            CanonicalValueV1::Bytes(installed.canonical_package_bytes.clone()),
        ),
        (
            "package_admission_id",
            hash_value(installed.package_admission_id.as_hash()),
        ),
        ("package_id", hash_value(installed.package_id.as_hash())),
        ("schema", text_value("echo.operation-installation/v1")),
    ]))
    .map_err(canonical_error)
}

pub(crate) fn recover_installation_v1(
    bytes: &[u8],
) -> Result<InstalledEchoOperationV1, EchoOperationArtifactErrorV1> {
    let value = decode_canonical_cbor_v1(bytes).map_err(canonical_error)?;
    let mut fields = exact_text_map(
        value,
        &[
            "admission_policy_id",
            "admission_policy",
            "installed_operation_id",
            "package_bytes",
            "package_admission_id",
            "package_id",
            "schema",
        ],
    )?;
    require_text(&mut fields, "schema", "echo.operation-installation/v1")?;
    let package_id = EchoOperationPackageIdV1(take_hash(&mut fields, "package_id")?);
    let canonical_package_bytes = take_bytes(&mut fields, "package_bytes")?;
    if echo_operation_package_id_v1(&canonical_package_bytes) != package_id {
        return Err(invalid_structure(
            "retained installation package identity does not match exact bytes",
        ));
    }
    let admission_policy =
        EchoOperationAdmissionPolicyV1::from_value(take_field(&mut fields, "admission_policy")?)?;
    let admission_policy_id = take_hash(&mut fields, "admission_policy_id")?;
    if admission_policy.identity() != admission_policy_id {
        return Err(invalid_structure(
            "retained package-admission policy identity mismatch",
        ));
    }
    let package_admission_id = package_admission_id(package_id, admission_policy_id);
    let retained_package_admission_id =
        EchoOperationPackageAdmissionIdV1(take_hash(&mut fields, "package_admission_id")?);
    if retained_package_admission_id != package_admission_id {
        return Err(invalid_structure(
            "retained package-admission identity does not match package and policy",
        ));
    }
    let admitted =
        admit_package_v1(&admission_policy, canonical_package_bytes).map_err(|error| {
            invalid_structure(format!(
                "retained executable-operation package no longer admits: {error}"
            ))
        })?;
    if admitted.package_id != package_id || admitted.admission_id != package_admission_id {
        return Err(invalid_structure(
            "retained package-admission evidence does not match re-admission",
        ));
    }
    let installed = installed_from_admitted(admitted)?;
    let retained_installed_operation_id =
        InstalledEchoOperationIdV1(take_hash(&mut fields, "installed_operation_id")?);
    if retained_installed_operation_id != installed.installed_operation_id {
        return Err(invalid_structure(
            "retained installed-operation identity does not match exact installation",
        ));
    }
    if retain_installation_v1(&installed)? != bytes {
        return Err(artifact_error(
            EchoOperationArtifactErrorKindV1::NonCanonical,
            "installation did not reproduce the exact retained bytes",
        ));
    }
    Ok(installed)
}

pub(crate) fn install_recovered_v1(
    packages: &mut BTreeMap<EchoOperationPackageIdV1, InstalledEchoOperationV1>,
    operations: &mut BTreeMap<String, EchoOperationPackageIdV1>,
    installed: InstalledEchoOperationV1,
) -> Result<InstalledEchoOperationV1, EchoOperationInstallationErrorV1> {
    if let Some(existing) = packages.get(&installed.package_id) {
        if existing == &installed {
            return Ok(existing.clone());
        }
        return Err(installation_error(
            EchoOperationInstallationErrorKindV1::PackageIdentityConflict,
            "recovered package conflicts with installed exact evidence",
        ));
    }
    if operations
        .get(&installed.operation_coordinate)
        .is_some_and(|existing| *existing != installed.package_id)
    {
        return Err(installation_error(
            EchoOperationInstallationErrorKindV1::OperationCoordinateConflict,
            "recovered operation coordinate conflicts with installed package",
        ));
    }
    operations.insert(installed.operation_coordinate.clone(), installed.package_id);
    packages.insert(installed.package_id, installed.clone());
    Ok(installed)
}

/// Application-owned basis proposition carried inside the exact Echo basis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EchoOperationApplicationBasisV1 {
    schema_identity: Hash,
    value_identity: Hash,
}

impl EchoOperationApplicationBasisV1 {
    /// Creates one typed application basis proposition.
    #[must_use]
    pub const fn new(schema_identity: Hash, value_identity: Hash) -> Self {
        Self {
            schema_identity,
            value_identity,
        }
    }

    /// Returns its schema identity.
    #[must_use]
    pub const fn schema_identity(self) -> Hash {
        self.schema_identity
    }

    /// Returns its value identity.
    #[must_use]
    pub const fn value_identity(self) -> Hash {
        self.value_identity
    }
}

/// Exact parent basis against which Echo evaluated a prepared operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EchoOperationEvaluationBasisV1 {
    writer_head: WriterHeadKey,
    worldline_tick: WorldlineTick,
    commit_global_tick: Option<GlobalTick>,
    state_root: Hash,
    commit_id: Hash,
    application_basis: EchoOperationApplicationBasisV1,
}

impl EchoOperationEvaluationBasisV1 {
    pub(crate) fn new(
        writer_head: WriterHeadKey,
        worldline_tick: WorldlineTick,
        commit_global_tick: Option<GlobalTick>,
        state_root: Hash,
        commit_id: Hash,
        application_basis: EchoOperationApplicationBasisV1,
    ) -> Self {
        Self {
            writer_head,
            worldline_tick,
            commit_global_tick,
            state_root,
            commit_id,
            application_basis,
        }
    }

    /// Returns the writer head whose frontier is named.
    #[must_use]
    pub const fn writer_head(self) -> WriterHeadKey {
        self.writer_head
    }

    /// Returns the exact worldline tick.
    #[must_use]
    pub const fn worldline_tick(self) -> WorldlineTick {
        self.worldline_tick
    }

    /// Returns the committing global tick, or `None` for U0.
    #[must_use]
    pub const fn commit_global_tick(self) -> Option<GlobalTick> {
        self.commit_global_tick
    }

    /// Returns the exact state root.
    #[must_use]
    pub const fn state_root(self) -> Hash {
        self.state_root
    }

    /// Returns the exact commit identity (including the U0 derived identity).
    #[must_use]
    pub const fn commit_id(self) -> Hash {
        self.commit_id
    }

    /// Returns the application-owned basis proposition.
    #[must_use]
    pub const fn application_basis(self) -> EchoOperationApplicationBasisV1 {
        self.application_basis
    }

    /// Returns the stable identity of every basis field in the ADR-defined order.
    #[must_use]
    pub fn identity(self) -> EchoOperationEvaluationBasisIdV1 {
        let mut hasher = Hasher::new();
        hasher.update(BASIS_ID_DOMAIN);
        hasher.update(self.writer_head.worldline_id.as_bytes());
        hasher.update(self.writer_head.head_id.as_bytes());
        hasher.update(&self.worldline_tick.as_u64().to_le_bytes());
        match self.commit_global_tick {
            None => {
                hasher.update(&[0]);
            }
            Some(tick) => {
                hasher.update(&[1]);
                hasher.update(&tick.as_u64().to_le_bytes());
            }
        }
        hasher.update(&self.state_root);
        hasher.update(&self.commit_id);
        hasher.update(&self.application_basis.schema_identity);
        hasher.update(&self.application_basis.value_identity);
        EchoOperationEvaluationBasisIdV1(hasher.finalize().into())
    }

    fn to_value(self) -> CanonicalValueV1 {
        map_value([
            (
                "application_basis_schema_identity",
                hash_value(self.application_basis.schema_identity),
            ),
            (
                "application_basis_value_identity",
                hash_value(self.application_basis.value_identity),
            ),
            ("commit_id", hash_value(self.commit_id)),
            (
                "commit_global_tick",
                self.commit_global_tick
                    .map_or(CanonicalValueV1::Null, |tick| uint_value(tick.as_u64())),
            ),
            ("head_id", hash_value(*self.writer_head.head_id.as_bytes())),
            ("state_root", hash_value(self.state_root)),
            (
                "worldline_id",
                hash_value(*self.writer_head.worldline_id.as_bytes()),
            ),
            ("worldline_tick", uint_value(self.worldline_tick.as_u64())),
        ])
    }

    fn from_value(value: CanonicalValueV1) -> Result<Self, EchoOperationArtifactErrorV1> {
        let mut fields = exact_text_map(
            value,
            &[
                "application_basis_schema_identity",
                "application_basis_value_identity",
                "commit_global_tick",
                "commit_id",
                "head_id",
                "state_root",
                "worldline_id",
                "worldline_tick",
            ],
        )?;
        let commit_global_tick = match take_field(&mut fields, "commit_global_tick")? {
            CanonicalValueV1::Null => None,
            CanonicalValueV1::Integer(value) => Some(GlobalTick::from_raw(i128_to_u64(value)?)),
            _ => return Err(invalid_structure("commit_global_tick must be null or uint")),
        };
        Ok(Self {
            writer_head: WriterHeadKey {
                worldline_id: crate::WorldlineId::from_bytes(take_hash(
                    &mut fields,
                    "worldline_id",
                )?),
                head_id: crate::HeadId::from_bytes(take_hash(&mut fields, "head_id")?),
            },
            worldline_tick: WorldlineTick::from_raw(take_u64(&mut fields, "worldline_tick")?),
            commit_global_tick,
            state_root: take_hash(&mut fields, "state_root")?,
            commit_id: take_hash(&mut fields, "commit_id")?,
            application_basis: EchoOperationApplicationBasisV1::new(
                take_hash(&mut fields, "application_basis_schema_identity")?,
                take_hash(&mut fields, "application_basis_value_identity")?,
            ),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EchoOperationInvocationKindV1 {
    AnchoredNodeAttachmentCompareAndSet { expected_value_digest: Hash },
    AnchoredNodeAttachmentCreateIfAbsent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AnchoredNodeOperationModeV1 {
    CompareAndSet { expected_value_digest: Hash },
    CreateIfAbsent,
}

/// Canonical basis-bearing invocation emitted by a generated client/helper.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EchoOperationInvocationV1 {
    package_id: EchoOperationPackageIdV1,
    operation_coordinate: String,
    evaluation_basis: EchoOperationEvaluationBasisV1,
    authority_grant_identity: Hash,
    delegated_budget: EchoOperationBudgetV1,
    node: NodeKey,
    kind: EchoOperationInvocationKindV1,
    replacement_bytes: Vec<u8>,
}

impl EchoOperationInvocationV1 {
    /// Creates an invocation for the update-only compare-and-set program.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn anchored_node_attachment_compare_and_set(
        package_id: EchoOperationPackageIdV1,
        operation_coordinate: impl Into<String>,
        evaluation_basis: EchoOperationEvaluationBasisV1,
        authority_grant_identity: Hash,
        delegated_budget: EchoOperationBudgetV1,
        node: NodeKey,
        expected_value_digest: Hash,
        replacement_bytes: Vec<u8>,
    ) -> Self {
        Self {
            package_id,
            operation_coordinate: operation_coordinate.into(),
            evaluation_basis,
            authority_grant_identity,
            delegated_budget,
            node,
            kind: EchoOperationInvocationKindV1::AnchoredNodeAttachmentCompareAndSet {
                expected_value_digest,
            },
            replacement_bytes,
        }
    }

    /// Creates an invocation for the create-if-absent program.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn anchored_node_attachment_create_if_absent(
        package_id: EchoOperationPackageIdV1,
        operation_coordinate: impl Into<String>,
        evaluation_basis: EchoOperationEvaluationBasisV1,
        authority_grant_identity: Hash,
        delegated_budget: EchoOperationBudgetV1,
        node: NodeKey,
        replacement_bytes: Vec<u8>,
    ) -> Self {
        Self {
            package_id,
            operation_coordinate: operation_coordinate.into(),
            evaluation_basis,
            authority_grant_identity,
            delegated_budget,
            node,
            kind: EchoOperationInvocationKindV1::AnchoredNodeAttachmentCreateIfAbsent,
            replacement_bytes,
        }
    }

    /// Returns the canonical invocation identity.
    pub fn identity(&self) -> Result<EchoOperationInvocationIdV1, EchoOperationArtifactErrorV1> {
        Ok(EchoOperationInvocationIdV1(domain_hash(
            INVOCATION_ID_DOMAIN,
            &self.to_canonical_bytes()?,
        )))
    }

    /// Encodes exact invocation bytes using Edict's canonical CBOR profile.
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, EchoOperationArtifactErrorV1> {
        if self.operation_coordinate.is_empty() {
            return Err(artifact_error(
                EchoOperationArtifactErrorKindV1::EmptyOperationCoordinate,
                "invocation operation coordinate must not be empty",
            ));
        }
        if !self.delegated_budget.is_nonzero() {
            return Err(artifact_error(
                EchoOperationArtifactErrorKindV1::InvalidBudget,
                "invocation delegated step budget must be nonzero",
            ));
        }
        let common = |schema| {
            [
                (
                    "authority_grant_identity",
                    hash_value(self.authority_grant_identity),
                ),
                ("delegated_budget", self.delegated_budget.to_value()),
                ("evaluation_basis", self.evaluation_basis.to_value()),
                ("node_id", hash_value(self.node.local_id.0)),
                (
                    "operation_coordinate",
                    text_value(&self.operation_coordinate),
                ),
                ("package_id", hash_value(self.package_id.as_hash())),
                (
                    "replacement_bytes",
                    CanonicalValueV1::Bytes(self.replacement_bytes.clone()),
                ),
                ("schema", text_value(schema)),
                ("warp_id", hash_value(self.node.warp_id.0)),
            ]
        };
        let value = match self.kind {
            EchoOperationInvocationKindV1::AnchoredNodeAttachmentCompareAndSet {
                expected_value_digest,
            } => {
                let mut fields = Vec::from(common(INVOCATION_SCHEMA));
                fields.push(("expected_value_digest", hash_value(expected_value_digest)));
                CanonicalValueV1::Map(
                    fields
                        .into_iter()
                        .map(|(key, value)| (text_value(key), value))
                        .collect(),
                )
            }
            EchoOperationInvocationKindV1::AnchoredNodeAttachmentCreateIfAbsent => {
                let mut fields = Vec::from(common(CREATE_INVOCATION_SCHEMA));
                fields.push((
                    "absence_precondition",
                    text_value(CREATE_ABSENCE_PRECONDITION),
                ));
                CanonicalValueV1::Map(
                    fields
                        .into_iter()
                        .map(|(key, value)| (text_value(key), value))
                        .collect(),
                )
            }
        };
        encode_canonical_cbor_v1(&value).map_err(canonical_error)
    }

    fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, EchoOperationArtifactErrorV1> {
        let value = decode_canonical_cbor_v1(bytes).map_err(canonical_error)?;
        let schema = match &value {
            CanonicalValueV1::Map(entries) => entries
                .iter()
                .find_map(|(key, value)| {
                    (key == &CanonicalValueV1::Text("schema".to_owned())).then_some(value)
                })
                .and_then(|value| match value {
                    CanonicalValueV1::Text(value) => Some(value.as_str()),
                    _ => None,
                })
                .ok_or_else(|| invalid_structure("invocation schema must be text"))?,
            _ => return Err(invalid_structure("artifact root must be a map")),
        };
        let (expected_fields, create_if_absent) = match schema {
            INVOCATION_SCHEMA => (
                &[
                    "authority_grant_identity",
                    "delegated_budget",
                    "evaluation_basis",
                    "expected_value_digest",
                    "node_id",
                    "operation_coordinate",
                    "package_id",
                    "replacement_bytes",
                    "schema",
                    "warp_id",
                ][..],
                false,
            ),
            CREATE_INVOCATION_SCHEMA => (
                &[
                    "absence_precondition",
                    "authority_grant_identity",
                    "delegated_budget",
                    "evaluation_basis",
                    "node_id",
                    "operation_coordinate",
                    "package_id",
                    "replacement_bytes",
                    "schema",
                    "warp_id",
                ][..],
                true,
            ),
            _ => return Err(invalid_structure("unsupported invocation schema")),
        };
        let mut fields = exact_text_map(value, expected_fields)?;
        require_text(
            &mut fields,
            "schema",
            if create_if_absent {
                CREATE_INVOCATION_SCHEMA
            } else {
                INVOCATION_SCHEMA
            },
        )?;
        let kind = if create_if_absent {
            require_text(
                &mut fields,
                "absence_precondition",
                CREATE_ABSENCE_PRECONDITION,
            )?;
            EchoOperationInvocationKindV1::AnchoredNodeAttachmentCreateIfAbsent
        } else {
            EchoOperationInvocationKindV1::AnchoredNodeAttachmentCompareAndSet {
                expected_value_digest: take_hash(&mut fields, "expected_value_digest")?,
            }
        };
        let invocation = Self {
            package_id: EchoOperationPackageIdV1(take_hash(&mut fields, "package_id")?),
            operation_coordinate: take_text(&mut fields, "operation_coordinate")?,
            evaluation_basis: EchoOperationEvaluationBasisV1::from_value(take_field(
                &mut fields,
                "evaluation_basis",
            )?)?,
            authority_grant_identity: take_hash(&mut fields, "authority_grant_identity")?,
            delegated_budget: EchoOperationBudgetV1::from_value(take_field(
                &mut fields,
                "delegated_budget",
            )?)?,
            node: NodeKey {
                warp_id: crate::WarpId(take_hash(&mut fields, "warp_id")?),
                local_id: crate::NodeId(take_hash(&mut fields, "node_id")?),
            },
            kind,
            replacement_bytes: take_bytes(&mut fields, "replacement_bytes")?,
        };
        if invocation.operation_coordinate.is_empty() || !invocation.delegated_budget.is_nonzero() {
            return Err(invalid_structure(
                "invocation coordinate and delegated step budget must be nonempty",
            ));
        }
        if invocation.to_canonical_bytes()? != bytes {
            return Err(artifact_error(
                EchoOperationArtifactErrorKindV1::NonCanonical,
                "invocation did not reproduce the exact admitted bytes",
            ));
        }
        Ok(invocation)
    }
}

/// Runtime-owner invocation policy, separate from authored invocation bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EchoOperationInvocationAdmissionPolicyV1 {
    authority_profile_identity: Hash,
    authority_grant_identity: Hash,
    maximum_delegated_budget: EchoOperationBudgetV1,
}

impl EchoOperationInvocationAdmissionPolicyV1 {
    /// Creates one independently supplied invocation policy.
    #[must_use]
    pub const fn new(
        authority_profile_identity: Hash,
        authority_grant_identity: Hash,
        maximum_delegated_budget: EchoOperationBudgetV1,
    ) -> Self {
        Self {
            authority_profile_identity,
            authority_grant_identity,
            maximum_delegated_budget,
        }
    }

    fn identity(self) -> Hash {
        let mut hasher = Hasher::new();
        hasher.update(b"echo:operation-invocation-admission-policy:v1\0");
        hasher.update(&self.authority_profile_identity);
        hasher.update(&self.authority_grant_identity);
        hash_budget(&mut hasher, self.maximum_delegated_budget);
        hasher.finalize().into()
    }
}

/// Stable invocation-admission refusal categories.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EchoOperationInvocationAdmissionErrorKindV1 {
    /// Invocation bytes were malformed or noncanonical.
    MalformedInvocation,
    /// No admitted executable operation package is installed under the claimed id.
    OperationUnavailable,
    /// The invocation's public operation coordinate disagrees with the package.
    OperationCoordinateMismatch,
    /// The invocation schema does not match the installed program profile.
    OperationProfileMismatch,
    /// The runtime-owned authority profile disagrees with the package.
    AuthorityProfileMismatch,
    /// The invocation's authority grant was not admitted by runtime policy.
    AuthorityGrantMismatch,
    /// Delegated budget exceeded the installed package or runtime policy.
    BudgetExceeded,
    /// The invocation named a basis other than the current exact parent basis.
    BasisMismatch,
}

/// One structured invocation-admission refusal.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("{kind:?}: {detail}")]
pub struct EchoOperationInvocationAdmissionErrorV1 {
    kind: EchoOperationInvocationAdmissionErrorKindV1,
    detail: String,
}

impl EchoOperationInvocationAdmissionErrorV1 {
    /// Returns the stable refusal category.
    #[must_use]
    pub const fn kind(&self) -> EchoOperationInvocationAdmissionErrorKindV1 {
        self.kind
    }
}

/// Opaque evidence that Echo admitted an installed, basis-bearing invocation.
#[derive(Clone, Debug)]
pub struct AdmittedEchoOperationInvocationV1 {
    invocation: EchoOperationInvocationV1,
    invocation_id: EchoOperationInvocationIdV1,
    canonical_invocation_bytes: Vec<u8>,
    admission_policy_id: Hash,
    admission_id: EchoOperationInvocationAdmissionIdV1,
    installed_operation_id: InstalledEchoOperationIdV1,
    evaluation_authority: EchoOperationEvaluationAuthorityV1,
    admission_policy: EchoOperationInvocationAdmissionPolicyV1,
}

impl AdmittedEchoOperationInvocationV1 {
    pub(crate) const fn package_id(&self) -> EchoOperationPackageIdV1 {
        self.invocation.package_id
    }

    pub(crate) const fn evaluation_basis(&self) -> EchoOperationEvaluationBasisV1 {
        self.invocation.evaluation_basis
    }

    pub(crate) const fn scope(&self) -> NodeKey {
        self.invocation.node
    }

    pub(crate) const fn installed_operation_id(&self) -> InstalledEchoOperationIdV1 {
        self.installed_operation_id
    }

    pub(crate) const fn admission_id(&self) -> EchoOperationInvocationAdmissionIdV1 {
        self.admission_id
    }
}

pub(crate) fn admit_invocation_v1(
    installed: Option<&InstalledEchoOperationV1>,
    policy: EchoOperationInvocationAdmissionPolicyV1,
    canonical_invocation_bytes: &[u8],
    current_basis: EchoOperationEvaluationBasisV1,
    current_state: &WorldlineState,
    evaluation_authority: EchoOperationEvaluationAuthorityV1,
) -> Result<AdmittedEchoOperationInvocationV1, EchoOperationInvocationAdmissionErrorV1> {
    let (invocation, installed) =
        admit_invocation_static_v1(installed, policy, canonical_invocation_bytes)?;
    if invocation.evaluation_basis != current_basis {
        return Err(invocation_admission_error(
            EchoOperationInvocationAdmissionErrorKindV1::BasisMismatch,
            "invocation does not name the current exact parent basis",
        ));
    }
    if invocation
        .evaluation_basis
        .application_basis
        .schema_identity
        != installed.application_basis_schema_identity
    {
        return Err(invocation_admission_error(
            EchoOperationInvocationAdmissionErrorKindV1::BasisMismatch,
            "invocation application-basis schema differs from installed package",
        ));
    }
    let current_application_basis =
        current_application_basis(installed, &invocation, current_state)?;
    if invocation.evaluation_basis.application_basis != current_application_basis {
        return Err(invocation_admission_error(
            EchoOperationInvocationAdmissionErrorKindV1::BasisMismatch,
            "invocation application basis differs from Echo's current graph proposition",
        ));
    }
    Ok(build_admitted_invocation_v1(
        invocation,
        installed,
        policy,
        canonical_invocation_bytes,
        evaluation_authority,
    ))
}

/// Admits the static installed meaning and runtime-owned authority for one
/// scheduler-bound executable-operation Action.
///
/// Exact basis freshness is deliberately deferred to private evaluation during
/// Tick construction. That lets an Action which was durably accepted at an
/// earlier basis reach the evaluator and receive a typed `BasisChanged`
/// obstruction instead of failing the host admission loop.
pub(crate) fn admit_action_invocation_v1(
    installed: Option<&InstalledEchoOperationV1>,
    policy: EchoOperationInvocationAdmissionPolicyV1,
    canonical_invocation_bytes: &[u8],
    evaluation_authority: EchoOperationEvaluationAuthorityV1,
) -> Result<AdmittedEchoOperationInvocationV1, EchoOperationInvocationAdmissionErrorV1> {
    let (invocation, installed) =
        admit_invocation_static_v1(installed, policy, canonical_invocation_bytes)?;
    if invocation
        .evaluation_basis
        .application_basis
        .schema_identity
        != installed.application_basis_schema_identity
    {
        return Err(invocation_admission_error(
            EchoOperationInvocationAdmissionErrorKindV1::BasisMismatch,
            "invocation application-basis schema differs from installed package",
        ));
    }
    Ok(build_admitted_invocation_v1(
        invocation,
        installed,
        policy,
        canonical_invocation_bytes,
        evaluation_authority,
    ))
}

fn admit_invocation_static_v1<'a>(
    installed: Option<&'a InstalledEchoOperationV1>,
    policy: EchoOperationInvocationAdmissionPolicyV1,
    canonical_invocation_bytes: &[u8],
) -> Result<
    (EchoOperationInvocationV1, &'a InstalledEchoOperationV1),
    EchoOperationInvocationAdmissionErrorV1,
> {
    let invocation = EchoOperationInvocationV1::from_canonical_bytes(canonical_invocation_bytes)
        .map_err(|error| {
            invocation_admission_error(
                EchoOperationInvocationAdmissionErrorKindV1::MalformedInvocation,
                error.to_string(),
            )
        })?;
    let installed = installed.ok_or_else(|| {
        invocation_admission_error(
            EchoOperationInvocationAdmissionErrorKindV1::OperationUnavailable,
            "claimed executable operation package is not installed",
        )
    })?;
    if invocation.operation_coordinate != installed.operation_coordinate {
        return Err(invocation_admission_error(
            EchoOperationInvocationAdmissionErrorKindV1::OperationCoordinateMismatch,
            "invocation operation coordinate differs from installed package",
        ));
    }
    if !installed.program.accepts_invocation_kind(invocation.kind) {
        return Err(invocation_admission_error(
            EchoOperationInvocationAdmissionErrorKindV1::OperationProfileMismatch,
            "invocation schema differs from the installed program profile",
        ));
    }
    if policy.authority_profile_identity != installed.authority_profile_identity {
        return Err(invocation_admission_error(
            EchoOperationInvocationAdmissionErrorKindV1::AuthorityProfileMismatch,
            "runtime authority profile differs from installed package",
        ));
    }
    if invocation.authority_grant_identity != policy.authority_grant_identity {
        return Err(invocation_admission_error(
            EchoOperationInvocationAdmissionErrorKindV1::AuthorityGrantMismatch,
            "invocation authority grant was not admitted by runtime policy",
        ));
    }
    if !invocation
        .delegated_budget
        .fits_within(installed.budget_ceiling)
        || !invocation
            .delegated_budget
            .fits_within(policy.maximum_delegated_budget)
        || !installed
            .program
            .minimum_budget()
            .fits_within(invocation.delegated_budget)
    {
        return Err(invocation_admission_error(
            EchoOperationInvocationAdmissionErrorKindV1::BudgetExceeded,
            "delegated budget is below the program minimum or exceeds an admitted ceiling",
        ));
    }
    Ok((invocation, installed))
}

fn build_admitted_invocation_v1(
    invocation: EchoOperationInvocationV1,
    installed: &InstalledEchoOperationV1,
    policy: EchoOperationInvocationAdmissionPolicyV1,
    canonical_invocation_bytes: &[u8],
    evaluation_authority: EchoOperationEvaluationAuthorityV1,
) -> AdmittedEchoOperationInvocationV1 {
    let invocation_id = EchoOperationInvocationIdV1(domain_hash(
        INVOCATION_ID_DOMAIN,
        canonical_invocation_bytes,
    ));
    let admission_policy_id = policy.identity();
    let installed_operation_id = installed.installed_operation_id;
    let evaluation_basis_id = invocation.evaluation_basis.identity();
    AdmittedEchoOperationInvocationV1 {
        invocation,
        invocation_id,
        canonical_invocation_bytes: canonical_invocation_bytes.to_vec(),
        admission_policy_id,
        admission_id: invocation_admission_id(
            installed_operation_id,
            invocation_id,
            admission_policy_id,
            evaluation_basis_id,
        ),
        installed_operation_id,
        evaluation_authority,
        admission_policy: policy,
    }
}

fn current_application_basis(
    installed: &InstalledEchoOperationV1,
    invocation: &EchoOperationInvocationV1,
    state: &WorldlineState,
) -> Result<EchoOperationApplicationBasisV1, EchoOperationInvocationAdmissionErrorV1> {
    let basis_mismatch = |detail| {
        invocation_admission_error(
            EchoOperationInvocationAdmissionErrorKindV1::BasisMismatch,
            detail,
        )
    };
    match installed.program {
        EchoOperationProgramV1::AnchoredNodeAttachmentCompareAndSet { .. } => {
            let store = state
                .store(&invocation.node.warp_id)
                .ok_or_else(|| basis_mismatch("application-basis warp is unavailable"))?;
            store
                .node(&invocation.node.local_id)
                .ok_or_else(|| basis_mismatch("application-basis node is unavailable"))?;
            let attachment = store
                .node_attachment(&invocation.node.local_id)
                .ok_or_else(|| basis_mismatch("application-basis attachment is unavailable"))?;
            let AttachmentValue::Atom(atom) = attachment else {
                return Err(basis_mismatch(
                    "application-basis attachment is not a canonical atom",
                ));
            };
            let atom_len = u64::try_from(atom.bytes.len()).map_err(|_| {
                invocation_admission_error(
                    EchoOperationInvocationAdmissionErrorKindV1::BudgetExceeded,
                    "application-basis attachment length exceeds the v1 budget domain",
                )
            })?;
            let required_read_bytes = 64_u64.checked_add(atom_len).ok_or_else(|| {
                invocation_admission_error(
                    EchoOperationInvocationAdmissionErrorKindV1::BudgetExceeded,
                    "application-basis read requirement overflowed",
                )
            })?;
            if required_read_bytes > invocation.delegated_budget.read_bytes {
                return Err(invocation_admission_error(
                    EchoOperationInvocationAdmissionErrorKindV1::BudgetExceeded,
                    "application-basis corroboration exceeds the delegated read budget",
                ));
            }
            Ok(echo_operation_anchored_node_application_basis_v1(
                invocation.node,
                atom.type_id,
                &atom.bytes,
            ))
        }
        EchoOperationProgramV1::AnchoredNodeAttachmentCreateIfAbsent { .. } => {
            let store = state
                .store(&invocation.node.warp_id)
                .ok_or_else(|| basis_mismatch("application-basis warp is unavailable"))?;
            let occupancy = EchoOperationAnchoredNodeOccupancyV1::from_presence(
                store.node(&invocation.node.local_id).is_some(),
                store.node_attachment(&invocation.node.local_id).is_some(),
            );
            Ok(echo_operation_anchored_node_creation_application_basis_v1(
                invocation.node,
                occupancy,
            ))
        }
    }
}

pub(crate) fn decode_invocation_route_v1(
    canonical_invocation_bytes: &[u8],
) -> Result<
    (EchoOperationPackageIdV1, EchoOperationEvaluationBasisV1),
    EchoOperationInvocationAdmissionErrorV1,
> {
    let invocation = EchoOperationInvocationV1::from_canonical_bytes(canonical_invocation_bytes)
        .map_err(|error| {
            invocation_admission_error(
                EchoOperationInvocationAdmissionErrorKindV1::MalformedInvocation,
                error.to_string(),
            )
        })?;
    Ok((invocation.package_id, invocation.evaluation_basis))
}

pub(crate) struct EchoOperationActionInvocationEvidenceV1 {
    pub package_id: EchoOperationPackageIdV1,
    pub scope: NodeKey,
    pub evaluation_basis: EchoOperationEvaluationBasisV1,
    pub invocation_id: EchoOperationInvocationIdV1,
    pub invocation_bytes_digest: Hash,
}

pub(crate) fn inspect_action_invocation_v1(
    canonical_invocation_bytes: &[u8],
) -> Result<EchoOperationActionInvocationEvidenceV1, EchoOperationInvocationAdmissionErrorV1> {
    let invocation = EchoOperationInvocationV1::from_canonical_bytes(canonical_invocation_bytes)
        .map_err(|error| {
            invocation_admission_error(
                EchoOperationInvocationAdmissionErrorKindV1::MalformedInvocation,
                error.to_string(),
            )
        })?;
    Ok(EchoOperationActionInvocationEvidenceV1 {
        package_id: invocation.package_id,
        scope: invocation.node,
        evaluation_basis: invocation.evaluation_basis,
        invocation_id: EchoOperationInvocationIdV1(domain_hash(
            INVOCATION_ID_DOMAIN,
            canonical_invocation_bytes,
        )),
        invocation_bytes_digest: domain_hash(
            INVOCATION_BYTES_DIGEST_DOMAIN,
            canonical_invocation_bytes,
        ),
    })
}

pub(crate) fn invocation_runtime_error(
    detail: impl Into<String>,
) -> EchoOperationInvocationAdmissionErrorV1 {
    invocation_admission_error(
        EchoOperationInvocationAdmissionErrorKindV1::BasisMismatch,
        detail,
    )
}

pub(crate) fn runtime_basis_obstruction(
    admitted: AdmittedEchoOperationInvocationV1,
) -> EchoOperationPreparationV1 {
    EchoOperationPreparationV1::Obstructed(EchoOperationObstructionV1 {
        kind: EchoOperationObstructionKindV1::BasisChanged,
        package_id: admitted.invocation.package_id,
        installed_operation_id: admitted.installed_operation_id,
        invocation_admission_id: admitted.admission_id,
        invocation_id: admitted.invocation_id,
        evaluation_basis_id: admitted.invocation.evaluation_basis.identity(),
    })
}

/// Stable private-evaluation obstruction categories.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EchoOperationObstructionKindV1 {
    /// The installed operation changed or disappeared after admission.
    OperationUnavailable,
    /// Admission belongs to another Echo runtime owner.
    EvaluationAuthorityMismatch,
    /// The parent basis changed before or during preparation.
    BasisChanged,
    /// The delegated budget could not cover deterministic evaluation.
    BudgetExceeded,
    /// The anchored node is absent.
    NodeMissing,
    /// The anchored node has a different skeleton type.
    NodeTypeMismatch,
    /// The anchored node has no alpha attachment.
    AttachmentMissing,
    /// The alpha attachment is descended rather than an atom.
    AttachmentNotAtom,
    /// The alpha atom has a different declared type.
    AttachmentTypeMismatch,
    /// The current atom digest differs from the invocation precondition, or
    /// the invocation expected the node and attachment to be entirely
    /// absent (create-from-absence) but one or both already exist.
    PreconditionMismatch,
    /// Actual resource access exceeded the declared footprint contract.
    FootprintViolation,
    /// The program rejected a replacement outside its bound.
    ReplacementTooLarge,
}

/// One typed obstruction. Obstruction never carries a parent-visible patch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EchoOperationObstructionV1 {
    kind: EchoOperationObstructionKindV1,
    package_id: EchoOperationPackageIdV1,
    installed_operation_id: InstalledEchoOperationIdV1,
    invocation_admission_id: EchoOperationInvocationAdmissionIdV1,
    invocation_id: EchoOperationInvocationIdV1,
    evaluation_basis_id: EchoOperationEvaluationBasisIdV1,
}

impl EchoOperationObstructionV1 {
    /// Returns the stable obstruction category.
    #[must_use]
    pub const fn kind(&self) -> EchoOperationObstructionKindV1 {
        self.kind
    }

    /// Returns the installed package whose evaluation was obstructed.
    #[must_use]
    pub const fn package_id(&self) -> EchoOperationPackageIdV1 {
        self.package_id
    }

    /// Returns the exact installed operation against which evaluation was attempted.
    #[must_use]
    pub const fn installed_operation_id(&self) -> InstalledEchoOperationIdV1 {
        self.installed_operation_id
    }

    /// Returns the exact canonical invocation refused by evaluation.
    #[must_use]
    pub const fn invocation_id(&self) -> EchoOperationInvocationIdV1 {
        self.invocation_id
    }

    /// Returns the exact private-evaluation basis refused by evaluation.
    #[must_use]
    pub const fn evaluation_basis_id(&self) -> EchoOperationEvaluationBasisIdV1 {
        self.evaluation_basis_id
    }

    /// Returns the exact runtime admission that authorized this evaluation attempt.
    #[must_use]
    pub const fn invocation_admission_id(&self) -> EchoOperationInvocationAdmissionIdV1 {
        self.invocation_admission_id
    }

    /// Returns the identity of this typed no-parent-patch obstruction.
    #[must_use]
    pub fn identity(&self) -> EchoOperationObstructionIdV1 {
        let mut hasher = Hasher::new();
        hasher.update(OBSTRUCTION_ID_DOMAIN);
        hasher.update(&self.package_id.as_hash());
        hasher.update(&self.installed_operation_id.as_hash());
        hasher.update(&self.invocation_admission_id.as_hash());
        hasher.update(&self.invocation_id.as_hash());
        hasher.update(&self.evaluation_basis_id.as_hash());
        hasher.update(&[obstruction_kind_code(self.kind)]);
        EchoOperationObstructionIdV1(hasher.finalize().into())
    }
}

/// Typed terminal outcome for one executable-operation Action selected by a
/// scheduler-owned Tick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EchoOperationActionOutcomeV1 {
    /// The Action's prepared member consequence entered the composite Tick.
    Committed(Box<EchoOperationReceiptV1>),
    /// Private bounded evaluation refused the Action and emitted no mutation.
    Obstructed(EchoOperationObstructionV1),
    /// An earlier applied Action in the same Tick reserved an overlapping
    /// footprint.
    RejectedFootprintConflict {
        /// Exact installed operation whose prepared consequence conflicted.
        installed_operation_id: InstalledEchoOperationIdV1,
        /// Exact canonical invocation whose prepared consequence conflicted.
        invocation_id: EchoOperationInvocationIdV1,
        /// Identity of the privately prepared candidate that was not applied.
        preparation_id: PreparedEchoOperationIdV1,
        /// Canonical indices of earlier applied Tick members that blocked it.
        blocked_by: Vec<u32>,
    },
}

pub(crate) fn retain_action_outcome_v1(
    submission_id: Hash,
    ingress_id: Hash,
    outcome: &EchoOperationActionOutcomeV1,
) -> Result<Vec<u8>, EchoOperationArtifactErrorV1> {
    let mut out = Vec::new();
    out.extend_from_slice(ACTION_OUTCOME_RECORD_MAGIC);
    out.extend_from_slice(&submission_id);
    out.extend_from_slice(&ingress_id);
    match outcome {
        EchoOperationActionOutcomeV1::Committed(receipt) => {
            out.push(1);
            let receipt_bytes = receipt.to_canonical_bytes()?;
            let receipt_len = u64::try_from(receipt_bytes.len())
                .map_err(|_| invalid_structure("Action receipt length is not representable"))?;
            out.extend_from_slice(&receipt_len.to_le_bytes());
            out.extend_from_slice(&receipt_bytes);
        }
        EchoOperationActionOutcomeV1::Obstructed(obstruction) => {
            out.push(2);
            out.push(obstruction_kind_code(obstruction.kind));
            out.extend_from_slice(&obstruction.package_id.as_hash());
            out.extend_from_slice(&obstruction.installed_operation_id.as_hash());
            out.extend_from_slice(&obstruction.invocation_admission_id.as_hash());
            out.extend_from_slice(&obstruction.invocation_id.as_hash());
            out.extend_from_slice(&obstruction.evaluation_basis_id.as_hash());
        }
        EchoOperationActionOutcomeV1::RejectedFootprintConflict {
            installed_operation_id,
            invocation_id,
            preparation_id,
            blocked_by,
        } => {
            out.push(3);
            out.extend_from_slice(&installed_operation_id.as_hash());
            out.extend_from_slice(&invocation_id.as_hash());
            out.extend_from_slice(&preparation_id.as_hash());
            let blocker_count = u64::try_from(blocked_by.len())
                .map_err(|_| invalid_structure("Action blocker count is not representable"))?;
            out.extend_from_slice(&blocker_count.to_le_bytes());
            for blocker in blocked_by {
                out.extend_from_slice(&blocker.to_le_bytes());
            }
        }
    }
    Ok(out)
}

pub(crate) fn recover_action_outcome_v1(
    bytes: &[u8],
) -> Result<(Hash, Hash, EchoOperationActionOutcomeV1), EchoOperationArtifactErrorV1> {
    let minimum = ACTION_OUTCOME_RECORD_MAGIC.len() + 32 + 32 + 1;
    if bytes.len() < minimum
        || &bytes[..ACTION_OUTCOME_RECORD_MAGIC.len()] != ACTION_OUTCOME_RECORD_MAGIC
    {
        return Err(invalid_structure(
            "executable-operation Action outcome record has invalid framing",
        ));
    }
    let mut offset = ACTION_OUTCOME_RECORD_MAGIC.len();
    let submission_id = read_action_outcome_hash(bytes, &mut offset)?;
    let ingress_id = read_action_outcome_hash(bytes, &mut offset)?;
    let tag = *bytes
        .get(offset)
        .ok_or_else(|| invalid_structure("Action outcome tag is missing"))?;
    offset += 1;
    let outcome = match tag {
        1 => {
            let len = read_action_outcome_u64(bytes, &mut offset)?;
            let len = usize::try_from(len)
                .map_err(|_| invalid_structure("Action receipt length is not representable"))?;
            let end = offset
                .checked_add(len)
                .ok_or_else(|| invalid_structure("Action receipt length overflows"))?;
            let receipt_bytes = bytes
                .get(offset..end)
                .ok_or_else(|| invalid_structure("Action receipt bytes are truncated"))?;
            offset = end;
            EchoOperationActionOutcomeV1::Committed(Box::new(
                EchoOperationReceiptV1::from_canonical_bytes(receipt_bytes)?,
            ))
        }
        2 => {
            let kind_code = *bytes
                .get(offset)
                .ok_or_else(|| invalid_structure("Action obstruction kind is missing"))?;
            offset += 1;
            EchoOperationActionOutcomeV1::Obstructed(EchoOperationObstructionV1 {
                kind: obstruction_kind_from_code(kind_code)?,
                package_id: EchoOperationPackageIdV1(read_action_outcome_hash(bytes, &mut offset)?),
                installed_operation_id: InstalledEchoOperationIdV1(read_action_outcome_hash(
                    bytes,
                    &mut offset,
                )?),
                invocation_admission_id: EchoOperationInvocationAdmissionIdV1(
                    read_action_outcome_hash(bytes, &mut offset)?,
                ),
                invocation_id: EchoOperationInvocationIdV1(read_action_outcome_hash(
                    bytes,
                    &mut offset,
                )?),
                evaluation_basis_id: EchoOperationEvaluationBasisIdV1(read_action_outcome_hash(
                    bytes,
                    &mut offset,
                )?),
            })
        }
        3 => {
            let installed_operation_id =
                InstalledEchoOperationIdV1(read_action_outcome_hash(bytes, &mut offset)?);
            let invocation_id =
                EchoOperationInvocationIdV1(read_action_outcome_hash(bytes, &mut offset)?);
            let preparation_id =
                PreparedEchoOperationIdV1(read_action_outcome_hash(bytes, &mut offset)?);
            let blocker_count = read_action_outcome_u64(bytes, &mut offset)?;
            let maximum_encoded_blockers =
                u64::try_from(bytes.len().saturating_sub(offset) / core::mem::size_of::<u32>())
                    .unwrap_or(u64::MAX);
            if blocker_count > maximum_encoded_blockers {
                return Err(invalid_structure("Action blocker bytes are truncated"));
            }
            let blocker_count = usize::try_from(blocker_count)
                .map_err(|_| invalid_structure("Action blocker count is not representable"))?;
            let mut blocked_by = Vec::with_capacity(blocker_count);
            for _ in 0..blocker_count {
                let end = offset
                    .checked_add(4)
                    .ok_or_else(|| invalid_structure("Action blocker offset overflows"))?;
                let raw: [u8; 4] = bytes
                    .get(offset..end)
                    .ok_or_else(|| invalid_structure("Action blocker bytes are truncated"))?
                    .try_into()
                    .map_err(|_| invalid_structure("Action blocker is not four bytes"))?;
                offset = end;
                blocked_by.push(u32::from_le_bytes(raw));
            }
            EchoOperationActionOutcomeV1::RejectedFootprintConflict {
                installed_operation_id,
                invocation_id,
                preparation_id,
                blocked_by,
            }
        }
        _ => {
            return Err(invalid_structure(
                "unknown executable-operation Action outcome tag",
            ))
        }
    };
    if offset != bytes.len() {
        return Err(invalid_structure(
            "executable-operation Action outcome record has trailing bytes",
        ));
    }
    Ok((submission_id, ingress_id, outcome))
}

fn read_action_outcome_hash(
    bytes: &[u8],
    offset: &mut usize,
) -> Result<Hash, EchoOperationArtifactErrorV1> {
    let end = offset
        .checked_add(32)
        .ok_or_else(|| invalid_structure("Action outcome hash offset overflows"))?;
    let value = bytes
        .get(*offset..end)
        .ok_or_else(|| invalid_structure("Action outcome hash is truncated"))?
        .try_into()
        .map_err(|_| invalid_structure("Action outcome hash is not 32 bytes"))?;
    *offset = end;
    Ok(value)
}

fn read_action_outcome_u64(
    bytes: &[u8],
    offset: &mut usize,
) -> Result<u64, EchoOperationArtifactErrorV1> {
    let end = offset
        .checked_add(8)
        .ok_or_else(|| invalid_structure("Action outcome integer offset overflows"))?;
    let raw: [u8; 8] = bytes
        .get(*offset..end)
        .ok_or_else(|| invalid_structure("Action outcome integer is truncated"))?
        .try_into()
        .map_err(|_| invalid_structure("Action outcome integer is not eight bytes"))?;
    *offset = end;
    Ok(u64::from_le_bytes(raw))
}

fn obstruction_kind_from_code(
    code: u8,
) -> Result<EchoOperationObstructionKindV1, EchoOperationArtifactErrorV1> {
    match code {
        1 => Ok(EchoOperationObstructionKindV1::OperationUnavailable),
        2 => Ok(EchoOperationObstructionKindV1::BasisChanged),
        3 => Ok(EchoOperationObstructionKindV1::BudgetExceeded),
        4 => Ok(EchoOperationObstructionKindV1::NodeMissing),
        5 => Ok(EchoOperationObstructionKindV1::NodeTypeMismatch),
        6 => Ok(EchoOperationObstructionKindV1::AttachmentMissing),
        7 => Ok(EchoOperationObstructionKindV1::AttachmentNotAtom),
        8 => Ok(EchoOperationObstructionKindV1::AttachmentTypeMismatch),
        9 => Ok(EchoOperationObstructionKindV1::PreconditionMismatch),
        10 => Ok(EchoOperationObstructionKindV1::FootprintViolation),
        11 => Ok(EchoOperationObstructionKindV1::ReplacementTooLarge),
        12 => Ok(EchoOperationObstructionKindV1::EvaluationAuthorityMismatch),
        _ => Err(invalid_structure(
            "unknown executable-operation obstruction kind",
        )),
    }
}

/// Private evaluation result: exactly one complete preparation or one obstruction.
#[derive(Clone, Debug)]
pub enum EchoOperationPreparationV1 {
    /// A complete, privately evaluated patch that may be offered for exact-basis commit.
    Prepared(Box<PreparedEchoOperationV1>),
    /// A typed obstruction with no parent-visible patch.
    Obstructed(EchoOperationObstructionV1),
}

/// Complete private evaluation, bound to all substitution and resource evidence.
#[derive(Clone, Debug)]
pub struct PreparedEchoOperationV1 {
    installed: InstalledEchoOperationV1,
    invocation: EchoOperationInvocationV1,
    invocation_id: EchoOperationInvocationIdV1,
    canonical_invocation_bytes: Vec<u8>,
    invocation_admission_policy_id: Hash,
    invocation_admission_maximum_budget: EchoOperationBudgetV1,
    invocation_admission_id: EchoOperationInvocationAdmissionIdV1,
    evaluation_basis: EchoOperationEvaluationBasisV1,
    declared_footprint: Footprint,
    actual_footprint: Footprint,
    declared_footprint_digest: Hash,
    actual_footprint_digest: Hash,
    consumed_budget: EchoOperationBudgetV1,
    patch: WarpTickPatchV1,
    result_id: EchoOperationResultIdV1,
    private_evaluation_id: EchoOperationPrivateEvaluationIdV1,
    preparation_id: PreparedEchoOperationIdV1,
    evaluation_authority: EchoOperationEvaluationAuthorityV1,
}

impl PreparedEchoOperationV1 {
    /// Returns the exact basis on which private evaluation occurred.
    #[must_use]
    pub const fn evaluation_basis(&self) -> &EchoOperationEvaluationBasisV1 {
        &self.evaluation_basis
    }

    /// Returns the invocation-derived declared footprint.
    #[must_use]
    pub const fn declared_footprint(&self) -> &Footprint {
        &self.declared_footprint
    }

    /// Returns the evaluator-recorded actual footprint.
    #[must_use]
    pub const fn actual_footprint(&self) -> &Footprint {
        &self.actual_footprint
    }

    /// Returns the complete parent-visible patch produced in private evaluation.
    #[must_use]
    pub const fn patch(&self) -> &WarpTickPatchV1 {
        &self.patch
    }

    /// Returns the exact invocation-admission evidence consumed by evaluation.
    #[must_use]
    pub const fn invocation_admission_id(&self) -> EchoOperationInvocationAdmissionIdV1 {
        self.invocation_admission_id
    }

    /// Returns the declared-footprint identity bound by evaluation.
    #[must_use]
    pub const fn declared_footprint_digest(&self) -> Hash {
        self.declared_footprint_digest
    }

    /// Returns the evaluator-recorded actual-footprint identity.
    #[must_use]
    pub const fn actual_footprint_digest(&self) -> Hash {
        self.actual_footprint_digest
    }

    /// Returns the invocation's admitted delegated budget.
    #[must_use]
    pub const fn delegated_budget(&self) -> EchoOperationBudgetV1 {
        self.invocation.delegated_budget
    }

    /// Returns the resource budget consumed during private evaluation.
    #[must_use]
    pub const fn consumed_budget(&self) -> EchoOperationBudgetV1 {
        self.consumed_budget
    }

    /// Returns the bounded private-evaluation identity.
    #[must_use]
    pub const fn private_evaluation_id(&self) -> EchoOperationPrivateEvaluationIdV1 {
        self.private_evaluation_id
    }

    /// Returns this complete committable preparation's identity.
    #[must_use]
    pub const fn preparation_id(&self) -> PreparedEchoOperationIdV1 {
        self.preparation_id
    }

    /// Returns the typed result identity produced by evaluation.
    #[must_use]
    pub const fn result_id(&self) -> EchoOperationResultIdV1 {
        self.result_id
    }

    pub(crate) const fn package_id(&self) -> EchoOperationPackageIdV1 {
        self.installed.package_id
    }

    pub(crate) const fn installed_operation_id(&self) -> InstalledEchoOperationIdV1 {
        self.installed.installed_operation_id
    }

    pub(crate) const fn invocation_id(&self) -> EchoOperationInvocationIdV1 {
        self.invocation_id
    }

    pub(crate) fn is_owned_by(&self, authority: &EchoOperationEvaluationAuthorityV1) -> bool {
        self.evaluation_authority.same_owner(authority)
    }
}

pub(crate) fn prepare_operation_v1(
    installed: Option<&InstalledEchoOperationV1>,
    admitted: AdmittedEchoOperationInvocationV1,
    current_basis: EchoOperationEvaluationBasisV1,
    state: &WorldlineState,
    policy_id: u32,
    evaluation_authority: &EchoOperationEvaluationAuthorityV1,
) -> EchoOperationPreparationV1 {
    let package_id = admitted.invocation.package_id;
    let invocation_id = admitted.invocation_id;
    let obstruction = |kind| {
        EchoOperationPreparationV1::Obstructed(EchoOperationObstructionV1 {
            kind,
            package_id,
            installed_operation_id: admitted.installed_operation_id,
            invocation_admission_id: admitted.admission_id,
            invocation_id,
            evaluation_basis_id: admitted.invocation.evaluation_basis.identity(),
        })
    };
    let Some(installed) = installed else {
        return obstruction(EchoOperationObstructionKindV1::OperationUnavailable);
    };
    if installed.package_id != package_id
        || installed.operation_coordinate != admitted.invocation.operation_coordinate
        || installed.installed_operation_id != admitted.installed_operation_id
    {
        return obstruction(EchoOperationObstructionKindV1::OperationUnavailable);
    }
    if !admitted
        .evaluation_authority
        .same_owner(evaluation_authority)
    {
        return obstruction(EchoOperationObstructionKindV1::EvaluationAuthorityMismatch);
    }
    if admitted.invocation.evaluation_basis != current_basis {
        return obstruction(EchoOperationObstructionKindV1::BasisChanged);
    }

    let (required_node_type, required_attachment_type, max_replacement_bytes, mode) =
        match (&installed.program, admitted.invocation.kind) {
            (
                EchoOperationProgramV1::AnchoredNodeAttachmentCompareAndSet {
                    required_node_type,
                    required_attachment_type,
                    max_replacement_bytes,
                },
                EchoOperationInvocationKindV1::AnchoredNodeAttachmentCompareAndSet {
                    expected_value_digest,
                },
            ) => (
                *required_node_type,
                *required_attachment_type,
                *max_replacement_bytes,
                AnchoredNodeOperationModeV1::CompareAndSet {
                    expected_value_digest,
                },
            ),
            (
                EchoOperationProgramV1::AnchoredNodeAttachmentCreateIfAbsent {
                    required_node_type,
                    required_attachment_type,
                    max_replacement_bytes,
                },
                EchoOperationInvocationKindV1::AnchoredNodeAttachmentCreateIfAbsent,
            ) => (
                *required_node_type,
                *required_attachment_type,
                *max_replacement_bytes,
                AnchoredNodeOperationModeV1::CreateIfAbsent,
            ),
            _ => return obstruction(EchoOperationObstructionKindV1::OperationUnavailable),
        };
    let Ok(replacement_len) = u64::try_from(admitted.invocation.replacement_bytes.len()) else {
        return obstruction(EchoOperationObstructionKindV1::ReplacementTooLarge);
    };
    if replacement_len > max_replacement_bytes {
        return obstruction(EchoOperationObstructionKindV1::ReplacementTooLarge);
    }
    let node = admitted.invocation.node;
    let mut actual_footprint = Footprint::default();
    let mut budget_meter = EchoOperationBudgetMeterV1::new(admitted.invocation.delegated_budget);
    let descent_stack =
        match operation_descent_stack_with_portal_reads(state, node.warp_id, |portal| {
            if !budget_meter.charge(1, 32, 0) {
                return false;
            }
            actual_footprint.a_read.insert(portal);
            true
        }) {
            Ok(descent_stack) => descent_stack,
            Err(kind) => return obstruction(kind),
        };
    let Some(store) = state.store(&node.warp_id) else {
        return obstruction(EchoOperationObstructionKindV1::NodeMissing);
    };
    let declared_footprint = match mode {
        AnchoredNodeOperationModeV1::CompareAndSet { .. } => {
            anchored_node_compare_and_set_footprint(node, &descent_stack)
        }
        AnchoredNodeOperationModeV1::CreateIfAbsent => {
            anchored_node_create_if_absent_footprint(node, &descent_stack)
        }
    };
    if !budget_meter.charge(1, 32, 0) {
        return obstruction(EchoOperationObstructionKindV1::BudgetExceeded);
    }
    record_node_read(&mut actual_footprint, node);
    let slot = AttachmentKey::node_alpha(node);
    let (out_slots, write_ops) = match mode {
        AnchoredNodeOperationModeV1::CreateIfAbsent => {
            if store.node(&node.local_id).is_some() {
                return obstruction(EchoOperationObstructionKindV1::PreconditionMismatch);
            }
            if !budget_meter.charge(1, 32, 0) {
                return obstruction(EchoOperationObstructionKindV1::BudgetExceeded);
            }
            actual_footprint.a_read.insert(slot);
            if store.node_attachment(&node.local_id).is_some() {
                return obstruction(EchoOperationObstructionKindV1::PreconditionMismatch);
            }
            let Some(write_bytes) = 64_u64.checked_add(replacement_len) else {
                return obstruction(EchoOperationObstructionKindV1::BudgetExceeded);
            };
            if !budget_meter.charge(1, 0, write_bytes) {
                return obstruction(EchoOperationObstructionKindV1::BudgetExceeded);
            }
            actual_footprint.n_write.insert(node);
            actual_footprint.a_write.insert(slot);
            (
                vec![SlotId::Node(node), SlotId::Attachment(slot)],
                vec![
                    WarpOp::UpsertNode {
                        node,
                        record: NodeRecord {
                            ty: required_node_type,
                        },
                    },
                    WarpOp::SetAttachment {
                        key: slot,
                        value: Some(AttachmentValue::Atom(AtomPayload::new(
                            required_attachment_type,
                            Bytes::from(admitted.invocation.replacement_bytes.clone()),
                        ))),
                    },
                ],
            )
        }
        AnchoredNodeOperationModeV1::CompareAndSet {
            expected_value_digest,
        } => {
            let Some(record) = store.node(&node.local_id) else {
                return obstruction(EchoOperationObstructionKindV1::NodeMissing);
            };
            if record.ty != required_node_type {
                return obstruction(EchoOperationObstructionKindV1::NodeTypeMismatch);
            }
            if !budget_meter.charge(1, 32, 0) {
                return obstruction(EchoOperationObstructionKindV1::BudgetExceeded);
            }
            actual_footprint.a_read.insert(slot);
            let Some(attachment) = store.node_attachment(&node.local_id) else {
                return obstruction(EchoOperationObstructionKindV1::AttachmentMissing);
            };
            let AttachmentValue::Atom(atom) = attachment else {
                return obstruction(EchoOperationObstructionKindV1::AttachmentNotAtom);
            };
            if atom.type_id != required_attachment_type {
                return obstruction(EchoOperationObstructionKindV1::AttachmentTypeMismatch);
            }
            let Ok(current_value_len) = u64::try_from(atom.bytes.len()) else {
                return obstruction(EchoOperationObstructionKindV1::BudgetExceeded);
            };
            if !budget_meter.charge(1, current_value_len, 0) {
                return obstruction(EchoOperationObstructionKindV1::BudgetExceeded);
            }
            if echo_operation_atom_value_digest_v1(atom.type_id, &atom.bytes)
                != expected_value_digest
            {
                return obstruction(EchoOperationObstructionKindV1::PreconditionMismatch);
            }
            let Some(write_bytes) = 32_u64.checked_add(replacement_len) else {
                return obstruction(EchoOperationObstructionKindV1::BudgetExceeded);
            };
            if !budget_meter.charge(1, 0, write_bytes) {
                return obstruction(EchoOperationObstructionKindV1::BudgetExceeded);
            }
            actual_footprint.a_write.insert(slot);
            (
                vec![SlotId::Attachment(slot)],
                vec![WarpOp::SetAttachment {
                    key: slot,
                    value: Some(AttachmentValue::Atom(AtomPayload::new(
                        required_attachment_type,
                        Bytes::from(admitted.invocation.replacement_bytes.clone()),
                    ))),
                }],
            )
        }
    };
    let consumed_budget = budget_meter.consumed();
    if actual_footprint != declared_footprint {
        return obstruction(EchoOperationObstructionKindV1::FootprintViolation);
    }
    let mut in_slots = vec![SlotId::Node(node), SlotId::Attachment(slot)];
    in_slots.extend(descent_stack.iter().copied().map(SlotId::Attachment));
    let patch = WarpTickPatchV1::new(
        policy_id,
        installed.installed_operation_id.as_hash(),
        TickCommitStatus::Committed,
        in_slots,
        out_slots,
        write_ops,
    );
    let declared_footprint_digest = footprint_digest(&declared_footprint);
    let actual_footprint_digest = footprint_digest(&actual_footprint);
    let replacement_value_digest = echo_operation_atom_value_digest_v1(
        required_attachment_type,
        &admitted.invocation.replacement_bytes,
    );
    let result_id = operation_result_id(
        installed,
        &admitted.invocation,
        mode,
        replacement_value_digest,
        patch.digest(),
    );
    let private_evaluation_id = private_evaluation_id_from_parts(
        installed.installed_operation_id,
        installed.program_id,
        admitted.admission_id,
        admitted.invocation_id,
        current_basis.identity(),
        declared_footprint_digest,
        actual_footprint_digest,
        consumed_budget,
        patch.digest(),
        result_id,
    );
    let preparation_id = preparation_id(private_evaluation_id, patch.digest(), result_id);
    EchoOperationPreparationV1::Prepared(Box::new(PreparedEchoOperationV1 {
        installed: installed.clone(),
        invocation: admitted.invocation,
        invocation_id,
        canonical_invocation_bytes: admitted.canonical_invocation_bytes,
        invocation_admission_policy_id: admitted.admission_policy_id,
        invocation_admission_maximum_budget: admitted.admission_policy.maximum_delegated_budget,
        invocation_admission_id: admitted.admission_id,
        evaluation_basis: current_basis,
        declared_footprint,
        actual_footprint,
        declared_footprint_digest,
        actual_footprint_digest,
        consumed_budget,
        patch,
        result_id,
        private_evaluation_id,
        preparation_id,
        evaluation_authority: admitted.evaluation_authority,
    }))
}

/// Terminal posture bound by an executable-operation receipt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EchoOperationTerminalPostureV1 {
    /// The exact prepared patch entered the parent worldline.
    Committed,
    /// The parent basis changed; no prepared patch entered history.
    NotCommittedBasisChanged,
    /// The admitted package was unavailable at the commit crossing.
    NotCommittedInstallationUnavailable,
    /// Private evaluation belongs to another Echo runtime owner.
    NotCommittedEvaluationAuthorityMismatch,
}

/// Typed receipt for the exact executable semantics and terminal outcome.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EchoOperationReceiptV1 {
    package_id: EchoOperationPackageIdV1,
    package_admission_id: EchoOperationPackageAdmissionIdV1,
    installed_operation_id: InstalledEchoOperationIdV1,
    operation_coordinate: String,
    semantic_identity: Hash,
    lawpack_identity: Hash,
    target_profile_identity: Hash,
    interpreter_profile_identity: Hash,
    intrinsic_profile_identity: Hash,
    package_admission_policy_id: Hash,
    authority_profile_identity: Hash,
    authority_grant_identity: Hash,
    invocation_admission_policy_id: Hash,
    invocation_admission_maximum_budget: EchoOperationBudgetV1,
    invocation_admission_id: EchoOperationInvocationAdmissionIdV1,
    program_id: EchoOperationProgramIdV1,
    invocation_id: EchoOperationInvocationIdV1,
    invocation_bytes_digest: Hash,
    evaluation_basis: EchoOperationEvaluationBasisV1,
    evaluation_basis_id: EchoOperationEvaluationBasisIdV1,
    declared_footprint_digest: Hash,
    actual_footprint_digest: Hash,
    delegated_budget: EchoOperationBudgetV1,
    consumed_budget: EchoOperationBudgetV1,
    private_evaluation_id: EchoOperationPrivateEvaluationIdV1,
    preparation_id: PreparedEchoOperationIdV1,
    prepared_patch_digest: Hash,
    prepared_result_id: EchoOperationResultIdV1,
    committed_patch_digest: Option<Hash>,
    committed_result_id: Option<EchoOperationResultIdV1>,
    state_root_before: Hash,
    state_root_after: Hash,
    commit_id: Hash,
    composition_digest: Option<Hash>,
    tick_receipt_digest: Hash,
    commit_global_tick: Option<GlobalTick>,
    worldline_tick_after: WorldlineTick,
    terminal_posture: EchoOperationTerminalPostureV1,
    terminal_outcome_digest: Hash,
    receipt_digest: Hash,
}

impl EchoOperationReceiptV1 {
    /// Returns the terminal disposition.
    #[must_use]
    pub const fn terminal_posture(&self) -> EchoOperationTerminalPostureV1 {
        self.terminal_posture
    }

    /// Returns the admitted package identity.
    #[must_use]
    pub const fn package_id(&self) -> EchoOperationPackageIdV1 {
        self.package_id
    }

    /// Returns Echo's package-admission evidence identity.
    #[must_use]
    pub const fn package_admission_id(&self) -> EchoOperationPackageAdmissionIdV1 {
        self.package_admission_id
    }

    /// Returns Echo's installed-operation identity.
    #[must_use]
    pub const fn installed_operation_id(&self) -> InstalledEchoOperationIdV1 {
        self.installed_operation_id
    }

    /// Returns the public Edict operation coordinate.
    #[must_use]
    pub fn operation_coordinate(&self) -> &str {
        &self.operation_coordinate
    }

    /// Returns the canonical invocation identity admitted by Echo.
    #[must_use]
    pub const fn invocation_id(&self) -> EchoOperationInvocationIdV1 {
        self.invocation_id
    }

    /// Returns the separately domain-bound digest of canonical invocation bytes.
    #[must_use]
    pub const fn invocation_bytes_digest(&self) -> Hash {
        self.invocation_bytes_digest
    }

    /// Returns the exact program identity subordinate to the package.
    #[must_use]
    pub const fn program_id(&self) -> EchoOperationProgramIdV1 {
        self.program_id
    }

    /// Returns the exact private-evaluation basis identity.
    #[must_use]
    pub const fn evaluation_basis_id(&self) -> EchoOperationEvaluationBasisIdV1 {
        self.evaluation_basis_id
    }

    /// Returns the exact complete basis value used during private evaluation.
    #[must_use]
    pub const fn evaluation_basis(&self) -> EchoOperationEvaluationBasisV1 {
        self.evaluation_basis
    }

    /// Returns the invocation-derived declared-footprint identity.
    #[must_use]
    pub const fn declared_footprint_digest(&self) -> Hash {
        self.declared_footprint_digest
    }

    /// Returns the evaluator-recorded actual-footprint identity.
    #[must_use]
    pub const fn actual_footprint_digest(&self) -> Hash {
        self.actual_footprint_digest
    }

    /// Returns the invocation's admitted delegated budget.
    #[must_use]
    pub const fn delegated_budget(&self) -> EchoOperationBudgetV1 {
        self.delegated_budget
    }

    /// Returns the resource budget consumed during private evaluation.
    #[must_use]
    pub const fn consumed_budget(&self) -> EchoOperationBudgetV1 {
        self.consumed_budget
    }

    /// Returns the invocation-admission evidence consumed by evaluation.
    #[must_use]
    pub const fn invocation_admission_id(&self) -> EchoOperationInvocationAdmissionIdV1 {
        self.invocation_admission_id
    }

    /// Returns the bounded private-evaluation identity.
    #[must_use]
    pub const fn private_evaluation_id(&self) -> EchoOperationPrivateEvaluationIdV1 {
        self.private_evaluation_id
    }

    /// Returns the complete preparation identity.
    #[must_use]
    pub const fn preparation_id(&self) -> PreparedEchoOperationIdV1 {
        self.preparation_id
    }

    /// Returns the typed result produced during private evaluation.
    #[must_use]
    pub const fn prepared_result_id(&self) -> EchoOperationResultIdV1 {
        self.prepared_result_id
    }

    /// Returns the typed result only when it entered the committed consequence.
    #[must_use]
    pub const fn committed_result_id(&self) -> Option<EchoOperationResultIdV1> {
        self.committed_result_id
    }

    /// Returns the parent-visible patch digest only for a committed consequence.
    #[must_use]
    pub const fn committed_patch_digest(&self) -> Option<Hash> {
        self.committed_patch_digest
    }

    /// Returns the evaluated candidate patch identity, committed or not.
    #[must_use]
    pub const fn prepared_patch_digest(&self) -> Hash {
        self.prepared_patch_digest
    }

    /// Returns composition evidence only for a committed consequence.
    ///
    /// Direct compatibility commits bind singleton composition. Scheduler-owned
    /// Action receipts bind the exact composite Tick consequence.
    #[must_use]
    pub const fn composition_digest(&self) -> Option<Hash> {
        self.composition_digest
    }

    /// Replaces composition evidence for adversarial recovery tests.
    #[cfg(any(test, feature = "host_test"))]
    pub fn replace_composition_digest_for_test(&mut self, composition_digest: Hash) {
        self.composition_digest = Some(composition_digest);
    }

    /// Returns the closed terminal-outcome identity.
    #[must_use]
    pub const fn terminal_outcome_digest(&self) -> Hash {
        self.terminal_outcome_digest
    }

    /// Returns the graph-state root at the terminal commit crossing's start.
    #[must_use]
    pub const fn state_root_before(&self) -> Hash {
        self.state_root_before
    }

    /// Returns the graph-state root after the terminal crossing.
    #[must_use]
    pub const fn state_root_after(&self) -> Hash {
        self.state_root_after
    }

    /// Returns the committed consequence identity, or the zero sentinel when not committed.
    #[must_use]
    pub const fn commit_id(&self) -> Hash {
        self.commit_id
    }

    /// Returns the commit's global tick, or `None` when no consequence committed.
    #[must_use]
    pub const fn commit_global_tick(&self) -> Option<GlobalTick> {
        self.commit_global_tick
    }

    /// Returns the worldline frontier tick after the terminal crossing.
    #[must_use]
    pub const fn worldline_tick_after(&self) -> WorldlineTick {
        self.worldline_tick_after
    }

    /// Returns the content identity of this typed receipt.
    #[must_use]
    pub const fn digest(&self) -> Hash {
        self.receipt_digest
    }

    pub(crate) const fn tick_receipt_digest(&self) -> Hash {
        self.tick_receipt_digest
    }

    /// Encodes the complete typed receipt as canonical Edict CBOR.
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, EchoOperationArtifactErrorV1> {
        encode_canonical_cbor_v1(&self.to_value()).map_err(canonical_error)
    }

    fn to_value(&self) -> CanonicalValueV1 {
        map_value([
            (
                "actual_footprint_digest",
                hash_value(self.actual_footprint_digest),
            ),
            (
                "authority_grant_identity",
                hash_value(self.authority_grant_identity),
            ),
            (
                "authority_profile_identity",
                hash_value(self.authority_profile_identity),
            ),
            ("commit_id", hash_value(self.commit_id)),
            (
                "commit_global_tick",
                self.commit_global_tick
                    .map_or(CanonicalValueV1::Null, |tick| uint_value(tick.as_u64())),
            ),
            ("consumed_budget", self.consumed_budget.to_value()),
            (
                "composition_digest",
                self.composition_digest
                    .map_or(CanonicalValueV1::Null, hash_value),
            ),
            (
                "declared_footprint_digest",
                hash_value(self.declared_footprint_digest),
            ),
            ("delegated_budget", self.delegated_budget.to_value()),
            ("evaluation_basis", self.evaluation_basis.to_value()),
            (
                "evaluation_basis_id",
                hash_value(self.evaluation_basis_id.as_hash()),
            ),
            (
                "installed_operation_id",
                hash_value(self.installed_operation_id.as_hash()),
            ),
            (
                "interpreter_profile_identity",
                hash_value(self.interpreter_profile_identity),
            ),
            (
                "intrinsic_profile_identity",
                hash_value(self.intrinsic_profile_identity),
            ),
            (
                "invocation_admission_id",
                hash_value(self.invocation_admission_id.as_hash()),
            ),
            (
                "invocation_admission_policy_id",
                hash_value(self.invocation_admission_policy_id),
            ),
            (
                "invocation_admission_maximum_budget",
                self.invocation_admission_maximum_budget.to_value(),
            ),
            (
                "invocation_bytes_digest",
                hash_value(self.invocation_bytes_digest),
            ),
            ("invocation_id", hash_value(self.invocation_id.as_hash())),
            ("lawpack_identity", hash_value(self.lawpack_identity)),
            (
                "operation_coordinate",
                text_value(&self.operation_coordinate),
            ),
            (
                "package_admission_policy_id",
                hash_value(self.package_admission_policy_id),
            ),
            (
                "package_admission_id",
                hash_value(self.package_admission_id.as_hash()),
            ),
            ("package_id", hash_value(self.package_id.as_hash())),
            (
                "prepared_patch_digest",
                hash_value(self.prepared_patch_digest),
            ),
            (
                "prepared_result_id",
                hash_value(self.prepared_result_id.as_hash()),
            ),
            ("preparation_id", hash_value(self.preparation_id.as_hash())),
            (
                "private_evaluation_id",
                hash_value(self.private_evaluation_id.as_hash()),
            ),
            (
                "committed_patch_digest",
                self.committed_patch_digest
                    .map_or(CanonicalValueV1::Null, hash_value),
            ),
            (
                "committed_result_id",
                self.committed_result_id
                    .map_or(CanonicalValueV1::Null, |id| hash_value(id.as_hash())),
            ),
            ("program_id", hash_value(self.program_id.as_hash())),
            ("receipt_digest", hash_value(self.receipt_digest)),
            ("schema", text_value("echo.operation-receipt/v1")),
            ("semantic_identity", hash_value(self.semantic_identity)),
            ("state_root_after", hash_value(self.state_root_after)),
            ("state_root_before", hash_value(self.state_root_before)),
            (
                "target_profile_identity",
                hash_value(self.target_profile_identity),
            ),
            (
                "terminal_posture",
                text_value(match self.terminal_posture {
                    EchoOperationTerminalPostureV1::Committed => "committed",
                    EchoOperationTerminalPostureV1::NotCommittedBasisChanged => {
                        "not-committed:basis-changed"
                    }
                    EchoOperationTerminalPostureV1::NotCommittedInstallationUnavailable => {
                        "not-committed:installation-unavailable"
                    }
                    EchoOperationTerminalPostureV1::NotCommittedEvaluationAuthorityMismatch => {
                        "not-committed:evaluation-authority-mismatch"
                    }
                }),
            ),
            (
                "terminal_outcome_digest",
                hash_value(self.terminal_outcome_digest),
            ),
            ("tick_receipt_digest", hash_value(self.tick_receipt_digest)),
            (
                "worldline_tick_after",
                uint_value(self.worldline_tick_after.as_u64()),
            ),
        ])
    }

    pub(crate) fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, EchoOperationArtifactErrorV1> {
        let value = decode_canonical_cbor_v1(bytes).map_err(canonical_error)?;
        let mut fields = exact_text_map(
            value,
            &[
                "actual_footprint_digest",
                "authority_grant_identity",
                "authority_profile_identity",
                "commit_global_tick",
                "commit_id",
                "committed_result_id",
                "composition_digest",
                "consumed_budget",
                "declared_footprint_digest",
                "delegated_budget",
                "evaluation_basis",
                "evaluation_basis_id",
                "installed_operation_id",
                "interpreter_profile_identity",
                "intrinsic_profile_identity",
                "invocation_admission_id",
                "invocation_admission_policy_id",
                "invocation_admission_maximum_budget",
                "invocation_bytes_digest",
                "invocation_id",
                "lawpack_identity",
                "operation_coordinate",
                "package_admission_id",
                "package_admission_policy_id",
                "package_id",
                "committed_patch_digest",
                "prepared_patch_digest",
                "prepared_result_id",
                "preparation_id",
                "private_evaluation_id",
                "program_id",
                "receipt_digest",
                "schema",
                "semantic_identity",
                "state_root_after",
                "state_root_before",
                "target_profile_identity",
                "terminal_posture",
                "terminal_outcome_digest",
                "tick_receipt_digest",
                "worldline_tick_after",
            ],
        )?;
        require_text(&mut fields, "schema", "echo.operation-receipt/v1")?;
        let terminal_posture = match take_text(&mut fields, "terminal_posture")?.as_str() {
            "committed" => EchoOperationTerminalPostureV1::Committed,
            "not-committed:basis-changed" => {
                EchoOperationTerminalPostureV1::NotCommittedBasisChanged
            }
            "not-committed:installation-unavailable" => {
                EchoOperationTerminalPostureV1::NotCommittedInstallationUnavailable
            }
            "not-committed:evaluation-authority-mismatch" => {
                EchoOperationTerminalPostureV1::NotCommittedEvaluationAuthorityMismatch
            }
            _ => return Err(invalid_structure("unknown operation receipt posture")),
        };
        let commit_global_tick = match take_field(&mut fields, "commit_global_tick")? {
            CanonicalValueV1::Null => None,
            CanonicalValueV1::Integer(value) => Some(GlobalTick::from_raw(i128_to_u64(value)?)),
            _ => return Err(invalid_structure("commit_global_tick must be null or uint")),
        };
        let mut receipt = Self {
            package_id: EchoOperationPackageIdV1(take_hash(&mut fields, "package_id")?),
            package_admission_id: EchoOperationPackageAdmissionIdV1(take_hash(
                &mut fields,
                "package_admission_id",
            )?),
            installed_operation_id: InstalledEchoOperationIdV1(take_hash(
                &mut fields,
                "installed_operation_id",
            )?),
            operation_coordinate: take_text(&mut fields, "operation_coordinate")?,
            semantic_identity: take_hash(&mut fields, "semantic_identity")?,
            lawpack_identity: take_hash(&mut fields, "lawpack_identity")?,
            target_profile_identity: take_hash(&mut fields, "target_profile_identity")?,
            interpreter_profile_identity: take_hash(&mut fields, "interpreter_profile_identity")?,
            intrinsic_profile_identity: take_hash(&mut fields, "intrinsic_profile_identity")?,
            package_admission_policy_id: take_hash(&mut fields, "package_admission_policy_id")?,
            authority_profile_identity: take_hash(&mut fields, "authority_profile_identity")?,
            authority_grant_identity: take_hash(&mut fields, "authority_grant_identity")?,
            invocation_admission_policy_id: take_hash(
                &mut fields,
                "invocation_admission_policy_id",
            )?,
            invocation_admission_maximum_budget: EchoOperationBudgetV1::from_value(take_field(
                &mut fields,
                "invocation_admission_maximum_budget",
            )?)?,
            invocation_admission_id: EchoOperationInvocationAdmissionIdV1(take_hash(
                &mut fields,
                "invocation_admission_id",
            )?),
            program_id: EchoOperationProgramIdV1(take_hash(&mut fields, "program_id")?),
            invocation_id: EchoOperationInvocationIdV1(take_hash(&mut fields, "invocation_id")?),
            invocation_bytes_digest: take_hash(&mut fields, "invocation_bytes_digest")?,
            evaluation_basis: EchoOperationEvaluationBasisV1::from_value(take_field(
                &mut fields,
                "evaluation_basis",
            )?)?,
            evaluation_basis_id: EchoOperationEvaluationBasisIdV1(take_hash(
                &mut fields,
                "evaluation_basis_id",
            )?),
            declared_footprint_digest: take_hash(&mut fields, "declared_footprint_digest")?,
            actual_footprint_digest: take_hash(&mut fields, "actual_footprint_digest")?,
            delegated_budget: EchoOperationBudgetV1::from_value(take_field(
                &mut fields,
                "delegated_budget",
            )?)?,
            consumed_budget: EchoOperationBudgetV1::from_value(take_field(
                &mut fields,
                "consumed_budget",
            )?)?,
            private_evaluation_id: EchoOperationPrivateEvaluationIdV1(take_hash(
                &mut fields,
                "private_evaluation_id",
            )?),
            preparation_id: PreparedEchoOperationIdV1(take_hash(&mut fields, "preparation_id")?),
            prepared_patch_digest: take_hash(&mut fields, "prepared_patch_digest")?,
            prepared_result_id: EchoOperationResultIdV1(take_hash(
                &mut fields,
                "prepared_result_id",
            )?),
            committed_patch_digest: match take_field(&mut fields, "committed_patch_digest")? {
                CanonicalValueV1::Null => None,
                CanonicalValueV1::Bytes(bytes) => Some(bytes.try_into().map_err(|_| {
                    invalid_structure("committed_patch_digest must be exactly 32 bytes")
                })?),
                _ => {
                    return Err(invalid_structure(
                        "committed_patch_digest must be null or 32 bytes",
                    ))
                }
            },
            committed_result_id: take_optional_hash(&mut fields, "committed_result_id")?
                .map(EchoOperationResultIdV1),
            state_root_before: take_hash(&mut fields, "state_root_before")?,
            state_root_after: take_hash(&mut fields, "state_root_after")?,
            commit_id: take_hash(&mut fields, "commit_id")?,
            composition_digest: take_optional_hash(&mut fields, "composition_digest")?,
            tick_receipt_digest: take_hash(&mut fields, "tick_receipt_digest")?,
            commit_global_tick,
            worldline_tick_after: WorldlineTick::from_raw(take_u64(
                &mut fields,
                "worldline_tick_after",
            )?),
            terminal_posture,
            terminal_outcome_digest: take_hash(&mut fields, "terminal_outcome_digest")?,
            receipt_digest: take_hash(&mut fields, "receipt_digest")?,
        };
        if receipt.evaluation_basis.identity() != receipt.evaluation_basis_id {
            return Err(invalid_structure(
                "operation receipt basis identity does not match complete basis",
            ));
        }
        if package_admission_id(receipt.package_id, receipt.package_admission_policy_id)
            != receipt.package_admission_id
        {
            return Err(invalid_structure(
                "operation receipt package-admission identity mismatch",
            ));
        }
        if invocation_admission_id(
            receipt.installed_operation_id,
            receipt.invocation_id,
            receipt.invocation_admission_policy_id,
            receipt.evaluation_basis_id,
        ) != receipt.invocation_admission_id
        {
            return Err(invalid_structure(
                "operation receipt invocation-admission identity mismatch",
            ));
        }
        let retained_invocation_policy = EchoOperationInvocationAdmissionPolicyV1::new(
            receipt.authority_profile_identity,
            receipt.authority_grant_identity,
            receipt.invocation_admission_maximum_budget,
        );
        if retained_invocation_policy.identity() != receipt.invocation_admission_policy_id {
            return Err(invalid_structure(
                "operation receipt invocation-admission policy identity mismatch",
            ));
        }
        if !receipt
            .delegated_budget
            .fits_within(receipt.invocation_admission_maximum_budget)
            || !receipt
                .consumed_budget
                .fits_within(receipt.delegated_budget)
            || !receipt.consumed_budget.is_nonzero()
        {
            return Err(invalid_structure(
                "operation receipt budget evidence is internally inconsistent",
            ));
        }
        if receipt.declared_footprint_digest != receipt.actual_footprint_digest {
            return Err(invalid_structure(
                "operation receipt actual footprint differs from the v1 exact contract",
            ));
        }
        if private_evaluation_id_from_parts(
            receipt.installed_operation_id,
            receipt.program_id,
            receipt.invocation_admission_id,
            receipt.invocation_id,
            receipt.evaluation_basis_id,
            receipt.declared_footprint_digest,
            receipt.actual_footprint_digest,
            receipt.consumed_budget,
            receipt.prepared_patch_digest,
            receipt.prepared_result_id,
        ) != receipt.private_evaluation_id
        {
            return Err(invalid_structure(
                "operation receipt private-evaluation identity mismatch",
            ));
        }
        if preparation_id(
            receipt.private_evaluation_id,
            receipt.prepared_patch_digest,
            receipt.prepared_result_id,
        ) != receipt.preparation_id
        {
            return Err(invalid_structure(
                "operation receipt preparation identity mismatch",
            ));
        }
        if terminal_outcome_digest(&receipt) != receipt.terminal_outcome_digest {
            return Err(invalid_structure(
                "operation receipt terminal-outcome identity mismatch",
            ));
        }
        let retained_digest = receipt.receipt_digest;
        receipt.receipt_digest = [0; 32];
        let expected_digest = receipt_digest(&receipt);
        receipt.receipt_digest = retained_digest;
        if retained_digest != expected_digest {
            return Err(invalid_structure("operation receipt digest mismatch"));
        }
        let committed_worldline_tick_after = receipt
            .evaluation_basis
            .worldline_tick
            .as_u64()
            .checked_add(1)
            .map(WorldlineTick::from_raw);
        let expected_composition_digest = singleton_composition_digest_from_parts(
            receipt.preparation_id,
            receipt.prepared_patch_digest,
            receipt.prepared_result_id,
            receipt.evaluation_basis_id,
            receipt.actual_footprint_digest,
        );
        match receipt.terminal_posture {
            EchoOperationTerminalPostureV1::Committed
                if receipt.commit_global_tick.is_none()
                    || receipt.commit_id == [0; 32]
                    || receipt.tick_receipt_digest == [0; 32]
                    || receipt.committed_patch_digest.is_none()
                    || receipt.committed_result_id != Some(receipt.prepared_result_id)
                    || receipt.composition_digest.is_none()
                    || (receipt.committed_patch_digest == Some(receipt.prepared_patch_digest)
                        && receipt.composition_digest != Some(expected_composition_digest))
                    || receipt.state_root_before != receipt.evaluation_basis.state_root
                    || committed_worldline_tick_after != Some(receipt.worldline_tick_after) =>
            {
                return Err(invalid_structure(
                    "committed receipt is missing commit or tick evidence",
                ));
            }
            EchoOperationTerminalPostureV1::NotCommittedBasisChanged
            | EchoOperationTerminalPostureV1::NotCommittedInstallationUnavailable
            | EchoOperationTerminalPostureV1::NotCommittedEvaluationAuthorityMismatch
                if receipt.commit_global_tick.is_some()
                    || receipt.commit_id != [0; 32]
                    || receipt.tick_receipt_digest != [0; 32]
                    || receipt.committed_patch_digest.is_some()
                    || receipt.committed_result_id.is_some()
                    || receipt.composition_digest.is_some()
                    || receipt.state_root_before != receipt.state_root_after =>
            {
                return Err(invalid_structure(
                    "noncommitted receipt carries committed consequence evidence",
                ));
            }
            _ => {}
        }
        if receipt.to_canonical_bytes()? != bytes {
            return Err(artifact_error(
                EchoOperationArtifactErrorKindV1::NonCanonical,
                "receipt did not reproduce the exact retained bytes",
            ));
        }
        Ok(receipt)
    }
}

/// Terminal operation evidence, deliberately separate from generic [`TickReceipt`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EchoOperationExecutionEvidenceV1 {
    receipt: EchoOperationReceiptV1,
    snapshot: Option<Snapshot>,
    tick_receipt: Option<TickReceipt>,
    patch: Option<WarpTickPatchV1>,
}

impl EchoOperationExecutionEvidenceV1 {
    /// Returns the typed operation receipt.
    #[must_use]
    pub const fn receipt(&self) -> &EchoOperationReceiptV1 {
        &self.receipt
    }

    /// Returns the committed snapshot, if the exact-basis crossing succeeded.
    #[must_use]
    pub const fn snapshot(&self) -> Option<&Snapshot> {
        self.snapshot.as_ref()
    }

    /// Returns Echo's singleton commit receipt when committed.
    #[must_use]
    pub const fn tick_receipt(&self) -> Option<&TickReceipt> {
        self.tick_receipt.as_ref()
    }

    /// Returns the parent-visible committed patch, if any.
    #[must_use]
    pub const fn committed_patch(&self) -> Option<&WarpTickPatchV1> {
        self.patch.as_ref()
    }
}

pub(crate) fn retain_committed_execution_v1(
    evidence: &EchoOperationExecutionEvidenceV1,
) -> Result<Vec<u8>, EchoOperationArtifactErrorV1> {
    if evidence.receipt.terminal_posture != EchoOperationTerminalPostureV1::Committed
        || evidence.snapshot.is_none()
        || evidence.tick_receipt.is_none()
        || evidence.patch.is_none()
    {
        return Err(invalid_structure(
            "WAL execution retention requires one committed operation consequence",
        ));
    }
    evidence.receipt.to_canonical_bytes()
}

pub(crate) fn recover_committed_execution_receipt_v1(
    bytes: &[u8],
) -> Result<EchoOperationReceiptV1, EchoOperationArtifactErrorV1> {
    let receipt = EchoOperationReceiptV1::from_canonical_bytes(bytes)?;
    if receipt.terminal_posture != EchoOperationTerminalPostureV1::Committed {
        return Err(invalid_structure(
            "WAL execution evidence cannot retain a noncommitted candidate patch",
        ));
    }
    Ok(receipt)
}

pub(crate) fn validate_receipt_installation_v1(
    receipt: &EchoOperationReceiptV1,
    installed: &InstalledEchoOperationV1,
) -> Result<(), EchoOperationArtifactErrorV1> {
    let package_evidence_matches = receipt.package_id == installed.package_id
        && receipt.package_admission_id == installed.package_admission_id
        && receipt.installed_operation_id == installed.installed_operation_id;
    let package_policy_matches =
        receipt.package_admission_policy_id == installed.admission_policy_id;
    let semantic_evidence_matches = receipt.operation_coordinate == installed.operation_coordinate
        && receipt.semantic_identity == installed.semantic_identity
        && receipt.lawpack_identity == installed.lawpack_identity
        && receipt.target_profile_identity == installed.target_profile_identity
        && receipt.interpreter_profile_identity == installed.interpreter_profile_identity
        && receipt.intrinsic_profile_identity == installed.intrinsic_profile_identity
        && receipt.authority_profile_identity == installed.authority_profile_identity
        && receipt.evaluation_basis.application_basis.schema_identity
            == installed.application_basis_schema_identity
        && receipt.program_id == installed.program_id;
    let resource_evidence_matches = receipt
        .delegated_budget
        .fits_within(installed.budget_ceiling)
        && installed
            .program
            .minimum_budget()
            .fits_within(receipt.delegated_budget)
        && receipt
            .consumed_budget
            .fits_within(receipt.delegated_budget);
    if !package_evidence_matches
        || !package_policy_matches
        || !semantic_evidence_matches
        || !resource_evidence_matches
    {
        return Err(invalid_structure(
            "operation receipt does not match its retained installation",
        ));
    }
    Ok(())
}

/// Failure while turning one complete private preparation into commit material.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum EchoOperationCommitErrorV1 {
    /// The parent-visible patch was structurally invalid.
    #[error(transparent)]
    Patch(#[from] TickPatchError),
    /// The worldline-local transaction coordinate cannot advance.
    #[error("executable-operation transaction coordinate overflow")]
    TransactionCoordinateOverflow,
    /// The candidate count cannot be represented by Tick blocker indices.
    #[error("executable-operation Action batch has too many candidates")]
    TooManyCandidates,
    /// More than one candidate claimed the same canonical ingress identity.
    #[error("executable-operation Action batch contains duplicate ingress identity")]
    DuplicateCandidateIngress,
    /// Test-only scheduler fault injected before Action Tick construction.
    #[cfg(all(
        feature = "native_rule_bootstrap",
        feature = "trusted_runtime",
        any(test, feature = "host_test")
    ))]
    #[error("injected executable-operation Action Tick construction failure")]
    InjectedTickConstructionFailure,
}

pub(crate) struct EchoOperationCommitMaterialV1 {
    pub evidence: EchoOperationExecutionEvidenceV1,
    pub snapshot: Snapshot,
    pub tick_receipt: TickReceipt,
    pub patch: WarpTickPatchV1,
}

pub(crate) struct SchedulerEchoOperationCandidateV1 {
    pub ingress_id: Hash,
    pub scope: NodeKey,
    pub rule_id: Hash,
    pub preparation: EchoOperationPreparationV1,
}

pub(crate) struct EchoOperationActionBatchCommitMaterialV1 {
    pub snapshot: Snapshot,
    pub tick_receipt: TickReceipt,
    pub patch: WarpTickPatchV1,
    pub outcomes: Vec<(Hash, EchoOperationActionOutcomeV1)>,
}

enum SchedulerEchoOperationDecisionV1 {
    Applied(Box<PreparedEchoOperationV1>),
    Obstructed(EchoOperationObstructionV1),
    RejectedFootprintConflict {
        installed_operation_id: InstalledEchoOperationIdV1,
        invocation_id: EchoOperationInvocationIdV1,
        preparation_id: PreparedEchoOperationIdV1,
        blocked_by: Vec<u32>,
    },
}

#[derive(Clone, Copy, Debug)]
struct EchoOperationTerminalMaterialV1 {
    posture: EchoOperationTerminalPostureV1,
    state_root_before: Hash,
    state_root_after: Hash,
    commit_id: Hash,
    tick_receipt_digest: Hash,
    commit_global_tick: Option<GlobalTick>,
    worldline_tick_after: WorldlineTick,
}

pub(crate) fn commit_scheduler_action_batch_to_state_v1(
    mut candidates: Vec<SchedulerEchoOperationCandidateV1>,
    state: &mut WorldlineState,
    commit_global_tick: GlobalTick,
    policy_id: u32,
) -> Result<EchoOperationActionBatchCommitMaterialV1, EchoOperationCommitErrorV1> {
    // This order is part of the receipt-correlation contract: coordinator
    // outcome lookup maps the same Tick's ingress-sorted correlations onto
    // these receipt entries by position.
    candidates.sort_by_key(|candidate| candidate.ingress_id);
    if candidates
        .windows(2)
        .any(|pair| pair[0].ingress_id == pair[1].ingress_id)
    {
        return Err(EchoOperationCommitErrorV1::DuplicateCandidateIngress);
    }
    let state_root_before = state.state_root();
    let tx_raw = state
        .current_tick()
        .as_u64()
        .checked_add(1)
        .ok_or(EchoOperationCommitErrorV1::TransactionCoordinateOverflow)?;
    let tx = TxId::from_raw(tx_raw);

    let mut entries = Vec::with_capacity(candidates.len());
    let mut blocked_by = Vec::with_capacity(candidates.len());
    let mut decisions = Vec::with_capacity(candidates.len());
    let mut accepted_footprints: Vec<(u32, Footprint)> = Vec::new();
    let mut in_slots = Vec::new();
    let mut out_slots = Vec::new();
    let mut ops = Vec::new();
    let mut ordered_rule_ids = Vec::with_capacity(candidates.len());

    for (entry_index, candidate) in candidates.into_iter().enumerate() {
        let entry_index_u32 = u32::try_from(entry_index)
            .map_err(|_| EchoOperationCommitErrorV1::TooManyCandidates)?;
        ordered_rule_ids.push(candidate.rule_id);
        let scope_hash = crate::scope_hash(&candidate.rule_id, &candidate.scope);
        match candidate.preparation {
            EchoOperationPreparationV1::Obstructed(obstruction) => {
                entries.push(TickReceiptEntry {
                    rule_id: candidate.rule_id,
                    scope_hash,
                    scope: candidate.scope,
                    disposition: TickReceiptDisposition::Rejected(
                        TickReceiptRejection::ExecutableOperationObstruction,
                    ),
                });
                blocked_by.push(Vec::new());
                decisions.push((
                    candidate.ingress_id,
                    SchedulerEchoOperationDecisionV1::Obstructed(obstruction),
                ));
            }
            EchoOperationPreparationV1::Prepared(prepared) => {
                let blockers = accepted_footprints
                    .iter()
                    .filter_map(|(accepted_index, footprint)| {
                        crate::engine_impl::footprints_conflict(
                            prepared.actual_footprint(),
                            footprint,
                        )
                        .then_some(*accepted_index)
                    })
                    .collect::<Vec<_>>();
                if blockers.is_empty() {
                    entries.push(TickReceiptEntry {
                        rule_id: candidate.rule_id,
                        scope_hash,
                        scope: candidate.scope,
                        disposition: TickReceiptDisposition::Applied,
                    });
                    blocked_by.push(Vec::new());
                    accepted_footprints
                        .push((entry_index_u32, prepared.actual_footprint().clone()));
                    in_slots.extend_from_slice(prepared.patch().in_slots());
                    out_slots.extend_from_slice(prepared.patch().out_slots());
                    ops.extend_from_slice(prepared.patch().ops());
                    decisions.push((
                        candidate.ingress_id,
                        SchedulerEchoOperationDecisionV1::Applied(prepared),
                    ));
                } else {
                    entries.push(TickReceiptEntry {
                        rule_id: candidate.rule_id,
                        scope_hash,
                        scope: candidate.scope,
                        disposition: TickReceiptDisposition::Rejected(
                            TickReceiptRejection::FootprintConflict,
                        ),
                    });
                    blocked_by.push(blockers.clone());
                    decisions.push((
                        candidate.ingress_id,
                        SchedulerEchoOperationDecisionV1::RejectedFootprintConflict {
                            installed_operation_id: prepared.installed_operation_id(),
                            invocation_id: prepared.invocation_id(),
                            preparation_id: prepared.preparation_id(),
                            blocked_by: blockers,
                        },
                    ));
                }
            }
        }
    }

    let rule_pack_id = action_batch_rule_pack_id(&ordered_rule_ids);
    let patch = WarpTickPatchV1::new(
        policy_id,
        rule_pack_id,
        TickCommitStatus::Committed,
        in_slots,
        out_slots,
        ops,
    );
    let tick_receipt = TickReceipt::new(tx, entries, blocked_by);
    let plan_digest = action_batch_plan_digest(tick_receipt.entries());
    let applied = decisions
        .iter()
        .filter_map(|(_, decision)| match decision {
            SchedulerEchoOperationDecisionV1::Applied(prepared) => Some(prepared.as_ref()),
            SchedulerEchoOperationDecisionV1::Obstructed(_)
            | SchedulerEchoOperationDecisionV1::RejectedFootprintConflict { .. } => None,
        })
        .collect::<Vec<_>>();
    let rewrites_digest = action_batch_rewrites_digest(&applied);
    let composition_digest = action_batch_composition_digest(&applied, patch.digest());

    let mut next_state = state.warp_state.clone();
    patch.apply_to_state(&mut next_state)?;
    let state_root_after =
        crate::snapshot::compute_state_root_for_warp_state(&next_state, state.root());
    let parents = state
        .last_snapshot
        .as_ref()
        .map(|snapshot| vec![snapshot.hash])
        .unwrap_or_default();
    let commit_id = compute_commit_hash_v2(
        &state_root_after,
        &parents,
        &patch.digest(),
        patch.policy_id(),
    );
    let snapshot = Snapshot {
        root: *state.root(),
        hash: commit_id,
        state_root: state_root_after,
        parents,
        plan_digest,
        decision_digest: tick_receipt.digest(),
        rewrites_digest,
        patch_digest: patch.digest(),
        policy_id: patch.policy_id(),
        tx,
    };

    let outcomes = decisions
        .into_iter()
        .map(|(ingress_id, decision)| {
            let outcome = match decision {
                SchedulerEchoOperationDecisionV1::Applied(prepared) => {
                    let mut receipt = build_receipt(
                        &prepared,
                        EchoOperationTerminalMaterialV1 {
                            posture: EchoOperationTerminalPostureV1::Committed,
                            state_root_before,
                            state_root_after,
                            commit_id,
                            tick_receipt_digest: tick_receipt.digest(),
                            commit_global_tick: Some(commit_global_tick),
                            worldline_tick_after: WorldlineTick::from_raw(tx_raw),
                        },
                    );
                    receipt.committed_patch_digest = Some(patch.digest());
                    receipt.composition_digest = Some(composition_digest);
                    receipt.terminal_outcome_digest = terminal_outcome_digest(&receipt);
                    receipt.receipt_digest = receipt_digest(&receipt);
                    EchoOperationActionOutcomeV1::Committed(Box::new(receipt))
                }
                SchedulerEchoOperationDecisionV1::Obstructed(obstruction) => {
                    EchoOperationActionOutcomeV1::Obstructed(obstruction)
                }
                SchedulerEchoOperationDecisionV1::RejectedFootprintConflict {
                    installed_operation_id,
                    invocation_id,
                    preparation_id,
                    blocked_by,
                } => EchoOperationActionOutcomeV1::RejectedFootprintConflict {
                    installed_operation_id,
                    invocation_id,
                    preparation_id,
                    blocked_by,
                },
            };
            (ingress_id, outcome)
        })
        .collect();

    state.warp_state = next_state;
    state.last_snapshot = Some(snapshot.clone());
    state
        .tick_history
        .push((snapshot.clone(), tick_receipt.clone(), patch.clone()));
    state.tx_counter = tx_raw;

    Ok(EchoOperationActionBatchCommitMaterialV1 {
        snapshot,
        tick_receipt,
        patch,
        outcomes,
    })
}

fn action_batch_rule_pack_id(rule_ids: &[Hash]) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(ACTION_BATCH_RULE_PACK_DOMAIN);
    hasher.update(&(rule_ids.len() as u64).to_le_bytes());
    for rule_id in rule_ids {
        hasher.update(rule_id);
    }
    hasher.finalize().into()
}

fn action_batch_plan_digest(entries: &[TickReceiptEntry]) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(ACTION_BATCH_PLAN_DOMAIN);
    hasher.update(&(entries.len() as u64).to_le_bytes());
    for entry in entries {
        hasher.update(&entry.rule_id);
        hasher.update(&entry.scope_hash);
    }
    hasher.finalize().into()
}

fn action_batch_rewrites_digest(prepared: &[&PreparedEchoOperationV1]) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(ACTION_BATCH_REWRITES_DOMAIN);
    hasher.update(&(prepared.len() as u64).to_le_bytes());
    for member in prepared {
        hasher.update(&member.preparation_id().as_hash());
    }
    hasher.finalize().into()
}

fn action_batch_composition_digest(
    prepared: &[&PreparedEchoOperationV1],
    committed_patch_digest: Hash,
) -> Hash {
    action_batch_composition_digest_from_parts(
        committed_patch_digest,
        prepared.len(),
        prepared.iter().map(|member| {
            (
                member.preparation_id(),
                member.patch().digest(),
                member.result_id(),
                member.evaluation_basis().identity(),
                member.actual_footprint_digest(),
            )
        }),
    )
}

pub(crate) fn action_batch_composition_digest_from_receipts_v1(
    receipts: &[&EchoOperationReceiptV1],
    committed_patch_digest: Hash,
) -> Hash {
    action_batch_composition_digest_from_parts(
        committed_patch_digest,
        receipts.len(),
        receipts.iter().map(|receipt| {
            (
                receipt.preparation_id,
                receipt.prepared_patch_digest,
                receipt.prepared_result_id,
                receipt.evaluation_basis_id,
                receipt.actual_footprint_digest,
            )
        }),
    )
}

fn action_batch_composition_digest_from_parts(
    committed_patch_digest: Hash,
    member_count: usize,
    members: impl IntoIterator<
        Item = (
            PreparedEchoOperationIdV1,
            Hash,
            EchoOperationResultIdV1,
            EchoOperationEvaluationBasisIdV1,
            Hash,
        ),
    >,
) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(ACTION_BATCH_COMPOSITION_DOMAIN);
    hasher.update(&committed_patch_digest);
    hasher.update(&(member_count as u64).to_le_bytes());
    for (
        preparation_id,
        prepared_patch_digest,
        prepared_result_id,
        evaluation_basis_id,
        actual_footprint_digest,
    ) in members
    {
        hasher.update(&preparation_id.as_hash());
        hasher.update(&prepared_patch_digest);
        hasher.update(&prepared_result_id.as_hash());
        hasher.update(&evaluation_basis_id.as_hash());
        hasher.update(&actual_footprint_digest);
    }
    hasher.finalize().into()
}

pub(crate) fn not_committed_basis_changed(
    prepared: &PreparedEchoOperationV1,
    current_state_root: Hash,
    current_worldline_tick: WorldlineTick,
) -> EchoOperationExecutionEvidenceV1 {
    let receipt = build_receipt(
        prepared,
        EchoOperationTerminalMaterialV1 {
            posture: EchoOperationTerminalPostureV1::NotCommittedBasisChanged,
            state_root_before: current_state_root,
            state_root_after: current_state_root,
            commit_id: [0; 32],
            tick_receipt_digest: [0; 32],
            commit_global_tick: None,
            worldline_tick_after: current_worldline_tick,
        },
    );
    EchoOperationExecutionEvidenceV1 {
        receipt,
        snapshot: None,
        tick_receipt: None,
        patch: None,
    }
}

pub(crate) fn not_committed_installation_unavailable(
    prepared: &PreparedEchoOperationV1,
    current_state_root: Hash,
    current_worldline_tick: WorldlineTick,
) -> EchoOperationExecutionEvidenceV1 {
    let receipt = build_receipt(
        prepared,
        EchoOperationTerminalMaterialV1 {
            posture: EchoOperationTerminalPostureV1::NotCommittedInstallationUnavailable,
            state_root_before: current_state_root,
            state_root_after: current_state_root,
            commit_id: [0; 32],
            tick_receipt_digest: [0; 32],
            commit_global_tick: None,
            worldline_tick_after: current_worldline_tick,
        },
    );
    EchoOperationExecutionEvidenceV1 {
        receipt,
        snapshot: None,
        tick_receipt: None,
        patch: None,
    }
}

pub(crate) fn not_committed_evaluation_authority_mismatch(
    prepared: &PreparedEchoOperationV1,
    current_state_root: Hash,
    current_worldline_tick: WorldlineTick,
) -> EchoOperationExecutionEvidenceV1 {
    let receipt = build_receipt(
        prepared,
        EchoOperationTerminalMaterialV1 {
            posture: EchoOperationTerminalPostureV1::NotCommittedEvaluationAuthorityMismatch,
            state_root_before: current_state_root,
            state_root_after: current_state_root,
            commit_id: [0; 32],
            tick_receipt_digest: [0; 32],
            commit_global_tick: None,
            worldline_tick_after: current_worldline_tick,
        },
    );
    EchoOperationExecutionEvidenceV1 {
        receipt,
        snapshot: None,
        tick_receipt: None,
        patch: None,
    }
}

pub(crate) fn commit_prepared_to_state(
    prepared: &PreparedEchoOperationV1,
    state: &mut WorldlineState,
    commit_global_tick: GlobalTick,
) -> Result<EchoOperationCommitMaterialV1, EchoOperationCommitErrorV1> {
    let state_root_before = state.state_root();
    let tx_raw = state
        .current_tick()
        .as_u64()
        .checked_add(1)
        .ok_or(EchoOperationCommitErrorV1::TransactionCoordinateOverflow)?;
    let tx = TxId::from_raw(tx_raw);
    let scope = prepared.invocation.node;
    let rule_id = prepared.installed.installed_operation_id.as_hash();
    let tick_receipt = TickReceipt::new(
        tx,
        vec![TickReceiptEntry {
            rule_id,
            scope_hash: crate::scope_hash(&rule_id, &scope),
            scope,
            disposition: TickReceiptDisposition::Applied,
        }],
        vec![Vec::new()],
    );
    let mut next_state = state.warp_state.clone();
    prepared.patch.apply_to_state(&mut next_state)?;
    let state_root_after =
        crate::snapshot::compute_state_root_for_warp_state(&next_state, state.root());
    let parents = state
        .last_snapshot
        .as_ref()
        .map(|snapshot| vec![snapshot.hash])
        .unwrap_or_default();
    let patch_digest = prepared.patch.digest();
    let commit_id = compute_commit_hash_v2(
        &state_root_after,
        &parents,
        &patch_digest,
        prepared.patch.policy_id(),
    );
    let snapshot = Snapshot {
        root: *state.root(),
        hash: commit_id,
        state_root: state_root_after,
        parents,
        plan_digest: prepared_plan_digest(prepared),
        decision_digest: tick_receipt.digest(),
        rewrites_digest: prepared_rewrites_digest(prepared),
        patch_digest,
        policy_id: prepared.patch.policy_id(),
        tx,
    };
    let receipt = build_receipt(
        prepared,
        EchoOperationTerminalMaterialV1 {
            posture: EchoOperationTerminalPostureV1::Committed,
            state_root_before,
            state_root_after,
            commit_id,
            tick_receipt_digest: tick_receipt.digest(),
            commit_global_tick: Some(commit_global_tick),
            worldline_tick_after: WorldlineTick::from_raw(tx_raw),
        },
    );
    state.warp_state = next_state;
    state.last_snapshot = Some(snapshot.clone());
    state.tick_history.push((
        snapshot.clone(),
        tick_receipt.clone(),
        prepared.patch.clone(),
    ));
    state.tx_counter = tx_raw;
    Ok(EchoOperationCommitMaterialV1 {
        evidence: EchoOperationExecutionEvidenceV1 {
            receipt,
            snapshot: Some(snapshot.clone()),
            tick_receipt: Some(tick_receipt.clone()),
            patch: Some(prepared.patch.clone()),
        },
        snapshot,
        tick_receipt,
        patch: prepared.patch.clone(),
    })
}

pub(crate) fn genesis_commit_id(writer_head: WriterHeadKey, state_root: Hash) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(GENESIS_COMMIT_DOMAIN);
    hasher.update(writer_head.worldline_id.as_bytes());
    hasher.update(writer_head.head_id.as_bytes());
    hasher.update(&state_root);
    hasher.finalize().into()
}

fn build_receipt(
    prepared: &PreparedEchoOperationV1,
    terminal: EchoOperationTerminalMaterialV1,
) -> EchoOperationReceiptV1 {
    let invocation_bytes_digest = domain_hash(
        INVOCATION_BYTES_DIGEST_DOMAIN,
        &prepared.canonical_invocation_bytes,
    );
    let committed = terminal.posture == EchoOperationTerminalPostureV1::Committed;
    let mut receipt = EchoOperationReceiptV1 {
        package_id: prepared.installed.package_id,
        package_admission_id: prepared.installed.package_admission_id,
        installed_operation_id: prepared.installed.installed_operation_id,
        operation_coordinate: prepared.installed.operation_coordinate.clone(),
        semantic_identity: prepared.installed.semantic_identity,
        lawpack_identity: prepared.installed.lawpack_identity,
        target_profile_identity: prepared.installed.target_profile_identity,
        interpreter_profile_identity: prepared.installed.interpreter_profile_identity,
        intrinsic_profile_identity: prepared.installed.intrinsic_profile_identity,
        package_admission_policy_id: prepared.installed.admission_policy_id,
        authority_profile_identity: prepared.installed.authority_profile_identity,
        authority_grant_identity: prepared.invocation.authority_grant_identity,
        invocation_admission_policy_id: prepared.invocation_admission_policy_id,
        invocation_admission_maximum_budget: prepared.invocation_admission_maximum_budget,
        invocation_admission_id: prepared.invocation_admission_id,
        program_id: prepared.installed.program_id,
        invocation_id: prepared.invocation_id,
        invocation_bytes_digest,
        evaluation_basis: prepared.evaluation_basis,
        evaluation_basis_id: prepared.evaluation_basis.identity(),
        declared_footprint_digest: prepared.declared_footprint_digest,
        actual_footprint_digest: prepared.actual_footprint_digest,
        delegated_budget: prepared.invocation.delegated_budget,
        consumed_budget: prepared.consumed_budget,
        private_evaluation_id: prepared.private_evaluation_id,
        preparation_id: prepared.preparation_id,
        prepared_patch_digest: prepared.patch.digest(),
        prepared_result_id: prepared.result_id,
        committed_patch_digest: committed.then(|| prepared.patch.digest()),
        committed_result_id: committed.then_some(prepared.result_id),
        state_root_before: terminal.state_root_before,
        state_root_after: terminal.state_root_after,
        commit_id: terminal.commit_id,
        composition_digest: committed.then(|| singleton_composition_digest(prepared)),
        tick_receipt_digest: terminal.tick_receipt_digest,
        commit_global_tick: terminal.commit_global_tick,
        worldline_tick_after: terminal.worldline_tick_after,
        terminal_posture: terminal.posture,
        terminal_outcome_digest: [0; 32],
        receipt_digest: [0; 32],
    };
    receipt.terminal_outcome_digest = terminal_outcome_digest(&receipt);
    receipt.receipt_digest = receipt_digest(&receipt);
    receipt
}

fn receipt_digest(receipt: &EchoOperationReceiptV1) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(RECEIPT_DIGEST_DOMAIN);
    hasher.update(&receipt.package_id.as_hash());
    hasher.update(&receipt.package_admission_id.as_hash());
    hasher.update(&receipt.installed_operation_id.as_hash());
    hash_len_bytes(&mut hasher, receipt.operation_coordinate.as_bytes());
    hasher.update(&receipt.semantic_identity);
    hasher.update(&receipt.lawpack_identity);
    hasher.update(&receipt.target_profile_identity);
    hasher.update(&receipt.interpreter_profile_identity);
    hasher.update(&receipt.intrinsic_profile_identity);
    hasher.update(&receipt.package_admission_policy_id);
    hasher.update(&receipt.authority_profile_identity);
    hasher.update(&receipt.authority_grant_identity);
    hasher.update(&receipt.invocation_admission_policy_id);
    hash_budget(&mut hasher, receipt.invocation_admission_maximum_budget);
    hasher.update(&receipt.invocation_admission_id.as_hash());
    hasher.update(&receipt.program_id.as_hash());
    hasher.update(&receipt.invocation_id.as_hash());
    hasher.update(&receipt.invocation_bytes_digest);
    hasher.update(&receipt.evaluation_basis_id.as_hash());
    hasher.update(&receipt.declared_footprint_digest);
    hasher.update(&receipt.actual_footprint_digest);
    hash_budget(&mut hasher, receipt.delegated_budget);
    hash_budget(&mut hasher, receipt.consumed_budget);
    hasher.update(&receipt.private_evaluation_id.as_hash());
    hasher.update(&receipt.preparation_id.as_hash());
    hasher.update(&receipt.prepared_patch_digest);
    hasher.update(&receipt.prepared_result_id.as_hash());
    match receipt.committed_patch_digest {
        None => {
            hasher.update(&[0]);
        }
        Some(digest) => {
            hasher.update(&[1]);
            hasher.update(&digest);
        }
    }
    hash_optional_id(
        &mut hasher,
        receipt
            .committed_result_id
            .map(EchoOperationResultIdV1::as_hash),
    );
    hasher.update(&receipt.state_root_before);
    hasher.update(&receipt.state_root_after);
    hasher.update(&receipt.commit_id);
    hash_optional_id(&mut hasher, receipt.composition_digest);
    hasher.update(&receipt.tick_receipt_digest);
    match receipt.commit_global_tick {
        None => {
            hasher.update(&[0]);
        }
        Some(tick) => {
            hasher.update(&[1]);
            hasher.update(&tick.as_u64().to_le_bytes());
        }
    }
    hasher.update(&receipt.worldline_tick_after.as_u64().to_le_bytes());
    hasher.update(&[terminal_posture_code(receipt.terminal_posture)]);
    hasher.update(&receipt.terminal_outcome_digest);
    hasher.finalize().into()
}

fn terminal_outcome_digest(receipt: &EchoOperationReceiptV1) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(TERMINAL_OUTCOME_ID_DOMAIN);
    hasher.update(&receipt.preparation_id.as_hash());
    hasher.update(&receipt.prepared_patch_digest);
    hasher.update(&receipt.prepared_result_id.as_hash());
    hasher.update(&[terminal_posture_code(receipt.terminal_posture)]);
    hash_optional_id(&mut hasher, receipt.committed_patch_digest);
    hash_optional_id(
        &mut hasher,
        receipt
            .committed_result_id
            .map(EchoOperationResultIdV1::as_hash),
    );
    hash_optional_id(&mut hasher, receipt.composition_digest);
    hasher.update(&receipt.state_root_before);
    hasher.update(&receipt.state_root_after);
    hasher.update(&receipt.commit_id);
    hasher.finalize().into()
}

fn terminal_posture_code(posture: EchoOperationTerminalPostureV1) -> u8 {
    match posture {
        EchoOperationTerminalPostureV1::Committed => 1,
        EchoOperationTerminalPostureV1::NotCommittedBasisChanged => 2,
        EchoOperationTerminalPostureV1::NotCommittedInstallationUnavailable => 3,
        EchoOperationTerminalPostureV1::NotCommittedEvaluationAuthorityMismatch => 4,
    }
}

fn hash_optional_id(hasher: &mut Hasher, value: Option<Hash>) {
    match value {
        None => {
            hasher.update(&[0]);
        }
        Some(value) => {
            hasher.update(&[1]);
            hasher.update(&value);
        }
    }
}

fn singleton_composition_digest(prepared: &PreparedEchoOperationV1) -> Hash {
    singleton_composition_digest_from_parts(
        prepared.preparation_id,
        prepared.patch.digest(),
        prepared.result_id,
        prepared.evaluation_basis.identity(),
        prepared.actual_footprint_digest,
    )
}

fn singleton_composition_digest_from_parts(
    preparation_id: PreparedEchoOperationIdV1,
    patch_digest: Hash,
    result_id: EchoOperationResultIdV1,
    evaluation_basis_id: EchoOperationEvaluationBasisIdV1,
    actual_footprint_digest: Hash,
) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(COMPOSITION_DIGEST_DOMAIN);
    hasher.update(&1_u64.to_le_bytes());
    hasher.update(&preparation_id.as_hash());
    hasher.update(&patch_digest);
    hasher.update(&result_id.as_hash());
    hasher.update(&evaluation_basis_id.as_hash());
    hasher.update(&actual_footprint_digest);
    hasher.finalize().into()
}

fn prepared_plan_digest(prepared: &PreparedEchoOperationV1) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(PLAN_DIGEST_DOMAIN);
    hasher.update(&prepared.installed.installed_operation_id.as_hash());
    hasher.update(&prepared.invocation_admission_id.as_hash());
    hasher.update(&prepared.preparation_id.as_hash());
    hasher.update(&prepared.declared_footprint_digest);
    hasher.update(&prepared.invocation_admission_policy_id);
    hasher.finalize().into()
}

fn prepared_rewrites_digest(prepared: &PreparedEchoOperationV1) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(REWRITES_DIGEST_DOMAIN);
    hasher.update(&prepared.installed.program_id.as_hash());
    hasher.update(&prepared.private_evaluation_id.as_hash());
    hasher.update(&prepared.patch.digest());
    hasher.update(&prepared.result_id.as_hash());
    hasher.finalize().into()
}

fn package_admission_id(
    package_id: EchoOperationPackageIdV1,
    admission_policy_id: Hash,
) -> EchoOperationPackageAdmissionIdV1 {
    let mut hasher = Hasher::new();
    hasher.update(PACKAGE_ADMISSION_ID_DOMAIN);
    hasher.update(&package_id.as_hash());
    hasher.update(&admission_policy_id);
    EchoOperationPackageAdmissionIdV1(hasher.finalize().into())
}

fn installed_operation_id(installed: &InstalledEchoOperationV1) -> InstalledEchoOperationIdV1 {
    let mut hasher = Hasher::new();
    hasher.update(INSTALLATION_ID_DOMAIN);
    hasher.update(&installed.package_id.as_hash());
    hasher.update(&installed.package_admission_id.as_hash());
    hash_len_bytes(&mut hasher, installed.operation_coordinate.as_bytes());
    hasher.update(&installed.program_id.as_hash());
    hash_len_bytes(&mut hasher, &installed.canonical_package_bytes);
    InstalledEchoOperationIdV1(hasher.finalize().into())
}

fn invocation_admission_id(
    installed_operation_id: InstalledEchoOperationIdV1,
    invocation_id: EchoOperationInvocationIdV1,
    admission_policy_id: Hash,
    evaluation_basis_id: EchoOperationEvaluationBasisIdV1,
) -> EchoOperationInvocationAdmissionIdV1 {
    let mut hasher = Hasher::new();
    hasher.update(INVOCATION_ADMISSION_ID_DOMAIN);
    hasher.update(&installed_operation_id.as_hash());
    hasher.update(&invocation_id.as_hash());
    hasher.update(&admission_policy_id);
    hasher.update(&evaluation_basis_id.as_hash());
    EchoOperationInvocationAdmissionIdV1(hasher.finalize().into())
}

fn operation_result_id(
    installed: &InstalledEchoOperationV1,
    invocation: &EchoOperationInvocationV1,
    mode: AnchoredNodeOperationModeV1,
    replacement_value_digest: Hash,
    patch_digest: Hash,
) -> EchoOperationResultIdV1 {
    let mut hasher = Hasher::new();
    match mode {
        AnchoredNodeOperationModeV1::CompareAndSet {
            expected_value_digest,
        } => {
            // This is the exact pre-ADR-0024 update identity layout. Do not
            // add a variant tag or otherwise widen these hash inputs.
            hasher.update(RESULT_ID_DOMAIN);
            hasher.update(&installed.installed_operation_id.as_hash());
            hasher.update(&profile_digest(RESULT_SCHEMA));
            hasher.update(invocation.node.warp_id.as_bytes());
            hasher.update(invocation.node.local_id.as_bytes());
            hasher.update(&expected_value_digest);
        }
        AnchoredNodeOperationModeV1::CreateIfAbsent => {
            hasher.update(CREATE_RESULT_ID_DOMAIN);
            hasher.update(&installed.installed_operation_id.as_hash());
            hasher.update(&profile_digest(CREATE_RESULT_SCHEMA));
            hasher.update(invocation.node.warp_id.as_bytes());
            hasher.update(invocation.node.local_id.as_bytes());
            hasher.update(CREATE_RESULT_ABSENCE_PROPOSITION_DOMAIN);
        }
    }
    hasher.update(&replacement_value_digest);
    hasher.update(&patch_digest);
    EchoOperationResultIdV1(hasher.finalize().into())
}

#[allow(clippy::too_many_arguments)]
fn private_evaluation_id_from_parts(
    installed_operation_id: InstalledEchoOperationIdV1,
    program_id: EchoOperationProgramIdV1,
    invocation_admission_id: EchoOperationInvocationAdmissionIdV1,
    invocation_id: EchoOperationInvocationIdV1,
    evaluation_basis_id: EchoOperationEvaluationBasisIdV1,
    declared_footprint_digest: Hash,
    actual_footprint_digest: Hash,
    consumed_budget: EchoOperationBudgetV1,
    patch_digest: Hash,
    result_id: EchoOperationResultIdV1,
) -> EchoOperationPrivateEvaluationIdV1 {
    let mut hasher = Hasher::new();
    hasher.update(PRIVATE_EVALUATION_ID_DOMAIN);
    hasher.update(&installed_operation_id.as_hash());
    hasher.update(&program_id.as_hash());
    hasher.update(&invocation_admission_id.as_hash());
    hasher.update(&invocation_id.as_hash());
    hasher.update(&evaluation_basis_id.as_hash());
    hasher.update(&declared_footprint_digest);
    hasher.update(&actual_footprint_digest);
    hash_budget(&mut hasher, consumed_budget);
    hasher.update(&patch_digest);
    hasher.update(&result_id.as_hash());
    EchoOperationPrivateEvaluationIdV1(hasher.finalize().into())
}

fn preparation_id(
    private_evaluation_id: EchoOperationPrivateEvaluationIdV1,
    patch_digest: Hash,
    result_id: EchoOperationResultIdV1,
) -> PreparedEchoOperationIdV1 {
    let mut hasher = Hasher::new();
    hasher.update(PREPARATION_ID_DOMAIN);
    hasher.update(&private_evaluation_id.as_hash());
    hasher.update(&patch_digest);
    hasher.update(&result_id.as_hash());
    PreparedEchoOperationIdV1(hasher.finalize().into())
}

fn obstruction_kind_code(kind: EchoOperationObstructionKindV1) -> u8 {
    match kind {
        EchoOperationObstructionKindV1::OperationUnavailable => 1,
        EchoOperationObstructionKindV1::BasisChanged => 2,
        EchoOperationObstructionKindV1::BudgetExceeded => 3,
        EchoOperationObstructionKindV1::NodeMissing => 4,
        EchoOperationObstructionKindV1::NodeTypeMismatch => 5,
        EchoOperationObstructionKindV1::AttachmentMissing => 6,
        EchoOperationObstructionKindV1::AttachmentNotAtom => 7,
        EchoOperationObstructionKindV1::AttachmentTypeMismatch => 8,
        EchoOperationObstructionKindV1::PreconditionMismatch => 9,
        EchoOperationObstructionKindV1::FootprintViolation => 10,
        EchoOperationObstructionKindV1::ReplacementTooLarge => 11,
        EchoOperationObstructionKindV1::EvaluationAuthorityMismatch => 12,
    }
}

fn anchored_node_compare_and_set_footprint(
    node: NodeKey,
    descent_stack: &[AttachmentKey],
) -> Footprint {
    let mut footprint = Footprint::default();
    record_node_read(&mut footprint, node);
    for portal in descent_stack {
        footprint.a_read.insert(*portal);
    }
    let attachment = AttachmentKey::node_alpha(node);
    footprint.a_read.insert(attachment);
    footprint.a_write.insert(attachment);
    footprint
}

fn anchored_node_create_if_absent_footprint(
    node: NodeKey,
    descent_stack: &[AttachmentKey],
) -> Footprint {
    let mut footprint = anchored_node_compare_and_set_footprint(node, descent_stack);
    footprint.n_write.insert(node);
    footprint
}

pub(crate) fn operation_descent_stack(
    state: &WorldlineState,
    target_warp: crate::WarpId,
) -> Option<Vec<AttachmentKey>> {
    operation_descent_stack_with_portal_reads(state, target_warp, |_| true).ok()
}

fn operation_descent_stack_with_portal_reads(
    state: &WorldlineState,
    target_warp: crate::WarpId,
    mut read_portal: impl FnMut(AttachmentKey) -> bool,
) -> Result<Vec<AttachmentKey>, EchoOperationObstructionKindV1> {
    let mut current_warp = target_warp;
    let mut visited = BTreeSet::new();
    let mut reversed = Vec::new();

    loop {
        if !visited.insert(current_warp) {
            return Err(EchoOperationObstructionKindV1::FootprintViolation);
        }
        let instance = state
            .warp_state()
            .instance(&current_warp)
            .ok_or(EchoOperationObstructionKindV1::FootprintViolation)?;
        let Some(parent) = instance.parent else {
            if current_warp != state.root().warp_id {
                return Err(EchoOperationObstructionKindV1::FootprintViolation);
            }
            reversed.reverse();
            return Ok(reversed);
        };
        if !read_portal(parent) {
            return Err(EchoOperationObstructionKindV1::BudgetExceeded);
        }
        let parent_warp = match parent.owner {
            crate::AttachmentOwner::Node(node) => {
                if state
                    .store(&node.warp_id)
                    .ok_or(EchoOperationObstructionKindV1::FootprintViolation)?
                    .node_attachment(&node.local_id)
                    != Some(&AttachmentValue::Descend(current_warp))
                {
                    return Err(EchoOperationObstructionKindV1::FootprintViolation);
                }
                node.warp_id
            }
            crate::AttachmentOwner::Edge(edge) => {
                if state
                    .store(&edge.warp_id)
                    .ok_or(EchoOperationObstructionKindV1::FootprintViolation)?
                    .edge_attachment(&edge.local_id)
                    != Some(&AttachmentValue::Descend(current_warp))
                {
                    return Err(EchoOperationObstructionKindV1::FootprintViolation);
                }
                edge.warp_id
            }
        };
        reversed.push(parent);
        current_warp = parent_warp;
    }
}

fn record_node_read(footprint: &mut Footprint, node: NodeKey) {
    footprint.n_read.insert(node);
    footprint.factor_mask |= 1_u64 << (node.local_id.0[0] & 63);
}

fn footprint_digest(footprint: &Footprint) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(FOOTPRINT_DIGEST_DOMAIN);
    hash_node_set(
        &mut hasher,
        b"nr",
        footprint.n_read.len(),
        footprint.n_read.iter(),
    );
    hash_node_set(
        &mut hasher,
        b"nw",
        footprint.n_write.len(),
        footprint.n_write.iter(),
    );
    hash_edge_set(
        &mut hasher,
        b"er",
        footprint.e_read.len(),
        footprint.e_read.iter(),
    );
    hash_edge_set(
        &mut hasher,
        b"ew",
        footprint.e_write.len(),
        footprint.e_write.iter(),
    );
    hash_attachment_set(
        &mut hasher,
        b"ar",
        footprint.a_read.len(),
        footprint.a_read.iter(),
    );
    hash_attachment_set(
        &mut hasher,
        b"aw",
        footprint.a_write.len(),
        footprint.a_write.iter(),
    );
    hash_port_set(
        &mut hasher,
        b"bi",
        footprint.b_in.len(),
        footprint.b_in.iter(),
    );
    hash_port_set(
        &mut hasher,
        b"bo",
        footprint.b_out.len(),
        footprint.b_out.iter(),
    );
    hasher.update(&footprint.factor_mask.to_le_bytes());
    hasher.finalize().into()
}

fn hash_edge_set<'a>(
    hasher: &mut Hasher,
    label: &[u8],
    count: usize,
    values: impl Iterator<Item = &'a EdgeKey>,
) {
    hasher.update(label);
    hasher.update(&(count as u64).to_le_bytes());
    for value in values {
        hasher.update(value.warp_id.as_bytes());
        hasher.update(value.local_id.as_bytes());
    }
}

fn hash_node_set<'a>(
    hasher: &mut Hasher,
    label: &[u8],
    count: usize,
    values: impl Iterator<Item = &'a NodeKey>,
) {
    hasher.update(label);
    hasher.update(&(count as u64).to_le_bytes());
    for value in values {
        hasher.update(value.warp_id.as_bytes());
        hasher.update(value.local_id.as_bytes());
    }
}

fn hash_attachment_set<'a>(
    hasher: &mut Hasher,
    label: &[u8],
    count: usize,
    values: impl Iterator<Item = &'a AttachmentKey>,
) {
    hasher.update(label);
    hasher.update(&(count as u64).to_le_bytes());
    for value in values {
        match value.owner {
            crate::AttachmentOwner::Node(node) => {
                hasher.update(&[1]);
                hasher.update(node.warp_id.as_bytes());
                hasher.update(node.local_id.as_bytes());
            }
            crate::AttachmentOwner::Edge(edge) => {
                hasher.update(&[2]);
                hasher.update(edge.warp_id.as_bytes());
                hasher.update(edge.local_id.as_bytes());
            }
        }
        hasher.update(&[match value.plane {
            crate::AttachmentPlane::Alpha => 1,
            crate::AttachmentPlane::Beta => 2,
        }]);
    }
}

fn hash_port_set<'a>(
    hasher: &mut Hasher,
    label: &[u8],
    count: usize,
    values: impl Iterator<Item = &'a WarpScopedPortKey>,
) {
    hasher.update(label);
    hasher.update(&(count as u64).to_le_bytes());
    for (warp_id, port) in values {
        hasher.update(warp_id.as_bytes());
        hasher.update(&port.to_le_bytes());
    }
}

fn domain_hash(domain: &[u8], bytes: &[u8]) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(domain);
    hasher.update(&(bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
    hasher.finalize().into()
}

fn profile_digest(label: &str) -> Hash {
    domain_hash(b"echo:operation-profile:v1\0", label.as_bytes())
}

fn hash_len_bytes(hasher: &mut Hasher, bytes: &[u8]) {
    hasher.update(&(bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
}

fn hash_budget(hasher: &mut Hasher, budget: EchoOperationBudgetV1) {
    hasher.update(&budget.steps.to_le_bytes());
    hasher.update(&budget.read_bytes.to_le_bytes());
    hasher.update(&budget.write_bytes.to_le_bytes());
}

fn map_value<const N: usize>(fields: [(&str, CanonicalValueV1); N]) -> CanonicalValueV1 {
    CanonicalValueV1::Map(
        fields
            .into_iter()
            .map(|(key, value)| (text_value(key), value))
            .collect(),
    )
}

fn text_value(value: &str) -> CanonicalValueV1 {
    CanonicalValueV1::Text(value.to_owned())
}

fn hash_value(value: Hash) -> CanonicalValueV1 {
    CanonicalValueV1::Bytes(value.to_vec())
}

fn uint_value(value: u64) -> CanonicalValueV1 {
    CanonicalValueV1::Integer(i128::from(value))
}

fn exact_text_map(
    value: CanonicalValueV1,
    expected: &[&str],
) -> Result<BTreeMap<String, CanonicalValueV1>, EchoOperationArtifactErrorV1> {
    let CanonicalValueV1::Map(entries) = value else {
        return Err(invalid_structure("artifact root must be a map"));
    };
    let mut fields = BTreeMap::new();
    for (key, value) in entries {
        let CanonicalValueV1::Text(key) = key else {
            return Err(invalid_structure("artifact map keys must be text"));
        };
        if fields.insert(key, value).is_some() {
            return Err(invalid_structure("artifact map keys must be unique"));
        }
    }
    let actual = fields.keys().map(String::as_str).collect::<Vec<_>>();
    let mut expected = expected.to_vec();
    expected.sort_unstable();
    if actual != expected {
        return Err(invalid_structure(
            "artifact map fields differ from the v1 schema",
        ));
    }
    Ok(fields)
}

fn take_field(
    fields: &mut BTreeMap<String, CanonicalValueV1>,
    name: &str,
) -> Result<CanonicalValueV1, EchoOperationArtifactErrorV1> {
    fields
        .remove(name)
        .ok_or_else(|| invalid_structure(format!("missing field {name}")))
}

fn take_hash(
    fields: &mut BTreeMap<String, CanonicalValueV1>,
    name: &str,
) -> Result<Hash, EchoOperationArtifactErrorV1> {
    let CanonicalValueV1::Bytes(bytes) = take_field(fields, name)? else {
        return Err(invalid_structure(format!("field {name} must be bytes")));
    };
    bytes
        .try_into()
        .map_err(|_| invalid_structure(format!("field {name} must be exactly 32 bytes")))
}

fn take_optional_hash(
    fields: &mut BTreeMap<String, CanonicalValueV1>,
    name: &str,
) -> Result<Option<Hash>, EchoOperationArtifactErrorV1> {
    match take_field(fields, name)? {
        CanonicalValueV1::Null => Ok(None),
        CanonicalValueV1::Bytes(bytes) => bytes
            .try_into()
            .map(Some)
            .map_err(|_| invalid_structure(format!("field {name} must be null or 32 bytes"))),
        _ => Err(invalid_structure(format!(
            "field {name} must be null or bytes"
        ))),
    }
}

fn take_bytes(
    fields: &mut BTreeMap<String, CanonicalValueV1>,
    name: &str,
) -> Result<Vec<u8>, EchoOperationArtifactErrorV1> {
    let CanonicalValueV1::Bytes(bytes) = take_field(fields, name)? else {
        return Err(invalid_structure(format!("field {name} must be bytes")));
    };
    Ok(bytes)
}

fn take_text(
    fields: &mut BTreeMap<String, CanonicalValueV1>,
    name: &str,
) -> Result<String, EchoOperationArtifactErrorV1> {
    let CanonicalValueV1::Text(value) = take_field(fields, name)? else {
        return Err(invalid_structure(format!("field {name} must be text")));
    };
    Ok(value)
}

fn take_u64(
    fields: &mut BTreeMap<String, CanonicalValueV1>,
    name: &str,
) -> Result<u64, EchoOperationArtifactErrorV1> {
    let CanonicalValueV1::Integer(value) = take_field(fields, name)? else {
        return Err(invalid_structure(format!(
            "field {name} must be an unsigned integer"
        )));
    };
    i128_to_u64(value)
}

fn i128_to_u64(value: i128) -> Result<u64, EchoOperationArtifactErrorV1> {
    u64::try_from(value).map_err(|_| invalid_structure("integer must fit in u64"))
}

fn require_text(
    fields: &mut BTreeMap<String, CanonicalValueV1>,
    name: &str,
    expected: &str,
) -> Result<(), EchoOperationArtifactErrorV1> {
    let actual = take_text(fields, name)?;
    if actual == expected {
        Ok(())
    } else {
        Err(invalid_structure(format!(
            "field {name} must equal {expected}"
        )))
    }
}

fn canonical_error(error: CanonicalValueError) -> EchoOperationArtifactErrorV1 {
    let kind = if error.kind() == CanonicalValueErrorKind::NonCanonical {
        EchoOperationArtifactErrorKindV1::NonCanonical
    } else {
        EchoOperationArtifactErrorKindV1::MalformedCanonicalBytes
    };
    artifact_error(kind, error.to_string())
}

fn artifact_error(
    kind: EchoOperationArtifactErrorKindV1,
    detail: impl Into<String>,
) -> EchoOperationArtifactErrorV1 {
    EchoOperationArtifactErrorV1 {
        kind,
        detail: detail.into(),
    }
}

fn invalid_structure(detail: impl Into<String>) -> EchoOperationArtifactErrorV1 {
    artifact_error(EchoOperationArtifactErrorKindV1::InvalidStructure, detail)
}

fn admission_error(
    kind: EchoOperationAdmissionErrorKindV1,
    detail: impl Into<String>,
) -> EchoOperationAdmissionErrorV1 {
    EchoOperationAdmissionErrorV1 {
        kind,
        detail: detail.into(),
        artifact: None,
    }
}

fn invocation_admission_error(
    kind: EchoOperationInvocationAdmissionErrorKindV1,
    detail: impl Into<String>,
) -> EchoOperationInvocationAdmissionErrorV1 {
    EchoOperationInvocationAdmissionErrorV1 {
        kind,
        detail: detail.into(),
    }
}

fn installation_error(
    kind: EchoOperationInstallationErrorKindV1,
    detail: impl Into<String>,
) -> EchoOperationInstallationErrorV1 {
    EchoOperationInstallationErrorV1 {
        kind,
        detail: detail.into(),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn digest(byte: u8) -> Hash {
        [byte; 32]
    }

    fn node_key_from_bytes(bytes: [u8; 64]) -> NodeKey {
        let mut warp_id = [0; 32];
        let mut node_id = [0; 32];
        warp_id.copy_from_slice(&bytes[..32]);
        node_id.copy_from_slice(&bytes[32..]);
        NodeKey {
            warp_id: crate::WarpId(warp_id),
            local_id: crate::NodeId(node_id),
        }
    }

    #[test]
    fn footprint_digest_is_prefix_free_across_adjacent_sets() {
        let mut read_node_bytes = [0; 64];
        read_node_bytes[..2].copy_from_slice(b"nw");
        let mut write_node_bytes = [0; 64];
        write_node_bytes[..62].copy_from_slice(&read_node_bytes[2..]);
        write_node_bytes[62..].copy_from_slice(b"nw");

        let mut read_footprint = Footprint::default();
        read_footprint
            .n_read
            .insert(node_key_from_bytes(read_node_bytes));
        read_footprint.factor_mask = 1;
        let mut write_footprint = Footprint::default();
        write_footprint
            .n_write
            .insert(node_key_from_bytes(write_node_bytes));
        write_footprint.factor_mask = 1;

        assert_ne!(
            footprint_digest(&read_footprint),
            footprint_digest(&write_footprint),
            "set cardinalities must prevent boundary-shift digest collisions"
        );
    }

    #[test]
    fn footprint_digest_binds_every_resource_axis() {
        let node = node_key_from_bytes([7; 64]);
        let edge = EdgeKey {
            warp_id: node.warp_id,
            local_id: crate::EdgeId(digest(8)),
        };
        let attachment = AttachmentKey::node_alpha(node);
        let mut footprints = Vec::new();

        let mut footprint = Footprint::default();
        footprint.n_read.insert(node);
        footprints.push(footprint);
        let mut footprint = Footprint::default();
        footprint.n_write.insert(node);
        footprints.push(footprint);
        let mut footprint = Footprint::default();
        footprint.e_read.insert(edge);
        footprints.push(footprint);
        let mut footprint = Footprint::default();
        footprint.e_write.insert(edge);
        footprints.push(footprint);
        let mut footprint = Footprint::default();
        footprint.a_read.insert(attachment);
        footprints.push(footprint);
        let mut footprint = Footprint::default();
        footprint.a_write.insert(attachment);
        footprints.push(footprint);
        let mut footprint = Footprint::default();
        footprint.b_in.insert(node.warp_id, 9);
        footprints.push(footprint);
        let mut footprint = Footprint::default();
        footprint.b_out.insert(node.warp_id, 9);
        footprints.push(footprint);
        let mut footprint = Footprint::default();
        footprint.factor_mask = 1;
        footprints.push(footprint);

        let digests = footprints.iter().map(footprint_digest).collect::<Vec<_>>();
        for (index, digest) in digests.iter().enumerate() {
            assert_ne!(
                *digest,
                footprint_digest(&Footprint::default()),
                "resource axis {index} must differ from an empty footprint"
            );
            for (other_index, other) in digests.iter().enumerate().skip(index + 1) {
                assert_ne!(
                    digest, other,
                    "resource axes {index} and {other_index} must not alias"
                );
            }
        }
    }

    #[test]
    fn descended_creation_reads_and_retains_its_portal_chain() {
        let root_warp = crate::make_warp_id("operation-descended-root");
        let root_node = crate::make_node_id("operation-descended-root-node");
        let root = NodeKey {
            warp_id: root_warp,
            local_id: root_node,
        };
        let root_portal = AttachmentKey::node_alpha(root);
        let upper_warp = crate::make_warp_id("operation-descended-upper");
        let upper_root = crate::make_node_id("operation-descended-upper-root");
        let upper_root_key = NodeKey {
            warp_id: upper_warp,
            local_id: upper_root,
        };
        let upper_portal = AttachmentKey::node_alpha(upper_root_key);
        let middle_warp = crate::make_warp_id("operation-descended-middle");
        let middle_root = crate::make_node_id("operation-descended-middle-root");
        let middle_root_key = NodeKey {
            warp_id: middle_warp,
            local_id: middle_root,
        };
        let middle_portal = AttachmentKey::node_alpha(middle_root_key);
        let child_warp = crate::make_warp_id("operation-descended-child");
        let child_root = crate::make_node_id("operation-descended-child-root");
        let target = NodeKey {
            warp_id: child_warp,
            local_id: crate::make_node_id("operation-descended-target"),
        };

        let mut root_store = crate::GraphStore::new(root_warp);
        root_store.insert_node(
            root_node,
            NodeRecord {
                ty: crate::make_type_id("operation-descended-root-type"),
            },
        );
        root_store.set_node_attachment(root_node, Some(AttachmentValue::Descend(upper_warp)));
        let mut upper_store = crate::GraphStore::new(upper_warp);
        upper_store.insert_node(
            upper_root,
            NodeRecord {
                ty: crate::make_type_id("operation-descended-upper-root-type"),
            },
        );
        upper_store.set_node_attachment(upper_root, Some(AttachmentValue::Descend(middle_warp)));
        let mut middle_store = crate::GraphStore::new(middle_warp);
        middle_store.insert_node(
            middle_root,
            NodeRecord {
                ty: crate::make_type_id("operation-descended-middle-root-type"),
            },
        );
        middle_store.set_node_attachment(middle_root, Some(AttachmentValue::Descend(child_warp)));
        let mut child_store = crate::GraphStore::new(child_warp);
        child_store.insert_node(
            child_root,
            NodeRecord {
                ty: crate::make_type_id("operation-descended-child-root-type"),
            },
        );
        let mut warp_state = crate::WarpState::new();
        warp_state.upsert_instance(
            crate::WarpInstance {
                warp_id: root_warp,
                root_node,
                parent: None,
            },
            root_store,
        );
        warp_state.upsert_instance(
            crate::WarpInstance {
                warp_id: upper_warp,
                root_node: upper_root,
                parent: Some(root_portal),
            },
            upper_store,
        );
        warp_state.upsert_instance(
            crate::WarpInstance {
                warp_id: middle_warp,
                root_node: middle_root,
                parent: Some(upper_portal),
            },
            middle_store,
        );
        warp_state.upsert_instance(
            crate::WarpInstance {
                warp_id: child_warp,
                root_node: child_root,
                parent: Some(middle_portal),
            },
            child_store,
        );
        let state = WorldlineState::new(warp_state, root).expect("the descended fixture is lawful");

        let operation_coordinate = "echo.fixture.DescendedCreateIfAbsent.v1";
        let authority_profile = digest(40);
        let package = ExecutableOperationPackageV1::new(
            operation_coordinate,
            EchoOperationSemanticClosureV1::new(
                digest(41),
                digest(42),
                digest(43),
                digest(44),
                "echo.fixture.DescendedSchema.v1",
                digest(45),
                "echo.fixture.DescendedLawpack.v1",
                digest(46),
            ),
            echo_operation_create_if_absent_target_profile_identity_v1(),
            authority_profile,
            EchoOperationBudgetV1::new(16, 4_096, 4_096),
            EchoOperationProgramV1::anchored_node_attachment_create_if_absent(
                crate::make_type_id("operation-descended-created-node"),
                crate::make_type_id("operation-descended-created-atom"),
                1_024,
            ),
        );
        let package_bytes = package.to_canonical_bytes().expect("package encodes");
        let package_id = echo_operation_package_id_v1(&package_bytes);
        let installed = installed_from_admitted(
            admit_package_v1(
                &EchoOperationAdmissionPolicyV1::exact(
                    package_id,
                    operation_coordinate,
                    authority_profile,
                    EchoOperationBudgetV1::new(16, 4_096, 4_096),
                ),
                package_bytes,
            )
            .expect("package admits"),
        )
        .expect("package installs");

        let writer_head = WriterHeadKey {
            worldline_id: crate::WorldlineId::from_bytes(digest(47)),
            head_id: crate::HeadId::from_bytes(digest(48)),
        };
        let evaluation_basis = EchoOperationEvaluationBasisV1::new(
            writer_head,
            WorldlineTick::ZERO,
            None,
            state.state_root(),
            digest(49),
            echo_operation_anchored_node_absent_application_basis_v1(target),
        );
        let authority_grant = digest(50);
        let invocation = EchoOperationInvocationV1::anchored_node_attachment_create_if_absent(
            installed.package_id,
            operation_coordinate,
            evaluation_basis,
            authority_grant,
            EchoOperationBudgetV1::new(6, 160, 71),
            target,
            b"created".to_vec(),
        );
        let invocation_bytes = invocation.to_canonical_bytes().expect("invocation encodes");
        let invocation_policy = EchoOperationInvocationAdmissionPolicyV1::new(
            authority_profile,
            authority_grant,
            EchoOperationBudgetV1::new(16, 4_096, 4_096),
        );
        let evaluation_authority = EchoOperationEvaluationAuthorityV1::new();
        let admitted = admit_invocation_v1(
            Some(&installed),
            invocation_policy,
            &invocation_bytes,
            evaluation_basis,
            &state,
            evaluation_authority.clone(),
        )
        .expect("the descended invocation admits");
        let EchoOperationPreparationV1::Prepared(prepared) = prepare_operation_v1(
            Some(&installed),
            admitted,
            evaluation_basis,
            &state,
            crate::POLICY_ID_NO_POLICY_V0,
            &evaluation_authority,
        ) else {
            panic!("the descended invocation prepares");
        };

        for portal in [root_portal, upper_portal, middle_portal] {
            assert!(
                prepared
                    .actual_footprint()
                    .a_read
                    .iter()
                    .any(|key| key == &portal),
                "a descended creation must read every portal that makes its target reachable"
            );
            assert!(
                prepared
                    .patch()
                    .in_slots()
                    .contains(&SlotId::Attachment(portal)),
                "the replayable patch must retain every portal-chain dependency"
            );
        }
        assert_eq!(
            prepared.consumed_budget(),
            EchoOperationBudgetV1::new(6, 160, 71),
            "each portal pointer read must be charged as a bounded evaluator step"
        );

        let mut budget_limited_state = state.clone();
        budget_limited_state
            .warp_state
            .store_mut(&root_warp)
            .expect("the fixture retains its root store")
            .set_node_attachment(
                root_node,
                Some(AttachmentValue::Atom(AtomPayload::new(
                    crate::make_type_id("operation-descended-out-of-budget-corruption"),
                    Bytes::from_static(b"must-not-be-read"),
                ))),
            );
        let budget_limited_basis = EchoOperationEvaluationBasisV1::new(
            writer_head,
            WorldlineTick::ZERO,
            None,
            budget_limited_state.state_root(),
            digest(51),
            echo_operation_anchored_node_absent_application_basis_v1(target),
        );
        let budget_limited_invocation =
            EchoOperationInvocationV1::anchored_node_attachment_create_if_absent(
                installed.package_id,
                operation_coordinate,
                budget_limited_basis,
                authority_grant,
                EchoOperationBudgetV1::new(3, 64, 71),
                target,
                b"created".to_vec(),
            );
        let admitted = admit_invocation_v1(
            Some(&installed),
            EchoOperationInvocationAdmissionPolicyV1::new(
                authority_profile,
                authority_grant,
                EchoOperationBudgetV1::new(16, 4_096, 4_096),
            ),
            &budget_limited_invocation
                .to_canonical_bytes()
                .expect("budget-limited invocation encodes"),
            budget_limited_basis,
            &budget_limited_state,
            evaluation_authority.clone(),
        )
        .expect("the budget-limited invocation admits");
        let EchoOperationPreparationV1::Obstructed(obstruction) = prepare_operation_v1(
            Some(&installed),
            admitted,
            budget_limited_basis,
            &budget_limited_state,
            crate::POLICY_ID_NO_POLICY_V0,
            &evaluation_authority,
        ) else {
            panic!("the portal traversal must stop at its delegated read allowance");
        };
        assert_eq!(
            obstruction.kind(),
            EchoOperationObstructionKindV1::BudgetExceeded,
            "out-of-budget portal state must not influence the obstruction"
        );
    }

    #[test]
    fn public_artifact_encoders_refuse_values_their_decoders_refuse() {
        let invalid_program = EchoOperationProgramV1::anchored_node_attachment_compare_and_set(
            crate::make_type_id("invalid-program-node"),
            crate::make_type_id("invalid-program-atom"),
            0,
        );
        let error = invalid_program
            .to_canonical_bytes()
            .expect_err("a zero replacement bound must not produce program bytes");
        assert_eq!(
            error.kind(),
            EchoOperationArtifactErrorKindV1::UnsupportedProgram
        );
        invalid_program
            .identity()
            .expect_err("an invalid program must not mint a stable identity");

        let node = node_key_from_bytes([10; 64]);
        let basis = EchoOperationEvaluationBasisV1::new(
            WriterHeadKey {
                worldline_id: crate::WorldlineId::from_bytes(digest(11)),
                head_id: crate::HeadId::from_bytes(digest(12)),
            },
            WorldlineTick::ZERO,
            None,
            digest(13),
            digest(14),
            EchoOperationApplicationBasisV1::new(digest(15), digest(16)),
        );
        let invalid_invocation =
            EchoOperationInvocationV1::anchored_node_attachment_compare_and_set(
                EchoOperationPackageIdV1(digest(17)),
                "echo.fixture.InvalidBudget.v1",
                basis,
                digest(18),
                EchoOperationBudgetV1::new(0, 64, 32),
                node,
                digest(19),
                Vec::new(),
            );
        let error = invalid_invocation
            .to_canonical_bytes()
            .expect_err("a zero-step budget must not produce invocation bytes");
        assert_eq!(
            error.kind(),
            EchoOperationArtifactErrorKindV1::InvalidBudget
        );
        invalid_invocation
            .identity()
            .expect_err("an invalid invocation must not mint a stable identity");
    }

    fn retained_fixture_installation() -> InstalledEchoOperationV1 {
        let package = ExecutableOperationPackageV1::new(
            "echo.fixture.Retention.v1",
            EchoOperationSemanticClosureV1::new(
                digest(1),
                digest(2),
                digest(3),
                digest(4),
                "echo.fixture.RetentionSchema.v1",
                digest(5),
                "echo.fixture.RetentionLawpack.v1",
                digest(6),
            ),
            echo_operation_target_profile_identity_v1(),
            digest(7),
            EchoOperationBudgetV1::new(8, 512, 512),
            EchoOperationProgramV1::anchored_node_attachment_compare_and_set(
                crate::make_type_id("retention-node"),
                crate::make_type_id("retention-atom"),
                128,
            ),
        );
        let bytes = package.to_canonical_bytes().expect("package encodes");
        let package_id = echo_operation_package_id_v1(&bytes);
        let policy = EchoOperationAdmissionPolicyV1::exact(
            package_id,
            "echo.fixture.Retention.v1",
            digest(7),
            EchoOperationBudgetV1::new(8, 512, 512),
        );
        installed_from_admitted(admit_package_v1(&policy, bytes).expect("package admits"))
            .expect("an admitted program remains canonically encodable")
    }

    fn replace_map_field(bytes: &[u8], field_name: &str, replacement: CanonicalValueV1) -> Vec<u8> {
        let CanonicalValueV1::Map(mut fields) =
            decode_canonical_cbor_v1(bytes).expect("fixture bytes decode")
        else {
            panic!("fixture bytes must encode a map");
        };
        let field = fields
            .iter_mut()
            .find(|(key, _)| key == &CanonicalValueV1::Text(field_name.to_owned()))
            .expect("fixture field exists");
        field.1 = replacement;
        encode_canonical_cbor_v1(&CanonicalValueV1::Map(fields)).expect("fixture map encodes")
    }

    #[test]
    fn retained_action_outcome_rejects_impossible_blocker_count_before_allocation() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(ACTION_OUTCOME_RECORD_MAGIC);
        bytes.extend_from_slice(&digest(1));
        bytes.extend_from_slice(&digest(2));
        bytes.push(3);
        bytes.extend_from_slice(&digest(3));
        bytes.extend_from_slice(&digest(4));
        bytes.extend_from_slice(&digest(5));
        bytes.extend_from_slice(&u64::MAX.to_le_bytes());

        let error = recover_action_outcome_v1(&bytes)
            .expect_err("an impossible blocker count must fail before allocation");
        assert_eq!(
            error.kind(),
            EchoOperationArtifactErrorKindV1::InvalidStructure
        );
    }

    #[test]
    fn retained_installation_rejects_identity_substitution() {
        let installed = retained_fixture_installation();
        let bytes = retain_installation_v1(&installed).expect("installation retains");
        for field_name in [
            "admission_policy_id",
            "package_id",
            "package_admission_id",
            "installed_operation_id",
        ] {
            let substituted = replace_map_field(
                &bytes,
                field_name,
                CanonicalValueV1::Bytes(digest(99).to_vec()),
            );
            let error = recover_installation_v1(&substituted)
                .expect_err("retained installation identities cannot be substituted");
            assert_eq!(
                error.kind(),
                EchoOperationArtifactErrorKindV1::InvalidStructure,
                "field {field_name}"
            );
        }
        let substituted_policy = EchoOperationAdmissionPolicyV1::exact(
            installed.package_id,
            installed.operation_coordinate.clone(),
            installed.authority_profile_identity,
            EchoOperationBudgetV1::new(9, 512, 512),
        );
        let substituted =
            replace_map_field(&bytes, "admission_policy", substituted_policy.to_value());
        let error = recover_installation_v1(&substituted)
            .expect_err("recovery must revalidate retained package-admission policy material");
        assert_eq!(
            error.kind(),
            EchoOperationArtifactErrorKindV1::InvalidStructure
        );
    }

    #[test]
    fn retained_committed_receipt_rejects_field_substitution() {
        let installed = retained_fixture_installation();
        let basis = EchoOperationEvaluationBasisV1::new(
            WriterHeadKey {
                worldline_id: crate::WorldlineId::from_bytes(digest(8)),
                head_id: crate::HeadId::from_bytes(digest(9)),
            },
            WorldlineTick::ZERO,
            None,
            digest(10),
            digest(11),
            EchoOperationApplicationBasisV1::new(
                installed.application_basis_schema_identity,
                digest(13),
            ),
        );
        let invocation_id = EchoOperationInvocationIdV1(digest(14));
        let invocation_admission_maximum_budget = EchoOperationBudgetV1::new(8, 512, 512);
        let invocation_admission_policy_id = EchoOperationInvocationAdmissionPolicyV1::new(
            installed.authority_profile_identity,
            digest(19),
            invocation_admission_maximum_budget,
        )
        .identity();
        let invocation_admission_id = invocation_admission_id(
            installed.installed_operation_id,
            invocation_id,
            invocation_admission_policy_id,
            basis.identity(),
        );
        let declared_footprint_digest = digest(21);
        let actual_footprint_digest = declared_footprint_digest;
        let delegated_budget = EchoOperationBudgetV1::new(8, 512, 512);
        let consumed_budget = EchoOperationBudgetV1::new(4, 70, 40);
        let prepared_patch_digest = digest(17);
        let prepared_result_id = EchoOperationResultIdV1(digest(18));
        let private_evaluation_id = private_evaluation_id_from_parts(
            installed.installed_operation_id,
            installed.program_id,
            invocation_admission_id,
            invocation_id,
            basis.identity(),
            declared_footprint_digest,
            actual_footprint_digest,
            consumed_budget,
            prepared_patch_digest,
            prepared_result_id,
        );
        let prepared_id = preparation_id(
            private_evaluation_id,
            prepared_patch_digest,
            prepared_result_id,
        );
        let composition_digest = singleton_composition_digest_from_parts(
            prepared_id,
            prepared_patch_digest,
            prepared_result_id,
            basis.identity(),
            actual_footprint_digest,
        );
        let mut receipt = EchoOperationReceiptV1 {
            package_id: installed.package_id,
            package_admission_id: installed.package_admission_id,
            installed_operation_id: installed.installed_operation_id,
            operation_coordinate: installed.operation_coordinate.clone(),
            semantic_identity: installed.semantic_identity,
            lawpack_identity: installed.lawpack_identity,
            target_profile_identity: installed.target_profile_identity,
            interpreter_profile_identity: installed.interpreter_profile_identity,
            intrinsic_profile_identity: installed.intrinsic_profile_identity,
            package_admission_policy_id: installed.admission_policy_id,
            authority_profile_identity: installed.authority_profile_identity,
            authority_grant_identity: digest(19),
            invocation_admission_policy_id,
            invocation_admission_maximum_budget,
            invocation_admission_id,
            program_id: installed.program_id,
            invocation_id,
            invocation_bytes_digest: digest(20),
            evaluation_basis: basis,
            evaluation_basis_id: basis.identity(),
            declared_footprint_digest,
            actual_footprint_digest,
            delegated_budget,
            consumed_budget,
            private_evaluation_id,
            preparation_id: prepared_id,
            prepared_patch_digest,
            prepared_result_id,
            committed_patch_digest: Some(prepared_patch_digest),
            committed_result_id: Some(prepared_result_id),
            state_root_before: basis.state_root,
            state_root_after: digest(24),
            commit_id: digest(25),
            composition_digest: Some(composition_digest),
            tick_receipt_digest: digest(27),
            commit_global_tick: Some(GlobalTick::from_raw(1)),
            worldline_tick_after: WorldlineTick::from_raw(1),
            terminal_posture: EchoOperationTerminalPostureV1::Committed,
            terminal_outcome_digest: [0; 32],
            receipt_digest: [0; 32],
        };
        receipt.terminal_outcome_digest = terminal_outcome_digest(&receipt);
        receipt.receipt_digest = receipt_digest(&receipt);
        validate_receipt_installation_v1(&receipt, &installed)
            .expect("the retained receipt fixture matches its installation");
        let mut mismatched_policy_receipt = receipt.clone();
        mismatched_policy_receipt.invocation_admission_maximum_budget =
            EchoOperationBudgetV1::new(9, 512, 512);
        mismatched_policy_receipt.receipt_digest = receipt_digest(&mismatched_policy_receipt);
        let mismatched_policy_bytes = mismatched_policy_receipt
            .to_canonical_bytes()
            .expect("mismatched policy receipt retains structurally");
        let error = recover_committed_execution_receipt_v1(&mismatched_policy_bytes)
            .expect_err("retained invocation-admission policy material must match its identity");
        assert_eq!(
            error.kind(),
            EchoOperationArtifactErrorKindV1::InvalidStructure
        );

        let mut forged_private_evaluation = receipt.clone();
        forged_private_evaluation.private_evaluation_id =
            EchoOperationPrivateEvaluationIdV1(digest(90));
        forged_private_evaluation.preparation_id = preparation_id(
            forged_private_evaluation.private_evaluation_id,
            forged_private_evaluation.prepared_patch_digest,
            forged_private_evaluation.prepared_result_id,
        );
        forged_private_evaluation.composition_digest =
            Some(singleton_composition_digest_from_parts(
                forged_private_evaluation.preparation_id,
                forged_private_evaluation.prepared_patch_digest,
                forged_private_evaluation.prepared_result_id,
                forged_private_evaluation.evaluation_basis_id,
                forged_private_evaluation.actual_footprint_digest,
            ));
        forged_private_evaluation.terminal_outcome_digest =
            terminal_outcome_digest(&forged_private_evaluation);
        forged_private_evaluation.receipt_digest = receipt_digest(&forged_private_evaluation);
        let forged_private_bytes = forged_private_evaluation
            .to_canonical_bytes()
            .expect("coordinated private-evaluation substitution encodes");
        recover_committed_execution_receipt_v1(&forged_private_bytes)
            .expect_err("recovery must independently derive private-evaluation identity");

        let mut forged_composition = receipt.clone();
        forged_composition.composition_digest = Some(digest(91));
        forged_composition.terminal_outcome_digest = terminal_outcome_digest(&forged_composition);
        forged_composition.receipt_digest = receipt_digest(&forged_composition);
        let forged_composition_bytes = forged_composition
            .to_canonical_bytes()
            .expect("coordinated composition substitution encodes");
        recover_committed_execution_receipt_v1(&forged_composition_bytes)
            .expect_err("recovery must independently derive singleton composition identity");

        let mut impossible_budget = receipt.clone();
        impossible_budget.delegated_budget = EchoOperationBudgetV1::new(1, 1, 1);
        impossible_budget.receipt_digest = receipt_digest(&impossible_budget);
        let impossible_budget_bytes = impossible_budget
            .to_canonical_bytes()
            .expect("inconsistent budget evidence encodes");
        recover_committed_execution_receipt_v1(&impossible_budget_bytes)
            .expect_err("consumption cannot exceed the retained delegated budget");

        let mut below_program_minimum = receipt.clone();
        below_program_minimum.delegated_budget = EchoOperationBudgetV1::new(1, 1, 1);
        below_program_minimum.consumed_budget = EchoOperationBudgetV1::new(1, 1, 1);
        below_program_minimum.private_evaluation_id = private_evaluation_id_from_parts(
            below_program_minimum.installed_operation_id,
            below_program_minimum.program_id,
            below_program_minimum.invocation_admission_id,
            below_program_minimum.invocation_id,
            below_program_minimum.evaluation_basis_id,
            below_program_minimum.declared_footprint_digest,
            below_program_minimum.actual_footprint_digest,
            below_program_minimum.consumed_budget,
            below_program_minimum.prepared_patch_digest,
            below_program_minimum.prepared_result_id,
        );
        below_program_minimum.preparation_id = preparation_id(
            below_program_minimum.private_evaluation_id,
            below_program_minimum.prepared_patch_digest,
            below_program_minimum.prepared_result_id,
        );
        below_program_minimum.composition_digest = Some(singleton_composition_digest_from_parts(
            below_program_minimum.preparation_id,
            below_program_minimum.prepared_patch_digest,
            below_program_minimum.prepared_result_id,
            below_program_minimum.evaluation_basis_id,
            below_program_minimum.actual_footprint_digest,
        ));
        below_program_minimum.terminal_outcome_digest =
            terminal_outcome_digest(&below_program_minimum);
        below_program_minimum.receipt_digest = receipt_digest(&below_program_minimum);
        let below_program_minimum_bytes = below_program_minimum
            .to_canonical_bytes()
            .expect("coordinated low-budget evidence encodes");
        let recovered_below_program_minimum =
            recover_committed_execution_receipt_v1(&below_program_minimum_bytes)
                .expect("receipt self-validation cannot inspect an installed program");
        validate_receipt_installation_v1(&recovered_below_program_minimum, &installed)
            .expect_err("retained delegation must still cover the installed program minimum");

        let mut foreign_before_root = receipt.clone();
        foreign_before_root.state_root_before = digest(92);
        foreign_before_root.terminal_outcome_digest = terminal_outcome_digest(&foreign_before_root);
        foreign_before_root.receipt_digest = receipt_digest(&foreign_before_root);
        let foreign_before_root_bytes = foreign_before_root
            .to_canonical_bytes()
            .expect("foreign before-root evidence encodes");
        recover_committed_execution_receipt_v1(&foreign_before_root_bytes)
            .expect_err("committed evidence must start at the retained evaluation basis root");

        let bytes = receipt.to_canonical_bytes().expect("receipt retains");
        let substituted = replace_map_field(
            &bytes,
            "commit_id",
            CanonicalValueV1::Bytes(digest(100).to_vec()),
        );
        let error = recover_committed_execution_receipt_v1(&substituted)
            .expect_err("a retained receipt field cannot change under its digest");
        assert_eq!(
            error.kind(),
            EchoOperationArtifactErrorKindV1::InvalidStructure
        );
    }

    #[test]
    fn operation_result_id_preserves_the_legacy_hash_for_updates() {
        // The compare-and-set program keeps the exact pre-ADR-0024 hash
        // layout: a raw 32-byte previous digest with no variant tag.
        let installed = retained_fixture_installation();
        let node = node_key_from_bytes([20; 64]);
        let invocation = EchoOperationInvocationV1::anchored_node_attachment_compare_and_set(
            installed.package_id(),
            installed.operation_coordinate(),
            EchoOperationEvaluationBasisV1::new(
                WriterHeadKey {
                    worldline_id: crate::WorldlineId::from_bytes(digest(21)),
                    head_id: crate::HeadId::from_bytes(digest(22)),
                },
                WorldlineTick::ZERO,
                None,
                digest(23),
                digest(24),
                EchoOperationApplicationBasisV1::new(digest(25), digest(26)),
            ),
            digest(27),
            EchoOperationBudgetV1::new(8, 512, 512),
            node,
            digest(28),
            Vec::new(),
        );
        let replacement_value_digest = digest(29);
        let patch_digest = digest(30);
        let previous = digest(28);

        let mut expected = Hasher::new();
        expected.update(RESULT_ID_DOMAIN);
        expected.update(&installed.installed_operation_id.as_hash());
        expected.update(&profile_digest(RESULT_SCHEMA));
        expected.update(invocation.node.warp_id.as_bytes());
        expected.update(invocation.node.local_id.as_bytes());
        expected.update(&previous);
        expected.update(&replacement_value_digest);
        expected.update(&patch_digest);
        let expected_some = EchoOperationResultIdV1(expected.finalize().into());

        assert_eq!(
            operation_result_id(
                &installed,
                &invocation,
                AnchoredNodeOperationModeV1::CompareAndSet {
                    expected_value_digest: previous,
                },
                replacement_value_digest,
                patch_digest,
            ),
            expected_some,
            "compare-and-set must hash identically to the legacy untagged digest layout"
        );
    }

    #[test]
    fn legacy_compare_and_set_program_and_invocation_bytes_remain_fixed() {
        let required_node_type = TypeId(digest(31));
        let required_attachment_type = TypeId(digest(32));
        let program = EchoOperationProgramV1::anchored_node_attachment_compare_and_set(
            required_node_type,
            required_attachment_type,
            1_024,
        );
        let expected_program = encode_canonical_cbor_v1(&map_value([
            (
                "interpreter_profile_identity",
                hash_value(profile_digest("echo.operation-interpreter/v1")),
            ),
            (
                "intrinsic_profile_identity",
                hash_value(profile_digest("echo.operation-attachment-algebra/v1")),
            ),
            (
                "kind",
                text_value("anchored-node-attachment-compare-and-set/v1"),
            ),
            ("max_replacement_bytes", uint_value(1_024)),
            (
                "required_attachment_type",
                hash_value(required_attachment_type.0),
            ),
            ("required_node_type", hash_value(required_node_type.0)),
            ("schema", text_value("echo.operation-program/v1")),
        ]))
        .expect("the legacy program fixture encodes");
        assert_eq!(
            program.to_canonical_bytes().expect("program encodes"),
            expected_program,
            "the legacy update program bytes are a compatibility fixture"
        );
        assert_eq!(
            program.identity().expect("program has an identity"),
            EchoOperationProgramIdV1(domain_hash(
                b"echo:operation-program:v1\0",
                &expected_program,
            ))
        );

        let node = node_key_from_bytes([33; 64]);
        let basis = EchoOperationEvaluationBasisV1::new(
            WriterHeadKey {
                worldline_id: crate::WorldlineId::from_bytes(digest(34)),
                head_id: crate::HeadId::from_bytes(digest(35)),
            },
            WorldlineTick::from_raw(7),
            Some(GlobalTick::from_raw(11)),
            digest(36),
            digest(37),
            EchoOperationApplicationBasisV1::new(digest(38), digest(39)),
        );
        let invocation = EchoOperationInvocationV1::anchored_node_attachment_compare_and_set(
            EchoOperationPackageIdV1(digest(40)),
            "echo.fixture.LegacyCompareAndSet.v1",
            basis,
            digest(41),
            EchoOperationBudgetV1::new(4, 70, 37),
            node,
            digest(42),
            b"after".to_vec(),
        );
        let expected_invocation = encode_canonical_cbor_v1(&map_value([
            ("authority_grant_identity", hash_value(digest(41))),
            (
                "delegated_budget",
                EchoOperationBudgetV1::new(4, 70, 37).to_value(),
            ),
            ("evaluation_basis", basis.to_value()),
            ("expected_value_digest", hash_value(digest(42))),
            ("node_id", hash_value(node.local_id.0)),
            (
                "operation_coordinate",
                text_value("echo.fixture.LegacyCompareAndSet.v1"),
            ),
            ("package_id", hash_value(digest(40))),
            (
                "replacement_bytes",
                CanonicalValueV1::Bytes(b"after".to_vec()),
            ),
            ("schema", text_value("echo.operation-invocation/v1")),
            ("warp_id", hash_value(node.warp_id.0)),
        ]))
        .expect("the legacy invocation fixture encodes");
        assert_eq!(
            invocation.to_canonical_bytes().expect("invocation encodes"),
            expected_invocation,
            "the legacy update invocation bytes are a compatibility fixture"
        );

        let mut widened_value =
            decode_canonical_cbor_v1(&expected_invocation).expect("fixture decodes");
        let CanonicalValueV1::Map(fields) = &mut widened_value else {
            panic!("fixture is a map");
        };
        let expected_digest = fields
            .iter_mut()
            .find(|(key, _)| key == &text_value("expected_value_digest"))
            .expect("fixture carries the update precondition");
        expected_digest.1 = CanonicalValueV1::Null;
        let widened_bytes =
            encode_canonical_cbor_v1(&widened_value).expect("mutated fixture re-encodes");
        assert!(
            EchoOperationInvocationV1::from_canonical_bytes(&widened_bytes).is_err(),
            "null must never gain creation meaning under the legacy invocation schema"
        );
    }
}
