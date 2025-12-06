// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Top-level application wiring and event loop handler.

use crate::{
    core::UiState,
    render_port::WinitRenderPort,
    scene::{sample_wire_graph, scene_from_wire},
    ui_effects::{self, UiEffectsRunner},
    ui_state,
    viewer_state::ViewerState,
    viewport::Viewport,
};
use echo_app_core::{
    config::ConfigService,
    config_port::ConfigPort,
    toast::{ToastKind, ToastScope, ToastService},
};
use echo_config_fs::FsConfigStore;
use echo_session_client::tool::ChannelSession;
use egui_extras::install_image_loaders;
use egui_winit::winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowAttributes},
};
use egui_winit::State as EguiWinitState;
use std::time::Instant;

pub struct App {
    pub viewports: Vec<Viewport>,
    pub egui_ctx: egui::Context,
    pub config: Option<Box<dyn ConfigPort>>,
    pub ui_runner: ui_effects::RealEffectsRunner,
    pub toasts: ToastService,
    pub session: ChannelSession,
    pub ui: UiState,
    pub viewer: ViewerState,
}

impl App {
    pub fn new() -> Self {
        let egui_ctx = egui::Context::default();
        install_image_loaders(&egui_ctx);
        let config = FsConfigStore::new()
            .map(ConfigService::new)
            .map(|svc| Box::new(svc) as Box<dyn ConfigPort>)
            .ok();
        let prefs = config
            .as_ref()
            .and_then(|c| c.load_prefs())
            .unwrap_or_default();
        let mut toasts = ToastService::new(32);
        if config.is_none() {
            toasts.push(
                ToastKind::Warn,
                ToastScope::Local,
                "Config store unavailable",
                Some(String::from(
                    "FsConfigStore init failed; prefs won't persist this session",
                )),
                std::time::Duration::from_secs(6),
                Instant::now(),
            );
        }
        let mut viewer = ViewerState {
            ..Default::default()
        };
        viewer.wire_graph = sample_wire_graph();
        viewer.graph = scene_from_wire(&viewer.wire_graph);
        viewer.history.append(viewer.wire_graph.clone(), 0);
        viewer.epoch = Some(0);
        viewer.apply_prefs(&prefs);

        Self {
            viewports: Vec::new(),
            egui_ctx,
            config,
            ui_runner: ui_effects::RealEffectsRunner,
            toasts,
            session: ChannelSession::new(),
            ui: UiState::new(),
            viewer,
        }
    }

    pub fn apply_ui_event(&mut self, ev: ui_state::UiEvent) {
        let (next, effects) = ui_state::reduce(&self.ui, ev);
        self.ui = next;
        for eff in effects.iter() {
            if matches!(eff, ui_state::UiEffect::SavePrefs) {
                if let Some(cfg) = &self.config {
                    cfg.save_prefs(&self.viewer.export_prefs());
                }
            }
        }
        let followups = self.ui_runner.run(effects, &mut self.session, &self.ui);
        for ev in followups {
            self.apply_ui_event(ev);
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if !self.viewports.is_empty() {
            return;
        }
        let window = event_loop
            .create_window(
                WindowAttributes::default()
                    .with_title("Echo RMG Viewer 3D")
                    .with_visible(true),
            )
            .expect("window");
        let window: &'static Window = Box::leak(Box::new(window));
        let gpu = pollster::block_on(crate::gpu::Gpu::new(window)).expect("gpu init");
        let egui_state = EguiWinitState::new(
            self.egui_ctx.clone(),
            egui::ViewportId::ROOT,
            event_loop,
            None,
            None,
            None,
        );
        let egui_renderer = egui_wgpu::Renderer::new(
            &gpu.device,
            gpu.config.format,
            egui_wgpu::RendererOptions::default(),
        );
        let render_port = WinitRenderPort::new(window);
        self.viewports.push(Viewport {
            window,
            gpu,
            egui_state,
            egui_renderer,
            render_port,
        });
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        window_id: egui_winit::winit::window::WindowId,
        event: WindowEvent,
    ) {
        self.handle_window_event(window_id, event);
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if self.viewports.is_empty() {
            return;
        }
        self.frame();
    }
}
