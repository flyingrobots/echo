// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Render helper: prepares GPU buffers and draws the 3D scene + egui.

use crate::{
    gpu::{EdgeInstance, Globals, Instance},
    viewer_state::ViewerState,
    viewport::Viewport,
};
use egui_wgpu::wgpu;
use glam::Mat4;

pub struct RenderOutputs {
    pub frame_ms: f32,
}

/// Render the scene and UI. Returns frame timing.
#[allow(clippy::too_many_arguments)]
pub fn render_frame(
    vp: &mut Viewport,
    viewer: &mut ViewerState,
    view_proj: Mat4,
    radius: f32,
    paint_jobs: Vec<egui::epaint::ClippedPrimitive>,
    textures_delta: egui::TexturesDelta,
    screen_desc: egui_wgpu::ScreenDescriptor,
    _debug_arc: Option<(egui::Pos2, egui::Pos2)>,
) -> RenderOutputs {
    let gpu = &mut vp.gpu;

    let globals = Globals {
        view_proj: view_proj.to_cols_array_2d(),
        light_dir: [0.2, 0.7, 0.6],
        _pad: 0.0,
    };
    gpu.queue
        .write_buffer(&gpu.globals_buf, 0, bytemuck::bytes_of(&globals));

    let graph_rot = Mat4::from_quat(viewer.graph_rot);

    let mut instances = Vec::with_capacity(viewer.graph.nodes.len() + 1);
    for n in &viewer.graph.nodes {
        let world_pos = viewer.graph_rot * n.pos;
        let model = (Mat4::from_translation(world_pos)
            * graph_rot
            * Mat4::from_scale(glam::Vec3::splat(7.0)))
        .to_cols_array_2d();
        instances.push(Instance {
            model,
            color: [n.color[0], n.color[1], n.color[2], 1.0],
        });
    }
    let node_instance_count = instances.len() as u32;
    let sphere_instance_offset = instances.len() as u32;
    if viewer.debug_show_sphere {
        let model = (graph_rot * Mat4::from_scale(glam::Vec3::splat(radius))).to_cols_array_2d();
        instances.push(Instance {
            model,
            color: [1.0, 0.9, 0.2, 0.3],
        });
    }
    gpu.queue
        .write_buffer(&gpu.instance_buf, 0, bytemuck::cast_slice(&instances));

    let mut edge_instances = Vec::with_capacity(viewer.graph.edges.len() + 2);
    for (a, b) in &viewer.graph.edges {
        let sa = viewer.graph_rot * viewer.graph.nodes[*a].pos;
        let sb = viewer.graph_rot * viewer.graph.nodes[*b].pos;
        let color = viewer.graph.nodes[*a].color;
        let head = 7.0; // node radius to keep arrowheads off the sphere
        edge_instances.push(EdgeInstance {
            start: sa.to_array(),
            end: sb.to_array(),
            color,
            head,
        });
    }
    if viewer.debug_show_arc {
        if let (Some(a), Some(b)) = (viewer.arc_last_hit, viewer.arc_curr_hit) {
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
            gpu.resize(egui_winit::winit::dpi::PhysicalSize::new(
                gpu.config.width,
                gpu.config.height,
            ));
            match gpu.surface.get_current_texture() {
                Ok(f) => f,
                Err(_) => return RenderOutputs { frame_ms: 0.0 },
            }
        }
        Err(wgpu::SurfaceError::OutOfMemory) => {
            std::process::exit(1);
        }
        Err(_) => return RenderOutputs { frame_ms: 0.0 },
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
            label: Some("main"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: color_view,
                resolve_target: resolve_view,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
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
        let node_pipeline = if viewer.wireframe {
            &gpu.pipelines.node_wire
        } else {
            &gpu.pipelines.node
        };
        rpass.set_pipeline(node_pipeline);
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
        if viewer.debug_show_sphere {
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

    let egui_renderer = &mut vp.egui_renderer;
    let cmd_ui = {
        let mut egui_encoder = gpu
            .device
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
        }
        for id in textures_delta.free {
            egui_renderer.free_texture(&id);
        }

        egui_encoder.finish()
    };

    gpu.queue.submit([cmd_main, cmd_ui]);
    frame.present();

    let frame_ms = viewer.last_frame.elapsed().as_secs_f32() * 1000.0;
    RenderOutputs { frame_ms }
}
