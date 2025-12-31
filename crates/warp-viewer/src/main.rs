// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! warp-viewer: 3D WARP visualizer entrypoint. Main wires App into winit.

use anyhow::Result;
use egui_winit::winit::event_loop::EventLoop;

mod app;
mod app_events;
mod app_frame;
mod camera;
mod core;
mod gpu;
mod input;
mod perf;
mod render;
mod render_port;
mod scene;
mod session_logic;
mod ui;
mod ui_effects;
mod ui_state;
mod viewer_state;
mod viewport;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .without_time()
        .init();
    let event_loop = EventLoop::new()?;
    let mut app = app::App::new();
    event_loop.run_app(&mut app)?;
    Ok(())
}
