// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! WebSocket ↔ Unix socket bridge for the Echo session service.
//! Browsers speak WebSocket; the bridge forwards binary JS-ABI frames to the Unix bus.

use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    path::PathBuf,
    sync::Arc,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Context, Result};
use axum::body::Bytes;
use axum::{
    extract::ws::{Message, WebSocket},
    extract::{ConnectInfo, State, WebSocketUpgrade},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{Html, IntoResponse},
    routing::get,
    Json, Router,
};
use axum_server::{tls_rustls::RustlsConfig, Handle};
use clap::Parser;
use echo_session_proto::{
    wire::decode_message as decode_session_message, Message as SessionMessage,
};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::task::JoinError;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
    sync::{mpsc, Mutex},
    time::{self, Duration},
};
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

/// JS-ABI v1 framing header size (bytes).
///
/// The payload length is encoded as a big-endian `u32` at offsets `8..12`.
const JS_ABI_HEADER_BYTES: usize = 12;
/// Trailing per-frame hash/checksum size (bytes).
const JS_ABI_HASH_BYTES: usize = 32;
/// JS-ABI framing overhead: header + trailing hash/checksum bytes.
const JS_ABI_OVERHEAD_BYTES: usize = JS_ABI_HEADER_BYTES + JS_ABI_HASH_BYTES;
type TaskResult<T> = std::result::Result<T, JoinError>;

const DASHBOARD_HTML: &str = include_str!("../assets/dashboard.html");
const D3_JS: &[u8] = include_bytes!("../../../docs/benchmarks/vendor/d3.v7.min.js");

#[derive(Debug)]
struct ConnMetrics {
    peer: SocketAddr,
    subscribed_warps: HashSet<u64>,
    published_warps: HashSet<u64>,
    last_seen_ms: u64,
}

impl ConnMetrics {
    fn new(peer: SocketAddr, now_ms: u64) -> Self {
        Self {
            peer,
            subscribed_warps: HashSet::new(),
            published_warps: HashSet::new(),
            last_seen_ms: now_ms,
        }
    }
}

#[derive(Debug, Default)]
struct WarpMetrics {
    subscribers: HashSet<u64>,
    publishers: HashSet<u64>,
    last_epoch: Option<u64>,
    last_frame: Option<&'static str>,
    last_ts: Option<u64>,
    last_state_hash: Option<[u8; 32]>,
    snapshot_count: u64,
    diff_count: u64,
    last_update_ms: u64,
}

#[derive(Debug, Default, Serialize, Clone, Copy)]
struct MessageCounters {
    handshake: u64,
    handshake_ack: u64,
    subscribe_warp: u64,
    warp_stream: u64,
    notification: u64,
    error: u64,
}

#[derive(Debug, Default)]
struct GatewayMetrics {
    next_conn_id: u64,
    total_connections: u64,
    active_connections: usize,

    ws_to_uds_bytes: u64,
    ws_to_uds_frames: u64,
    uds_to_ws_bytes: u64,
    uds_to_ws_frames: u64,

    invalid_ws_frames: u64,
    decode_errors: u64,

    messages: MessageCounters,

    connections: HashMap<u64, ConnMetrics>,
    warps: HashMap<u64, WarpMetrics>,
}

#[derive(Debug, Serialize)]
struct DirectionCounters {
    bytes: u64,
    frames: u64,
}

#[derive(Debug, Serialize)]
struct WarpMetricsResponse {
    warp_id: u64,
    subscribers: usize,
    publishers: usize,
    last_epoch: Option<u64>,
    last_frame: Option<&'static str>,
    last_ts: Option<u64>,
    last_state_hash: Option<String>,
    snapshot_count: u64,
    diff_count: u64,
    last_update_ms: u64,
}

#[derive(Debug, Serialize)]
struct ConnMetricsResponse {
    conn_id: u64,
    peer: String,
    subscribed_warps: Vec<u64>,
    published_warps: Vec<u64>,
    last_seen_ms: u64,
}

#[derive(Debug, Serialize)]
struct MetricsResponse {
    started_at_unix_ms: u64,
    uptime_ms: u64,
    active_connections: usize,
    total_connections: u64,

