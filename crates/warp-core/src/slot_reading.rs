// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Slot readings resolved from tick patches without materializing a graph.
//!
//! Every [`WarpOp`] is an absolute write. The value of a slot at a cursor is
//! therefore the value that the latest operation on that slot before the
//! cursor wrote, or the value in the lane's initial boundary when no operation
//! wrote it. [`SlotWriteIndex`] records, for each slot, the positions of the
//! operations that wrote it, so a reading resolves one slot without replaying
//! unrelated ticks.
//!
//! The index is derived from patches and can always be rebuilt from them. It
//! is a reading aid, never an authority.

use std::collections::BTreeMap;

use thiserror::Error;

use crate::attachment::{AttachmentKey, AttachmentOwner, AttachmentValue};
use crate::ident::{EdgeKey, NodeKey, WarpId};
use crate::record::{EdgeRecord, NodeRecord};
use crate::tick_patch::{PortalInit, SlotId, WarpOp, WarpTickPatchV1};
use crate::warp_state::WarpState;

/// Position of one operation in a lane's patch sequence: tick index, then
/// operation index within that tick's canonical operation order.
type OpPosition = (usize, usize);

/// Value of one slot in a reading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlotReading {
    /// A skeleton node record, or `None` when the node is absent.
    Node(Option<NodeRecord>),
    /// A skeleton edge record, or `None` when the edge is absent.
    Edge(Option<EdgeRecord>),
    /// An attachment value, or `None` when the slot holds no attachment.
    Attachment(Option<AttachmentValue>),
}

/// Reasons a slot reading cannot be produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum SlotReadingError {
    /// The cursor names more ticks than the index or the patches cover.
    #[error("cursor {cursor} lies beyond the {available} available ticks")]
    CursorBeyondHistory {
        /// Requested cursor coordinate.
        cursor: usize,
        /// Number of ticks covered by both the index and the supplied patches.
        available: usize,
    },
    /// Port slots carry boundary values, not graph values.
    #[error("port slots carry no graph value")]
    UnsupportedSlot,
    /// The index names an operation that the supplied patches do not contain.
    #[error("the supplied patches do not match the index")]
    PatchMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InstanceEvent {
    /// The instance exists after this operation.
    Present,
    /// The instance and every slot in it are gone after this operation.
    Deleted,
}

/// Index from a slot to the operations that wrote it.
///
/// Tick `n` of the index is the `n`-th pushed patch. A cursor `c` reads the
/// boundary after patches `0..c`, matching replay cursor coordinates.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SlotWriteIndex {
    writes: BTreeMap<SlotId, Vec<OpPosition>>,
    instance_events: BTreeMap<WarpId, Vec<(OpPosition, InstanceEvent)>>,
    tick_count: usize,
}

impl SlotWriteIndex {
    /// Creates an index over no ticks.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the number of indexed ticks.
    #[must_use]
    pub fn tick_count(&self) -> usize {
        self.tick_count
    }

    /// Indexes the next tick's patch.
    pub fn push_patch(&mut self, patch: &WarpTickPatchV1) {
        let tick = self.tick_count;
        for (op_index, op) in patch.ops().iter().enumerate() {
            let position = (tick, op_index);
            for slot in written_slots(op).into_iter().flatten() {
                self.writes.entry(slot).or_default().push(position);
            }
            if let Some((warp_id, event)) = instance_event(op) {
                self.instance_events
                    .entry(warp_id)
                    .or_default()
                    .push((position, event));
            }
        }
        self.tick_count += 1;
    }

