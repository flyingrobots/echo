// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! rmg-viewer: 3D RMG visualizer (wgpu 27, egui 0.33, winit 0.30 via egui-winit re-export).

use anyhow::Result;
use blake3::Hasher;
use bytemuck::{Pod, Zeroable};
use echo_app_core::{
    config::ConfigService,
    prefs::ViewerPrefs,
    toast::{ToastKind, ToastScope, ToastService},
};
use echo_config_fs::FsConfigStore;
use echo_graph::{RenderGraph as WireGraph, RmgFrame};
use echo_session_client::connect_channels_for;
mod core;
use core::{Screen, TitleMode, UiState, ViewerOverlay};
mod session;
use echo_session_proto::{NotifyKind, NotifyScope};
use egui_extras::install_image_loaders;
use egui_wgpu::wgpu;
use egui_wgpu::wgpu::util::DeviceExt;
use egui_winit::winit; // module alias for type paths
use egui_winit::winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::KeyCode,
    window::{Window, WindowAttributes},
};
use egui_winit::State as EguiWinitState;
use glam::{Mat4, Quat, Vec3};
use rmg_core::{
    make_edge_id, make_node_id, make_type_id, EdgeRecord, GraphStore, NodeRecord, TypeId,
};
use session::{SessionClient, SessionPort};
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use std::time::Instant;

// ------------------------------------------------------------
// Data
// ------------------------------------------------------------

struct ViewerState {
    wire_graph: WireGraph,
    graph: RenderGraph,
    history: History,
    epoch: Option<u64>,
    camera: Camera,
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
            camera: Camera::default(),
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
                show_watermark: self.show_watermark,
                vsync: self.vsync,
            },
        }
    }
}

#[derive(Clone, Debug)]
struct RenderNode {
    #[allow(dead_code)]
    ty: TypeId,
    color: [f32; 3],
    pos: Vec3,
    vel: Vec3,
}

#[derive(Clone, Debug, Default)]
struct RenderGraph {
    nodes: Vec<RenderNode>,
    edges: Vec<(usize, usize)>,
    #[allow(dead_code)]
    max_depth: usize,
}

#[derive(Clone, Debug, Default)]
struct HistoryNode {
    #[allow(dead_code)]
    graph: WireGraph,
    #[allow(dead_code)]
    revision: u64,
    next: Option<Box<HistoryNode>>,
}

#[derive(Clone, Debug, Default)]
struct History {
    head: Option<Box<HistoryNode>>,
    tail_rev: u64,
    len: usize,
}

impl History {
    fn append(&mut self, graph: WireGraph, revision: u64) {
        let node = Box::new(HistoryNode {
            graph,
            revision,
            next: None,
        });
        match self.head.as_mut() {
            None => {
                self.tail_rev = revision;
                self.head = Some(node);
                self.len = 1;
            }
            Some(head) => {
                // Walk to tail and append (append-only, tiny list expected)
                let mut cur = head;
                while cur.next.is_some() {
                    let next = cur.next.as_mut().unwrap();
                    cur = next;
                }
                cur.next = Some(node);
                self.tail_rev = revision;
                self.len += 1;
            }
        }
    }

    #[allow(dead_code)]
    fn latest(&self) -> Option<&WireGraph> {
        let mut cur = self.head.as_ref()?;
        while let Some(n) = cur.next.as_ref() {
            cur = n;
        }
        Some(&cur.graph)
    }
}

fn sample_wire_graph() -> WireGraph {
    let store = build_sample_graph();
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    for (id, node) in store.iter_nodes() {
        nodes.push(echo_graph::RenderNode {
            id: id_to_u64(&id.0),
            kind: echo_graph::NodeKind::Generic,
            data: echo_graph::NodeData { raw: Vec::new() },
        });
        let _ = node; // silence
    }
    for (from, outs) in store.iter_edges() {
        for e in outs {
            edges.push(echo_graph::RenderEdge {
                id: id_to_u64(&e.id.0),
                src: id_to_u64(&from.0),
                dst: id_to_u64(&e.to.0),
                kind: echo_graph::EdgeKind::Generic,
                data: echo_graph::EdgeData { raw: Vec::new() },
            });
        }
    }
    WireGraph { nodes, edges }
}

fn id_to_u64(bytes: &[u8]) -> u64 {
    let mut arr = [0u8; 8];
    let take = bytes.len().min(8);
    arr[..take].copy_from_slice(&bytes[..take]);
    u64::from_le_bytes(arr)
}

