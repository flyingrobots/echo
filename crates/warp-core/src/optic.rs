// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Generic Echo optic nouns and deterministic identifiers.

use std::collections::{BTreeMap, BTreeSet};

use blake3::Hasher;
use echo_wasm_abi::kernel_port as abi;

use crate::attachment::{AttachmentKey, AttachmentOwner, AttachmentPlane};
use crate::clock::WorldlineTick;
use crate::ident::{EdgeKey, Hash, NodeKey, TypeId, WarpId};
use crate::materialization::ChannelId;
use crate::observation::{
    ObservationPayload, ReadingBudgetPosture, ReadingEnvelope, ReadingResidualPosture,
    ReadingRightsPosture, ReadingWitnessRef,
};
use crate::provenance_store::ProvenanceRef;
use crate::strand::StrandId;
use crate::worldline::WorldlineId;

const OPTIC_ID_DOMAIN: &[u8] = b"echo:optic-id:v1\0";
const FOCUS_DIGEST_DOMAIN: &[u8] = b"echo:optic-focus:v1\0";
const APERTURE_DIGEST_DOMAIN: &[u8] = b"echo:optic-aperture:v1\0";
const READ_IDENTITY_DOMAIN: &[u8] = b"echo:read-identity:v1\0";
const RETAINED_READING_KEY_DOMAIN: &[u8] = b"echo:retained-reading-key:v1\0";

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
    /// Stable identity for the encoding used by a retained reading payload.
    RetainedReadingCodecId
);

opaque_id!(
    /// Stable identity for an intent family admitted through an optic.
    IntentFamilyId
);

opaque_id!(
    /// Stable identity for an admission law used by optic dispatch.
    AdmissionLawId
);

opaque_id!(
    /// Stable identity for an optic capability basis.
    OpticCapabilityId
);

opaque_id!(
    /// Stable identity for an actor opening or using an optic.
    OpticActorId
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

impl RetainedReadingKey {
    /// Derives a retained-reading key from semantic identity and retained bytes.
    #[must_use]
    pub fn derive(
        read_identity: &ReadIdentity,
        content_hash: Hash,
        codec_id: RetainedReadingCodecId,
        byte_len: u64,
    ) -> Self {
        derive_retained_reading_key(read_identity, &content_hash, codec_id, byte_len)
    }
}

/// Descriptor for a retained reading payload.
///
/// The CAS/content hash names bytes. This descriptor's key additionally names
/// the semantic read identity and codec, so equal bytes answering different
/// questions do not alias.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RetainedReadingDescriptor {
    /// Stable key derived from the semantic read identity and byte identity.
    pub key: RetainedReadingKey,
    /// Semantic identity of the question answered by the retained payload.
    pub read_identity: ReadIdentity,
    /// Content hash of the retained payload bytes.
    pub content_hash: Hash,
    /// Codec used for the retained payload bytes.
    pub codec_id: RetainedReadingCodecId,
    /// Retained payload byte length.
    pub byte_len: u64,
}

impl RetainedReadingDescriptor {
    /// Builds a retained-reading descriptor and derives its stable key.
    #[must_use]
    pub fn new(
        read_identity: ReadIdentity,
        content_hash: Hash,
        codec_id: RetainedReadingCodecId,
        byte_len: u64,
    ) -> Self {
        let key = RetainedReadingKey::derive(&read_identity, content_hash, codec_id, byte_len);
        Self {
            key,
            read_identity,
            content_hash,
            codec_id,
            byte_len,
        }
    }

    /// Converts the descriptor to the shared ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::RetainedReadingDescriptor {
        abi::RetainedReadingDescriptor {
            key: retained_reading_key_to_abi(self.key),
            read_identity: self.read_identity.to_abi(),
            content_hash: self.content_hash.to_vec(),
            codec_id: retained_reading_codec_id_to_abi(self.codec_id),
            byte_len: self.byte_len,
        }
    }
}

/// Request to retain reading payload bytes under a semantic read identity.
///
/// Retention stores bytes and a descriptor only. It does not create substrate
/// truth and does not mutate the optic subject.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RetainReadingRequest {
    /// Semantic identity of the read question answered by the payload.
    pub read_identity: ReadIdentity,
    /// Codec used to encode the retained payload bytes.
    pub codec_id: RetainedReadingCodecId,
    /// Encoded reading payload bytes.
    pub payload: Vec<u8>,
}

/// Result of retaining reading payload bytes.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RetainReadingResult {
    /// Descriptor naming both the retained bytes and their semantic read identity.
    pub descriptor: RetainedReadingDescriptor,
}

