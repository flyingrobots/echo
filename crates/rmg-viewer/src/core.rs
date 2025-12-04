// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Domain-level UI state for the RMG viewer (screens/overlays/connect settings).

#[derive(Clone, Debug)]
pub enum Screen {
    Title,
    Connecting,
    View,
    Error(String),
}

#[derive(Clone, Debug)]
pub enum TitleMode {
    Menu,
    ConnectForm,
    Settings,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ViewerOverlay {
    None,
    Menu,
    Settings,
    Publish,
    Subscribe,
}

#[derive(Clone, Debug)]
pub struct UiState {
    pub screen: Screen,
    pub title_mode: TitleMode,
    pub overlay: ViewerOverlay,
    pub connect_host: String,
    pub connect_port: u16,
    pub rmg_id: u64,
    pub connect_log: Vec<String>,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            screen: Screen::Title,
            title_mode: TitleMode::Menu,
            overlay: ViewerOverlay::None,
            connect_host: "localhost".into(),
            connect_port: 9000,
            rmg_id: 1,
            connect_log: Vec::new(),
        }
    }
}
