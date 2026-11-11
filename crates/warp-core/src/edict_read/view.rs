// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
use super::access::{Material, Outgoing};
use super::{ReadError, ReadObstruction};
use crate::edict_pure::evaluate::Meter;
use crate::{
    AttachmentValue, Hash, NodeKey, OutgoingEdgeRange, TypeId, WorldlineFrontier, WorldlineId,
    WorldlineTick,
};

const MAX_APERTURE_NODES: usize = 65_536;
const MAX_APERTURE_TYPES: usize = 65_536;
const SCANNED_EDGE_BYTES: u64 = 128;

/// Host-selected typed authority for graph material a program discovers.
///
/// A node aperture names exact keys. Edges and the nodes they lead to are not
/// known before evaluation, so this aperture names the edge types a program may
/// scan and the node types it may materialize by following them. It grants no
/// authority to an application request and carries no mutation authority.
#[derive(Clone, Copy, Debug)]
pub struct GraphReadAperture<'a> {
    /// Sorted, unique edge types whose outgoing ranges may be read.
    pub edge_types: &'a [TypeId],
    /// Sorted, unique actual node types a discovered node may have.
    pub node_types: &'a [TypeId],
    /// Bucket edges one evaluation may inspect, matching or not.
    pub max_edge_scans: u64,
}

/// Exact identity of a trusted host's selected materialized frontier.
///
/// This binds real state bytes, worldline, and tick; it is not an admission token
/// or proof of causal-history retention. Application basis remains separate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReadBasis {
    worldline_id: WorldlineId,
    tick: WorldlineTick,
    state_root: Hash,
}

impl ReadBasis {
    /// Hashes the real frontier state during host view preparation.
    ///
    /// This full-state preparation cost is outside the interpreted read budget.
    /// Callers should not mistake a small read aperture for bounded view setup.
    #[must_use]
    pub fn at(frontier: &WorldlineFrontier) -> Self {
        Self {
            worldline_id: frontier.worldline_id(),
            tick: frontier.frontier_tick(),
            state_root: frontier.state().state_root(),
        }
    }

    /// Selected worldline identity.
    #[must_use]
    pub const fn worldline_id(self) -> WorldlineId {
        self.worldline_id
    }
    /// Selected worldline tick.
    #[must_use]
    pub const fn tick(self) -> WorldlineTick {
        self.tick
    }
    /// Exact state-root identity.
    #[must_use]
    pub const fn state_root(self) -> Hash {
        self.state_root
    }
}

/// Immutable borrowed storage with an explicit, release-independent host aperture.
///
/// The borrow prevents mutation for the view's lifetime. No caller-controlled
/// byte callback, `Snapshot` metadata, or package profile can construct its data.
#[derive(Debug)]
pub struct ReadView<'a> {
    frontier: &'a WorldlineFrontier,
    basis: ReadBasis,
    aperture: &'a [NodeKey],
    graph: Option<GraphReadAperture<'a>>,
}

impl<'a> ReadView<'a> {
    /// Derives the current basis once and validates a bounded node aperture.
    ///
    /// The trusted caller selects the frontier and aperture independently of
    /// application input. Use [`Self::new`] to check an independently selected
    /// expected basis. This constructor does not authorize a public request.
    pub fn at(frontier: &'a WorldlineFrontier, aperture: &'a [NodeKey]) -> Result<Self, ReadError> {
        Self::validate_aperture(aperture)?;
        Ok(Self {
            frontier,
            basis: ReadBasis::at(frontier),
            aperture,
            graph: None,
        })
    }

    /// Validates a real basis and a sorted, unique, bounded node aperture.
    ///
    /// The trusted caller must select this frontier and aperture independently
    /// of application input. This constructor does not authorize a public request.
    pub fn new(
        frontier: &'a WorldlineFrontier,
        expected: ReadBasis,
        aperture: &'a [NodeKey],
    ) -> Result<Self, ReadError> {
        Self::validate_aperture(aperture)?;
        if ReadBasis::at(frontier) != expected {
            return Err(ReadError::BasisMismatch);
        }
        Ok(Self {
            frontier,
            basis: expected,
            aperture,
            graph: None,
        })
    }