/// Request to reveal a retained reading payload.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RevealReadingRequest {
    /// Retained-reading key returned by retention.
    pub key: RetainedReadingKey,
    /// Exact semantic identity the caller is authorized to reveal.
    pub read_identity: ReadIdentity,
}

/// Result of revealing a retained reading payload.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RevealReadingResult {
    /// Descriptor for the revealed payload.
    pub descriptor: RetainedReadingDescriptor,
    /// Encoded retained payload bytes.
    pub payload: Vec<u8>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct RetainedReadingCacheEntry {
    descriptor: RetainedReadingDescriptor,
    payload: Vec<u8>,
}

/// In-memory semantic cache for retained optic readings.
///
/// This cache is intentionally above CAS. The content hash names bytes, while
/// the retained-reading key names bytes plus the semantic `ReadIdentity` and
/// codec. Revealing by content hash alone is not a supported operation.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct RetainedReadingCache {
    entries: BTreeMap<RetainedReadingKey, RetainedReadingCacheEntry>,
    content_index: BTreeMap<Hash, BTreeSet<RetainedReadingKey>>,
}

impl RetainedReadingCache {
    /// Returns the number of retained semantic readings.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` when the cache has no retained readings.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Retains encoded reading bytes under their semantic read identity.
    ///
    /// The derived key includes the content hash, codec, byte length, and exact
    /// read identity. Equal bytes answering different questions therefore retain
    /// under different keys.
    pub fn retain_reading(&mut self, request: RetainReadingRequest) -> RetainReadingResult {
        let content_hash = retained_payload_hash(&request.payload);
        let byte_len = request.payload.len() as u64;
        let descriptor = RetainedReadingDescriptor::new(
            request.read_identity,
            content_hash,
            request.codec_id,
            byte_len,
        );
        self.content_index
            .entry(content_hash)
            .or_default()
            .insert(descriptor.key);
        self.entries.insert(
            descriptor.key,
            RetainedReadingCacheEntry {
                descriptor: descriptor.clone(),
                payload: request.payload,
            },
        );
        RetainReadingResult { descriptor }
    }

    /// Reveals retained reading bytes only when key and read identity both match.
    ///
    /// A content hash alone cannot reveal payload bytes because it does not name
    /// the coordinate, aperture, witness basis, projection/reducer versions,
    /// rights posture, budget posture, or residual posture the bytes answer.
    pub fn reveal_reading(
        &self,
        request: &RevealReadingRequest,
    ) -> Result<RevealReadingResult, Box<OpticObstruction>> {
        let Some(entry) = self.entries.get(&request.key) else {
            return Err(retained_reading_obstruction(
                request.key,
                &request.read_identity,
                "retained reading key was not found",
            ));
        };
        if entry.descriptor.read_identity != request.read_identity {
            return Err(retained_reading_obstruction(
                request.key,
                &request.read_identity,
                "retained reading identity mismatch; reveal requires the exact read identity",
            ));
        }

        Ok(RevealReadingResult {
            descriptor: entry.descriptor.clone(),
            payload: entry.payload.clone(),
        })
    }

    /// Returns retained-reading keys that share the same byte content hash.
    ///
    /// This is an index/diagnostic query, not reveal authority. Callers must use
    /// [`Self::reveal_reading`] with an exact `ReadIdentity` to obtain bytes.
    #[must_use]
    pub fn keys_for_content_hash(&self, content_hash: Hash) -> Vec<RetainedReadingKey> {
        self.content_index
            .get(&content_hash)
            .map_or_else(Vec::new, |keys| keys.iter().copied().collect())
    }
}

/// Bounded read request through an Echo optic.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ObserveOpticRequest {
    /// Optic being observed.
    pub optic_id: OpticId,
    /// Focus being observed.
    pub focus: OpticFocus,
    /// Explicit causal coordinate for the read.
    pub coordinate: EchoCoordinate,
    /// Bounded aperture selected by the read.
    pub aperture: OpticAperture,
    /// Projection law version requested by the read.
    pub projection_version: ProjectionVersion,
    /// Reducer law version requested by the read, when present.
    pub reducer_version: Option<ReducerVersion>,
    /// Capability basis for the read.
    pub capability: OpticCapabilityId,
}

