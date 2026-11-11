// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
use super::EffectReadImageError as Error;
use crate::{AttachmentOwner, AttachmentPlane, SlotId};
use bytemuck::{Pod, Zeroable};

/// Fixed canonical slot descriptor, 104 bytes, distinct from native SlotId order.
#[repr(C, align(8))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct EffectSlotRow {
    /// Node 0, attachment 1, edge 2, outgoing range 3.
    pub class: u8,
    /// Attachment owner: node 0, edge 1; zero for every other class.
    pub owner: u8,
    /// Attachment plane: alpha 0, beta 1; zero for every other class.
    pub plane: u8,
    /// Canonical zero padding.
    pub padding: [u8; 5],
    /// Exact WARP identifier.
    pub warp_id: crate::Hash,
    /// Node, edge, attachment-owner or range-source local identifier.
    pub local_id: crate::Hash,
    /// Range edge type only; zero for every other class.
    pub edge_type: crate::Hash,
}
/// Fixed canonical typed observation, 200 bytes.
#[repr(C, align(8))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct EffectReadRow {
    /// Canonical slot descriptor.
    pub slot: EffectSlotRow,
    /// Present 0, absent 1, range 2, portal 3.
    pub outcome: u8,
    /// Whether a range or graph read consulted this node's outgoing bucket:
    /// no 0, yes 1. Only a present node row may carry 1. A consulted bucket
    /// decided a scan bound, whether the scan then ran or was refused.
    pub bucket: u8,
    /// Canonical zero padding.
    pub padding: [u8; 6],
    /// Actual/named/portal-owner type, or zero for a range.
    pub read_type: crate::Hash,
    /// Present value or portal digest; zero for absent/range outcomes.
    pub subject: crate::Hash,
    /// Contiguous edge-pair run for a range; zero for all other outcomes.
    pub pairs: crate::wsc::types::Range,
    /// Edges of every type in the consulted bucket, little-endian; zero
    /// unless `bucket` is 1. Every range from this node retains at most this
    /// many pairs.
    pub bucket_len_le: u64,
}
/// One outgoing-range result, 64 bytes, in edge-local-ID order.
#[repr(C, align(8))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct EffectEdgePairRow {
    /// Exact edge local identifier, unique within its range.
    pub edge_local_id: crate::Hash,
    /// Exact target local identifier in the range's WARP.
    pub target_local_id: crate::Hash,
}
const _: () = assert!(std::mem::size_of::<EffectSlotRow>() == 104);
const _: () = assert!(std::mem::size_of::<EffectReadRow>() == 200);
const _: () = assert!(std::mem::size_of::<EffectEdgePairRow>() == 64);

impl EffectSlotRow {
    pub(in crate::edict_effects) fn from_slot(slot: SlotId) -> Result<Self, Error> {
        let mut row = Self::zeroed();
        match slot {
            SlotId::Node(key) => {
                row.warp_id = key.warp_id.0;
                row.local_id = key.local_id.0;
            }
            SlotId::Edge(key) => {
                row.class = 2;
                row.warp_id = key.warp_id.0;
                row.local_id = key.local_id.0;
            }
            SlotId::Attachment(key) => {
                row.class = 1;
                match (key.owner, key.plane) {
                    (AttachmentOwner::Node(node), AttachmentPlane::Alpha) => {
                        row.warp_id = node.warp_id.0;
                        row.local_id = node.local_id.0;
                    }
                    (AttachmentOwner::Edge(edge), AttachmentPlane::Beta) => {
                        row.owner = 1;
                        row.plane = 1;
                        row.warp_id = edge.warp_id.0;
                        row.local_id = edge.local_id.0;
                    }
                    _ => return Err(Error::InconsistentObservation),
                }
            }
            SlotId::OutgoingEdges(range) => {
                row.class = 3;
                row.warp_id = range.source.warp_id.0;
                row.local_id = range.source.local_id.0;
                row.edge_type = range.edge_type.0;
            }
            SlotId::Port(_) => return Err(Error::InconsistentObservation),
        }
        Ok(row)
    }
    pub(in crate::edict_effects) fn compare(&self, other: &Self) -> std::cmp::Ordering {
        (
            self.class,
            self.warp_id,
            self.local_id,
            self.edge_type,
            self.owner,
            self.plane,
        )
            .cmp(&(
                other.class,
                other.warp_id,
                other.local_id,
                other.edge_type,
                other.owner,
                other.plane,
            ))
    }
    pub(super) fn valid(&self) -> bool {
        self.padding == [0; 5]
            && self.class <= 3
            && (if self.class == 1 {
                (self.owner, self.plane) == (0, 0) || (self.owner, self.plane) == (1, 1)
            } else {
                self.owner == 0 && self.plane == 0
            })
            && (self.class == 3 || self.edge_type == [0; 32])
    }
}
impl EffectReadRow {
    /// Edges in the consulted source bucket; zero when `bucket` is 0.
    #[must_use]
    pub const fn bucket_len(&self) -> u64 {
        u64::from_le(self.bucket_len_le)
    }
    pub(super) fn compare(&self, other: &Self) -> std::cmp::Ordering {
        self.slot
            .compare(&other.slot)
            .then_with(|| self.read_type.cmp(&other.read_type))
    }
    pub(super) fn valid(&self) -> bool {
        if !self.slot.valid() || self.padding != [0; 6] {
            return false;
        }
        let witnesses_bucket = match self.bucket {
            0 => self.bucket_len_le == 0,
            1 => self.slot.class == 0 && self.outcome == 0,
            _ => false,
        };
        if !witnesses_bucket {
            return false;
        }
        let no_pairs = self.pairs.start() == 0 && self.pairs.is_empty();
        match self.outcome {
            0 => self.slot.class != 3 && no_pairs,
            1 => self.slot.class != 3 && no_pairs && self.subject == [0; 32],
            2 => self.slot.class == 3 && self.read_type == [0; 32] && self.subject == [0; 32],
            3 => self.slot.class == 1 && self.slot.owner == 0 && self.slot.plane == 0 && no_pairs,
            _ => false,
        }
    }
}
