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
pub struct IntentKind(pub Hash);

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
/// Inbox addresses allow multiple logical entry points per worldline without
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
    pub ingress_id: Hash,
    /// Routing target.
    pub target: IngressTarget,
    /// Causal parent references (empty for local intents in early phases).
    pub causal_parents: Vec<Hash>,
    /// The payload.
    pub payload: IngressPayload,
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
}

/// Computes the content address of a local intent.
fn compute_ingress_id(kind: &IntentKind, bytes: &[u8]) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"ingress:");
    hasher.update(&kind.0);
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

// =============================================================================
// HeadInbox
// =============================================================================

/// Per-head inbox with deterministic admission and idempotent deduplication.
///
/// Pending envelopes are stored in a `BTreeMap` keyed by `ingress_id`
/// (content address), which provides:
/// - deterministic iteration order,
/// - automatic deduplication (re-ingesting the same envelope is a no-op).
#[derive(Clone, Debug, Default)]
pub struct HeadInbox {
    pending: BTreeMap<Hash, IngressEnvelope>,
    policy: InboxPolicy,
}

impl HeadInbox {
    /// Creates a new inbox with the given policy.
    #[must_use]
    pub fn new(policy: InboxPolicy) -> Self {
        Self {
            pending: BTreeMap::new(),
            policy,
        }
    }

    /// Ingests an envelope. Returns `true` if it was new, `false` if duplicate.
    pub fn ingest(&mut self, envelope: IngressEnvelope) -> bool {
        use std::collections::btree_map::Entry;
        match self.pending.entry(envelope.ingress_id) {
            Entry::Vacant(v) => {
                v.insert(envelope);
                true
            }
            Entry::Occupied(_) => false,
        }
    }

    /// Admits pending envelopes according to the inbox policy.
    ///
    /// Returns the admitted envelopes in deterministic (`ingress_id`) order
    /// and removes them from the pending set.
    pub fn admit(&mut self) -> Vec<IngressEnvelope> {
        match &self.policy {
            InboxPolicy::AcceptAll => {
                let admitted: Vec<_> = self.pending.values().cloned().collect();
                self.pending.clear();
                admitted
            }
            InboxPolicy::KindFilter(allowed) => {
                let mut admitted = Vec::new();
                let mut to_remove = Vec::new();
                for (id, env) in &self.pending {
                    let dominated = match &env.payload {
                        IngressPayload::LocalIntent { intent_kind, .. } => {
                            allowed.contains(intent_kind)
                        }
                    };
                    if dominated {
                        admitted.push(env.clone());
                        to_remove.push(*id);
                    }
                }
                for id in to_remove {
                    self.pending.remove(&id);
                }
                admitted
            }
            InboxPolicy::Budgeted { max_per_tick } => {
                let limit = *max_per_tick as usize;
                let mut admitted = Vec::with_capacity(limit);
                let mut to_remove = Vec::with_capacity(limit);
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
    pub fn set_policy(&mut self, policy: InboxPolicy) {
        self.policy = policy;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wl(n: u8) -> WorldlineId {
        WorldlineId([n; 32])
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
        let mut inbox = HeadInbox::new(InboxPolicy::AcceptAll);
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
                admitted[i - 1].ingress_id < admitted[i].ingress_id,
                "admission must be in ingress_id order"
            );
        }
    }

    #[test]
    fn re_ingesting_same_envelope_is_idempotent() {
        let mut inbox = HeadInbox::new(InboxPolicy::AcceptAll);
        let env = make_envelope(test_kind(), b"payload");

        assert!(inbox.ingest(env.clone()));
        assert!(!inbox.ingest(env));
        assert_eq!(inbox.pending_count(), 1);
    }

    #[test]
    fn budget_enforcement() {
        let mut inbox = HeadInbox::new(InboxPolicy::Budgeted { max_per_tick: 2 });
        let kind = test_kind();

        inbox.ingest(make_envelope(kind, b"a"));
        inbox.ingest(make_envelope(kind, b"b"));
        inbox.ingest(make_envelope(kind, b"c"));

        let admitted = inbox.admit();
        assert_eq!(admitted.len(), 2, "budget should limit to 2");
        assert_eq!(inbox.pending_count(), 1, "one should remain pending");
    }

    #[test]
    fn kind_filter_admits_only_matching() {
        let mut allowed = BTreeSet::new();
        allowed.insert(test_kind());
        let mut inbox = HeadInbox::new(InboxPolicy::KindFilter(allowed));

        inbox.ingest(make_envelope(test_kind(), b"accepted"));
        inbox.ingest(make_envelope(other_kind(), b"rejected"));

        let admitted = inbox.admit();
        assert_eq!(admitted.len(), 1);
        assert_eq!(inbox.pending_count(), 1, "rejected envelope stays pending");
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
            env1.ingress_id, env2.ingress_id,
            "same payload must produce same ingress_id"
        );

        let env3 = make_envelope(kind, b"different-payload");
        assert_ne!(
            env1.ingress_id, env3.ingress_id,
            "different payload must produce different ingress_id"
        );
    }

    #[test]
    fn admit_clears_pending() {
        let mut inbox = HeadInbox::new(InboxPolicy::AcceptAll);
        inbox.ingest(make_envelope(test_kind(), b"data"));
        assert_eq!(inbox.pending_count(), 1);

        inbox.admit();
        assert!(inbox.is_empty());
    }
}