    /// Removes the ticks at and after `tick_count`.
    ///
    /// `removed` must be the patches of exactly those ticks, in order. The
    /// cost is proportional to the operations removed, not to the index.
    pub fn truncate(&mut self, tick_count: usize, removed: &[WarpTickPatchV1]) {
        if tick_count >= self.tick_count {
            return;
        }
        for patch in removed {
            for op in patch.ops() {
                for slot in written_slots(op).into_iter().flatten() {
                    if let Some(positions) = self.writes.get_mut(&slot) {
                        while positions.last().is_some_and(|last| last.0 >= tick_count) {
                            positions.pop();
                        }
                        if positions.is_empty() {
                            self.writes.remove(&slot);
                        }
                    }
                }
                if let Some((warp_id, _)) = instance_event(op) {
                    if let Some(events) = self.instance_events.get_mut(&warp_id) {
                        while events.last().is_some_and(|last| last.0 .0 >= tick_count) {
                            events.pop();
                        }
                        if events.is_empty() {
                            self.instance_events.remove(&warp_id);
                        }
                    }
                }
            }
        }
        self.tick_count = tick_count;
    }

    /// Reads `slot` at `cursor`: the boundary after patches `0..cursor`.
    ///
    /// `patches` must be the patches this index was built from, in order, and
    /// `initial` the boundary they apply to.
    ///
    /// # Errors
    ///
    /// Returns [`SlotReadingError`] when the cursor lies beyond the indexed
    /// ticks, the slot is a port slot, or the patches do not match the index.
    pub fn read_slot(
        &self,
        patches: &[WarpTickPatchV1],
        initial: &WarpState,
        slot: SlotId,
        cursor: usize,
    ) -> Result<SlotReading, SlotReadingError> {
        let available = self.tick_count.min(patches.len());
        if cursor > available {
            return Err(SlotReadingError::CursorBeyondHistory { cursor, available });
        }
        let warp_id = slot_warp(slot).ok_or(SlotReadingError::UnsupportedSlot)?;
        let bound = (cursor, 0);

        let events = self
            .instance_events
            .get(&warp_id)
            .map_or(&[][..], Vec::as_slice);
        let before = events.partition_point(|(position, _)| *position < bound);
        let preceding = events.get(..before).unwrap_or(&[]);
        let present = preceding.last().map_or_else(
            || initial.store(&warp_id).is_some(),
            |(_, event)| *event == InstanceEvent::Present,
        );
        if !present {
            return Ok(absent(slot));
        }
        // Deleting an instance removes every slot in it. A later instance with
        // the same id starts empty, so writes before the deletion do not count.
        let deleted_at = preceding
            .iter()
            .rev()
            .find(|(_, event)| *event == InstanceEvent::Deleted)
            .map(|(position, _)| *position);

        let latest_write = self.writes.get(&slot).and_then(|positions| {
            let before = positions.partition_point(|position| *position < bound);
            before
                .checked_sub(1)
                .and_then(|index| positions.get(index))
                .copied()
        });
        match (latest_write, deleted_at) {
            (Some(write), Some(deleted)) if write < deleted => Ok(absent(slot)),
            (Some(write), _) => written_value(patches, write, slot),
            (None, Some(_)) => Ok(absent(slot)),
            (None, None) => Ok(materialized_slot(initial, slot)),
        }
    }
}

/// Reads `slot` directly from a materialized state.
///
/// # Panics
///
/// Never panics. A port slot reads as an absent attachment.
#[must_use]
pub(crate) fn materialized_slot(state: &WarpState, slot: SlotId) -> SlotReading {
    match slot {
        SlotId::Node(node) => SlotReading::Node(
            state
                .store(&node.warp_id)
                .and_then(|store| store.node(&node.local_id))
                .cloned(),
        ),
        SlotId::Edge(edge) => SlotReading::Edge(state.store(&edge.warp_id).and_then(|store| {
            let from = store.edge_index.get(&edge.local_id)?;
            store
                .edges_from(from)
                .find(|record| record.id == edge.local_id)
                .cloned()
        })),
        SlotId::Attachment(key) => SlotReading::Attachment(match key.owner {
            AttachmentOwner::Node(node) => state
                .store(&node.warp_id)
                .and_then(|store| store.node_attachment(&node.local_id))
                .cloned(),
            AttachmentOwner::Edge(edge) => state
                .store(&edge.warp_id)
                .and_then(|store| store.edge_attachment(&edge.local_id))
                .cloned(),
        }),
        SlotId::Port(_) => SlotReading::Attachment(None),
    }
}