fn scene_from_wire(w: &WireGraph) -> RenderGraph {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    for (i, n) in w.nodes.iter().enumerate() {
        let pos = radial_pos_u64(i as u64);
        let color = hash_color_u64(n.id);
        nodes.push(RenderNode {
            ty: make_type_id("node"),
            color,
            pos,
            vel: Vec3::ZERO,
        });
    }
    for e in &w.edges {
        edges.push((
            e.src as usize % nodes.len().max(1),
            e.dst as usize % nodes.len().max(1),
        ));
    }
    let max_depth = compute_depth(&edges, nodes.len());
    RenderGraph {
        nodes,
        edges,
        max_depth,
    }
}

impl RenderGraph {
    fn step_layout(&mut self, dt: f32) {
        let n = self.nodes.len();
        if n == 0 {
            return;
        }
        let mut forces = vec![Vec3::ZERO; n];
        for i in 0..n {
            for j in (i + 1)..n {
                let delta = self.nodes[i].pos - self.nodes[j].pos;
                let dist2 = delta.length_squared().max(9.0);
                let f = delta.normalize_or_zero() * (2400.0 / dist2);
                forces[i] += f;
                forces[j] -= f;
            }
        }
        for &(a, b) in &self.edges {
            let delta = self.nodes[b].pos - self.nodes[a].pos;
            let dist = delta.length().max(1.0);
            let dir = delta / dist;
            let target = 140.0;
            let f = dir * ((dist - target) * 0.08);
            forces[a] += f;
            forces[b] -= f;
        }
        for (i, node) in self.nodes.iter_mut().enumerate() {
            node.vel += forces[i] * dt;
            node.vel *= 0.9;
            node.pos += node.vel * dt;
        }
    }

    fn bounding_radius(&self) -> f32 {
        self.nodes
            .iter()
            .map(|n| n.pos.length())
            .fold(0.0, f32::max)
            .max(1.0)
    }
}

#[derive(Clone, Copy, Debug)]
struct Camera {
    pos: Vec3,
    orientation: Quat,
    pitch: f32,
    fov_y: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            pos: Vec3::new(0.0, 0.0, 520.0),
            orientation: Quat::IDENTITY,
            pitch: 0.0,
            fov_y: 60f32.to_radians(),
        }
    }
}

impl Camera {
    fn basis(&self) -> (Vec3, Vec3, Vec3) {
        let forward = self.orientation * -Vec3::Z;
        let right = self.orientation * Vec3::X;
        let up = self.orientation * Vec3::Y;
        (forward, right, up)
    }

    fn view_proj(&self, aspect: f32) -> Mat4 {
        let (f, _, u) = self.basis();
        let view = Mat4::look_to_rh(self.pos, f, u);
        let proj = Mat4::perspective_rh(self.fov_y, aspect, 0.1, 10_000.0);
        proj * view
    }

    fn rotate_by_mouse(&mut self, delta: glam::Vec2, invert_x: bool, invert_y: bool) {
        // Standard FPS-style mouse look with optional axis inversion
        let sensitivity = 0.0025;
        let yaw_delta = delta.x * sensitivity * if invert_x { -1.0 } else { 1.0 };
        let pitch_delta = (-delta.y) * sensitivity * if invert_y { -1.0 } else { 1.0 };

        // yaw about global Y
        let yaw_q = Quat::from_axis_angle(Vec3::Y, yaw_delta);
        self.orientation = yaw_q * self.orientation;

        // pitch about camera right, with clamp
        let new_pitch = (self.pitch + pitch_delta).clamp(-1.4, 1.4);
        let applied = new_pitch - self.pitch;
        if applied.abs() > 1e-6 {
            let right = self.orientation * Vec3::X;
            let pitch_q = Quat::from_axis_angle(right, applied);
            self.orientation = pitch_q * self.orientation;
            self.pitch = new_pitch;
        }

        self.orientation = self.orientation.normalize();
    }

    fn pick_ray(&self, ndc: glam::Vec2, aspect: f32) -> Vec3 {
        // ndc in [-1,1] with y up
        let (f, r, u) = self.basis();
        let t = (self.fov_y * 0.5).tan();
        (f + r * (ndc.x * t * aspect) + u * (ndc.y * t)).normalize()
    }

    fn move_relative(&mut self, dir: Vec3) {
        let (f, r, u) = self.basis();
        self.pos += f * dir.z + r * dir.x + u * dir.y;
    }

    fn zoom_fov(&mut self, factor: f32) {
        let deg = (self.fov_y.to_degrees() * factor).clamp(10.0, 120.0);
        self.fov_y = deg.to_radians();
    }
}

#[derive(Clone, Debug)]
struct PerfStats {
    frame_ms: VecDeque<f32>,
    max_samples: usize,
}
impl Default for PerfStats {
    fn default() -> Self {
        Self {
            frame_ms: VecDeque::with_capacity(400),
            max_samples: 400,
        }
    }
}
impl PerfStats {
    fn push(&mut self, frame: f32) {
        if self.frame_ms.len() == self.max_samples {
            self.frame_ms.pop_front();
        }
        self.frame_ms.push_back(frame);
    }
    fn fps(&self) -> f32 {
        self.frame_ms.back().map(|ms| 1000.0 / ms).unwrap_or(0.0)
    }
}

