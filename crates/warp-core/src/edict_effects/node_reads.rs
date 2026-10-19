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
    pub(crate) aperture: &'r [NodeKey],
}

impl NodeAtomAccess for RecordedNodeAccess<'_, '_> {
    type Error = EffectFrameError;
    fn basis(&self) -> ReadBasis {
        self.reads.basis()
    }
    fn contains(&self, node: NodeKey) -> bool {
        self.aperture.binary_search(&node).is_ok()
    }
    fn atom(
        &mut self,
        node: NodeKey,
        expected: TypeId,
        max_bytes: u64,
        meter: &mut Meter,
    ) -> Result<Result<&[u8], ReadObstruction>, Self::Error> {
        self.reads.record_atom(node, expected, max_bytes, meter)
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
