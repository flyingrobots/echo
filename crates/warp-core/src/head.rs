// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Writer and reader head types for worldline-aware scheduling.
//!
//! A **head** is a control object describing a participant in the worldline
//! runtime. Writer heads advance a worldline's frontier state through
//! deterministic commit; reader heads observe historical state via replay.
//!
//! Heads are **not** private mutable stores. A worldline owns exactly one
//! mutable frontier state (see [`WorldlineFrontier`](super::worldline_state::WorldlineFrontier)).
//! Multiple writer heads may target the same worldline, executing serially in
//! canonical `(worldline_id, head_id)` order.
//!
//! # Identifier Policy
//!
//! [`HeadId`] is an opaque stable identifier derived from a domain-separated
//! hash of its creation label. It is not `TypeId`, not derived from mutable
//! runtime structure, and not dependent on the current contents of the head.

use std::collections::BTreeMap;

use crate::head_inbox::{HeadInbox, InboxAddress, InboxPolicy};
use crate::ident::Hash;
use crate::playback::PlaybackMode;
use crate::worldline::WorldlineId;

// =============================================================================
// HeadId
// =============================================================================

/// Opaque stable identifier for a head (writer or reader).
///
/// Derived from a domain-separated BLAKE3 hash of the head's creation label
/// (`"head:" || label`). Never derived from mutable runtime structure.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct HeadId(Hash);

impl HeadId {
    /// Inclusive minimum key used by internal `BTreeMap` range queries.
    pub(crate) const MIN: Self = Self([0u8; 32]);
    /// Inclusive maximum key used by internal `BTreeMap` range queries.
    pub(crate) const MAX: Self = Self([0xff; 32]);

    /// Returns the canonical byte representation of this id.
    #[must_use]
    pub fn as_bytes(&self) -> &Hash {
        &self.0
    }
}

/// Produces a stable, domain-separated head identifier (prefix `b"head:"`) using BLAKE3.
#[must_use]
pub fn make_head_id(label: &str) -> HeadId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"head:");
    hasher.update(label.as_bytes());
    HeadId(hasher.finalize().into())
}

// =============================================================================
// WriterHeadKey
// =============================================================================

/// Composite key identifying a writer head within its worldline.
///
/// Ordering is `(worldline_id, head_id)` for canonical scheduling.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct WriterHeadKey {
    /// The worldline this head targets.
    pub worldline_id: WorldlineId,
    /// The head identity within that worldline.
    pub head_id: HeadId,
}

// =============================================================================
// WriterHead
// =============================================================================

/// A writer head is a control object: identity, mode, and scheduling metadata.
///
/// It is **not** a private mutable store. All live mutation for a worldline
/// goes through deterministic commit against that worldline's single frontier
/// state.
#[derive(Clone, Debug)]
pub struct WriterHead {
    /// Composite key identifying this head (immutable after construction).
    ///
    /// Private to prevent mutation via `PlaybackHeadRegistry::get_mut()`,
    /// which would break the BTreeMap key invariant. Use [`key()`](WriterHead::key).
    key: WriterHeadKey,
    /// Current playback mode (paused, playing, seeking, etc.).
    ///
    /// Private so pause state is derived from one source of truth. Use
    /// [`mode()`](WriterHead::mode), [`pause()`](WriterHead::pause), and
    /// [`unpause()`](WriterHead::unpause) to read/mutate.
    mode: PlaybackMode,
    /// Per-head deterministic ingress inbox.
    inbox: HeadInbox,
    /// Optional public inbox address for application routing.
    public_inbox: Option<InboxAddress>,
    /// Whether this head is the default writer for its worldline.
    is_default_writer: bool,
}

impl WriterHead {
    /// Creates a new writer head in the given mode.
    ///
    /// The head is paused if and only if `mode` is [`PlaybackMode::Paused`].
    /// When adding new `PlaybackMode` variants, audit whether they should be
    /// treated as paused for scheduling purposes.
    #[must_use]
    pub fn new(key: WriterHeadKey, mode: PlaybackMode) -> Self {
        Self::with_routing(key, mode, InboxPolicy::AcceptAll, None, false)
    }

    /// Creates a new writer head with explicit inbox routing metadata.
    #[must_use]
    pub fn with_routing(
        key: WriterHeadKey,
        mode: PlaybackMode,
        inbox_policy: InboxPolicy,
        public_inbox: Option<InboxAddress>,
        is_default_writer: bool,
    ) -> Self {
        Self {
            key,
            mode,
            inbox: HeadInbox::new(key, inbox_policy),
            public_inbox,
            is_default_writer,
        }
    }

    /// Returns the composite key identifying this head.
    #[must_use]
    pub fn key(&self) -> &WriterHeadKey {
        &self.key
    }

    /// Returns the current playback mode.
    #[must_use]
    pub fn mode(&self) -> &PlaybackMode {
        &self.mode
    }

    /// Returns `true` if this head is paused.
    #[must_use]
    pub fn is_paused(&self) -> bool {
        matches!(self.mode, PlaybackMode::Paused)
    }

