// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! rmg-viewer: 3D RMG visualizer (wgpu 27, egui 0.33, winit 0.30 via egui-winit re-export).

use anyhow::Result;
use echo_app_core::{
    config::ConfigService,
    config_port::ConfigPort,
    prefs::ViewerPrefs,
    render_port::RenderPort,
    toast::{ToastKind, ToastScope, ToastService},
};
use echo_config_fs::FsConfigStore;
use echo_graph::RenderGraph as WireGraph;
use echo_session_client::connect_channels_for;
mod core;
use core::{Screen, UiState};
mod camera;
mod gpu;
mod input;
mod render_port;
mod ui_effects;
use render_port::WinitRenderPort;
mod session;
use echo_session_proto::{NotifyKind, NotifyScope};
use session::{SessionClient, SessionPort};
mod perf;
mod render;
mod scene;
mod session_logic;
mod ui;
mod ui_state;
use egui_extras::install_image_loaders;
use egui_winit::winit; // module alias for type paths
use egui_winit::winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::KeyCode,
    window::{Window, WindowAttributes},
};
use egui_winit::State as EguiWinitState;
use glam::{Quat, Vec3};
use gpu::{EdgeInstance, Globals, Gpu, Instance};
use perf::PerfStats;
use scene::{sample_wire_graph, scene_from_wire, History, RenderGraph};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;
use ui::{draw_connecting_screen, draw_error_screen, draw_title_screen, draw_view_hud};
use ui_effects::UiEffectsRunner;

// ------------------------------------------------------------
// Data
// ------------------------------------------------------------

struct ViewerState {
    wire_graph: WireGraph,
    graph: RenderGraph,
    history: History,
    epoch: Option<u64>,
    camera: camera::Camera,
    perf: PerfStats,
    last_frame: Instant,
    keys: HashSet<KeyCode>,
    // Arcball spin state for right-drag spinning the graph itself
    arc_active: bool,
    arc_last: Option<glam::Vec3>,
    arc_last_hit: Option<Vec3>,
    arc_curr_hit: Option<Vec3>,
    graph_rot: glam::Quat,
    graph_ang_vel: glam::Vec3,
    graph_damping: f32,
    debug_show_sphere: bool,
    debug_show_arc: bool,
    debug_invert_cam_x: bool,
    debug_invert_cam_y: bool,
    wireframe: bool,
    show_watermark: bool,
    #[allow(dead_code)]
    watermark_bytes: Arc<[u8]>,
    vsync: bool,
}

impl Default for ViewerState {
    fn default() -> Self {
        let svg = include_str!("../../../docs/assets/ECHO_chunky.svg");
        let svg_no_stroke = svg
            .replace("stroke=\"#ffffff\"", "stroke=\"none\"")
            .replace("stroke=\"#FFF\"", "stroke=\"none\"");
        let watermark_bytes: Arc<[u8]> = svg_no_stroke.into_bytes().into();
        let wire_graph = sample_wire_graph();
        let graph = scene_from_wire(&wire_graph);
        Self {
            wire_graph,
            graph,
            history: History::default(),
            epoch: None,
            camera: camera::Camera::default(),
            perf: PerfStats::default(),
            last_frame: Instant::now(),
            keys: HashSet::new(),
            arc_active: false,
            arc_last: None,
            arc_last_hit: None,
            arc_curr_hit: None,
            graph_rot: Quat::IDENTITY,
            graph_ang_vel: Vec3::ZERO,
            graph_damping: 2.5,
            debug_show_sphere: false,
            debug_show_arc: false,
            debug_invert_cam_x: false,
            debug_invert_cam_y: false,
            wireframe: false,
            show_watermark: true,
            watermark_bytes,
            vsync: false,
        }
    }
}

impl ViewerState {
    fn apply_prefs(&mut self, cfg: &ViewerPrefs) {
        let cam = &cfg.camera;
        let q = Quat::from_xyzw(
            cam.orientation[0],
            cam.orientation[1],
            cam.orientation[2],
            cam.orientation[3],
        );
        if q.is_finite() && q.length_squared() > 0.0 {
            self.camera.orientation = q.normalize();
        }
        if cam.pos.iter().all(|p| p.is_finite()) {
            self.camera.pos = Vec3::from_array(cam.pos);
        }
        if cam.pitch.is_finite() {
            self.camera.pitch = cam.pitch.clamp(-1.55, 1.55);
        }
        if cam.fov_y.is_finite() {
            self.camera.fov_y = cam.fov_y.clamp(15f32.to_radians(), 120f32.to_radians());
        }

        let hud = &cfg.hud;
        self.debug_show_sphere = hud.debug_show_sphere;
        self.debug_show_arc = hud.debug_show_arc;
        self.debug_invert_cam_x = hud.debug_invert_cam_x;
        self.debug_invert_cam_y = hud.debug_invert_cam_y;
        self.wireframe = hud.wireframe;
        self.show_watermark = hud.show_watermark;
        self.vsync = hud.vsync;
    }

