// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Viewer runtime state (camera, graph, prefs, input flags).

use crate::{
    camera,
    perf::PerfStats,
    scene::{History, RenderGraph},
};
use echo_app_core::prefs::ViewerPrefs;
use echo_graph::RenderGraph as WireGraph;
use glam::{Quat, Vec3};
use std::{collections::HashSet, sync::Arc, time::Instant};

pub struct ViewerState {
    pub wire_graph: WireGraph,
    pub graph: RenderGraph,
    pub history: History,
    pub epoch: Option<u64>,
    pub camera: camera::Camera,
    pub perf: PerfStats,
    pub last_frame: Instant,
    pub keys: HashSet<egui_winit::winit::keyboard::KeyCode>,
    pub arc_active: bool,
    pub arc_last: Option<glam::Vec3>,
    pub arc_last_hit: Option<Vec3>,
    pub arc_curr_hit: Option<Vec3>,
    pub graph_rot: glam::Quat,
    pub graph_ang_vel: glam::Vec3,
    pub graph_damping: f32,
    pub debug_show_sphere: bool,
    pub debug_show_arc: bool,
    pub debug_invert_cam_x: bool,
    pub debug_invert_cam_y: bool,
    pub wireframe: bool,
    pub show_watermark: bool,
    #[allow(dead_code)]
    pub watermark_bytes: Arc<[u8]>,
    pub vsync: bool,
}

impl Default for ViewerState {
    fn default() -> Self {
        let svg = include_str!("../../../docs/assets/ECHO_chunky.svg");
        let svg_no_stroke = svg
            .replace("stroke=\"#ffffff\"", "stroke=\"none\"")
            .replace("stroke=\"#FFF\"", "stroke=\"none\"");
        let watermark_bytes: Arc<[u8]> = svg_no_stroke.into_bytes().into();
        Self {
            wire_graph: WireGraph {
                nodes: Vec::new(),
                edges: Vec::new(),
            },
            graph: RenderGraph::default(),
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
    pub fn apply_prefs(&mut self, cfg: &ViewerPrefs) {
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

    pub fn export_prefs(&self) -> ViewerPrefs {
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