    ws_to_uds: DirectionCounters,
    uds_to_ws: DirectionCounters,

    invalid_ws_frames: u64,
    decode_errors: u64,
    messages: MessageCounters,

    warps: Vec<WarpMetricsResponse>,
    connections: Vec<ConnMetricsResponse>,
}

impl GatewayMetrics {
    fn alloc_conn(&mut self, peer: SocketAddr, now_ms: u64) -> u64 {
        let conn_id = self.next_conn_id;
        self.next_conn_id = self.next_conn_id.wrapping_add(1);
        self.total_connections = self.total_connections.wrapping_add(1);
        self.active_connections = self.active_connections.saturating_add(1);
        self.connections
            .insert(conn_id, ConnMetrics::new(peer, now_ms));
        conn_id
    }

    fn remove_conn(&mut self, conn_id: u64) {
        self.active_connections = self.active_connections.saturating_sub(1);
        let Some(conn) = self.connections.remove(&conn_id) else {
            return;
        };

        for warp_id in conn.subscribed_warps {
            if let Some(warp) = self.warps.get_mut(&warp_id) {
                warp.subscribers.remove(&conn_id);
            }
        }
        for warp_id in conn.published_warps {
            if let Some(warp) = self.warps.get_mut(&warp_id) {
                warp.publishers.remove(&conn_id);
            }
        }
    }

    fn touch_conn(&mut self, conn_id: u64, now_ms: u64) {
        if let Some(conn) = self.connections.get_mut(&conn_id) {
            conn.last_seen_ms = now_ms;
        }
    }

    fn observe_message(&mut self, conn_id: u64, msg: &SessionMessage, ts: u64, now_ms: u64) {
        self.touch_conn(conn_id, now_ms);

        match msg {
            SessionMessage::Handshake(_) => self.messages.handshake += 1,
            SessionMessage::HandshakeAck(_) => self.messages.handshake_ack += 1,
            SessionMessage::Notification(_) => self.messages.notification += 1,
            SessionMessage::Error(_) => self.messages.error += 1,
            SessionMessage::SubscribeWarp { warp_id } => {
                self.messages.subscribe_warp += 1;
                if let Some(conn) = self.connections.get_mut(&conn_id) {
                    conn.subscribed_warps.insert(*warp_id);
                }
                self.warps
                    .entry(*warp_id)
                    .or_default()
                    .subscribers
                    .insert(conn_id);
            }
            SessionMessage::WarpStream { warp_id, frame } => {
                self.messages.warp_stream += 1;
                if let Some(conn) = self.connections.get_mut(&conn_id) {
                    conn.published_warps.insert(*warp_id);
                }
                let entry = self.warps.entry(*warp_id).or_default();
                entry.publishers.insert(conn_id);
                entry.last_ts = Some(ts);
                entry.last_update_ms = now_ms;
                match frame {
                    echo_session_proto::WarpFrame::Snapshot(snapshot) => {
                        entry.snapshot_count += 1;
                        entry.last_frame = Some("snapshot");
                        entry.last_epoch = Some(snapshot.epoch);
                        entry.last_state_hash = snapshot.state_hash;
                    }
                    echo_session_proto::WarpFrame::Diff(diff) => {
                        entry.diff_count += 1;
                        entry.last_frame = Some("diff");
                        entry.last_epoch = Some(diff.to_epoch);
                        entry.last_state_hash = diff.state_hash;
                    }
                }
            }
        }
    }