    /// Returns the head inbox.
    #[must_use]
    pub fn inbox(&self) -> &HeadInbox {
        &self.inbox
    }

    /// Returns a mutable reference to the head inbox.
    pub fn inbox_mut(&mut self) -> &mut HeadInbox {
        &mut self.inbox
    }

    /// Returns the public inbox address for this head, if one exists.
    #[must_use]
    pub fn public_inbox(&self) -> Option<&InboxAddress> {
        self.public_inbox.as_ref()
    }

    /// Returns `true` if this head is the default writer for its worldline.
    #[must_use]
    pub fn is_default_writer(&self) -> bool {
        self.is_default_writer
    }

    /// Pauses this head. The scheduler will skip it.
    pub fn pause(&mut self) {
        self.mode = PlaybackMode::Paused;
    }

    /// Unpauses this head and sets it to the given mode.
    ///
    /// # Panics
    ///
    /// Panics if `mode` is `Paused` (passing `Paused` would
    /// create an inconsistent state). This is a programmer error.
    pub fn unpause(&mut self, mode: PlaybackMode) {
        assert!(
            !matches!(mode, PlaybackMode::Paused),
            "unpause() called with PlaybackMode::Paused — use pause() instead"
        );
        self.mode = mode;
    }
}

// =============================================================================
// PlaybackHeadRegistry
// =============================================================================

/// Registry of all writer heads in the runtime.
///
/// Heads are stored in a `BTreeMap` keyed by [`WriterHeadKey`], which provides
/// canonical `(worldline_id, head_id)` iteration order — the exact order
/// required by the serial canonical scheduler.
#[derive(Clone, Debug, Default)]
pub struct PlaybackHeadRegistry {
    heads: BTreeMap<WriterHeadKey, WriterHead>,
}

impl PlaybackHeadRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a writer head. Returns the previous head if one existed at this key.
    pub fn insert(&mut self, head: WriterHead) -> Option<WriterHead> {
        self.heads.insert(head.key, head)
    }

    /// Removes a writer head by key. Returns the removed head if it existed.
    pub fn remove(&mut self, key: &WriterHeadKey) -> Option<WriterHead> {
        self.heads.remove(key)
    }

    /// Returns a reference to the writer head at the given key.
    #[must_use]
    pub fn get(&self, key: &WriterHeadKey) -> Option<&WriterHead> {
        self.heads.get(key)
    }

    /// Returns a mutable reference to the inbox for the given head.
    pub(crate) fn inbox_mut(&mut self, key: &WriterHeadKey) -> Option<&mut HeadInbox> {
        self.heads.get_mut(key).map(WriterHead::inbox_mut)
    }

    /// Returns the number of registered heads.
    #[must_use]
    pub fn len(&self) -> usize {
        self.heads.len()
    }

    /// Returns `true` if the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.heads.is_empty()
    }

    /// Iterates over all heads in canonical `(worldline_id, head_id)` order.
    pub fn iter(&self) -> impl Iterator<Item = (&WriterHeadKey, &WriterHead)> {
        self.heads.iter()
    }

    /// Returns all head keys for a given worldline, in canonical order.
    ///
    /// Uses BTreeMap range queries for O(log n + k) instead of a full scan.
    pub fn heads_for_worldline(
        &self,
        worldline_id: WorldlineId,
    ) -> impl Iterator<Item = &WriterHeadKey> {
        let start = WriterHeadKey {
            worldline_id,
            head_id: HeadId::MIN,
        };
        let end = WriterHeadKey {
            worldline_id,
            head_id: HeadId::MAX,
        };
        self.heads.range(start..=end).map(|(k, _)| k)
    }
}

// =============================================================================
// RunnableWriterSet
// =============================================================================

/// Ordered live index of writer heads that are eligible for scheduling.
///
/// A head is runnable if and only if it is not paused. The set maintains
/// canonical `(worldline_id, head_id)` ordering for deterministic iteration.
#[derive(Clone, Debug, Default)]
pub struct RunnableWriterSet {
    keys: Vec<WriterHeadKey>,
}

impl RunnableWriterSet {
    /// Creates an empty runnable set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Rebuilds the runnable set from the registry.
    ///
    /// This is the canonical way to update the set after head state changes.
    /// It iterates all heads in `BTreeMap` order (already canonical) and
    /// collects those that are not paused.
    pub fn rebuild(&mut self, registry: &PlaybackHeadRegistry) {
        self.keys.clear();
        for (key, head) in registry.iter() {
            if !head.is_paused() {
                self.keys.push(*key);
            }
        }
    }

    /// Iterates over runnable head keys in canonical order.
    pub fn iter(&self) -> impl Iterator<Item = &WriterHeadKey> {
        self.keys.iter()
    }

