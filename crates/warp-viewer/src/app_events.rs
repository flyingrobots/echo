// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Window-event handling for the App.

use crate::{app::App, input};
use egui_winit::winit::event::WindowEvent;

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

        let outcome = input::handle_window_event(&event, win, &mut self.viewer, &mut self.ui);
        if let Some(ev) = outcome.ui_event {
            self.apply_ui_event(ev);
        }

        if let Some(vp) = self.viewports.get_mut(idx) {
            let _ = vp.egui_state.on_window_event(win, &event);
        }
    }
}
