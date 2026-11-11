// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! The same bounded-read interpreter can use plain or runtime-recording access.
use super::{ReadBasis, ReadError, ReadObstruction, ReadView};
use crate::edict_pure::{evaluate::Meter, EvaluationError};
use crate::{EdgeRecord, NodeKey, OutgoingEdgeRange, TypeId};

/// One typed outgoing-range resolution.
pub(crate) enum Outgoing {
    /// The source node is absent; no bucket was inspected.
    SourceMissing,
    /// The whole bucket exceeds the remaining scan allowance; none was inspected.
    ScanBound,
    /// The matching edges in edge-identifier order.
    Edges {
        /// Every edge in the source's bucket, matching or not.
        inspected: u64,
        /// The edges of the requested type.
        edges: Vec<EdgeRecord>,
    },
}

/// One node's record and alpha attachment, resolved without an expected type.
pub(crate) enum Material<'a> {
    /// The node is absent.
    Missing,
    /// The alpha attachment descends into another warp.
    Portal,
    /// The node's actual type and its alpha atom, if any.
    Node {
        ty: TypeId,
        atom: Option<(TypeId, &'a [u8])>,
    },
}

pub(crate) trait NodeAtomAccess {
    type Error: From<ReadError> + From<EvaluationError>;
    fn basis(&self) -> ReadBasis;
    fn contains(&self, node: NodeKey) -> bool;
    fn atom(
        &mut self,
        node: NodeKey,
        expected: TypeId,
        max_bytes: u64,
        meter: &mut Meter,
    ) -> Result<Result<&[u8], ReadObstruction>, Self::Error>;
    /// Bucket edges this access lets one evaluation inspect.
    fn scan_ceiling(&self) -> u64;
    /// Resolves a typed range. `source_type` is the already observed type of
    /// a discovered source; `None` names a source the host aperture selected.
    /// The whole bucket is charged before any edge is inspected.
    fn outgoing(
        &mut self,
        range: OutgoingEdgeRange,
        source_type: Option<TypeId>,
        allowance: u64,
        meter: &mut Meter,
    ) -> Result<Outgoing, Self::Error>;
    /// Resolves a node's material. `referrer` is the observed type of the node
    /// whose edge named this one; `None` names a host-selected start node.
    /// An atom over `max_bytes` is resolved without charging its payload; the
    /// caller refuses it.
    fn material(
        &mut self,
        node: NodeKey,
        referrer: Option<TypeId>,
        max_bytes: u64,
        meter: &mut Meter,
    ) -> Result<Material<'_>, Self::Error>;
}

impl NodeAtomAccess for &ReadView<'_> {
    type Error = ReadError;
    fn basis(&self) -> ReadBasis {
        ReadView::basis(self)
    }
    fn contains(&self, node: NodeKey) -> bool {
        ReadView::contains(self, node)
    }
    fn atom(
        &mut self,
        node: NodeKey,
        expected: TypeId,
        max_bytes: u64,
        _meter: &mut Meter,
    ) -> Result<Result<&[u8], ReadObstruction>, Self::Error> {
        Ok(ReadView::atom(self, node, expected, max_bytes))
    }
    fn scan_ceiling(&self) -> u64 {
        ReadView::scan_ceiling(self)
    }
    fn outgoing(
        &mut self,
        range: OutgoingEdgeRange,
        _source_type: Option<TypeId>,
        allowance: u64,
        meter: &mut Meter,
    ) -> Result<Outgoing, Self::Error> {
        ReadView::outgoing(self, range, allowance, meter)
    }
    fn material(
        &mut self,
        node: NodeKey,
        _referrer: Option<TypeId>,
        _max_bytes: u64,
        _meter: &mut Meter,
    ) -> Result<Material<'_>, Self::Error> {
        ReadView::material(self, node)
    }
}
