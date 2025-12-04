// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Thin session adapter: holds notification/RMG channels and basic helpers.

use echo_graph::RmgFrame;
use echo_session_proto::Notification;
use std::sync::mpsc::Receiver;

#[derive(Default)]
pub struct SessionClient {
    notif_rx: Option<Receiver<Notification>>,
    rmg_rx: Option<Receiver<RmgFrame>>,
}

impl SessionClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_channels(&mut self, rmg_rx: Receiver<RmgFrame>, notif_rx: Receiver<Notification>) {
        self.rmg_rx = Some(rmg_rx);
        self.notif_rx = Some(notif_rx);
    }

    pub fn rmg_rx(&self) -> Option<&Receiver<RmgFrame>> {
        self.rmg_rx.as_ref()
    }

    pub fn notif_rx(&self) -> Option<&Receiver<Notification>> {
        self.notif_rx.as_ref()
    }

    pub fn clear_streams(&mut self) {
        self.rmg_rx = None;
    }
}
