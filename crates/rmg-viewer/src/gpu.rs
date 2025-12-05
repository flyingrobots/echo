// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! GPU setup and resources for the viewer.

use anyhow::Result;
use egui_wgpu::wgpu;
use egui_winit::winit::dpi::PhysicalSize;
use egui_winit::winit::window::Window;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Instance {
    pub model: [[f32; 4]; 4],
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct EdgeInstance {
    pub start: [f32; 3],
    pub end: [f32; 3],
    pub color: [f32; 3],
    pub head: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Globals {
    pub view_proj: [[f32; 4]; 4],
    pub light_dir: [f32; 3],
    pub _pad: f32,
}

pub struct Mesh {
    pub vbuf: wgpu::Buffer,
    pub ibuf: wgpu::Buffer,
    pub count: u32,
}

pub struct Pipelines {
    pub node: wgpu::RenderPipeline,
    pub node_wire: wgpu::RenderPipeline,
    pub edge: wgpu::RenderPipeline,
}

pub struct Gpu {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub pmode_fast: wgpu::PresentMode,
    pub pmode_vsync: wgpu::PresentMode,
    pub sample_count: u32,
    pub max_tex: u32,
    pub msaa_view: Option<wgpu::TextureView>,
    pub depth: wgpu::TextureView,
    pub mesh_sphere: Mesh,
    pub mesh_debug_sphere: Mesh,
    pub globals_buf: wgpu::Buffer,
    pub instance_buf: wgpu::Buffer,
    pub edge_buf: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub pipelines: Pipelines,
}

impl Gpu {
    pub async fn new(window: &'static Window) -> Result<Self> {
        let instance = wgpu::Instance::default();
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

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
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

    pub fn set_vsync(&mut self, on: bool) {
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

// Helpers ------------------------------------------------------------

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
