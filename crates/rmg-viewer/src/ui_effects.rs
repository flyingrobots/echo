// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Effect runner for UiEffect -> concrete ports; includes a simple fake for tests.

use crate::ui_state::{UiEffect, UiEvent};
use crate::{core::UiState, session::SessionClient};
use echo_session_client::connect_channels_for;

pub trait UiEffectsRunner {
    /// Run effects, possibly emitting follow-up events (e.g., failures).
    fn run(
        &mut self,
        effects: Vec<UiEffect>,
        session: &mut SessionClient,
        ui_state: &UiState,
    ) -> Vec<UiEvent>;
}

pub struct RealEffectsRunner;

impl UiEffectsRunner for RealEffectsRunner {
    fn run(
        &mut self,
        effects: Vec<UiEffect>,
        session: &mut SessionClient,
        _ui_state: &UiState,
    ) -> Vec<UiEvent> {
        let followups = Vec::new();
        for eff in effects {
            match eff {
                UiEffect::SavePrefs => {
                    // handled upstream in App::apply_ui_event (config port)
                }
                UiEffect::RequestConnect { host, port } => {
                    let path = format!("{host}:{port}");
                    let (rmg_rx, notif_rx) = connect_channels_for(&path, _ui_state.rmg_id);
                    session.set_channels(rmg_rx, notif_rx);
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
        _session: &mut SessionClient,
        _ui_state: &UiState,
    ) -> Vec<UiEvent> {
        let mut followups = Vec::new();
        for eff in effects {
            match &eff {
                UiEffect::RequestConnect { .. } if self.fail_connect => {
                    followups.push(UiEvent::ShowError("Connect failed".into()));
                }
                _ => {}
            }
            self.calls.push(eff);
        }
        followups
    }
}