    fn snapshot(&self, started_at_unix_ms: u64, uptime_ms: u64) -> MetricsResponse {
        let mut warps: Vec<WarpMetricsResponse> = self
            .warps
            .iter()
            .map(|(&warp_id, w)| WarpMetricsResponse {
                warp_id,
                subscribers: w.subscribers.len(),
                publishers: w.publishers.len(),
                last_epoch: w.last_epoch,
                last_frame: w.last_frame,
                last_ts: w.last_ts,
                last_state_hash: w.last_state_hash.map(hex32),
                snapshot_count: w.snapshot_count,
                diff_count: w.diff_count,
                last_update_ms: w.last_update_ms,
            })
            .collect();
        warps.sort_by_key(|w| w.warp_id);

        let mut connections: Vec<ConnMetricsResponse> = self
            .connections
            .iter()
            .map(|(&conn_id, c)| {
                let mut subscribed_warps: Vec<u64> = c.subscribed_warps.iter().copied().collect();
                subscribed_warps.sort_unstable();
                let mut published_warps: Vec<u64> = c.published_warps.iter().copied().collect();
                published_warps.sort_unstable();

                ConnMetricsResponse {
                    conn_id,
                    peer: c.peer.to_string(),
                    subscribed_warps,
                    published_warps,
                    last_seen_ms: c.last_seen_ms,
                }
            })
            .collect();
        connections.sort_by_key(|c| c.conn_id);

        MetricsResponse {
            started_at_unix_ms,
            uptime_ms,
            active_connections: self.active_connections,
            total_connections: self.total_connections,
            ws_to_uds: DirectionCounters {
                bytes: self.ws_to_uds_bytes,
                frames: self.ws_to_uds_frames,
            },
            uds_to_ws: DirectionCounters {
                bytes: self.uds_to_ws_bytes,
                frames: self.uds_to_ws_frames,
            },
            invalid_ws_frames: self.invalid_ws_frames,
            decode_errors: self.decode_errors,
            messages: self.messages,
            warps,
            connections,
        }
    }
}

fn hex32(bytes: [u8; 32]) -> String {
    const LUT: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for b in bytes {
        out.push(LUT[(b >> 4) as usize] as char);
        out.push(LUT[(b & 0x0f) as usize] as char);
    }
    out
}

#[derive(Parser, Debug)]
#[command(author, version, about = "Echo session WebSocket gateway")]
struct Args {
    /// TCP listener for browser clients (e.g. 0.0.0.0:8787)
    #[arg(long, default_value = "0.0.0.0:8787")]
    listen: SocketAddr,
    /// Path to the Unix socket exposed by echo-session-service
    #[arg(long, default_value = "/tmp/echo-session.sock")]
    unix_socket: PathBuf,
    /// Maximum frame payload in bytes (binary WS message must match exact frame length)
    #[arg(long, default_value_t = 8 * 1024 * 1024)]
    max_frame_bytes: usize,
    /// Optional allowed Origin values (repeatable). If none provided, all origins are accepted.
    #[arg(long)]
    allow_origin: Vec<String>,
    /// TLS certificate (PEM). If provided, key must also be provided.
    #[arg(long)]
    tls_cert: Option<PathBuf>,
    /// TLS private key (PEM). If provided, cert must also be provided.
    #[arg(long)]
    tls_key: Option<PathBuf>,
}

#[derive(Clone)]
struct AppState {
    unix_socket: PathBuf,
    max_frame_bytes: usize,
    allow_origins: Option<HashSet<String>>,
    started_at_unix_ms: u64,
    start_instant: Instant,
    metrics: Arc<Mutex<GatewayMetrics>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    let allow_origins = if args.allow_origin.is_empty() {
        None
    } else {
        Some(args.allow_origin.iter().cloned().collect())
    };

    let started_at_unix_ms: u64 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(0);

    let metrics = Arc::new(Mutex::new(GatewayMetrics::default()));

    let state = Arc::new(AppState {
        unix_socket: args.unix_socket.clone(),
        max_frame_bytes: args.max_frame_bytes,
        allow_origins,
        started_at_unix_ms,
        start_instant: Instant::now(),
        metrics,
    });

    let app = Router::new()
        .route("/", get(dashboard_handler))
        .route("/dashboard", get(dashboard_handler))
        .route("/vendor/d3.v7.min.js", get(d3_handler))
        .route("/api/metrics", get(metrics_handler))
        .route("/ws", get(ws_handler))
        .with_state(state);

