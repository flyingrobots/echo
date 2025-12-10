// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Tool-facing session adapter: channels + port trait.
//!
//! This module provides a reusable `SessionPort` trait and a simple channel-based
//! implementation (`ChannelSession`) that wraps the receivers returned from
//! `connect_channels` / `connect_channels_for`. Tools (viewer, inspector, etc.)
//! can depend on this API without knowing about the underlying socket framing.

use echo_session_proto::{Notification, RmgFrame};
use std::sync::mpsc::Receiver;

/// Abstract port for receiving session events (RMG frames + notifications).
pub trait SessionPort {
    /// Drain up to `max` notifications from the underlying stream.
    fn drain_notifications(&mut self, max: usize) -> Vec<Notification>;
    /// Drain up to `max` RMG frames from the underlying stream.
    fn drain_frames(&mut self, max: usize) -> Vec<RmgFrame>;
    /// Clear any RMG streams (e.g., after desync) without closing notifications.
    fn clear_streams(&mut self);
}

/// Simple channel-backed session adapter for tools.
#[derive(Default)]
pub struct ChannelSession {
    notif_rx: Option<Receiver<Notification>>,
    rmg_rx: Option<Receiver<RmgFrame>>,
}

impl ChannelSession {
    /// Construct a new, empty session adapter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Install the underlying RMG/notification channels.
    pub fn set_channels(&mut self, rmg_rx: Receiver<RmgFrame>, notif_rx: Receiver<Notification>) {
        self.rmg_rx = Some(rmg_rx);
        self.notif_rx = Some(notif_rx);
    }
}

impl SessionPort for ChannelSession {
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
