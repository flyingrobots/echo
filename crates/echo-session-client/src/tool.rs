// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Tool-facing session adapter: channels + port trait.
//!
//! This module provides a reusable `SessionPort` trait and a simple channel-based
//! implementation (`ChannelSession`) that wraps the receivers returned from
//! `connect_channels` / `connect_channels_for`. Tools (viewer, inspector, etc.)
//! can depend on this API without knowing about the underlying socket framing.

use anyhow::{anyhow, Result};
use echo_session_proto::{Message, Notification, WarpFrame, WarpId};
use std::sync::mpsc::Receiver;

/// Abstract port for receiving session events (WARP frames + notifications).
pub trait SessionPort {
    /// Drain up to `max` notifications from the underlying stream.
    fn drain_notifications(&mut self, max: usize) -> Vec<Notification>;
    /// Drain up to `max` WARP frames from the underlying stream.
    fn drain_frames(&mut self, max: usize) -> Vec<WarpFrame>;
    /// Clear any WARP streams (e.g., after desync) without closing notifications.
    fn clear_streams(&mut self);
}

/// Simple channel-backed session adapter for tools.
#[derive(Default)]
pub struct ChannelSession {
    notif_rx: Option<Receiver<Notification>>,
    warp_rx: Option<Receiver<WarpFrame>>,
    send_tx: Option<std::sync::mpsc::Sender<Message>>,
}

impl ChannelSession {
    /// Construct a new, empty session adapter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Install the underlying WARP/notification channels.
    pub fn set_channels(&mut self, warp_rx: Receiver<WarpFrame>, notif_rx: Receiver<Notification>) {
        self.warp_rx = Some(warp_rx);
        self.notif_rx = Some(notif_rx);
    }

    /// Install an outbound message channel for publishing.
    pub fn set_sender(&mut self, send_tx: std::sync::mpsc::Sender<Message>) {
        self.send_tx = Some(send_tx);
    }

    /// Install both inbound receivers and an outbound sender.
    pub fn set_link(
        &mut self,
        warp_rx: Receiver<WarpFrame>,
        notif_rx: Receiver<Notification>,
        send_tx: std::sync::mpsc::Sender<Message>,
    ) {
        self.set_channels(warp_rx, notif_rx);
        self.set_sender(send_tx);
    }

    /// Send an outbound session message (best-effort).
    pub fn send(&self, msg: Message) -> Result<()> {
        let tx = self
            .send_tx
            .as_ref()
            .ok_or_else(|| anyhow!("session is not connected (no sender installed)"))?;
        tx.send(msg)
            .map_err(|_| anyhow!("session send failed (connection dropped)"))?;
        Ok(())
    }

    /// Convenience: subscribe to a WARP stream on an existing connection.
    pub fn subscribe_warp(&self, warp_id: WarpId) -> Result<()> {
        self.send(Message::SubscribeWarp { warp_id })
    }

    /// Convenience: publish a WARP frame on an existing connection.
    pub fn publish_warp_frame(&self, warp_id: WarpId, frame: WarpFrame) -> Result<()> {
        self.send(Message::WarpStream { warp_id, frame })
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

    fn drain_frames(&mut self, max: usize) -> Vec<WarpFrame> {
        let mut out = Vec::new();
        if let Some(rx) = &self.warp_rx {
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
        self.warp_rx = None;
    }
}
