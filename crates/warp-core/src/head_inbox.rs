// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Deterministic ingress model and per-head inbox policy (ADR-0008 Phase 3).
//!
//! This module introduces the unified [`IngressEnvelope`] model and the
//! [`HeadInbox`] that replaces raw per-head byte queues with deterministic,
//! content-addressed ingress.
//!
//! # Design Notes
//!
//! - `TypeId` is banned here. Stable kind identifiers only ([`IntentKind`]).
//! - `pending` is keyed by content address for deterministic order and idempotence.
//! - Routing uses [`IngressTarget`]: application traffic targets `DefaultWriter`
//!   or `InboxAddress`, control/debug traffic may target `ExactHead`.

use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;

use crate::head::WriterHeadKey;
use crate::ident::Hash;
use crate::worldline::WorldlineId;
use crate::{CausalTickReceiptRef, CAUSAL_TICK_RECEIPT_REF_LEN};

const RETAINED_INGRESS_ENVELOPE_MAGIC_V1: &[u8; 8] = b"EINGR001";
const RETAINED_INGRESS_ENVELOPE_MAGIC_V2: &[u8; 8] = b"EINGR002";

// =============================================================================
// IntentKind
// =============================================================================

/// Stable, content-addressed intent kind identifier.
///
/// This is **not** a Rust `TypeId`. It is a domain-separated BLAKE3 hash of
/// the intent kind label, ensuring stability across compiler versions and
/// platforms.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct IntentKind(Hash);

impl IntentKind {
    /// Reconstructs a stable intent kind from its canonical hash.
    #[must_use]
    pub const fn from_hash(hash: Hash) -> Self {
        Self(hash)
    }

    /// Returns the canonical hash backing this stable intent-kind identifier.
    #[must_use]
    pub fn as_hash(&self) -> &Hash {
        &self.0
    }
}

/// Produces a stable, domain-separated intent kind identifier.
#[must_use]
pub fn make_intent_kind(label: &str) -> IntentKind {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"intent-kind:");
    hasher.update(label.as_bytes());
    IntentKind(hasher.finalize().into())
}

// =============================================================================
// IngressTarget
// =============================================================================

/// Named inbox address within a worldline.
///
/// Inbox addresses are human-readable string aliases (not content-addressed
/// hashes). They allow multiple logical entry points per worldline without
/// exposing internal head identities to application code.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct InboxAddress(pub String);

/// Routing target for an ingress envelope.
///
/// Application code targets worldlines or named inbox addresses.
/// Exact-head routing is for control/debug/admin paths only.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum IngressTarget {
    /// Route to the default writer head for the given worldline.
    DefaultWriter {
        /// Target worldline.
        worldline_id: WorldlineId,
    },
    /// Route to a named inbox address within a worldline.
    InboxAddress {
        /// Target worldline.
        worldline_id: WorldlineId,
        /// Named inbox within that worldline.
        inbox: InboxAddress,
    },
    /// Route to a specific head (control/debug only).
    ExactHead {
        /// The exact head key to target.
        key: WriterHeadKey,
    },
}

impl IngressTarget {
    /// Returns the worldline targeted by this ingress.
    #[must_use]
    pub fn worldline_id(&self) -> WorldlineId {
        match self {
            Self::DefaultWriter { worldline_id } | Self::InboxAddress { worldline_id, .. } => {
                *worldline_id
            }
            Self::ExactHead { key } => key.worldline_id,
        }
    }
}

// =============================================================================
// IngressPayload
// =============================================================================

/// Payload carried by an ingress envelope.
///
/// Early phases use only `LocalIntent`. Cross-worldline messages and imports
/// are added in Phases 10 and 11 respectively.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IngressPayload {
    /// A local intent from the application layer.
    LocalIntent {
        /// Stable kind identifier for this intent.
        intent_kind: IntentKind,
        /// Raw intent bytes (content-addressed).
        intent_bytes: Vec<u8>,
    },
    // Phase 10: CrossWorldlineMessage { ... }
    // Phase 11: ImportedPatch { ... }
    // Phase 9C: ConflictArtifact { ... }
}

/// Typed retained causal evidence cited by an ingress claim.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum IngressCausalParent {
    /// Scheduler-owned tick receipt cited by a later contract intent.
    TickReceipt {
        /// Exact causal coordinate of the retained target receipt.
        receipt_ref: CausalTickReceiptRef,
    },
    /// Exact receipt whose contract transition an admitted inverse derives from.
    ContractInverseTarget {
        /// Exact causal coordinate of the transition being inverted.
        receipt_ref: CausalTickReceiptRef,
    },
}

impl IngressCausalParent {
    /// Returns the cited causal tick-receipt coordinate.
    #[must_use]
    pub const fn receipt_ref(self) -> CausalTickReceiptRef {
        match self {
            Self::TickReceipt { receipt_ref } | Self::ContractInverseTarget { receipt_ref } => {
                receipt_ref
            }
        }
    }
}

// =============================================================================
// IngressEnvelope
// =============================================================================