impl ObserveOpticRequest {
    /// Converts the request to the shared ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::ObserveOpticRequest {
        abi::ObserveOpticRequest {
            optic_id: optic_id_to_abi(self.optic_id),
            focus: self.focus.to_abi(),
            coordinate: self.coordinate.to_abi(),
            aperture: self.aperture.to_abi(),
            projection_version: self.projection_version.to_abi(),
            reducer_version: self.reducer_version.map(ReducerVersion::to_abi),
            capability: optic_capability_id_to_abi(self.capability),
        }
    }
}

/// Intent payload dispatched through an optic.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OpticIntentPayload {
    /// Canonical Echo intent v1 bytes.
    EintV1 {
        /// Complete EINT v1 envelope bytes.
        bytes: Vec<u8>,
    },
}

impl OpticIntentPayload {
    /// Converts the payload to the shared ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::OpticIntentPayload {
        match self {
            Self::EintV1 { bytes } => abi::OpticIntentPayload::EintV1 {
                bytes: bytes.clone(),
            },
        }
    }
}

/// Write-side proposal request through an Echo optic.
///
/// This is not a setter. It names an explicit causal basis and carries an
/// intent payload for normal Echo admission.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DispatchOpticIntentRequest {
    /// Optic being used as the proposal boundary.
    pub optic_id: OpticId,
    /// Explicit causal basis for the proposal.
    pub base_coordinate: EchoCoordinate,
    /// Intent family being proposed.
    pub intent_family: IntentFamilyId,
    /// Focus targeted by the proposal.
    pub focus: OpticFocus,
    /// Actor/cause associated with the proposal.
    pub cause: OpticCause,
    /// Capability basis for the proposal.
    pub capability: OpticCapability,
    /// Admission law requested for the proposal.
    pub admission_law: AdmissionLawId,
    /// Intent payload carried by the proposal.
    pub payload: OpticIntentPayload,
}

impl DispatchOpticIntentRequest {
    /// Validates the generic optic proposal boundary without dispatching it.
    ///
    /// # Errors
    ///
    /// Returns a typed obstruction when focus, base coordinate, actor,
    /// capability, intent family, or payload evidence does not line up.
    pub fn validate_proposal(&self) -> Result<(), Box<OpticObstruction>> {
        if !focus_matches_coordinate(&self.focus, &self.base_coordinate) {
            return Err(self.dispatch_obstruction(
                OpticObstructionKind::ConflictingFrontier,
                "optic dispatch focus and base coordinate name different subjects",
            ));
        }

        if self.capability.actor != self.cause.actor {
            return Err(self.dispatch_obstruction(
                OpticObstructionKind::CapabilityDenied,
                "optic dispatch capability actor does not match cause actor",
            ));
        }

        if self.capability.allowed_focus != self.focus {
            return Err(self.dispatch_obstruction(
                OpticObstructionKind::CapabilityDenied,
                "optic dispatch capability does not authorize focus",
            ));
        }

        if self.capability.allowed_intent_family != self.intent_family {
            return Err(self.dispatch_obstruction(
                OpticObstructionKind::UnsupportedIntentFamily,
                "optic dispatch capability does not authorize intent family",
            ));
        }

        match &self.payload {
            OpticIntentPayload::EintV1 { bytes } => {
                if let Err(error) = echo_wasm_abi::unpack_intent_v1(bytes) {
                    return Err(self.dispatch_obstruction(
                        OpticObstructionKind::UnsupportedIntentFamily,
                        format!("optic dispatch EINT v1 payload is malformed: {error}"),
                    ));
                }
            }
        }

        Ok(())
    }

    /// Validates the proposal against a known current coordinate.
    ///
    /// # Errors
    ///
    /// Returns [`OpticObstructionKind::StaleBasis`] when the request names a
    /// concrete base coordinate older than the supplied current coordinate.
    pub fn validate_proposal_against_current(
        &self,
        current_coordinate: &EchoCoordinate,
    ) -> Result<(), Box<OpticObstruction>> {
        self.validate_proposal()?;

        if !coordinates_name_same_subject(&self.base_coordinate, current_coordinate) {
            return Err(self.dispatch_obstruction(
                OpticObstructionKind::ConflictingFrontier,
                "optic dispatch current coordinate names a different subject",
            ));
        }

        if base_coordinate_is_stale(&self.base_coordinate, current_coordinate) {
            return Err(self.dispatch_obstruction(
                OpticObstructionKind::StaleBasis,
                "optic dispatch base coordinate is stale relative to current frontier",
            ));
        }

        Ok(())
    }

    fn dispatch_obstruction(
        &self,
        kind: OpticObstructionKind,
        message: impl Into<String>,
    ) -> Box<OpticObstruction> {
        Box::new(OpticObstruction {
            kind,
            optic_id: Some(self.optic_id),
            focus: Some(self.focus.clone()),
            coordinate: Some(self.base_coordinate.clone()),
            witness_basis: None,
            message: message.into(),
        })
    }

