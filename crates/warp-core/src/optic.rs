// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Generic Echo optic nouns and deterministic identifiers.

use blake3::Hasher;
use echo_wasm_abi::kernel_port as abi;

use crate::attachment::{AttachmentKey, AttachmentOwner, AttachmentPlane};
use crate::clock::WorldlineTick;
use crate::ident::{EdgeKey, Hash, NodeKey, TypeId, WarpId};
use crate::materialization::ChannelId;
use crate::observation::{
    ReadingBudgetPosture, ReadingEnvelope, ReadingResidualPosture, ReadingRightsPosture,
    ReadingWitnessRef,
};
use crate::provenance_store::ProvenanceRef;
use crate::strand::StrandId;
use crate::worldline::WorldlineId;

const OPTIC_ID_DOMAIN: &[u8] = b"echo:optic-id:v1\0";
const FOCUS_DIGEST_DOMAIN: &[u8] = b"echo:optic-focus:v1\0";
const APERTURE_DIGEST_DOMAIN: &[u8] = b"echo:optic-aperture:v1\0";
const READ_IDENTITY_DOMAIN: &[u8] = b"echo:read-identity:v1\0";

macro_rules! opaque_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[repr(transparent)]
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        pub struct $name([u8; 32]);

        impl $name {
            /// Reconstructs the id from canonical bytes.
            #[must_use]
            pub const fn from_bytes(bytes: [u8; 32]) -> Self {
                Self(bytes)
            }

            /// Returns the canonical byte representation.
            #[must_use]
            pub const fn as_bytes(&self) -> &[u8; 32] {
                &self.0
            }
        }
    };
}

opaque_id!(
    /// Stable identity for an Echo optic descriptor.
    OpticId
);

opaque_id!(
    /// Stable identity for a generic braid.
    BraidId
);

opaque_id!(
    /// Stable key for a retained reading.
    RetainedReadingKey
);

opaque_id!(
    /// Stable identity for an intent family admitted through an optic.
    IntentFamilyId
);

opaque_id!(
    /// Stable identity for an optic capability basis.
    OpticCapabilityId
);

/// Version of the projection law used by an optic read.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct ProjectionVersion(u32);

impl ProjectionVersion {
    /// Builds a projection version from its raw value.
    #[must_use]
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    /// Returns the raw version value.
    #[must_use]
    pub const fn as_u32(self) -> u32 {
        self.0
    }

    fn to_abi(self) -> abi::ProjectionVersion {
        abi::ProjectionVersion(self.0)
    }
}

/// Version of the reducer law used by an optic read, when present.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct ReducerVersion(u32);

impl ReducerVersion {
    /// Builds a reducer version from its raw value.
    #[must_use]
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    /// Returns the raw version value.
    #[must_use]
    pub const fn as_u32(self) -> u32 {
        self.0
    }

    fn to_abi(self) -> abi::ReducerVersion {
        abi::ReducerVersion(self.0)
    }
}

/// Lawful subject named by an optic.
///
/// This is deliberately not a global graph handle. Each variant names a
/// focused substrate subject or boundary that can be observed under an explicit
/// coordinate and capability.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OpticFocus {
    /// A whole worldline.
    Worldline {
        /// Target worldline.
        worldline_id: WorldlineId,
    },
    /// A live or retained strand.
    Strand {
        /// Target strand.
        strand_id: StrandId,
    },
    /// A generic braid projection.
    Braid {
        /// Target braid.
        braid_id: BraidId,
    },
    /// A retained reading.
    RetainedReading {
        /// Retained reading key.
        key: RetainedReadingKey,
    },
    /// Explicit attachment boundary. Descending through it is a separate
    /// aperture/capability decision.
    AttachmentBoundary {
        /// Attachment boundary key.
        key: AttachmentKey,
    },
}

impl OpticFocus {
    /// Converts the focus to the shared ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::OpticFocus {
        match self {
            Self::Worldline { worldline_id } => abi::OpticFocus::Worldline {
                worldline_id: worldline_id_to_abi(*worldline_id),
            },
            Self::Strand { strand_id } => abi::OpticFocus::Strand {
                strand_id: strand_id_to_abi(*strand_id),
            },
            Self::Braid { braid_id } => abi::OpticFocus::Braid {
                braid_id: braid_id_to_abi(*braid_id),
            },
            Self::RetainedReading { key } => abi::OpticFocus::RetainedReading {
                key: retained_reading_key_to_abi(*key),
            },
            Self::AttachmentBoundary { key } => abi::OpticFocus::AttachmentBoundary {
                key: attachment_key_to_abi(*key),
            },
        }
    }

    /// Returns a stable digest of this focus for read-identity construction.
    #[must_use]
    pub fn digest(&self) -> Hash {
        let mut hasher = Hasher::new();
        hasher.update(FOCUS_DIGEST_DOMAIN);
        self.feed_hash(&mut hasher);
        hasher.finalize().into()
    }

    fn feed_hash(&self, hasher: &mut Hasher) {
        match self {
            Self::Worldline { worldline_id } => {
                feed_tag(hasher, 1);
                hasher.update(worldline_id.as_bytes());
            }
            Self::Strand { strand_id } => {
                feed_tag(hasher, 2);
                hasher.update(strand_id.as_bytes());
            }
            Self::Braid { braid_id } => {
                feed_tag(hasher, 3);
                hasher.update(braid_id.as_bytes());
            }
            Self::RetainedReading { key } => {
                feed_tag(hasher, 4);
                hasher.update(key.as_bytes());
            }
            Self::AttachmentBoundary { key } => {
                feed_tag(hasher, 5);
                feed_attachment_key(hasher, *key);
            }
        }
    }
}

/// Requested position within a substrate coordinate.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CoordinateAt {
    /// Observe or target the current frontier.
    Frontier,
    /// Observe or target a specific committed tick.
    Tick(WorldlineTick),
    /// Observe or target a full provenance coordinate.
    Provenance(ProvenanceRef),
}

impl CoordinateAt {
    fn to_abi(self) -> abi::CoordinateAt {
        match self {
            Self::Frontier => abi::CoordinateAt::Frontier,
            Self::Tick(worldline_tick) => abi::CoordinateAt::Tick {
                worldline_tick: worldline_tick_to_abi(worldline_tick),
            },
            Self::Provenance(reference) => abi::CoordinateAt::Provenance {
                reference: provenance_ref_to_abi(reference),
            },
        }
    }

    fn feed_hash(self, hasher: &mut Hasher) {
        match self {
            Self::Frontier => feed_tag(hasher, 1),
            Self::Tick(worldline_tick) => {
                feed_tag(hasher, 2);
                feed_u64(hasher, worldline_tick.as_u64());
            }
            Self::Provenance(reference) => {
                feed_tag(hasher, 3);
                feed_provenance_ref(hasher, reference);
            }
        }
    }
}

/// Causal coordinate named by an optic.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EchoCoordinate {
    /// Coordinate on a worldline.
    Worldline {
        /// Target worldline.
        worldline_id: WorldlineId,
        /// Requested position.
        at: CoordinateAt,
    },
    /// Coordinate on a strand.
    Strand {
        /// Target strand.
        strand_id: StrandId,
        /// Requested position.
        at: CoordinateAt,
        /// Optional parent basis that makes the strand read honest.
        parent_basis: Option<ProvenanceRef>,
    },
    /// Coordinate on a braid projection.
    Braid {
        /// Target braid.
        braid_id: BraidId,
        /// Projection digest at the named member frontier.
        projection_digest: Hash,
        /// Number of members included in the projection.
        member_count: u64,
    },
    /// Coordinate of a retained reading.
    RetainedReading {
        /// Retained reading key.
        key: RetainedReadingKey,
    },
}