// GPU types --------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    pos: [f32; 3],
    normal: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Instance {
    model: [[f32; 4]; 4],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct EdgeInstance {
    start: [f32; 3],
    end: [f32; 3],
    color: [f32; 3],
    head: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Globals {
    view_proj: [[f32; 4]; 4],
    light_dir: [f32; 3],
    _pad: f32,
}

struct Mesh {
    vbuf: wgpu::Buffer,
    ibuf: wgpu::Buffer,
    count: u32,
}

struct Pipelines {
    node: wgpu::RenderPipeline,
    node_wire: wgpu::RenderPipeline,
    edge: wgpu::RenderPipeline,
}

struct Gpu {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pmode_fast: wgpu::PresentMode,
    pmode_vsync: wgpu::PresentMode,
    sample_count: u32,
    max_tex: u32,
    msaa_view: Option<wgpu::TextureView>,
    depth: wgpu::TextureView,
    mesh_sphere: Mesh,
    mesh_debug_sphere: Mesh,
    globals_buf: wgpu::Buffer,
    instance_buf: wgpu::Buffer,
    edge_buf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    pipelines: Pipelines,
}

impl Gpu {
    async fn new(window: &'static Window) -> Result<Self> {
        let instance = wgpu::Instance::default();
        // wgpu 27 keeps surface lifetime tied to window; leak window for 'static
        let surface = instance.create_surface(window)?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("GPU adapter");
        let limits = adapter.limits();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("rmg-viewer-device"),
                required_features: wgpu::Features::POLYGON_MODE_LINE,
                required_limits: wgpu::Limits::downlevel_defaults()
                    .using_resolution(limits.clone()),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
            })
            .await?;

        let size = window.inner_size();
        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);
        let pmode_fast = caps
            .present_modes
            .iter()
            .copied()
            .find(|m| {
                matches!(
                    m,
                    wgpu::PresentMode::Immediate | wgpu::PresentMode::AutoNoVsync
                )
            })
            .unwrap_or(wgpu::PresentMode::Fifo);
        let pmode_vsync = caps
            .present_modes
            .iter()
            .copied()
            .find(|m| matches!(m, wgpu::PresentMode::Fifo))
            .unwrap_or(pmode_fast);
        let max_dim = limits.max_texture_dimension_2d;
        let sample_count = 4;
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.min(max_dim).max(1),
            height: size.height.min(max_dim).max(1),
            present_mode: pmode_fast,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);
        let depth = create_depth(&device, config.width, config.height, sample_count);
        let msaa_view = create_msaa(
            &device,
            config.format,
            config.width,
            config.height,
            sample_count,
        );

        let mesh_sphere = unit_octahedron(&device);
        let mesh_debug_sphere = unit_uv_sphere(&device, 24, 16);

        let globals_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("globals"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let instance_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instances"),
            size: (std::mem::size_of::<Instance>() * 8192) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let edge_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("edges"),
            size: (std::mem::size_of::<EdgeInstance>() * 16384) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let globals_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("globals_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("globals_bg"),
            layout: &globals_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: globals_buf.as_entire_binding(),
            }],
        });

        let shader_nodes = device.create_shader_module(wgpu::include_wgsl!("shader_nodes.wgsl"));
        let shader_edges = device.create_shader_module(wgpu::include_wgsl!("shader_edges.wgsl"));

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline_layout"),
            bind_group_layouts: &[&globals_layout],
            push_constant_ranges: &[],
        });

        let node = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("node_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_nodes,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0=>Float32x3,1=>Float32x3],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Instance>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            wgpu::VertexAttribute {
                                shader_location: 2,
                                offset: 0,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                            wgpu::VertexAttribute {
                                shader_location: 3,
                                offset: 16,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                            wgpu::VertexAttribute {
                                shader_location: 4,
                                offset: 32,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                            wgpu::VertexAttribute {
                                shader_location: 5,
                                offset: 48,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                            wgpu::VertexAttribute {
                                shader_location: 6,
                                offset: 64,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                        ],
                    },
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_nodes,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: sample_count,
                ..Default::default()
            },
            multiview: None,
            cache: None,
        });

        let node_wire = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("node_wire_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_nodes,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0=>Float32x3,1=>Float32x3],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Instance>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            wgpu::VertexAttribute {
                                shader_location: 2,
                                offset: 0,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                            wgpu::VertexAttribute {
                                shader_location: 3,
                                offset: 16,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                            wgpu::VertexAttribute {
                                shader_location: 4,
                                offset: 32,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                            wgpu::VertexAttribute {
                                shader_location: 5,
                                offset: 48,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                            wgpu::VertexAttribute {
                                shader_location: 6,
                                offset: 64,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                        ],
                    },
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_nodes,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                polygon_mode: wgpu::PolygonMode::Line,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: sample_count,
                ..Default::default()
            },
            multiview: None,
            cache: None,
        });

        let edge = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("edge_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_edges,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<EdgeInstance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![
                        0=>Float32x3,
                        1=>Float32x3,
                        2=>Float32x3,
                        3=>Float32
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_edges,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: sample_count,
                ..Default::default()
            },
            multiview: None,
            cache: None,
        });

        Ok(Self {
            surface,
            device,
            queue,
            config,
            pmode_fast,
            pmode_vsync,
            sample_count,
            max_tex: max_dim,
            msaa_view,
            depth,
            mesh_sphere,
            mesh_debug_sphere,
            globals_buf,
            instance_buf,
            edge_buf,
            bind_group,
            pipelines: Pipelines {
                node,
                node_wire,
                edge,
            },
        })
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.config.width = size.width.min(self.max_tex);
        self.config.height = size.height.min(self.max_tex);
        self.surface.configure(&self.device, &self.config);
        self.depth = create_depth(
            &self.device,
            self.config.width,
            self.config.height,
            self.sample_count,
        );
        self.msaa_view = create_msaa(
            &self.device,
            self.config.format,
            self.config.width,
            self.config.height,
            self.sample_count,
        );
    }

    fn set_vsync(&mut self, on: bool) {
        let mode = if on {
            self.pmode_vsync
        } else {
            self.pmode_fast
        };
        if self.config.present_mode != mode {
            self.config.present_mode = mode;
            self.surface.configure(&self.device, &self.config);
        }
    }
}