/// Content-addressed, deterministic ingress envelope.
///
/// All inbound work flows through this envelope model:
/// - content-addressed by `ingress_id` for idempotence,
/// - deterministically routed via `target`,
/// - causally linked via `causal_parents`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IngressEnvelope {
    /// Content address of this envelope (BLAKE3 of payload).
    ingress_id: Hash,
    /// Routing target.
    target: IngressTarget,
    /// Causal parent references (empty for local intents in early phases).
    causal_parents: Vec<IngressCausalParent>,
    /// The payload.
    payload: IngressPayload,
}

impl IngressEnvelope {
    /// Creates a new local intent envelope with auto-computed ingress_id.
    #[must_use]
    pub fn local_intent(
        target: IngressTarget,
        intent_kind: IntentKind,
        intent_bytes: Vec<u8>,
    ) -> Self {
        Self::local_intent_with_causal_parents(target, intent_kind, intent_bytes, Vec::new())
    }

    /// Creates a local intent that explicitly cites retained causal evidence.
    ///
    /// Parent references are canonicalized as a set. The trusted contract-inverse
    /// admission path uses the typed inverse-target role; ordinary runtime
    /// submission rejects that reserved role.
    #[must_use]
    pub fn local_intent_with_causal_parents(
        target: IngressTarget,
        intent_kind: IntentKind,
        intent_bytes: Vec<u8>,
        mut causal_parents: Vec<IngressCausalParent>,
    ) -> Self {
        causal_parents.sort_unstable();
        causal_parents.dedup();
        let ingress_id = compute_ingress_id(&intent_kind, &intent_bytes, &causal_parents);
        Self {
            ingress_id,
            target,
            causal_parents,
            payload: IngressPayload::LocalIntent {
                intent_kind,
                intent_bytes,
            },
        }
    }

    /// Returns the canonical content address of this envelope.
    #[must_use]
    pub fn ingress_id(&self) -> Hash {
        self.ingress_id
    }

    /// Returns the routing target for this envelope.
    #[must_use]
    pub fn target(&self) -> &IngressTarget {
        &self.target
    }

    /// Returns the payload carried by this envelope.
    #[must_use]
    pub fn payload(&self) -> &IngressPayload {
        &self.payload
    }

    /// Returns the causal parents for this envelope.
    #[must_use]
    pub fn causal_parents(&self) -> &[IngressCausalParent] {
        &self.causal_parents
    }

    /// Returns the canonical receipt set cited by every typed causal relation.
    #[must_use]
    pub(crate) fn canonical_causal_parent_receipt_refs(&self) -> Vec<CausalTickReceiptRef> {
        let mut receipt_refs = self
            .causal_parents
            .iter()
            .copied()
            .map(IngressCausalParent::receipt_ref)
            .collect::<Vec<_>>();
        receipt_refs.sort_unstable();
        receipt_refs.dedup();
        receipt_refs
    }

    /// Encodes this envelope as bounded, versioned retained material.
    ///
    /// These bytes preserve the canonical claim needed for restart replay. The
    /// ingress id remains the semantic content address; this retention codec is
    /// a storage format and does not replace that identity law.
    #[must_use]
    pub fn to_retained_bytes_v2(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(RETAINED_INGRESS_ENVELOPE_MAGIC_V2);
        match &self.target {
            IngressTarget::DefaultWriter { worldline_id } => {
                out.push(1);
                out.extend_from_slice(worldline_id.as_bytes());
            }
            IngressTarget::InboxAddress {
                worldline_id,
                inbox,
            } => {
                out.push(2);
                out.extend_from_slice(worldline_id.as_bytes());
                push_len(&mut out, inbox.0.len());
                out.extend_from_slice(inbox.0.as_bytes());
            }
            IngressTarget::ExactHead { key } => {
                out.push(3);
                out.extend_from_slice(key.worldline_id.as_bytes());
                out.extend_from_slice(key.head_id.as_bytes());
            }
        }
        push_len(&mut out, self.causal_parents.len());
        for parent in &self.causal_parents {
            match parent {
                IngressCausalParent::TickReceipt { receipt_ref } => {
                    out.push(1);
                    out.extend_from_slice(&receipt_ref.to_canonical_bytes());
                }
                IngressCausalParent::ContractInverseTarget { receipt_ref } => {
                    out.push(2);
                    out.extend_from_slice(&receipt_ref.to_canonical_bytes());
                }
            }
        }
        match &self.payload {
            IngressPayload::LocalIntent {
                intent_kind,
                intent_bytes,
            } => {
                out.push(1);
                out.extend_from_slice(intent_kind.as_hash());
                push_len(&mut out, intent_bytes.len());
                out.extend_from_slice(intent_bytes);
            }
        }
        out
    }