    /// Converts the request to the shared ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::DispatchOpticIntentRequest {
        abi::DispatchOpticIntentRequest {
            optic_id: optic_id_to_abi(self.optic_id),
            base_coordinate: self.base_coordinate.to_abi(),
            intent_family: intent_family_id_to_abi(self.intent_family),
            focus: self.focus.to_abi(),
            cause: self.cause.to_abi(),
            capability: self.capability.to_abi(),
            admission_law: admission_law_id_to_abi(self.admission_law),
            payload: self.payload.to_abi(),
        }
    }
}

/// Successful bounded reading returned through an optic.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OpticReading {
    /// Reading-envelope metadata.
    pub envelope: ReadingEnvelope,
    /// Stable read identity for the question this reading answered.
    pub read_identity: ReadIdentity,
    /// Observation payload emitted by the observer.
    pub payload: ObservationPayload,
    /// Retained reading key, when the payload was retained.
    pub retained: Option<RetainedReadingKey>,
}

impl OpticReading {
    /// Converts the reading to the shared ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::OpticReading {
        abi::OpticReading {
            envelope: self.envelope.to_abi(),
            read_identity: self.read_identity.to_abi(),
            payload: self.payload.to_abi(),
            retained: self.retained.map(retained_reading_key_to_abi),
        }
    }
}

/// Result of observing an optic.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ObserveOpticResult {
    /// The optic emitted a bounded reading.
    Reading(Box<OpticReading>),
    /// The optic could not lawfully emit a reading.
    Obstructed(Box<OpticObstruction>),
}

impl ObserveOpticResult {
    /// Converts the result to the shared ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::ObserveOpticResult {
        match self {
            Self::Reading(reading) => abi::ObserveOpticResult::Reading(Box::new(reading.to_abi())),
            Self::Obstructed(obstruction) => {
                abi::ObserveOpticResult::Obstructed(Box::new(obstruction.to_abi()))
            }
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

/// Auditable cause for opening, closing, observing, or dispatching through an optic.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OpticCause {
    /// Actor associated with the cause.
    pub actor: OpticActorId,
    /// Stable digest of the host-level cause or request.
    pub cause_hash: Hash,
    /// Optional diagnostic label for humans.
    pub label: Option<String>,
}

impl OpticCause {
    /// Converts the cause to the shared ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::OpticCause {
        abi::OpticCause {
            actor: optic_actor_id_to_abi(self.actor),
            cause_hash: self.cause_hash.to_vec(),
            label: self.label.clone(),
        }
    }
}

/// Capability grant used while validating an optic descriptor.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OpticCapability {
    /// Stable capability identity retained in opened optic descriptors.
    pub capability_id: OpticCapabilityId,
    /// Actor to which the capability was issued.
    pub actor: OpticActorId,
    /// Provenance ref for the issuer or policy source, when available.
    pub issuer_ref: Option<ProvenanceRef>,
    /// Stable digest of the capability policy.
    pub policy_hash: Hash,
    /// Focus this minimal capability authorizes.
    pub allowed_focus: OpticFocus,
    /// Projection law version this capability authorizes.
    pub projection_version: ProjectionVersion,
    /// Reducer law version this capability authorizes, when required.
    pub reducer_version: Option<ReducerVersion>,
    /// Intent family this capability authorizes.
    pub allowed_intent_family: IntentFamilyId,
    /// Maximum read budget authorized by this capability.
    pub max_budget: OpticReadBudget,
}

impl OpticCapability {
    /// Converts the capability to the shared ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::OpticCapability {
        abi::OpticCapability {
            capability_id: optic_capability_id_to_abi(self.capability_id),
            actor: optic_actor_id_to_abi(self.actor),
            issuer_ref: self.issuer_ref.map(provenance_ref_to_abi),
            policy_hash: self.policy_hash.to_vec(),
            allowed_focus: self.allowed_focus.to_abi(),
            projection_version: self.projection_version.to_abi(),
            reducer_version: self.reducer_version.map(ReducerVersion::to_abi),
            allowed_intent_family: intent_family_id_to_abi(self.allowed_intent_family),
            max_budget: self.max_budget.to_abi(),
        }
    }
}

