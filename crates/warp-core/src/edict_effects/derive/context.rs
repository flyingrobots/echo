// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Charged immutable-parent resolutions, separate from program-view observations.
use super::{
    AttachmentKey, AttachmentValue, BTreeMap, BTreeSet, CompletedEffectEvaluation,
    EchoOperationBudgetV1, EdgeKey, EdgeRecord, EffectDerivationError, EffectDerivationKind,
    Footprint, NodeId, NodeKey, NodeRecord, OutgoingEdgeRange, SlotId, SlotReading, TypeId, WarpId,
};
use crate::{AttachmentOwner, AttachmentPlane};
pub(super) struct Context<'a> {
    pub state: &'a crate::WorldlineState,
    pub program: &'a super::super::EffectProgram,
    pub meter: crate::echo_operation::EchoOperationBudgetMeterV1,
    pub records: BTreeMap<SlotId, SlotReading>,
    pub footprint: Footprint,
    pub read_slots: BTreeSet<SlotId>,
    pub normal_nodes: BTreeSet<NodeKey>,
    pub implied_atoms: BTreeSet<AttachmentKey>,
    pub implied_edges: BTreeSet<EdgeKey>,
    pub own_nodes: BTreeMap<NodeKey, NodeRecord>,
    pub created: BTreeSet<NodeKey>,
    known: BTreeSet<SlotId>,
    reached: BTreeSet<WarpId>,
    scanned: BTreeMap<NodeKey, Vec<EdgeRecord>>,
    lengths: BTreeMap<NodeKey, usize>,
}
impl<'a> Context<'a> {
    pub fn new(
        state: &'a crate::WorldlineState,
        program: &'a super::super::EffectProgram,
        completed: &CompletedEffectEvaluation,
        budget: EchoOperationBudgetV1,
    ) -> Result<Self, EffectDerivationError> {
        let mut meter = crate::echo_operation::EchoOperationBudgetMeterV1::new(budget);
        let used = completed.read_set_image().consumed();
        if !meter.charge(used.steps(), used.read_bytes(), used.write_bytes()) {
            return Err(EffectDerivationError::new(
                EffectDerivationKind::OriginMismatch,
                used,
            ));
        }
        let view = super::super::EffectReadSetView::parse(completed.read_set_image().as_bytes())
            .map_err(|_| EffectDerivationError::new(EffectDerivationKind::OriginMismatch, used))?;
        let mut known = BTreeSet::new();
        let mut footprint = Footprint::default();
        for row in view.rows() {
            let key = NodeKey {
                warp_id: WarpId(row.slot.warp_id),
                local_id: NodeId(row.slot.local_id),
            };
            let slot = match row.slot.class {
                0 => SlotId::Node(key),
                1 => SlotId::Attachment(if row.slot.owner == 0 {
                    AttachmentKey::node_alpha(key)
                } else {
                    AttachmentKey::edge_beta(EdgeKey {
                        warp_id: key.warp_id,
                        local_id: crate::EdgeId(key.local_id.0),
                    })
                }),
                2 => SlotId::Edge(EdgeKey {
                    warp_id: key.warp_id,
                    local_id: crate::EdgeId(key.local_id.0),
                }),
                3 => SlotId::OutgoingEdges(OutgoingEdgeRange {
                    source: key,
                    edge_type: TypeId(row.slot.edge_type),
                }),
                _ => {
                    return Err(EffectDerivationError::new(
                        EffectDerivationKind::OriginMismatch,
                        used,
                    ))
                }
            };
            mark_read(&mut footprint, slot);
            known.insert(slot);
        }
        Ok(Self {
            state,
            program,
            meter,
            records: BTreeMap::new(),
            footprint,
            read_slots: known.clone(),
            known,
            normal_nodes: BTreeSet::new(),
            implied_atoms: BTreeSet::new(),
            implied_edges: BTreeSet::new(),
            own_nodes: BTreeMap::new(),
            created: BTreeSet::new(),
            reached: BTreeSet::new(),
            scanned: BTreeMap::new(),
            lengths: BTreeMap::new(),
        })
    }
    pub fn error(&self, kind: EffectDerivationKind) -> EffectDerivationError {
        EffectDerivationError::new(kind, self.meter.consumed())
    }
    pub fn charge(&mut self, steps: u64, bytes: u64) -> Result<(), EffectDerivationError> {
        if self.meter.charge(steps, bytes, 0) {
            Ok(())
        } else {
            Err(self.error(EffectDerivationKind::BudgetExceeded))
        }
    }
    fn read(&mut self, slot: SlotId) -> Result<SlotReading, EffectDerivationError> {
        if let Some(value) = self.records.get(&slot) {
            return Ok(value.clone());
        }
        let cost = if matches!(slot, SlotId::Edge(_)) {
            128
        } else {
            64
        };
        let fresh = !self.known.contains(&slot);
        if fresh {
            self.charge(1, cost)?;
        }
        // Length/type metadata is inspected before any payload is retained or hashed.
        let value = match slot {
            SlotId::Node(key) => SlotReading::Node(
                self.state
                    .store(&key.warp_id)
                    .and_then(|s| s.node(&key.local_id))
                    .cloned(),
            ),
            SlotId::Edge(key) => SlotReading::Edge(
                self.state
                    .store(&key.warp_id)
                    .and_then(|s| s.edge_record(&key.local_id)),
            ),
            SlotId::Attachment(key) => {
                let value = match key.owner {
                    AttachmentOwner::Node(node) => self
                        .state
                        .store(&node.warp_id)
                        .and_then(|s| s.node_attachment(&node.local_id)),
                    AttachmentOwner::Edge(edge) => self
                        .state
                        .store(&edge.warp_id)
                        .and_then(|s| s.edge_attachment(&edge.local_id)),
                };
                if fresh {
                    if let Some(AttachmentValue::Atom(atom)) = value {
                        self.charge(0, atom.bytes.len() as u64)?;
                    }
                }
                let result = SlotReading::Attachment(value.cloned());
                self.records.insert(slot, result.clone());
                self.read_slots.insert(slot);
                mark_read(&mut self.footprint, slot);
                return Ok(result);
            }
            _ => return Err(self.error(EffectDerivationKind::OriginMismatch)),
        };
        self.records.insert(slot, value.clone());
        self.read_slots.insert(slot);
        mark_read(&mut self.footprint, slot);
        Ok(value)
    }
    pub fn node(&mut self, key: NodeKey) -> Result<Option<NodeRecord>, EffectDerivationError> {
        self.normal_nodes.insert(key);
        match self.read(SlotId::Node(key))? {
            SlotReading::Node(value) => Ok(value),
            _ => Err(self.error(EffectDerivationKind::OriginMismatch)),
        }
    }
    pub fn alpha(
        &mut self,
        key: NodeKey,
    ) -> Result<Option<AttachmentValue>, EffectDerivationError> {
        let slot = AttachmentKey::node_alpha(key);
        self.implied_atoms.insert(slot);
        match self.read(SlotId::Attachment(slot))? {
            SlotReading::Attachment(value) => Ok(value),
            _ => Err(self.error(EffectDerivationKind::OriginMismatch)),
        }
    }
    pub fn edge(&mut self, key: EdgeKey) -> Result<Option<EdgeRecord>, EffectDerivationError> {
        self.implied_edges.insert(key);
        match self.read(SlotId::Edge(key))? {
            SlotReading::Edge(value) => Ok(value),
            _ => Err(self.error(EffectDerivationKind::OriginMismatch)),
        }
    }
    pub fn endpoint(&mut self, key: NodeKey) -> Result<Option<NodeRecord>, EffectDerivationError> {
        if let Some(record) = self.own_nodes.get(&key) {
            return Ok(Some(record.clone()));
        }
        self.node(key)
    }
    pub fn empty_bucket(&mut self, key: NodeKey) -> Result<(), EffectDerivationError> {
        if !self.lengths.contains_key(&key) {
            self.charge(1, 64)?;
            let size = self
                .state
                .store(&key.warp_id)
                .map_or(0, |s| s.edges_from(&key.local_id).len());
            self.lengths.insert(key, size);
        }
        if self.lengths[&key] != 0 {
            return Err(self.error(EffectDerivationKind::PreconditionMismatch));
        }
        Ok(())
    }
    pub fn bucket(
        &mut self,
        key: NodeKey,
        ty: Option<TypeId>,
    ) -> Result<Vec<EdgeRecord>, EffectDerivationError> {
        if let Some(ty) = ty {
            self.footprint.r_read.insert(OutgoingEdgeRange {
                source: key,
                edge_type: ty,
            });
        }
        if let Some(edges) = self.scanned.get(&key) {
            return Ok(edges.clone());
        }
        let total = self
            .state
            .store(&key.warp_id)
            .map_or(0, |s| s.edges_from(&key.local_id).len());
        // Known record IDs are resolved through the index, without inspecting
        // any unknown bucket record before its whole scan reservation.
        let mut cached = 0usize;
        let mut known = self
            .known
            .iter()
            .filter_map(|slot| {
                if let SlotId::Edge(edge) = slot {
                    Some(*edge)
                } else {
                    None
                }
            })
            .collect::<BTreeSet<_>>();
        known.extend(self.records.keys().filter_map(|slot| {
            if let SlotId::Edge(edge) = slot {
                Some(*edge)
            } else {
                None
            }
        }));
        for edge in known {
            if edge.warp_id == key.warp_id
                && self
                    .state
                    .store(&key.warp_id)
                    .and_then(|s| s.edge_record(&edge.local_id))
                    .is_some_and(|r| r.from == key.local_id)
            {
                cached += 1;
            }
        }
        let unknown = total
            .checked_sub(cached)
            .ok_or_else(|| self.error(EffectDerivationKind::OriginMismatch))?
            as u64;
        self.charge(
            unknown,
            unknown
                .checked_mul(128)
                .ok_or_else(|| self.error(EffectDerivationKind::BudgetExceeded))?,
        )?;
        let edges = self.state.store(&key.warp_id).map_or_else(Vec::new, |s| {
            s.edges_from(&key.local_id).cloned().collect::<Vec<_>>()
        });
        for edge in &edges {
            let slot = SlotId::Edge(EdgeKey {
                warp_id: key.warp_id,
                local_id: edge.id,
            });
            self.implied_edges.insert(EdgeKey {
                warp_id: key.warp_id,
                local_id: edge.id,
            });
            self.read_slots.insert(slot);
            mark_read(&mut self.footprint, slot);
            self.records
                .insert(slot, SlotReading::Edge(Some(edge.clone())));
            self.footprint.r_read.insert(OutgoingEdgeRange {
                source: key,
                edge_type: edge.ty,
            });
        }
        self.scanned.insert(key, edges.clone());
        Ok(edges)
    }
    pub fn reachable(&mut self, target: WarpId) -> Result<(), EffectDerivationError> {
        let root = self.state.root().warp_id;
        if target == root || self.reached.contains(&target) {
            return Ok(());
        }
        let mut cursor = target;
        let mut seen = BTreeSet::new();
        let mut chain = Vec::new();
        while cursor != root && !self.reached.contains(&cursor) {
            if !seen.insert(cursor) {
                return Err(self.error(EffectDerivationKind::UnreachableWarp));
            }
            let parent = self
                .state
                .warp_state()
                .instance(&cursor)
                .and_then(|i| i.parent)
                .ok_or_else(|| self.error(EffectDerivationKind::UnreachableWarp))?;
            let AttachmentOwner::Node(owner) = parent.owner else {
                return Err(self.error(EffectDerivationKind::FootprintLawViolation));
            };
            if parent.plane != AttachmentPlane::Alpha {
                return Err(self.error(EffectDerivationKind::FootprintLawViolation));
            }
            chain.push((cursor, owner));
            cursor = owner.warp_id;
        }
        for (child, owner) in chain.into_iter().rev() {
            let SlotReading::Node(Some(record)) = self.read(SlotId::Node(owner))? else {
                return Err(self.error(EffectDerivationKind::UnreachableWarp));
            };
            if !self.program.footprint_law().portals.contains(&record.ty) {
                return Err(self.error(EffectDerivationKind::FootprintLawViolation));
            }
            if self.read(SlotId::Attachment(AttachmentKey::node_alpha(owner)))?
                != SlotReading::Attachment(Some(AttachmentValue::Descend(child)))
            {
                return Err(self.error(EffectDerivationKind::UnreachableWarp));
            }
            self.reached.insert(child);
        }
        Ok(())
    }
}
fn mark_read(footprint: &mut Footprint, slot: SlotId) {
    match slot {
        SlotId::Node(key) => {
            footprint.n_read.insert(key);
        }
        SlotId::Edge(key) => {
            footprint.e_read.insert(key);
        }
        SlotId::Attachment(key) => {
            footprint.a_read.insert(key);
        }
        SlotId::OutgoingEdges(key) => {
            footprint.r_read.insert(key);
        }
        SlotId::Port(_) => {}
    }
}
