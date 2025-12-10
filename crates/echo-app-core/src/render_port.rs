// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Port trait for UI/adapter layers to request a redraw on the underlying
//! surface/window without depending on a specific windowing crate.

/// Minimal redraw port; implementations are expected to be cheap/best-effort
/// and typically just forward to a windowing surface's `request_redraw`.
pub trait RenderPort {
    /// Request a redraw of the main surface/window.
    fn request_redraw(&self);
}