impl EchoCoordinate {
    /// Converts the coordinate to the shared ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::EchoCoordinate {
        match self {
            Self::Worldline { worldline_id, at } => abi::EchoCoordinate::Worldline {
                worldline_id: worldline_id_to_abi(*worldline_id),
                at: at.to_abi(),
            },
            Self::Strand {
                strand_id,
                at,
                parent_basis,
            } => abi::EchoCoordinate::Strand {
                strand_id: strand_id_to_abi(*strand_id),
                at: at.to_abi(),
                parent_basis: parent_basis.map(provenance_ref_to_abi),
            },
            Self::Braid {
                braid_id,
                projection_digest,
                member_count,
            } => abi::EchoCoordinate::Braid {
                braid_id: braid_id_to_abi(*braid_id),
                projection_digest: projection_digest.to_vec(),
                member_count: *member_count,
            },
            Self::RetainedReading { key } => abi::EchoCoordinate::RetainedReading {
                key: retained_reading_key_to_abi(*key),
            },
        }
    }

    fn feed_hash(&self, hasher: &mut Hasher) {
        match self {
            Self::Worldline { worldline_id, at } => {
                feed_tag(hasher, 1);
                hasher.update(worldline_id.as_bytes());
                at.feed_hash(hasher);
            }
            Self::Strand {
                strand_id,
                at,
                parent_basis,
            } => {
                feed_tag(hasher, 2);
                hasher.update(strand_id.as_bytes());
                at.feed_hash(hasher);
                feed_optional_provenance_ref(hasher, *parent_basis);
            }
            Self::Braid {
                braid_id,
                projection_digest,
                member_count,
            } => {
                feed_tag(hasher, 3);
                hasher.update(braid_id.as_bytes());
                hasher.update(projection_digest);
                feed_u64(hasher, *member_count);
            }
            Self::RetainedReading { key } => {
                feed_tag(hasher, 4);
                hasher.update(key.as_bytes());
            }
        }
    }
}

/// Attachment recursion policy for an optic aperture.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AttachmentDescentPolicy {
    /// Stop at the attachment boundary and expose only the boundary reference.
    BoundaryOnly,
    /// Recursive descent was explicitly requested and remains budget/capability checked.
    Explicit,
}

impl AttachmentDescentPolicy {
    fn to_abi(self) -> abi::AttachmentDescentPolicy {
        match self {
            Self::BoundaryOnly => abi::AttachmentDescentPolicy::BoundaryOnly,
            Self::Explicit => abi::AttachmentDescentPolicy::Explicit,
        }
    }

    fn feed_hash(self, hasher: &mut Hasher) {
        match self {
            Self::BoundaryOnly => feed_tag(hasher, 1),
            Self::Explicit => feed_tag(hasher, 2),
        }
    }
}

/// Budget bound for an optic read.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OpticReadBudget {
    /// Maximum payload bytes to produce.
    pub max_bytes: Option<u64>,
    /// Maximum graph nodes or entities to visit.
    pub max_nodes: Option<u64>,
    /// Maximum causal ticks to reduce.
    pub max_ticks: Option<u64>,
    /// Maximum attachment boundaries to descend through.
    pub max_attachments: Option<u64>,
}

impl OpticReadBudget {
    fn to_abi(self) -> abi::OpticReadBudget {
        abi::OpticReadBudget {
            max_bytes: self.max_bytes,
            max_nodes: self.max_nodes,
            max_ticks: self.max_ticks,
            max_attachments: self.max_attachments,
        }
    }

    fn feed_hash(self, hasher: &mut Hasher) {
        feed_optional_u64(hasher, self.max_bytes);
        feed_optional_u64(hasher, self.max_nodes);
        feed_optional_u64(hasher, self.max_ticks);
        feed_optional_u64(hasher, self.max_attachments);
    }
}

/// Bounded aperture shape selected by an optic read.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OpticApertureShape {
    /// Head/frontier metadata only.
    Head,
    /// Snapshot metadata only.
    SnapshotMetadata,
    /// Recorded truth channels.
    TruthChannels {
        /// Optional channel filter. `None` means all recorded channels within budget.
        channels: Option<Vec<ChannelId>>,
    },
    /// Contract query bytes identified by query id and vars digest.
    QueryBytes {
        /// Stable query identifier.
        query_id: u32,
        /// Hash of canonical query variables.
        vars_digest: Hash,
    },
    /// Bounded byte range aperture.
    ByteRange {
        /// Start byte offset.
        start: u64,
        /// Maximum byte length to return.
        len: u64,
    },
    /// Explicit attachment boundary.
    AttachmentBoundary,
}

impl OpticApertureShape {
    fn to_abi(&self) -> abi::OpticApertureShape {
        match self {
            Self::Head => abi::OpticApertureShape::Head,
            Self::SnapshotMetadata => abi::OpticApertureShape::SnapshotMetadata,
            Self::TruthChannels { channels } => abi::OpticApertureShape::TruthChannels {
                channels: channels
                    .as_ref()
                    .map(|ids| ids.iter().map(channel_id_to_abi).collect()),
            },
            Self::QueryBytes {
                query_id,
                vars_digest,
            } => abi::OpticApertureShape::QueryBytes {
                query_id: *query_id,
                vars_digest: vars_digest.to_vec(),
            },
            Self::ByteRange { start, len } => abi::OpticApertureShape::ByteRange {
                start: *start,
                len: *len,
            },
            Self::AttachmentBoundary => abi::OpticApertureShape::AttachmentBoundary,
        }
    }

    fn feed_hash(&self, hasher: &mut Hasher) {
        match self {
            Self::Head => feed_tag(hasher, 1),
            Self::SnapshotMetadata => feed_tag(hasher, 2),
            Self::TruthChannels { channels } => {
                feed_tag(hasher, 3);
                feed_optional_hash_list(hasher, channels.as_deref());
            }
            Self::QueryBytes {
                query_id,
                vars_digest,
            } => {
                feed_tag(hasher, 4);
                feed_u32(hasher, *query_id);
                hasher.update(vars_digest);
            }
            Self::ByteRange { start, len } => {
                feed_tag(hasher, 5);
                feed_u64(hasher, *start);
                feed_u64(hasher, *len);
            }
            Self::AttachmentBoundary => feed_tag(hasher, 6),
        }
    }
}

/// Complete aperture for one optic read.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OpticAperture {
    /// Shape of the read aperture.
    pub shape: OpticApertureShape,
    /// Read budget.
    pub budget: OpticReadBudget,
    /// Attachment recursion policy.
    pub attachment_descent: AttachmentDescentPolicy,
}

impl OpticAperture {
    /// Converts the aperture to the shared ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::OpticAperture {
        abi::OpticAperture {
            shape: self.shape.to_abi(),
            budget: self.budget.to_abi(),
            attachment_descent: self.attachment_descent.to_abi(),
        }
    }

    /// Returns a stable digest of this aperture for read-identity construction.
    #[must_use]
    pub fn digest(&self) -> Hash {
        let mut hasher = Hasher::new();
        hasher.update(APERTURE_DIGEST_DOMAIN);
        self.feed_hash(&mut hasher);
        hasher.finalize().into()
    }

    fn feed_hash(&self, hasher: &mut Hasher) {
        self.shape.feed_hash(hasher);
        self.budget.feed_hash(hasher);
        self.attachment_descent.feed_hash(hasher);
    }
}

/// Opened optic descriptor. This is not a mutable handle.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EchoOptic {
    /// Stable optic identity derived from the descriptor.
    pub optic_id: OpticId,
    /// Lawful subject being observed or targeted by intent dispatch.
    pub focus: OpticFocus,
    /// Explicit causal coordinate.
    pub coordinate: EchoCoordinate,
    /// Projection law version.
    pub projection_version: ProjectionVersion,
    /// Reducer law version, if a reducer participates.
    pub reducer_version: Option<ReducerVersion>,
    /// Intent family allowed through this optic.
    pub intent_family: IntentFamilyId,
    /// Capability basis under which the optic was opened.
    pub capability: OpticCapabilityId,
}

impl EchoOptic {
    /// Builds a descriptor and derives its stable id from the generic optic fields.
    #[must_use]
    pub fn new(
        focus: OpticFocus,
        coordinate: EchoCoordinate,
        projection_version: ProjectionVersion,
        reducer_version: Option<ReducerVersion>,
        intent_family: IntentFamilyId,
        capability: OpticCapabilityId,
    ) -> Self {
        let optic_id = derive_optic_id(
            &focus,
            &coordinate,
            projection_version,
            reducer_version,
            intent_family,
            capability,
        );
        Self {
            optic_id,
            focus,
            coordinate,
            projection_version,
            reducer_version,
            intent_family,
            capability,
        }
    }

    /// Converts the descriptor to the shared ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::EchoOptic {
        abi::EchoOptic {
            optic_id: optic_id_to_abi(self.optic_id),
            focus: self.focus.to_abi(),
            coordinate: self.coordinate.to_abi(),
            projection_version: self.projection_version.to_abi(),
            reducer_version: self.reducer_version.map(ReducerVersion::to_abi),
            intent_family: intent_family_id_to_abi(self.intent_family),
            capability: optic_capability_id_to_abi(self.capability),
        }
    }
}

/// Reason an optic read identity cannot name a complete witness basis yet.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MissingWitnessBasisReason {
    /// Required witness evidence is unavailable.
    EvidenceUnavailable,
    /// The requested read exceeded its declared budget.
    BudgetLimited,
    /// The current capability does not permit revealing the basis.
    RightsLimited,
    /// The requested basis posture is not supported by this projection law.
    UnsupportedBasis,
}

