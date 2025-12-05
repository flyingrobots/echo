// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Top-level application wiring and event loop handler.

use crate::{
    core::{Screen, UiState},
    input,
    render,
    render_port::WinitRenderPort,
    session::{SessionClient, SessionPort},
    session_logic,
    ui::{draw_connecting_screen, draw_error_screen, draw_title_screen, draw_view_hud},
    ui_effects::{self, UiEffectsRunner},
    ui_state,
    viewport::Viewport,
    viewer_state::ViewerState,
    scene::{sample_wire_graph, scene_from_wire},
};
use echo_app_core::{
    config::ConfigService,
    config_port::ConfigPort,
    toast::{ToastKind, ToastScope, ToastService},
    render_port::RenderPort,
};
use echo_config_fs::FsConfigStore;
use echo_session_proto::{NotifyKind, NotifyScope};
use egui_extras::install_image_loaders;
use egui_winit::winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    keyboard::KeyCode,
    window::{Window, WindowAttributes},
};
use egui_winit::State as EguiWinitState;
use glam::{Quat, Vec3};
use std::time::Instant;

pub struct App {
    pub viewports: Vec<Viewport>,
    pub egui_ctx: egui::Context,
    pub config: Option<Box<dyn ConfigPort>>,
    pub ui_runner: ui_effects::RealEffectsRunner,
    pub toasts: ToastService,
    pub session: SessionClient,
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
            session: SessionClient::new(),
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

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if self.viewports.is_empty() {
            return;
        }
        let (win, width_px, height_px, raw_input) = {
            let vp = self.viewports.get_mut(0).unwrap();
            let raw = vp.egui_state.take_egui_input(vp.window);
            (vp.window, vp.gpu.config.width, vp.gpu.config.height, raw)
        };

        for n in SessionPort::drain_notifications(&mut self.session, 64) {
            let kind = match n.kind {
                NotifyKind::Info => ToastKind::Info,
                NotifyKind::Warn => ToastKind::Warn,
                NotifyKind::Error => ToastKind::Error,
            };
            let scope = match n.scope {
                NotifyScope::Global => ToastScope::Global,
                NotifyScope::Session(_) => ToastScope::Session,
                NotifyScope::Rmg(_) => ToastScope::Session,
                NotifyScope::Local => ToastScope::Local,
            };
            self.toasts.push(
                kind,
                scope,
                n.title,
                n.body,
                std::time::Duration::from_secs(8),
                Instant::now(),
            );
        }

        let outcome = session_logic::process_frames(
            &mut self.ui,
            &mut self.viewer,
            &mut self.toasts,
            SessionPort::drain_frames(&mut self.session, 64),
        );
        if outcome.enter_view {
            self.apply_ui_event(ui_state::UiEvent::EnterView);
        }
        if let Some(reason) = outcome.desync {
            SessionPort::clear_streams(&mut self.session);
            self.apply_ui_event(ui_state::UiEvent::ShowError(reason));
        }

        let dt = self.viewer.last_frame.elapsed().as_secs_f32().min(0.05);
        self.viewer.last_frame = Instant::now();
        let now = self.viewer.last_frame;
        self.toasts.retain_visible(now);
        let visible_toasts = self.toasts.visible(now);
        let aspect = width_px as f32 / height_px as f32;

        let speed = if self.viewer.keys.contains(&KeyCode::ShiftLeft)
            || self.viewer.keys.contains(&KeyCode::ShiftRight)
        {
            420.0
        } else {
            160.0
        };
        let mut mv = Vec3::ZERO;
        if self.viewer.keys.contains(&KeyCode::KeyW) {
            mv.z += speed * dt;
        }
        if self.viewer.keys.contains(&KeyCode::KeyS) {
            mv.z -= speed * dt;
        }
        if self.viewer.keys.contains(&KeyCode::KeyA) {
            mv.x -= speed * dt;
        }
        if self.viewer.keys.contains(&KeyCode::KeyD) {
            mv.x += speed * dt;
        }
        if self.viewer.keys.contains(&KeyCode::KeyQ) {
            mv.y -= speed * dt;
        }
        if self.viewer.keys.contains(&KeyCode::KeyE) {
            mv.y += speed * dt;
        }
        self.viewer.camera.move_relative(mv);

        if matches!(self.ui.screen, Screen::View) {
            self.viewer.graph.step_layout(dt);
        }

        let pointer = self.egui_ctx.input(|i| i.pointer.clone());
        let win_size = glam::Vec2::new(width_px as f32, height_px as f32);
        let pixels_per_point = win.scale_factor() as f32;
        let to_ndc = |pos: egui::Pos2| {
            let px = glam::Vec2::new(pos.x * pixels_per_point, pos.y * pixels_per_point);
            let ndc = (px / win_size) * 2.0 - glam::Vec2::splat(1.0);
            glam::Vec2::new(ndc.x, -ndc.y)
        };

        let radius = self.viewer.graph.bounding_radius();
        let arcball_vec = |ndc: glam::Vec2| {
            let mut v = glam::Vec3::new(ndc.x, ndc.y, 0.0);
            let d = (ndc.x * ndc.x + ndc.y * ndc.y).min(1.0);
            v.z = (1.0 - d).max(0.0).sqrt();
            v.normalize_or_zero()
        };

