// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Client helper for talking to the Echo session hub over Unix sockets
//! (CBOR-framed), plus tool-facing adapters (channels + ports).

use anyhow::{anyhow, Result};
use echo_session_proto::{
    wire::{decode_message, encode_message},
    HandshakePayload, Message, Notification, NotifyKind, NotifyScope, WarpFrame, WarpId,
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
/// Error codes below this threshold are treated as client-side decode/wire failures (Local scope).
/// Codes at or above this threshold are treated as session/service protocol errors (Global scope).
const ERROR_CODE_GLOBAL_THRESHOLD: u32 = 400;

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

    /// Subscribe to a WARP stream.
    pub async fn subscribe_warp(&mut self, warp_id: echo_session_proto::WarpId) -> Result<()> {
        let pkt = encode_message(Message::SubscribeWarp { warp_id }, 0)?;
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
/// Returns (WarpFrame receiver, Notification receiver). On connection failure, receivers stay empty.
pub fn connect_channels(path: &str) -> (Receiver<WarpFrame>, Receiver<Notification>) {
    let (warp_tx, warp_rx) = mpsc::channel();
    let (notif_tx, notif_rx) = mpsc::channel();
    let path = path.to_string();

    thread::spawn(move || {
        if let Ok(stream) = UnixStream::connect(path) {
            run_message_loop(stream, 1, warp_tx, notif_tx);
        }
    });

    (warp_rx, notif_rx)
}

/// Connect, hello, and subscribe to a specific warp_id; returns frame + notification receivers.
///
/// This performs the initial Unix socket connect synchronously so callers can
/// surface connection errors in their UI. After a successful connect, the
/// stream is moved into a background thread that handles handshake,
/// subscription, and message decoding.
pub fn connect_channels_for(
    path: &str,
    warp_id: WarpId,
) -> std::io::Result<(Receiver<WarpFrame>, Receiver<Notification>)> {
    let (warp_tx, warp_rx) = mpsc::channel();
    let (notif_tx, notif_rx) = mpsc::channel();
    let path = path.to_string();

    // Try to connect synchronously so callers can handle errors immediately.
    let stream = UnixStream::connect(&path)?;

    thread::spawn(move || {
        run_message_loop(stream, warp_id, warp_tx, notif_tx);
    });

    Ok((warp_rx, notif_rx))
}

/// Connect, hello, and provide a bidirectional channel for publishing session messages.
///
/// This is a tool-friendly adapter: it connects synchronously (so UIs can report
/// errors immediately), then spawns background threads:
/// - a reader loop that decodes inbound packets into `WarpFrame` + `Notification` receivers,
/// - a writer loop that accepts outbound `Message` values over a `Sender<Message>`.
///
/// The writer loop sends an initial handshake and (optionally) an initial
/// `SubscribeWarp` before it begins draining outbound messages.
pub fn connect_channels_for_bidir(
    path: &str,
    warp_id: WarpId,
) -> std::io::Result<(
    mpsc::Sender<Message>,
    Receiver<WarpFrame>,
    Receiver<Notification>,
)> {
    let (warp_tx, warp_rx) = mpsc::channel();
    let (notif_tx, notif_rx) = mpsc::channel();
    let (out_tx, out_rx) = mpsc::channel();

    // Try to connect synchronously so callers can handle errors immediately.
    let stream = UnixStream::connect(path)?;
    let reader = stream.try_clone()?;
    let writer = stream;

    thread::spawn(move || {
        run_read_loop(reader, warp_tx, notif_tx);
    });

    thread::spawn(move || {
        run_write_loop(writer, Some(warp_id), out_rx);
    });

    Ok((out_tx, warp_rx, notif_rx))
}

fn run_message_loop(
    mut stream: UnixStream,
    warp_id: WarpId,
    warp_tx: mpsc::Sender<WarpFrame>,
    notif_tx: mpsc::Sender<Notification>,
) {
    if !write_handshake_and_subscribe(&mut stream, Some(warp_id)) {
        return;
    }
    run_read_loop(stream, warp_tx, notif_tx);
}

fn write_handshake_and_subscribe(stream: &mut UnixStream, warp_id: Option<WarpId>) -> bool {
    let msg = Message::Handshake(HandshakePayload {
        client_version: 1,
        capabilities: vec![],
        agent_id: None,
        session_meta: None,
    });
    let pkt = match encode_message(msg, 0) {
        Ok(pkt) => pkt,
        Err(err) => {
            tracing::warn!(error = %err, "failed to encode handshake message");
            return false;
        }
    };
    if let Err(err) = stream.write_all(&pkt) {
        tracing::warn!(error = %err, "failed to write handshake message");
        return false;
    }

    if let Some(warp_id) = warp_id {
        let pkt = match encode_message(Message::SubscribeWarp { warp_id }, 0) {
            Ok(pkt) => pkt,
            Err(err) => {
                tracing::warn!(error = %err, %warp_id, "failed to encode subscribe message");
                return false;
            }
        };
        if let Err(err) = stream.write_all(&pkt) {
            tracing::warn!(error = %err, %warp_id, "failed to write subscribe message");
            return false;
        }
    }
    true
}

fn run_write_loop(mut stream: UnixStream, warp_id: Option<WarpId>, out_rx: Receiver<Message>) {
    if !write_handshake_and_subscribe(&mut stream, warp_id) {
        return;
    }

    for msg in out_rx {
        let pkt = match encode_message(msg, 0) {
            Ok(pkt) => pkt,
            Err(err) => {
                tracing::warn!(error = %err, "failed to encode outbound session message");
                break;
            }
        };
        if let Err(err) = stream.write_all(&pkt) {
            tracing::warn!(error = %err, "session socket write failed");
            break;
        }
    }
}

fn run_read_loop(
    mut stream: UnixStream,
    warp_tx: mpsc::Sender<WarpFrame>,
    notif_tx: mpsc::Sender<Notification>,
) {
    loop {
        let mut header = [0u8; 12];
        if let Err(err) = stream.read_exact(&mut header) {
            tracing::debug!(error = %err, "read loop exiting: header read failed");
            break;
        }
        let len = u32::from_be_bytes([header[8], header[9], header[10], header[11]]) as usize;
        if len > MAX_PAYLOAD {
            tracing::warn!(
                payload_len = len,
                max_payload = MAX_PAYLOAD,
                "read loop exiting: payload exceeds MAX_PAYLOAD"
            );
            break;
        }
        let frame_len = match 12usize.checked_add(len).and_then(|v| v.checked_add(32)) {
            Some(v) => v,
            None => {
                tracing::warn!(
                    payload_len = len,
                    "read loop exiting: frame length overflow"
                );
                break;
            }
        };
        let mut rest = vec![0u8; len + 32];
        if let Err(err) = stream.read_exact(&mut rest) {
            tracing::debug!(error = %err, "read loop exiting: body read failed");
            break;
        }
        let mut packet = Vec::with_capacity(frame_len);
        packet.extend_from_slice(&header);
        packet.extend_from_slice(&rest);
        match decode_message(&packet) {
            Ok((Message::WarpStream { frame, .. }, _, _)) => {
                let _ = warp_tx.send(frame);
            }
            Ok((Message::Notification(n), _, _)) => {
                let _ = notif_tx.send(n);
            }
            Ok((Message::Error(err), _, _)) => {
                let scope = if err.code >= ERROR_CODE_GLOBAL_THRESHOLD {
                    NotifyScope::Global
                } else {
                    NotifyScope::Local
                };
                let _ = notif_tx.send(Notification {
                    kind: NotifyKind::Error,
                    scope,
                    title: format!("{} ({})", err.name, err.code),
                    body: Some(err.message),
                });
            }
            Ok((msg, _, _)) => {
                tracing::debug!(op = msg.op_name(), "read loop ignoring unsupported message");
                continue;
            }
            Err(err) => {
                tracing::warn!(error = %err, "read loop dropping invalid packet");
                continue;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use echo_session_proto::{NotifyKind, NotifyScope};
    use std::time::Duration;
    use tokio::io::AsyncWriteExt;
    use tokio::task;

    #[test]
    fn run_message_loop_classifies_error_scope_by_code() {
        let (client_stream, mut server_stream) = UnixStream::pair().unwrap();
        server_stream
            .set_read_timeout(Some(Duration::from_secs(1)))
            .expect("set_read_timeout failed");

        let (warp_tx, _warp_rx) = mpsc::channel();
        let (notif_tx, notif_rx) = mpsc::channel();

        let handle = thread::spawn(move || {
            run_message_loop(client_stream, 1, warp_tx, notif_tx);
        });

        let read_packet = |stream: &mut UnixStream| -> Vec<u8> {
            let mut header = [0u8; 12];
            stream.read_exact(&mut header).expect("read header");
            let len = u32::from_be_bytes([header[8], header[9], header[10], header[11]]) as usize;
            let mut rest = vec![0u8; len + 32];
            stream.read_exact(&mut rest).expect("read body");
            let mut packet = Vec::with_capacity(12 + len + 32);
            packet.extend_from_slice(&header);
            packet.extend_from_slice(&rest);
            packet
        };

        let (handshake, _, _) = decode_message(&read_packet(&mut server_stream)).unwrap();
        assert!(matches!(handshake, Message::Handshake(_)));

        let (subscribe, _, _) = decode_message(&read_packet(&mut server_stream)).unwrap();
        assert!(matches!(subscribe, Message::SubscribeWarp { warp_id: 1 }));

        let encoded_global = encode_message(
            Message::Error(echo_session_proto::ErrorPayload {
                code: 403,
                name: "E_FORBIDDEN_PUBLISH".into(),
                details: None,
                message: "forbidden".into(),
            }),
            1,
        )
        .unwrap();
        server_stream.write_all(&encoded_global).unwrap();

        let encoded_local = encode_message(
            Message::Error(echo_session_proto::ErrorPayload {
                code: 3,
                name: "E_BAD_PAYLOAD".into(),
                details: None,
                message: "bad payload".into(),
            }),
            2,
        )
        .unwrap();
        server_stream.write_all(&encoded_local).unwrap();
        drop(server_stream);

        let n1 = notif_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert_eq!(n1.scope, NotifyScope::Global);
        assert_eq!(n1.kind, NotifyKind::Error);
        assert_eq!(n1.title, "E_FORBIDDEN_PUBLISH (403)");
        assert_eq!(n1.body, Some("forbidden".to_string()));

        let n2 = notif_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert_eq!(n2.scope, NotifyScope::Local);
        assert_eq!(n2.kind, NotifyKind::Error);
        assert_eq!(n2.title, "E_BAD_PAYLOAD (3)");
        assert_eq!(n2.body, Some("bad payload".to_string()));

        handle.join().unwrap();
    }

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

    fn read_one_packet(stream: &mut UnixStream) -> Vec<u8> {
        let mut header = [0u8; 12];
        stream.read_exact(&mut header).expect("read header");
        let len = u32::from_be_bytes([header[8], header[9], header[10], header[11]]) as usize;
        assert!(len <= MAX_PAYLOAD, "len within cap");
        let mut rest = vec![0u8; len + 32];
        stream.read_exact(&mut rest).expect("read payload+checksum");
        let mut pkt = Vec::with_capacity(12 + len + 32);
        pkt.extend_from_slice(&header);
        pkt.extend_from_slice(&rest);
        pkt
    }

    #[test]
    fn write_loop_sends_handshake_subscribe_and_outbound_messages() {
        let (client_stream, mut server_stream) = UnixStream::pair().unwrap();
        let (out_tx, out_rx) = mpsc::channel();

        let handle = thread::spawn(move || {
            run_write_loop(client_stream, Some(7), out_rx);
        });

        let pkt_handshake = read_one_packet(&mut server_stream);
        let (m1, _ts1, _) = decode_message(&pkt_handshake).expect("decode handshake");
        assert!(matches!(m1, Message::Handshake(_)));

        let pkt_sub = read_one_packet(&mut server_stream);
        let (m2, _ts2, _) = decode_message(&pkt_sub).expect("decode subscribe");
        assert!(matches!(m2, Message::SubscribeWarp { warp_id: 7 }));

        out_tx
            .send(Message::WarpStream {
                warp_id: 7,
                frame: WarpFrame::Snapshot(echo_session_proto::WarpSnapshot {
                    epoch: 0,
                    graph: echo_session_proto::RenderGraph::default(),
                    state_hash: None,
                }),
            })
            .unwrap();

        let pkt_stream = read_one_packet(&mut server_stream);
        let (m3, _ts3, _) = decode_message(&pkt_stream).expect("decode stream");
        assert!(matches!(m3, Message::WarpStream { warp_id: 7, .. }));

        drop(out_tx);
        handle.join().unwrap();
    }
}
