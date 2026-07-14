// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Canonical retained encoding for replayable local-commit provenance.

use bytes::Bytes;
use thiserror::Error;

use crate::{
    attachment::{AtomPayload, AttachmentKey, AttachmentOwner, AttachmentPlane, AttachmentValue},
    clock::{GlobalTick, WorldlineTick},
    compute_commit_hash_v2,
    head::{HeadId, WriterHeadKey},
    ident::{EdgeId, EdgeKey, Hash, NodeId, NodeKey, TypeId, WarpId},
    provenance_store::{ProvenanceEntry, ProvenanceEventKind, ProvenanceRef},
    receipt::{TickReceipt, TickReceiptDisposition, TickReceiptEntry, TickReceiptRejection},
    record::{EdgeRecord, NodeRecord},
    tick_patch::{PortalInit, SlotId, TickCommitStatus, WarpOp, WarpTickPatchV1},
    tx::TxId,
    warp_state::WarpInstance,
    worldline::{AtomWrite, HashTriplet, WorldlineId, WorldlineTickHeaderV1, WorldlineTickPatchV1},
};

const RETAINED_PROVENANCE_MAGIC_V1: &[u8; 8] = b"EPRV0001";

/// Error returned by the retained local-commit provenance codec.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum RetainedProvenanceError {
    /// Retained bytes ended before the declared value was complete.
    #[error("retained provenance ended unexpectedly")]
    UnexpectedEof,
    /// A retained length cannot be represented on this platform.
    #[error("retained provenance length exceeds the platform limit")]
    LengthOverflow,
    /// Retained bytes did not carry the v1 provenance magic.
    #[error("retained provenance has invalid magic")]
    InvalidMagic,
    /// A tagged retained value used an unknown variant.
    #[error("retained provenance has unknown {family} tag {tag}")]
    UnknownTag {
        /// Tagged value family.
        family: &'static str,
        /// Unknown variant tag.
        tag: u8,
    },
    /// The codec currently retains only scheduler-produced local commits.
    #[error("retained provenance entry is not a local commit")]
    UnsupportedEventKind,
    /// A local commit did not identify its producing writer head.
    #[error("retained local commit is missing its writer head")]
    MissingHeadKey,
    /// A local commit did not carry its replay patch.
    #[error("retained local commit is missing its replay patch")]
    MissingPatch,
    /// A replayable local commit did not retain its exact scheduler receipt.
    #[error("retained local commit is missing its tick receipt")]
    MissingTickReceipt,
    /// An attachment owner and plane did not form a legal slot.
    #[error("retained provenance contains an invalid attachment key")]
    InvalidAttachmentKey,
    /// Redundant retained metadata disagreed.
    #[error("retained provenance has inconsistent {0}")]
    Inconsistent(&'static str),
    /// A patch was not in canonical replay order or its digest did not match.
    #[error("retained provenance contains a non-canonical patch")]
    NonCanonicalPatch,
    /// A retained receipt violated canonical blocker or disposition invariants.
    #[error("retained provenance contains a non-canonical tick receipt")]
    NonCanonicalTickReceipt,
    /// Retained bytes decoded but did not use the canonical v1 encoding.
    #[error("retained provenance is not canonically encoded")]
    NonCanonical,
    /// Retained bytes contained material outside the v1 entry.
    #[error("retained provenance has trailing bytes")]
    TrailingBytes,
    /// A retained textual identity field was not valid UTF-8.
    #[error("retained provenance contains invalid UTF-8")]
    InvalidUtf8,
}

/// Encodes one scheduler-produced local commit as replayable retained material.
pub(crate) fn encode_local_commit_v1(
    entry: &ProvenanceEntry,
) -> Result<Vec<u8>, RetainedProvenanceError> {
    validate_local_commit(entry)?;
    let head_key = entry
        .head_key
        .ok_or(RetainedProvenanceError::MissingHeadKey)?;
    let patch = entry
        .patch
        .as_ref()
        .ok_or(RetainedProvenanceError::MissingPatch)?;
    let receipt = entry
        .tick_receipt
        .as_ref()
        .ok_or(RetainedProvenanceError::MissingTickReceipt)?;

    let mut out = Vec::new();
    out.extend_from_slice(RETAINED_PROVENANCE_MAGIC_V1);
    push_worldline_id(&mut out, entry.worldline_id);
    push_u64(&mut out, entry.worldline_tick.as_u64());
    push_u64(&mut out, entry.commit_global_tick.as_u64());
    out.push(1); // ProvenanceEventKind::LocalCommit
    push_writer_head_key(&mut out, head_key);
    push_len(&mut out, entry.parents.len());
    for parent in &entry.parents {
        push_provenance_ref(&mut out, parent);
    }
    push_hash_triplet(&mut out, &entry.expected);
    push_tick_receipt(&mut out, receipt);
    push_worldline_patch(&mut out, patch);
    push_len(&mut out, entry.outputs.len());
    for (channel, bytes) in &entry.outputs {
        push_hash(&mut out, channel.as_bytes());
        push_bytes(&mut out, bytes);
    }
    push_len(&mut out, entry.atom_writes.len());
    for write in &entry.atom_writes {
        push_atom_write(&mut out, write);
    }
    Ok(out)
}

