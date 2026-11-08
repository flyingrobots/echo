// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Runtime-owned whole-parent mutation validation and counterfactual patch derivation.
use super::CompletedEffectEvaluation;
use crate::{EchoOperationBudgetV1, Footprint, Hash, WarpTickPatchV1};
/// First failed validation rule, before publication or scheduler composition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectDerivationKind {
    /// Admission or completed evaluation belongs to another runtime owner.
    AuthorityMismatch,
    /// The installed meaning is absent or has a different program kind.
    OperationUnavailable,
    /// Producing program/input/parent or evaluated resource bounds do not match.
    OriginMismatch,
    /// The runtime parent has changed since evaluation/admission.
    BasisChanged,
    /// Remaining delegated work or I/O cannot cover resolution.
    BudgetExceeded,
    /// An effect key has no lawful root-to-target WARP chain.
    UnreachableWarp,
    /// Effects share a target record or attachment slot.
    DuplicateEffectTarget,
    /// A carried/resolved type or endpoint pair violates the program schema.
    SchemaViolation,
    /// A content/slot identity or mutability rule does not hold.
    IdentityLawViolation,
    /// A typed permission or resolved write maximum is exceeded.
    FootprintLawViolation,
    /// Atom replacement has no owner node at the parent.
    NodeMissing,
    /// A required absence or comparison does not hold.
    PreconditionMismatch,
    /// Ensure finds different content at its target.
    ContentIdentityConflict,
    /// An introduced/retargeted edge has a missing endpoint.
    DanglingReference,
    /// Derived operations differ from their resolved write set.
    FootprintMismatch,
}
/// Failed derivation with actual admitted costs; no patch is published.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("effect derivation refused: {kind:?}")]
pub struct EffectDerivationError {
    kind: EffectDerivationKind,
    consumed: EchoOperationBudgetV1,
}
impl EffectDerivationError {
    pub(crate) const fn new(kind: EffectDerivationKind, consumed: EchoOperationBudgetV1) -> Self {
        Self { kind, consumed }
    }
    /// First failed rule.
    #[must_use]
    pub const fn kind(&self) -> EffectDerivationKind {
        self.kind
    }
    /// Successful charges after origin authentication, including private evaluation.
    /// Authority, installation and origin refusals report zero unauthenticated usage.
    #[must_use]
    pub const fn consumed(&self) -> EchoOperationBudgetV1 {
        self.consumed
    }
}
/// Counterfactual patch derived by the runtime owner at one exact full parent.
///
/// No public constructor or commit API accepts this as scheduler authority.
/// Same-Tick composition, scoped history and receipt/WAL publication remain separate.
#[derive(Debug)]
pub struct DerivedEffectPatch {
    completed: CompletedEffectEvaluation,
    patch: WarpTickPatchV1,
    footprint: Footprint,
    consumed: EchoOperationBudgetV1,
    invocation: Hash,
}
impl DerivedEffectPatch {
    /// Canonical derived operations, without applying them.
    #[must_use]
    pub const fn patch(&self) -> &WarpTickPatchV1 {
        &self.patch
    }
    /// Runtime-recorded reads and derived writes, including outgoing ranges.
    #[must_use]
    pub const fn footprint(&self) -> &Footprint {
        &self.footprint
    }
    /// Evaluation and validation charges together.
    #[must_use]
    pub const fn consumed(&self) -> EchoOperationBudgetV1 {
        self.consumed
    }
    /// Exact admitted invocation identity.
    #[must_use]
    pub const fn invocation_identity(&self) -> Hash {
        self.invocation
    }
    /// Producing evaluation origin and result.
    #[must_use]
    pub const fn completed(&self) -> &CompletedEffectEvaluation {
        &self.completed
    }
}