    let handle = Handle::new();
    // graceful shutdown on Ctrl+C
    let shutdown = handle.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install ctrl-c handler");
        shutdown.shutdown();
    });

    match (args.tls_cert, args.tls_key) {
        (Some(cert), Some(key)) => {
            let tls_config = load_tls(cert, key).await.context("load tls config")?;
            info!("ws gateway listening (TLS) on {}", args.listen);
            axum_server::bind_rustls(args.listen, tls_config)
                .handle(handle)
                .serve(app.into_make_service_with_connect_info::<SocketAddr>())
                .await?;
        }
        (None, None) => {
            info!("ws gateway listening on {}", args.listen);
            axum_server::bind(args.listen)
                .handle(handle)
                .serve(app.into_make_service_with_connect_info::<SocketAddr>())
                .await?;
        }
        _ => {
            return Err(anyhow!(
                "must provide both --tls-cert and --tls-key or neither"
            ))
        }
    }

    Ok(())
}

async fn dashboard_handler() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}

async fn d3_handler() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/javascript; charset=utf-8"),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    (headers, Bytes::from_static(D3_JS))
}

async fn metrics_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let uptime_ms: u64 = state
        .start_instant
        .elapsed()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX);

    let snapshot = {
        let metrics = state.metrics.lock().await;
        metrics.snapshot(state.started_at_unix_ms, uptime_ms)
    };

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, max-age=0"),
    );
    // Make this endpoint easy to consume from local docs or other dev servers.
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("*"),
    );
    (headers, Json(snapshot))
}