impl MissingWitnessBasisReason {
    fn to_abi(self) -> abi::MissingWitnessBasisReason {
        match self {
            Self::EvidenceUnavailable => abi::MissingWitnessBasisReason::EvidenceUnavailable,
            Self::BudgetLimited => abi::MissingWitnessBasisReason::BudgetLimited,
            Self::RightsLimited => abi::MissingWitnessBasisReason::RightsLimited,
            Self::UnsupportedBasis => abi::MissingWitnessBasisReason::UnsupportedBasis,
        }
    }

    fn feed_hash(self, hasher: &mut Hasher) {
        match self {
            Self::EvidenceUnavailable => feed_tag(hasher, 1),
            Self::BudgetLimited => feed_tag(hasher, 2),
            Self::RightsLimited => feed_tag(hasher, 3),
            Self::UnsupportedBasis => feed_tag(hasher, 4),
        }
    }
}

/// Witness basis named by a read identity.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum WitnessBasis {
    /// One resolved provenance commit witnesses the reading.
    ResolvedCommit {
        /// Provenance coordinate that witnesses the reading.
        reference: ProvenanceRef,
        /// State root at the witness coordinate.
        state_root: Hash,
        /// Commit hash at the witness coordinate.
        commit_hash: Hash,
    },
    /// A checkpoint plus explicit live-tail witness set witnesses the reading.
    CheckpointPlusTail {
        /// Checkpoint coordinate used as the cold basis.
        checkpoint_ref: ProvenanceRef,
        /// Checkpoint content hash.
        checkpoint_hash: Hash,
        /// Live-tail provenance refs reduced after the checkpoint.
        tail_witness_refs: Vec<ProvenanceRef>,
        /// Digest of the live-tail witness set.
        tail_digest: Hash,
    },
    /// A witness set whose exact semantics are named by the contained refs and digest.
    WitnessSet {
        /// Witness refs supporting the read.
        refs: Vec<ReadingWitnessRef>,
        /// Digest over the witness set.
        witness_set_hash: Hash,
    },
    /// The basis is missing; callers must treat the read as obstructed or incomplete.
    Missing {
        /// Deterministic reason the basis is missing.
        reason: MissingWitnessBasisReason,
    },
}

impl WitnessBasis {
    /// Converts the witness basis to the shared ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::WitnessBasis {
        match self {
            Self::ResolvedCommit {
                reference,
                state_root,
                commit_hash,
            } => abi::WitnessBasis::ResolvedCommit {
                reference: provenance_ref_to_abi(*reference),
                state_root: state_root.to_vec(),
                commit_hash: commit_hash.to_vec(),
            },
            Self::CheckpointPlusTail {
                checkpoint_ref,
                checkpoint_hash,
                tail_witness_refs,
                tail_digest,
            } => abi::WitnessBasis::CheckpointPlusTail {
                checkpoint_ref: provenance_ref_to_abi(*checkpoint_ref),
                checkpoint_hash: checkpoint_hash.to_vec(),
                tail_witness_refs: tail_witness_refs
                    .iter()
                    .copied()
                    .map(provenance_ref_to_abi)
                    .collect(),
                tail_digest: tail_digest.to_vec(),
            },
            Self::WitnessSet {
                refs,
                witness_set_hash,
            } => abi::WitnessBasis::WitnessSet {
                refs: refs.iter().map(reading_witness_ref_to_abi).collect(),
                witness_set_hash: witness_set_hash.to_vec(),
            },
            Self::Missing { reason } => abi::WitnessBasis::Missing {
                reason: reason.to_abi(),
            },
        }
    }

    fn feed_hash(&self, hasher: &mut Hasher) {
        match self {
            Self::ResolvedCommit {
                reference,
                state_root,
                commit_hash,
            } => {
                feed_tag(hasher, 1);
                feed_provenance_ref(hasher, *reference);
                hasher.update(state_root);
                hasher.update(commit_hash);
            }
            Self::CheckpointPlusTail {
                checkpoint_ref,
                checkpoint_hash,
                tail_witness_refs,
                tail_digest,
            } => {
                feed_tag(hasher, 2);
                feed_provenance_ref(hasher, *checkpoint_ref);
                hasher.update(checkpoint_hash);
                feed_u64(hasher, tail_witness_refs.len() as u64);
                for reference in tail_witness_refs {
                    feed_provenance_ref(hasher, *reference);
                }
                hasher.update(tail_digest);
            }
            Self::WitnessSet {
                refs,
                witness_set_hash,
            } => {
                feed_tag(hasher, 3);
                feed_u64(hasher, refs.len() as u64);
                for reference in refs {
                    feed_reading_witness_ref(hasher, reference);
                }
                hasher.update(witness_set_hash);
            }
            Self::Missing { reason } => {
                feed_tag(hasher, 4);
                reason.feed_hash(hasher);
            }
        }
    }
}

/// Stable identity of the question an optic read answered.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ReadIdentity {
    /// Stable hash over all identity fields.
    pub read_identity_hash: Hash,
    /// Optic being observed.
    pub optic_id: OpticId,
    /// Digest of the focus named by the read.
    pub focus_digest: Hash,
    /// Coordinate named by the read.
    pub coordinate: EchoCoordinate,
    /// Digest of the aperture named by the read.
    pub aperture_digest: Hash,
    /// Projection law version.
    pub projection_version: ProjectionVersion,
    /// Reducer law version, if present.
    pub reducer_version: Option<ReducerVersion>,
    /// Witness basis used by the read.
    pub witness_basis: WitnessBasis,
    /// Rights posture of the emitted reading.
    pub rights_posture: ReadingRightsPosture,
    /// Budget posture of the emitted reading.
    pub budget_posture: ReadingBudgetPosture,
    /// Residual posture of the emitted reading.
    pub residual_posture: ReadingResidualPosture,
}

impl ReadIdentity {
    /// Builds a read identity from the full question and evidence posture.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        optic_id: OpticId,
        focus: &OpticFocus,
        coordinate: EchoCoordinate,
        aperture: &OpticAperture,
        projection_version: ProjectionVersion,
        reducer_version: Option<ReducerVersion>,
        witness_basis: WitnessBasis,
        rights_posture: ReadingRightsPosture,
        budget_posture: ReadingBudgetPosture,
        residual_posture: ReadingResidualPosture,
    ) -> Self {
        let focus_digest = focus.digest();
        let aperture_digest = aperture.digest();
        let read_identity_hash = derive_read_identity_hash(
            optic_id,
            &focus_digest,
            &coordinate,
            &aperture_digest,
            projection_version,
            reducer_version,
            &witness_basis,
            rights_posture,
            budget_posture,
            residual_posture,
        );
        Self {
            read_identity_hash,
            optic_id,
            focus_digest,
            coordinate,
            aperture_digest,
            projection_version,
            reducer_version,
            witness_basis,
            rights_posture,
            budget_posture,
            residual_posture,
        }
    }

    /// Builds a compatible identity using posture fields from an existing reading envelope.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn from_reading_envelope(
        optic_id: OpticId,
        focus: &OpticFocus,
        coordinate: EchoCoordinate,
        aperture: &OpticAperture,
        projection_version: ProjectionVersion,
        reducer_version: Option<ReducerVersion>,
        witness_basis: WitnessBasis,
        reading: &ReadingEnvelope,
    ) -> Self {
        Self::new(
            optic_id,
            focus,
            coordinate,
            aperture,
            projection_version,
            reducer_version,
            witness_basis,
            reading.rights_posture,
            reading.budget_posture,
            reading.residual_posture,
        )
    }

    /// Converts the read identity to the shared ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::ReadIdentity {
        abi::ReadIdentity {
            read_identity_hash: self.read_identity_hash.to_vec(),
            optic_id: optic_id_to_abi(self.optic_id),
            focus_digest: self.focus_digest.to_vec(),
            coordinate: self.coordinate.to_abi(),
            aperture_digest: self.aperture_digest.to_vec(),
            projection_version: self.projection_version.to_abi(),
            reducer_version: self.reducer_version.map(ReducerVersion::to_abi),
            witness_basis: self.witness_basis.to_abi(),
            rights_posture: reading_rights_posture_to_abi(self.rights_posture),
            budget_posture: reading_budget_posture_to_abi(self.budget_posture),
            residual_posture: reading_residual_posture_to_abi(self.residual_posture),
        }
    }
}