    pub(crate) fn validate_aperture(aperture: &[NodeKey]) -> Result<(), ReadError> {
        if aperture.len() > MAX_APERTURE_NODES || aperture.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(ReadError::InvalidAperture);
        }
        Ok(())
    }

    /// Identity established from the borrowed frontier.
    #[must_use]
    pub const fn basis(&self) -> ReadBasis {
        self.basis
    }

    pub(super) fn contains(&self, node: NodeKey) -> bool {
        self.aperture.binary_search(&node).is_ok()
    }

    pub(super) fn atom(
        &self,
        node: NodeKey,
        expected_type: TypeId,
        max_bytes: u64,
    ) -> Result<&'a [u8], ReadObstruction> {
        let store = self
            .frontier
            .state()
            .store(&node.warp_id)
            .ok_or(ReadObstruction::Missing)?;
        store.node(&node.local_id).ok_or(ReadObstruction::Missing)?;
        let attachment = store
            .node_attachment(&node.local_id)
            .ok_or(ReadObstruction::Missing)?;
        let AttachmentValue::Atom(atom) = attachment else {
            return Err(ReadObstruction::AtomRequired);
        };
        if atom.type_id != expected_type {
            return Err(ReadObstruction::TypeMismatch);
        }
        if atom.bytes.len() as u64 > max_bytes {
            return Err(ReadObstruction::AtomTooLarge);
        }
        Ok(&atom.bytes)
    }
}

impl<'a> ReadView<'a> {
    /// Adds a typed graph aperture for outgoing-edge and closure reads.
    ///
    /// Without one, every graph read refuses. The explicit node aperture still
    /// selects each edge-read source and each closure start node.
    ///
    /// # Errors
    ///
    /// Refuses oversized, unsorted or duplicated type sets.
    pub fn with_graph_aperture(mut self, graph: GraphReadAperture<'a>) -> Result<Self, ReadError> {
        for types in [graph.edge_types, graph.node_types] {
            if types.len() > MAX_APERTURE_TYPES || types.windows(2).any(|pair| pair[0] >= pair[1]) {
                return Err(ReadError::InvalidAperture);
            }
        }
        self.graph = Some(graph);
        Ok(self)
    }

    pub(super) fn scan_ceiling(&self) -> u64 {
        self.graph.map_or(0, |graph| graph.max_edge_scans)
    }

    pub(super) fn outgoing(
        &self,
        range: OutgoingEdgeRange,
        allowance: u64,
        meter: &mut Meter,
    ) -> Result<Outgoing, ReadError> {
        if self
            .graph
            .is_none_or(|graph| graph.edge_types.binary_search(&range.edge_type).is_err())
        {
            return Err(ReadError::OutsideGraphAperture(range.edge_type));
        }
        let Some(store) = self
            .frontier
            .state()
            .store(&range.source.warp_id)
            .filter(|store| store.node(&range.source.local_id).is_some())
        else {
            return Ok(Outgoing::SourceMissing);
        };
        let bucket = store.edges_from(&range.source.local_id);
        let inspected = u64::try_from(bucket.len()).map_err(|_| ReadError::ReadCountExceeded)?;
        if inspected > allowance {
            return Ok(Outgoing::ScanBound);
        }
        // The whole bucket is charged before any edge is inspected or copied.
        meter.charge_steps(inspected)?;
        meter.allocate(inspected.saturating_mul(SCANNED_EDGE_BYTES))?;
        let mut edges: Vec<_> = bucket
            .filter(|edge| edge.ty == range.edge_type)
            .cloned()
            .collect();
        edges.sort_unstable_by_key(|edge| edge.id);
        Ok(Outgoing::Edges { inspected, edges })
    }

    pub(super) fn material(&self, node: NodeKey) -> Result<Material<'a>, ReadError> {
        let Some(store) = self.frontier.state().store(&node.warp_id) else {
            return Ok(Material::Missing);
        };
        let Some(record) = store.node(&node.local_id) else {
            return Ok(Material::Missing);
        };
        if !self.contains(node)
            && self
                .graph
                .is_none_or(|graph| graph.node_types.binary_search(&record.ty).is_err())
        {
            return Err(ReadError::OutsideGraphAperture(record.ty));
        }
        Ok(match store.node_attachment(&node.local_id) {
            None => Material::Node {
                ty: record.ty,
                atom: None,
            },
            Some(AttachmentValue::Descend(_)) => Material::Portal,
            Some(AttachmentValue::Atom(atom)) => Material::Node {
                ty: record.ty,
                atom: Some((atom.type_id, &atom.bytes)),
            },
        })
    }
}
