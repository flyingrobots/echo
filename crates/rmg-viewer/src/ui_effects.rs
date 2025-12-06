// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Effect runner for UiEffect -> concrete ports; includes a simple fake for tests.

use crate::core::UiState;
use crate::ui_state::{UiEffect, UiEvent};
use echo_session_client::connect_channels_for;
use echo_session_proto::DEFAULT_SOCKET_PATH;

pub trait UiEffectsRunner {
    /// Run effects, possibly emitting follow-up events (e.g., failures).
    fn run(
        &mut self,
        effects: Vec<UiEffect>,
        session: &mut echo_session_client::tool::ChannelSession,
        ui_state: &UiState,
    ) -> Vec<UiEvent>;
}

pub struct RealEffectsRunner;

impl UiEffectsRunner for RealEffectsRunner {
    fn run(
        &mut self,
        effects: Vec<UiEffect>,
        session: &mut echo_session_client::tool::ChannelSession,
        ui_state: &UiState,
    ) -> Vec<UiEvent> {
        let mut followups = Vec::new();
        for eff in effects {
            match eff {
                UiEffect::SavePrefs => {
                    // handled upstream in App::apply_ui_event (config port)
                }
                UiEffect::RequestConnect => {
                    // For now, connect to the default local Unix socket path.
                    // Host/port fields are kept in UiState for future TCP support.
                    match connect_channels_for(DEFAULT_SOCKET_PATH, ui_state.rmg_id) {
                        Ok((rmg_rx, notif_rx)) => {
                            session.set_channels(rmg_rx, notif_rx);
                        }
                        Err(err) => {
                            followups.push(UiEvent::ShowError(format!("Connect failed: {err}")));
                        }
                    }
                }
                UiEffect::QuitApp => {
                    std::process::exit(0);
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
