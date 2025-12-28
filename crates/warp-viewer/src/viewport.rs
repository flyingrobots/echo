// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Bundle of window + GPU + egui state for a single viewport.

use crate::{gpu::Gpu, render_port::WinitRenderPort};
use egui_wgpu::Renderer;
use egui_winit::winit::window::Window;
use egui_winit::State as EguiWinitState;

pub struct Viewport {
    pub window: &'static Window,
    pub gpu: Gpu,
    pub egui_state: EguiWinitState,
    pub egui_renderer: Renderer,
    pub render_port: WinitRenderPort,
}