    /// Decodes retained ingress material without admitting or executing it.
    ///
    /// # Errors
    ///
    /// Returns a typed decode error for truncated, malformed, unsupported, or
    /// non-canonical retained material.
    pub fn from_retained_bytes(bytes: &[u8]) -> Result<Self, IngressEnvelopeDecodeError> {
        let magic = bytes
            .get(..RETAINED_INGRESS_ENVELOPE_MAGIC_V2.len())
            .ok_or(IngressEnvelopeDecodeError::UnexpectedEof)?;
        if magic == RETAINED_INGRESS_ENVELOPE_MAGIC_V2 {
            return Self::from_retained_bytes_v2(bytes);
        }
        if magic == RETAINED_INGRESS_ENVELOPE_MAGIC_V1 {
            return Self::from_retained_bytes_v1(bytes);
        }
        Err(IngressEnvelopeDecodeError::InvalidMagic)
    }

    fn from_retained_bytes_v2(bytes: &[u8]) -> Result<Self, IngressEnvelopeDecodeError> {
        let mut cursor = RetainedIngressCursor::new(bytes);
        if cursor.read_exact(RETAINED_INGRESS_ENVELOPE_MAGIC_V2.len())?
            != RETAINED_INGRESS_ENVELOPE_MAGIC_V2
        {
            return Err(IngressEnvelopeDecodeError::InvalidMagic);
        }
        let target = match cursor.read_u8()? {
            1 => IngressTarget::DefaultWriter {
                worldline_id: WorldlineId::from_bytes(cursor.read_hash()?),
            },
            2 => {
                let worldline_id = WorldlineId::from_bytes(cursor.read_hash()?);
                let inbox_len = cursor.read_len()?;
                let inbox = String::from_utf8(cursor.read_exact(inbox_len)?.to_vec())
                    .map_err(|_| IngressEnvelopeDecodeError::InvalidInboxUtf8)?;
                IngressTarget::InboxAddress {
                    worldline_id,
                    inbox: InboxAddress(inbox),
                }
            }
            3 => IngressTarget::ExactHead {
                key: WriterHeadKey {
                    worldline_id: WorldlineId::from_bytes(cursor.read_hash()?),
                    head_id: crate::head::HeadId::from_bytes(cursor.read_hash()?),
                },
            },
            tag => return Err(IngressEnvelopeDecodeError::UnknownTargetTag(tag)),
        };
        let parent_count = cursor.read_len()?;
        if parent_count > cursor.remaining_len() / (1 + CAUSAL_TICK_RECEIPT_REF_LEN) {
            return Err(IngressEnvelopeDecodeError::UnexpectedEof);
        }
        let mut causal_parents = Vec::with_capacity(parent_count);
        for _ in 0..parent_count {
            let parent = match cursor.read_u8()? {
                1 => IngressCausalParent::TickReceipt {
                    receipt_ref: CausalTickReceiptRef::from_canonical_bytes(
                        cursor
                            .read_exact(CAUSAL_TICK_RECEIPT_REF_LEN)?
                            .try_into()
                            .map_err(|_| IngressEnvelopeDecodeError::UnexpectedEof)?,
                    ),
                },
                2 => IngressCausalParent::ContractInverseTarget {
                    receipt_ref: CausalTickReceiptRef::from_canonical_bytes(
                        cursor
                            .read_exact(CAUSAL_TICK_RECEIPT_REF_LEN)?
                            .try_into()
                            .map_err(|_| IngressEnvelopeDecodeError::UnexpectedEof)?,
                    ),
                },
                tag => return Err(IngressEnvelopeDecodeError::UnknownCausalParentTag(tag)),
            };
            causal_parents.push(parent);
        }
        let envelope = match cursor.read_u8()? {
            1 => {
                let intent_kind = IntentKind::from_hash(cursor.read_hash()?);
                let intent_len = cursor.read_len()?;
                let intent_bytes = cursor.read_exact(intent_len)?.to_vec();
                Self::local_intent_with_causal_parents(
                    target,
                    intent_kind,
                    intent_bytes,
                    causal_parents,
                )
            }
            tag => return Err(IngressEnvelopeDecodeError::UnknownPayloadTag(tag)),
        };
        cursor.finish()?;
        if envelope.to_retained_bytes_v2() != bytes {
            return Err(IngressEnvelopeDecodeError::NonCanonical);
        }
        Ok(envelope)
    }

