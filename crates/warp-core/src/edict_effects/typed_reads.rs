// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Typed resolutions and root-first portal authority for an immutable frame.
use super::{EffectRangeReadError as Error, EffectRangeReadSession, OBSERVATION_STORAGE_BYTES};
use crate::edict_pure::evaluate::Meter;
use crate::{
    AttachmentKey, AttachmentOwner, AttachmentPlane, AttachmentValue, NodeKey, NodeRecord, SlotId,
    SlotReading, TypeId, WarpId,
};
use std::collections::BTreeSet;

/// Node aperture declaration for a typed recorded-read component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypedNodeRead {
    /// Exact node key selected by the trusted host.
    pub node: NodeKey,
    /// Type authorizing absence only; present records use their actual type.
    pub absent_node_type: TypeId,
}

pub(super) enum ReadPermission {
    Node,
    Atom,
    Portal,
}

impl EffectRangeReadSession<'_> {
    pub(super) fn authorize(
        &mut self,
        slot: SlotId,
        ty: TypeId,
        permission: ReadPermission,
        meter: &mut Meter,
    ) -> Result<(), Error> {
        let law = self.law.as_ref().ok_or(Error::FootprintLawViolation)?;
        let allowed = match permission {
            ReadPermission::Node => law.nodes.contains(&ty),
            ReadPermission::Atom => law.atoms.contains(&ty),
            ReadPermission::Portal => law.portals.contains(&ty),
        };
        if !allowed {
            return Err(Error::FootprintLawViolation);
        }
        self.retain_authorization(slot, ty, meter)
    }

    pub(super) fn retain_authorization(
        &mut self,
        slot: SlotId,
        ty: TypeId,
        meter: &mut Meter,
    ) -> Result<(), Error> {
        if self.authorized_reads.contains(&(slot, ty)) {
            return Ok(());
        }
        let law = self.law.as_ref().ok_or(Error::FootprintLawViolation)?;
        if self.authorized_reads.len() as u64 >= law.max_reads {
            return Err(Error::FootprintLawViolation);
        }
        if !self.charge(0, 0, 64, Some(meter)) {
            return Err(Error::BudgetExceeded);
        }
        self.authorized_reads.insert((slot, ty));
        Ok(())
    }

    pub(super) fn typed_node(
        &mut self,
        key: NodeKey,
        absent_type: TypeId,
        meter: &mut Meter,
    ) -> Result<Option<NodeRecord>, Error> {
        self.reachable(key.warp_id, meter)?;
        if !self.charge(1, 64, OBSERVATION_STORAGE_BYTES, Some(&mut *meter)) {
            return Err(Error::BudgetExceeded);
        }
        let value = self
            .frontier
            .state()
            .store(&key.warp_id)
            .and_then(|store| store.node(&key.local_id))
            .cloned();
        let ty = value.as_ref().map_or(absent_type, |node| node.ty);
        self.authorize(SlotId::Node(key), ty, ReadPermission::Node, meter)?;
        self.readings
            .insert(SlotId::Node(key), SlotReading::Node(value.clone()));
        self.footprint.n_read.insert(key);
        Ok(value)
    }

    pub(super) fn reachable(&mut self, target: WarpId, meter: &mut Meter) -> Result<(), Error> {
        let root = self.frontier.state().root().warp_id;
        if target == root || self.validated_warps.contains(&target) {
            return Ok(());
        }
        let mut current = target;
        let mut seen = BTreeSet::new();
        let mut chain = Vec::new();
        while current != root && !self.validated_warps.contains(&current) {
            // Reverse instance metadata is bounded preparation for root-first
            // resolutions. Each hop reserves both vector and cycle-index storage.
            if !self.charge(1, 64, 256, Some(&mut *meter)) {
                return Err(Error::BudgetExceeded);
            }
            if !seen.insert(current) {
                return Err(Error::UnreachableWarp);
            }
            let parent = self
                .frontier
                .state()
                .warp_state()
                .instance(&current)
                .and_then(|instance| instance.parent)
                .ok_or(Error::UnreachableWarp)?;
            let AttachmentOwner::Node(owner) = parent.owner else {
                return Err(Error::FootprintLawViolation);
            };
            if parent.plane != AttachmentPlane::Alpha {
                return Err(Error::FootprintLawViolation);
            }
            chain
                .try_reserve(1)
                .map_err(|_| Error::HostAllocationFailed)?;
            chain.push((current, owner));
            current = owner.warp_id;
        }
        for (child, owner) in chain.into_iter().rev() {
            self.portal(owner, child, meter)?;
            self.validated_warps.insert(child);
        }
        Ok(())
    }

    fn portal(&mut self, owner: NodeKey, child: WarpId, meter: &mut Meter) -> Result<(), Error> {
        if !self.charge(1, 64, OBSERVATION_STORAGE_BYTES, Some(&mut *meter)) {
            return Err(Error::BudgetExceeded);
        }
        let ty = self
            .frontier
            .state()
            .store(&owner.warp_id)
            .and_then(|store| store.node(&owner.local_id))
            .ok_or(Error::UnreachableWarp)?
            .ty;
        self.authorize(SlotId::Node(owner), ty, ReadPermission::Portal, meter)?;
        self.readings.insert(
            SlotId::Node(owner),
            SlotReading::Node(Some(NodeRecord { ty })),
        );
        self.footprint.n_read.insert(owner);
        if !self.charge(1, 64, OBSERVATION_STORAGE_BYTES, Some(&mut *meter)) {
            return Err(Error::BudgetExceeded);
        }
        let key = AttachmentKey::node_alpha(owner);
        if self
            .frontier
            .state()
            .store(&owner.warp_id)
            .and_then(|store| store.node_attachment(&owner.local_id))
            != Some(&AttachmentValue::Descend(child))
        {
            return Err(Error::UnreachableWarp);
        }
        self.authorize(SlotId::Attachment(key), ty, ReadPermission::Portal, meter)?;
        self.readings.insert(
            SlotId::Attachment(key),
            SlotReading::Attachment(Some(AttachmentValue::Descend(child))),
        );
        self.footprint.a_read.insert(key);
        Ok(())
    }
}
