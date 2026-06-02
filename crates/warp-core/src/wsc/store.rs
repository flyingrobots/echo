// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Generic WSC storage port and deterministic envelope format.

use std::collections::BTreeMap;

use blake3::Hasher;
use bytes::Bytes;

use crate::attachment::{AtomPayload, AttachmentValue};
use crate::causal_wal::{
    SubmissionAcceptanceRecord, TickReceiptRecord, WalReceiptCorrelationRecord,
};
use crate::graph::GraphStore;
use crate::ident::{make_node_id, make_type_id, make_warp_id, EdgeId, Hash, NodeId};
use crate::record::{EdgeRecord, NodeRecord};

use super::build::build_one_warp_input;
use super::types::AttRow;
use super::validate::validate_wsc;
use super::view::WscFile;
use super::write::write_wsc_one_warp;

const WSC_STORE_ENVELOPE_MAGIC: &[u8; 8] = b"ECWSCST1";
const WSC_STORE_ENVELOPE_VERSION: u16 = 1;
const WSC_STORE_ENVELOPE_ID_DOMAIN: &[u8] = b"echo:wsc_store:envelope_id:v1\0";
const WSC_STORE_BYTES_DOMAIN: &[u8] = b"echo:wsc_store:wsc_bytes:v1\0";
const WSC_ACCEPTED_SUBMISSION_BASIS_DOMAIN: &[u8] =
    b"echo:wsc_store:accepted_submission_basis:v1\0";
const WSC_ACCEPTED_SUBMISSION_NODE_DOMAIN: &[u8] = b"echo:wsc_store:accepted_submission_node:v1\0";
const WSC_ACCEPTED_SUBMISSION_EDGE_DOMAIN: &[u8] = b"echo:wsc_store:accepted_submission_edge:v1\0";
const WSC_ACCEPTED_SUBMISSION_SCHEMA: &str = "echo/wsc-store/accepted-submissions/v1";
const WSC_ACCEPTED_SUBMISSION_WARP: &str = "echo/wsc-store/accepted-submissions";
const WSC_ACCEPTED_SUBMISSION_ROOT: &str = "echo/wsc-store/accepted-submissions/root";
const WSC_ACCEPTED_SUBMISSION_NODE_TYPE: &str = "echo/wsc-store/accepted-submissions/node/v1";
const WSC_ACCEPTED_SUBMISSION_EDGE_TYPE: &str = "echo/wsc-store/accepted-submissions/member/v1";
const WSC_ACCEPTED_SUBMISSION_ATTACHMENT_TYPE: &str =
    "echo/wsc-store/accepted-submissions/record/v1";
const WSC_RECEIPT_CORRELATION_BASIS_DOMAIN: &[u8] =
    b"echo:wsc_store:receipt_correlation_basis:v1\0";
const WSC_RECEIPT_CORRELATION_NODE_DOMAIN: &[u8] = b"echo:wsc_store:receipt_correlation_node:v1\0";
const WSC_RECEIPT_CORRELATION_EDGE_DOMAIN: &[u8] = b"echo:wsc_store:receipt_correlation_edge:v1\0";
const WSC_RECEIPT_CORRELATION_SCHEMA: &str = "echo/wsc-store/receipt-correlations/v1";
const WSC_RECEIPT_CORRELATION_WARP: &str = "echo/wsc-store/receipt-correlations";
const WSC_RECEIPT_CORRELATION_ROOT: &str = "echo/wsc-store/receipt-correlations/root";
const WSC_RECEIPT_CORRELATION_NODE_TYPE: &str = "echo/wsc-store/receipt-correlations/node/v1";
const WSC_RECEIPT_CORRELATION_EDGE_TYPE: &str = "echo/wsc-store/receipt-correlations/member/v1";
const WSC_TICK_RECEIPT_ATTACHMENT_TYPE: &str = "echo/wsc-store/receipt-correlations/receipt/v1";
const WSC_RECEIPT_CORRELATION_ATTACHMENT_TYPE: &str =
    "echo/wsc-store/receipt-correlations/correlation/v1";