/// Decodes one scheduler-produced local commit without applying its patch.
pub(crate) fn decode_local_commit_v1(
    bytes: &[u8],
) -> Result<ProvenanceEntry, RetainedProvenanceError> {
    let mut cursor = RetainedProvenanceCursor::new(bytes);
    if cursor.read_exact(RETAINED_PROVENANCE_MAGIC_V1.len())? != RETAINED_PROVENANCE_MAGIC_V1 {
        return Err(RetainedProvenanceError::InvalidMagic);
    }
    let worldline_id = cursor.read_worldline_id()?;
    let worldline_tick = WorldlineTick::from_raw(cursor.read_u64()?);
    let commit_global_tick = GlobalTick::from_raw(cursor.read_u64()?);
    match cursor.read_u8()? {
        1 => {}
        tag => return Err(unknown_tag("provenance event", tag)),
    }
    let head_key = cursor.read_writer_head_key()?;
    let parent_count = cursor.read_count(72)?;
    let mut parents = Vec::with_capacity(parent_count);
    for _ in 0..parent_count {
        parents.push(cursor.read_provenance_ref()?);
    }
    let expected = cursor.read_hash_triplet()?;
    let receipt = cursor.read_tick_receipt()?;
    let patch = cursor.read_worldline_patch()?;
    let output_count = cursor.read_count(40)?;
    let mut outputs = Vec::with_capacity(output_count);
    for _ in 0..output_count {
        outputs.push((TypeId(cursor.read_hash()?), cursor.read_vec()?));
    }
    let write_count = cursor.read_count(113)?;
    let mut atom_writes = Vec::with_capacity(write_count);
    for _ in 0..write_count {
        atom_writes.push(cursor.read_atom_write()?);
    }
    cursor.finish()?;

    let entry = ProvenanceEntry::local_commit(
        worldline_id,
        worldline_tick,
        commit_global_tick,
        head_key,
        parents,
        expected,
        patch,
        outputs,
        atom_writes,
    )
    .with_tick_receipt(receipt);
    validate_local_commit(&entry)?;
    if encode_local_commit_v1(&entry)? != bytes {
        return Err(RetainedProvenanceError::NonCanonical);
    }
    Ok(entry)
}

fn validate_local_commit(entry: &ProvenanceEntry) -> Result<(), RetainedProvenanceError> {
    if entry.event_kind != ProvenanceEventKind::LocalCommit {
        return Err(RetainedProvenanceError::UnsupportedEventKind);
    }
    let head_key = entry
        .head_key
        .ok_or(RetainedProvenanceError::MissingHeadKey)?;
    if head_key.worldline_id != entry.worldline_id {
        return Err(RetainedProvenanceError::Inconsistent(
            "writer-head worldline",
        ));
    }
    let patch = entry
        .patch
        .as_ref()
        .ok_or(RetainedProvenanceError::MissingPatch)?;
    let receipt = entry
        .tick_receipt
        .as_ref()
        .ok_or(RetainedProvenanceError::MissingTickReceipt)?;
    if patch.commit_global_tick() != entry.commit_global_tick {
        return Err(RetainedProvenanceError::Inconsistent("global tick"));
    }
    if patch.patch_digest != entry.expected.patch_digest {
        return Err(RetainedProvenanceError::Inconsistent("patch digest"));
    }
    validate_tick_receipt(entry, patch, receipt)?;
    let canonical_patch = WarpTickPatchV1::new(
        patch.policy_id(),
        patch.rule_pack_id(),
        TickCommitStatus::Committed,
        patch.in_slots.clone(),
        patch.out_slots.clone(),
        patch.ops.clone(),
    );
    if canonical_patch.in_slots() != patch.in_slots
        || canonical_patch.out_slots() != patch.out_slots
        || canonical_patch.ops() != patch.ops
        || canonical_patch.digest() != patch.patch_digest
    {
        return Err(RetainedProvenanceError::NonCanonicalPatch);
    }
    if !entry
        .parents
        .windows(2)
        .all(|pair| pair[0].commit_hash < pair[1].commit_hash)
    {
        return Err(RetainedProvenanceError::Inconsistent("parent ordering"));
    }
    let parent_hashes = entry
        .parents
        .iter()
        .map(|parent| parent.commit_hash)
        .collect::<Vec<_>>();
    if compute_commit_hash_v2(
        &entry.expected.state_root,
        &parent_hashes,
        &entry.expected.patch_digest,
        patch.policy_id(),
    ) != entry.expected.commit_hash
    {
        return Err(RetainedProvenanceError::Inconsistent("commit hash"));
    }
    Ok(())
}

