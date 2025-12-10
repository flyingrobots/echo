// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Stateless egui render helpers for the viewer UI screens/overlays.

use crate::{
    app::App,
    core::{TitleMode, ViewerOverlay},
    ui_state::UiEvent,
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
                        app.apply_ui_event(UiEvent::ConnectClicked);
                    }
                    if ui.button("Settings").clicked() {
                        app.apply_ui_event(UiEvent::SettingsClicked);
                    }
                    if ui.button("Exit").clicked() {
                        app.apply_ui_event(UiEvent::ExitClicked);
                    }
                }
                TitleMode::ConnectForm => {
                    ui.label("Host:");
                    let mut host = app.ui.connect_host.clone();
                    if ui.text_edit_singleline(&mut host).changed() {
                        app.apply_ui_event(UiEvent::ConnectHostChanged(host));
                    }
                    ui.label("Port:");
                    let mut port = app.ui.connect_port;
                    if ui.add(egui::DragValue::new(&mut port).speed(1)).changed() {
                        app.apply_ui_event(UiEvent::ConnectPortChanged(port));
                    }
                    if ui.button("Connect").clicked() {
                        app.apply_ui_event(UiEvent::ConnectSubmit);
                    }
                    if ui.button("Back").clicked() {
                        app.apply_ui_event(UiEvent::BackToTitle);
                    }
                }
                TitleMode::Settings => {
                    ui.label("(Placeholder settings)");
                    if ui.button("Save").clicked() {
                        app.apply_ui_event(UiEvent::SavePrefs);
                    }
                    if ui.button("Back").clicked() {
                        app.apply_ui_event(UiEvent::BackToTitle);
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
                app.apply_ui_event(UiEvent::BackToTitle);
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
                app.apply_ui_event(UiEvent::OpenMenu);
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
    match app.ui.overlay {
        ViewerOverlay::None => {}
        ViewerOverlay::Menu => {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(40.0);
                    if ui.button("Settings").clicked() {
                        app.apply_ui_event(UiEvent::OpenSettingsOverlay);
                    }
                    if ui.button("Publish Local RMG").clicked() {
                        app.apply_ui_event(UiEvent::OpenPublishOverlay);
                    }
                    if ui.button("Subscribe to RMG").clicked() {
                        app.apply_ui_event(UiEvent::OpenSubscribeOverlay);
                    }
                    if ui.button("Back").clicked() {
                        app.apply_ui_event(UiEvent::CloseOverlay);
                    }
                });
            });
        }
        ViewerOverlay::Settings => {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("Viewer Settings");
                ui.separator();
                ui.checkbox(&mut app.viewer.vsync, "Enable VSync");
                ui.checkbox(&mut app.viewer.wireframe, "Wireframe mode");
                ui.checkbox(&mut app.viewer.show_watermark, "Show watermark");
                ui.checkbox(&mut app.viewer.debug_show_arc, "Show drag arc");
                ui.checkbox(&mut app.viewer.debug_show_sphere, "Show debug sphere");
                ui.add_space(12.0);
                if ui.button("Save").clicked() {
                    app.apply_ui_event(UiEvent::SavePrefs);
                }
                if ui.button("Close").clicked() {
                    app.apply_ui_event(UiEvent::CloseOverlay);
                }
            });
        }
        ViewerOverlay::Publish => {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("Publish Local RMG");
                ui.label("Publishing from the local runtime will appear here.");
                ui.label("Hook up engine output to stream snapshots/diffs to the hub.");
                ui.add_space(12.0);
                if ui.button("Close").clicked() {
                    app.apply_ui_event(UiEvent::CloseOverlay);
                }
            });
        }
        ViewerOverlay::Subscribe => {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("Subscribe to RMG");
                ui.label("Choose an RMG id to follow from the session hub.");
                let mut rmg_id = app.ui.rmg_id;
                if ui
                    .add(
                        egui::DragValue::new(&mut rmg_id)
                            .speed(1)
                            .prefix("RMG id: "),
                    )
                    .changed()
                {
                    app.ui.rmg_id = rmg_id.max(1);
                }
                ui.add_space(12.0);
                if ui.button("Close").clicked() {
                    app.apply_ui_event(UiEvent::CloseOverlay);
                }
            });
        }
    }
}
