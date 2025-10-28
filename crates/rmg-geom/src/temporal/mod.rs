//! Temporal types and helpers used for tick-based motion and broad-phase.

#[doc = "Broad-phase proxy carrying entity id, tick, and fat AABB."]
pub mod manifold;
#[doc = "Discrete Chronos ticks (u64 newtype)."]
pub mod tick;
#[doc = "Start/end transforms over a tick and fat AABB computation."]
pub mod timespan;
