//! Temporal types and helpers used for tick-based motion and broad-phase.

#[doc = "Broad-phase proxy carrying entity id, tick, and fat AABB."]
#[allow(clippy::module_name_repetitions)]
pub mod temporal_proxy;
#[doc = "Start/end transforms over a tick and fat AABB computation."]
#[allow(clippy::module_name_repetitions)]
pub mod temporal_transform;
#[doc = "Discrete Chronos ticks (u64 newtype)."]
pub mod tick;
