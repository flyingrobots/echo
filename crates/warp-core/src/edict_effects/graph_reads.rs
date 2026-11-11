// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Typed-law recording of the bounded-read graph primitives.
//!
//! This module owns how a typed outgoing-edge range and a discovered node's
//! material become read-image observations. Every value a graph read delivers
//! to the program is first authorized by the frame's footprint law and then
//! retained, so the frame's read image witnesses exactly what was observed.
use super::typed_reads::ReadPermission;
use super::OBSERVATION_STORAGE_BYTES;
use super::{EffectFrameError, EffectRangeReadError as Error, EffectRangeReadSession};
use crate::edict_pure::evaluate::Meter;
use crate::edict_read::{Material, Outgoing};
use crate::{
    AttachmentKey, AttachmentValue, NodeKey, OutgoingEdgeRange, SlotId, SlotReading, TypeId,
};

/// Absence carries no type; an empty alpha slot is retained under this one.
const NO_TYPE: TypeId = TypeId([0; 32]);

impl<'a> EffectRangeReadSession<'a> {
    /// Bucket edges one typed graph evaluation may inspect in total: the law's
    /// range bound, which also bounds every single bucket, intersected with the
    /// host's.
    pub(super) fn graph_scan_ceiling(&self) -> u64 {
        self.law
            .as_ref()
            .map_or(0, |law| self.max_range_edges.min(law.max_range_edges))
    }

    /// Resolves one typed range for a graph read.
    ///
    /// `absent_type` authorizes the source's absence; a discovered source is
    /// already known present. A bucket larger than `allowance` is refused as
    /// the authored scan bound after retaining the source observation that
    /// decided it; no edge of that bucket is inspected or retained.
    pub(super) fn record_graph_range(
        &mut self,
        range: OutgoingEdgeRange,
        absent_type: TypeId,
        allowance: u64,
        meter: &mut Meter,
    ) -> Result<Outgoing, EffectFrameError> {
        let law = self.law.as_ref().ok_or(Error::FootprintLawViolation)?;
        if !law.ranges.contains(&range.edge_type) || !self.allowed_types.contains(&range.edge_type)
        {
            return Err(Error::FootprintLawViolation.into());
        }
        let frontier = self.frontier;
        let store = frontier.state().store(&range.source.warp_id);
        // Bucket length is bounded metadata; no edge is delivered by reading it.
        let bucket = store
            .filter(|store| store.node(&range.source.local_id).is_some())
            .map(|store| store.edges_from(&range.source.local_id).len());
        let inspected = u64::try_from(bucket.unwrap_or(0)).map_err(|_| Error::BudgetExceeded)?;
        if bucket.is_some() && inspected > allowance {
            self.typed_node(range.source, absent_type, meter)?;
            return Ok(Outgoing::ScanBound);
        }
        Ok(
            match self.read_range_inner(range, Some(meter), Some(absent_type))? {
                None => Outgoing::SourceMissing,
                Some(edges) => Outgoing::Edges { inspected, edges },
            },
        )
    }

    /// Resolves a node and its alpha slot for a graph read.
    ///
    /// A present node is authorized under its actual type, as a portal owner
    /// when its alpha slot descends. An alpha atom is authorized under its
    /// actual atom type; an empty alpha slot is retained as typeless absence.
    /// `absent_type` authorizes the node's absence: the host-named type for a
    /// selected start node, or the type of the node whose edge named it.
    /// An atom over `max_bytes` is authorized and retained, but its payload is
    /// not charged: the caller refuses it as malformed storage, exactly as over a
    /// plain view, and never copies it.
    pub(super) fn record_material(
        &mut self,
        key: NodeKey,
        absent_type: TypeId,
        max_bytes: u64,
        meter: &mut Meter,
    ) -> Result<Material<'a>, EffectFrameError> {
        self.reachable(key.warp_id, meter)?;
        let frontier = self.frontier;
        if !self.charge(1, 64, OBSERVATION_STORAGE_BYTES, Some(&mut *meter)) {
            return Err(Error::BudgetExceeded.into());
        }
        let store = frontier.state().store(&key.warp_id);
        let Some(node) = store.and_then(|store| store.node(&key.local_id)) else {
            self.authorize(SlotId::Node(key), absent_type, ReadPermission::Node, meter)?;
            self.readings
                .insert(SlotId::Node(key), SlotReading::Node(None));
            self.footprint.n_read.insert(key);
            return Ok(Material::Missing);
        };
        if !self.charge(1, 64, OBSERVATION_STORAGE_BYTES, Some(&mut *meter)) {
            return Err(Error::BudgetExceeded.into());
        }
        let value = store.and_then(|store| store.node_attachment(&key.local_id));
        let owner = if matches!(value, Some(AttachmentValue::Descend(_))) {
            ReadPermission::Portal
        } else {
            ReadPermission::Node
        };
        self.authorize(SlotId::Node(key), node.ty, owner, meter)?;
        self.readings
            .insert(SlotId::Node(key), SlotReading::Node(Some(node.clone())));
        self.footprint.n_read.insert(key);
        let attachment = SlotId::Attachment(AttachmentKey::node_alpha(key));
        let payload = match value {
            Some(AttachmentValue::Descend(_)) => {
                self.authorize(attachment, node.ty, ReadPermission::Portal, meter)?;
                0
            }
            Some(AttachmentValue::Atom(atom)) => {
                self.authorize(attachment, atom.type_id, ReadPermission::Atom, meter)?;
                u64::try_from(atom.bytes.len()).map_err(|_| Error::BudgetExceeded)?
            }
            None => {
                self.retain_authorization(attachment, NO_TYPE, meter)?;
                0
            }
        };
        // An oversized atom is retained as observed but never delivered.
        let delivered = if payload > max_bytes { 0 } else { payload };
        if !self.charge(0, delivered, delivered, Some(meter)) {
            return Err(Error::BudgetExceeded.into());
        }
        self.readings
            .insert(attachment, SlotReading::Attachment(value.cloned()));
        self.footprint.a_read.insert(AttachmentKey::node_alpha(key));
        Ok(match value {
            Some(AttachmentValue::Descend(_)) => Material::Portal,
            Some(AttachmentValue::Atom(atom)) => Material::Node {
                ty: node.ty,
                atom: Some((atom.type_id, &atom.bytes[..])),
            },
            None => Material::Node {
                ty: node.ty,
                atom: None,
            },
        })
    }
}