/// Capability posture returned after successfully validating an optic descriptor.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CapabilityPosture {
    /// The descriptor is authorized by the named capability grant.
    Granted {
        /// Capability identity retained in the opened descriptor.
        capability_id: OpticCapabilityId,
        /// Actor to which the capability was issued.
        actor: OpticActorId,
        /// Provenance ref for the issuer or policy source, when available.
        issuer_ref: Option<ProvenanceRef>,
        /// Stable digest of the capability policy.
        policy_hash: Hash,
    },
}

impl CapabilityPosture {
    fn to_abi(&self) -> abi::CapabilityPosture {
        match self {
            Self::Granted {
                capability_id,
                actor,
                issuer_ref,
                policy_hash,
            } => abi::CapabilityPosture::Granted {
                capability_id: optic_capability_id_to_abi(*capability_id),
                actor: optic_actor_id_to_abi(*actor),
                issuer_ref: issuer_ref.map(provenance_ref_to_abi),
                policy_hash: policy_hash.to_vec(),
            },
        }
    }
}

/// Descriptor-validation request for opening a session-local optic resource.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OpenOpticRequest {
    /// Lawful subject being observed or targeted by intent dispatch.
    pub focus: OpticFocus,
    /// Explicit causal coordinate for the optic descriptor.
    pub coordinate: EchoCoordinate,
    /// Projection law version requested by the descriptor.
    pub projection_version: ProjectionVersion,
    /// Reducer law version requested by the descriptor, when present.
    pub reducer_version: Option<ReducerVersion>,
    /// Intent family allowed through the opened optic.
    pub intent_family: IntentFamilyId,
    /// Capability grant used to validate this descriptor.
    pub capability: OpticCapability,
    /// Auditable cause for opening the descriptor.
    pub cause: OpticCause,
}

impl OpenOpticRequest {
    /// Validates the descriptor and derives the session-local optic identity.
    ///
    /// This is descriptor validation only. It does not create a mutable handle
    /// to the subject and does not mutate causal history.
    ///
    /// # Errors
    ///
    /// Returns a typed obstruction when focus, coordinate, projection law,
    /// reducer law, intent family, or capability evidence does not line up.
    pub fn validate_descriptor(&self) -> Result<OpenOpticResult, OpticOpenError> {
        if !focus_matches_coordinate(&self.focus, &self.coordinate) {
            return Err(self.open_obstruction(
                OpticObstructionKind::ConflictingFrontier,
                "optic focus and coordinate name different subjects",
            ));
        }

        if self.capability.actor != self.cause.actor {
            return Err(self.open_obstruction(
                OpticObstructionKind::CapabilityDenied,
                "capability actor does not match optic cause actor",
            ));
        }

        if self.capability.allowed_focus != self.focus {
            return Err(self.open_obstruction(
                OpticObstructionKind::CapabilityDenied,
                "capability does not authorize optic focus",
            ));
        }

        if self.capability.projection_version != self.projection_version
            || self.capability.reducer_version != self.reducer_version
        {
            return Err(self.open_obstruction(
                OpticObstructionKind::UnsupportedProjectionLaw,
                "capability does not authorize projection or reducer law",
            ));
        }

        if self.capability.allowed_intent_family != self.intent_family {
            return Err(self.open_obstruction(
                OpticObstructionKind::UnsupportedIntentFamily,
                "capability does not authorize intent family",
            ));
        }

        let optic = EchoOptic::new(
            self.focus.clone(),
            self.coordinate.clone(),
            self.projection_version,
            self.reducer_version,
            self.intent_family,
            self.capability.capability_id,
        );
        Ok(OpenOpticResult {
            optic,
            capability_posture: CapabilityPosture::Granted {
                capability_id: self.capability.capability_id,
                actor: self.capability.actor,
                issuer_ref: self.capability.issuer_ref,
                policy_hash: self.capability.policy_hash,
            },
        })
    }

    fn open_obstruction(&self, kind: OpticObstructionKind, message: &str) -> OpticOpenError {
        OpticOpenError::Obstructed(Box::new(OpticObstruction {
            kind,
            optic_id: None,
            focus: Some(self.focus.clone()),
            coordinate: Some(self.coordinate.clone()),
            witness_basis: None,
            message: message.to_owned(),
        }))
    }

    /// Converts the request to the shared ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::OpenOpticRequest {
        abi::OpenOpticRequest {
            focus: self.focus.to_abi(),
            coordinate: self.coordinate.to_abi(),
            projection_version: self.projection_version.to_abi(),
            reducer_version: self.reducer_version.map(ReducerVersion::to_abi),
            intent_family: intent_family_id_to_abi(self.intent_family),
            capability: self.capability.to_abi(),
            cause: self.cause.to_abi(),
        }
    }
}

