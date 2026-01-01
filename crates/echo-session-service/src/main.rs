// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Minimal Unix-socket CBOR hub skeleton.

use anyhow::Result;
use echo_app_core::config::ConfigService;
use echo_config_fs::FsConfigStore;
use echo_graph::{WarpFrame, WarpSnapshot};
use echo_session_proto::{
    default_socket_path,
    wire::{decode_message, encode_message},
    AckStatus, ErrorPayload, HandshakeAckPayload, Message, WarpId,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::Mutex;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HostPrefs {
    socket_path: String,
}

impl Default for HostPrefs {
    fn default() -> Self {
        Self {
            socket_path: default_socket_path().display().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use echo_graph::{RenderGraph, WarpDiff, WarpFrame, WarpSnapshot};
    use echo_session_proto::{wire::decode_message, HandshakePayload, NotifyKind};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::time::{timeout, Duration};

    async fn add_conn(hub: &Arc<Mutex<HubState>>) -> (u64, tokio::sync::mpsc::Receiver<Vec<u8>>) {
        let (tx, rx) = tokio::sync::mpsc::channel::<Vec<u8>>(64);
        let mut h = hub.lock().await;
        let id = h.next_conn_id;
        h.next_conn_id += 1;
        h.conns.insert(
            id,
            ConnState {
                subscribed: HashSet::new(),
                tx,
            },
        );
        (id, rx)
    }

    async fn wait_for_conn(hub: &Arc<Mutex<HubState>>, conn_id: u64) {
        timeout(Duration::from_secs(1), async {
            loop {
                if hub.lock().await.conns.contains_key(&conn_id) {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("conn registered");
    }

    async fn spawn_loopback_client(hub: &Arc<Mutex<HubState>>) -> (u64, TestClient) {
        let (server_end, client_end) = UnixStream::pair().expect("unix pair");
        let expected_id = hub.lock().await.next_conn_id;
        let hub_state = hub.clone();
        tokio::spawn(async move {
            let _ = handle_client(server_end, hub_state).await;
        });
        wait_for_conn(hub, expected_id).await;
        (expected_id, TestClient::new(client_end))
    }

    async fn wait_for_subscription(hub: &Arc<Mutex<HubState>>, warp_id: WarpId, conn_id: u64) {
        timeout(Duration::from_secs(1), async {
            loop {
                let is_subscribed = hub
                    .lock()
                    .await
                    .streams
                    .get(&warp_id)
                    .is_some_and(|stream| stream.subscribers.contains(&conn_id));
                if is_subscribed {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("subscription registered");
    }

    async fn recv_packet(reader: &mut tokio::io::ReadHalf<UnixStream>) -> Vec<u8> {
        let mut header = [0u8; 12];
        reader.read_exact(&mut header).await.expect("read header");
        let len = u32::from_be_bytes([header[8], header[9], header[10], header[11]]) as usize;
        let mut rest = vec![0u8; len + 32];
        reader
            .read_exact(&mut rest)
            .await
            .expect("read payload+checksum");
        let mut pkt = Vec::with_capacity(12 + rest.len());
        pkt.extend_from_slice(&header);
        pkt.extend_from_slice(&rest);
        pkt
    }

    struct TestClient {
        reader: tokio::io::ReadHalf<UnixStream>,
        writer: tokio::io::WriteHalf<UnixStream>,
    }

    impl TestClient {
        fn new(stream: UnixStream) -> Self {
            let (reader, writer) = tokio::io::split(stream);
            Self { reader, writer }
        }

        async fn send(&mut self, msg: Message) {
            let pkt = encode_message(msg, 0).expect("encode message");
            self.writer.write_all(&pkt).await.expect("write packet");
        }

        async fn recv(&mut self) -> (Message, u64) {
            let pkt = recv_packet(&mut self.reader).await;
            let (msg, ts, _) = decode_message(&pkt).expect("decode packet");
            (msg, ts)
        }

        async fn recv_timeout(&mut self, dur: Duration) -> Option<(Message, u64)> {
            (timeout(dur, self.recv()).await).ok()
        }
    }

    #[tokio::test]
    async fn ts_is_monotonic_for_handshake_and_notification() {
        let hub = Arc::new(Mutex::new(HubState::default()));
        let (conn_id, mut rx) = add_conn(&hub).await;

        // first message: handshake -> ts should be 0
        handle_message(
            Message::Handshake(HandshakePayload {
                agent_id: None,
                capabilities: vec![],
                client_version: 1,
                session_meta: None,
            }),
            conn_id,
            &hub,
        )
        .await
        .unwrap();
        let pkt1 = timeout(Duration::from_secs(1), rx.recv())
            .await
            .ok()
            .flatten()
            .expect("handshake ack");
        let (_msg1, ts1, _) = decode_message(&pkt1).expect("decode ack");

        // second message: notification -> ts should be ts1 + 1
        handle_message(
            Message::Notification(echo_session_proto::Notification {
                kind: NotifyKind::Info,
                scope: echo_session_proto::NotifyScope::Global,
                title: "hello".into(),
                body: None,
            }),
            conn_id,
            &hub,
        )
        .await
        .unwrap();
        let pkt2 = timeout(Duration::from_secs(1), rx.recv())
            .await
            .ok()
            .flatten()
            .expect("notification");
        let (_msg2, ts2, _) = decode_message(&pkt2).expect("decode notify");

        assert_eq!(ts1, 0);
        assert_eq!(ts2, ts1 + 1);
    }

    #[tokio::test]
    async fn warp_stream_is_gapless_and_monotonic() {
        let hub = Arc::new(Mutex::new(HubState::default()));
        let (producer, _rx_prod) = add_conn(&hub).await;
        let (subscriber, mut rx_sub) = add_conn(&hub).await;

        // subscriber registers interest in warp_id 1
        handle_message(Message::SubscribeWarp { warp_id: 1 }, subscriber, &hub)
            .await
            .unwrap();

        // producer sends snapshot epoch 0
        handle_message(
            Message::WarpStream {
                warp_id: 1,
                frame: WarpFrame::Snapshot(WarpSnapshot {
                    epoch: 0,
                    graph: RenderGraph::default(),
                    state_hash: None,
                }),
            },
            producer,
            &hub,
        )
        .await
        .unwrap();
        let pkt_snap = timeout(Duration::from_secs(1), rx_sub.recv())
            .await
            .ok()
            .flatten()
            .expect("snapshot to subscriber");
        let (_m_snap, ts_snap, _) = decode_message(&pkt_snap).expect("decode snapshot");
        assert_eq!(ts_snap, 0);

        // producer sends diff 0->1 (valid)
        handle_message(
            Message::WarpStream {
                warp_id: 1,
                frame: WarpFrame::Diff(WarpDiff {
                    from_epoch: 0,
                    to_epoch: 1,
                    ops: vec![],
                    state_hash: None,
                }),
            },
            producer,
            &hub,
        )
        .await
        .unwrap();
        let pkt_diff = rx_sub.recv().await.expect("diff to subscriber");
        let (_m_diff, ts_diff, _) = decode_message(&pkt_diff).expect("decode diff");
        assert_eq!(ts_diff, ts_snap + 1);

        // gapful diff should error and not deliver anything
        let err = handle_message(
            Message::WarpStream {
                warp_id: 1,
                frame: WarpFrame::Diff(WarpDiff {
                    from_epoch: 3,
                    to_epoch: 4,
                    ops: vec![],
                    state_hash: None,
                }),
            },
            producer,
            &hub,
        )
        .await;
        assert!(err.is_err());
        assert!(
            rx_sub.try_recv().is_err(),
            "no packet should be sent on gap"
        );
    }

    #[tokio::test]
    async fn loopback_warp_stream_happy_path_and_errors() {
        let hub = Arc::new(Mutex::new(HubState::default()));

        let (_producer_id, mut producer) = spawn_loopback_client(&hub).await;
        let (subscriber_id, mut subscriber) = spawn_loopback_client(&hub).await;
        let (_attacker_id, mut attacker) = spawn_loopback_client(&hub).await;

        subscriber
            .send(Message::SubscribeWarp { warp_id: 42 })
            .await;
        wait_for_subscription(&hub, 42, subscriber_id).await;

        producer
            .send(Message::WarpStream {
                warp_id: 42,
                frame: WarpFrame::Snapshot(WarpSnapshot {
                    epoch: 0,
                    graph: RenderGraph::default(),
                    state_hash: None,
                }),
            })
            .await;

        let (msg, ts_snap) = subscriber
            .recv_timeout(Duration::from_secs(1))
            .await
            .expect("snapshot delivered");
        match msg {
            Message::WarpStream { warp_id, frame } => {
                assert_eq!(warp_id, 42);
                let WarpFrame::Snapshot(snap) = frame else {
                    panic!("expected snapshot");
                };
                assert_eq!(snap.epoch, 0);
            }
            other => panic!("expected warp stream, got {:?}", other),
        }

        producer
            .send(Message::WarpStream {
                warp_id: 42,
                frame: WarpFrame::Diff(WarpDiff {
                    from_epoch: 0,
                    to_epoch: 1,
                    ops: vec![],
                    state_hash: None,
                }),
            })
            .await;

        let (msg, ts_diff) = subscriber
            .recv_timeout(Duration::from_secs(1))
            .await
            .expect("diff delivered");
        assert!(
            ts_diff > ts_snap,
            "expected monotonic ts for delivered stream frames"
        );
        match msg {
            Message::WarpStream { warp_id, frame } => {
                assert_eq!(warp_id, 42);
                let WarpFrame::Diff(diff) = frame else {
                    panic!("expected diff");
                };
                assert_eq!(diff.from_epoch, 0);
                assert_eq!(diff.to_epoch, 1);
            }
            other => panic!("expected warp stream, got {:?}", other),
        }

        // Attacker cannot publish (producer already claimed ownership).
        attacker
            .send(Message::WarpStream {
                warp_id: 42,
                frame: WarpFrame::Diff(WarpDiff {
                    from_epoch: 1,
                    to_epoch: 2,
                    ops: vec![],
                    state_hash: None,
                }),
            })
            .await;

        let (msg, _ts) = attacker
            .recv_timeout(Duration::from_secs(1))
            .await
            .expect("attacker error delivered");
        match msg {
            Message::Error(payload) => {
                assert_eq!(payload.name, "E_FORBIDDEN_PUBLISH");
                assert_eq!(payload.code, 403);
            }
            other => panic!("expected error, got {:?}", other),
        }

        assert!(
            subscriber
                .recv_timeout(Duration::from_millis(150))
                .await
                .is_none(),
            "subscriber should not receive attacker frames"
        );

        // Gapful diff from the producer should be rejected, and should not be broadcast.
        producer
            .send(Message::WarpStream {
                warp_id: 42,
                frame: WarpFrame::Diff(WarpDiff {
                    from_epoch: 9,
                    to_epoch: 10,
                    ops: vec![],
                    state_hash: None,
                }),
            })
            .await;

        let (msg, _ts) = producer
            .recv_timeout(Duration::from_secs(1))
            .await
            .expect("producer gap error delivered");
        match msg {
            Message::Error(payload) => {
                assert_eq!(payload.name, "E_WARP_EPOCH_GAP");
                assert_eq!(payload.code, 409);
            }
            other => panic!("expected error, got {:?}", other),
        }

        assert!(
            subscriber
                .recv_timeout(Duration::from_millis(150))
                .await
                .is_none(),
            "subscriber should not receive gapful frames"
        );
    }

    #[tokio::test]
    async fn non_owner_publish_is_rejected_with_error() {
        let hub = Arc::new(Mutex::new(HubState::default()));
        let (owner, _rx_owner) = add_conn(&hub).await;
        let (attacker, mut rx_attacker) = add_conn(&hub).await;
        let (subscriber, mut rx_sub) = add_conn(&hub).await;

        handle_message(Message::SubscribeWarp { warp_id: 7 }, subscriber, &hub)
            .await
            .unwrap();

        handle_message(
            Message::WarpStream {
                warp_id: 7,
                frame: WarpFrame::Snapshot(WarpSnapshot {
                    epoch: 0,
                    graph: RenderGraph::default(),
                    state_hash: None,
                }),
            },
            owner,
            &hub,
        )
        .await
        .unwrap();
        timeout(Duration::from_secs(1), rx_sub.recv())
            .await
            .ok()
            .flatten()
            .expect("subscriber received snapshot");

        let err = handle_message(
            Message::WarpStream {
                warp_id: 7,
                frame: WarpFrame::Diff(WarpDiff {
                    from_epoch: 0,
                    to_epoch: 1,
                    ops: vec![],
                    state_hash: None,
                }),
            },
            attacker,
            &hub,
        )
        .await;
        assert!(err.is_err());

        let pkt = timeout(Duration::from_secs(1), rx_attacker.recv())
            .await
            .ok()
            .flatten()
            .expect("attacker receives error");
        let (msg, _ts, _) = decode_message(&pkt).expect("decode error payload");
        match msg {
            Message::Error(payload) => {
                assert_eq!(payload.name, "E_FORBIDDEN_PUBLISH");
                assert_eq!(payload.code, 403);
            }
            other => panic!("expected error, got {:?}", other),
        }

        assert!(
            rx_sub.try_recv().is_err(),
            "subscriber should not receive attacker diff"
        );
    }
}

#[derive(Default)]
struct StreamState {
    last_epoch: Option<u64>,
    last_hash: Option<echo_graph::Hash32>,
    latest_snapshot: Option<WarpSnapshot>,
    subscribers: HashSet<u64>,
    producer: Option<u64>,
}

struct ConnState {
    subscribed: HashSet<WarpId>,
    tx: tokio::sync::mpsc::Sender<Vec<u8>>,
}

#[derive(Default)]
struct HubState {
    next_conn_id: u64,
    next_ts: u64,
    streams: HashMap<WarpId, StreamState>,
    conns: HashMap<u64, ConnState>,
}

impl HubState {
    fn alloc_ts(&mut self) -> u64 {
        let t = self.next_ts;
        self.next_ts += 1;
        t
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Config (best-effort)
    let config: Option<ConfigService<FsConfigStore>> =
        FsConfigStore::new().map(ConfigService::new).ok();

    let prefs: HostPrefs = config
        .as_ref()
        .and_then(|c| c.load::<HostPrefs>("session_host").ok().flatten())
        .unwrap_or_default();

    // Persist defaults once if absent
    if let Some(cfg) = &config {
        let _ = cfg.save("session_host", &prefs);
    }

    let socket_path = prefs.socket_path.clone();

    let hub = Arc::new(Mutex::new(HubState::default()));

    // Remove stale socket if present
    let _ = std::fs::remove_file(&socket_path);
    let listener = UnixListener::bind(&socket_path)?;
    info!("session hub listening at {}", socket_path);

    loop {
        let (stream, _) = listener.accept().await?;
        let hub_state = hub.clone();
        tokio::spawn(async move {
            if let Err(err) = handle_client(stream, hub_state).await {
                warn!(?err, "client handler error");
            }
        });
    }
}

async fn handle_client(stream: UnixStream, hub: Arc<Mutex<HubState>>) -> Result<()> {
    // split stream
    let (mut reader, writer) = tokio::io::split(stream);

    // allocate conn id and outbox
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(256);
    let conn_id = {
        let mut h = hub.lock().await;
        let id = h.next_conn_id;
        h.next_conn_id += 1;
        h.conns.insert(
            id,
            ConnState {
                subscribed: HashSet::new(),
                tx,
            },
        );
        id
    };

    // greet with stub notification before moving writer into task
    // no greeting; wait for handshake

    // writer task
    tokio::spawn(async move {
        let mut ws = writer;
        while let Some(buf) = rx.recv().await {
            if ws.write_all(&buf).await.is_err() {
                break;
            }
        }
    });

    const MAX_PAYLOAD: usize = 8 * 1024 * 1024;
    let mut read_buf: Vec<u8> = vec![0u8; 16 * 1024];
    let mut acc: Vec<u8> = Vec::with_capacity(32 * 1024);
    loop {
        let n = reader.read(&mut read_buf).await?;
        if n == 0 {
            break;
        }
        acc.extend_from_slice(&read_buf[..n]);

        // process as many frames as available
        loop {
            if acc.len() < 12 {
                break;
            }
            let len = u32::from_be_bytes([acc[8], acc[9], acc[10], acc[11]]) as usize;
            if len > MAX_PAYLOAD {
                warn!("payload too large from conn {}", conn_id);
                return Ok(());
            }
            let frame_len = 12usize
                .checked_add(len)
                .and_then(|v| v.checked_add(32))
                .unwrap_or(usize::MAX);
            if frame_len == usize::MAX || acc.len() < frame_len {
                // need more data
                break;
            }
            let packet: Vec<u8> = acc.drain(..frame_len).collect();
            match decode_message(&packet) {
                Ok((msg, _ts, _used)) => {
                    if let Err(err) = handle_message(msg, conn_id, &hub).await {
                        warn!(?err, "dropping connection {}", conn_id);
                        return Ok(());
                    }
                }
                Err(err) => {
                    warn!(?err, "failed to decode packet");
                    return Ok(());
                }
            }
        }
    }

    // cleanup connection
    let mut h = hub.lock().await;
    if let Some(conn) = h.conns.remove(&conn_id) {
        for warp_id in conn.subscribed {
            if let Some(stream_state) = h.streams.get_mut(&warp_id) {
                stream_state.subscribers.remove(&conn_id);
                if stream_state.producer == Some(conn_id) {
                    stream_state.producer = None;
                }
            }
        }
    }

    Ok(())
}

// Handle a single inbound message from a connection.
async fn handle_message(msg: Message, conn_id: u64, hub: &Arc<Mutex<HubState>>) -> Result<()> {
    match msg {
        Message::Handshake(handshake) => {
            // accept all handshakes for now
            let mut h = hub.lock().await;
            let ts = h.alloc_ts();
            let ack = Message::HandshakeAck(HandshakeAckPayload {
                status: AckStatus::Ok,
                server_version: handshake.client_version, // echo back
                capabilities: handshake.capabilities.clone(),
                session_id: "default".into(),
                error: None,
            });
            // send ack
            if let Some(conn) = h.conns.get(&conn_id) {
                let pkt = encode_message(ack, ts)?;
                let _ = conn.tx.send(pkt).await;
            }
        }
        Message::SubscribeWarp { warp_id } => {
            let mut h = hub.lock().await;
            let conn = h
                .conns
                .get_mut(&conn_id)
                .ok_or_else(|| anyhow::anyhow!("missing conn"))?;
            conn.subscribed.insert(warp_id);
            let stream = h.streams.entry(warp_id).or_default();
            stream.subscribers.insert(conn_id);
            if let Some(snap) = stream.latest_snapshot.clone() {
                if let Some(tx) = h.conns.get(&conn_id).map(|c| c.tx.clone()) {
                    let ts = h.alloc_ts();
                    let pkt = encode_message(
                        Message::WarpStream {
                            warp_id,
                            frame: WarpFrame::Snapshot(snap),
                        },
                        ts,
                    )?;
                    let _ = tx.send(pkt).await;
                }
            }
        }
        Message::WarpStream { warp_id, frame } => {
            let (subs, pkt) = {
                let mut h = hub.lock().await;
                let mut error: Option<ErrorPayload> = None;
                let mut err_reason: Option<String> = None;
                let mut subs: Option<HashSet<u64>> = None;
                {
                    let stream = h.streams.entry(warp_id).or_default();
                    // enforce single producer
                    if let Some(p) = stream.producer {
                        if p != conn_id {
                            error = Some(ErrorPayload {
                                code: 403,
                                name: "E_FORBIDDEN_PUBLISH".into(),
                                details: None,
                                message: format!("warp_id {} is owned by {}", warp_id, p),
                            });
                            err_reason = Some(format!("producer mismatch for warp_id {}", warp_id));
                        }
                    } else {
                        stream.producer = Some(conn_id);
                    }

                    if error.is_none() {
                        match &frame {
                            WarpFrame::Snapshot(s) => {
                                stream.last_epoch = Some(s.epoch);
                                stream.last_hash = s.state_hash;
                                stream.latest_snapshot = Some(s.clone());
                            }
                            WarpFrame::Diff(d) => {
                                let last = match stream.last_epoch {
                                    Some(v) => v,
                                    None => {
                                        error = Some(ErrorPayload {
                                            code: 409,
                                            name: "E_WARP_SNAPSHOT_REQUIRED".into(),
                                            details: None,
                                            message: "send a snapshot before the first diff".into(),
                                        });
                                        err_reason = Some("diff before snapshot".into());
                                        0 // placeholder, unused when error is set
                                    }
                                };
                                if error.is_none()
                                    && (d.from_epoch != last || d.to_epoch != d.from_epoch + 1)
                                {
                                    error = Some(ErrorPayload {
                                        code: 409,
                                        name: "E_WARP_EPOCH_GAP".into(),
                                        details: None,
                                        message: format!(
                                            "expected {}->{} but got {}->{}",
                                            last,
                                            last + 1,
                                            d.from_epoch,
                                            d.to_epoch
                                        ),
                                    });
                                    err_reason = Some(format!(
                                        "gap for warp_id {}: got {}->{} expected {}->{}",
                                        warp_id,
                                        d.from_epoch,
                                        d.to_epoch,
                                        last,
                                        last + 1
                                    ));
                                }
                                if error.is_none() {
                                    stream.last_epoch = Some(d.to_epoch);
                                    stream.last_hash = d.state_hash;
                                }
                            }
                        }
                    }

                    if error.is_none() {
                        subs = Some(stream.subscribers.clone());
                    }
                } // drop stream borrow

                if let Some(payload) = error {
                    let tx = h.conns.get(&conn_id).map(|c| c.tx.clone());
                    let ts = h.alloc_ts();
                    if let Some(tx) = tx {
                        let pkt = encode_message(Message::Error(payload), ts)?;
                        let _ = tx.send(pkt).await;
                    }
                    let reason = err_reason.unwrap_or_else(|| "warp stream error".into());
                    anyhow::bail!(reason);
                }

                let subs = subs.unwrap_or_default();
                let ts = h.alloc_ts();
                let pkt = encode_message(Message::WarpStream { warp_id, frame }, ts)?;
                (subs, pkt)
            };

            let h = hub.lock().await;
            for sub in subs {
                if let Some(conn) = h.conns.get(&sub) {
                    let _ = conn.tx.send(pkt.clone()).await;
                }
            }
        }
        Message::Notification(_) => {
            // Broadcast notifications globally
            let (pkt, conns) = {
                let mut h = hub.lock().await;
                let ts = h.alloc_ts();
                let pkt = encode_message(msg.clone(), ts)?;
                let conns: Vec<_> = h.conns.values().map(|c| c.tx.clone()).collect();
                (pkt, conns)
            };
            for tx in conns {
                let _ = tx.send(pkt.clone()).await;
            }
        }
        Message::HandshakeAck(_) | Message::Error(_) => {
            // should not be initiated by clients; ignore
        }
    }
    Ok(())
}