const HEADER_LEN: usize = 124;

/// Stable identifier for a WSC store envelope.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WscStoreEnvelopeId(Hash);

impl WscStoreEnvelopeId {
    /// Builds an envelope id from a canonical digest.
    #[must_use]
    pub const fn from_hash(hash: Hash) -> Self {
        Self(hash)
    }

    /// Returns the canonical digest bytes.
    #[must_use]
    pub const fn as_hash(self) -> Hash {
        self.0
    }
}

/// Generic kind of WSC material stored by Echo.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum WscStoreRecordKind {
    /// Materialized causal snapshot.
    Snapshot,
    /// Causal-history material.
    CausalHistory,
    /// Retained evidence material.
    RetainedEvidence,
}

impl WscStoreRecordKind {
    const fn code(self) -> u16 {
        match self {
            Self::Snapshot => 1,
            Self::CausalHistory => 2,
            Self::RetainedEvidence => 3,
        }
    }

    const fn from_code(code: u16) -> Option<Self> {
        match code {
            1 => Some(Self::Snapshot),
            2 => Some(Self::CausalHistory),
            3 => Some(Self::RetainedEvidence),
            _ => None,
        }
    }
}

/// Subject named by a WSC store obstruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WscStoreSubject {
    /// Envelope identity was implicated.
    Envelope {
        /// Envelope id.
        envelope_id: WscStoreEnvelopeId,
    },
    /// Encoded bytes were malformed near an offset.
    EnvelopeBytes {
        /// Byte offset implicated by the obstruction.
        offset: usize,
    },
    /// Encoded bytes carried a digest mismatch.
    EnvelopeDigest {
        /// Expected digest recorded by the envelope.
        expected: Hash,
        /// Actual digest computed from the payload.
        actual: Hash,
    },
    /// WSC payload was invalid.
    WscPayload {
        /// Digest of the invalid WSC payload.
        digest: Hash,
    },
}

/// Generic WSC store obstruction kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WscStoreObstructionKind {
    /// Requested envelope was absent.
    MissingEnvelope,
    /// Envelope header or structural fields were malformed.
    InvalidEnvelope,
    /// WSC payload failed WSC parsing or validation.
    InvalidWsc,
    /// Encoded envelope digest did not match its payload.
    DigestMismatch,
    /// Existing envelope id maps to different material.
    DuplicateEnvelopeMismatch,
}

/// Typed obstruction returned instead of hidden fallback or invented success.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WscStoreObstruction {
    /// Obstruction kind.
    pub kind: WscStoreObstructionKind,
    /// Obstruction subject.
    pub subject: WscStoreSubject,
}

impl WscStoreObstruction {
    fn invalid_envelope(offset: usize) -> Self {
        Self {
            kind: WscStoreObstructionKind::InvalidEnvelope,
            subject: WscStoreSubject::EnvelopeBytes { offset },
        }
    }

    fn invalid_wsc(wsc_digest: Hash) -> Self {
        Self {
            kind: WscStoreObstructionKind::InvalidWsc,
            subject: WscStoreSubject::WscPayload { digest: wsc_digest },
        }
    }

    fn digest_mismatch(expected: Hash, actual: Hash) -> Self {
        Self {
            kind: WscStoreObstructionKind::DigestMismatch,
            subject: WscStoreSubject::EnvelopeDigest { expected, actual },
        }
    }

    fn missing_envelope(envelope_id: WscStoreEnvelopeId) -> Self {
        Self {
            kind: WscStoreObstructionKind::MissingEnvelope,
            subject: WscStoreSubject::Envelope { envelope_id },
        }
    }