/// Reading envelope plus first-class optic read identity.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OpticReadingEnvelope {
    /// Existing observation reading envelope.
    pub reading: ReadingEnvelope,
    /// Stable read identity for the question this reading answered.
    pub read_identity: ReadIdentity,
}

impl OpticReadingEnvelope {
    /// Builds an optic reading envelope from an existing reading envelope and identity.
    #[must_use]
    pub fn new(reading: ReadingEnvelope, read_identity: ReadIdentity) -> Self {
        Self {
            reading,
            read_identity,
        }
    }

    /// Converts the envelope to the shared ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::OpticReadingEnvelope {
        abi::OpticReadingEnvelope {
            reading: self.reading.to_abi(),
            read_identity: self.read_identity.to_abi(),
        }
    }
}

/// Deterministic reason an optic read or dispatch could not lawfully proceed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OpticObstructionKind {
    /// Required witness evidence is unavailable.
    MissingWitness,
    /// A retained reading named by the optic cannot be found or revealed.
    MissingRetainedReading,
    /// The dispatch named a base coordinate that is no longer the admitted basis.
    StaleBasis,
    /// The capability basis does not authorize the requested read or dispatch.
    CapabilityDenied,
    /// The declared read or dispatch budget was exceeded.
    BudgetExceeded,
    /// The requested aperture is not supported by this optic or projection law.
    UnsupportedAperture,
    /// The requested projection law/version is not available.
    UnsupportedProjectionLaw,
    /// The requested intent family is not available through this optic.
    UnsupportedIntentFamily,
    /// The read reached an attachment boundary and explicit descent is required.
    AttachmentDescentRequired,
    /// The requested attachment descent is not authorized.
    AttachmentDescentDenied,
    /// A live-tail read requires additional bounded reduction before it is honest.
    LiveTailRequiresReduction,
    /// The requested coordinate names an incompatible frontier.
    ConflictingFrontier,
    /// The request would collapse plurality without an explicit policy.
    PluralityRequiresExplicitPolicy,
}

impl OpticObstructionKind {
    fn to_abi(self) -> abi::OpticObstructionKind {
        match self {
            Self::MissingWitness => abi::OpticObstructionKind::MissingWitness,
            Self::MissingRetainedReading => abi::OpticObstructionKind::MissingRetainedReading,
            Self::StaleBasis => abi::OpticObstructionKind::StaleBasis,
            Self::CapabilityDenied => abi::OpticObstructionKind::CapabilityDenied,
            Self::BudgetExceeded => abi::OpticObstructionKind::BudgetExceeded,
            Self::UnsupportedAperture => abi::OpticObstructionKind::UnsupportedAperture,
            Self::UnsupportedProjectionLaw => abi::OpticObstructionKind::UnsupportedProjectionLaw,
            Self::UnsupportedIntentFamily => abi::OpticObstructionKind::UnsupportedIntentFamily,
            Self::AttachmentDescentRequired => abi::OpticObstructionKind::AttachmentDescentRequired,
            Self::AttachmentDescentDenied => abi::OpticObstructionKind::AttachmentDescentDenied,
            Self::LiveTailRequiresReduction => abi::OpticObstructionKind::LiveTailRequiresReduction,
            Self::ConflictingFrontier => abi::OpticObstructionKind::ConflictingFrontier,
            Self::PluralityRequiresExplicitPolicy => {
                abi::OpticObstructionKind::PluralityRequiresExplicitPolicy
            }
        }
    }
}

/// Typed obstruction returned instead of a hidden fallback or fake success.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OpticObstruction {
    /// Deterministic obstruction kind.
    pub kind: OpticObstructionKind,
    /// Optic implicated by the obstruction, when known.
    pub optic_id: Option<OpticId>,
    /// Focus implicated by the obstruction, when known.
    pub focus: Option<OpticFocus>,
    /// Coordinate implicated by the obstruction, when known.
    pub coordinate: Option<EchoCoordinate>,
    /// Witness basis posture that explains evidence availability, when known.
    pub witness_basis: Option<WitnessBasis>,
    /// Human-readable diagnostic text.
    pub message: String,
}

impl OpticObstruction {
    /// Converts the obstruction to the shared ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::OpticObstruction {
        abi::OpticObstruction {
            kind: self.kind.to_abi(),
            optic_id: self.optic_id.map(optic_id_to_abi),
            focus: self.focus.as_ref().map(OpticFocus::to_abi),
            coordinate: self.coordinate.as_ref().map(EchoCoordinate::to_abi),
            witness_basis: self.witness_basis.as_ref().map(WitnessBasis::to_abi),
            message: self.message.clone(),
        }
    }
}

/// Admission result for an optic intent that Echo accepted into witnessed history.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AdmittedIntent {
    /// Optic through which the intent was dispatched.
    pub optic_id: OpticId,
    /// Explicit causal basis named by the dispatch.
    pub base_coordinate: EchoCoordinate,
    /// Intent family admitted through the optic.
    pub intent_family: IntentFamilyId,
    /// Provenance coordinate produced or identified by admission.
    pub admitted_ref: ProvenanceRef,
    /// Receipt digest witnessing the admission outcome.
    pub receipt_hash: Hash,
}

impl AdmittedIntent {
    /// Converts the admitted outcome to the shared ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::AdmittedIntent {
        abi::AdmittedIntent {
            optic_id: optic_id_to_abi(self.optic_id),
            base_coordinate: self.base_coordinate.to_abi(),
            intent_family: intent_family_id_to_abi(self.intent_family),
            admitted_ref: provenance_ref_to_abi(self.admitted_ref),
            receipt_hash: self.receipt_hash.to_vec(),
        }
    }
}

/// Reason an optic intent is staged instead of admitted immediately.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StagedIntentReason {
    /// The proposal needs an explicit rebase before admission can proceed.
    RebaseRequired,
    /// The proposal is waiting for additional capability evidence.
    AwaitingCapability,
    /// The proposal is waiting for additional witness evidence.
    AwaitingWitness,
    /// The proposal was deliberately staged for later explicit admission.
    AwaitingExplicitAdmission,
}

impl StagedIntentReason {
    fn to_abi(self) -> abi::StagedIntentReason {
        match self {
            Self::RebaseRequired => abi::StagedIntentReason::RebaseRequired,
            Self::AwaitingCapability => abi::StagedIntentReason::AwaitingCapability,
            Self::AwaitingWitness => abi::StagedIntentReason::AwaitingWitness,
            Self::AwaitingExplicitAdmission => abi::StagedIntentReason::AwaitingExplicitAdmission,
        }
    }
}

/// Admission result for an optic intent retained without mutating the frontier.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StagedIntent {
    /// Optic through which the intent was dispatched.
    pub optic_id: OpticId,
    /// Explicit causal basis named by the dispatch.
    pub base_coordinate: EchoCoordinate,
    /// Intent family proposed through the optic.
    pub intent_family: IntentFamilyId,
    /// Stable digest or storage key for the staged proposal.
    pub stage_ref: Hash,
    /// Deterministic reason the proposal is staged.
    pub reason: StagedIntentReason,
}

impl StagedIntent {
    /// Converts the staged outcome to the shared ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::StagedIntent {
        abi::StagedIntent {
            optic_id: optic_id_to_abi(self.optic_id),
            base_coordinate: self.base_coordinate.to_abi(),
            intent_family: intent_family_id_to_abi(self.intent_family),
            stage_ref: self.stage_ref.to_vec(),
            reason: self.reason.to_abi(),
        }
    }
}

/// Admission result that preserves lawful plurality instead of selecting one winner.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PluralIntent {
    /// Optic through which the intent was dispatched.
    pub optic_id: OpticId,
    /// Explicit causal basis named by the dispatch.
    pub base_coordinate: EchoCoordinate,
    /// Intent family proposed through the optic.
    pub intent_family: IntentFamilyId,
    /// Candidate coordinates that remain lawful plural outcomes.
    pub candidate_refs: Vec<ProvenanceRef>,
    /// Residual posture associated with the preserved plurality.
    pub residual_posture: ReadingResidualPosture,
}

impl PluralIntent {
    /// Converts the plural outcome to the shared ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::PluralIntent {
        abi::PluralIntent {
            optic_id: optic_id_to_abi(self.optic_id),
            base_coordinate: self.base_coordinate.to_abi(),
            intent_family: intent_family_id_to_abi(self.intent_family),
            candidate_refs: self
                .candidate_refs
                .iter()
                .copied()
                .map(provenance_ref_to_abi)
                .collect(),
            residual_posture: reading_residual_posture_to_abi(self.residual_posture),
        }
    }
}