        if pointer.secondary_down() && !self.egui_ctx.is_using_pointer() {
            if let Some(pos) = pointer.interact_pos() {
                let ndc = to_ndc(pos);
                let dir = self.viewer.camera.pick_ray(ndc, aspect);
                let oc = self.viewer.camera.pos;
                let b = oc.dot(dir);
                let c = oc.length_squared() - radius * radius;
                let disc = b * b - c;
                if disc >= 0.0 {
                    let t = -b - disc.sqrt();
                    if t > 0.0 {
                        let hit = oc + dir * t;
                        let v = arcball_vec(ndc);
                        self.viewer.arc_active = true;
                        self.viewer.arc_last = Some(v);
                        self.viewer.arc_last_hit = Some(hit);
                        self.viewer.arc_curr_hit = Some(hit);
                    }
                }
            }
        } else if !pointer.secondary_down() {
            self.viewer.arc_active = false;
            self.viewer.arc_last = None;
            self.viewer.arc_last_hit = None;
            self.viewer.arc_curr_hit = None;
        }

        if self.viewer.arc_active {
            if let (Some(last), Some(pos)) = (self.viewer.arc_last, pointer.interact_pos()) {
                let ndc = to_ndc(pos);
                let curr = arcball_vec(ndc);
                let dir = self.viewer.camera.pick_ray(ndc, aspect);
                let oc = self.viewer.camera.pos;
                let b = oc.dot(dir);
                let c = oc.length_squared() - radius * radius;
                let disc = b * b - c;
                if disc >= 0.0 {
                    let t = -b - disc.sqrt();
                    if t > 0.0 {
                        let hit = oc + dir * t;
                        self.viewer.arc_curr_hit = Some(hit);
                    }
                }
                if curr.length_squared() > 0.0 && last.length_squared() > 0.0 {
                    let axis = last.cross(curr);
                    let dot = last.dot(curr).clamp(-1.0, 1.0);
                    let angle = dot.acos();
                    if axis.length_squared() > 0.0 && angle.is_finite() {
                        let dq = Quat::from_axis_angle(axis.normalize(), angle);
                        self.viewer.graph_rot = dq * self.viewer.graph_rot;
                        self.viewer.graph_ang_vel = axis.normalize() * (angle / dt.max(1e-4));
                    }
                }
                self.viewer.arc_last = Some(curr);
            }
        } else {
            let w = self.viewer.graph_ang_vel;
            let w_len = w.length();
            if w_len > 1e-4 {
                let angle = w_len * dt;
                let dq = Quat::from_axis_angle(w / w_len, angle);
                self.viewer.graph_rot = dq * self.viewer.graph_rot;
                let decay = (-self.viewer.graph_damping * dt).exp();
                self.viewer.graph_ang_vel *= decay;
            }
        }

        if pointer.primary_down() && !self.egui_ctx.is_using_pointer() {
            let delta = self.egui_ctx.input(|i| i.pointer.delta());
            let d = glam::Vec2::new(delta.x, delta.y);
            self.viewer
                .camera
                .rotate_by_mouse(d, self.viewer.debug_invert_cam_x, self.viewer.debug_invert_cam_y);
        }

        let aspect = width_px as f32 / height_px as f32;
        let view_proj = self.viewer.camera.view_proj(aspect, radius);

        let debug_arc_screen: Option<(egui::Pos2, egui::Pos2)> = if self.viewer.debug_show_arc {
            if let (Some(a), Some(b)) = (self.viewer.arc_last_hit, self.viewer.arc_curr_hit) {
                let proj = |p: Vec3| {
                    let v = view_proj * p.extend(1.0);
                    if v.w.abs() < 1e-5 {
                        return None;
                    }
                    let ndc = v.truncate() / v.w;
                    Some(ndc)
                };
                if let (Some(na), Some(nb)) = (proj(a), proj(b)) {
                    let w = width_px as f32 / win.scale_factor() as f32;
                    let h = height_px as f32 / win.scale_factor() as f32;
                    let to_screen = |n: Vec3| egui::Pos2 {
                        x: (n.x * 0.5 + 0.5) * w,
                        y: (-n.y * 0.5 + 0.5) * h,
                    };
                    Some((to_screen(na), to_screen(nb)))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let prev_vsync = self.viewer.vsync;

        let egui_ctx = self.egui_ctx.clone();
        let full_output = egui_ctx.run(raw_input, |ctx| match self.ui.screen.clone() {
            Screen::Title => {
                draw_title_screen(ctx, self);
            }
            Screen::Connecting => {
                draw_connecting_screen(ctx, &self.ui.connect_log);
            }
            Screen::Error(msg) => {
                draw_error_screen(ctx, self, &msg);
            }
            Screen::View => {
                draw_view_hud(ctx, self, &visible_toasts, &debug_arc_screen);
            }
        });

        let vp = self.viewports.get_mut(0).unwrap();
        vp.egui_state
            .handle_platform_output(win, full_output.platform_output);

        {
            let gpu = &mut vp.gpu;
            if self.viewer.vsync != prev_vsync {
                gpu.set_vsync(self.viewer.vsync);
            }
        }

        let paint_jobs = self
            .egui_ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);
        let textures_delta = full_output.textures_delta;

        let screen_desc = {
            let gpu = &vp.gpu;
            egui_wgpu::ScreenDescriptor {
                size_in_pixels: [gpu.config.width, gpu.config.height],
                pixels_per_point: win.scale_factor() as f32,
            }
        };

        let render_out = render::render_frame(
            vp,
            &mut self.viewer,
            view_proj,
            radius,
            paint_jobs,
            textures_delta,
            screen_desc,
            debug_arc_screen,
        );

        self.viewer.perf.push(render_out.frame_ms);

        vp.render_port.request_redraw();
    }
}