/// Successful descriptor-validation result for opening an optic.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OpenOpticResult {
    /// Opened optic descriptor. This is not a mutable subject handle.
    pub optic: EchoOptic,
    /// Capability posture that authorized the descriptor.
    pub capability_posture: CapabilityPosture,
}

impl OpenOpticResult {
    /// Converts the result to the shared ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::OpenOpticResult {
        abi::OpenOpticResult {
            optic: self.optic.to_abi(),
            capability_posture: self.capability_posture.to_abi(),
        }
    }
}

/// Error returned while opening an optic descriptor.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OpticOpenError {
    /// Opening failed with a typed obstruction.
    Obstructed(Box<OpticObstruction>),
}

impl OpticOpenError {
    /// Converts the error to the shared ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::OpticOpenError {
        match self {
            Self::Obstructed(obstruction) => abi::OpticOpenError::Obstructed(obstruction.to_abi()),
        }
    }
}

/// Request for releasing a session-local optic descriptor resource.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CloseOpticRequest {
    /// Optic descriptor to release from the session.
    pub optic_id: OpticId,
    /// Auditable cause for closing the descriptor.
    pub cause: OpticCause,
}

impl CloseOpticRequest {
    /// Builds the close result without naming or mutating any subject coordinate.
    #[must_use]
    pub fn close_session_descriptor(&self) -> CloseOpticResult {
        CloseOpticResult {
            optic_id: self.optic_id,
        }
    }

    /// Converts the request to the shared ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::CloseOpticRequest {
        abi::CloseOpticRequest {
            optic_id: optic_id_to_abi(self.optic_id),
            cause: self.cause.to_abi(),
        }
    }
}

/// Result for releasing a session-local optic descriptor resource.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CloseOpticResult {
    /// Optic descriptor released from the session.
    pub optic_id: OpticId,
}

impl CloseOpticResult {
    /// Converts the result to the shared ABI DTO.
    #[must_use]
    pub fn to_abi(self) -> abi::CloseOpticResult {
        abi::CloseOpticResult {
            optic_id: optic_id_to_abi(self.optic_id),
        }
    }
}

/// Error returned while closing an optic descriptor.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OpticCloseError {
    /// Closing failed with a typed obstruction.
    Obstructed(Box<OpticObstruction>),
}

impl OpticCloseError {
    /// Converts the error to the shared ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::OpticCloseError {
        match self {
            Self::Obstructed(obstruction) => abi::OpticCloseError::Obstructed(obstruction.to_abi()),
        }
    }
}

/// Narrow built-in example optic over a worldline head.
///
/// This type exists to validate the ergonomics of the generic optics API
/// without introducing a universal optic engine or an application-specific
/// substrate. It is still only a request builder: reads go through
/// `observe_optic`, and proposals go through `dispatch_optic_intent`.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WorldlineHeadOptic {
    /// Opened optic descriptor. This is not a mutable handle.
    pub optic: EchoOptic,
    /// Capability grant that authorized the descriptor.
    pub capability: OpticCapability,
}

impl WorldlineHeadOptic {
    /// Opens a narrow worldline-head optic descriptor.
    ///
    /// # Errors
    ///
    /// Returns the same typed open obstruction as [`OpenOpticRequest`] when
    /// descriptor fields or capability evidence do not line up.
    pub fn open(
        worldline_id: WorldlineId,
        coordinate_at: CoordinateAt,
        actor: OpticActorId,
        capability_id: OpticCapabilityId,
        intent_family: IntentFamilyId,
        policy_hash: Hash,
    ) -> Result<Self, OpticOpenError> {
        let focus = OpticFocus::Worldline { worldline_id };
        let coordinate = EchoCoordinate::Worldline {
            worldline_id,
            at: coordinate_at,
        };
        let capability = OpticCapability {
            capability_id,
            actor,
            issuer_ref: None,
            policy_hash,
            allowed_focus: focus.clone(),
            projection_version: ProjectionVersion::from_raw(1),
            reducer_version: None,
            allowed_intent_family: intent_family,
            max_budget: OpticReadBudget {
                max_bytes: Some(4096),
                max_nodes: Some(64),
                max_ticks: Some(16),
                max_attachments: Some(0),
            },
        };
        let cause = OpticCause {
            actor,
            cause_hash: derive_example_cause_hash(
                worldline_id,
                coordinate_at,
                intent_family,
                capability_id,
            ),
            label: Some("worldline head optic example".to_owned()),
        };
        let request = OpenOpticRequest {
            focus,
            coordinate,
            projection_version: ProjectionVersion::from_raw(1),
            reducer_version: None,
            intent_family,
            capability: capability.clone(),
            cause,
        };
        let result = request.validate_descriptor()?;
        Ok(Self {
            optic: result.optic,
            capability,
        })
    }