    fn duplicate_mismatch(envelope_id: WscStoreEnvelopeId) -> Self {
        Self {
            kind: WscStoreObstructionKind::DuplicateEnvelopeMismatch,
            subject: WscStoreSubject::Envelope { envelope_id },
        }
    }
}

/// Deterministic WSC store envelope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WscStoreEnvelope {
    id: WscStoreEnvelopeId,
    record_kind: WscStoreRecordKind,
    basis_digest: Hash,
    schema_hash: Hash,
    tick: u64,
    wsc_digest: Hash,
    wsc_len: u64,
    wsc_bytes: Vec<u8>,
}

impl WscStoreEnvelope {
    /// Builds and validates a WSC store envelope.
    ///
    /// # Errors
    ///
    /// Returns [`WscStoreObstructionKind::InvalidWsc`] when the payload is not
    /// valid WSC material.
    pub fn validated(
        record_kind: WscStoreRecordKind,
        basis_digest: Hash,
        wsc_bytes: Vec<u8>,
    ) -> Result<Self, WscStoreObstruction> {
        let wsc_digest = digest_wsc_bytes(&wsc_bytes);
        let file = WscFile::from_bytes(wsc_bytes.clone())
            .map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
        validate_wsc(&file).map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
        let schema_hash = *file.schema_hash();
        let tick = file.tick();
        let wsc_len = u64::try_from(wsc_bytes.len())
            .map_err(|_| WscStoreObstruction::invalid_envelope(HEADER_LEN))?;
        let id = derive_envelope_id(
            record_kind,
            &basis_digest,
            &schema_hash,
            tick,
            &wsc_digest,
            wsc_len,
        );
        Ok(Self {
            id,
            record_kind,
            basis_digest,
            schema_hash,
            tick,
            wsc_digest,
            wsc_len,
            wsc_bytes,
        })
    }

    /// Decodes and validates a deterministic WSC store envelope.
    ///
    /// # Errors
    ///
    /// Returns a typed WSC store obstruction for malformed envelopes, digest
    /// mismatch, or invalid WSC payloads.
    pub fn decode(bytes: &[u8]) -> Result<Self, WscStoreObstruction> {
        let magic = read_array::<8>(bytes, 0)?;
        if &magic != WSC_STORE_ENVELOPE_MAGIC {
            return Err(WscStoreObstruction::invalid_envelope(0));
        }
        let version = u16::from_le_bytes(read_array::<2>(bytes, 8)?);
        if version != WSC_STORE_ENVELOPE_VERSION {
            return Err(WscStoreObstruction::invalid_envelope(8));
        }
        let record_kind_code = u16::from_le_bytes(read_array::<2>(bytes, 10)?);
        let record_kind = WscStoreRecordKind::from_code(record_kind_code)
            .ok_or_else(|| WscStoreObstruction::invalid_envelope(10))?;
        let schema_hash = read_array::<32>(bytes, 12)?;
        let basis_digest = read_array::<32>(bytes, 44)?;
        let wsc_digest = read_array::<32>(bytes, 76)?;
        let tick = u64::from_le_bytes(read_array::<8>(bytes, 108)?);
        let wsc_len = u64::from_le_bytes(read_array::<8>(bytes, 116)?);
        let payload_start = 124usize;
        let payload_len =
            usize::try_from(wsc_len).map_err(|_| WscStoreObstruction::invalid_envelope(116))?;
        let payload_end = payload_start
            .checked_add(payload_len)
            .ok_or_else(|| WscStoreObstruction::invalid_envelope(payload_start))?;
        let payload = bytes
            .get(payload_start..payload_end)
            .ok_or_else(|| WscStoreObstruction::invalid_envelope(payload_start))?;
        if payload_end != bytes.len() {
            return Err(WscStoreObstruction::invalid_envelope(payload_end));
        }
        let actual_digest = digest_wsc_bytes(payload);
        if actual_digest != wsc_digest {
            return Err(WscStoreObstruction::digest_mismatch(
                wsc_digest,
                actual_digest,
            ));
        }
        let envelope = Self::validated(record_kind, basis_digest, payload.to_vec())?;
        if envelope.schema_hash != schema_hash
            || envelope.tick != tick
            || envelope.wsc_len != wsc_len
        {
            return Err(WscStoreObstruction::invalid_envelope(12));
        }
        Ok(envelope)
    }