const fn absent(slot: SlotId) -> SlotReading {
    match slot {
        SlotId::Node(_) => SlotReading::Node(None),
        SlotId::Edge(_) => SlotReading::Edge(None),
        SlotId::Attachment(_) | SlotId::Port(_) => SlotReading::Attachment(None),
    }
}

const fn slot_warp(slot: SlotId) -> Option<WarpId> {
    match slot {
        SlotId::Node(node) => Some(node.warp_id),
        SlotId::Edge(edge) => Some(edge.warp_id),
        SlotId::Attachment(key) => Some(match key.owner {
            AttachmentOwner::Node(node) => node.warp_id,
            AttachmentOwner::Edge(edge) => edge.warp_id,
        }),
        SlotId::Port(_) => None,
    }
}

/// Slots whose value `op` sets, including values it clears as a consequence.
fn written_slots(op: &WarpOp) -> [Option<SlotId>; 2] {
    match op {
        WarpOp::UpsertNode { node, .. } => [Some(SlotId::Node(*node)), None],
        // Deleting a node also drops its attachment.
        WarpOp::DeleteNode { node } => [
            Some(SlotId::Node(*node)),
            Some(SlotId::Attachment(AttachmentKey::node_alpha(*node))),
        ],
        WarpOp::UpsertEdge { warp_id, record } => [
            Some(SlotId::Edge(EdgeKey {
                warp_id: *warp_id,
                local_id: record.id,
            })),
            None,
        ],
        // Deleting an edge also drops its attachment.
        WarpOp::DeleteEdge {
            warp_id, edge_id, ..
        } => {
            let edge = EdgeKey {
                warp_id: *warp_id,
                local_id: *edge_id,
            };
            [
                Some(SlotId::Edge(edge)),
                Some(SlotId::Attachment(AttachmentKey::edge_beta(edge))),
            ]
        }
        WarpOp::SetAttachment { key, .. } => [Some(SlotId::Attachment(*key)), None],
        // An empty-initialized portal leaves the child root equal to its
        // record whether or not the child instance already existed.
        WarpOp::OpenPortal {
            key,
            child_warp,
            child_root,
            init,
        } => [
            Some(SlotId::Attachment(*key)),
            match init {
                PortalInit::Empty { .. } => Some(SlotId::Node(NodeKey {
                    warp_id: *child_warp,
                    local_id: *child_root,
                })),
                PortalInit::RequireExisting => None,
            },
        ],
        WarpOp::UpsertWarpInstance { .. } | WarpOp::DeleteWarpInstance { .. } => [None, None],
    }
}

const fn instance_event(op: &WarpOp) -> Option<(WarpId, InstanceEvent)> {
    match op {
        WarpOp::UpsertWarpInstance { instance } => Some((instance.warp_id, InstanceEvent::Present)),
        WarpOp::OpenPortal { child_warp, .. } => Some((*child_warp, InstanceEvent::Present)),
        WarpOp::DeleteWarpInstance { warp_id } => Some((*warp_id, InstanceEvent::Deleted)),
        WarpOp::UpsertNode { .. }
        | WarpOp::DeleteNode { .. }
        | WarpOp::UpsertEdge { .. }
        | WarpOp::DeleteEdge { .. }
        | WarpOp::SetAttachment { .. } => None,
    }
}