    /// Builds a bounded head read request for this optic.
    #[must_use]
    pub fn observe_head_request(&self, budget: OpticReadBudget) -> ObserveOpticRequest {
        ObserveOpticRequest {
            optic_id: self.optic.optic_id,
            focus: self.optic.focus.clone(),
            coordinate: self.optic.coordinate.clone(),
            aperture: OpticAperture {
                shape: OpticApertureShape::Head,
                budget,
                attachment_descent: AttachmentDescentPolicy::BoundaryOnly,
            },
            projection_version: self.optic.projection_version,
            reducer_version: self.optic.reducer_version,
            capability: self.capability.capability_id,
        }
    }

    /// Builds a QueryBytes-shaped read request against this optic.
    ///
    /// The current example host does not install a contract query observer, so
    /// executing this request should produce a typed projection-law obstruction.
    #[must_use]
    pub fn observe_query_bytes_request(
        &self,
        query_id: u32,
        vars_digest: Hash,
        budget: OpticReadBudget,
    ) -> ObserveOpticRequest {
        ObserveOpticRequest {
            optic_id: self.optic.optic_id,
            focus: self.optic.focus.clone(),
            coordinate: self.optic.coordinate.clone(),
            aperture: OpticAperture {
                shape: OpticApertureShape::QueryBytes {
                    query_id,
                    vars_digest,
                },
                budget,
                attachment_descent: AttachmentDescentPolicy::BoundaryOnly,
            },
            projection_version: self.optic.projection_version,
            reducer_version: self.optic.reducer_version,
            capability: self.capability.capability_id,
        }
    }

    /// Builds an EINT v1 proposal request with an explicit causal basis.
    ///
    /// This is intentionally not named `set`: the returned value is a proposal
    /// request that must be passed to `dispatch_optic_intent`.
    #[must_use]
    pub fn dispatch_eint_v1_request(
        &self,
        base_coordinate: EchoCoordinate,
        cause: OpticCause,
        admission_law: AdmissionLawId,
        bytes: Vec<u8>,
    ) -> DispatchOpticIntentRequest {
        DispatchOpticIntentRequest {
            optic_id: self.optic.optic_id,
            base_coordinate,
            intent_family: self.optic.intent_family,
            focus: self.optic.focus.clone(),
            cause,
            capability: self.capability.clone(),
            admission_law,
            payload: OpticIntentPayload::EintV1 { bytes },
        }
    }
}

fn focus_matches_coordinate(focus: &OpticFocus, coordinate: &EchoCoordinate) -> bool {
    match (focus, coordinate) {
        (
            OpticFocus::Worldline { worldline_id },
            EchoCoordinate::Worldline {
                worldline_id: coordinate_worldline,
                ..
            },
        ) => worldline_id == coordinate_worldline,
        (
            OpticFocus::Strand { strand_id },
            EchoCoordinate::Strand {
                strand_id: coordinate_strand,
                ..
            },
        ) => strand_id == coordinate_strand,
        (
            OpticFocus::Braid { braid_id },
            EchoCoordinate::Braid {
                braid_id: coordinate_braid,
                ..
            },
        ) => braid_id == coordinate_braid,
        (
            OpticFocus::RetainedReading { key },
            EchoCoordinate::RetainedReading {
                key: coordinate_key,
            },
        ) => key == coordinate_key,
        (OpticFocus::AttachmentBoundary { .. }, _) => true,
        _ => false,
    }
}

fn coordinates_name_same_subject(base: &EchoCoordinate, current: &EchoCoordinate) -> bool {
    match (base, current) {
        (
            EchoCoordinate::Worldline { worldline_id, .. },
            EchoCoordinate::Worldline {
                worldline_id: current_worldline,
                ..
            },
        ) => worldline_id == current_worldline,
        (
            EchoCoordinate::Strand { strand_id, .. },
            EchoCoordinate::Strand {
                strand_id: current_strand,
                ..
            },
        ) => strand_id == current_strand,
        (
            EchoCoordinate::Braid { braid_id, .. },
            EchoCoordinate::Braid {
                braid_id: current_braid,
                ..
            },
        ) => braid_id == current_braid,
        (
            EchoCoordinate::RetainedReading { key },
            EchoCoordinate::RetainedReading { key: current_key },
        ) => key == current_key,
        _ => false,
    }
}