    /// Encodes this envelope into deterministic bytes.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(HEADER_LEN + self.wsc_bytes.len());
        bytes.extend_from_slice(WSC_STORE_ENVELOPE_MAGIC);
        bytes.extend_from_slice(&WSC_STORE_ENVELOPE_VERSION.to_le_bytes());
        bytes.extend_from_slice(&self.record_kind.code().to_le_bytes());
        bytes.extend_from_slice(&self.schema_hash);
        bytes.extend_from_slice(&self.basis_digest);
        bytes.extend_from_slice(&self.wsc_digest);
        bytes.extend_from_slice(&self.tick.to_le_bytes());
        bytes.extend_from_slice(&self.wsc_len.to_le_bytes());
        bytes.extend_from_slice(&self.wsc_bytes);
        bytes
    }

    /// Returns the envelope id.
    #[must_use]
    pub const fn id(&self) -> WscStoreEnvelopeId {
        self.id
    }

    /// Returns the generic record kind.
    #[must_use]
    pub const fn record_kind(&self) -> WscStoreRecordKind {
        self.record_kind
    }

    /// Returns the basis digest.
    #[must_use]
    pub const fn basis_digest(&self) -> &Hash {
        &self.basis_digest
    }

    /// Returns the WSC schema hash.
    #[must_use]
    pub const fn schema_hash(&self) -> &Hash {
        &self.schema_hash
    }

    /// Returns the WSC tick.
    #[must_use]
    pub const fn tick(&self) -> u64 {
        self.tick
    }

    /// Returns the WSC payload digest.
    #[must_use]
    pub const fn wsc_digest(&self) -> &Hash {
        &self.wsc_digest
    }

    /// Returns the WSC bytes.
    #[must_use]
    pub fn wsc_bytes(&self) -> &[u8] {
        &self.wsc_bytes
    }
}

/// Receipt returned after a WSC envelope write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WscStoreWriteReceipt {
    /// Written envelope id.
    pub envelope_id: WscStoreEnvelopeId,
    /// WSC payload digest.
    pub wsc_digest: Hash,
    /// Encoded envelope byte length.
    pub encoded_len: u64,
}

/// Receipt and correlation records recovered from WSC material.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WscReceiptCorrelationRecords {
    /// Tick receipt records with decision posture.
    pub receipts: Vec<TickReceiptRecord>,
    /// Ticket/submission/receipt correlation records.
    pub correlations: Vec<WalReceiptCorrelationRecord>,
}

/// Generic WSC store port.
pub trait WscStorePort {
    /// Writes a validated WSC envelope.
    fn write_envelope(
        &mut self,
        envelope: WscStoreEnvelope,
    ) -> Result<WscStoreWriteReceipt, WscStoreObstruction>;

    /// Reads a WSC envelope by id.
    fn read_envelope(
        &self,
        envelope_id: WscStoreEnvelopeId,
    ) -> Result<WscStoreEnvelope, WscStoreObstruction>;

    /// Lists known envelope ids in deterministic order.
    fn list_envelopes(&self) -> Vec<WscStoreEnvelopeId>;
}

/// In-memory WSC store implementation for tests and adapters.
#[derive(Debug, Default)]
pub struct InMemoryWscStore {
    envelopes: BTreeMap<WscStoreEnvelopeId, WscStoreEnvelope>,
}