// ------------------------------------------------------------
// Helpers
// ------------------------------------------------------------

fn create_depth(device: &wgpu::Device, w: u32, h: u32, sample_count: u32) -> wgpu::TextureView {
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth"),
        size: wgpu::Extent3d {
            width: w.max(1),
            height: h.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    tex.create_view(&wgpu::TextureViewDescriptor::default())
}

fn create_msaa(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    w: u32,
    h: u32,
    sample_count: u32,
) -> Option<wgpu::TextureView> {
    if sample_count <= 1 {
        return None;
    }
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("msaa_color"),
        size: wgpu::Extent3d {
            width: w.max(1),
            height: h.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    Some(tex.create_view(&wgpu::TextureViewDescriptor::default()))
}

fn unit_octahedron(device: &wgpu::Device) -> Mesh {
    let verts: [Vertex; 6] = [
        Vertex {
            pos: [1.0, 0.0, 0.0],
            normal: [1.0, 0.0, 0.0],
        },
        Vertex {
            pos: [-1.0, 0.0, 0.0],
            normal: [-1.0, 0.0, 0.0],
        },
        Vertex {
            pos: [0.0, 1.0, 0.0],
            normal: [0.0, 1.0, 0.0],
        },
        Vertex {
            pos: [0.0, -1.0, 0.0],
            normal: [0.0, -1.0, 0.0],
        },
        Vertex {
            pos: [0.0, 0.0, 1.0],
            normal: [0.0, 0.0, 1.0],
        },
        Vertex {
            pos: [0.0, 0.0, -1.0],
            normal: [0.0, 0.0, -1.0],
        },
    ];
    let idx: [u16; 24] = [
        0, 2, 4, 2, 1, 4, 1, 3, 4, 3, 0, 4, 2, 0, 5, 1, 2, 5, 3, 1, 5, 0, 3, 5,
    ];
    let vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("oct_vb"),
        contents: bytemuck::cast_slice(&verts),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("oct_ib"),
        contents: bytemuck::cast_slice(&idx),
        usage: wgpu::BufferUsages::INDEX,
    });
    Mesh {
        vbuf,
        ibuf,
        count: idx.len() as u32,
    }
}

fn unit_uv_sphere(device: &wgpu::Device, segments: u32, rings: u32) -> Mesh {
    let mut verts = Vec::new();
    let mut idx = Vec::new();
    for y in 0..=rings {
        let v = y as f32 / rings as f32;
        let theta = v * std::f32::consts::PI;
        for x in 0..=segments {
            let u = x as f32 / segments as f32;
            let phi = u * std::f32::consts::TAU;
            let nx = phi.sin() * theta.sin();
            let ny = theta.cos();
            let nz = phi.cos() * theta.sin();
            verts.push(Vertex {
                pos: [nx, ny, nz],
                normal: [nx, ny, nz],
            });
        }
    }
    let stride = segments + 1;
    for y in 0..rings {
        for x in 0..segments {
            let i0 = y * stride + x;
            let i1 = i0 + 1;
            let i2 = i0 + stride;
            let i3 = i2 + 1;
            idx.extend_from_slice(&[i0 as u16, i2 as u16, i1 as u16]);
            idx.extend_from_slice(&[i1 as u16, i2 as u16, i3 as u16]);
        }
    }

    let vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("uv_sphere_vb"),
        contents: bytemuck::cast_slice(&verts),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("uv_sphere_ib"),
        contents: bytemuck::cast_slice(&idx),
        usage: wgpu::BufferUsages::INDEX,
    });
    Mesh {
        vbuf,
        ibuf,
        count: idx.len() as u32,
    }
}

