// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Effect runner for UiEffect -> concrete ports; includes a simple fake for tests.

use crate::core::UiState;
use crate::ui_state::{UiEffect, UiEvent};
use crate::viewer_state::ViewerState;
use echo_app_core::config_port::ConfigPort;
use echo_session_client::connect_channels_for;
use echo_session_proto::default_socket_path;
use std::sync::mpsc;
use std::time::Duration;

fn resolve_socket_path(host: &str, port: u16) -> String {
    if host.trim().is_empty() {
        return default_socket_path().display().to_string();
    }
    if host.starts_with('/') {
        return host.to_string();
    }
    let mut base = default_socket_path();
    if let Some(parent) = base.parent() {
        let fname = format!("echo-session-{}-{}.sock", host, port);
        base = parent.join(fname);
    }
    base.display().to_string()
}

pub trait UiEffectsRunner {
    /// Run effects, possibly emitting follow-up events (e.g., failures).
    fn run(
        &mut self,
        effects: Vec<UiEffect>,
        session: &mut echo_session_client::tool::ChannelSession,
        ui_state: &UiState,
        config: Option<&dyn ConfigPort>,
        viewer: &ViewerState,
    ) -> Vec<UiEvent>;
}

pub struct RealEffectsRunner;

impl UiEffectsRunner for RealEffectsRunner {
    fn run(
        &mut self,
        effects: Vec<UiEffect>,
        session: &mut echo_session_client::tool::ChannelSession,
        ui_state: &UiState,
        config: Option<&dyn ConfigPort>,
        viewer: &ViewerState,
    ) -> Vec<UiEvent> {
        let mut followups = Vec::new();
        for eff in effects {
            match eff {
                UiEffect::SavePrefs => {
                    if let Some(cfg) = config {
                        cfg.save_prefs(&viewer.export_prefs());
                    }
                }
                UiEffect::RequestConnect => {
                    // For now, connect to the default local Unix socket path.
                    // If the user entered an absolute path in host, honor it; otherwise use the per-user default.
                    let (tx, rx) = mpsc::channel();
                    let rmg_id = ui_state.rmg_id;
                    let path = resolve_socket_path(&ui_state.connect_host, ui_state.connect_port);
                    std::thread::spawn(move || {
                        let res = connect_channels_for(&path, rmg_id).map_err(|e| e.to_string());
                        let _ = tx.send(res);
                    });

                    match rx.recv_timeout(Duration::from_secs(1)) {
                        Ok(Ok((rmg_rx, notif_rx))) => {
                            session.set_channels(rmg_rx, notif_rx);
                        }
                        Ok(Err(err)) => {
                            followups.push(UiEvent::ShowError(format!("Connect failed: {err}")));
                        }
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            followups.push(UiEvent::ShowError("Connect timed out".into()));
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => {
                            followups
                                .push(UiEvent::ShowError("Connect failed (disconnected)".into()));
                        }
                    }
                }
                UiEffect::QuitApp => {
                    followups.push(UiEvent::ShutdownRequested);
                }
            }
        }
        followups
    }
}

/// Test fake: records effects and lets tests inject failures.
#[derive(Default)]
#[allow(dead_code)]
pub struct FakeEffectsRunner {
    pub calls: Vec<UiEffect>,
    pub fail_connect: bool,
}

impl UiEffectsRunner for FakeEffectsRunner {
    fn run(
        &mut self,
        effects: Vec<UiEffect>,
        _session: &mut echo_session_client::tool::ChannelSession,
        _ui_state: &UiState,
        _config: Option<&dyn ConfigPort>,
        _viewer: &ViewerState,
    ) -> Vec<UiEvent> {
        let mut followups = Vec::new();
        for eff in effects {
            match &eff {
                UiEffect::RequestConnect if self.fail_connect => {
                    followups.push(UiEvent::ShowError("Connect failed".into()));
                }
                _ => {}
            }
            self.calls.push(eff);
        }
        followups
    }
}
