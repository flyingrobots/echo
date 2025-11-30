// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

use crate::temporal::tick::Tick;
use crate::types::aabb::Aabb;

/// Broad-phase proxy summarizing an entity’s swept volume over a tick.
///
/// Stores a conservative fat AABB and the owning `entity` identifier (opaque
/// to the geometry layer). The proxy is suitable for insertion into a broad-
/// phase accelerator.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct SweepProxy {
    entity: u64,
    tick: Tick,
    fat: Aabb,
}

impl SweepProxy {
    /// Creates a new proxy for `entity` at `tick` with precomputed `fat` AABB.
    #[must_use]
    pub const fn new(entity: u64, tick: Tick, fat: Aabb) -> Self {
        Self { entity, tick, fat }
    }

    /// Opaque entity identifier owning this proxy.
    #[must_use]
    pub const fn entity(&self) -> u64 {
        self.entity
    }

    /// Tick associated with the motion window.
    #[must_use]
    pub const fn tick(&self) -> Tick {
        self.tick
    }

    /// Conservative fat AABB for this proxy.
    #[must_use]
    pub const fn fat(&self) -> Aabb {
        self.fat
    }
}
