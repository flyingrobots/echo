// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Stateless egui render helpers for the viewer UI screens/overlays.

use crate::{
    core::{Screen, TitleMode, ViewerOverlay},
    App,
};
use egui::{self, Context};

pub fn draw_title_screen(ctx: &Context, app: &mut App) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(80.0);
            ui.heading("Echo RMG Viewer");
            ui.label(format!("v{}", env!("CARGO_PKG_VERSION")));
            ui.add_space(20.0);
            match app.ui.title_mode {
                TitleMode::Menu => {
                    if ui.button("Connect").clicked() {
                        app.ui.title_mode = TitleMode::ConnectForm;
                    }
                    if ui.button("Settings").clicked() {
                        app.ui.title_mode = TitleMode::Settings;
                    }
                    if ui.button("Exit").clicked() {
                        std::process::exit(0);
                    }
                }
                TitleMode::ConnectForm => {
                    ui.label("Host:");
                    ui.text_edit_singleline(&mut app.ui.connect_host);
                    ui.label("Port:");
                    ui.add(egui::DragValue::new(&mut app.ui.connect_port).speed(1));
                    if ui.button("Connect").clicked() {
                        app.start_connect();
                    }
                    if ui.button("Back").clicked() {
                        app.ui.title_mode = TitleMode::Menu;
                    }
                }
                TitleMode::Settings => {
                    ui.label("(Placeholder settings)");
                    if ui.button("Save").clicked() {
                        app.ui.title_mode = TitleMode::Menu;
                    }
                    if ui.button("Back").clicked() {
                        app.ui.title_mode = TitleMode::Menu;
                    }
                }
            }
        });
    });
}

pub fn draw_connecting_screen(ctx: &Context, log: &[String]) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);
            ui.heading("Connecting...");
            ui.add_space(10.0);
            for line in log {
                ui.label(line);
            }
            ui.add_space(20.0);
            ui.label("ECHO");
        });
    });
}

pub fn draw_error_screen(ctx: &Context, app: &mut App, msg: &str) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(80.0);
            ui.heading("Error");
            ui.label(msg);
            ui.add_space(12.0);
            if ui.button("Back to Title").clicked() {
                app.ui.screen = Screen::Title;
                app.ui.title_mode = TitleMode::Menu;
            }
        });
    });
}

pub fn draw_view_hud(
    ctx: &Context,
    app: &mut App,
    toasts: &[echo_app_core::toast::ToastRender],
    _debug_arc: &Option<(egui::Pos2, egui::Pos2)>,
) {
    // Menu button
    egui::Area::new("menu_button".into())
        .anchor(egui::Align2::LEFT_TOP, egui::vec2(12.0, 12.0))
        .show(ctx, |ui| {
            if ui.button("Menu").clicked() {
                app.ui.overlay = ViewerOverlay::Menu;
            }
        });

    // Toasts stack (simple)
    egui::Area::new("toasts".into())
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-12.0, 12.0))
        .show(ctx, |ui| {
            for t in toasts {
                ui.label(format!("{:?}: {}", t.kind, t.title));
            }
        });

    // HUD panels
    egui::Area::new("perf".into())
        .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(12.0, -12.0))
        .show(ctx, |ui| {
            ui.label(format!("FPS: {:.1}", app.viewer.perf.fps()));
        });

    egui::Area::new("controls".into())
        .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(12.0, -50.0))
        .show(ctx, |ui| {
            ui.label("WASD/QE move, L-drag look, R-drag spin, Wheel zoom, Arrows cycle RMG");
            ui.checkbox(&mut app.viewer.wireframe, "Wireframe");
        });

    egui::Area::new("stats".into())
        .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -12.0))
        .show(ctx, |ui| {
            let epoch = app.viewer.epoch.unwrap_or(0);
            ui.label(format!("RMG id {} | epoch {}", app.ui.rmg_id, epoch));
        });

    egui::Area::new("watermark".into())
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-12.0, -12.0))
        .show(ctx, |ui| {
            ui.label(format!("ECHO v{}", env!("CARGO_PKG_VERSION")));
        });

    // Overlays
    if let ViewerOverlay::Menu = app.ui.overlay {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                if ui.button("Settings").clicked() {
                    app.ui.overlay = ViewerOverlay::Settings;
                }
                if ui.button("Publish Local RMG").clicked() {
                    app.ui.overlay = ViewerOverlay::Publish;
                }
                if ui.button("Subscribe to RMG").clicked() {
                    app.ui.overlay = ViewerOverlay::Subscribe;
                }
                if ui.button("Back").clicked() {
                    app.ui.overlay = ViewerOverlay::None;
                }
            });
        });
    }
}