    /// Returns the number of runnable heads.
    #[must_use]
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// Returns `true` if no heads are runnable.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn wl(n: u8) -> WorldlineId {
        WorldlineId([n; 32])
    }

    fn hd(label: &str) -> HeadId {
        make_head_id(label)
    }

    fn make_head(key: WriterHeadKey, mode: PlaybackMode) -> WriterHead {
        WriterHead::new(key, mode)
    }

    #[test]
    fn head_id_domain_separation() {
        let a = make_head_id("foo");
        let b = make_head_id("bar");
        assert_ne!(a, b);
        // Stable
        assert_eq!(a, make_head_id("foo"));
    }

    #[test]
    fn head_id_does_not_collide_with_other_id_domains() {
        use crate::ident::{make_edge_id, make_node_id, make_type_id, make_warp_id};
        let label = "collision-test";
        let head = *make_head_id(label).as_bytes();
        assert_ne!(head, make_node_id(label).0);
        assert_ne!(head, make_type_id(label).0);
        assert_ne!(head, make_edge_id(label).0);
        assert_ne!(head, make_warp_id(label).0);
    }

    #[test]
    fn registry_crud() {
        let mut reg = PlaybackHeadRegistry::new();
        let key = WriterHeadKey {
            worldline_id: wl(1),
            head_id: hd("h1"),
        };
        let head = make_head(key, PlaybackMode::Play);

        assert!(reg.is_empty());
        assert!(reg.insert(head).is_none());
        assert_eq!(reg.len(), 1);
        assert!(reg.get(&key).is_some());

        let removed = reg.remove(&key);
        assert!(removed.is_some());
        assert!(reg.is_empty());
    }

    #[test]
    fn runnable_set_ordering() {
        let mut reg = PlaybackHeadRegistry::new();

        // Insert heads in non-canonical order
        let k3 = WriterHeadKey {
            worldline_id: wl(2),
            head_id: hd("h1"),
        };
        let k1 = WriterHeadKey {
            worldline_id: wl(1),
            head_id: hd("h1"),
        };
        let k2 = WriterHeadKey {
            worldline_id: wl(1),
            head_id: hd("h2"),
        };

        reg.insert(make_head(k3, PlaybackMode::Play));
        reg.insert(make_head(k1, PlaybackMode::Play));
        reg.insert(make_head(k2, PlaybackMode::Play));

        let mut runnable = RunnableWriterSet::new();
        runnable.rebuild(&reg);

        let keys: Vec<_> = runnable.iter().collect();
        assert_eq!(keys.len(), 3);

        // Must be in canonical (worldline_id, head_id) order
        for i in 1..keys.len() {
            assert!(
                keys[i - 1] < keys[i],
                "runnable set must be in canonical order"
            );
        }
    }

    #[test]
    fn paused_heads_excluded_from_runnable() {
        let mut reg = PlaybackHeadRegistry::new();
        let k1 = WriterHeadKey {
            worldline_id: wl(1),
            head_id: hd("active"),
        };
        let k2 = WriterHeadKey {
            worldline_id: wl(1),
            head_id: hd("paused"),
        };

        reg.insert(make_head(k1, PlaybackMode::Play));
        reg.insert(make_head(k2, PlaybackMode::Paused));

        let mut runnable = RunnableWriterSet::new();
        runnable.rebuild(&reg);

        assert_eq!(runnable.len(), 1);
        assert_eq!(*runnable.iter().next().unwrap(), k1);
    }

    #[test]
    fn multiple_heads_on_same_worldline() {
        let mut reg = PlaybackHeadRegistry::new();
        let wl1 = wl(1);

        for i in 0..5 {
            let key = WriterHeadKey {
                worldline_id: wl1,
                head_id: hd(&format!("head-{i}")),
            };
            reg.insert(make_head(key, PlaybackMode::Play));
        }

        let count = reg.heads_for_worldline(wl1).count();
        assert_eq!(count, 5);

        // All heads on same worldline should be in the runnable set
        let mut runnable = RunnableWriterSet::new();
        runnable.rebuild(&reg);
        assert_eq!(runnable.len(), 5);
    }

    #[test]
    #[should_panic(expected = "unpause() called with PlaybackMode::Paused")]
    fn unpause_rejects_paused_mode() {
        let key = WriterHeadKey {
            worldline_id: wl(1),
            head_id: hd("writer"),
        };
        let mut head = make_head(key, PlaybackMode::Play);
        head.unpause(PlaybackMode::Paused);
    }

    #[test]
    fn worldline_owns_one_frontier_state() {
        // This test documents the architectural invariant:
        // A worldline has exactly one frontier state, not per-head stores.
        // We verify this by showing that head registration does not create
        // any per-head state — heads are pure control objects.
        let key = WriterHeadKey {
            worldline_id: wl(1),
            head_id: hd("writer"),
        };
        let head = WriterHead::with_routing(
            key,
            PlaybackMode::Play,
            InboxPolicy::AcceptAll,
            Some(InboxAddress("orders".to_string())),
            true,
        );

        // WriterHead has no store field — it's a control object only
        assert_eq!(head.key().worldline_id, wl(1));
        assert!(!head.is_paused());
        assert!(head.is_default_writer());
        assert_eq!(
            head.public_inbox(),
            Some(&InboxAddress("orders".to_string()))
        );
        assert_eq!(head.inbox().head_key(), &key);
    }
}