fn radial_pos_u64(id: u64) -> Vec3 {
    let mut h = Hasher::new();
    h.update(&id.to_le_bytes());
    let bytes = h.finalize();
    let theta = u32::from_le_bytes(bytes.as_bytes()[0..4].try_into().unwrap()) as f32
        / u32::MAX as f32
        * std::f32::consts::TAU;
    let phi = u32::from_le_bytes(bytes.as_bytes()[4..8].try_into().unwrap()) as f32
        / u32::MAX as f32
        * std::f32::consts::PI
        - std::f32::consts::FRAC_PI_2;
    let r = 200.0;
    Vec3::new(
        r * phi.cos() * theta.cos(),
        r * phi.sin(),
        r * phi.cos() * theta.sin(),
    )
}

fn compute_depth(edges: &[(usize, usize)], n: usize) -> usize {
    let mut adj = vec![Vec::new(); n];
    for &(a, b) in edges {
        if a < n && b < n {
            adj[a].push(b);
        }
    }
    let mut depth = vec![0usize; n];
    let mut stack = vec![0usize];
    let mut visited = vec![false; n];
    while let Some(v) = stack.pop() {
        visited[v] = true;
        let d = depth[v] + 1;
        for &m in &adj[v] {
            depth[m] = depth[m].max(d);
            if !visited[m] {
                stack.push(m);
            }
        }
    }
    depth.into_iter().max().unwrap_or(0)
}

fn hash_color_u64(id: u64) -> [f32; 3] {
    let h = blake3::hash(&id.to_be_bytes());
    let b = h.as_bytes();
    [
        b[0] as f32 / 255.0,
        b[1] as f32 / 255.0,
        b[2] as f32 / 255.0,
    ]
}

// ------------------------------------------------------------
// Sample graph (placeholder until hooked to Echo pipeline)
// ------------------------------------------------------------

fn build_sample_graph() -> GraphStore {
    let mut store = GraphStore::default();
    let world_ty = make_type_id("world");
    let region_ty = make_type_id("region");
    let leaf_ty = make_type_id("leaf");
    let worm_ty = make_type_id("wormhole");

    let world = make_node_id("world");
    store.insert_node(
        world,
        NodeRecord {
            ty: world_ty,
            payload: None,
        },
    );

    for i in 0..8u8 {
        let id = make_node_id(&format!("region-{i}"));
        store.insert_node(
            id,
            NodeRecord {
                ty: region_ty,
                payload: None,
            },
        );
        store.insert_edge(
            world,
            EdgeRecord {
                id: make_edge_id(&format!("world-region-{i}")),
                from: world,
                to: id,
                ty: region_ty,
                payload: None,
            },
        );
        for j in 0..3u8 {
            let leaf = make_node_id(&format!("leaf-{i}-{j}"));
            store.insert_node(
                leaf,
                NodeRecord {
                    ty: leaf_ty,
                    payload: None,
                },
            );
            store.insert_edge(
                id,
                EdgeRecord {
                    id: make_edge_id(&format!("edge-{i}-{j}")),
                    from: id,
                    to: leaf,
                    ty: leaf_ty,
                    payload: None,
                },
            );
        }
    }
    for pair in [(0, 3), (2, 6), (5, 7)] {
        let (a, b) = pair;
        let a_id = make_node_id(&format!("region-{a}"));
        let b_id = make_node_id(&format!("region-{b}"));
        store.insert_edge(
            a_id,
            EdgeRecord {
                id: make_edge_id(&format!("worm-{a}-{b}")),
                from: a_id,
                to: b_id,
                ty: worm_ty,
                payload: None,
            },
        );
        store.insert_edge(
            b_id,
            EdgeRecord {
                id: make_edge_id(&format!("worm-{b}-{a}")),
                from: b_id,
                to: a_id,
                ty: worm_ty,
                payload: None,
            },
        );
    }
    store
}

// ------------------------------------------------------------
// ApplicationHandler
// ------------------------------------------------------------

struct App {
    window: Option<&'static Window>,
    gpu: Option<Gpu>,
    egui_ctx: egui::Context,
    egui_state: Option<EguiWinitState>,
    egui_renderer: Option<egui_wgpu::Renderer>,
    config: Option<ConfigService<FsConfigStore>>,
    toasts: ToastService,
    session: SessionClient,
    ui: UiState,
    viewer: ViewerState,
}