    fn from_retained_bytes_v1(bytes: &[u8]) -> Result<Self, IngressEnvelopeDecodeError> {
        let mut cursor = RetainedIngressCursor::new(bytes);
        if cursor.read_exact(RETAINED_INGRESS_ENVELOPE_MAGIC_V1.len())?
            != RETAINED_INGRESS_ENVELOPE_MAGIC_V1
        {
            return Err(IngressEnvelopeDecodeError::InvalidMagic);
        }
        let target = match cursor.read_u8()? {
            1 => IngressTarget::DefaultWriter {
                worldline_id: WorldlineId::from_bytes(cursor.read_hash()?),
            },
            2 => {
                let worldline_id = WorldlineId::from_bytes(cursor.read_hash()?);
                let inbox_len = cursor.read_len()?;
                let inbox = String::from_utf8(cursor.read_exact(inbox_len)?.to_vec())
                    .map_err(|_| IngressEnvelopeDecodeError::InvalidInboxUtf8)?;
                IngressTarget::InboxAddress {
                    worldline_id,
                    inbox: InboxAddress(inbox),
                }
            }
            3 => IngressTarget::ExactHead {
                key: WriterHeadKey {
                    worldline_id: WorldlineId::from_bytes(cursor.read_hash()?),
                    head_id: crate::head::HeadId::from_bytes(cursor.read_hash()?),
                },
            },
            tag => return Err(IngressEnvelopeDecodeError::UnknownTargetTag(tag)),
        };
        let parent_count = cursor.read_len()?;
        if parent_count > cursor.remaining_len() / (1 + core::mem::size_of::<Hash>()) {
            return Err(IngressEnvelopeDecodeError::UnexpectedEof);
        }
        let mut legacy_parent_digests = Vec::with_capacity(parent_count);
        for _ in 0..parent_count {
            match cursor.read_u8()? {
                1 => legacy_parent_digests.push(cursor.read_hash()?),
                tag => return Err(IngressEnvelopeDecodeError::UnknownCausalParentTag(tag)),
            }
        }
        let (intent_kind, intent_bytes) = match cursor.read_u8()? {
            1 => {
                let intent_kind = IntentKind::from_hash(cursor.read_hash()?);
                let intent_len = cursor.read_len()?;
                let intent_bytes = cursor.read_exact(intent_len)?.to_vec();
                (intent_kind, intent_bytes)
            }
            tag => return Err(IngressEnvelopeDecodeError::UnknownPayloadTag(tag)),
        };
        cursor.finish()?;
        if let Some(receipt_digest) = legacy_parent_digests.first().copied() {
            let mut canonical_parents = legacy_parent_digests.clone();
            canonical_parents.sort_unstable();
            canonical_parents.dedup();
            if canonical_parents != legacy_parent_digests {
                return Err(IngressEnvelopeDecodeError::NonCanonical);
            }
            return Err(
                IngressEnvelopeDecodeError::AmbiguousLegacyTickReceiptParent { receipt_digest },
            );
        }
        let envelope = Self::local_intent(target, intent_kind, intent_bytes);
        let mut canonical = envelope.to_retained_bytes_v2();
        canonical[..RETAINED_INGRESS_ENVELOPE_MAGIC_V1.len()]
            .copy_from_slice(RETAINED_INGRESS_ENVELOPE_MAGIC_V1);
        if canonical != bytes {
            return Err(IngressEnvelopeDecodeError::NonCanonical);
        }
        Ok(envelope)
    }

    fn expected_ingress_id(&self) -> Hash {
        match &self.payload {
            IngressPayload::LocalIntent {
                intent_kind,
                intent_bytes,
            } => compute_ingress_id(intent_kind, intent_bytes, &self.causal_parents),
        }
    }

    fn assert_canonical_ingress_id(&self) {
        assert_eq!(
            self.ingress_id,
            self.expected_ingress_id(),
            "ingress_id does not match payload — envelope was constructed incorrectly"
        );
    }
}

/// Error returned while decoding retained ingress envelope material.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum IngressEnvelopeDecodeError {
    /// Retained bytes ended before the declared value was complete.
    #[error("retained ingress envelope ended unexpectedly")]
    UnexpectedEof,
    /// Retained bytes did not carry a supported envelope magic.
    #[error("retained ingress envelope has invalid magic")]
    InvalidMagic,
    /// Retained target tag is unknown.
    #[error("retained ingress envelope has unknown target tag {0}")]
    UnknownTargetTag(u8),
    /// Retained causal parent tag is unknown.
    #[error("retained ingress envelope has unknown causal parent tag {0}")]
    UnknownCausalParentTag(u8),
    /// A v1 parent names receipt content but not one admitted receipt event.
    #[error("legacy retained ingress cites ambiguous tick receipt digest {receipt_digest:?}")]
    AmbiguousLegacyTickReceiptParent {
        /// Bare receipt-content digest that cannot identify one causal event.
        receipt_digest: Hash,
    },
    /// Retained payload tag is unknown.
    #[error("retained ingress envelope has unknown payload tag {0}")]
    UnknownPayloadTag(u8),
    /// Named inbox material was not valid UTF-8.
    #[error("retained ingress envelope inbox is not valid UTF-8")]
    InvalidInboxUtf8,
    /// Retained bytes decoded but did not use the canonical versioned encoding.
    #[error("retained ingress envelope is not canonically encoded")]
    NonCanonical,
    /// Retained bytes contained trailing material outside the envelope.
    #[error("retained ingress envelope has trailing bytes")]
    TrailingBytes,
}

fn push_len(out: &mut Vec<u8>, len: usize) {
    out.extend_from_slice(&(len as u64).to_le_bytes());
}