impl WscStorePort for InMemoryWscStore {
    fn write_envelope(
        &mut self,
        envelope: WscStoreEnvelope,
    ) -> Result<WscStoreWriteReceipt, WscStoreObstruction> {
        let envelope_id = envelope.id();
        if let Some(existing) = self.envelopes.get(&envelope_id) {
            if existing != &envelope {
                return Err(WscStoreObstruction::duplicate_mismatch(envelope_id));
            }
        }
        let encoded_len = u64::try_from(envelope.encode().len())
            .map_err(|_| WscStoreObstruction::invalid_envelope(HEADER_LEN))?;
        let receipt = WscStoreWriteReceipt {
            envelope_id,
            wsc_digest: envelope.wsc_digest,
            encoded_len,
        };
        self.envelopes.insert(envelope_id, envelope);
        Ok(receipt)
    }

    fn read_envelope(
        &self,
        envelope_id: WscStoreEnvelopeId,
    ) -> Result<WscStoreEnvelope, WscStoreObstruction> {
        self.envelopes
            .get(&envelope_id)
            .cloned()
            .ok_or_else(|| WscStoreObstruction::missing_envelope(envelope_id))
    }

    fn list_envelopes(&self) -> Vec<WscStoreEnvelopeId> {
        self.envelopes.keys().copied().collect()
    }
}

/// Builds a generic WSC envelope for accepted submission records.
///
/// Duplicate identical records are represented once. A duplicate submission id
/// with different material is a typed obstruction.
///
/// # Errors
///
/// Returns [`WscStoreObstructionKind::DuplicateEnvelopeMismatch`] for
/// conflicting duplicate submission ids or [`WscStoreObstructionKind::InvalidWsc`]
/// when generated WSC material fails validation.
pub fn accepted_submission_records_to_wsc_envelope(
    records: &[SubmissionAcceptanceRecord],
) -> Result<WscStoreEnvelope, WscStoreObstruction> {
    let records = canonical_accepted_submission_records(records)?;
    let mut store = GraphStore::new(make_warp_id(WSC_ACCEPTED_SUBMISSION_WARP));
    let root = make_node_id(WSC_ACCEPTED_SUBMISSION_ROOT);
    store.insert_node(
        root,
        NodeRecord {
            ty: make_type_id(WSC_ACCEPTED_SUBMISSION_NODE_TYPE),
        },
    );
    for record in &records {
        let node = accepted_submission_node_id(&record.submission_id);
        store.insert_node(
            node,
            NodeRecord {
                ty: make_type_id(WSC_ACCEPTED_SUBMISSION_NODE_TYPE),
            },
        );
        store.insert_edge(
            root,
            EdgeRecord {
                id: accepted_submission_edge_id(&record.submission_id),
                from: root,
                to: node,
                ty: make_type_id(WSC_ACCEPTED_SUBMISSION_EDGE_TYPE),
            },
        );
        store.set_node_attachment(
            node,
            Some(AttachmentValue::Atom(AtomPayload::new(
                make_type_id(WSC_ACCEPTED_SUBMISSION_ATTACHMENT_TYPE),
                Bytes::from(record.to_payload_bytes()),
            ))),
        );
    }
    let basis_digest = accepted_submission_basis_digest(&records);
    let input = build_one_warp_input(&store, root);
    let wsc_bytes = write_wsc_one_warp(&input, make_type_id(WSC_ACCEPTED_SUBMISSION_SCHEMA).0, 0)
        .map_err(|_| WscStoreObstruction::invalid_wsc(basis_digest))?;
    WscStoreEnvelope::validated(WscStoreRecordKind::CausalHistory, basis_digest, wsc_bytes)
}

