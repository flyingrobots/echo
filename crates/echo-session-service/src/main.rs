// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Minimal Unix-socket CBOR hub skeleton.

use anyhow::Result;
use echo_app_core::config::ConfigService;
use echo_config_fs::FsConfigStore;
use echo_graph::{RmgFrame, RmgSnapshot};
use echo_session_proto::{
    wire::{decode_message, encode_message},
    AckStatus, HandshakeAckPayload, Message, RmgId,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::Mutex;
use tracing::{info, warn};

const DEFAULT_SOCKET_PATH: &str = "/tmp/echo-session.sock";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HostPrefs {
    socket_path: String,
}

impl Default for HostPrefs {
    fn default() -> Self {
        Self {
            socket_path: DEFAULT_SOCKET_PATH.into(),
        }
    }
}

#[derive(Default)]
struct StreamState {
    last_epoch: Option<u64>,
    last_hash: Option<echo_graph::Hash32>,
    latest_snapshot: Option<RmgSnapshot>,
    subscribers: HashSet<u64>,
    producer: Option<u64>,
}

struct ConnState {
    subscribed: HashSet<RmgId>,
    tx: tokio::sync::mpsc::Sender<Vec<u8>>,
}

#[derive(Default)]
struct HubState {
    next_conn_id: u64,
    next_ts: u64,
    streams: HashMap<RmgId, StreamState>,
    conns: HashMap<u64, ConnState>,
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

    let mut buf = vec![0u8; 16 * 1024];
    loop {
        let n = reader.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        let slice = &buf[..n];
        match decode_message(slice) {
            Ok((msg, _ts, _used)) => {
                if let Err(err) = handle_message(msg, conn_id, &hub).await {
                    warn!(?err, "dropping connection {}", conn_id);
                    break;
                }
            }
            Err(err) => {
                warn!(?err, "failed to decode packet");
                break;
            }
        }
    }

    // cleanup connection
    let mut h = hub.lock().await;
    if let Some(conn) = h.conns.remove(&conn_id) {
        for rmg_id in conn.subscribed {
            if let Some(stream_state) = h.streams.get_mut(&rmg_id) {
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
            let ts = {
                let t = h.next_ts;
                h.next_ts += 1;
                t
            };
            let ack = Message::HandshakeAck(HandshakeAckPayload {
                status: AckStatus::OK,
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
        Message::SubscribeRmg { rmg_id } => {
            let mut h = hub.lock().await;
            let conn = h
                .conns
                .get_mut(&conn_id)
                .ok_or_else(|| anyhow::anyhow!("missing conn"))?;
            conn.subscribed.insert(rmg_id);
            let stream = h.streams.entry(rmg_id).or_default();
            stream.subscribers.insert(conn_id);
            if let Some(snap) = stream.latest_snapshot.clone() {
                if let Some(tx) = h.conns.get(&conn_id).map(|c| c.tx.clone()) {
                    let ts = {
                        let t = h.next_ts;
                        h.next_ts += 1;
                        t
                    };
                    let pkt = encode_message(
                        Message::RmgStream {
                            rmg_id,
                            frame: RmgFrame::Snapshot(snap),
                        },
                        ts,
                    )?;
                    let _ = tx.send(pkt).await;
                }
            }
        }
        Message::RmgStream { rmg_id, frame } => {
            let mut h = hub.lock().await;
            let ts = {
                let t = h.next_ts;
                h.next_ts += 1;
                t
            };
            let stream = h.streams.entry(rmg_id).or_default();
            // enforce single producer
            if let Some(p) = stream.producer {
                if p != conn_id {
                    anyhow::bail!("producer mismatch for rmg_id {}", rmg_id);
                }
            } else {
                stream.producer = Some(conn_id);
            }
            match &frame {
                RmgFrame::Snapshot(s) => {
                    stream.last_epoch = Some(s.epoch);
                    stream.last_hash = s.state_hash;
                    stream.latest_snapshot = Some(s.clone());
                }
                RmgFrame::Diff(d) => {
                    let last = stream
                        .last_epoch
                        .ok_or_else(|| anyhow::anyhow!("diff before snapshot"))?;
                    if d.from_epoch != last || d.to_epoch != d.from_epoch + 1 {
                        anyhow::bail!(
                            "gap for rmg_id {}: got {}->{} expected {}->{}",
                            rmg_id,
                            d.from_epoch,
                            d.to_epoch,
                            last,
                            last + 1
                        );
                    }
                    stream.last_epoch = Some(d.to_epoch);
                    stream.last_hash = d.state_hash;
                }
            }
            // fan out to subscribers
            let pkt = encode_message(Message::RmgStream { rmg_id, frame }, ts)?;
            let subs = stream.subscribers.clone();
            for sub in subs {
                if let Some(conn) = h.conns.get(&sub) {
                    let _ = conn.tx.send(pkt.clone()).await;
                }
            }
        }
        Message::Notification(_) => {
            // Broadcast notifications globally
            let mut h = hub.lock().await;
            let ts = {
                let t = h.next_ts;
                h.next_ts += 1;
                t
            };
            let pkt = encode_message(msg, ts)?;
            let h = hub.lock().await;
            for conn in h.conns.values() {
                let _ = conn.tx.send(pkt.clone()).await;
            }
        }
        Message::HandshakeAck(_) | Message::Error(_) => {
            // should not be initiated by clients; ignore
        }
    }
    Ok(())
}