mod checks;
mod context;
use crate::{
    AttachmentKey, AttachmentValue, EdgeKey, EdgeRecord, NodeId, NodeKey, NodeRecord,
    OutgoingEdgeRange, SlotId, SlotReading, TypeId, WarpId,
};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn derive(
    state: &crate::WorldlineState,
    program: &super::EffectProgram,
    completed: CompletedEffectEvaluation,
    budget: EchoOperationBudgetV1,
    policy: u32,
    installed: Hash,
    invocation: Hash,
) -> Result<DerivedEffectPatch, EffectDerivationError> {
    let mut ctx = context::Context::new(state, program, &completed, budget)?;
    let mut effects = completed.mutations().iter().collect::<Vec<_>>();
    effects.sort_by(|a, b| checks::compare(a, b));
    // Whole-parent derivation refuses any changed basis at the owner crossing.
    // Dependency-scoped history and candidate composition are not enabled here.
    for effect in &effects {
        ctx.reachable(checks::key(effect).1)?;
    }
    let prior = ordered_read_slots(&ctx.read_slots)
        .map_err(|_| ctx.error(EffectDerivationKind::OriginMismatch))?;
    for slot in prior {
        let warp = match slot {
            SlotId::Node(key) => key.warp_id,
            SlotId::Edge(key) => key.warp_id,
            SlotId::OutgoingEdges(key) => key.source.warp_id,
            SlotId::Attachment(key) => match key.owner {
                crate::AttachmentOwner::Node(n) => n.warp_id,
                crate::AttachmentOwner::Edge(e) => e.warp_id,
            },
            SlotId::Port(_) => return Err(ctx.error(EffectDerivationKind::OriginMismatch)),
        };
        ctx.reachable(warp)?;
    }
    let resolved = checks::resolve(&mut ctx, &effects)?;
    let mut targets = BTreeSet::new();
    for effect in &effects {
        for slot in checks::targets(effect) {
            if !targets.insert(slot) {
                return Err(ctx.error(EffectDerivationKind::DuplicateEffectTarget));
            }
        }
    }
    checks::schema(&ctx, &resolved)?;
    checks::identities(&ctx, &resolved, &effects)?;
    let expected = checks::writes(&resolved);
    checks::law(&ctx, &resolved, &expected)?;
    checks::preconditions(&ctx, &resolved)?;
    let ops = checks::operations(&resolved);
    let mut actual = BTreeSet::new();
    for op in &ops {
        match op {
            crate::WarpOp::UpsertNode { node, .. } => {
                actual.insert(SlotId::Node(*node));
                ctx.footprint.n_write.insert(*node);
            }
            crate::WarpOp::SetAttachment { key, .. } => {
                actual.insert(SlotId::Attachment(*key));
                ctx.footprint.a_write.insert(*key);
            }
            crate::WarpOp::UpsertEdge { warp_id, record } => {
                let key = EdgeKey {
                    warp_id: *warp_id,
                    local_id: record.id,
                };
                let range = OutgoingEdgeRange {
                    source: NodeKey {
                        warp_id: *warp_id,
                        local_id: record.from,
                    },
                    edge_type: record.ty,
                };
                actual.insert(SlotId::Edge(key));
                actual.insert(SlotId::OutgoingEdges(range));
                ctx.footprint.e_write.insert(key);
                ctx.footprint.r_write.insert(range);
            }
            _ => return Err(ctx.error(EffectDerivationKind::FootprintMismatch)),
        }
    }
    if actual != expected {
        return Err(ctx.error(EffectDerivationKind::FootprintMismatch));
    }
    let count = ops.len();
    let records = |slot: &SlotId| !matches!(slot, SlotId::OutgoingEdges(_));
    let patch = WarpTickPatchV1::new(
        policy,
        installed,
        crate::TickCommitStatus::Committed,
        ctx.read_slots
            .iter()
            .filter(|s| records(s))
            .copied()
            .collect(),
        expected.iter().filter(|s| records(s)).copied().collect(),
        ops,
    );
    if patch.ops().len() != count {
        return Err(ctx.error(EffectDerivationKind::FootprintMismatch));
    }
    Ok(DerivedEffectPatch {
        completed,
        patch,
        footprint: ctx.footprint,
        consumed: ctx.meter.consumed(),
        invocation,
    })
}

fn ordered_read_slots(
    slots: &BTreeSet<SlotId>,
) -> Result<Vec<SlotId>, super::EffectReadImageError> {
    let mut ordered = slots
        .iter()
        .map(|slot| super::EffectSlotRow::from_slot(*slot).map(|row| (row, *slot)))
        .collect::<Result<Vec<_>, _>>()?;
    ordered.sort_unstable_by(|a, b| a.0.compare(&b.0));
    Ok(ordered.into_iter().map(|(_, slot)| slot).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Native SlotId orders edges before attachments. The effect contract does
    // the opposite; changing the visited order changes the first refusal.
    #[test]
    fn prior_read_reachability_uses_effect_slot_order() {
        let attachment = SlotId::Attachment(AttachmentKey::node_alpha(NodeKey {
            warp_id: WarpId([2; 32]),
            local_id: NodeId([4; 32]),
        }));
        let edge = SlotId::Edge(EdgeKey {
            warp_id: WarpId([3; 32]),
            local_id: crate::EdgeId([5; 32]),
        });
        assert_eq!(
            ordered_read_slots(&BTreeSet::from([edge, attachment])),
            Ok(vec![attachment, edge])
        );
    }
}