fn validate_tick_receipt(
    entry: &ProvenanceEntry,
    patch: &WorldlineTickPatchV1,
    receipt: &TickReceipt,
) -> Result<(), RetainedProvenanceError> {
    let expected_tx = entry
        .worldline_tick
        .as_u64()
        .checked_add(1)
        .ok_or(RetainedProvenanceError::Inconsistent("receipt transaction"))?;
    if receipt.tx().value() != expected_tx {
        return Err(RetainedProvenanceError::Inconsistent("receipt transaction"));
    }
    if receipt.digest() != patch.header.decision_digest {
        return Err(RetainedProvenanceError::Inconsistent("receipt digest"));
    }
    for (index, receipt_entry) in receipt.entries().iter().enumerate() {
        let blockers = receipt.blocked_by(index);
        if !blockers.windows(2).all(|pair| pair[0] < pair[1])
            || blockers
                .iter()
                .any(|blocker| usize::try_from(*blocker).map_or(true, |blocker| blocker >= index))
            || matches!(receipt_entry.disposition, TickReceiptDisposition::Applied)
                && !blockers.is_empty()
            || matches!(
                receipt_entry.disposition,
                TickReceiptDisposition::Rejected(TickReceiptRejection::FootprintConflict)
            ) && blockers.is_empty()
        {
            return Err(RetainedProvenanceError::NonCanonicalTickReceipt);
        }
    }
    Ok(())
}

fn push_tick_receipt(out: &mut Vec<u8>, receipt: &TickReceipt) {
    push_u64(out, receipt.tx().value());
    push_len(out, receipt.entries().len());
    for (index, entry) in receipt.entries().iter().enumerate() {
        push_hash(out, &entry.rule_id);
        push_hash(out, &entry.scope_hash);
        push_node_key(out, entry.scope);
        out.push(match entry.disposition {
            TickReceiptDisposition::Applied => 1,
            TickReceiptDisposition::Rejected(TickReceiptRejection::FootprintConflict) => 2,
        });
        let blockers = receipt.blocked_by(index);
        push_len(out, blockers.len());
        for blocker in blockers {
            out.extend_from_slice(&blocker.to_le_bytes());
        }
    }
}

fn push_worldline_patch(out: &mut Vec<u8>, patch: &WorldlineTickPatchV1) {
    push_u64(out, patch.header.commit_global_tick.as_u64());
    out.extend_from_slice(&patch.header.policy_id.to_le_bytes());
    push_hash(out, &patch.header.rule_pack_id);
    push_hash(out, &patch.header.plan_digest);
    push_hash(out, &patch.header.decision_digest);
    push_hash(out, &patch.header.rewrites_digest);
    push_hash(out, patch.warp_id.as_bytes());
    push_len(out, patch.ops.len());
    for op in &patch.ops {
        push_warp_op(out, op);
    }
    push_len(out, patch.in_slots.len());
    for slot in &patch.in_slots {
        push_slot(out, *slot);
    }
    push_len(out, patch.out_slots.len());
    for slot in &patch.out_slots {
        push_slot(out, *slot);
    }
    push_hash(out, &patch.patch_digest);
}