/// Deterministic conflict reason for an optic intent dispatch.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum IntentConflictReason {
    /// The named base coordinate is no longer the applicable basis.
    StaleBasis,
    /// The request conflicts with the named or observed frontier.
    ConflictingFrontier,
    /// Capability evidence conflicts with the requested operation.
    CapabilityConflict,
    /// The verified footprint conflicts with concurrent causal claims.
    FootprintConflict,
    /// The requested admission law conflicts with the available host law.
    AdmissionLawConflict,
    /// The request needs an explicit plurality policy before admission.
    UnsupportedPluralityPolicy,
}

impl IntentConflictReason {
    fn to_abi(self) -> abi::IntentConflictReason {
        match self {
            Self::StaleBasis => abi::IntentConflictReason::StaleBasis,
            Self::ConflictingFrontier => abi::IntentConflictReason::ConflictingFrontier,
            Self::CapabilityConflict => abi::IntentConflictReason::CapabilityConflict,
            Self::FootprintConflict => abi::IntentConflictReason::FootprintConflict,
            Self::AdmissionLawConflict => abi::IntentConflictReason::AdmissionLawConflict,
            Self::UnsupportedPluralityPolicy => {
                abi::IntentConflictReason::UnsupportedPluralityPolicy
            }
        }
    }
}

/// Admission result for incompatible causal claims under an optic dispatch.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IntentConflict {
    /// Optic through which the intent was dispatched.
    pub optic_id: OpticId,
    /// Explicit causal basis named by the dispatch.
    pub base_coordinate: EchoCoordinate,
    /// Intent family proposed through the optic.
    pub intent_family: IntentFamilyId,
    /// Deterministic conflict reason.
    pub reason: IntentConflictReason,
    /// Provenance coordinate implicated by the conflict, when known.
    pub conflict_ref: Option<ProvenanceRef>,
    /// Digest of compact conflict evidence.
    pub evidence_digest: Hash,
    /// Human-readable diagnostic text.
    pub message: String,
}

impl IntentConflict {
    /// Converts the conflict outcome to the shared ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::IntentConflict {
        abi::IntentConflict {
            optic_id: optic_id_to_abi(self.optic_id),
            base_coordinate: self.base_coordinate.to_abi(),
            intent_family: intent_family_id_to_abi(self.intent_family),
            reason: self.reason.to_abi(),
            conflict_ref: self.conflict_ref.map(provenance_ref_to_abi),
            evidence_digest: self.evidence_digest.to_vec(),
            message: self.message.clone(),
        }
    }
}

/// Typed top-level result for dispatching an intent through an optic.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum IntentDispatchResult {
    /// Echo accepted the intent into witnessed history.
    Admitted(AdmittedIntent),
    /// Echo retained the proposal without mutating the named frontier.
    Staged(StagedIntent),
    /// Echo preserved lawful plurality instead of selecting a single result.
    Plural(PluralIntent),
    /// Echo found incompatible causal claims under the named admission law.
    Conflict(IntentConflict),
    /// Echo could not lawfully proceed because basis, evidence, rights, or law is missing.
    Obstructed(OpticObstruction),
}

impl IntentDispatchResult {
    /// Converts the dispatch result to the shared ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::IntentDispatchResult {
        match self {
            Self::Admitted(outcome) => abi::IntentDispatchResult::Admitted(outcome.to_abi()),
            Self::Staged(outcome) => abi::IntentDispatchResult::Staged(outcome.to_abi()),
            Self::Plural(outcome) => abi::IntentDispatchResult::Plural(outcome.to_abi()),
            Self::Conflict(outcome) => abi::IntentDispatchResult::Conflict(outcome.to_abi()),
            Self::Obstructed(obstruction) => {
                abi::IntentDispatchResult::Obstructed(obstruction.to_abi())
            }
        }
    }
}

fn derive_optic_id(
    focus: &OpticFocus,
    coordinate: &EchoCoordinate,
    projection_version: ProjectionVersion,
    reducer_version: Option<ReducerVersion>,
    intent_family: IntentFamilyId,
    capability: OpticCapabilityId,
) -> OpticId {
    let mut hasher = Hasher::new();
    hasher.update(OPTIC_ID_DOMAIN);
    focus.feed_hash(&mut hasher);
    coordinate.feed_hash(&mut hasher);
    feed_u32(&mut hasher, projection_version.as_u32());
    match reducer_version {
        Some(version) => {
            feed_tag(&mut hasher, 1);
            feed_u32(&mut hasher, version.as_u32());
        }
        None => feed_tag(&mut hasher, 0),
    }
    hasher.update(intent_family.as_bytes());
    hasher.update(capability.as_bytes());
    OpticId::from_bytes(hasher.finalize().into())
}

#[allow(clippy::too_many_arguments)]
fn derive_read_identity_hash(
    optic_id: OpticId,
    focus_digest: &Hash,
    coordinate: &EchoCoordinate,
    aperture_digest: &Hash,
    projection_version: ProjectionVersion,
    reducer_version: Option<ReducerVersion>,
    witness_basis: &WitnessBasis,
    rights_posture: ReadingRightsPosture,
    budget_posture: ReadingBudgetPosture,
    residual_posture: ReadingResidualPosture,
) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(READ_IDENTITY_DOMAIN);
    hasher.update(optic_id.as_bytes());
    hasher.update(focus_digest);
    coordinate.feed_hash(&mut hasher);
    hasher.update(aperture_digest);
    feed_u32(&mut hasher, projection_version.as_u32());
    match reducer_version {
        Some(version) => {
            feed_tag(&mut hasher, 1);
            feed_u32(&mut hasher, version.as_u32());
        }
        None => feed_tag(&mut hasher, 0),
    }
    witness_basis.feed_hash(&mut hasher);
    feed_reading_rights_posture(&mut hasher, rights_posture);
    feed_reading_budget_posture(&mut hasher, budget_posture);
    feed_reading_residual_posture(&mut hasher, residual_posture);
    hasher.finalize().into()
}

fn feed_tag(hasher: &mut Hasher, tag: u8) {
    hasher.update(&[tag]);
}

fn feed_u32(hasher: &mut Hasher, value: u32) {
    hasher.update(&value.to_le_bytes());
}

fn feed_u64(hasher: &mut Hasher, value: u64) {
    hasher.update(&value.to_le_bytes());
}

fn feed_optional_u64(hasher: &mut Hasher, value: Option<u64>) {
    match value {
        Some(value) => {
            feed_tag(hasher, 1);
            feed_u64(hasher, value);
        }
        None => feed_tag(hasher, 0),
    }
}

fn feed_optional_hash_list(hasher: &mut Hasher, hashes: Option<&[TypeId]>) {
    match hashes {
        Some(hashes) => {
            feed_tag(hasher, 1);
            feed_u64(hasher, hashes.len() as u64);
            for hash in hashes {
                hasher.update(hash.as_bytes());
            }
        }
        None => feed_tag(hasher, 0),
    }
}

fn feed_optional_provenance_ref(hasher: &mut Hasher, reference: Option<ProvenanceRef>) {
    match reference {
        Some(reference) => {
            feed_tag(hasher, 1);
            feed_provenance_ref(hasher, reference);
        }
        None => feed_tag(hasher, 0),
    }
}

fn feed_provenance_ref(hasher: &mut Hasher, reference: ProvenanceRef) {
    hasher.update(reference.worldline_id.as_bytes());
    feed_u64(hasher, reference.worldline_tick.as_u64());
    hasher.update(&reference.commit_hash);
}

fn feed_reading_witness_ref(hasher: &mut Hasher, reference: &ReadingWitnessRef) {
    match reference {
        ReadingWitnessRef::ResolvedCommit { reference } => {
            feed_tag(hasher, 1);
            feed_provenance_ref(hasher, *reference);
        }
        ReadingWitnessRef::EmptyFrontier {
            worldline_id,
            state_root,
            commit_hash,
        } => {
            feed_tag(hasher, 2);
            hasher.update(worldline_id.as_bytes());
            hasher.update(state_root);
            hasher.update(commit_hash);
        }
    }
}

fn feed_reading_budget_posture(hasher: &mut Hasher, posture: ReadingBudgetPosture) {
    match posture {
        ReadingBudgetPosture::UnboundedOneShot => feed_tag(hasher, 1),
    }
}

fn feed_reading_rights_posture(hasher: &mut Hasher, posture: ReadingRightsPosture) {
    match posture {
        ReadingRightsPosture::KernelPublic => feed_tag(hasher, 1),
    }
}

