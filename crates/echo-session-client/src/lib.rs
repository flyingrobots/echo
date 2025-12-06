// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Client helper for talking to the Echo session hub over Unix sockets
//! (CBOR-framed), plus tool-facing adapters (channels + ports).

use anyhow::Result;
use echo_session_proto::{
    wire::{decode_message, encode_message},
    HandshakePayload, Message, Notification, RmgFrame, RmgId,
};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream as AsyncUnixStream;

pub mod tool;

/// Minimal async client over Unix sockets.
pub struct SessionClient {
    stream: AsyncUnixStream,
}

impl SessionClient {
    /// Connect to the hub at the given Unix socket path.
    pub async fn connect(path: &str) -> Result<Self> {
        let stream = AsyncUnixStream::connect(path).await?;
        Ok(Self { stream })
    }

    /// Send a handshake message (JS-ABI v1.0).
    pub async fn send_handshake(&mut self, payload: HandshakePayload) -> Result<()> {
        let pkt = encode_message(Message::Handshake(payload), 0)?;
        self.stream.write_all(&pkt).await?;
        Ok(())
    }

    /// Subscribe to an RMG stream.
    pub async fn subscribe_rmg(&mut self, rmg_id: echo_session_proto::RmgId) -> Result<()> {
        let pkt = encode_message(Message::SubscribeRmg { rmg_id }, 0)?;
        self.stream.write_all(&pkt).await?;
        Ok(())
    }

    /// Poll a single message if available (non-blocking). Returns Ok(None) when no complete frame is present.
    pub async fn poll_message(&mut self) -> Result<Option<Message>> {
        let mut header = [0u8; 12];
        let n = self.stream.read(&mut header).await?;
        if n == 0 {
            return Ok(None);
        }
        if n < 12 {
            return Ok(None);
        }
        let len = u32::from_be_bytes([header[8], header[9], header[10], header[11]]) as usize;
        let mut rest = vec![0u8; len + 32];
        self.stream.read_exact(&mut rest).await?;
        let mut packet = Vec::with_capacity(12 + len + 32);
        packet.extend_from_slice(&header);
        packet.extend_from_slice(&rest);
        let (msg, _ts, _) = decode_message(&packet)?;
        Ok(Some(msg))
    }

    /// Convenience: drain messages until none are immediately available.
    pub async fn poll_notifications(&mut self) -> Result<Vec<Notification>> {
        let mut out = Vec::new();
        while let Some(msg) = self.poll_message().await? {
            if let Message::Notification(n) = msg {
                out.push(n);
            }
        }
        Ok(out)
    }

    /// Expose the underlying stream (e.g., for select!).
    pub fn stream(&mut self) -> &mut AsyncUnixStream {
        &mut self.stream
    }
}

/// Blocking helper: connect and stream frames/notifications on background threads.
/// Returns (RmgFrame receiver, Notification receiver). On connection failure, receivers stay empty.
pub fn connect_channels(path: &str) -> (Receiver<RmgFrame>, Receiver<Notification>) {
    let (rmg_tx, rmg_rx) = mpsc::channel();
    let (notif_tx, notif_rx) = mpsc::channel();
    let path = path.to_string();

    thread::spawn(move || {
        if let Ok(mut stream) = UnixStream::connect(path) {
            let _ = stream.write_all(
                &encode_message(
                    Message::Handshake(HandshakePayload {
                        client_version: 1,
                        capabilities: vec![],
                        agent_id: None,
                        session_meta: None,
                    }),
                    0,
                )
                .unwrap_or_default(),
            );
            let _ = stream.write_all(
                &encode_message(Message::SubscribeRmg { rmg_id: 1 }, 0).unwrap_or_default(),
            );
            loop {
                let mut header = [0u8; 12];
                if stream.read_exact(&mut header).is_err() {
                    break;
                }
                let len =
                    u32::from_be_bytes([header[8], header[9], header[10], header[11]]) as usize;
                let mut rest = vec![0u8; len + 32];
                if stream.read_exact(&mut rest).is_err() {
                    break;
                }
                let mut packet = Vec::with_capacity(12 + len + 32);
                packet.extend_from_slice(&header);
                packet.extend_from_slice(&rest);
                match decode_message(&packet) {
                    Ok((Message::RmgStream { frame, .. }, _, _)) => {
                        let _ = rmg_tx.send(frame);
                    }
                    Ok((Message::Notification(n), _, _)) => {
                        let _ = notif_tx.send(n);
                    }
                    _ => continue,
                }
            }
        }
    });

    (rmg_rx, notif_rx)
}

/// Connect, hello, and subscribe to a specific rmg_id; returns frame + notification receivers.
///
/// This performs the initial Unix socket connect synchronously so callers can
/// surface connection errors in their UI. After a successful connect, the
/// stream is moved into a background thread that handles handshake,
/// subscription, and message decoding.
pub fn connect_channels_for(
    path: &str,
    rmg_id: RmgId,
) -> std::io::Result<(Receiver<RmgFrame>, Receiver<Notification>)> {
    let (rmg_tx, rmg_rx) = mpsc::channel();
    let (notif_tx, notif_rx) = mpsc::channel();
    let path = path.to_string();

    // Try to connect synchronously so callers can handle errors immediately.
    let stream = UnixStream::connect(&path)?;

    thread::spawn(move || {
        let mut stream = stream;
        let _ = stream.write_all(
            &encode_message(
                Message::Handshake(HandshakePayload {
                    client_version: 1,
                    capabilities: vec![],
                    agent_id: None,
                    session_meta: None,
                }),
                0,
            )
            .unwrap_or_default(),
        );
        let _ = stream
            .write_all(&encode_message(Message::SubscribeRmg { rmg_id }, 0).unwrap_or_default());
        loop {
            let mut header = [0u8; 12];
            if stream.read_exact(&mut header).is_err() {
                break;
            }
            let len = u32::from_be_bytes([header[8], header[9], header[10], header[11]]) as usize;
            let mut rest = vec![0u8; len + 32];
            if stream.read_exact(&mut rest).is_err() {
                break;
            }
            let mut packet = Vec::with_capacity(12 + len + 32);
            packet.extend_from_slice(&header);
            packet.extend_from_slice(&rest);
            match decode_message(&packet) {
                Ok((Message::RmgStream { frame, .. }, _, _)) => {
                    let _ = rmg_tx.send(frame);
                }
                Ok((Message::Notification(n), _, _)) => {
                    let _ = notif_tx.send(n);
                }
                _ => continue,
            }
        }
    });

    Ok((rmg_rx, notif_rx))
}
