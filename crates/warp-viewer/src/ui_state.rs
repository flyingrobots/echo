// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Pure state transitions for the viewer UI (screens, overlays, menu actions).

use crate::core::{Screen, TitleMode, UiState, ViewerOverlay};
use echo_session_proto::default_socket_path;

#[derive(Debug, Clone)]
pub enum UiEvent {
    ConnectClicked,
    SettingsClicked,
    ExitClicked,
    ConnectHostChanged(String),
    ConnectPortChanged(u16),
    ConnectSubmit,
    SavePrefs,
    BackToTitle,
    OpenMenu,
    CloseOverlay,
    OpenSettingsOverlay,
    OpenPublishOverlay,
    OpenSubscribeOverlay,
    ShowError(String),
    ShutdownRequested,
    EnterView,
}

#[derive(Debug, Clone)]
pub enum UiEffect {
    SavePrefs,
    RequestConnect,
    QuitApp,
}

pub fn reduce(ui: &UiState, ev: UiEvent) -> (UiState, Vec<UiEffect>) {
    let mut next = ui.clone();
    let mut fx = Vec::new();
    match ev {
        UiEvent::ConnectClicked => {
            next.title_mode = TitleMode::ConnectForm;
        }
        UiEvent::SettingsClicked => {
            next.title_mode = TitleMode::Settings;
        }
        UiEvent::ExitClicked => {
            fx.push(UiEffect::QuitApp);
        }
        UiEvent::ConnectHostChanged(h) => next.connect_host = h,
        UiEvent::ConnectPortChanged(p) => next.connect_port = p,
        UiEvent::ConnectSubmit => {
            next.connect_log.clear();
            let target = if !next.connect_host.trim().is_empty() {
                if next.connect_host.starts_with('/') {
                    next.connect_host.clone()
                } else {
                    format!(
                        "{}:{} (runtime sock name)",
                        next.connect_host, next.connect_port
                    )
                }
            } else {
                default_socket_path().display().to_string()
            };
            next.connect_log
                .push(format!("Connecting to {target} (WARP {})...", next.warp_id));
            next.screen = Screen::Connecting;
            next.title_mode = TitleMode::Menu;
            fx.push(UiEffect::RequestConnect);
        }
        UiEvent::SavePrefs => {
            fx.push(UiEffect::SavePrefs);
            next.overlay = ViewerOverlay::None;
            next.title_mode = TitleMode::Menu;
        }
        UiEvent::BackToTitle => {
            next.screen = Screen::Title;
            next.title_mode = TitleMode::Menu;
            next.overlay = ViewerOverlay::None;
        }
        UiEvent::OpenMenu => next.overlay = ViewerOverlay::Menu,
        UiEvent::CloseOverlay => next.overlay = ViewerOverlay::None,
        UiEvent::OpenSettingsOverlay => next.overlay = ViewerOverlay::Settings,
        UiEvent::OpenPublishOverlay => next.overlay = ViewerOverlay::Publish,
        UiEvent::OpenSubscribeOverlay => next.overlay = ViewerOverlay::Subscribe,
        UiEvent::ShowError(msg) => {
            next.connect_log.push(format!("Connection error: {}", msg));
            next.screen = Screen::Error(msg);
        }
        UiEvent::ShutdownRequested => {
            // handled by App; no state change needed here
        }
        UiEvent::EnterView => next.screen = Screen::View,
    }
    (next, fx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ViewerOverlay;

    #[test]
    fn connect_flow_sets_connecting_and_effect() {
        let ui = UiState::new();
        let (ui2, fx) = reduce(&ui, UiEvent::ConnectSubmit);
        assert!(matches!(ui2.screen, Screen::Connecting));
        assert!(matches!(ui2.title_mode, TitleMode::Menu));
        assert!(fx.iter().any(|f| matches!(f, UiEffect::RequestConnect)));
    }

    #[test]
    fn overlays_open_and_close() {
        let ui = UiState::new();
        let (ui2, _) = reduce(&ui, UiEvent::OpenMenu);
        assert_eq!(ui2.overlay, ViewerOverlay::Menu);
        let (ui3, _) = reduce(&ui2, UiEvent::CloseOverlay);
        assert_eq!(ui3.overlay, ViewerOverlay::None);
    }

    #[test]
    fn save_prefs_effect() {
        let ui = UiState::new();
        let (ui2, fx) = reduce(&ui, UiEvent::SavePrefs);
        assert!(matches!(ui2.title_mode, TitleMode::Menu));
        assert!(fx.iter().any(|f| matches!(f, UiEffect::SavePrefs)));
    }
}