fn push_warp_op(out: &mut Vec<u8>, op: &WarpOp) {
    match op {
        WarpOp::OpenPortal {
            key,
            child_warp,
            child_root,
            init,
        } => {
            out.push(1);
            push_attachment_key(out, *key);
            push_hash(out, child_warp.as_bytes());
            push_hash(out, child_root.as_bytes());
            match init {
                PortalInit::Empty { root_record } => {
                    out.push(1);
                    push_node_record(out, root_record);
                }
                PortalInit::RequireExisting => out.push(2),
            }
        }
        WarpOp::UpsertWarpInstance { instance } => {
            out.push(2);
            push_warp_instance(out, instance);
        }
        WarpOp::DeleteWarpInstance { warp_id } => {
            out.push(3);
            push_hash(out, warp_id.as_bytes());
        }
        WarpOp::UpsertNode { node, record } => {
            out.push(4);
            push_node_key(out, *node);
            push_node_record(out, record);
        }
        WarpOp::DeleteNode { node } => {
            out.push(5);
            push_node_key(out, *node);
        }
        WarpOp::UpsertEdge { warp_id, record } => {
            out.push(6);
            push_hash(out, warp_id.as_bytes());
            push_edge_record(out, record);
        }
        WarpOp::DeleteEdge {
            warp_id,
            from,
            edge_id,
        } => {
            out.push(7);
            push_hash(out, warp_id.as_bytes());
            push_hash(out, from.as_bytes());
            push_hash(out, edge_id.as_bytes());
        }
        WarpOp::SetAttachment { key, value } => {
            out.push(8);
            push_attachment_key(out, *key);
            push_optional_attachment_value(out, value.as_ref());
        }
    }
}

fn push_slot(out: &mut Vec<u8>, slot: SlotId) {
    match slot {
        SlotId::Node(node) => {
            out.push(1);
            push_node_key(out, node);
        }
        SlotId::Edge(edge) => {
            out.push(2);
            push_edge_key(out, edge);
        }
        SlotId::Attachment(key) => {
            out.push(3);
            push_attachment_key(out, key);
        }
        SlotId::Port((warp_id, port)) => {
            out.push(4);
            push_hash(out, warp_id.as_bytes());
            push_u64(out, port);
        }
    }
}

fn push_warp_instance(out: &mut Vec<u8>, instance: &WarpInstance) {
    push_hash(out, instance.warp_id.as_bytes());
    push_hash(out, instance.root_node.as_bytes());
    match instance.parent {
        Some(parent) => {
            out.push(1);
            push_attachment_key(out, parent);
        }
        None => out.push(0),
    }
}

fn push_optional_attachment_value(out: &mut Vec<u8>, value: Option<&AttachmentValue>) {
    let Some(value) = value else {
        out.push(0);
        return;
    };
    out.push(1);
    match value {
        AttachmentValue::Atom(atom) => {
            out.push(1);
            push_hash(out, atom.type_id.as_bytes());
            push_bytes(out, atom.bytes.as_ref());
        }
        AttachmentValue::Descend(warp_id) => {
            out.push(2);
            push_hash(out, warp_id.as_bytes());
        }
    }
}

fn push_attachment_key(out: &mut Vec<u8>, key: AttachmentKey) {
    match key.owner {
        AttachmentOwner::Node(node) => {
            out.push(1);
            push_node_key(out, node);
        }
        AttachmentOwner::Edge(edge) => {
            out.push(2);
            push_edge_key(out, edge);
        }
    }
    out.push(match key.plane {
        AttachmentPlane::Alpha => 1,
        AttachmentPlane::Beta => 2,
    });
}

fn push_node_key(out: &mut Vec<u8>, key: NodeKey) {
    push_hash(out, key.warp_id.as_bytes());
    push_hash(out, key.local_id.as_bytes());
}

fn push_edge_key(out: &mut Vec<u8>, key: EdgeKey) {
    push_hash(out, key.warp_id.as_bytes());
    push_hash(out, key.local_id.as_bytes());
}

fn push_node_record(out: &mut Vec<u8>, record: &NodeRecord) {
    push_hash(out, record.ty.as_bytes());
}

fn push_edge_record(out: &mut Vec<u8>, record: &EdgeRecord) {
    push_hash(out, record.id.as_bytes());
    push_hash(out, record.from.as_bytes());
    push_hash(out, record.to.as_bytes());
    push_hash(out, record.ty.as_bytes());
}

fn push_atom_write(out: &mut Vec<u8>, write: &AtomWrite) {
    push_node_key(out, write.atom);
    push_hash(out, &write.rule_id);
    push_u64(out, write.tick);
    match &write.old_value {
        Some(bytes) => {
            out.push(1);
            push_bytes(out, bytes);
        }
        None => out.push(0),
    }
    push_bytes(out, &write.new_value);
}

