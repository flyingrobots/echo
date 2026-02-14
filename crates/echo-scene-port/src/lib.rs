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

mod camera;
mod canon;
mod highlight;
mod port;
mod types;

pub use camera::*;
pub use canon::*;
pub use highlight::*;
pub use port::*;
pub use types::*;