fn feed_reading_residual_posture(hasher: &mut Hasher, posture: ReadingResidualPosture) {
    match posture {
        ReadingResidualPosture::Complete => feed_tag(hasher, 1),
        ReadingResidualPosture::Residual => feed_tag(hasher, 2),
        ReadingResidualPosture::PluralityPreserved => feed_tag(hasher, 3),
        ReadingResidualPosture::Obstructed => feed_tag(hasher, 4),
    }
}

fn feed_attachment_key(hasher: &mut Hasher, key: AttachmentKey) {
    match key.owner {
        AttachmentOwner::Node(NodeKey { warp_id, local_id }) => {
            feed_tag(hasher, 1);
            hasher.update(warp_id.as_bytes());
            hasher.update(local_id.as_bytes());
        }
        AttachmentOwner::Edge(EdgeKey { warp_id, local_id }) => {
            feed_tag(hasher, 2);
            hasher.update(warp_id.as_bytes());
            hasher.update(local_id.as_bytes());
        }
    }
    match key.plane {
        AttachmentPlane::Alpha => feed_tag(hasher, 1),
        AttachmentPlane::Beta => feed_tag(hasher, 2),
    }
}

fn optic_id_to_abi(id: OpticId) -> abi::OpticId {
    abi::OpticId::from_bytes(*id.as_bytes())
}

fn braid_id_to_abi(id: BraidId) -> abi::BraidId {
    abi::BraidId::from_bytes(*id.as_bytes())
}

fn retained_reading_key_to_abi(key: RetainedReadingKey) -> abi::RetainedReadingKey {
    abi::RetainedReadingKey::from_bytes(*key.as_bytes())
}

fn intent_family_id_to_abi(id: IntentFamilyId) -> abi::IntentFamilyId {
    abi::IntentFamilyId::from_bytes(*id.as_bytes())
}

fn optic_capability_id_to_abi(id: OpticCapabilityId) -> abi::OpticCapabilityId {
    abi::OpticCapabilityId::from_bytes(*id.as_bytes())
}

fn worldline_id_to_abi(worldline_id: WorldlineId) -> abi::WorldlineId {
    abi::WorldlineId::from_bytes(*worldline_id.as_bytes())
}

fn strand_id_to_abi(strand_id: StrandId) -> abi::StrandId {
    abi::StrandId::from_bytes(*strand_id.as_bytes())
}

fn worldline_tick_to_abi(worldline_tick: WorldlineTick) -> abi::WorldlineTick {
    abi::WorldlineTick(worldline_tick.as_u64())
}

fn provenance_ref_to_abi(reference: ProvenanceRef) -> abi::ProvenanceRef {
    abi::ProvenanceRef {
        worldline_id: worldline_id_to_abi(reference.worldline_id),
        worldline_tick: worldline_tick_to_abi(reference.worldline_tick),
        commit_hash: reference.commit_hash.to_vec(),
    }
}

fn reading_witness_ref_to_abi(reference: &ReadingWitnessRef) -> abi::ReadingWitnessRef {
    match reference {
        ReadingWitnessRef::ResolvedCommit { reference } => abi::ReadingWitnessRef::ResolvedCommit {
            reference: provenance_ref_to_abi(*reference),
        },
        ReadingWitnessRef::EmptyFrontier {
            worldline_id,
            state_root,
            commit_hash,
        } => abi::ReadingWitnessRef::EmptyFrontier {
            worldline_id: worldline_id_to_abi(*worldline_id),
            state_root: state_root.to_vec(),
            commit_hash: commit_hash.to_vec(),
        },
    }
}

fn reading_budget_posture_to_abi(posture: ReadingBudgetPosture) -> abi::ReadingBudgetPosture {
    match posture {
        ReadingBudgetPosture::UnboundedOneShot => abi::ReadingBudgetPosture::UnboundedOneShot,
    }
}

fn reading_rights_posture_to_abi(posture: ReadingRightsPosture) -> abi::ReadingRightsPosture {
    match posture {
        ReadingRightsPosture::KernelPublic => abi::ReadingRightsPosture::KernelPublic,
    }
}

fn reading_residual_posture_to_abi(posture: ReadingResidualPosture) -> abi::ReadingResidualPosture {
    match posture {
        ReadingResidualPosture::Complete => abi::ReadingResidualPosture::Complete,
        ReadingResidualPosture::Residual => abi::ReadingResidualPosture::Residual,
        ReadingResidualPosture::PluralityPreserved => {
            abi::ReadingResidualPosture::PluralityPreserved
        }
        ReadingResidualPosture::Obstructed => abi::ReadingResidualPosture::Obstructed,
    }
}

fn warp_id_to_abi(warp_id: WarpId) -> abi::WarpId {
    abi::WarpId::from_bytes(*warp_id.as_bytes())
}

fn node_key_to_abi(key: NodeKey) -> abi::AttachmentOwnerRef {
    abi::AttachmentOwnerRef::Node {
        warp_id: warp_id_to_abi(key.warp_id),
        node_id: abi::NodeId::from_bytes(*key.local_id.as_bytes()),
    }
}

fn edge_key_to_abi(key: EdgeKey) -> abi::AttachmentOwnerRef {
    abi::AttachmentOwnerRef::Edge {
        warp_id: warp_id_to_abi(key.warp_id),
        edge_id: abi::EdgeId::from_bytes(*key.local_id.as_bytes()),
    }
}

fn attachment_key_to_abi(key: AttachmentKey) -> abi::AttachmentKey {
    let owner = match key.owner {
        AttachmentOwner::Node(node) => node_key_to_abi(node),
        AttachmentOwner::Edge(edge) => edge_key_to_abi(edge),
    };
    let plane = match key.plane {
        AttachmentPlane::Alpha => abi::AttachmentPlane::Alpha,
        AttachmentPlane::Beta => abi::AttachmentPlane::Beta,
    };
    abi::AttachmentKey { owner, plane }
}