async fn ws_handler(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    if !origin_allowed(&state, &headers) {
        let origin = headers
            .get("origin")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("<missing>");
        warn!(?addr, origin = %origin, "origin rejected");
        return StatusCode::FORBIDDEN.into_response();
    }
    ws.on_upgrade(move |socket| handle_socket(socket, state, addr))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>, peer: SocketAddr) {
    let socket_path = state.unix_socket.clone();
    let unix = match time::timeout(Duration::from_secs(2), UnixStream::connect(&socket_path)).await
    {
        Ok(Ok(stream)) => stream,
        Ok(Err(err)) => {
            error!(?err, path = %socket_path.display(), "failed to connect to unix socket");
            let _ = socket
                .send(Message::Close(Some(axum::extract::ws::CloseFrame {
                    code: axum::extract::ws::close_code::ERROR,
                    reason: "upstream unavailable".into(),
                })))
                .await;
            return;
        }
        Err(_) => {
            warn!(
                ?peer,
                path = %socket_path.display(),
                "timed out connecting to unix socket"
            );
            let _ = socket
                .send(Message::Close(Some(axum::extract::ws::CloseFrame {
                    code: axum::extract::ws::close_code::ERROR,
                    reason: "upstream connect timeout".into(),
                })))
                .await;
            return;
        }
    };

    let conn_id = {
        let now_ms: u64 = state
            .start_instant
            .elapsed()
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX);
        let mut metrics = state.metrics.lock().await;
        metrics.alloc_conn(peer, now_ms)
    };

    let (mut ws_tx, mut ws_rx) = socket.split();
    let (mut uds_reader, mut uds_writer) = tokio::io::split(unix);
    let (out_tx, mut out_rx) = mpsc::channel::<Message>(256);

    // Writer task: WS outbound messages (binary frames, pongs)
    let writer = tokio::spawn(async move {
        while let Some(msg) = out_rx.recv().await {
            if ws_tx.send(msg).await.is_err() {
                break;
            }
        }
    });

    // UDS -> WS task: frame and forward packets
    let max_len = state.max_frame_bytes;
    let out_tx_clone = out_tx.clone();
    let metrics_uds_to_ws = state.metrics.clone();
    let start_instant_uds_to_ws = state.start_instant;
    let uds_to_ws = tokio::spawn(async move {
        let mut buf = vec![0u8; 16 * 1024];
        let mut acc: Vec<u8> = Vec::with_capacity(32 * 1024);
        let max_acc = max_len
            .saturating_add(JS_ABI_OVERHEAD_BYTES)
            .saturating_add(buf.len());
        loop {
            let n = uds_reader.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            acc.extend_from_slice(&buf[..n]);
            if acc.len() > max_acc {
                return Err(anyhow!(
                    "accumulator overflow ({} > {}): malformed upstream framing",
                    acc.len(),
                    max_acc
                ));
            }
            while let Some(pkt) = try_extract_frame(&mut acc, max_len)? {
                let now_ms: u64 = start_instant_uds_to_ws
                    .elapsed()
                    .as_millis()
                    .try_into()
                    .unwrap_or(u64::MAX);
                let decoded = decode_session_message(&pkt);
                {
                    let mut metrics = metrics_uds_to_ws.lock().await;
                    metrics.uds_to_ws_frames = metrics.uds_to_ws_frames.wrapping_add(1);
                    metrics.uds_to_ws_bytes = metrics
                        .uds_to_ws_bytes
                        .wrapping_add(pkt.len().try_into().unwrap_or(u64::MAX));

                    match decoded {
                        Ok((msg, ts, _)) => metrics.observe_message(conn_id, &msg, ts, now_ms),
                        Err(_) => {
                            metrics.decode_errors = metrics.decode_errors.wrapping_add(1);
                            metrics.touch_conn(conn_id, now_ms);
                        }
                    }
                }
                if out_tx_clone
                    .send(Message::Binary(pkt.into()))
                    .await
                    .is_err()
                {
                    return Ok::<(), anyhow::Error>(());
                }
            }
        }
        Ok::<(), anyhow::Error>(())
    });

    // WS -> UDS task: validate and forward binary frames
    let max_len_ws = state.max_frame_bytes;
    let pong_tx = out_tx.clone();
    let metrics_ws_to_uds = state.metrics.clone();
    let start_instant_ws_to_uds = state.start_instant;
    let ws_to_uds = tokio::spawn(async move {
        while let Some(msg) = ws_rx.next().await {
            match msg {
                Ok(Message::Binary(data)) => {
                    if let Err(err) = validate_frame(&data, max_len_ws) {
                        let now_ms: u64 = start_instant_ws_to_uds
                            .elapsed()
                            .as_millis()
                            .try_into()
                            .unwrap_or(u64::MAX);
                        let mut metrics = metrics_ws_to_uds.lock().await;
                        metrics.invalid_ws_frames = metrics.invalid_ws_frames.wrapping_add(1);
                        metrics.touch_conn(conn_id, now_ms);
                        warn!(?err, ?peer, "invalid frame from client");
                        break;
                    }

                    let now_ms: u64 = start_instant_ws_to_uds
                        .elapsed()
                        .as_millis()
                        .try_into()
                        .unwrap_or(u64::MAX);
                    let decoded = decode_session_message(&data);
                    {
                        let mut metrics = metrics_ws_to_uds.lock().await;
                        metrics.ws_to_uds_frames = metrics.ws_to_uds_frames.wrapping_add(1);
                        metrics.ws_to_uds_bytes = metrics
                            .ws_to_uds_bytes
                            .wrapping_add(data.len().try_into().unwrap_or(u64::MAX));

                        match decoded {
                            Ok((msg, ts, _)) => metrics.observe_message(conn_id, &msg, ts, now_ms),
                            Err(_) => {
                                metrics.decode_errors = metrics.decode_errors.wrapping_add(1);
                                metrics.touch_conn(conn_id, now_ms);
                            }
                        }
                    }
                    if let Err(err) = uds_writer.write_all(&data).await {
                        warn!(?err, "failed to write to uds");
                        break;
                    }
                }
                Ok(Message::Ping(payload)) => {
                    let _ = pong_tx.send(Message::Pong(payload)).await;
                }
                Ok(Message::Close(_)) => break,
                Ok(Message::Text(_)) => {
                    warn!(?peer, "ignoring text frame");
                    break;
                }
                Err(err) => {
                    warn!(?err, ?peer, "ws recv error");
                    break;
                }
                _ => {}
            }
        }
    });

    // Optional ping loop to keep connections alive.
    let ping_tx = out_tx.clone();
    let ping = tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(30));
        // tokio::time::interval() ticks immediately; discard the first tick so we
        // don't ping before the handshake has a chance to settle.
        interval.tick().await;
        loop {
            interval.tick().await;
            if ping_tx.send(Message::Ping(Bytes::new())).await.is_err() {
                break;
            }
        }
    });

    enum EndReason {
        Client(TaskResult<()>),
        Upstream(TaskResult<Result<(), anyhow::Error>>),
        Writer(TaskResult<()>),
    }

    let mut ws_to_uds = ws_to_uds;
    let mut uds_to_ws = uds_to_ws;
    let mut writer = writer;

    let reason: EndReason = tokio::select! {
        res = &mut ws_to_uds => EndReason::Client(res),
        res = &mut uds_to_ws => EndReason::Upstream(res),
        res = &mut writer => EndReason::Writer(res),
    };

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum EndKind {
        Client,
        Upstream,
        Writer,
    }

    let end_kind = match &reason {
        EndReason::Client(_) => EndKind::Client,
        EndReason::Upstream(_) => EndKind::Upstream,
        EndReason::Writer(_) => EndKind::Writer,
    };

    if matches!(end_kind, EndKind::Upstream) {
        warn!(?peer, "upstream disconnected; closing websocket");
        let _ = time::timeout(
            Duration::from_millis(250),
            out_tx.send(Message::Close(Some(axum::extract::ws::CloseFrame {
                code: axum::extract::ws::close_code::ERROR,
                reason: "upstream disconnected".into(),
            }))),
        )
        .await;
    }

    // Stop background tasks so handle_socket doesn't hang if one side exits early.
    ping.abort();
    ws_to_uds.abort();
    uds_to_ws.abort();
    drop(out_tx);

    // Best-effort flush for the close frame; force-cancel on slow/broken clients.
    if !matches!(end_kind, EndKind::Writer) {
        match time::timeout(Duration::from_secs(1), &mut writer).await {
            Ok(res) => log_void_task_result("writer", peer, res),
            Err(_) => {
                writer.abort();
                log_void_task_result("writer", peer, writer.await);
            }
        }
    }

    match reason {
        EndReason::Client(res) => log_void_task_result("ws_to_uds", peer, res),
        EndReason::Upstream(res) => log_result_task_result("uds_to_ws", peer, res),
        EndReason::Writer(res) => log_void_task_result("writer", peer, res),
    }

    // Await the aborted tasks to surface panics (cancellation is expected).
    log_void_task_result("ping", peer, ping.await);
    match end_kind {
        EndKind::Client => {
            log_result_task_result("uds_to_ws", peer, uds_to_ws.await);
        }
        EndKind::Upstream => {
            log_void_task_result("ws_to_uds", peer, ws_to_uds.await);
        }
        EndKind::Writer => {
            log_void_task_result("ws_to_uds", peer, ws_to_uds.await);
            log_result_task_result("uds_to_ws", peer, uds_to_ws.await);
        }
    }

    {
        let mut metrics = state.metrics.lock().await;
        metrics.remove_conn(conn_id);
    }
}

