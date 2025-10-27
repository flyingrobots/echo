//! Temporal types and helpers used for tick-based motion and broad-phase.

#[doc = "Discrete Chronos ticks (u64 newtype)."]
pub mod tick;
#[doc = "Start/end transforms over a tick and fat AABB computation."]
pub mod temporal_transform;
#[doc = "Broad-phase proxy carrying entity id, tick, and fat AABB."]
pub mod temporal_proxy;