fn base_coordinate_is_stale(base: &EchoCoordinate, current: &EchoCoordinate) -> bool {
    match (base, current) {
        (
            EchoCoordinate::Worldline { at, .. } | EchoCoordinate::Strand { at, .. },
            EchoCoordinate::Worldline { at: current_at, .. }
            | EchoCoordinate::Strand { at: current_at, .. },
        ) => coordinate_at_tick(*at).is_some_and(|base_tick| {
            coordinate_at_tick(*current_at).is_some_and(|current_tick| base_tick < current_tick)
        }),
        (
            EchoCoordinate::Braid { member_count, .. },
            EchoCoordinate::Braid {
                member_count: current_member_count,
                ..
            },
        ) => member_count < current_member_count,
        _ => false,
    }
}

fn coordinate_at_tick(at: CoordinateAt) -> Option<u64> {
    match at {
        CoordinateAt::Frontier => None,
        CoordinateAt::Tick(tick) => Some(tick.as_u64()),
        CoordinateAt::Provenance(reference) => Some(reference.worldline_tick.as_u64()),
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

fn derive_example_cause_hash(
    worldline_id: WorldlineId,
    coordinate_at: CoordinateAt,
    intent_family: IntentFamilyId,
    capability_id: OpticCapabilityId,
) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(b"echo:worldline-head-optic-example-cause:v1\0");
    hasher.update(worldline_id.as_bytes());
    coordinate_at.feed_hash(&mut hasher);
    hasher.update(intent_family.as_bytes());
    hasher.update(capability_id.as_bytes());
    hasher.finalize().into()
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

fn derive_retained_reading_key(
    read_identity: &ReadIdentity,
    content_hash: &Hash,
    codec_id: RetainedReadingCodecId,
    byte_len: u64,
) -> RetainedReadingKey {
    let mut hasher = Hasher::new();
    hasher.update(RETAINED_READING_KEY_DOMAIN);
    hasher.update(&read_identity.read_identity_hash);
    hasher.update(content_hash);
    hasher.update(codec_id.as_bytes());
    feed_u64(&mut hasher, byte_len);
    RetainedReadingKey::from_bytes(hasher.finalize().into())
}

fn retained_payload_hash(payload: &[u8]) -> Hash {
    blake3::hash(payload).into()
}

fn retained_reading_obstruction(
    key: RetainedReadingKey,
    read_identity: &ReadIdentity,
    message: impl Into<String>,
) -> Box<OpticObstruction> {
    Box::new(OpticObstruction {
        kind: OpticObstructionKind::MissingRetainedReading,
        optic_id: Some(read_identity.optic_id),
        focus: Some(OpticFocus::RetainedReading { key }),
        coordinate: Some(EchoCoordinate::RetainedReading { key }),
        witness_basis: Some(read_identity.witness_basis.clone()),
        message: message.into(),
    })
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
        ReadingBudgetPosture::Bounded {
            max_payload_bytes,
            payload_bytes,
            max_witness_refs,
            witness_refs,
        } => {
            feed_tag(hasher, 2);
            feed_u64(hasher, max_payload_bytes);
            feed_u64(hasher, payload_bytes);
            feed_u64(hasher, max_witness_refs);
            feed_u64(hasher, witness_refs);
        }
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

fn retained_reading_codec_id_to_abi(id: RetainedReadingCodecId) -> abi::RetainedReadingCodecId {
    abi::RetainedReadingCodecId::from_bytes(*id.as_bytes())
}

fn intent_family_id_to_abi(id: IntentFamilyId) -> abi::IntentFamilyId {
    abi::IntentFamilyId::from_bytes(*id.as_bytes())
}

fn admission_law_id_to_abi(id: AdmissionLawId) -> abi::AdmissionLawId {
    abi::AdmissionLawId::from_bytes(*id.as_bytes())
}

fn optic_capability_id_to_abi(id: OpticCapabilityId) -> abi::OpticCapabilityId {
    abi::OpticCapabilityId::from_bytes(*id.as_bytes())
}

fn optic_actor_id_to_abi(id: OpticActorId) -> abi::OpticActorId {
    abi::OpticActorId::from_bytes(*id.as_bytes())
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
        ReadingBudgetPosture::Bounded {
            max_payload_bytes,
            payload_bytes,
            max_witness_refs,
            witness_refs,
        } => abi::ReadingBudgetPosture::Bounded {
            max_payload_bytes,
            payload_bytes,
            max_witness_refs,
            witness_refs,
        },
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
mod tests;