fn push_provenance_ref(out: &mut Vec<u8>, reference: &ProvenanceRef) {
    push_worldline_id(out, reference.worldline_id);
    push_u64(out, reference.worldline_tick.as_u64());
    push_hash(out, &reference.commit_hash);
}

fn push_hash_triplet(out: &mut Vec<u8>, triplet: &HashTriplet) {
    push_hash(out, &triplet.state_root);
    push_hash(out, &triplet.patch_digest);
    push_hash(out, &triplet.commit_hash);
}

fn push_writer_head_key(out: &mut Vec<u8>, key: WriterHeadKey) {
    push_worldline_id(out, key.worldline_id);
    push_hash(out, key.head_id.as_bytes());
}

fn push_worldline_id(out: &mut Vec<u8>, worldline_id: WorldlineId) {
    push_hash(out, worldline_id.as_bytes());
}

fn push_hash(out: &mut Vec<u8>, hash: &Hash) {
    out.extend_from_slice(hash);
}

fn push_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    push_len(out, bytes.len());
    out.extend_from_slice(bytes);
}

fn push_len(out: &mut Vec<u8>, len: usize) {
    push_u64(out, len as u64);
}

fn push_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn unknown_tag(family: &'static str, tag: u8) -> RetainedProvenanceError {
    RetainedProvenanceError::UnknownTag { family, tag }
}

