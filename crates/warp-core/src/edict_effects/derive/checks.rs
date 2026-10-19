// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Ordered resolution, law checks and operation derivation for all five effects.
use super::super::{EffectMutation as M, EffectNodeAddressing as Addressing};
use super::context::Context;
use super::{
    AttachmentKey, AttachmentValue, BTreeSet, EdgeRecord, EffectDerivationError,
    EffectDerivationKind, Hash, NodeId, NodeKey, NodeRecord, OutgoingEdgeRange, SlotId,
    SlotReading, TypeId, WarpId,
};
pub(super) struct Resolved<'a> {
    effect: &'a M,
    node: Option<NodeRecord>,
    atom: Option<AttachmentValue>,
    edge: Option<EdgeRecord>,
    source: Option<NodeRecord>,
    target: Option<NodeRecord>,
    alias: bool,
    noop: bool,
}
pub(super) fn key(effect: &M) -> (u8, WarpId, Hash) {
    match effect {
        M::EnsureNode { node, .. } | M::CreateNode { node, .. } => {
            (0, node.warp_id, node.local_id.0)
        }
        M::ReplaceAtom { node, .. } => (1, node.warp_id, node.local_id.0),
        M::EnsureEdge { edge, .. } | M::ReplaceEdge { edge, .. } => {
            (2, edge.warp_id, edge.local_id.0)
        }
    }
}
fn row(effect: &M) -> ([u8; 280], &[u8]) {
    let mut row = [0; 280];
    let (_, warp, id) = key(effect);
    row[8..40].copy_from_slice(&warp.0);
    row[40..72].copy_from_slice(&id);
    let payload = match effect {
        M::EnsureNode { ty, atom, .. } | M::CreateNode { ty, atom, .. } => {
            row[0] = u8::from(matches!(effect, M::CreateNode { .. }));
            row[72..104].copy_from_slice(&ty.0);
            row[104..136].copy_from_slice(&atom.type_id.0);
            atom.bytes.as_ref()
        }
        M::ReplaceAtom { expected, atom, .. } => {
            row[0] = 2;
            row[104..136].copy_from_slice(&atom.type_id.0);
            row[152..184].copy_from_slice(expected);
            atom.bytes.as_ref()
        }
        M::EnsureEdge { from, to, ty, .. } => {
            row[0] = 3;
            row[72..104].copy_from_slice(&ty.0);
            row[184..216].copy_from_slice(&from.0);
            row[216..248].copy_from_slice(&to.0);
            &[]
        }
        M::ReplaceEdge {
            expected_to, to, ..
        } => {
            row[0] = 4;
            row[216..248].copy_from_slice(&to.0);
            row[248..280].copy_from_slice(&expected_to.0);
            &[]
        }
    };
    (row, payload)
}
pub(super) fn compare(a: &M, b: &M) -> std::cmp::Ordering {
    key(a).cmp(&key(b)).then_with(|| {
        let a = row(a);
        let b = row(b);
        a.0.cmp(&b.0).then_with(|| a.1.cmp(b.1))
    })
}
pub(super) fn targets(effect: &M) -> Vec<SlotId> {
    match effect {
        M::EnsureNode { node, .. } | M::CreateNode { node, .. } => vec![
            SlotId::Node(*node),
            SlotId::Attachment(AttachmentKey::node_alpha(*node)),
        ],
        M::ReplaceAtom { node, .. } => vec![SlotId::Attachment(AttachmentKey::node_alpha(*node))],
        M::EnsureEdge { edge, .. } | M::ReplaceEdge { edge, .. } => vec![SlotId::Edge(*edge)],
    }
}
pub(super) fn resolve<'e>(
    ctx: &mut Context<'_>,
    effects: &[&'e M],
) -> Result<Vec<Resolved<'e>>, EffectDerivationError> {
    let mut resolved = Vec::new();
    for effect in effects {
        let mut r = Resolved {
            effect,
            node: None,
            atom: None,
            edge: None,
            source: None,
            target: None,
            alias: false,
            noop: false,
        };
        match effect {
            M::EnsureNode { node, ty, atom } | M::CreateNode { node, ty, atom } => {
                r.node = ctx.node(*node)?;
                r.atom = ctx.alpha(*node)?;
                r.noop = matches!(&r.node,Some(record) if record.ty==*ty)
                    && r.atom == Some(AttachmentValue::Atom(atom.clone()));
                if matches!(effect, M::EnsureNode { .. })
                    && ctx.program.graph_schema().node_addressing(*ty) == Some(Addressing::Content)
                    && r.node.is_some()
                {
                    if !r.noop {
                        return Err(ctx.error(EffectDerivationKind::ContentIdentityConflict));
                    }
                    let mut actual = ctx.bucket(*node, None)?;
                    let mut planned = edges_for(node, effects);
                    actual.sort_by_key(|e| (e.ty, e.to, e.id));
                    planned.sort_by_key(|e| (e.ty, e.to, e.id));
                    if actual != planned {
                        return Err(ctx.error(EffectDerivationKind::ContentIdentityConflict));
                    }
                }
                if r.node.is_none() && r.atom.is_none() {
                    ctx.empty_bucket(*node)?;
                    ctx.created.insert(*node);
                    ctx.own_nodes.insert(*node, NodeRecord { ty: *ty });
                }
            }
            M::ReplaceAtom { node, .. } => {
                r.node = ctx.node(*node)?;
                if r.node.is_none() {
                    return Err(ctx.error(EffectDerivationKind::NodeMissing));
                }
                r.atom = ctx.alpha(*node)?;
            }
            M::EnsureEdge { edge, from, to, ty } => {
                r.edge = ctx.edge(*edge)?;
                r.noop = r
                    .edge
                    .as_ref()
                    .is_some_and(|old| old.from == *from && old.to == *to && old.ty == *ty);
                let source = r.edge.as_ref().map_or(*from, |old| old.from);
                let edge_type = r.edge.as_ref().map_or(*ty, |old| old.ty);
                let source_key = NodeKey {
                    warp_id: edge.warp_id,
                    local_id: source,
                };
                r.source = ctx.endpoint(source_key)?;
                if r.edge.is_none() {
                    r.target = ctx.endpoint(NodeKey {
                        warp_id: edge.warp_id,
                        local_id: *to,
                    })?;
                }
                if r.source.as_ref().is_some_and(|n| {
                    ctx.program.graph_schema().node_addressing(n.ty) == Some(Addressing::Mutable)
                }) {
                    r.alias = ctx
                        .bucket(source_key, Some(edge_type))?
                        .iter()
                        .any(|old| old.ty == edge_type && old.id != edge.local_id);
                }
            }
            M::ReplaceEdge { edge, to, .. } => {
                let old = ctx
                    .edge(*edge)?
                    .ok_or_else(|| ctx.error(EffectDerivationKind::PreconditionMismatch))?;
                let source = NodeKey {
                    warp_id: edge.warp_id,
                    local_id: old.from,
                };
                r.source = ctx.endpoint(source)?;
                r.target = ctx.endpoint(NodeKey {
                    warp_id: edge.warp_id,
                    local_id: *to,
                })?;
                if r.source.as_ref().is_some_and(|n| {
                    ctx.program.graph_schema().node_addressing(n.ty) == Some(Addressing::Mutable)
                }) {
                    r.alias = ctx
                        .bucket(source, Some(old.ty))?
                        .iter()
                        .any(|other| other.ty == old.ty && other.id != old.id);
                }
                r.edge = Some(old);
            }
        }
        resolved.push(r);
    }
    Ok(resolved)
}
fn edges_for(node: &NodeKey, effects: &[&M]) -> Vec<EdgeRecord> {
    effects
        .iter()
        .filter_map(|e| match e {
            M::EnsureEdge { edge, from, to, ty }
                if edge.warp_id == node.warp_id && *from == node.local_id =>
            {
                Some(EdgeRecord {
                    id: edge.local_id,
                    from: *from,
                    to: *to,
                    ty: *ty,
                })
            }
            _ => None,
        })
        .collect()
}
pub(super) fn schema(
    ctx: &Context<'_>,
    resolved: &[Resolved<'_>],
) -> Result<(), EffectDerivationError> {
    let schema = ctx.program.graph_schema();
    let bad = || ctx.error(EffectDerivationKind::SchemaViolation);
    for key in &ctx.normal_nodes {
        if let Some(SlotReading::Node(Some(node))) = ctx.records.get(&SlotId::Node(*key)) {
            if schema.node_addressing(node.ty).is_none() {
                return Err(bad());
            }
        }
    }
    for key in &ctx.implied_atoms {
        if let (
            crate::AttachmentOwner::Node(owner),
            Some(SlotReading::Attachment(Some(AttachmentValue::Atom(atom)))),
        ) = (key.owner, ctx.records.get(&SlotId::Attachment(*key)))
        {
            if let Some(SlotReading::Node(Some(node))) = ctx.records.get(&SlotId::Node(owner)) {
                if !schema.allows_atom(node.ty, atom.type_id) {
                    return Err(bad());
                }
            }
        }
    }
    for key in &ctx.implied_edges {
        if let Some(SlotReading::Edge(Some(edge))) = ctx.records.get(&SlotId::Edge(*key)) {
            if !schema.has_edge_type(edge.ty) {
                return Err(bad());
            }
        }
    }
    for r in resolved {
        match r.effect {
            M::EnsureNode { ty, atom, .. } | M::CreateNode { ty, atom, .. } => {
                if !schema.allows_atom(*ty, atom.type_id) {
                    return Err(bad());
                }
            }
            M::ReplaceAtom { atom, .. } => {
                if !r
                    .node
                    .as_ref()
                    .is_some_and(|n| schema.allows_atom(n.ty, atom.type_id))
                {
                    return Err(bad());
                }
            }
            M::EnsureEdge { ty, .. } => {
                if !schema.has_edge_type(*ty) {
                    return Err(bad());
                }
                if r.edge.is_none() {
                    if let (Some(from), Some(to)) = (&r.source, &r.target) {
                        if !schema.allows_edge(*ty, from.ty, to.ty) {
                            return Err(bad());
                        }
                    }
                }
            }
            M::ReplaceEdge { .. } => {
                if let (Some(edge), Some(from), Some(to)) = (&r.edge, &r.source, &r.target) {
                    if !schema.allows_edge(edge.ty, from.ty, to.ty) {
                        return Err(bad());
                    }
                }
            }
        }
    }
    Ok(())
}
pub(super) fn identities(
    ctx: &Context<'_>,
    resolved: &[Resolved<'_>],
    effects: &[&M],
) -> Result<(), EffectDerivationError> {
    let bad = || ctx.error(EffectDerivationKind::IdentityLawViolation);
    let schema = ctx.program.graph_schema();
    for r in resolved {
        match r.effect {
            M::EnsureNode { node, ty, atom } => {
                if schema.node_addressing(*ty) != Some(Addressing::Content) {
                    return Err(bad());
                }
                let mut edges = edges_for(node, effects);
                edges.sort_by_key(|e| (e.ty, e.to));
                let mut hash = blake3::Hasher::new();
                hash.update(b"echo.bridge.node-id.v1\0");
                hash.update(&node.warp_id.0);
                hash.update(&ty.0);
                hash.update(&super::super::value_digest::atom(atom));
                hash.update(&(edges.len() as u64).to_le_bytes());
                for edge in edges {
                    hash.update(&edge.ty.0);
                    hash.update(&edge.to.0);
                }
                if hash.finalize().as_bytes() != &node.local_id.0 {
                    return Err(bad());
                }
            }
            M::CreateNode { ty, .. } => {
                if schema.node_addressing(*ty) != Some(Addressing::Mutable) {
                    return Err(bad());
                }
            }
            M::ReplaceAtom { .. } => {
                if !r
                    .node
                    .as_ref()
                    .is_some_and(|n| schema.node_addressing(n.ty) == Some(Addressing::Mutable))
                {
                    return Err(bad());
                }
            }
            M::EnsureEdge { edge, from, to, ty } => {
                if let Some(source) = &r.source {
                    let (from, to, ty) = r
                        .edge
                        .as_ref()
                        .map_or((*from, *to, *ty), |old| (old.from, old.to, old.ty));
                    let address = schema.node_addressing(source.ty).ok_or_else(bad)?;
                    if address == Addressing::Content
                        && r.edge.is_none()
                        && !ctx.created.contains(&NodeKey {
                            warp_id: edge.warp_id,
                            local_id: from,
                        })
                    {
                        return Err(bad());
                    }
                    if r.alias || edge_id(edge.warp_id, from, ty, to, address) != edge.local_id.0 {
                        return Err(bad());
                    }
                }
            }
            M::ReplaceEdge { edge, .. } => {
                if let (Some(source), Some(old)) = (&r.source, &r.edge) {
                    if schema.node_addressing(source.ty) != Some(Addressing::Mutable)
                        || r.alias
                        || edge_id(edge.warp_id, old.from, old.ty, old.to, Addressing::Mutable)
                            != edge.local_id.0
                    {
                        return Err(bad());
                    }
                }
            }
        }
    }
    Ok(())
}
fn edge_id(warp: WarpId, from: NodeId, ty: TypeId, to: NodeId, kind: Addressing) -> Hash {
    let mut hash = blake3::Hasher::new();
    hash.update(if kind == Addressing::Content {
        b"echo.bridge.edge-id.v1\0"
    } else {
        b"echo.bridge.slot-edge-id.v1\0"
    });
    hash.update(&warp.0);
    hash.update(&from.0);
    hash.update(&ty.0);
    if kind == Addressing::Content {
        hash.update(&to.0);
    }
    hash.finalize().into()
}
pub(super) fn writes(resolved: &[Resolved<'_>]) -> BTreeSet<SlotId> {
    let mut slots = BTreeSet::new();
    for r in resolved {
        match r.effect {
            M::EnsureNode { .. } | M::EnsureEdge { .. } if r.noop => {}
            M::EnsureEdge { edge, from, ty, .. } => {
                slots.insert(SlotId::Edge(*edge));
                if r.edge.is_none() {
                    slots.insert(SlotId::OutgoingEdges(OutgoingEdgeRange {
                        source: NodeKey {
                            warp_id: edge.warp_id,
                            local_id: *from,
                        },
                        edge_type: *ty,
                    }));
                }
            }
            M::ReplaceEdge { edge, .. } => {
                if let Some(old) = &r.edge {
                    slots.insert(SlotId::Edge(*edge));
                    slots.insert(SlotId::OutgoingEdges(OutgoingEdgeRange {
                        source: NodeKey {
                            warp_id: edge.warp_id,
                            local_id: old.from,
                        },
                        edge_type: old.ty,
                    }));
                }
            }
            other => slots.extend(targets(other)),
        }
    }
    slots
}
pub(super) fn law(
    ctx: &Context<'_>,
    resolved: &[Resolved<'_>],
    writes: &BTreeSet<SlotId>,
) -> Result<(), EffectDerivationError> {
    let law = ctx.program.footprint_law();
    let bad = || ctx.error(EffectDerivationKind::FootprintLawViolation);
    if writes.len() as u64 > law.max_writes {
        return Err(bad());
    }
    for key in &ctx.normal_nodes {
        if let Some(SlotReading::Node(Some(n))) = ctx.records.get(&SlotId::Node(*key)) {
            if !law.nodes.contains(&n.ty) {
                return Err(bad());
            }
        }
    }
    for key in &ctx.implied_atoms {
        if let Some(SlotReading::Attachment(Some(AttachmentValue::Atom(atom)))) =
            ctx.records.get(&SlotId::Attachment(*key))
        {
            if !law.atoms.contains(&atom.type_id) {
                return Err(bad());
            }
        }
    }
    for key in &ctx.implied_edges {
        if let Some(SlotReading::Edge(Some(edge))) = ctx.records.get(&SlotId::Edge(*key)) {
            if !law.write_edges.contains(&edge.ty) && !law.ranges.contains(&edge.ty) {
                return Err(bad());
            }
        }
    }
    for r in resolved {
        match r.effect {
            M::EnsureNode { ty, atom, .. } | M::CreateNode { ty, atom, .. } => {
                if !law.write_nodes.contains(ty) || !law.atoms.contains(&atom.type_id) {
                    return Err(bad());
                }
            }
            M::ReplaceAtom { atom, .. } => {
                if !law.atoms.contains(&atom.type_id) {
                    return Err(bad());
                }
            }
            M::EnsureEdge { ty, .. } => {
                if !law.write_edges.contains(ty) {
                    return Err(bad());
                }
            }
            M::ReplaceEdge { .. } => {
                if !r
                    .edge
                    .as_ref()
                    .is_some_and(|e| law.write_edges.contains(&e.ty))
                {
                    return Err(bad());
                }
            }
        }
    }
    Ok(())
}
pub(super) fn preconditions(
    ctx: &Context<'_>,
    resolved: &[Resolved<'_>],
) -> Result<(), EffectDerivationError> {
    for r in resolved {
        let kind = match r.effect {
            M::CreateNode { .. } if r.node.is_some() || r.atom.is_some() => {
                Some(EffectDerivationKind::PreconditionMismatch)
            }
            M::EnsureNode { ty, atom, .. }
                if !(r.node.is_none() && r.atom.is_none()
                    || r.node.as_ref().is_some_and(|n| n.ty == *ty)
                        && r.atom == Some(AttachmentValue::Atom(atom.clone()))) =>
            {
                Some(EffectDerivationKind::ContentIdentityConflict)
            }
            M::ReplaceAtom { expected, .. } if !matches!(&r.atom,Some(AttachmentValue::Atom(atom)) if super::super::value_digest::atom(atom)==*expected) => {
                Some(EffectDerivationKind::PreconditionMismatch)
            }
            M::EnsureEdge { .. } if r.edge.is_some() && !r.noop => {
                Some(EffectDerivationKind::ContentIdentityConflict)
            }
            M::ReplaceEdge { expected_to, .. }
                if r.edge.as_ref().is_none_or(|e| e.to != *expected_to) =>
            {
                Some(EffectDerivationKind::PreconditionMismatch)
            }
            M::EnsureEdge { .. }
                if r.source.is_none() || (r.edge.is_none() && r.target.is_none()) =>
            {
                Some(EffectDerivationKind::DanglingReference)
            }
            M::ReplaceEdge { .. } if r.source.is_none() || r.target.is_none() => {
                Some(EffectDerivationKind::DanglingReference)
            }
            _ => None,
        };
        if let Some(kind) = kind {
            return Err(ctx.error(kind));
        }
    }
    Ok(())
}
pub(super) fn operations(resolved: &[Resolved<'_>]) -> Vec<crate::WarpOp> {
    let mut ops = Vec::new();
    for r in resolved {
        match r.effect {
            M::EnsureNode { .. } | M::EnsureEdge { .. } if r.noop => {}
            M::EnsureNode { node, ty, atom } | M::CreateNode { node, ty, atom } => {
                ops.push(crate::WarpOp::UpsertNode {
                    node: *node,
                    record: NodeRecord { ty: *ty },
                });
                ops.push(crate::WarpOp::SetAttachment {
                    key: AttachmentKey::node_alpha(*node),
                    value: Some(AttachmentValue::Atom(atom.clone())),
                });
            }
            M::ReplaceAtom { node, atom, .. } => ops.push(crate::WarpOp::SetAttachment {
                key: AttachmentKey::node_alpha(*node),
                value: Some(AttachmentValue::Atom(atom.clone())),
            }),
            M::EnsureEdge { edge, from, to, ty } => ops.push(crate::WarpOp::UpsertEdge {
                warp_id: edge.warp_id,
                record: EdgeRecord {
                    id: edge.local_id,
                    from: *from,
                    to: *to,
                    ty: *ty,
                },
            }),
            M::ReplaceEdge { edge, to, .. } => {
                if let Some(old) = &r.edge {
                    ops.push(crate::WarpOp::UpsertEdge {
                        warp_id: edge.warp_id,
                        record: EdgeRecord {
                            to: *to,
                            ..old.clone()
                        },
                    });
                }
            }
        }
    }
    ops
}