    fn export_prefs(&self) -> ViewerPrefs {
        ViewerPrefs {
            camera: echo_app_core::prefs::CameraPrefs {
                pos: self.camera.pos.to_array(),
                orientation: self.camera.orientation.to_array(),
                pitch: self.camera.pitch,
                fov_y: self.camera.fov_y,
            },
            hud: echo_app_core::prefs::HudPrefs {
                debug_show_sphere: self.debug_show_sphere,
                debug_show_arc: self.debug_show_arc,
                debug_invert_cam_x: self.debug_invert_cam_x,
                debug_invert_cam_y: self.debug_invert_cam_y,
                wireframe: self.wireframe,
                show_watermark: self.show_watermark,
                vsync: self.vsync,
            },
        }
    }
}

// ApplicationHandler
// ------------------------------------------------------------

struct Viewport {
    window: &'static Window,
    gpu: Gpu,
    egui_state: EguiWinitState,
    egui_renderer: egui_wgpu::Renderer,
    render_port: WinitRenderPort,
}

struct App {
    viewports: Vec<Viewport>,
    egui_ctx: egui::Context,
    config: Option<Box<dyn ConfigPort>>, // boxed to decouple from concrete store
    ui_runner: ui_effects::RealEffectsRunner,
    toasts: ToastService,
    session: SessionClient,
    ui: UiState,
    viewer: ViewerState,
}

impl App {
    fn apply_ui_event(&mut self, ev: ui_state::UiEvent) {
        let (next, effects) = ui_state::reduce(&self.ui, ev);
        self.ui = next;
        // Run config-bound effects locally
        for eff in effects.iter() {
            if matches!(eff, ui_state::UiEffect::SavePrefs) {
                if let Some(cfg) = &self.config {
                    cfg.save_prefs(&self.viewer.export_prefs());
                }
            }
        }
        // Run remaining effects via runner (session/quits) and handle follow-ups
        let followups = self.ui_runner.run(effects, &mut self.session, &self.ui);
        for ev in followups {
            self.apply_ui_event(ev);
        }
    }
    fn new() -> Self {
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
        viewer.graph = scene_from_wire(&viewer.wire_graph);
        viewer.history.append(viewer.wire_graph.clone(), 0);
        viewer.epoch = Some(0);
        viewer.apply_prefs(&prefs);

        // Session notifications + RMG frames via session client (best-effort, non-fatal)
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
        let gpu = pollster::block_on(Gpu::new(window)).expect("gpu init");
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
        let render_port = render_port::WinitRenderPort::new(window);
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
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let Some(idx) = self
            .viewports
            .iter()
            .position(|v| v.window.id() == window_id)
        else {
            return;
        };
        // Split the mutable borrow to avoid aliasing conflicts
        let win = self.viewports[idx].window;
        let egui_state_ptr: *mut EguiWinitState = &mut self.viewports[idx].egui_state;

        // input handling via helper
        let outcome = input::handle_window_event(&event, win, &mut self.viewer, &mut self.ui);
        if let Some(ev) = outcome.ui_event {
            self.apply_ui_event(ev);
        }

        // Always forward events to egui after we handled movement keys so releases clear our state.
        // SAFETY: egui_state_ptr points to the current viewport's egui_state; we haven't moved it.
        let egui_state = unsafe { &mut *egui_state_ptr };
        let _ = egui_state.on_window_event(win, &event);
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if self.viewports.is_empty() {
            return;
        }
        // Snapshot immutable bits we need before egui run
        let (win, width_px, height_px, raw_input) = {
            let vp = self.viewports.get_mut(0).unwrap();
            let raw = vp.egui_state.take_egui_input(vp.window);
            (vp.window, vp.gpu.config.width, vp.gpu.config.height, raw)
        };

        // Drain any session notifications into the toast queue
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

        // Drain RMG frames into wire graph and rebuild scene; enforce no gaps
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

        // Arcball spin: right-drag spins the graph; left-drag is FPS look.
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
                // update current hit point along the pick ray for debug
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

        // Mouse look: adjust yaw/pitch directly when not over egui
        if pointer.primary_down() && !self.egui_ctx.is_using_pointer() {
            let delta = self.egui_ctx.input(|i| i.pointer.delta());
            let d = glam::Vec2::new(delta.x, delta.y);
            self.viewer.camera.rotate_by_mouse(
                d,
                self.viewer.debug_invert_cam_x,
                self.viewer.debug_invert_cam_y,
            );
        }

        let aspect = width_px as f32 / height_px as f32;
        let view_proj = self.viewer.camera.view_proj(aspect, radius);

        // Project debug arc line into screen space for egui overlay
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

        let gpu = &mut vp.gpu;

        if self.viewer.vsync != prev_vsync {
            gpu.set_vsync(self.viewer.vsync);
        }

        let screen_desc = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [gpu.config.width, gpu.config.height],
            pixels_per_point: win.scale_factor() as f32,
        };
        let paint_jobs = self
            .egui_ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);
        let textures_delta = full_output.textures_delta;

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
// ------------------------------------------------------------
// Main
// ------------------------------------------------------------

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .without_time()
        .init();
    let event_loop = EventLoop::new()?;
    let mut app = App::new();
    event_loop.run_app(&mut app)?;
    Ok(())
}
