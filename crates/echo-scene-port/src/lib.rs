// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Scene port contract for Echo renderers.
//!
//! This crate defines the domain contract between TTD Controller and renderers.
//! It contains NO serialization logic—that lives in echo-scene-codec.
//!
//! # Design Principles
//!
//! - **Renderers are dumb** — They receive deltas and render. No domain logic.
//! - **No time ownership** — All timing comes from the app, not the renderer.
//! - **Cursor-scoped epochs** — Deltas are idempotent per (cursor_id, epoch).
//!
//! # Crate Features
//!
//! - `std` (default): Enables std library. Disable for no_std contexts.

#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

use thiserror::Error;

/// Error type for scene delta application.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ApplyError {
    /// The decoder encountered malformed CBOR data.
    #[error("decode error: {0}")]
    Decode(alloc::string::String),
    /// An invariant was violated (e.g., duplicate key, missing dependency).
    #[error("invariant violation: {0}")]
    Invariant(alloc::string::String),
    /// A backend-specific error occurred.
    #[error("backend error: {0}")]
    Backend(alloc::string::String),
}

mod camera;
mod canon;
mod highlight;
mod port;
mod types;

pub use camera::{CameraState, ProjectionKind};
pub use canon::{canonicalize_f32, canonicalize_position};
pub use highlight::HighlightState;
pub use port::ScenePort;
pub use types::{
    ColorRgba8, EdgeDef, EdgeKey, EdgeStyle, Hash, LabelAnchor, LabelDef, LabelKey, NodeDef,
    NodeKey, NodeShape, SceneDelta, SceneOp, MAX_OPS,
};
