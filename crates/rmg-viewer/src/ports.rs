// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Port traits to nudge the viewer toward hexagonal boundaries.

use echo_app_core::prefs::ViewerPrefs;
use echo_graph::RmgFrame;
use echo_session_proto::Notification;

/// Session-facing port: receive notifications and RMG frames, clear streams.
pub trait SessionPort {
    fn drain_notifications(&mut self, max: usize) -> Vec<Notification>;
    fn drain_frames(&mut self, max: usize) -> Vec<RmgFrame>;
    fn clear_streams(&mut self);
}

/// Config-facing port for loading/saving viewer preferences.
#[allow(dead_code)]
pub trait ConfigPort {
    fn load_prefs(&self) -> Option<ViewerPrefs>;
    fn save_prefs(&self, prefs: &ViewerPrefs);
}

/// Render-facing port; lets the UI request a redraw without coupling to winit.
#[allow(dead_code)]
pub trait RenderPort {
    fn request_redraw(&self);
}
