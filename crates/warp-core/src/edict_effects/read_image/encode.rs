// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
use super::super::EffectRangeReadSession;
use super::{
    EffectEdgePairRow, EffectReadImageError as Error, EffectReadRow, EffectReadSetImage,
    EffectReadSetView, EffectSlotRow, ImageWord,
};
use crate::{
    AttachmentKey, AttachmentOwner, AttachmentValue, EchoOperationEvaluationBasisV1, EdgeRecord,
    SlotId, SlotReading, TypeId,
};
use bytemuck::Zeroable;

pub(in crate::edict_effects) fn export(
    source: &EffectRangeReadSession<'_>,
    basis: EchoOperationEvaluationBasisV1,
    consumed: crate::EchoOperationBudgetV1,
    allocated_bytes: u64,
    max_bytes: usize,
) -> Result<EffectReadSetImage, Error> {
    let ceiling = usize::try_from(super::super::EFFECT_READ_STORAGE_CEILING_V1)
        .map_err(|_| Error::ExportLimitExceeded)?;
    let limit = max_bytes.min(ceiling);
    if limit < 16 {
        return Err(Error::ExportLimitExceeded);
    }
    let mut rows = Vec::new();
    rows.try_reserve_exact(source.authorized_reads.len())
        .map_err(|_| Error::HostAllocationFailed)?;
    for (slot, ty) in &source.authorized_reads {
        // A permission reservation can precede a failed payload charge. Only
        // completed observations belong in this failed-frame export.
        let Some(value) = source.readings.get(slot) else {
            continue;
        };
        rows.push(prepare(source, *slot, *ty, value)?);
    }
    rows.sort_unstable_by(|a, b| a.0.compare(&b.0));
    let pair_count = rows.iter().try_fold(0_usize, |count, (_, pairs)| {
        count
            .checked_add(pairs.map_or(0, <[EdgeRecord]>::len))
            .ok_or(Error::ExportLimitExceeded)
    })?;
    let size = rows
        .len()
        .checked_mul(192)
        .and_then(|n| {
            pair_count
                .checked_mul(64)
                .and_then(|pairs| n.checked_add(pairs))
        })
        .and_then(|n| n.checked_add(16))
        .filter(|size| *size <= limit)
        .ok_or(Error::ExportLimitExceeded)?;
    let mut words = Vec::new();
    words
        .try_reserve_exact(size / 8)
        .map_err(|_| Error::HostAllocationFailed)?;
    words.push(ImageWord(
        u64::try_from(rows.len())
            .map_err(|_| Error::ExportLimitExceeded)?
            .to_le_bytes(),
    ));
    let mut cursor = 0_u64;
    for (row, pairs) in &mut rows {
        if let Some(pairs) = pairs {
            row.pairs.start_le = cursor.to_le();
            let count = u64::try_from(pairs.len()).map_err(|_| Error::ExportLimitExceeded)?;
            row.pairs.len_le = count.to_le();
            cursor = cursor
                .checked_add(count)
                .ok_or(Error::ExportLimitExceeded)?;
        }
        append(&mut words, row)?;
    }
    words.push(ImageWord(cursor.to_le_bytes()));
    for (_, pairs) in rows {
        for edge in pairs.into_iter().flatten() {
            append(
                &mut words,
                &EffectEdgePairRow {
                    edge_local_id: edge.id.0,
                    target_local_id: edge.to.0,
                },
            )?;
        }
    }
    EffectReadSetView::parse(bytemuck::cast_slice(&words))?;
    Ok(EffectReadSetImage {
        words,
        basis,
        consumed,
        allocated_bytes,
    })
}
fn append<T: bytemuck::Pod>(words: &mut Vec<ImageWord>, row: &T) -> Result<(), Error> {
    words.extend_from_slice(
        bytemuck::try_cast_slice::<T, ImageWord>(std::slice::from_ref(row))
            .map_err(|_| Error::InvalidImage)?,
    );
    Ok(())
}
fn prepare<'a>(
    source: &EffectRangeReadSession<'_>,
    slot: SlotId,
    ty: TypeId,
    value: &'a SlotReading,
) -> Result<(EffectReadRow, Option<&'a [EdgeRecord]>), Error> {
    let mut row = EffectReadRow::zeroed();
    row.slot = EffectSlotRow::from_slot(slot)?;
    row.read_type = ty.0;
    match (slot, value) {
        (SlotId::Node(_), SlotReading::Node(Some(record))) if record.ty == ty => {
            let mut hash = blake3::Hasher::new();
            hash.update(b"echo.bridge.node-value.v1\0");
            hash.update(&record.ty.0);
            row.subject = hash.finalize().into();
        }
        (SlotId::Node(_), SlotReading::Node(None))
        | (SlotId::Edge(_), SlotReading::Edge(None))
        | (SlotId::Attachment(_), SlotReading::Attachment(None)) => row.outcome = 1,
        (SlotId::Edge(key), SlotReading::Edge(Some(record)))
            if key.local_id == record.id && record.ty == ty =>
        {
            let mut hash = blake3::Hasher::new();
            hash.update(b"echo.bridge.edge-value.v1\0");
            hash.update(&record.ty.0);
            hash.update(&record.from.0);
            hash.update(&record.to.0);
            row.subject = hash.finalize().into();
        }
        (SlotId::Attachment(_), SlotReading::Attachment(Some(AttachmentValue::Atom(atom))))
            if atom.type_id == ty =>
        {
            row.subject = crate::content_address_v1::attachment_digest(atom.type_id, &atom.bytes);
        }
        (
            SlotId::Attachment(key),
            SlotReading::Attachment(Some(AttachmentValue::Descend(warp))),
        ) => {
            let AttachmentOwner::Node(owner) = key.owner else {
                return Err(Error::InconsistentObservation);
            };
            if key != AttachmentKey::node_alpha(owner)
                || !matches!(source.readings.get(&SlotId::Node(owner)),Some(SlotReading::Node(Some(record))) if record.ty==ty)
            {
                return Err(Error::InconsistentObservation);
            }
            row.outcome = 3;
            row.subject = crate::content_address_v1::descend_attachment_digest(*warp);
        }
        (SlotId::OutgoingEdges(range), SlotReading::OutgoingEdges(edges)) if ty.0 == [0; 32] => {
            let owner = source.readings.get(&SlotId::Node(range.source));
            if !matches!(
                (owner, edges),
                (Some(SlotReading::Node(Some(_))), Some(_)) | (Some(SlotReading::Node(None)), None)
            ) {
                return Err(Error::InconsistentObservation);
            }
            let edges = edges.as_deref().unwrap_or(&[]);
            if edges
                .iter()
                .any(|edge| edge.ty != range.edge_type || edge.from != range.source.local_id)
            {
                return Err(Error::InconsistentObservation);
            }
            row.outcome = 2;
            return Ok((row, Some(edges)));
        }
        _ => return Err(Error::InconsistentObservation),
    }
    Ok((row, None))
}