fn origin_allowed(state: &AppState, headers: &HeaderMap) -> bool {
    let Some(allow) = &state.allow_origins else {
        return true;
    };
    if let Some(origin) = headers.get("origin") {
        if let Ok(origin_str) = origin.to_str() {
            return allow.contains(origin_str);
        }
    }
    false
}

fn log_void_task_result(name: &'static str, peer: SocketAddr, res: TaskResult<()>) {
    match res {
        Ok(()) => {}
        Err(err) => log_join_error(name, peer, err),
    }
}

fn log_result_task_result(
    name: &'static str,
    peer: SocketAddr,
    res: TaskResult<Result<(), anyhow::Error>>,
) {
    match res {
        Ok(Ok(())) => {}
        Ok(Err(err)) => warn!(?peer, ?err, "{name} task returned error"),
        Err(err) => log_join_error(name, peer, err),
    }
}

fn log_join_error(name: &'static str, peer: SocketAddr, err: JoinError) {
    if err.is_cancelled() {
        return;
    }
    if err.is_panic() {
        error!(?peer, ?err, "{name} task panicked");
    } else {
        warn!(?peer, ?err, "{name} task failed");
    }
}

fn validate_frame(buf: &[u8], max: usize) -> Result<()> {
    let Some(frame_len) = try_frame_len(buf, max)? else {
        return Err(anyhow!("frame too small"));
    };
    if buf.len() != frame_len {
        return Err(anyhow!("frame length mismatch"));
    }
    Ok(())
}

