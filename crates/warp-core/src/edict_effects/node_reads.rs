// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Recording adapter for the existing compiler-produced bounded-read interpreter.
use super::{
    EffectFrameError, EffectRangeReadError, EffectRangeReadSession, OBSERVATION_STORAGE_BYTES,
};
use crate::edict_pure::evaluate::Meter;
use crate::edict_read::{NodeAtomAccess, ReadBasis, ReadObstruction};
use crate::{AttachmentKey, AttachmentValue, NodeKey, SlotId, SlotReading, TypeId};

pub(crate) struct RecordedNodeAccess<'r, 'f> {
    pub(crate) reads: &'r mut EffectRangeReadSession<'f>,
    pub(crate) aperture: NodeAperture<'r>,
}

impl NodeAtomAccess for RecordedNodeAccess<'_, '_> {
    type Error = EffectFrameError;
    fn basis(&self) -> ReadBasis {
        self.reads.basis()
    }
    fn contains(&self, node: NodeKey) -> bool {
        self.aperture.absent_type(node).is_some()
    }
    fn atom(
        &mut self,
        node: NodeKey,
        expected: TypeId,
        max_bytes: u64,
        meter: &mut Meter,
    ) -> Result<Result<&[u8], ReadObstruction>, Self::Error> {
        match self.aperture {
            NodeAperture::Untyped(_) => self.reads.record_atom(node, expected, max_bytes, meter),
            NodeAperture::Typed(_) => {
                let absent_type = self
                    .aperture
                    .absent_type(node)
                    .ok_or(EffectRangeReadError::FootprintLawViolation)?;
                self.reads
                    .record_typed_atom(node, absent_type, expected, max_bytes, meter)
            }
        }
    }
}

impl EffectRangeReadSession<'_> {
    fn record_atom(
        &mut self,
        key: NodeKey,
        expected: TypeId,
        max_bytes: u64,
        meter: &mut Meter,
    ) -> Result<Result<&[u8], ReadObstruction>, EffectFrameError> {
        let frontier = self.frontier;
        if !self.charge(1, 64, OBSERVATION_STORAGE_BYTES, Some(&mut *meter)) {
            return Err(EffectFrameError::Read(EffectRangeReadError::BudgetExceeded));
        }
        let store = frontier.state().store(&key.warp_id);
        let node = store.and_then(|store| store.node(&key.local_id));
        self.readings
            .insert(SlotId::Node(key), SlotReading::Node(node.cloned()));
        self.footprint.n_read.insert(key);
        if node.is_none() {
            return Ok(Err(ReadObstruction::Missing));
        }
        let value = store.and_then(|store| store.node_attachment(&key.local_id));
        // Reading a length is bounded metadata access. Reserve the full value
        // before retaining it or inspecting its payload/type for the program.
        let payload = match value {
            Some(AttachmentValue::Atom(atom)) => u64::try_from(atom.bytes.len())
                .map_err(|_| EffectFrameError::Read(EffectRangeReadError::BudgetExceeded))?,
            _ => 0,
        };
        let read_bytes = 64_u64
            .checked_add(payload)
            .ok_or(EffectFrameError::Read(EffectRangeReadError::BudgetExceeded))?;
        let storage = OBSERVATION_STORAGE_BYTES
            .checked_add(payload)
            .ok_or(EffectFrameError::Read(EffectRangeReadError::BudgetExceeded))?;
        if !self.charge(1, read_bytes, storage, Some(meter)) {
            return Err(EffectFrameError::Read(EffectRangeReadError::BudgetExceeded));
        }
        let attachment = AttachmentKey::node_alpha(key);
        self.readings.insert(
            SlotId::Attachment(attachment),
            SlotReading::Attachment(value.cloned()),
        );
        self.footprint.a_read.insert(attachment);
        Ok(match value {
            None => Err(ReadObstruction::Missing),
            Some(AttachmentValue::Descend(_)) => Err(ReadObstruction::AtomRequired),
            Some(AttachmentValue::Atom(atom)) if atom.type_id != expected => {
                Err(ReadObstruction::TypeMismatch)
            }
            Some(AttachmentValue::Atom(atom)) if payload > max_bytes => {
                Err(ReadObstruction::AtomTooLarge)
            }
            Some(AttachmentValue::Atom(atom)) => Ok(&atom.bytes),
        })
    }
}