/// Recovers accepted submission records from a generic WSC envelope.
///
/// # Errors
///
/// Returns a typed WSC store obstruction when the envelope is not accepted
/// submission causal-history material or when record payloads are malformed.
pub fn accepted_submission_records_from_wsc_envelope(
    envelope: &WscStoreEnvelope,
) -> Result<Vec<SubmissionAcceptanceRecord>, WscStoreObstruction> {
    if envelope.record_kind() != WscStoreRecordKind::CausalHistory {
        return Err(WscStoreObstruction::invalid_envelope(0));
    }
    let wsc_digest = *envelope.wsc_digest();
    let file = WscFile::from_bytes(envelope.wsc_bytes().to_vec())
        .map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
    validate_wsc(&file).map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
    if file.schema_hash() != &make_type_id(WSC_ACCEPTED_SUBMISSION_SCHEMA).0 {
        return Err(WscStoreObstruction::invalid_wsc(wsc_digest));
    }
    let view = file
        .warp_view(0)
        .map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
    let mut records = Vec::new();
    for node_index in 0..view.nodes().len() {
        for attachment in view.node_attachments(node_index) {
            if attachment.type_or_warp != make_type_id(WSC_ACCEPTED_SUBMISSION_ATTACHMENT_TYPE).0 {
                return Err(WscStoreObstruction::invalid_wsc(wsc_digest));
            }
            let payload = atom_payload_bytes(&view, attachment, wsc_digest)?;
            let record = SubmissionAcceptanceRecord::from_payload_bytes(payload)
                .map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
            records.push(record);
        }
    }
    canonical_accepted_submission_records(&records)
}

/// Builds a generic WSC envelope for receipt and ticket correlation records.
///
/// # Errors
///
/// Returns a typed obstruction when generated WSC material fails validation.
pub fn receipt_correlation_records_to_wsc_envelope(
    receipts: &[TickReceiptRecord],
    correlations: &[WalReceiptCorrelationRecord],
) -> Result<WscStoreEnvelope, WscStoreObstruction> {
    let receipts = canonical_tick_receipts(receipts);
    let correlations = canonical_receipt_correlations(correlations);
    let mut store = GraphStore::new(make_warp_id(WSC_RECEIPT_CORRELATION_WARP));
    let root = make_node_id(WSC_RECEIPT_CORRELATION_ROOT);
    store.insert_node(
        root,
        NodeRecord {
            ty: make_type_id(WSC_RECEIPT_CORRELATION_NODE_TYPE),
        },
    );
    for receipt in &receipts {
        insert_receipt_material_node(
            &mut store,
            root,
            receipt_node_id(&receipt.receipt_digest),
            WSC_TICK_RECEIPT_ATTACHMENT_TYPE,
            receipt.to_payload_bytes(),
        );
    }
    for correlation in &correlations {
        insert_receipt_material_node(
            &mut store,
            root,
            correlation_node_id(&correlation.submission_id, &correlation.ticket_digest),
            WSC_RECEIPT_CORRELATION_ATTACHMENT_TYPE,
            correlation.to_payload_bytes(),
        );
    }
    let basis_digest = receipt_correlation_basis_digest(&receipts, &correlations);
    let input = build_one_warp_input(&store, root);
    let wsc_bytes = write_wsc_one_warp(&input, make_type_id(WSC_RECEIPT_CORRELATION_SCHEMA).0, 0)
        .map_err(|_| WscStoreObstruction::invalid_wsc(basis_digest))?;
    WscStoreEnvelope::validated(WscStoreRecordKind::CausalHistory, basis_digest, wsc_bytes)
}