impl App {
    fn new() -> Self {
        let egui_ctx = egui::Context::default();
        install_image_loaders(&egui_ctx);
        let config = FsConfigStore::new().map(ConfigService::new).ok();
        let prefs = config
            .as_ref()
            .and_then(|c| c.load::<ViewerPrefs>("viewer_prefs").ok().flatten())
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
            window: None,
            gpu: None,
            egui_ctx,
            egui_state: None,
            egui_renderer: None,
            config,
            toasts,
            session: SessionClient::new(),
            ui: UiState::new(),
            viewer,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
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
        self.window = Some(window);

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
        self.gpu = Some(gpu);
        self.egui_state = Some(egui_state);
        self.egui_renderer = Some(egui_renderer);
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        mut event: WindowEvent,
    ) {
        let Some(win) = self.window else { return };
        if win.id() != window_id {
            return;
        }
        let (Some(gpu), Some(egui_state)) = (&mut self.gpu, &mut self.egui_state) else {
            return;
        };

        match &mut event {
            WindowEvent::CloseRequested => {
                if let Some(cfg) = &self.config {
                    if let Err(e) = cfg.save("viewer_prefs", &self.viewer.export_prefs()) {
                        self.toasts.push(
                            ToastKind::Error,
                            ToastScope::Local,
                            "Failed to save viewer prefs",
                            Some(format!("{e:#}")),
                            std::time::Duration::from_secs(6),
                            Instant::now(),
                        );
                    }
                }
                std::process::exit(0);
            }
            WindowEvent::Resized(size) => gpu.resize(*size),
            WindowEvent::ScaleFactorChanged {
                scale_factor: _,
                inner_size_writer,
            } => {
                let size = win.inner_size();
                let _ = inner_size_writer.request_inner_size(size);
                gpu.resize(size);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let winit::keyboard::PhysicalKey::Code(code) = event.physical_key {
                    match event.state {
                        ElementState::Pressed => {
                            self.viewer.keys.insert(code);
                        }
                        ElementState::Released => {
                            self.viewer.keys.remove(&code);
                        }
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let y: f32 = match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 50.0,
                };
                self.viewer.camera.zoom_fov(1.0 - y * 0.05);
            }
            _ => {}
        }

        // Always forward events to egui after we handled movement keys so releases clear our state.
        let _ = egui_state.on_window_event(win, &event);
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let Some(win) = self.window else { return };
        let (width_px, height_px) = match self.gpu.as_ref() {
            Some(g) => (g.config.width, g.config.height),
            None => return,
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
        let mut desync: Option<String> = None;
        for frame in SessionPort::drain_frames(&mut self.session, 64) {
            match frame {
                RmgFrame::Snapshot(s) => {
                    self.viewer.wire_graph = s.graph;
                    self.viewer.epoch = Some(s.epoch);
                    self.viewer
                        .history
                        .append(self.viewer.wire_graph.clone(), s.epoch);
                    self.viewer.graph = scene_from_wire(&self.viewer.wire_graph);
                    self.ui.screen = Screen::View;
                    if let Some(expected) = s.state_hash {
                        let actual = self.viewer.wire_graph.compute_hash();
                        if actual != expected {
                            self.toasts.push(
                                ToastKind::Error,
                                ToastScope::Local,
                                "Snapshot hash mismatch",
                                None,
                                std::time::Duration::from_secs(6),
                                Instant::now(),
                            );
                        }
                    }
                }
                RmgFrame::Diff(d) => {
                    let Some(epoch) = self.viewer.epoch else {
                        self.toasts.push(
                            ToastKind::Error,
                            ToastScope::Local,
                            "Diff received before snapshot",
                            None,
                            std::time::Duration::from_secs(6),
                            Instant::now(),
                        );
                        continue;
                    };
                    if d.from_epoch != epoch || d.to_epoch != epoch + 1 {
                        self.toasts.push(
                            ToastKind::Error,
                            ToastScope::Local,
                            "Protocol violation: non-sequential diff",
                            Some(format!(
                                "from={}, to={}, local={}",
                                d.from_epoch, d.to_epoch, epoch
                            )),
                            std::time::Duration::from_secs(8),
                            Instant::now(),
                        );
                        desync = Some("Desynced (gap) — reconnect".into());
                        break;
                    }
                    for op in d.ops {
                        if let Err(err) = self.viewer.wire_graph.apply_op(op) {
                            self.toasts.push(
                                ToastKind::Error,
                                ToastScope::Local,
                                "Failed applying RMG op",
                                Some(format!("{err:#}")),
                                std::time::Duration::from_secs(8),
                                Instant::now(),
                            );
                            desync = Some("Desynced (apply failed) — reconnect".into());
                            break;
                        }
                    }
                    if desync.is_some() {
                        break;
                    }
                    self.viewer.epoch = Some(d.to_epoch);
                    if let Some(expected) = d.state_hash {
                        let actual = self.viewer.wire_graph.compute_hash();
                        if actual != expected {
                            self.toasts.push(
                                ToastKind::Error,
                                ToastScope::Local,
                                "State hash mismatch",
                                Some(format!("expected {:?}, got {:?}", expected, actual)),
                                std::time::Duration::from_secs(8),
                                Instant::now(),
                            );
                            desync = Some("Desynced (hash mismatch) — reconnect".into());
                        }
                    }
                    self.viewer
                        .history
                        .append(self.viewer.wire_graph.clone(), d.to_epoch);
                    self.viewer.graph = scene_from_wire(&self.viewer.wire_graph);
                    self.ui.screen = Screen::View;
                }
            }
        }
        if let Some(reason) = desync {
            SessionPort::clear_streams(&mut self.session);
            self.ui.screen = Screen::Error(reason);
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
        let view_proj = self.viewer.camera.view_proj(aspect);

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

        let raw_input = match self.egui_state.as_mut() {
            Some(es) => es.take_egui_input(win),
            None => return,
        };
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

        if let Some(es) = self.egui_state.as_mut() {
            es.handle_platform_output(win, full_output.platform_output);
        }

        let Some(gpu) = self.gpu.as_mut() else {
            return;
        };

        if self.viewer.vsync != prev_vsync {
            gpu.set_vsync(self.viewer.vsync);
        }

        let globals = Globals {
            view_proj: view_proj.to_cols_array_2d(),
            light_dir: [0.2, 0.7, 0.6],
            _pad: 0.0,
        };
        gpu.queue
            .write_buffer(&gpu.globals_buf, 0, bytemuck::bytes_of(&globals));

        let graph_rot = Mat4::from_quat(self.viewer.graph_rot);

        let mut instances = Vec::with_capacity(self.viewer.graph.nodes.len() + 1);
        for n in &self.viewer.graph.nodes {
            let world_pos = self.viewer.graph_rot * n.pos;
            let model = (Mat4::from_translation(world_pos)
                * graph_rot
                * Mat4::from_scale(Vec3::splat(7.0)))
            .to_cols_array_2d();
            instances.push(Instance {
                model,
                color: [n.color[0], n.color[1], n.color[2], 1.0],
            });
        }
        let node_instance_count = instances.len() as u32;
        let sphere_instance_offset = instances.len() as u32;
        if self.viewer.debug_show_sphere {
            let model = (graph_rot * Mat4::from_scale(Vec3::splat(radius))).to_cols_array_2d();
            instances.push(Instance {
                model,
                color: [1.0, 0.9, 0.2, 0.3],
            });
        }
        gpu.queue
            .write_buffer(&gpu.instance_buf, 0, bytemuck::cast_slice(&instances));

        let mut edge_instances = Vec::with_capacity(self.viewer.graph.edges.len() + 2);
        for (a, b) in &self.viewer.graph.edges {
            let sa = self.viewer.graph_rot * self.viewer.graph.nodes[*a].pos;
            let sb = self.viewer.graph_rot * self.viewer.graph.nodes[*b].pos;
            let color = self.viewer.graph.nodes[*a].color;
            let head = 7.0; // node radius to keep arrowheads off the sphere
            edge_instances.push(EdgeInstance {
                start: sa.to_array(),
                end: sb.to_array(),
                color,
                head,
            });
        }
        if self.viewer.debug_show_arc {
            if let (Some(a), Some(b)) = (self.viewer.arc_last_hit, self.viewer.arc_curr_hit) {
                edge_instances.push(EdgeInstance {
                    start: a.to_array(),
                    end: b.to_array(),
                    color: [1.0, 0.2, 0.8],
                    head: 0.0,
                });
            }
        }
        gpu.queue
            .write_buffer(&gpu.edge_buf, 0, bytemuck::cast_slice(&edge_instances));

        let frame = match gpu.surface.get_current_texture() {
            Ok(f) => f,
            Err(wgpu::SurfaceError::Lost) => {
                gpu.resize(PhysicalSize::new(gpu.config.width, gpu.config.height));
                match gpu.surface.get_current_texture() {
                    Ok(f) => f,
                    Err(_) => return,
                }
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                std::process::exit(1);
            }
            Err(e) => {
                eprintln!("frame drop: {e:?}");
                return;
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let (color_view, resolve_view) = if let Some(msaa) = &gpu.msaa_view {
            (msaa, Some(&view))
        } else {
            (&view, None)
        };

        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("main-encoder"),
            });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color_view,
                    resolve_target: resolve_view,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.05,
                            g: 0.06,
                            b: 0.08,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &gpu.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            rpass.set_pipeline(&gpu.pipelines.edge);
            rpass.set_bind_group(0, &gpu.bind_group, &[]);
            rpass.set_vertex_buffer(
                0,
                gpu.edge_buf.slice(
                    ..(edge_instances.len() as u64 * std::mem::size_of::<EdgeInstance>() as u64),
                ),
            );
            rpass.draw(0..2, 0..edge_instances.len() as u32);

            // draw nodes
            rpass.set_pipeline(&gpu.pipelines.node);
            rpass.set_bind_group(0, &gpu.bind_group, &[]);
            rpass.set_vertex_buffer(0, gpu.mesh_sphere.vbuf.slice(..));
            rpass.set_vertex_buffer(
                1,
                gpu.instance_buf
                    .slice(..(instances.len() as u64 * std::mem::size_of::<Instance>() as u64)),
            );
            rpass.set_index_buffer(gpu.mesh_sphere.ibuf.slice(..), wgpu::IndexFormat::Uint16);
            rpass.draw_indexed(0..gpu.mesh_sphere.count, 0, 0..node_instance_count);

            // debug sphere using higher-poly mesh
            if self.viewer.debug_show_sphere {
                let offset_bytes =
                    sphere_instance_offset as u64 * std::mem::size_of::<Instance>() as u64;
                rpass.set_vertex_buffer(0, gpu.mesh_debug_sphere.vbuf.slice(..));
                rpass.set_vertex_buffer(1, gpu.instance_buf.slice(offset_bytes..));
                rpass.set_index_buffer(
                    gpu.mesh_debug_sphere.ibuf.slice(..),
                    wgpu::IndexFormat::Uint16,
                );
                rpass.draw_indexed(0..gpu.mesh_debug_sphere.count, 0, 0..1);
                // wireframe overlay
                rpass.set_pipeline(&gpu.pipelines.node_wire);
                rpass.set_vertex_buffer(0, gpu.mesh_debug_sphere.vbuf.slice(..));
                rpass.set_vertex_buffer(1, gpu.instance_buf.slice(offset_bytes..));
                rpass.set_index_buffer(
                    gpu.mesh_debug_sphere.ibuf.slice(..),
                    wgpu::IndexFormat::Uint16,
                );
                rpass.draw_indexed(0..gpu.mesh_debug_sphere.count, 0, 0..1);
            }
        }

        let cmd_main = encoder.finish();

        let screen_desc = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [gpu.config.width, gpu.config.height],
            pixels_per_point: win.scale_factor() as f32,
        };
        let paint_jobs = self
            .egui_ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);
        let textures_delta = full_output.textures_delta;
        let Some(egui_renderer) = self.egui_renderer.as_mut() else {
            return;
        };

        let cmd_ui = {
            let mut egui_encoder =
                gpu.device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("egui-encoder"),
                    });

