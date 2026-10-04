// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
use super::{ReadError, ReadObstruction};
use crate::{
    AttachmentValue, Hash, NodeKey, TypeId, WorldlineFrontier, WorldlineId, WorldlineTick,
};

const MAX_APERTURE_NODES: usize = 65_536;

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
}

impl<'a> ReadView<'a> {
    /// Validates a real basis and a sorted, unique, bounded node aperture.
    ///
    /// The trusted caller must select this frontier and aperture independently
    /// of application input. This constructor does not authorize a public request.
    pub fn new(
        frontier: &'a WorldlineFrontier,
        expected: ReadBasis,
        aperture: &'a [NodeKey],
    ) -> Result<Self, ReadError> {
        if aperture.len() > MAX_APERTURE_NODES || aperture.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(ReadError::InvalidAperture);
        }
        if ReadBasis::at(frontier) != expected {
            return Err(ReadError::BasisMismatch);
        }
        Ok(Self {
            frontier,
            basis: expected,
            aperture,
        })
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
