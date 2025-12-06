// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Per-frame tick and window-event handling for the App.

use crate::{
    app::App,
    core::Screen,
    input,
    render,
    session::SessionPort,
    session_logic,
    ui::{draw_connecting_screen, draw_error_screen, draw_title_screen, draw_view_hud},
    ui_state,
};
use echo_app_core::{toast::{ToastKind, ToastScope}, render_port::RenderPort};
use echo_session_proto::{NotifyKind, NotifyScope};
use egui_winit::winit::event::WindowEvent;
use egui_winit::State as EguiWinitState;
use glam::{Quat, Vec3};
use std::time::Instant;

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

    pub fn frame(&mut self) {
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

        let speed = if self.viewer.keys.contains(&egui_winit::winit::keyboard::KeyCode::ShiftLeft)
            || self.viewer.keys.contains(&egui_winit::winit::keyboard::KeyCode::ShiftRight)
        {
            420.0
        } else {
            160.0
        };
        let mut mv = Vec3::ZERO;
        use egui_winit::winit::keyboard::KeyCode::*;
        if self.viewer.keys.contains(&KeyW) {
            mv.z += speed * dt;
        }
        if self.viewer.keys.contains(&KeyS) {
            mv.z -= speed * dt;
        }
        if self.viewer.keys.contains(&KeyA) {
            mv.x -= speed * dt;
        }
        if self.viewer.keys.contains(&KeyD) {
            mv.x += speed * dt;
        }
        if self.viewer.keys.contains(&KeyQ) {
            mv.y -= speed * dt;
        }
        if self.viewer.keys.contains(&KeyE) {
            mv.y += speed * dt;
        }
        self.viewer.camera.move_relative(mv);

        if matches!(self.ui.screen, Screen::View) {
            self.viewer.graph.step_layout(dt);
        }

        self.handle_pointer(aspect, width_px, height_px, win);

        let aspect = width_px as f32 / height_px as f32;
        let radius = self.viewer.graph.bounding_radius();
        let view_proj = self.viewer.camera.view_proj(aspect, radius);

        let debug_arc_screen = self.debug_arc_screen(width_px, height_px, win, view_proj);

        let prev_vsync = self.viewer.vsync;

        let egui_ctx = self.egui_ctx.clone();
        let full_output = egui_ctx.run(raw_input, |ctx| match self.ui.screen.clone() {
            Screen::Title => draw_title_screen(ctx, self),
            Screen::Connecting => draw_connecting_screen(ctx, &self.ui.connect_log),
            Screen::Error(msg) => draw_error_screen(ctx, self, &msg),
            Screen::View => draw_view_hud(ctx, self, &visible_toasts, &debug_arc_screen),
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

    fn handle_pointer(
        &mut self,
        aspect: f32,
        width_px: u32,
        height_px: u32,
        win: &'static egui_winit::winit::window::Window,
    ) {
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
                        self.viewer.graph_ang_vel = axis.normalize() * (angle / 0.0001_f32.max(0.0));
                    }
                }
                self.viewer.arc_last = Some(curr);
            }
        } else {
            let w = self.viewer.graph_ang_vel;
            let w_len = w.length();
            if w_len > 1e-4 {
                let angle = w_len * self.viewer.last_frame.elapsed().as_secs_f32();
                let dq = Quat::from_axis_angle(w / w_len, angle);
                self.viewer.graph_rot = dq * self.viewer.graph_rot;
                let decay =
                    (-self.viewer.graph_damping * self.viewer.last_frame.elapsed().as_secs_f32())
                        .exp();
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
    }

    fn debug_arc_screen(
        &self,
        width_px: u32,
        height_px: u32,
        win: &'static egui_winit::winit::window::Window,
        view_proj: glam::Mat4,
    ) -> Option<(egui::Pos2, egui::Pos2)> {
        if !self.viewer.debug_show_arc {
            return None;
        }
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
                return Some((to_screen(na), to_screen(nb)));
            }
        }
        None
    }
}