pub(crate) enum NodeAperture<'a> {
    Untyped(&'a [NodeKey]),
    Typed(&'a [super::TypedNodeRead]),
}
impl NodeAperture<'_> {
    pub(crate) fn validate(&self) -> Result<(), crate::edict_read::ReadError> {
        match self {
            Self::Untyped(nodes) => crate::edict_read::ReadView::validate_aperture(nodes),
            Self::Typed(nodes)
                if nodes.len() <= 65536
                    && nodes.windows(2).all(|pair| pair[0].node < pair[1].node) =>
            {
                Ok(())
            }
            Self::Typed(_) => Err(crate::edict_read::ReadError::InvalidAperture),
        }
    }
    fn absent_type(&self, key: NodeKey) -> Option<TypeId> {
        match self {
            Self::Untyped(nodes) => nodes.binary_search(&key).ok().map(|_| TypeId([0; 32])),
            Self::Typed(nodes) => nodes
                .binary_search_by_key(&key, |node| node.node)
                .ok()
                .map(|index| nodes[index].absent_node_type),
        }
    }
}

impl EffectRangeReadSession<'_> {
    fn record_typed_atom(
        &mut self,
        key: NodeKey,
        absent_type: TypeId,
        expected: TypeId,
        max_bytes: u64,
        meter: &mut Meter,
    ) -> Result<Result<&[u8], ReadObstruction>, EffectFrameError> {
        use super::typed_reads::ReadPermission;
        self.reachable(key.warp_id, meter)?;
        let frontier = self.frontier;
        if !self.charge(1, 64, OBSERVATION_STORAGE_BYTES, Some(&mut *meter)) {
            return Err(EffectRangeReadError::BudgetExceeded.into());
        }
        let store = frontier.state().store(&key.warp_id);
        let node = store.and_then(|store| store.node(&key.local_id));
        let Some(node) = node else {
            self.authorize(SlotId::Node(key), absent_type, ReadPermission::Node, meter)?;
            self.readings
                .insert(SlotId::Node(key), SlotReading::Node(None));
            self.footprint.n_read.insert(key);
            return Ok(Err(ReadObstruction::Missing));
        };
        // Attachment kind/type/length are bounded metadata, never payload data.
        if !self.charge(1, 64, OBSERVATION_STORAGE_BYTES, Some(&mut *meter)) {
            return Err(EffectRangeReadError::BudgetExceeded.into());
        }
        let value = store.and_then(|store| store.node_attachment(&key.local_id));
        let portal = matches!(value, Some(AttachmentValue::Descend(_)));
        let permission = if portal {
            ReadPermission::Portal
        } else {
            ReadPermission::Node
        };
        self.authorize(SlotId::Node(key), node.ty, permission, meter)?;
        self.readings
            .insert(SlotId::Node(key), SlotReading::Node(Some(node.clone())));
        self.footprint.n_read.insert(key);
        let attachment = AttachmentKey::node_alpha(key);
        let (ty, permission) = match value {
            Some(AttachmentValue::Descend(_)) => (node.ty, ReadPermission::Portal),
            Some(AttachmentValue::Atom(atom)) => (atom.type_id, ReadPermission::Atom),
            None => (expected, ReadPermission::Atom),
        };
        self.authorize(SlotId::Attachment(attachment), ty, permission, meter)?;
        let payload = match value {
            Some(AttachmentValue::Atom(atom)) => {
                u64::try_from(atom.bytes.len()).map_err(|_| EffectRangeReadError::BudgetExceeded)?
            }
            _ => 0,
        };
        if !self.charge(0, payload, payload, Some(meter)) {
            return Err(EffectRangeReadError::BudgetExceeded.into());
        }
        self.readings.insert(
            SlotId::Attachment(attachment),
            SlotReading::Attachment(value.cloned()),
        );
        self.footprint.a_read.insert(attachment);
        Ok(match value {
            None => Err(ReadObstruction::Missing),
            Some(AttachmentValue::Descend(_)) => Err(ReadObstruction::AtomRequired),
            Some(AttachmentValue::Atom(atom)) if atom.type_id != expected => {
                Err(ReadObstruction::TypeMismatch)
            }
            Some(AttachmentValue::Atom(_)) if payload > max_bytes => {
                Err(ReadObstruction::AtomTooLarge)
            }
            Some(AttachmentValue::Atom(atom)) => Ok(&atom.bytes),
        })
    }
}
