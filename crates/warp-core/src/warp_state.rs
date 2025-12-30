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
    #[must_use]
    pub(crate) fn take_or_create_store(&mut self, warp_id: WarpId) -> GraphStore {
        self.stores
            .remove(&warp_id)
            .unwrap_or_else(|| GraphStore::new(warp_id))
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
        self.stores.insert(instance.warp_id, store);
        self.instances.insert(instance.warp_id, instance);
    }

    /// Deletes a warp instance, its store, and its metadata.
    ///
    /// Returns `true` if the instance existed.
    pub(crate) fn delete_instance(&mut self, warp_id: &WarpId) -> bool {
        let existed = self.instances.remove(warp_id).is_some();
        let store_existed = self.stores.remove(warp_id).is_some();
        debug_assert_eq!(
            existed, store_existed,
            "WarpState stores/instances desynced for warp_id: {warp_id:?}"
        );
        existed
    }
}