/// Returns the value that the operation at `position` wrote to `slot`.
fn written_value(
    patches: &[WarpTickPatchV1],
    position: OpPosition,
    slot: SlotId,
) -> Result<SlotReading, SlotReadingError> {
    let op = patches
        .get(position.0)
        .and_then(|patch| patch.ops().get(position.1))
        .ok_or(SlotReadingError::PatchMismatch)?;
    let reading = match (op, slot) {
        (WarpOp::UpsertNode { node, record }, SlotId::Node(key)) if *node == key => {
            SlotReading::Node(Some(record.clone()))
        }
        (WarpOp::DeleteNode { node }, SlotId::Node(key)) if *node == key => SlotReading::Node(None),
        (WarpOp::DeleteNode { node }, SlotId::Attachment(key))
            if AttachmentKey::node_alpha(*node) == key =>
        {
            SlotReading::Attachment(None)
        }
        (WarpOp::UpsertEdge { warp_id, record }, SlotId::Edge(key))
            if *warp_id == key.warp_id && record.id == key.local_id =>
        {
            SlotReading::Edge(Some(record.clone()))
        }
        (
            WarpOp::DeleteEdge {
                warp_id, edge_id, ..
            },
            SlotId::Edge(key),
        ) if *warp_id == key.warp_id && *edge_id == key.local_id => SlotReading::Edge(None),
        (
            WarpOp::DeleteEdge {
                warp_id, edge_id, ..
            },
            SlotId::Attachment(key),
        ) if AttachmentKey::edge_beta(EdgeKey {
            warp_id: *warp_id,
            local_id: *edge_id,
        }) == key =>
        {
            SlotReading::Attachment(None)
        }
        (
            WarpOp::SetAttachment {
                key: written,
                value,
            },
            SlotId::Attachment(key),
        ) if *written == key => SlotReading::Attachment(value.clone()),
        (
            WarpOp::OpenPortal {
                key: written,
                child_warp,
                ..
            },
            SlotId::Attachment(key),
        ) if *written == key => {
            SlotReading::Attachment(Some(AttachmentValue::Descend(*child_warp)))
        }
        (
            WarpOp::OpenPortal {
                child_warp,
                child_root,
                init: PortalInit::Empty { root_record },
                ..
            },
            SlotId::Node(key),
        ) if *child_warp == key.warp_id && *child_root == key.local_id => {
            SlotReading::Node(Some(root_record.clone()))
        }
        _ => return Err(SlotReadingError::PatchMismatch),
    };
    Ok(reading)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use bytes::Bytes;

    use super::*;
    use crate::attachment::AtomPayload;
    use crate::graph::GraphStore;
    use crate::ident::{make_edge_id, make_node_id, make_type_id, make_warp_id, EdgeId, NodeId};
    use crate::tick_patch::TickCommitStatus;
    use crate::warp_state::WarpInstance;

    /// Deterministic generator for fixed-seed histories.
    struct Sequence(u64);

    impl Sequence {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }

        fn pick<T: Copy>(&mut self, items: &[T]) -> T {
            let len = u64::try_from(items.len()).unwrap();
            items[usize::try_from(self.next() % len).unwrap()]
        }
    }

    struct Universe {
        warps: [WarpId; 2],
        nodes: [NodeId; 4],
        edges: [EdgeId; 3],
    }

    impl Universe {
        fn new() -> Self {
            Self {
                warps: [make_warp_id("slot-root"), make_warp_id("slot-child")],
                nodes: [
                    make_node_id("slot-a"),
                    make_node_id("slot-b"),
                    make_node_id("slot-c"),
                    make_node_id("slot-d"),
                ],
                edges: [
                    make_edge_id("slot-e1"),
                    make_edge_id("slot-e2"),
                    make_edge_id("slot-e3"),
                ],
            }
        }

        fn slots(&self) -> Vec<SlotId> {
            let mut slots = Vec::new();
            for warp_id in self.warps {
                for local_id in self.nodes {
                    let node = NodeKey { warp_id, local_id };
                    slots.push(SlotId::Node(node));
                    slots.push(SlotId::Attachment(AttachmentKey::node_alpha(node)));
                }
                for local_id in self.edges {
                    let edge = EdgeKey { warp_id, local_id };
                    slots.push(SlotId::Edge(edge));
                    slots.push(SlotId::Attachment(AttachmentKey::edge_beta(edge)));
                }
            }
            slots
        }

        fn initial(&self) -> WarpState {
            let [root, _] = self.warps;
            let mut store = GraphStore::new(root);
            for label in ["initial-a", "initial-b"] {
                let (id, ty) = match label {
                    "initial-a" => (self.nodes[0], make_type_id("initial-a")),
                    _ => (self.nodes[1], make_type_id("initial-b")),
                };
                store.insert_node(id, NodeRecord { ty });
            }
            store.set_node_attachment(self.nodes[1], Some(atom(7)));
            store.upsert_edge_record(
                self.nodes[0],
                EdgeRecord {
                    id: self.edges[0],
                    from: self.nodes[0],
                    to: self.nodes[1],
                    ty: make_type_id("initial-edge"),
                },
            );
            store.set_edge_attachment(self.edges[0], Some(atom(9)));
            let mut state = WarpState::new();
            state.upsert_instance(
                WarpInstance {
                    warp_id: root,
                    root_node: self.nodes[0],
                    parent: None,
                },
                store,
            );
            state
        }

        fn op(&self, sequence: &mut Sequence) -> WarpOp {
            let warp_id = sequence.pick(&self.warps);
            let node = NodeKey {
                warp_id,
                local_id: sequence.pick(&self.nodes),
            };
            let edge_id = sequence.pick(&self.edges);
            let edge = EdgeKey {
                warp_id,
                local_id: edge_id,
            };
            let from = sequence.pick(&self.nodes);
            let ty =
                make_type_id(["t0", "t1", "t2"][usize::try_from(sequence.next() % 3).unwrap()]);
            let value = (sequence.next() % 3 != 0)
                .then(|| atom(u8::try_from(sequence.next() % 251).unwrap()));
            let [_, child] = self.warps;
            let portal_owner = self.portal_owner();
            match sequence.next() % 12 {
                0 | 1 => WarpOp::UpsertNode {
                    node,
                    record: NodeRecord { ty },
                },
                2 => WarpOp::DeleteNode { node },
                3 | 4 => WarpOp::UpsertEdge {
                    warp_id,
                    record: EdgeRecord {
                        id: edge_id,
                        from,
                        to: sequence.pick(&self.nodes),
                        ty,
                    },
                },
                5 => WarpOp::DeleteEdge {
                    warp_id,
                    from,
                    edge_id,
                },
                6 | 7 => WarpOp::SetAttachment {
                    key: AttachmentKey::node_alpha(node),
                    value,
                },
                8 => WarpOp::SetAttachment {
                    key: AttachmentKey::edge_beta(edge),
                    value,
                },
                9 => WarpOp::OpenPortal {
                    key: AttachmentKey::node_alpha(portal_owner),
                    child_warp: child,
                    child_root: self.nodes[3],
                    init: PortalInit::Empty {
                        root_record: NodeRecord {
                            ty: make_type_id("child-root"),
                        },
                    },
                },
                _ => WarpOp::UpsertWarpInstance {
                    instance: WarpInstance {
                        warp_id: child,
                        root_node: self.nodes[3],
                        parent: Some(AttachmentKey::node_alpha(portal_owner)),
                    },
                },
            }
        }

        /// The root-instance node that owns the portal to the child instance.
        fn portal_owner(&self) -> NodeKey {
            NodeKey {
                warp_id: self.warps[0],
                local_id: self.nodes[0],
            }
        }

        /// Operations for one tick. Deleting the child instance must clear the
        /// portal that points at it in the same tick.
        fn tick_ops(&self, sequence: &mut Sequence) -> Vec<WarpOp> {
            if sequence.next() % 7 == 0 {
                return vec![
                    WarpOp::DeleteWarpInstance {
                        warp_id: self.warps[1],
                    },
                    WarpOp::SetAttachment {
                        key: AttachmentKey::node_alpha(self.portal_owner()),
                        value: None,
                    },
                ];
            }
            let op_count = usize::try_from(sequence.next() % 3).unwrap() + 1;
            (0..op_count).map(|_| self.op(sequence)).collect()
        }
    }

    fn atom(byte: u8) -> AttachmentValue {
        AttachmentValue::Atom(AtomPayload::new(
            make_type_id("slot-atom"),
            Bytes::from(vec![byte; 3]),
        ))
    }

    fn patch(ops: Vec<WarpOp>) -> WarpTickPatchV1 {
        WarpTickPatchV1::new(
            0,
            [0; 32],
            TickCommitStatus::Committed,
            Vec::new(),
            Vec::new(),
            ops,
        )
    }

    /// Builds a history of patches that each apply, with the materialized
    /// boundary after every tick.
    fn history(seed: u64, ticks: usize) -> (WarpState, Vec<WarpTickPatchV1>, Vec<WarpState>) {
        let universe = Universe::new();
        let initial = universe.initial();
        let mut sequence = Sequence(seed);
        let mut patches = Vec::new();
        let mut boundaries = vec![initial.clone()];
        let mut attempts = 0;
        while patches.len() < ticks {
            attempts += 1;
            assert!(
                attempts < ticks * 400,
                "generator must keep finding valid patches"
            );
            let candidate = patch(universe.tick_ops(&mut sequence));
            let mut next = boundaries.last().unwrap().clone();
            if candidate.apply_to_state(&mut next).is_ok() {
                patches.push(candidate);
                boundaries.push(next);
            }
        }
        (initial, patches, boundaries)
    }

    #[test]
    fn every_slot_at_every_cursor_matches_the_materialized_boundary() {
        let slots = Universe::new().slots();
        for seed in [
            0x9E37_79B9_7F4A_7C15,
            0xD1B5_4A32_D192_ED03,
            0x2545_F491_4F6C_DD1D,
        ] {
            let (initial, patches, boundaries) = history(seed, 160);
            let mut index = SlotWriteIndex::new();
            for patch in &patches {
                index.push_patch(patch);
            }
            assert_eq!(index.tick_count(), patches.len());
            for (cursor, boundary) in boundaries.iter().enumerate() {
                for slot in &slots {
                    assert_eq!(
                        index.read_slot(&patches, &initial, *slot, cursor),
                        Ok(materialized_slot(boundary, *slot)),
                        "seed {seed:#x} cursor {cursor} slot {slot:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn generated_histories_exercise_instance_deletion_and_recreation() {
        let (_, patches, _) = history(0x9E37_79B9_7F4A_7C15, 160);
        let count = |wanted: fn(&WarpOp) -> bool| {
            patches
                .iter()
                .flat_map(WarpTickPatchV1::ops)
                .filter(|op| wanted(op))
                .count()
        };
        let counts = [
            count(|op| matches!(op, WarpOp::DeleteWarpInstance { .. })),
            count(|op| matches!(op, WarpOp::OpenPortal { .. })),
            count(|op| matches!(op, WarpOp::DeleteNode { .. })),
            count(|op| matches!(op, WarpOp::DeleteEdge { .. })),
        ];
        assert!(
            counts.iter().all(|count| *count >= 2),
            "instance deletions, portal openings, node deletions, edge deletions: {counts:?}"
        );
    }

    #[test]
    fn truncation_equals_an_index_built_from_the_shorter_history() {
        let (_, patches, _) = history(0xD1B5_4A32_D192_ED03, 120);
        for keep in [0, 1, 37, 119, 120] {
            let mut truncated = SlotWriteIndex::new();
            for patch in &patches {
                truncated.push_patch(patch);
            }
            truncated.truncate(keep, &patches[keep..]);

            let mut rebuilt = SlotWriteIndex::new();
            for patch in &patches[..keep] {
                rebuilt.push_patch(patch);
            }
            assert_eq!(truncated, rebuilt, "keep {keep}");
        }
    }

    #[test]
    fn a_cursor_beyond_history_and_a_port_slot_are_refused() {
        let (initial, patches, _) = history(0x2545_F491_4F6C_DD1D, 4);
        let mut index = SlotWriteIndex::new();
        for patch in &patches {
            index.push_patch(patch);
        }
        let slot = Universe::new().slots()[0];
        assert_eq!(
            index.read_slot(&patches, &initial, slot, 5),
            Err(SlotReadingError::CursorBeyondHistory {
                cursor: 5,
                available: 4
            })
        );
        assert_eq!(
            index.read_slot(&patches[..2], &initial, slot, 3),
            Err(SlotReadingError::CursorBeyondHistory {
                cursor: 3,
                available: 2
            })
        );
    }
}