fn try_extract_frame(acc: &mut Vec<u8>, max: usize) -> Result<Option<Vec<u8>>> {
    let Some(frame_len) = try_frame_len(acc, max)? else {
        return Ok(None);
    };
    if acc.len() < frame_len {
        return Ok(None);
    }
    let pkt: Vec<u8> = acc.drain(..frame_len).collect();
    Ok(Some(pkt))
}

fn try_frame_len(buf: &[u8], max_payload: usize) -> Result<Option<usize>> {
    if buf.len() < JS_ABI_HEADER_BYTES {
        return Ok(None);
    }
    let payload_len = u32::from_be_bytes([buf[8], buf[9], buf[10], buf[11]]) as usize;
    if payload_len > max_payload {
        return Err(anyhow!("payload too large"));
    }
    let frame_len = JS_ABI_HEADER_BYTES
        .checked_add(payload_len)
        .and_then(|v| v.checked_add(JS_ABI_HASH_BYTES))
        .ok_or_else(|| anyhow!("frame length overflow"))?;
    Ok(Some(frame_len))
}

async fn load_tls(cert_path: PathBuf, key_path: PathBuf) -> Result<RustlsConfig> {
    let cfg = RustlsConfig::from_pem_file(cert_path, key_path).await?;
    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_frame(payload_len: usize) -> Vec<u8> {
        let mut buf = vec![0u8; JS_ABI_HEADER_BYTES + payload_len + JS_ABI_HASH_BYTES];
        buf[8..12].copy_from_slice(&(payload_len as u32).to_be_bytes());
        buf
    }

    #[test]
    fn validate_frame_accepts_exact_frame_len() {
        let frame = make_frame(5);
        validate_frame(&frame, 5).expect("valid frame");
    }

    #[test]
    fn validate_frame_rejects_len_mismatch() {
        let mut frame = make_frame(5);
        frame.push(0u8);
        let err = validate_frame(&frame, 5).expect_err("expected mismatch");
        assert!(err.to_string().contains("length mismatch"));
    }

    #[test]
    fn validate_frame_rejects_too_small_buffer() {
        let err = validate_frame(&[], 5).expect_err("expected too-small error");
        assert!(err.to_string().contains("too small"));
    }

    #[test]
    fn validate_frame_rejects_payload_too_large() {
        let frame = make_frame(6);
        let err = validate_frame(&frame, 5).expect_err("expected payload-too-large error");
        assert!(err.to_string().contains("payload too large"));
    }

    #[test]
    fn try_extract_frame_drains_one_frame_and_preserves_remainder() {
        let f1 = make_frame(2);
        let f2 = make_frame(3);
        let mut acc = [f1.clone(), f2.clone()].concat();

        let pkt1 = try_extract_frame(&mut acc, 3).unwrap().expect("pkt1");
        assert_eq!(pkt1, f1);
        assert_eq!(acc, f2);

        let pkt2 = try_extract_frame(&mut acc, 3).unwrap().expect("pkt2");
        assert_eq!(pkt2, f2);
        assert!(acc.is_empty());
    }

    #[test]
    fn try_extract_frame_returns_none_for_partial_header() {
        let mut acc = vec![0u8; JS_ABI_HEADER_BYTES - 1];
        let pkt = try_extract_frame(&mut acc, 3).unwrap();
        assert!(pkt.is_none());
        assert_eq!(acc.len(), JS_ABI_HEADER_BYTES - 1);
    }

    #[test]
    fn try_extract_frame_returns_none_for_partial_frame() {
        let full = make_frame(3);
        let mut acc = full[..full.len() - 1].to_vec();
        let pkt = try_extract_frame(&mut acc, 3).unwrap();
        assert!(pkt.is_none());
        assert_eq!(acc.len(), full.len() - 1);
    }

    #[test]
    fn try_extract_frame_errors_on_payload_too_large_without_draining() {
        let mut acc = make_frame(6);
        let err = try_extract_frame(&mut acc, 5).expect_err("expected payload-too-large error");
        assert!(err.to_string().contains("payload too large"));
        assert_eq!(acc.len(), JS_ABI_HEADER_BYTES + 6 + JS_ABI_HASH_BYTES);
    }
}
