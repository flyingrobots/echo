// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Client helper for talking to the Echo session hub over Unix sockets (CBOR-framed).

use anyhow::Result;
use echo_session_proto::{ClientKind, Message, Notification, RmgFrame, RmgId};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream as AsyncUnixStream;

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

    /// Send a hello message (client kind + protocol version).
    pub async fn send_hello(&mut self, kind: ClientKind, version: u16) -> Result<()> {
        let pkt = echo_session_proto::wire::Packet::encode(&Message::Hello {
            protocol_version: version,
            client_kind: kind,
        })?;
        self.stream.write_all(&pkt).await?;
        Ok(())
    }

    /// Subscribe to an RMG stream.
    pub async fn subscribe_rmg(&mut self, rmg_id: echo_session_proto::RmgId) -> Result<()> {
        let pkt = echo_session_proto::wire::Packet::encode(&Message::SubscribeRmg { rmg_id })?;
        self.stream.write_all(&pkt).await?;
        Ok(())
    }

    /// Poll a single message if available (non-blocking). Returns Ok(None) when no complete frame is present.
    pub async fn poll_message(&mut self) -> Result<Option<Message>> {
        let mut len_buf = [0u8; 4];
        let n = self.stream.read(&mut len_buf).await?;
        if n == 0 {
            return Ok(None);
        }
        if n < 4 {
            // simplistic handling: treat incomplete header as no message
            return Ok(None);
        }
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut body = vec![0u8; len];
        self.stream.read_exact(&mut body).await?;
        let msg = echo_session_proto::wire::from_cbor(&body)?;
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
            // Send a hello and subscribe to default rmg_id 1 for simple consumers.
            let _ = stream.write_all(
                &echo_session_proto::wire::Packet::encode(&Message::Hello {
                    protocol_version: 1,
                    client_kind: ClientKind::Viewer,
                })
                .unwrap_or_default(),
            );
            // Default subscribe to RMG 1
            let _ = stream.write_all(
                &echo_session_proto::wire::Packet::encode(&Message::SubscribeRmg { rmg_id: 1 })
                    .unwrap_or_default(),
            );
            loop {
                let mut len_buf = [0u8; 4];
                if stream.read_exact(&mut len_buf).is_err() {
                    break;
                }
                let len = u32::from_be_bytes(len_buf) as usize;
                let mut body = vec![0u8; len];
                if stream.read_exact(&mut body).is_err() {
                    break;
                }
                match echo_session_proto::wire::from_cbor(&body) {
                    Ok(Message::RmgStream { frame, .. }) => {
                        let _ = rmg_tx.send(frame);
                    }
                    Ok(Message::Notification(n)) => {
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
pub fn connect_channels_for(
    path: &str,
    rmg_id: RmgId,
) -> (Receiver<RmgFrame>, Receiver<Notification>) {
    let (rmg_tx, rmg_rx) = mpsc::channel();
    let (notif_tx, notif_rx) = mpsc::channel();
    let path = path.to_string();

    thread::spawn(move || {
        if let Ok(mut stream) = UnixStream::connect(path) {
            let _ = stream.write_all(
                &echo_session_proto::wire::Packet::encode(&Message::Hello {
                    protocol_version: 1,
                    client_kind: ClientKind::Viewer,
                })
                .unwrap_or_default(),
            );
            let _ = stream.write_all(
                &echo_session_proto::wire::Packet::encode(&Message::SubscribeRmg { rmg_id })
                    .unwrap_or_default(),
            );
            loop {
                let mut len_buf = [0u8; 4];
                if stream.read_exact(&mut len_buf).is_err() {
                    break;
                }
                let len = u32::from_be_bytes(len_buf) as usize;
                let mut body = vec![0u8; len];
                if stream.read_exact(&mut body).is_err() {
                    break;
                }
                match echo_session_proto::wire::from_cbor(&body) {
                    Ok(Message::RmgStream { frame, .. }) => {
                        let _ = rmg_tx.send(frame);
                    }
                    Ok(Message::Notification(n)) => {
                        let _ = notif_tx.send(n);
                    }
                    _ => continue,
                }
            }
        }
    });

    (rmg_rx, notif_rx)
}