struct RetainedIngressCursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> RetainedIngressCursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn remaining_len(&self) -> usize {
        self.bytes.len().saturating_sub(self.offset)
    }

    fn read_exact(&mut self, len: usize) -> Result<&'a [u8], IngressEnvelopeDecodeError> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or(IngressEnvelopeDecodeError::UnexpectedEof)?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(IngressEnvelopeDecodeError::UnexpectedEof)?;
        self.offset = end;
        Ok(value)
    }

    fn read_u8(&mut self) -> Result<u8, IngressEnvelopeDecodeError> {
        Ok(self.read_exact(1)?[0])
    }

    fn read_hash(&mut self) -> Result<Hash, IngressEnvelopeDecodeError> {
        self.read_exact(core::mem::size_of::<Hash>())?
            .try_into()
            .map_err(|_| IngressEnvelopeDecodeError::UnexpectedEof)
    }

    fn read_len(&mut self) -> Result<usize, IngressEnvelopeDecodeError> {
        let raw = u64::from_le_bytes(
            self.read_exact(core::mem::size_of::<u64>())?
                .try_into()
                .map_err(|_| IngressEnvelopeDecodeError::UnexpectedEof)?,
        );
        usize::try_from(raw).map_err(|_| IngressEnvelopeDecodeError::UnexpectedEof)
    }

    fn finish(self) -> Result<(), IngressEnvelopeDecodeError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(IngressEnvelopeDecodeError::TrailingBytes)
        }
    }
}

