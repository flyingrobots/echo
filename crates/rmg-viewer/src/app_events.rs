// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Window-event handling for the App.

use crate::{app::App, input};
use egui_winit::{winit::event::WindowEvent, State as EguiWinitState};

impl App {
    pub fn handle_window_event(
        &mut self,
        window_id: egui_winit::winit::window::WindowId,
        event: WindowEvent,
    ) {
        let Some(idx) = self
            .viewports
            .iter()
            .position(|v| v.window.id() == window_id)
        else {
            return;
        };
        let win = self.viewports[idx].window;
        let egui_state_ptr: *mut EguiWinitState = &mut self.viewports[idx].egui_state;

        let outcome = input::handle_window_event(&event, win, &mut self.viewer, &mut self.ui);
        if let Some(ev) = outcome.ui_event {
            self.apply_ui_event(ev);
        }

        let egui_state = unsafe { &mut *egui_state_ptr };
        let _ = egui_state.on_window_event(win, &event);
    }
}
