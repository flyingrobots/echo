// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Stage B1: multi-instance WARP state (flattened indirection).
//!
//! This module introduces **`WarpInstances`**: each descended attachment points
//! to a separate instance (a namespaced skeleton graph) rather than embedding a
//! recursive Rust data structure.

use std::collections::BTreeMap;

use crate::attachment::AttachmentKey;
use crate::graph::GraphStore;
use crate::ident::{NodeId, WarpId};

/// Metadata record describing one WARP instance (a “layer”).
///
/// Instances are addressed by [`WarpId`]. Each instance has a designated root
/// node id within its local skeleton store. Descended instances optionally
/// record the attachment slot that descends into them (`parent`), enabling
/// deterministic “include the portal chain” slicing without searching the
/// entire attachment plane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WarpInstance {
    /// Instance identifier (namespace for local node/edge ids).
    pub warp_id: WarpId,
    /// Root node id within the instance's local [`GraphStore`].
    pub root_node: NodeId,
    /// Attachment slot that descends into this instance (`None` for the root instance).
    pub parent: Option<AttachmentKey>,
}

/// Multi-instance WARP state: a collection of instance-scoped skeleton stores
/// plus instance metadata.
///
/// Determinism contract:
/// - All maps are `BTreeMap` for stable iteration order.
/// - Invariants that affect determinism (e.g., missing instance metadata) are
///   treated as internal corruption and should be prevented by constructors and
///   patch replay validation.
#[derive(Debug, Clone, Default)]
pub struct WarpState {
    pub(crate) stores: BTreeMap<WarpId, GraphStore>,
    pub(crate) instances: BTreeMap<WarpId, WarpInstance>,
    /// Instance-level touches since recording began.
    pub(crate) touched: StateTouchLog,
}

/// Instance-level counterpart of [`crate::graph::TouchLog`].
///
/// Records each instance's prior record on first change, and the prior
/// content of every store created, replaced, or removed while recording.
/// Those warps are diffed as whole stores; every other store reports its own
/// touched slots. Like the store log, it is bookkeeping: a clone starts idle
/// and `Debug` does not print it.
#[derive(Default)]
pub(crate) struct StateTouchLog {
    pub(crate) recording: bool,
    pub(crate) instances: BTreeMap<WarpId, Option<WarpInstance>>,
    /// Warps whose store was created, replaced, or removed, with the store's
    /// content as of the start of recording (`None` when it had no store).
    pub(crate) replaced_stores: BTreeMap<WarpId, Option<GraphStore>>,
}

impl Clone for StateTouchLog {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl std::fmt::Debug for StateTouchLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("StateTouchLog")
    }
}

impl WarpState {
    /// Creates an empty state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the instance metadata for `warp_id` (if present).
    #[must_use]
    pub fn instance(&self, warp_id: &WarpId) -> Option<&WarpInstance> {
        self.instances.get(warp_id)
    }

    /// Returns the skeleton store for `warp_id` (if present).
    #[must_use]
    pub fn store(&self, warp_id: &WarpId) -> Option<&GraphStore> {
        self.stores.get(warp_id)
    }

    /// Iterate over all instance metadata entries in deterministic order.
    pub(crate) fn iter_instances(&self) -> impl Iterator<Item = (&WarpId, &WarpInstance)> {
        self.instances.iter()
    }

    /// Iterate over all stores in deterministic order.
    pub(crate) fn iter_stores(&self) -> impl Iterator<Item = (&WarpId, &GraphStore)> {
        self.stores.iter()
    }

    /// Returns a mutable skeleton store for `warp_id` (if present).
    pub fn store_mut(&mut self, warp_id: &WarpId) -> Option<&mut GraphStore> {
        self.stores.get_mut(warp_id)
    }

    /// Removes and returns the store for `warp_id` if it exists; otherwise returns a new empty store.
    /// Starts recording touches on the state and every store.
    pub(crate) fn begin_touch_log(&mut self) {
        self.touched = StateTouchLog {
            recording: true,
            ..StateTouchLog::default()
        };
        for store in self.stores.values_mut() {
            store.begin_touch_log();
        }
    }

    /// Stops recording touches and discards every log.
    pub(crate) fn end_touch_log(&mut self) {
        self.touched = StateTouchLog::default();
        for store in self.stores.values_mut() {
            store.end_touch_log();
        }
    }

    fn touch_instance(&mut self, warp_id: WarpId) {
        if self.touched.recording {
            if let std::collections::btree_map::Entry::Vacant(slot) =
                self.touched.instances.entry(warp_id)
            {
                slot.insert(self.instances.get(&warp_id).cloned());
            }
        }
    }

    /// Keeps `prior`, the store being dropped for `warp_id`, as that warp's
    /// content at the start of recording, unless one is already kept.
    fn keep_replaced_store(&mut self, warp_id: WarpId, prior: Option<GraphStore>) {
        if let std::collections::btree_map::Entry::Vacant(slot) =
            self.touched.replaced_stores.entry(warp_id)
        {
            slot.insert(prior.map(|mut store| {
                store.revert_touches();
                store
            }));
        }
    }

    /// Removes the store for `warp_id`, or creates an empty one.
    ///
    /// While touches are recorded, a store taken here must be returned through
    /// [`Self::upsert_instance`]: its log travels with it, and dropping it
    /// would lose the prior values of its touched slots.
    #[must_use]
    pub(crate) fn take_or_create_store(&mut self, warp_id: WarpId) -> GraphStore {
        if let Some(store) = self.stores.remove(&warp_id) {
            return store;
        }
        if self.touched.recording {
            self.keep_replaced_store(warp_id, None);
        }
        GraphStore::new(warp_id)
    }

    /// Inserts or replaces the store and metadata for a warp instance.
    ///
    /// This is primarily used by patch replay and construction utilities.
    pub(crate) fn upsert_instance(&mut self, instance: WarpInstance, mut store: GraphStore) {
        debug_assert_eq!(
            store.warp_id, instance.warp_id,
            "GraphStore.warp_id must match WarpInstance.warp_id"
        );
        // Canonicalize the store's warp id to the instance id (the instance metadata is the source of truth).
        store.warp_id = instance.warp_id;
        if self.touched.recording {
            self.touch_instance(instance.warp_id);
            // A store that is not recording did not come from this state
            // during recording, so it replaces whatever was there.
            if !store.touched.recording {
                let prior = self.stores.remove(&instance.warp_id);
                self.keep_replaced_store(instance.warp_id, prior);
            }
        }
        self.stores.insert(instance.warp_id, store);
        self.instances.insert(instance.warp_id, instance);
    }

    /// Deletes a warp instance, its store, and its metadata.
    ///
    /// Returns `true` if the instance existed.
    pub(crate) fn delete_instance(&mut self, warp_id: &WarpId) -> bool {
        self.touch_instance(*warp_id);
        let existed = self.instances.remove(warp_id).is_some();
        let removed_store = self.stores.remove(warp_id);
        let store_existed = removed_store.is_some();
        if self.touched.recording {
            self.keep_replaced_store(*warp_id, removed_store);
        }
        debug_assert_eq!(
            existed, store_existed,
            "WarpState stores/instances desynced for warp_id: {warp_id:?}"
        );
        existed
    }
}
