// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Generic Echo optic nouns and deterministic identifiers.

use blake3::Hasher;
use echo_wasm_abi::kernel_port as abi;

use crate::attachment::{AttachmentKey, AttachmentOwner, AttachmentPlane};
use crate::clock::WorldlineTick;
use crate::ident::{EdgeKey, Hash, NodeKey, TypeId, WarpId};
use crate::materialization::ChannelId;
use crate::provenance_store::ProvenanceRef;
use crate::strand::StrandId;
use crate::worldline::WorldlineId;

const OPTIC_ID_DOMAIN: &[u8] = b"echo:optic-id:v1\0";

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

    #[allow(dead_code)]
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
}
