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

use crate::head::WriterHeadKey;
use crate::ident::Hash;
use crate::worldline::WorldlineId;

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
#[derive(Clone, Debug)]
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

// =============================================================================
// IngressEnvelope
// =============================================================================

/// Content-addressed, deterministic ingress envelope.
///
/// All inbound work flows through this envelope model:
/// - content-addressed by `ingress_id` for idempotence,
/// - deterministically routed via `target`,
/// - causally linked via `causal_parents`.
#[derive(Clone, Debug)]
pub struct IngressEnvelope {
    /// Content address of this envelope (BLAKE3 of payload).
    ingress_id: Hash,
    /// Routing target.
    target: IngressTarget,
    /// Causal parent references (empty for local intents in early phases).
    causal_parents: Vec<Hash>,
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
        let ingress_id = compute_ingress_id(&intent_kind, &intent_bytes);
        Self {
            ingress_id,
            target,
            causal_parents: Vec::new(),
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
    pub fn causal_parents(&self) -> &[Hash] {
        &self.causal_parents
    }

    fn expected_ingress_id(&self) -> Hash {
        match &self.payload {
            IngressPayload::LocalIntent {
                intent_kind,
                intent_bytes,
            } => compute_ingress_id(intent_kind, intent_bytes),
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

/// Computes the content address of a local intent.
///
/// Hash structure: `BLAKE3("ingress:" || kind_hash || bytes)`.
/// No length prefix is needed because the kind hash is always exactly 32 bytes
/// (`Hash = [u8; 32]`), so the boundary between kind and payload is
/// unambiguous.
fn compute_ingress_id(kind: &IntentKind, bytes: &[u8]) -> Hash {
    let mut hasher = blake3::Hasher::new();
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
}