/// Computes the content address of a local intent.
///
/// Parentless intents preserve the original
/// `BLAKE3("ingress:" || kind_hash || bytes)` identity. Causal intents use a
/// separate versioned domain with explicit lengths so the same contract bytes
/// cited against different receipt evidence cannot collapse as duplicates.
fn compute_ingress_id(
    kind: &IntentKind,
    bytes: &[u8],
    causal_parents: &[IngressCausalParent],
) -> Hash {
    let mut hasher = blake3::Hasher::new();
    if !causal_parents.is_empty() {
        hasher.update(b"ingress:causal:v2\0");
        hasher.update(kind.as_hash());
        hasher.update(&(bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
        hasher.update(&(causal_parents.len() as u64).to_le_bytes());
        for parent in causal_parents {
            match parent {
                IngressCausalParent::TickReceipt { receipt_ref } => {
                    hasher.update(b"tick-receipt\0");
                    hasher.update(&receipt_ref.to_canonical_bytes());
                }
                IngressCausalParent::ContractInverseTarget { receipt_ref } => {
                    hasher.update(b"contract-inverse-target\0");
                    hasher.update(&receipt_ref.to_canonical_bytes());
                }
            }
        }
        return hasher.finalize().into();
    }
    hasher.update(b"ingress:");
    hasher.update(kind.as_hash());
    hasher.update(bytes);
    hasher.finalize().into()
}

// =============================================================================
// InboxPolicy
// =============================================================================

/// Policy controlling which envelopes a head's inbox will accept.
#[derive(Clone, Debug)]
pub enum InboxPolicy {
    /// Accept all envelopes.
    AcceptAll,
    /// Accept only envelopes whose intent kind is in the filter set.
    KindFilter(BTreeSet<IntentKind>),
    /// Accept up to `max_per_tick` envelopes per SuperTick.
    Budgeted {
        /// Maximum envelopes to admit per SuperTick.
        max_per_tick: u32,
    },
}

impl Default for InboxPolicy {
    fn default() -> Self {
        Self::AcceptAll
    }
}

/// Outcome of attempting to ingest an envelope into a head inbox.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InboxIngestResult {
    /// The envelope was accepted and stored as pending.
    Accepted,
    /// The envelope was already pending (idempotent retry).
    Duplicate,
    /// The envelope was rejected by the current inbox policy.
    Rejected,
}

// =============================================================================
// HeadInbox
// =============================================================================

/// Per-head inbox with deterministic admission and idempotent deduplication.
///
/// Pending envelopes are stored in a `BTreeMap` keyed by `ingress_id`
/// (content address), which provides:
/// - deterministic iteration order,
/// - automatic deduplication (re-ingesting the same envelope is a no-op).
#[derive(Clone, Debug)]
pub struct HeadInbox {
    head_key: WriterHeadKey,
    pending: BTreeMap<Hash, IngressEnvelope>,
    policy: InboxPolicy,
}

impl Default for HeadInbox {
    fn default() -> Self {
        Self {
            head_key: WriterHeadKey {
                worldline_id: WorldlineId::from_bytes([0u8; 32]),
                head_id: crate::head::HeadId::MIN,
            },
            pending: BTreeMap::new(),
            policy: InboxPolicy::AcceptAll,
        }
    }
}

impl HeadInbox {
    /// Creates a new inbox with the given policy.
    #[must_use]
    pub fn new(head_key: WriterHeadKey, policy: InboxPolicy) -> Self {
        Self {
            head_key,
            pending: BTreeMap::new(),
            policy,
        }
    }

    /// Returns the writer head that owns this inbox.
    #[must_use]
    pub fn head_key(&self) -> &WriterHeadKey {
        &self.head_key
    }

    /// Ingests an envelope if it passes the inbox policy.
    ///
    /// Returns the ingest outcome for this envelope. Envelopes that do not
    /// match a [`InboxPolicy::KindFilter`] are rejected at ingest time
    /// (never stored).
    pub fn ingest(&mut self, envelope: IngressEnvelope) -> InboxIngestResult {
        use std::collections::btree_map::Entry;

        // Invariant: content-addressed envelopes must remain canonical even in
        // release builds. Invalid ids indicate a programming error upstream.
        envelope.assert_canonical_ingress_id();
        let ingress_id = envelope.ingress_id();

        // Early rejection: check policy before storing.
        if !self.policy_accepts(&envelope) {
            return InboxIngestResult::Rejected;
        }

        match self.pending.entry(ingress_id) {
            Entry::Vacant(v) => {
                v.insert(envelope);
                InboxIngestResult::Accepted
            }
            Entry::Occupied(_) => InboxIngestResult::Duplicate,
        }
    }

    /// Returns `true` when this inbox policy would accept the envelope.
    ///
    /// This performs the policy check without storing the envelope. It is used
    /// by witnessed submission intake so Echo can record accepted ingress
    /// history without entering runtime scheduling.
    #[must_use]
    pub fn would_accept(&self, envelope: &IngressEnvelope) -> bool {
        envelope.assert_canonical_ingress_id();
        self.policy_accepts(envelope)
    }

    /// Returns `true` if the policy would accept this envelope.
    fn policy_accepts(&self, envelope: &IngressEnvelope) -> bool {
        match &self.policy {
            InboxPolicy::AcceptAll | InboxPolicy::Budgeted { .. } => true,
            InboxPolicy::KindFilter(allowed) => match &envelope.payload {
                IngressPayload::LocalIntent { intent_kind, .. } => allowed.contains(intent_kind),
            },
        }
    }

    /// Admits pending envelopes according to the inbox policy.
    ///
    /// Returns the admitted envelopes in deterministic (`ingress_id`) order
    /// and removes them from the pending set.
    pub fn admit(&mut self) -> Vec<IngressEnvelope> {
        match &self.policy {
            InboxPolicy::AcceptAll | InboxPolicy::KindFilter(_) => {
                // Drain all pending envelopes (already policy-compliant via
                // ingest-time filtering for KindFilter).
                std::mem::take(&mut self.pending).into_values().collect()
            }
            InboxPolicy::Budgeted { max_per_tick } => {
                let limit = *max_per_tick as usize;
                let reserve = limit.min(self.pending.len());
                let mut admitted = Vec::with_capacity(reserve);
                let mut to_remove = Vec::with_capacity(reserve);
                for (id, env) in &self.pending {
                    if admitted.len() >= limit {
                        break;
                    }
                    admitted.push(env.clone());
                    to_remove.push(*id);
                }
                for id in to_remove {
                    self.pending.remove(&id);
                }
                admitted
            }
        }
    }

    /// Admits one deterministic execution category without mixing it with
    /// other pending categories.
    ///
    /// The lowest canonical ingress id chooses whether this batch contains the
    /// supplied `partition_kind` or everything else. Entries from the other
    /// category remain pending for a later Tick. Existing per-Tick limits still
    /// bound the selected category.
    #[cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
    pub(crate) fn admit_partitioned(
        &mut self,
        partition_kind: IntentKind,
        partition_limit: usize,
    ) -> Vec<IngressEnvelope> {
        let Some(first) = self.pending.first_key_value().map(|(_, envelope)| envelope) else {
            return Vec::new();
        };
        let selected_partition = matches!(
            first.payload(),
            IngressPayload::LocalIntent { intent_kind, .. }
                if *intent_kind == partition_kind
        );
        let policy_limit = match self.policy {
            InboxPolicy::Budgeted { max_per_tick } => max_per_tick as usize,
            InboxPolicy::AcceptAll | InboxPolicy::KindFilter(_) => usize::MAX,
        };
        let limit = if selected_partition {
            policy_limit.min(partition_limit)
        } else {
            policy_limit
        };
        if limit == 0 {
            return Vec::new();
        }

        let mut selected_ids = Vec::new();
        for (ingress_id, envelope) in &self.pending {
            let in_partition = matches!(
                envelope.payload(),
                IngressPayload::LocalIntent { intent_kind, .. }
                    if *intent_kind == partition_kind
            );
            if in_partition == selected_partition {
                selected_ids.push(*ingress_id);
                if selected_ids.len() == limit {
                    break;
                }
            }
        }
        selected_ids
            .into_iter()
            .filter_map(|ingress_id| self.pending.remove(&ingress_id))
            .collect()
    }

    /// Returns `true` if calling [`HeadInbox::admit`] would yield at least one envelope.
    #[must_use]
    pub fn can_admit(&self) -> bool {
        match &self.policy {
            InboxPolicy::AcceptAll | InboxPolicy::KindFilter(_) => !self.pending.is_empty(),
            InboxPolicy::Budgeted { max_per_tick } => *max_per_tick > 0 && !self.pending.is_empty(),
        }
    }

    /// Returns the number of pending envelopes.
    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Returns `true` if there are no pending envelopes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// Returns a reference to the current policy.
    #[must_use]
    pub fn policy(&self) -> &InboxPolicy {
        &self.policy
    }

    /// Sets a new inbox policy.
    ///
    /// Pending envelopes that no longer pass the new policy are evicted
    /// immediately. This prevents envelopes accepted under a permissive
    /// policy from bypassing a stricter one.
    pub fn set_policy(&mut self, policy: InboxPolicy) {
        self.policy = policy;
        // Revalidate pending against the new policy. Borrow `self.policy`
        // separately from `self.pending` to satisfy the borrow checker.
        let policy_ref = &self.policy;
        self.pending.retain(|_, env| match policy_ref {
            InboxPolicy::AcceptAll | InboxPolicy::Budgeted { .. } => true,
            InboxPolicy::KindFilter(allowed) => match &env.payload {
                IngressPayload::LocalIntent { intent_kind, .. } => allowed.contains(intent_kind),
            },
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wl(n: u8) -> WorldlineId {
        WorldlineId::from_bytes([n; 32])
    }

    fn test_kind() -> IntentKind {
        make_intent_kind("test/action")
    }

    fn other_kind() -> IntentKind {
        make_intent_kind("test/other")
    }

    fn make_envelope(kind: IntentKind, bytes: &[u8]) -> IngressEnvelope {
        IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter {
                worldline_id: wl(1),
            },
            kind,
            bytes.to_vec(),
        )
    }

    #[test]
    fn intent_kind_domain_separation() {
        let a = make_intent_kind("foo");
        let b = make_intent_kind("bar");
        assert_ne!(a, b);
        assert_eq!(a, make_intent_kind("foo"));
    }

    #[test]
    fn deterministic_admission_order() {
        let mut inbox = HeadInbox::new(
            WriterHeadKey {
                worldline_id: wl(1),
                head_id: crate::head::make_head_id("default"),
            },
            InboxPolicy::AcceptAll,
        );
        let kind = test_kind();

        // Insert in non-deterministic order (content addresses will sort)
        inbox.ingest(make_envelope(kind, b"zzz"));
        inbox.ingest(make_envelope(kind, b"aaa"));
        inbox.ingest(make_envelope(kind, b"mmm"));

        let admitted = inbox.admit();
        assert_eq!(admitted.len(), 3);

        // Must be in ingress_id order (BTreeMap guarantees this)
        for i in 1..admitted.len() {
            assert!(
                admitted[i - 1].ingress_id() < admitted[i].ingress_id(),
                "admission must be in ingress_id order"
            );
        }
    }

    #[cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
    #[test]
    fn partitioned_admission_never_mixes_execution_categories() {
        let mut inbox = HeadInbox::new(
            WriterHeadKey {
                worldline_id: wl(1),
                head_id: crate::head::make_head_id("default"),
            },
            InboxPolicy::AcceptAll,
        );
        let partition_kind = test_kind();
        let envelopes = [
            make_envelope(partition_kind, b"partition-a"),
            make_envelope(other_kind(), b"legacy-a"),
            make_envelope(partition_kind, b"partition-b"),
            make_envelope(other_kind(), b"legacy-b"),
        ];
        let first_is_partition = envelopes
            .iter()
            .min_by_key(|envelope| envelope.ingress_id())
            .is_some_and(|envelope| {
                matches!(
                    envelope.payload(),
                    IngressPayload::LocalIntent { intent_kind, .. }
                        if *intent_kind == partition_kind
                )
            });
        for envelope in envelopes {
            assert_eq!(inbox.ingest(envelope), InboxIngestResult::Accepted);
        }

        let first_batch = inbox.admit_partitioned(partition_kind, 2);
        assert_eq!(first_batch.len(), 2);
        assert!(first_batch.iter().all(|envelope| {
            matches!(
                envelope.payload(),
                IngressPayload::LocalIntent { intent_kind, .. }
                    if (*intent_kind == partition_kind) == first_is_partition
            )
        }));
        assert_eq!(inbox.pending_count(), 2);

        let second_batch = inbox.admit_partitioned(partition_kind, 2);
        assert_eq!(second_batch.len(), 2);
        assert!(second_batch.iter().all(|envelope| {
            matches!(
                envelope.payload(),
                IngressPayload::LocalIntent { intent_kind, .. }
                    if (*intent_kind == partition_kind) != first_is_partition
            )
        }));
        assert!(inbox.is_empty());
    }

    #[test]
    fn re_ingesting_same_envelope_is_idempotent() {
        let mut inbox = HeadInbox::new(
            WriterHeadKey {
                worldline_id: wl(1),
                head_id: crate::head::make_head_id("default"),
            },
            InboxPolicy::AcceptAll,
        );
        let env = make_envelope(test_kind(), b"payload");

        assert_eq!(inbox.ingest(env.clone()), InboxIngestResult::Accepted);
        assert_eq!(inbox.ingest(env), InboxIngestResult::Duplicate);
        assert_eq!(inbox.pending_count(), 1);
    }

    #[test]
    fn budget_enforcement() {
        let mut inbox = HeadInbox::new(
            WriterHeadKey {
                worldline_id: wl(1),
                head_id: crate::head::make_head_id("default"),
            },
            InboxPolicy::Budgeted { max_per_tick: 2 },
        );
        let kind = test_kind();

        inbox.ingest(make_envelope(kind, b"a"));
        inbox.ingest(make_envelope(kind, b"b"));
        inbox.ingest(make_envelope(kind, b"c"));

        let admitted = inbox.admit();
        assert_eq!(admitted.len(), 2, "budget should limit to 2");
        assert_eq!(inbox.pending_count(), 1, "one should remain pending");
    }

    #[test]
    fn kind_filter_rejects_non_matching_at_ingest() {
        let mut allowed = BTreeSet::new();
        allowed.insert(test_kind());
        let mut inbox = HeadInbox::new(
            WriterHeadKey {
                worldline_id: wl(1),
                head_id: crate::head::make_head_id("default"),
            },
            InboxPolicy::KindFilter(allowed),
        );

        assert_eq!(
            inbox.ingest(make_envelope(test_kind(), b"accepted")),
            InboxIngestResult::Accepted
        );
        assert!(
            inbox.ingest(make_envelope(other_kind(), b"rejected")) == InboxIngestResult::Rejected,
            "non-matching kind must be rejected at ingest"
        );

        // Only the matching envelope should be pending
        assert_eq!(inbox.pending_count(), 1);

        let admitted = inbox.admit();
        assert_eq!(admitted.len(), 1);
        assert!(inbox.is_empty(), "all pending admitted");
    }

    #[test]
    fn routing_to_named_inbox() {
        let target = IngressTarget::InboxAddress {
            worldline_id: wl(1),
            inbox: InboxAddress("orders".to_string()),
        };
        assert_eq!(target.worldline_id(), wl(1));
    }

    #[test]
    fn ingress_id_is_content_addressed() {
        let kind = test_kind();
        let env1 = make_envelope(kind, b"same-payload");
        let env2 = make_envelope(kind, b"same-payload");
        assert_eq!(
            env1.ingress_id(),
            env2.ingress_id(),
            "same payload must produce same ingress_id"
        );

        let env3 = make_envelope(kind, b"different-payload");
        assert_ne!(
            env1.ingress_id(),
            env3.ingress_id(),
            "different payload must produce different ingress_id"
        );
    }

    #[test]
    fn policy_tightening_evicts_non_matching() {
        let mut inbox = HeadInbox::new(
            WriterHeadKey {
                worldline_id: wl(1),
                head_id: crate::head::make_head_id("default"),
            },
            InboxPolicy::AcceptAll,
        );
        inbox.ingest(make_envelope(test_kind(), b"kept"));
        inbox.ingest(make_envelope(other_kind(), b"evicted"));
        assert_eq!(inbox.pending_count(), 2);

        // Tighten policy to only accept test_kind
        let mut allowed = BTreeSet::new();
        allowed.insert(test_kind());
        inbox.set_policy(InboxPolicy::KindFilter(allowed));

        assert_eq!(
            inbox.pending_count(),
            1,
            "non-matching envelope must be evicted on policy change"
        );

        let admitted = inbox.admit();
        assert_eq!(admitted.len(), 1);
    }

    #[test]
    fn admit_clears_pending() {
        let mut inbox = HeadInbox::new(
            WriterHeadKey {
                worldline_id: wl(1),
                head_id: crate::head::make_head_id("default"),
            },
            InboxPolicy::AcceptAll,
        );
        inbox.ingest(make_envelope(test_kind(), b"data"));
        assert_eq!(inbox.pending_count(), 1);

        inbox.admit();
        assert!(inbox.is_empty());
    }

    #[test]
    #[should_panic(expected = "ingress_id does not match payload")]
    fn invalid_envelope_panics_on_ingest() {
        let mut inbox = HeadInbox::new(
            WriterHeadKey {
                worldline_id: wl(1),
                head_id: crate::head::make_head_id("default"),
            },
            InboxPolicy::AcceptAll,
        );
        let mut envelope = make_envelope(test_kind(), b"payload");
        envelope.ingress_id = [0xff; 32];
        let _ = inbox.ingest(envelope);
    }

    #[test]
    #[should_panic(expected = "ingress_id does not match payload")]
    fn invalid_envelope_panics_on_would_accept() {
        let inbox = HeadInbox::new(
            WriterHeadKey {
                worldline_id: wl(1),
                head_id: crate::head::make_head_id("default"),
            },
            InboxPolicy::AcceptAll,
        );
        let mut envelope = make_envelope(test_kind(), b"payload");
        envelope.ingress_id = [0xfe; 32];
        let _ = inbox.would_accept(&envelope);
    }
}
