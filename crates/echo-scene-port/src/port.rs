// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Scene port trait defining the renderer contract.

use crate::{ApplyError, CameraState, HighlightState, SceneDelta};

/// Scene rendering port trait.
///
/// Implementors receive deltas and render. No time ownership.
/// RenderContext/FrameResult are adapter-local concerns, not part of this contract.
///
/// # Design
///
/// This trait defines a hexagonal port for rendering. The domain (TTD Controller)
/// emits SceneDeltas; adapters (Three.js, wgpu) implement this trait to render.
///
/// # Epoch Semantics
///
/// Deltas are idempotent per (cursor_id, epoch). If an adapter receives a delta
/// with an epoch it has already processed for that cursor, it should skip it.
///
/// # Thread Safety
///
/// Implementations must be `Send` to allow for multi-threaded state application
/// and background decoding.
pub trait ScenePort: Send {
    /// Apply a scene delta. Idempotent per (cursor_id, epoch).
    ///
    /// # Errors
    ///
    /// Returns [`ApplyError`] if the delta is malformed or violates scene invariants.
    fn apply_scene_delta(&mut self, delta: &SceneDelta) -> Result<(), ApplyError>;

    /// Set camera state.
    fn set_camera(&mut self, camera: &CameraState);

    /// Set highlight state (selection/hover).
    fn set_highlight(&mut self, highlight: &HighlightState);

    /// Render the current scene.
    ///
    /// Takes no parameters—profiling/timing is the adapter's concern.
    fn render(&mut self);

    /// Resize viewport.
    ///
    /// # Arguments
    ///
    /// * `width` - Viewport width in pixels.
    /// * `height` - Viewport height in pixels.
    /// * `dpr` - Device Pixel Ratio (must be > 0.0).
    ///
    /// # Panics
    ///
    /// Implementations may panic if `dpr` is not finite or is <= 0.0.
    fn resize(&mut self, width: u32, height: u32, dpr: f32);

    /// Reset epoch tracking for a cursor.
    ///
    /// This ONLY clears epoch tracking. Scene state is NOT cleared.
    /// Use `SceneOp::Clear` to clear the scene.
    fn reset_cursor(&mut self, cursor_id: &[u8; 32]);

    /// Dispose all resources.
    fn dispose(&mut self);
}
