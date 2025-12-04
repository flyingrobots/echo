// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Thin session adapter: holds notification/RMG channels and basic helpers.

use echo_graph::RmgFrame;
use echo_session_proto::Notification;
use std::sync::mpsc::Receiver;

use crate::ports::SessionPort;

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
}

impl SessionPort for SessionClient {
    fn drain_notifications(&mut self, max: usize) -> Vec<Notification> {
        let mut out = Vec::new();
        if let Some(rx) = &self.notif_rx {
            for _ in 0..max {
                match rx.try_recv() {
                    Ok(n) => out.push(n),
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
                }
            }
        }
        out
    }

    fn drain_frames(&mut self, max: usize) -> Vec<RmgFrame> {
        let mut out = Vec::new();
        if let Some(rx) = &self.rmg_rx {
            for _ in 0..max {
                match rx.try_recv() {
                    Ok(f) => out.push(f),
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
                }
            }
        }
        out
    }

    fn clear_streams(&mut self) {
        self.rmg_rx = None;
    }
}
