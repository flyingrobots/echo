// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Pure state transitions for the viewer UI (screens, overlays, menu actions).

use crate::core::{Screen, TitleMode, UiState, ViewerOverlay};

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
    EnterView,
}

#[derive(Debug, Clone)]
pub enum UiEffect {
    SavePrefs,
    RequestConnect { host: String, port: u16 },
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
            next.screen = Screen::Connecting;
            next.title_mode = TitleMode::Menu;
            fx.push(UiEffect::RequestConnect {
                host: next.connect_host.clone(),
                port: next.connect_port,
            });
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
        UiEvent::ShowError(msg) => next.screen = Screen::Error(msg),
        UiEvent::EnterView => next.screen = Screen::View,
    }
    (next, fx)
}

pub fn connecting_push(ui: &mut UiState, line: impl Into<String>) {
    ui.connect_log.push(line.into());
}