fn channel_id_to_abi(channel_id: &ChannelId) -> abi::ChannelId {
    abi::ChannelId::from_bytes(*channel_id.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attachment::{AttachmentKey, AttachmentOwner, AttachmentPlane};
    use crate::ident::{EdgeId, EdgeKey, NodeId, NodeKey, TypeId, WarpId};
    use crate::observation::{
        BuiltinObserverPlan, ObservationBasisPosture, ReadingBudgetPosture, ReadingObserverBasis,
        ReadingObserverPlan, ReadingResidualPosture, ReadingRightsPosture, ReadingWitnessRef,
    };
    use crate::provenance_store::ProvenanceRef;
    use crate::strand::StrandId;
    use crate::worldline::WorldlineId;

    fn worldline(seed: u8) -> WorldlineId {
        WorldlineId::from_bytes([seed; 32])
    }

    fn strand(seed: u8) -> StrandId {
        StrandId::from_bytes([seed; 32])
    }

    fn braid(seed: u8) -> BraidId {
        BraidId::from_bytes([seed; 32])
    }

    fn retained(seed: u8) -> RetainedReadingKey {
        RetainedReadingKey::from_bytes([seed; 32])
    }

    fn intent_family(seed: u8) -> IntentFamilyId {
        IntentFamilyId::from_bytes([seed; 32])
    }

    fn capability(seed: u8) -> OpticCapabilityId {
        OpticCapabilityId::from_bytes([seed; 32])
    }

    fn node_key(seed: u8) -> NodeKey {
        NodeKey {
            warp_id: WarpId([seed; 32]),
            local_id: NodeId([seed.wrapping_add(1); 32]),
        }
    }

    fn edge_key(seed: u8) -> EdgeKey {
        EdgeKey {
            warp_id: WarpId([seed; 32]),
            local_id: EdgeId([seed.wrapping_add(1); 32]),
        }
    }

    fn provenance(seed: u8, tick: u64) -> ProvenanceRef {
        ProvenanceRef {
            worldline_id: worldline(seed),
            worldline_tick: crate::clock::WorldlineTick::from_raw(tick),
            commit_hash: [seed.wrapping_add(1); 32],
        }
    }

    fn worldline_focus() -> OpticFocus {
        OpticFocus::Worldline {
            worldline_id: worldline(1),
        }
    }

    fn frontier_coordinate() -> EchoCoordinate {
        EchoCoordinate::Worldline {
            worldline_id: worldline(1),
            at: CoordinateAt::Frontier,
        }
    }

    fn head_aperture() -> OpticAperture {
        OpticAperture {
            shape: OpticApertureShape::Head,
            budget: OpticReadBudget {
                max_bytes: Some(512),
                max_nodes: Some(8),
                max_ticks: Some(1),
                max_attachments: Some(0),
            },
            attachment_descent: AttachmentDescentPolicy::BoundaryOnly,
        }
    }

    fn witness_basis(seed: u8, tick: u64) -> WitnessBasis {
        let reference = provenance(seed, tick);
        WitnessBasis::ResolvedCommit {
            reference,
            state_root: [seed.wrapping_add(2); 32],
            commit_hash: reference.commit_hash,
        }
    }

    fn reading_envelope() -> ReadingEnvelope {
        ReadingEnvelope {
            observer_plan: ReadingObserverPlan::Builtin {
                plan: BuiltinObserverPlan::CommitBoundaryHead,
            },
            observer_basis: ReadingObserverBasis::CommitBoundary,
            witness_refs: vec![ReadingWitnessRef::ResolvedCommit {
                reference: provenance(1, 2),
            }],
            parent_basis_posture: ObservationBasisPosture::Worldline,
            budget_posture: ReadingBudgetPosture::UnboundedOneShot,
            rights_posture: ReadingRightsPosture::KernelPublic,
            residual_posture: ReadingResidualPosture::Complete,
        }
    }

    #[test]
    fn echo_optic_id_is_stable_and_descriptor_derived() {
        let focus = OpticFocus::Worldline {
            worldline_id: worldline(1),
        };
        let coordinate = EchoCoordinate::Worldline {
            worldline_id: worldline(1),
            at: CoordinateAt::Frontier,
        };

        let first = EchoOptic::new(
            focus.clone(),
            coordinate.clone(),
            ProjectionVersion::from_raw(1),
            Some(ReducerVersion::from_raw(7)),
            intent_family(4),
            capability(5),
        );
        let second = EchoOptic::new(
            focus,
            coordinate,
            ProjectionVersion::from_raw(1),
            Some(ReducerVersion::from_raw(7)),
            intent_family(4),
            capability(5),
        );

        assert_eq!(first.optic_id, second.optic_id);

        let changed_projection = EchoOptic::new(
            OpticFocus::Worldline {
                worldline_id: worldline(1),
            },
            EchoCoordinate::Worldline {
                worldline_id: worldline(1),
                at: CoordinateAt::Frontier,
            },
            ProjectionVersion::from_raw(2),
            Some(ReducerVersion::from_raw(7)),
            intent_family(4),
            capability(5),
        );

        assert_ne!(first.optic_id, changed_projection.optic_id);
    }

    #[test]
    fn optic_focus_covers_generic_subjects_without_graph_handle() {
        let focuses = vec![
            OpticFocus::Worldline {
                worldline_id: worldline(1),
            },
            OpticFocus::Strand {
                strand_id: strand(2),
            },
            OpticFocus::Braid { braid_id: braid(3) },
            OpticFocus::RetainedReading { key: retained(4) },
            OpticFocus::AttachmentBoundary {
                key: AttachmentKey {
                    owner: AttachmentOwner::Node(node_key(5)),
                    plane: AttachmentPlane::Alpha,
                },
            },
            OpticFocus::AttachmentBoundary {
                key: AttachmentKey {
                    owner: AttachmentOwner::Edge(edge_key(6)),
                    plane: AttachmentPlane::Beta,
                },
            },
        ];

        for focus in focuses {
            let encoded = focus.to_abi();
            assert!(matches!(
                encoded,
                echo_wasm_abi::kernel_port::OpticFocus::Worldline { .. }
                    | echo_wasm_abi::kernel_port::OpticFocus::Strand { .. }
                    | echo_wasm_abi::kernel_port::OpticFocus::Braid { .. }
                    | echo_wasm_abi::kernel_port::OpticFocus::RetainedReading { .. }
                    | echo_wasm_abi::kernel_port::OpticFocus::AttachmentBoundary { .. }
            ));
        }
    }

    #[test]
    fn strand_coordinate_names_explicit_parent_basis_in_abi() {
        let parent_basis = ProvenanceRef {
            worldline_id: worldline(9),
            worldline_tick: crate::clock::WorldlineTick::from_raw(11),
            commit_hash: [12; 32],
        };
        let coordinate = EchoCoordinate::Strand {
            strand_id: strand(2),
            at: CoordinateAt::Provenance(parent_basis),
            parent_basis: Some(parent_basis),
        };

        assert_eq!(
            coordinate.to_abi(),
            echo_wasm_abi::kernel_port::EchoCoordinate::Strand {
                strand_id: echo_wasm_abi::kernel_port::StrandId::from_bytes([2; 32]),
                at: echo_wasm_abi::kernel_port::CoordinateAt::Provenance {
                    reference: echo_wasm_abi::kernel_port::ProvenanceRef {
                        worldline_id: echo_wasm_abi::kernel_port::WorldlineId::from_bytes([9; 32]),
                        worldline_tick: echo_wasm_abi::kernel_port::WorldlineTick(11),
                        commit_hash: vec![12; 32],
                    },
                },
                parent_basis: Some(echo_wasm_abi::kernel_port::ProvenanceRef {
                    worldline_id: echo_wasm_abi::kernel_port::WorldlineId::from_bytes([9; 32]),
                    worldline_tick: echo_wasm_abi::kernel_port::WorldlineTick(11),
                    commit_hash: vec![12; 32],
                }),
            }
        );
    }

    #[test]
    fn optic_aperture_encodes_bounds_without_full_materialization_fallback() {
        let aperture = OpticAperture {
            shape: OpticApertureShape::QueryBytes {
                query_id: 42,
                vars_digest: [7; 32],
            },
            budget: OpticReadBudget {
                max_bytes: Some(4096),
                max_nodes: Some(128),
                max_ticks: Some(12),
                max_attachments: Some(0),
            },
            attachment_descent: AttachmentDescentPolicy::BoundaryOnly,
        };

        assert_eq!(
            aperture.to_abi(),
            echo_wasm_abi::kernel_port::OpticAperture {
                shape: echo_wasm_abi::kernel_port::OpticApertureShape::QueryBytes {
                    query_id: 42,
                    vars_digest: vec![7; 32],
                },
                budget: echo_wasm_abi::kernel_port::OpticReadBudget {
                    max_bytes: Some(4096),
                    max_nodes: Some(128),
                    max_ticks: Some(12),
                    max_attachments: Some(0),
                },
                attachment_descent:
                    echo_wasm_abi::kernel_port::AttachmentDescentPolicy::BoundaryOnly,
            }
        );
    }

    #[test]
    fn truth_channel_aperture_converts_channel_ids_to_abi_bytes() {
        let channel = TypeId([3; 32]);
        let aperture = OpticAperture {
            shape: OpticApertureShape::TruthChannels {
                channels: Some(vec![channel]),
            },
            budget: OpticReadBudget::default(),
            attachment_descent: AttachmentDescentPolicy::BoundaryOnly,
        };

        assert_eq!(
            aperture.to_abi().shape,
            echo_wasm_abi::kernel_port::OpticApertureShape::TruthChannels {
                channels: Some(vec![echo_wasm_abi::kernel_port::ChannelId::from_bytes(
                    [3; 32]
                )]),
            }
        );
    }

    #[test]
    fn read_identity_is_stable_for_same_read_question() {
        let focus = worldline_focus();
        let coordinate = frontier_coordinate();
        let aperture = head_aperture();
        let optic = EchoOptic::new(
            focus.clone(),
            coordinate.clone(),
            ProjectionVersion::from_raw(1),
            None,
            intent_family(1),
            capability(2),
        );

        let first = ReadIdentity::new(
            optic.optic_id,
            &focus,
            coordinate.clone(),
            &aperture,
            ProjectionVersion::from_raw(1),
            None,
            witness_basis(1, 2),
            ReadingRightsPosture::KernelPublic,
            ReadingBudgetPosture::UnboundedOneShot,
            ReadingResidualPosture::Complete,
        );
        let second = ReadIdentity::new(
            optic.optic_id,
            &focus,
            coordinate,
            &aperture,
            ProjectionVersion::from_raw(1),
            None,
            witness_basis(1, 2),
            ReadingRightsPosture::KernelPublic,
            ReadingBudgetPosture::UnboundedOneShot,
            ReadingResidualPosture::Complete,
        );

        assert_eq!(first, second);
        assert_eq!(first.read_identity_hash, second.read_identity_hash);
        assert_eq!(first.focus_digest, focus.digest());
        assert_eq!(first.aperture_digest, aperture.digest());
    }

    #[test]
    fn read_identity_changes_when_question_or_witness_changes() {
        let focus = worldline_focus();
        let coordinate = frontier_coordinate();
        let aperture = head_aperture();
        let optic_id = EchoOptic::new(
            focus.clone(),
            coordinate.clone(),
            ProjectionVersion::from_raw(1),
            None,
            intent_family(1),
            capability(2),
        )
        .optic_id;

        let base = ReadIdentity::new(
            optic_id,
            &focus,
            coordinate.clone(),
            &aperture,
            ProjectionVersion::from_raw(1),
            None,
            witness_basis(1, 2),
            ReadingRightsPosture::KernelPublic,
            ReadingBudgetPosture::UnboundedOneShot,
            ReadingResidualPosture::Complete,
        );
        let changed_coordinate = ReadIdentity::new(
            optic_id,
            &focus,
            EchoCoordinate::Worldline {
                worldline_id: worldline(1),
                at: CoordinateAt::Tick(crate::clock::WorldlineTick::from_raw(3)),
            },
            &aperture,
            ProjectionVersion::from_raw(1),
            None,
            witness_basis(1, 2),
            ReadingRightsPosture::KernelPublic,
            ReadingBudgetPosture::UnboundedOneShot,
            ReadingResidualPosture::Complete,
        );
        let changed_aperture = ReadIdentity::new(
            optic_id,
            &focus,
            coordinate.clone(),
            &OpticAperture {
                shape: OpticApertureShape::SnapshotMetadata,
                budget: aperture.budget,
                attachment_descent: aperture.attachment_descent,
            },
            ProjectionVersion::from_raw(1),
            None,
            witness_basis(1, 2),
            ReadingRightsPosture::KernelPublic,
            ReadingBudgetPosture::UnboundedOneShot,
            ReadingResidualPosture::Complete,
        );
        let changed_projection = ReadIdentity::new(
            optic_id,
            &focus,
            coordinate.clone(),
            &aperture,
            ProjectionVersion::from_raw(2),
            None,
            witness_basis(1, 2),
            ReadingRightsPosture::KernelPublic,
            ReadingBudgetPosture::UnboundedOneShot,
            ReadingResidualPosture::Complete,
        );
        let changed_witness = ReadIdentity::new(
            optic_id,
            &focus,
            coordinate,
            &aperture,
            ProjectionVersion::from_raw(1),
            None,
            witness_basis(1, 3),
            ReadingRightsPosture::KernelPublic,
            ReadingBudgetPosture::UnboundedOneShot,
            ReadingResidualPosture::Complete,
        );

        assert_ne!(
            base.read_identity_hash,
            changed_coordinate.read_identity_hash
        );
        assert_ne!(base.read_identity_hash, changed_aperture.read_identity_hash);
        assert_ne!(
            base.read_identity_hash,
            changed_projection.read_identity_hash
        );
        assert_ne!(base.read_identity_hash, changed_witness.read_identity_hash);
    }

    #[test]
    fn existing_reading_envelope_can_carry_compatible_optic_identity() {
        let focus = worldline_focus();
        let coordinate = frontier_coordinate();
        let aperture = head_aperture();
        let reading = reading_envelope();
        let optic_id = EchoOptic::new(
            focus.clone(),
            coordinate.clone(),
            ProjectionVersion::from_raw(1),
            None,
            intent_family(1),
            capability(2),
        )
        .optic_id;

        let identity = ReadIdentity::from_reading_envelope(
            optic_id,
            &focus,
            coordinate,
            &aperture,
            ProjectionVersion::from_raw(1),
            None,
            witness_basis(1, 2),
            &reading,
        );
        let envelope = OpticReadingEnvelope::new(reading, identity);
        let abi = envelope.to_abi();

        assert_eq!(abi.read_identity.optic_id, optic_id_to_abi(optic_id));
        assert_eq!(
            abi.read_identity.rights_posture,
            echo_wasm_abi::kernel_port::ReadingRightsPosture::KernelPublic
        );
        assert_eq!(
            abi.read_identity.budget_posture,
            echo_wasm_abi::kernel_port::ReadingBudgetPosture::UnboundedOneShot
        );
        assert_eq!(
            abi.read_identity.residual_posture,
            echo_wasm_abi::kernel_port::ReadingResidualPosture::Complete
        );
    }

    #[test]
    fn optic_obstruction_kinds_keep_fail_closed_cases_distinct() {
        use std::collections::BTreeSet;

        let required = [
            (
                OpticObstructionKind::StaleBasis,
                echo_wasm_abi::kernel_port::OpticObstructionKind::StaleBasis,
            ),
            (
                OpticObstructionKind::MissingWitness,
                echo_wasm_abi::kernel_port::OpticObstructionKind::MissingWitness,
            ),
            (
                OpticObstructionKind::BudgetExceeded,
                echo_wasm_abi::kernel_port::OpticObstructionKind::BudgetExceeded,
            ),
            (
                OpticObstructionKind::CapabilityDenied,
                echo_wasm_abi::kernel_port::OpticObstructionKind::CapabilityDenied,
            ),
            (
                OpticObstructionKind::AttachmentDescentRequired,
                echo_wasm_abi::kernel_port::OpticObstructionKind::AttachmentDescentRequired,
            ),
        ];

        let mut names = BTreeSet::new();
        for (core, expected) in required {
            let abi = core.to_abi();
            assert_eq!(abi, expected);
            assert!(names.insert(format!("{abi:?}")));
        }

        assert_eq!(names.len(), required.len());
    }

    #[test]
    fn intent_dispatch_result_matching_is_variant_exhaustive() {
        fn classify(result: &IntentDispatchResult) -> &'static str {
            match result {
                IntentDispatchResult::Admitted(_) => "admitted",
                IntentDispatchResult::Staged(_) => "staged",
                IntentDispatchResult::Plural(_) => "plural",
                IntentDispatchResult::Conflict(_) => "conflict",
                IntentDispatchResult::Obstructed(_) => "obstructed",
            }
        }

        let optic = EchoOptic::new(
            worldline_focus(),
            frontier_coordinate(),
            ProjectionVersion::from_raw(1),
            None,
            intent_family(1),
            capability(2),
        );
        let base_coordinate = frontier_coordinate();
        let family = intent_family(1);
        let admitted_ref = provenance(1, 3);
        let obstruction = OpticObstruction {
            kind: OpticObstructionKind::StaleBasis,
            optic_id: Some(optic.optic_id),
            focus: Some(worldline_focus()),
            coordinate: Some(base_coordinate.clone()),
            witness_basis: Some(WitnessBasis::Missing {
                reason: MissingWitnessBasisReason::EvidenceUnavailable,
            }),
            message: "base coordinate is stale".to_owned(),
        };
        let outcomes = vec![
            IntentDispatchResult::Admitted(AdmittedIntent {
                optic_id: optic.optic_id,
                base_coordinate: base_coordinate.clone(),
                intent_family: family,
                admitted_ref,
                receipt_hash: [4; 32],
            }),
            IntentDispatchResult::Staged(StagedIntent {
                optic_id: optic.optic_id,
                base_coordinate: base_coordinate.clone(),
                intent_family: family,
                stage_ref: [5; 32],
                reason: StagedIntentReason::RebaseRequired,
            }),
            IntentDispatchResult::Plural(PluralIntent {
                optic_id: optic.optic_id,
                base_coordinate: base_coordinate.clone(),
                intent_family: family,
                candidate_refs: vec![admitted_ref, provenance(1, 4)],
                residual_posture: ReadingResidualPosture::PluralityPreserved,
            }),
            IntentDispatchResult::Conflict(IntentConflict {
                optic_id: optic.optic_id,
                base_coordinate,
                intent_family: family,
                reason: IntentConflictReason::StaleBasis,
                conflict_ref: Some(admitted_ref),
                evidence_digest: [6; 32],
                message: "base conflicts with frontier".to_owned(),
            }),
            IntentDispatchResult::Obstructed(obstruction),
        ];

        assert_eq!(
            outcomes.iter().map(classify).collect::<Vec<_>>(),
            vec!["admitted", "staged", "plural", "conflict", "obstructed"]
        );
        assert!(matches!(
            outcomes[0].to_abi(),
            echo_wasm_abi::kernel_port::IntentDispatchResult::Admitted(_)
        ));
        assert!(matches!(
            outcomes[1].to_abi(),
            echo_wasm_abi::kernel_port::IntentDispatchResult::Staged(_)
        ));
        assert!(matches!(
            outcomes[2].to_abi(),
            echo_wasm_abi::kernel_port::IntentDispatchResult::Plural(_)
        ));
        assert!(matches!(
            outcomes[3].to_abi(),
            echo_wasm_abi::kernel_port::IntentDispatchResult::Conflict(_)
        ));
        assert!(matches!(
            outcomes[4].to_abi(),
            echo_wasm_abi::kernel_port::IntentDispatchResult::Obstructed(_)
        ));
    }
}
