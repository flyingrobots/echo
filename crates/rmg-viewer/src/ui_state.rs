// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Pure state transitions for the viewer UI (screens, overlays, menu actions).

use crate::core::{Screen, TitleMode, UiState, ViewerOverlay};

/// Advance UI based on a title-menu selection.
#[allow(dead_code)]
pub fn title_select_connect(ui: &mut UiState) {
    ui.title_mode = TitleMode::ConnectForm;
}

#[allow(dead_code)]
pub fn title_select_settings(ui: &mut UiState) {
    ui.title_mode = TitleMode::Settings;
}

#[allow(dead_code)]
pub fn title_back_to_menu(ui: &mut UiState) {
    ui.title_mode = TitleMode::Menu;
}

#[allow(dead_code)]
pub fn open_menu_overlay(ui: &mut UiState) {
    ui.overlay = ViewerOverlay::Menu;
}

#[allow(dead_code)]
pub fn close_overlay(ui: &mut UiState) {
    ui.overlay = ViewerOverlay::None;
}

#[allow(dead_code)]
pub fn open_settings_overlay(ui: &mut UiState) {
    ui.overlay = ViewerOverlay::Settings;
}

#[allow(dead_code)]
pub fn open_publish_overlay(ui: &mut UiState) {
    ui.overlay = ViewerOverlay::Publish;
}

#[allow(dead_code)]
pub fn open_subscribe_overlay(ui: &mut UiState) {
    ui.overlay = ViewerOverlay::Subscribe;
}

pub fn connecting_push(ui: &mut UiState, line: impl Into<String>) {
    ui.connect_log.push(line.into());
}

pub fn to_connecting(ui: &mut UiState) {
    ui.screen = Screen::Connecting;
    ui.title_mode = TitleMode::Menu;
}

#[allow(dead_code)]
pub fn to_error(ui: &mut UiState, msg: impl Into<String>) {
    ui.screen = Screen::Error(msg.into());
}

#[allow(dead_code)]
pub fn to_view(ui: &mut UiState) {
    ui.screen = Screen::View;
}
