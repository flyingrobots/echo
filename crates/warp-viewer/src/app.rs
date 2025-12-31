// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Top-level application wiring and event loop handler.

use crate::{
    core::UiState,
    render_port::WinitRenderPort,
    scene::{encode_viz_payload, sample_wire_graph, scene_from_wire},
    ui_effects::{self, UiEffectsRunner},
    ui_state,
    viewer_state::ViewerState,
    viewport::Viewport,
};
use blake3::Hasher;
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
    pub wvp: WvpState,
    shutdown_requested: bool,
}

/// Minimal state for exercising the WARP View Protocol from the viewer.
#[derive(Debug, Clone)]
pub struct WvpState {
    /// When true, publish local mutations as `warp_stream` frames.
    pub publish_enabled: bool,
    /// When true, apply inbound `warp_stream` frames to the local viewer state.
    pub receive_enabled: bool,
    /// Pending ops to publish in the next diff frame.
    pub pending_ops: Vec<echo_graph::WarpOp>,
    /// Next epoch we will publish as `from_epoch`.
    pub publish_epoch: u64,
    /// Whether we've published at least one snapshot on this connection.
    pub snapshot_published: bool,
    /// Local deterministic mutation counter (demo driver).
    pub pulse: u64,
}

impl Default for WvpState {
    fn default() -> Self {
        Self {
            publish_enabled: false,
            receive_enabled: true,
            pending_ops: Vec::new(),
            publish_epoch: 0,
            snapshot_published: false,
            pulse: 0,
        }
    }
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
            wvp: WvpState::default(),
            shutdown_requested: false,
        }
    }

    pub fn apply_ui_event(&mut self, ev: ui_state::UiEvent) {
        let ev_clone = ev.clone();
        let (next, effects) = ui_state::reduce(&self.ui, ev);
        self.ui = next;
        let followups = self.ui_runner.run(
            effects,
            &mut self.session,
            &self.ui,
            self.config.as_deref(),
            &self.viewer,
        );
        if matches!(ev_clone, ui_state::UiEvent::EnterView) {
            // A new connection requires a snapshot before any diffs. If we were already
            // publishing prior to reconnect, reset the per-connection flag so publishing
            // can resume without requiring a manual toggle.
            self.wvp.snapshot_published = false;
        }
        if matches!(ev_clone, ui_state::UiEvent::ShutdownRequested) {
            self.shutdown_requested = true;
        }
        for ev in followups {
            self.apply_ui_event(ev);
        }
    }

    /// Enable or disable publishing local mutations to the session hub.
    ///
    /// When enabling from a disabled state, clears `snapshot_published` so the next
    /// publish sends a snapshot before any diffs.
    pub fn set_publish_enabled(&mut self, enabled: bool) {
        if enabled && !self.wvp.publish_enabled {
            // First time enabling: ensure the next publish is a snapshot so we
            // satisfy the hub's "snapshot before first diff" rule.
            self.wvp.snapshot_published = false;
        }
        self.wvp.publish_enabled = enabled;
    }

    /// Enable or disable applying inbound WARP frames to the local viewer state.
    ///
    /// When enabling from a disabled state, resubscribes to the current `warp_id`
    /// to fetch a fresh snapshot. On success, clears `viewer.epoch`; on failure,
    /// pushes a warning toast.
    pub fn set_receive_enabled(&mut self, enabled: bool) {
        if enabled && !self.wvp.receive_enabled {
            // Re-request the latest snapshot so we can resume a gapless stream.
            // This is a v0 "resync" mechanism until we add an explicit resync op.
            if let Err(err) = self.session.subscribe_warp(self.ui.warp_id) {
                self.toasts.push(
                    ToastKind::Warn,
                    ToastScope::Local,
                    "Resubscribe failed",
                    Some(format!("{err:#}")),
                    std::time::Duration::from_secs(6),
                    Instant::now(),
                );
            } else {
                self.viewer.epoch = None;
            }
        }
        self.wvp.receive_enabled = enabled;
    }

    /// Force the next publish to be a snapshot rather than a diff.
    pub fn request_publish_snapshot(&mut self) {
        self.wvp.snapshot_published = false;
    }

    /// Perform a deterministic mutation on the first graph node for demo purposes.
    ///
    /// Each call increments an internal pulse counter and derives new position/color
    /// values from a hash of `(node_id, pulse)`. The mutation is applied locally and
    /// queued as a pending op for the next diff frame.
    pub fn pulse_local_graph(&mut self) {
        if self.viewer.wire_graph.nodes.is_empty() {
            self.toasts.push(
                ToastKind::Warn,
                ToastScope::Local,
                "Cannot mutate graph",
                Some("No nodes exist in the current graph".into()),
                std::time::Duration::from_secs(4),
                Instant::now(),
            );
            return;
        }

        self.wvp.pulse = self.wvp.pulse.saturating_add(1);
        let pulse = self.wvp.pulse;

        // Deterministic mutation: update the first node's payload with a CBOR
        // `{pos, color}` map understood by `scene_from_wire`.
        let id = self.viewer.wire_graph.nodes[0].id;
        let mut h = Hasher::new();
        h.update(&id.to_le_bytes());
        h.update(&pulse.to_le_bytes());
        let bytes = h.finalize();
        let b = bytes.as_bytes();

        let u32_at =
            |i: usize| -> u32 { u32::from_le_bytes(b[i..i + 4].try_into().expect("slice")) };

        let unit = |v: u32| (v as f32) / (u32::MAX as f32);
        let span = |v: u32, radius: f32| (unit(v) - 0.5) * 2.0 * radius;

        let pos = [
            span(u32_at(0), 260.0),
            span(u32_at(4), 180.0),
            span(u32_at(8), 260.0),
        ];
        let color = [unit(u32_at(12)), unit(u32_at(16)), unit(u32_at(20))];
        let raw = encode_viz_payload(pos, color);

        let op = echo_graph::WarpOp::UpdateNode {
            id,
            data: echo_graph::NodeDataPatch::Replace(echo_graph::NodeData { raw }),
        };

        if let Err(err) = self.viewer.wire_graph.apply_op(op.clone()) {
            self.toasts.push(
                ToastKind::Error,
                ToastScope::Local,
                "Local mutation failed",
                Some(format!("{err:#}")),
                std::time::Duration::from_secs(6),
                Instant::now(),
            );
            return;
        }

        self.wvp.pending_ops.push(op);
        self.viewer.graph = scene_from_wire(&self.viewer.wire_graph);
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
                    .with_title("Echo WARP Viewer 3D")
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

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.viewports.is_empty() {
            return;
        }
        if self.shutdown_requested {
            if let Some(cfg) = &self.config {
                cfg.save_prefs(&self.viewer.export_prefs());
            }
            // Let the app exit gracefully; dropping will clean up GPU/session.
            event_loop.exit();
            return;
        }
        self.frame();
    }
}
