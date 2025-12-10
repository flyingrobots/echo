// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Client helper for talking to the Echo session hub over Unix sockets
//! (CBOR-framed), plus tool-facing adapters (channels + ports).

use anyhow::{anyhow, Result};
use echo_session_proto::{
    wire::{decode_message, encode_message},
    HandshakePayload, Message, Notification, RmgFrame, RmgId,
};
use std::io::{self, ErrorKind, Read, Write};
use std::os::unix::net::UnixStream;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream as AsyncUnixStream;

pub mod tool;

/// Minimal async client over Unix sockets.
pub struct SessionClient {
    stream: AsyncUnixStream,
    buffer: Vec<u8>,
}

const MAX_PAYLOAD: usize = 8 * 1024 * 1024; // 8 MiB cap for frames

impl SessionClient {
    /// Connect to the hub at the given Unix socket path.
    pub async fn connect(path: &str) -> Result<Self> {
        let stream = AsyncUnixStream::connect(path).await?;
        Ok(Self {
            stream,
            buffer: Vec::with_capacity(4096),
        })
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

    /// Poll a single message if already available (non-blocking). Returns Ok(None) when no complete frame is buffered yet or on clean EOF.
    /// Buffers across calls so partial reads never drop bytes.
    pub async fn poll_message(&mut self) -> Result<Option<Message>> {
        const MAX_PAYLOAD: usize = 8 * 1024 * 1024; // 8 MiB cap

        loop {
            if self.buffer.len() >= 12 {
                let len = u32::from_be_bytes([
                    self.buffer[8],
                    self.buffer[9],
                    self.buffer[10],
                    self.buffer[11],
                ]) as usize;
                if len > MAX_PAYLOAD {
                    return Err(anyhow!("frame payload too large: {} bytes", len));
                }
                let frame_len = 12usize
                    .checked_add(len)
                    .and_then(|v| v.checked_add(32))
                    .ok_or_else(|| anyhow!("frame length overflow"))?;

                if self.buffer.len() >= frame_len {
                    let packet: Vec<u8> = self.buffer.drain(..frame_len).collect();
                    let (msg, _ts, _) = decode_message(&packet)?;
                    return Ok(Some(msg));
                }
            }

            let mut chunk = [0u8; 2048];
            match self.stream.try_read(&mut chunk) {
                Ok(0) => {
                    if self.buffer.is_empty() {
                        return Ok(None);
                    }
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        format!(
                            "truncated frame: have {} buffered bytes (need at least header)",
                            self.buffer.len()
                        ),
                    )
                    .into());
                }
                Ok(n) => {
                    self.buffer.extend_from_slice(&chunk[..n]);
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => return Ok(None),
                Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => return Err(e.into()),
            }
        }
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
                if len > MAX_PAYLOAD {
                    break;
                }
                let frame_len = 12usize
                    .checked_add(len)
                    .and_then(|v| v.checked_add(32))
                    .unwrap_or(usize::MAX);
                if frame_len == usize::MAX {
                    break;
                }
                let mut rest = vec![0u8; len + 32];
                if stream.read_exact(&mut rest).is_err() {
                    break;
                }
                let mut packet = Vec::with_capacity(frame_len);
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
            if len > MAX_PAYLOAD {
                break;
            }
            let frame_len = 12usize
                .checked_add(len)
                .and_then(|v| v.checked_add(32))
                .unwrap_or(usize::MAX);
            if frame_len == usize::MAX {
                break;
            }
            let mut rest = vec![0u8; len + 32];
            if stream.read_exact(&mut rest).is_err() {
                break;
            }
            let mut packet = Vec::with_capacity(frame_len);
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

#[cfg(test)]
mod tests {
    use super::*;
    use echo_session_proto::{NotifyKind, NotifyScope};
    use tokio::io::AsyncWriteExt;
    use tokio::task;

    #[tokio::test]
    async fn poll_message_handles_partial_header_without_losing_bytes() {
        let (client_stream, mut server_stream) = tokio::net::UnixStream::pair().unwrap();

        let notification = Notification {
            kind: NotifyKind::Info,
            scope: NotifyScope::Global,
            title: "partial-header".to_string(),
            body: Some("keep frame aligned".to_string()),
        };

        let encoded = encode_message(Message::Notification(notification.clone()), 42).unwrap();

        server_stream.write_all(&encoded[..5]).await.unwrap();
        task::yield_now().await;
        server_stream.write_all(&encoded[5..]).await.unwrap();
        drop(server_stream);

        let mut client = SessionClient {
            stream: client_stream,
            buffer: Vec::new(),
        };

        let mut received = None;
        for _ in 0..50 {
            if let Some(msg) = client.poll_message().await.unwrap() {
                received = Some(msg);
                break;
            }
            task::yield_now().await;
        }

        match received.expect("message not received") {
            Message::Notification(n) => assert_eq!(n, notification),
            other => panic!("expected notification, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn poll_message_errors_on_eof_mid_header() {
        let (client_stream, mut server_stream) = tokio::net::UnixStream::pair().unwrap();

        // Send only part of the header then close.
        let partial = vec![0u8; 5];
        server_stream.write_all(&partial).await.unwrap();
        drop(server_stream);

        let mut client = SessionClient {
            stream: client_stream,
            buffer: Vec::new(),
        };

        let mut got_err = false;
        for _ in 0..10 {
            match client.poll_message().await {
                Ok(Some(_)) => continue,
                Ok(None) => {
                    task::yield_now().await;
                    continue;
                }
                Err(e) => {
                    assert!(e.to_string().contains("truncated frame"));
                    got_err = true;
                    break;
                }
            }
        }
        assert!(got_err, "expected truncated frame error");
    }

    #[tokio::test]
    async fn poll_message_errors_on_eof_mid_body() {
        let (client_stream, mut server_stream) = tokio::net::UnixStream::pair().unwrap();

        let notification = Notification {
            kind: NotifyKind::Warn,
            scope: NotifyScope::Global,
            title: "mid-body".to_string(),
            body: None,
        };
        let encoded = encode_message(Message::Notification(notification), 0).unwrap();

        let header_len = 12;
        let cut = header_len + 5; // send header plus a few payload bytes
        server_stream.write_all(&encoded[..cut]).await.unwrap();
        drop(server_stream);

        let mut client = SessionClient {
            stream: client_stream,
            buffer: Vec::new(),
        };

        let mut got_err = false;
        for _ in 0..10 {
            match client.poll_message().await {
                Ok(Some(_)) => continue,
                Ok(None) => {
                    task::yield_now().await;
                    continue;
                }
                Err(e) => {
                    assert!(e.to_string().contains("truncated frame"));
                    got_err = true;
                    break;
                }
            }
        }
        assert!(got_err, "expected truncated frame error");
    }

    #[tokio::test]
    async fn poll_message_handles_back_to_back_frames() {
        let (client_stream, mut server_stream) = tokio::net::UnixStream::pair().unwrap();

        let n1 = Notification {
            kind: NotifyKind::Info,
            scope: NotifyScope::Global,
            title: "first".to_string(),
            body: None,
        };
        let n2 = Notification {
            kind: NotifyKind::Error,
            scope: NotifyScope::Global,
            title: "second".to_string(),
            body: Some("payload".into()),
        };
        let encoded = [
            encode_message(Message::Notification(n1.clone()), 1).unwrap(),
            encode_message(Message::Notification(n2.clone()), 2).unwrap(),
        ]
        .concat();

        server_stream.write_all(&encoded).await.unwrap();

        let mut client = SessionClient {
            stream: client_stream,
            buffer: Vec::new(),
        };

        let mut got = Vec::new();
        for _ in 0..10 {
            if let Some(msg) = client.poll_message().await.unwrap() {
                got.push(msg);
                if got.len() == 2 {
                    break;
                }
            } else {
                task::yield_now().await;
            }
        }

        assert_eq!(got.len(), 2);
        assert!(matches!(got[0], Message::Notification(ref n) if *n == n1));
        assert!(matches!(got[1], Message::Notification(ref n) if *n == n2));
    }
}
