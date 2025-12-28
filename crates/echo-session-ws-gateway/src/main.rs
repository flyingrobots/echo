// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! WebSocket ↔ Unix socket bridge for the Echo session service.
//! Browsers speak WebSocket; the bridge forwards binary JS-ABI frames to the Unix bus.

use std::{collections::HashSet, net::SocketAddr, path::PathBuf, sync::Arc};

use anyhow::{anyhow, Context, Result};
use axum::body::Bytes;
use axum::{
    extract::ws::{Message, WebSocket},
    extract::{ConnectInfo, State, WebSocketUpgrade},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use axum_server::{tls_rustls::RustlsConfig, Handle};
use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use tokio::task::JoinError;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
    sync::mpsc,
    time::{self, Duration},
};
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

const JS_ABI_HEADER_BYTES: usize = 12;
const JS_ABI_HASH_BYTES: usize = 32;
const JS_ABI_OVERHEAD_BYTES: usize = JS_ABI_HEADER_BYTES + JS_ABI_HASH_BYTES;
type TaskResult<T> = std::result::Result<T, JoinError>;

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

    let state = Arc::new(AppState {
        unix_socket: args.unix_socket.clone(),
        max_frame_bytes: args.max_frame_bytes,
        allow_origins,
    });

    let app = Router::new()
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
    let ws_to_uds = tokio::spawn(async move {
        while let Some(msg) = ws_rx.next().await {
            match msg {
                Ok(Message::Binary(data)) => {
                    if let Err(err) = validate_frame(&data, max_len_ws) {
                        warn!(?err, ?peer, "invalid frame from client");
                        break;
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
}