/// Recovers receipt and ticket correlation records from a generic WSC envelope.
///
/// # Errors
///
/// Returns a typed WSC store obstruction when the envelope is not receipt
/// correlation material or when record payloads are malformed.
pub fn receipt_correlation_records_from_wsc_envelope(
    envelope: &WscStoreEnvelope,
) -> Result<WscReceiptCorrelationRecords, WscStoreObstruction> {
    if envelope.record_kind() != WscStoreRecordKind::CausalHistory {
        return Err(WscStoreObstruction::invalid_envelope(0));
    }
    let wsc_digest = *envelope.wsc_digest();
    let file = WscFile::from_bytes(envelope.wsc_bytes().to_vec())
        .map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
    validate_wsc(&file).map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
    if file.schema_hash() != &make_type_id(WSC_RECEIPT_CORRELATION_SCHEMA).0 {
        return Err(WscStoreObstruction::invalid_wsc(wsc_digest));
    }
    let view = file
        .warp_view(0)
        .map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
    let mut receipts = Vec::new();
    let mut correlations = Vec::new();
    for node_index in 0..view.nodes().len() {
        for attachment in view.node_attachments(node_index) {
            let payload = atom_payload_bytes(&view, attachment, wsc_digest)?;
            if attachment.type_or_warp == make_type_id(WSC_TICK_RECEIPT_ATTACHMENT_TYPE).0 {
                let receipt = TickReceiptRecord::from_payload_bytes(payload)
                    .map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
                receipts.push(receipt);
            } else if attachment.type_or_warp
                == make_type_id(WSC_RECEIPT_CORRELATION_ATTACHMENT_TYPE).0
            {
                let correlation = WalReceiptCorrelationRecord::from_payload_bytes(payload)
                    .map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
                correlations.push(correlation);
            } else {
                return Err(WscStoreObstruction::invalid_wsc(wsc_digest));
            }
        }
    }
    Ok(WscReceiptCorrelationRecords {
        receipts: canonical_tick_receipts(&receipts),
        correlations: canonical_receipt_correlations(&correlations),
    })
}

fn read_array<const N: usize>(bytes: &[u8], offset: usize) -> Result<[u8; N], WscStoreObstruction> {
    let end = offset
        .checked_add(N)
        .ok_or_else(|| WscStoreObstruction::invalid_envelope(offset))?;
    let slice = bytes
        .get(offset..end)
        .ok_or_else(|| WscStoreObstruction::invalid_envelope(offset))?;
    let mut out = [0; N];
    out.copy_from_slice(slice);
    Ok(out)
}

fn derive_envelope_id(
    record_kind: WscStoreRecordKind,
    basis_digest: &Hash,
    schema_hash: &Hash,
    tick: u64,
    wsc_digest: &Hash,
    wsc_len: u64,
) -> WscStoreEnvelopeId {
    let mut hasher = Hasher::new();
    hasher.update(WSC_STORE_ENVELOPE_ID_DOMAIN);
    hasher.update(&record_kind.code().to_le_bytes());
    hasher.update(basis_digest);
    hasher.update(schema_hash);
    hasher.update(&tick.to_le_bytes());
    hasher.update(wsc_digest);
    hasher.update(&wsc_len.to_le_bytes());
    WscStoreEnvelopeId(hasher.finalize().into())
}

fn digest_wsc_bytes(bytes: &[u8]) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(WSC_STORE_BYTES_DOMAIN);
    hasher.update(bytes);
    hasher.finalize().into()
}

fn canonical_accepted_submission_records(
    records: &[SubmissionAcceptanceRecord],
) -> Result<Vec<SubmissionAcceptanceRecord>, WscStoreObstruction> {
    let mut by_submission = BTreeMap::new();
    for record in records {
        if let Some(existing) = by_submission.get(&record.submission_id) {
            if existing != record {
                return Err(WscStoreObstruction::duplicate_mismatch(
                    WscStoreEnvelopeId::from_hash(record.submission_id),
                ));
            }
        }
        by_submission.insert(record.submission_id, *record);
    }
    Ok(by_submission.into_values().collect())
}

fn accepted_submission_basis_digest(records: &[SubmissionAcceptanceRecord]) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(WSC_ACCEPTED_SUBMISSION_BASIS_DOMAIN);
    for record in records {
        hasher.update(&record.to_payload_bytes());
    }
    hasher.finalize().into()
}

fn accepted_submission_node_id(submission_id: &Hash) -> NodeId {
    let mut hasher = Hasher::new();
    hasher.update(WSC_ACCEPTED_SUBMISSION_NODE_DOMAIN);
    hasher.update(submission_id);
    NodeId(hasher.finalize().into())
}

