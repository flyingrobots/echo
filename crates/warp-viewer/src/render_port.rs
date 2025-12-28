// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Winit-backed RenderPort implementation for the viewer.

use echo_app_core::render_port::RenderPort;
use egui_winit::winit::window::Window;

#[derive(Clone)]
pub struct WinitRenderPort {
    win: &'static Window,
}

impl WinitRenderPort {
    pub fn new(win: &'static Window) -> Self {
        Self { win }
    }
}

impl RenderPort for WinitRenderPort {
    fn request_redraw(&self) {
        self.win.request_redraw();
    }
}