            for (id, delta) in textures_delta.set {
                egui_renderer.update_texture(&gpu.device, &gpu.queue, id, &delta);
            }
            egui_renderer.update_buffers(
                &gpu.device,
                &gpu.queue,
                &mut egui_encoder,
                &paint_jobs,
                &screen_desc,
            );
            {
                let rpass = egui_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("egui"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });
                let mut rpass = rpass.forget_lifetime();
                egui_renderer.render(&mut rpass, &paint_jobs, &screen_desc);
                drop(rpass);
            }
            for id in textures_delta.free {
                egui_renderer.free_texture(&id);
            }

            egui_encoder.finish()
        };
        gpu.queue.submit([cmd_main, cmd_ui]);
        frame.present();

        let frame_ms = self.viewer.last_frame.elapsed().as_secs_f32() * 1000.0;
        self.viewer.perf.push(frame_ms);

        win.request_redraw();
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

fn draw_title_screen(ctx: &egui::Context, app: &mut App) {
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
                        // start connecting
                        app.ui.connect_log.clear();
                        app.ui.connect_log.push(format!(
                            "Connecting to {}:{}",
                            app.ui.connect_host, app.ui.connect_port
                        ));
                        let path = format!("{}:{}", app.ui.connect_host, app.ui.connect_port);
                        let (rmg_rx, notif_rx) = connect_channels_for(&path, app.ui.rmg_id);
                        app.session.set_channels(rmg_rx, notif_rx);
                        app.ui.screen = Screen::Connecting;
                        app.ui.title_mode = TitleMode::Menu;
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

fn draw_connecting_screen(ctx: &egui::Context, log: &[String]) {
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

fn draw_error_screen(ctx: &egui::Context, app: &mut App, msg: &str) {
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

fn draw_view_hud(
    ctx: &egui::Context,
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