fn accepted_submission_edge_id(submission_id: &Hash) -> EdgeId {
    let mut hasher = Hasher::new();
    hasher.update(WSC_ACCEPTED_SUBMISSION_EDGE_DOMAIN);
    hasher.update(submission_id);
    EdgeId(hasher.finalize().into())
}

fn atom_payload_bytes<'a>(
    view: &'a super::view::WarpView<'a>,
    attachment: &AttRow,
    wsc_digest: Hash,
) -> Result<&'a [u8], WscStoreObstruction> {
    if !attachment.is_atom() {
        return Err(WscStoreObstruction::invalid_wsc(wsc_digest));
    }
    view.blob_for_attachment(attachment)
        .ok_or_else(|| WscStoreObstruction::invalid_wsc(wsc_digest))
}

fn canonical_tick_receipts(records: &[TickReceiptRecord]) -> Vec<TickReceiptRecord> {
    let mut by_receipt = BTreeMap::new();
    for record in records {
        by_receipt.insert(record.receipt_digest, *record);
    }
    by_receipt.into_values().collect()
}

fn canonical_receipt_correlations(
    records: &[WalReceiptCorrelationRecord],
) -> Vec<WalReceiptCorrelationRecord> {
    let mut by_correlation = BTreeMap::new();
    for record in records {
        by_correlation.insert(
            (
                record.submission_id,
                record.ticket_digest,
                record.receipt_digest,
            ),
            *record,
        );
    }
    by_correlation.into_values().collect()
}

fn insert_receipt_material_node(
    store: &mut GraphStore,
    root: NodeId,
    node: NodeId,
    attachment_type: &str,
    payload_bytes: Vec<u8>,
) {
    store.insert_node(
        node,
        NodeRecord {
            ty: make_type_id(WSC_RECEIPT_CORRELATION_NODE_TYPE),
        },
    );
    store.insert_edge(
        root,
        EdgeRecord {
            id: receipt_material_edge_id(&node.0),
            from: root,
            to: node,
            ty: make_type_id(WSC_RECEIPT_CORRELATION_EDGE_TYPE),
        },
    );
    store.set_node_attachment(
        node,
        Some(AttachmentValue::Atom(AtomPayload::new(
            make_type_id(attachment_type),
            Bytes::from(payload_bytes),
        ))),
    );
}

fn receipt_correlation_basis_digest(
    receipts: &[TickReceiptRecord],
    correlations: &[WalReceiptCorrelationRecord],
) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(WSC_RECEIPT_CORRELATION_BASIS_DOMAIN);
    for receipt in receipts {
        hasher.update(b"receipt");
        hasher.update(&receipt.to_payload_bytes());
    }
    for correlation in correlations {
        hasher.update(b"correlation");
        hasher.update(&correlation.to_payload_bytes());
    }
    hasher.finalize().into()
}

fn receipt_node_id(receipt_digest: &Hash) -> NodeId {
    let mut hasher = Hasher::new();
    hasher.update(WSC_RECEIPT_CORRELATION_NODE_DOMAIN);
    hasher.update(b"receipt");
    hasher.update(receipt_digest);
    NodeId(hasher.finalize().into())
}

fn correlation_node_id(submission_id: &Hash, ticket_digest: &Hash) -> NodeId {
    let mut hasher = Hasher::new();
    hasher.update(WSC_RECEIPT_CORRELATION_NODE_DOMAIN);
    hasher.update(b"correlation");
    hasher.update(submission_id);
    hasher.update(ticket_digest);
    NodeId(hasher.finalize().into())
}

fn receipt_material_edge_id(node_id: &Hash) -> EdgeId {
    let mut hasher = Hasher::new();
    hasher.update(WSC_RECEIPT_CORRELATION_EDGE_DOMAIN);
    hasher.update(node_id);
    EdgeId(hasher.finalize().into())
}
