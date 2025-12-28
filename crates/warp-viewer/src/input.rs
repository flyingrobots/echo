// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Input handling: map winit events into viewer state and UI events.

use egui_winit::winit::{
    event::{ElementState, MouseScrollDelta, WindowEvent},
    keyboard::KeyCode,
    keyboard::PhysicalKey,
    window::Window,
};

use crate::{core::UiState, ui_state::UiEvent, viewer_state::ViewerState};

#[derive(Default)]
pub struct InputOutcome {
    pub ui_event: Option<UiEvent>,
}

pub fn handle_window_event(
    event: &WindowEvent,
    window: &Window,
    viewer: &mut ViewerState,
    _ui: &mut UiState,
) -> InputOutcome {
    let mut out = InputOutcome::default();
    match event {
        WindowEvent::Resized(size) => {
            let _ = size;
        }
        WindowEvent::ScaleFactorChanged {
            scale_factor: _,
            inner_size_writer: _,
        } => {
            let _size = window.inner_size();
            // cannot mutate inner_size_writer with immutable reference; ignore
        }
        WindowEvent::KeyboardInput { event, .. } => {
            if let PhysicalKey::Code(code) = event.physical_key {
                match event.state {
                    ElementState::Pressed => {
                        viewer.keys.insert(code);
                        if code == KeyCode::Escape {
                            out.ui_event = Some(UiEvent::OpenMenu);
                        }
                    }
                    ElementState::Released => {
                        viewer.keys.remove(&code);
                    }
                }
            }
        }
        WindowEvent::MouseWheel { delta, .. } => {
            let y: f32 = match delta {
                MouseScrollDelta::LineDelta(_, y) => *y,
                MouseScrollDelta::PixelDelta(p) => p.y as f32 / 50.0,
            };
            viewer.camera.zoom_fov(1.0 - y * 0.05);
        }
        _ => {}
    }
    out
}