struct RetainedProvenanceCursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> RetainedProvenanceCursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn remaining_len(&self) -> usize {
        self.bytes.len().saturating_sub(self.offset)
    }

    fn read_exact(&mut self, len: usize) -> Result<&'a [u8], RetainedProvenanceError> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or(RetainedProvenanceError::UnexpectedEof)?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(RetainedProvenanceError::UnexpectedEof)?;
        self.offset = end;
        Ok(value)
    }

    fn read_u8(&mut self) -> Result<u8, RetainedProvenanceError> {
        Ok(self.read_exact(1)?[0])
    }

    fn read_u32(&mut self) -> Result<u32, RetainedProvenanceError> {
        Ok(u32::from_le_bytes(
            self.read_exact(4)?
                .try_into()
                .map_err(|_| RetainedProvenanceError::UnexpectedEof)?,
        ))
    }

    fn read_u64(&mut self) -> Result<u64, RetainedProvenanceError> {
        Ok(u64::from_le_bytes(
            self.read_exact(8)?
                .try_into()
                .map_err(|_| RetainedProvenanceError::UnexpectedEof)?,
        ))
    }

    fn read_len(&mut self) -> Result<usize, RetainedProvenanceError> {
        usize::try_from(self.read_u64()?).map_err(|_| RetainedProvenanceError::LengthOverflow)
    }

    fn read_count(&mut self, minimum_encoded_len: usize) -> Result<usize, RetainedProvenanceError> {
        let count = self.read_len()?;
        if minimum_encoded_len != 0 && count > self.remaining_len() / minimum_encoded_len {
            return Err(RetainedProvenanceError::UnexpectedEof);
        }
        Ok(count)
    }

    fn read_vec(&mut self) -> Result<Vec<u8>, RetainedProvenanceError> {
        let len = self.read_len()?;
        Ok(self.read_exact(len)?.to_vec())
    }

    fn read_hash(&mut self) -> Result<Hash, RetainedProvenanceError> {
        self.read_exact(32)?
            .try_into()
            .map_err(|_| RetainedProvenanceError::UnexpectedEof)
    }

    fn read_worldline_id(&mut self) -> Result<WorldlineId, RetainedProvenanceError> {
        Ok(WorldlineId::from_bytes(self.read_hash()?))
    }

    fn read_tick_receipt(&mut self) -> Result<TickReceipt, RetainedProvenanceError> {
        let tx = TxId::from_raw(self.read_u64()?);
        let entry_count = self.read_count(137)?;
        let mut entries = Vec::with_capacity(entry_count);
        let mut blocked_by = Vec::with_capacity(entry_count);
        for _ in 0..entry_count {
            let rule_id = self.read_hash()?;
            let scope_hash = self.read_hash()?;
            let scope = self.read_node_key()?;
            let disposition = match self.read_u8()? {
                1 => TickReceiptDisposition::Applied,
                2 => TickReceiptDisposition::Rejected(TickReceiptRejection::FootprintConflict),
                tag => return Err(unknown_tag("tick receipt disposition", tag)),
            };
            let blocker_count = self.read_count(4)?;
            let mut blockers = Vec::with_capacity(blocker_count);
            for _ in 0..blocker_count {
                blockers.push(self.read_u32()?);
            }
            entries.push(TickReceiptEntry {
                rule_id,
                scope_hash,
                scope,
                disposition,
            });
            blocked_by.push(blockers);
        }
        TickReceipt::try_from_retained_parts(tx, entries, blocked_by)
            .map_err(|_| RetainedProvenanceError::NonCanonicalTickReceipt)
    }

    fn read_writer_head_key(&mut self) -> Result<WriterHeadKey, RetainedProvenanceError> {
        Ok(WriterHeadKey {
            worldline_id: self.read_worldline_id()?,
            head_id: HeadId::from_bytes(self.read_hash()?),
        })
    }

    fn read_provenance_ref(&mut self) -> Result<ProvenanceRef, RetainedProvenanceError> {
        Ok(ProvenanceRef {
            worldline_id: self.read_worldline_id()?,
            worldline_tick: WorldlineTick::from_raw(self.read_u64()?),
            commit_hash: self.read_hash()?,
        })
    }

    fn read_hash_triplet(&mut self) -> Result<HashTriplet, RetainedProvenanceError> {
        Ok(HashTriplet {
            state_root: self.read_hash()?,
            patch_digest: self.read_hash()?,
            commit_hash: self.read_hash()?,
        })
    }

    fn read_worldline_patch(&mut self) -> Result<WorldlineTickPatchV1, RetainedProvenanceError> {
        let header = WorldlineTickHeaderV1 {
            commit_global_tick: GlobalTick::from_raw(self.read_u64()?),
            policy_id: self.read_u32()?,
            rule_pack_id: self.read_hash()?,
            plan_digest: self.read_hash()?,
            decision_digest: self.read_hash()?,
            rewrites_digest: self.read_hash()?,
        };
        let warp_id = WarpId(self.read_hash()?);
        let op_count = self.read_count(1)?;
        let mut ops = Vec::with_capacity(op_count);
        for _ in 0..op_count {
            ops.push(self.read_warp_op()?);
        }
        let in_slot_count = self.read_count(41)?;
        let mut in_slots = Vec::with_capacity(in_slot_count);
        for _ in 0..in_slot_count {
            in_slots.push(self.read_slot()?);
        }
        let out_slot_count = self.read_count(41)?;
        let mut out_slots = Vec::with_capacity(out_slot_count);
        for _ in 0..out_slot_count {
            out_slots.push(self.read_slot()?);
        }
        let patch_digest = self.read_hash()?;
        Ok(WorldlineTickPatchV1 {
            header,
            warp_id,
            ops,
            in_slots,
            out_slots,
            patch_digest,
        })
    }

    fn read_warp_op(&mut self) -> Result<WarpOp, RetainedProvenanceError> {
        match self.read_u8()? {
            1 => {
                let key = self.read_attachment_key()?;
                let child_warp = WarpId(self.read_hash()?);
                let child_root = NodeId(self.read_hash()?);
                let init = match self.read_u8()? {
                    1 => PortalInit::Empty {
                        root_record: self.read_node_record()?,
                    },
                    2 => PortalInit::RequireExisting,
                    tag => return Err(unknown_tag("portal init", tag)),
                };
                Ok(WarpOp::OpenPortal {
                    key,
                    child_warp,
                    child_root,
                    init,
                })
            }
            2 => Ok(WarpOp::UpsertWarpInstance {
                instance: self.read_warp_instance()?,
            }),
            3 => Ok(WarpOp::DeleteWarpInstance {
                warp_id: WarpId(self.read_hash()?),
            }),
            4 => Ok(WarpOp::UpsertNode {
                node: self.read_node_key()?,
                record: self.read_node_record()?,
            }),
            5 => Ok(WarpOp::DeleteNode {
                node: self.read_node_key()?,
            }),
            6 => Ok(WarpOp::UpsertEdge {
                warp_id: WarpId(self.read_hash()?),
                record: self.read_edge_record()?,
            }),
            7 => Ok(WarpOp::DeleteEdge {
                warp_id: WarpId(self.read_hash()?),
                from: NodeId(self.read_hash()?),
                edge_id: EdgeId(self.read_hash()?),
            }),
            8 => Ok(WarpOp::SetAttachment {
                key: self.read_attachment_key()?,
                value: self.read_optional_attachment_value()?,
            }),
            tag => Err(unknown_tag("warp operation", tag)),
        }
    }

    fn read_slot(&mut self) -> Result<SlotId, RetainedProvenanceError> {
        match self.read_u8()? {
            1 => Ok(SlotId::Node(self.read_node_key()?)),
            2 => Ok(SlotId::Edge(self.read_edge_key()?)),
            3 => Ok(SlotId::Attachment(self.read_attachment_key()?)),
            4 => Ok(SlotId::Port((WarpId(self.read_hash()?), self.read_u64()?))),
            tag => Err(unknown_tag("slot", tag)),
        }
    }

    fn read_warp_instance(&mut self) -> Result<WarpInstance, RetainedProvenanceError> {
        let warp_id = WarpId(self.read_hash()?);
        let root_node = NodeId(self.read_hash()?);
        let parent = match self.read_u8()? {
            0 => None,
            1 => Some(self.read_attachment_key()?),
            tag => return Err(unknown_tag("optional attachment key", tag)),
        };
        Ok(WarpInstance {
            warp_id,
            root_node,
            parent,
        })
    }

    fn read_optional_attachment_value(
        &mut self,
    ) -> Result<Option<AttachmentValue>, RetainedProvenanceError> {
        match self.read_u8()? {
            0 => Ok(None),
            1 => match self.read_u8()? {
                1 => Ok(Some(AttachmentValue::Atom(AtomPayload::new(
                    TypeId(self.read_hash()?),
                    Bytes::from(self.read_vec()?),
                )))),
                2 => Ok(Some(AttachmentValue::Descend(WarpId(self.read_hash()?)))),
                tag => Err(unknown_tag("attachment value", tag)),
            },
            tag => Err(unknown_tag("optional attachment value", tag)),
        }
    }

    fn read_attachment_key(&mut self) -> Result<AttachmentKey, RetainedProvenanceError> {
        let owner = match self.read_u8()? {
            1 => AttachmentOwner::Node(self.read_node_key()?),
            2 => AttachmentOwner::Edge(self.read_edge_key()?),
            tag => return Err(unknown_tag("attachment owner", tag)),
        };
        let plane = match self.read_u8()? {
            1 => AttachmentPlane::Alpha,
            2 => AttachmentPlane::Beta,
            tag => return Err(unknown_tag("attachment plane", tag)),
        };
        let key = AttachmentKey { owner, plane };
        if !key.is_plane_valid() {
            return Err(RetainedProvenanceError::InvalidAttachmentKey);
        }
        Ok(key)
    }

    fn read_node_key(&mut self) -> Result<NodeKey, RetainedProvenanceError> {
        Ok(NodeKey {
            warp_id: WarpId(self.read_hash()?),
            local_id: NodeId(self.read_hash()?),
        })
    }

    fn read_edge_key(&mut self) -> Result<EdgeKey, RetainedProvenanceError> {
        Ok(EdgeKey {
            warp_id: WarpId(self.read_hash()?),
            local_id: EdgeId(self.read_hash()?),
        })
    }

    fn read_node_record(&mut self) -> Result<NodeRecord, RetainedProvenanceError> {
        Ok(NodeRecord {
            ty: TypeId(self.read_hash()?),
        })
    }

    fn read_edge_record(&mut self) -> Result<EdgeRecord, RetainedProvenanceError> {
        Ok(EdgeRecord {
            id: EdgeId(self.read_hash()?),
            from: NodeId(self.read_hash()?),
            to: NodeId(self.read_hash()?),
            ty: TypeId(self.read_hash()?),
        })
    }

    fn read_atom_write(&mut self) -> Result<AtomWrite, RetainedProvenanceError> {
        let atom = self.read_node_key()?;
        let rule_id = self.read_hash()?;
        let tick = self.read_u64()?;
        let old_value = match self.read_u8()? {
            0 => None,
            1 => Some(self.read_vec()?),
            tag => return Err(unknown_tag("optional atom value", tag)),
        };
        let new_value = self.read_vec()?;
        Ok(AtomWrite::new(atom, rule_id, tick, old_value, new_value))
    }

    fn finish(self) -> Result<(), RetainedProvenanceError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(RetainedProvenanceError::TrailingBytes)
        }
    }
}
