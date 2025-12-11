// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! WebSocket ↔ Unix socket bridge for the Echo session service.
//! Browsers speak WebSocket; the bridge forwards binary JS-ABI frames to the Unix bus.

use std::{collections::HashSet, net::SocketAddr, path::PathBuf, sync::Arc};

use anyhow::{anyhow, Context, Result};
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
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
    sync::mpsc,
    time::{self, Duration},
};
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

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
        warn!(?addr, "origin rejected");
        return StatusCode::FORBIDDEN.into_response();
    }
    ws.on_upgrade(move |socket| handle_socket(socket, state, addr))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>, peer: SocketAddr) {
    let unix = match UnixStream::connect(&state.unix_socket).await {
        Ok(stream) => stream,
        Err(err) => {
            error!(?err, "failed to connect to unix socket");
            let _ = socket
                .send(Message::Close(Some(axum::extract::ws::CloseFrame {
                    code: axum::extract::ws::close_code::ERROR,
                    reason: "upstream unavailable".into(),
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
        loop {
            let n = uds_reader.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            acc.extend_from_slice(&buf[..n]);
            while let Some(pkt) = try_extract_frame(&mut acc, max_len)? {
                if out_tx_clone.send(Message::Binary(pkt)).await.is_err() {
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

    // Optional ping loop to keep connections alive
    let ping_tx = out_tx.clone();
    let ping = tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            if ping_tx.send(Message::Ping(Vec::new())).await.is_err() {
                break;
            }
        }
    });

    let _ = tokio::join!(ws_to_uds, uds_to_ws, writer, ping);
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

fn validate_frame(buf: &[u8], max: usize) -> Result<()> {
    if buf.len() < 12 {
        return Err(anyhow!("frame too small"));
    }
    let len = u32::from_be_bytes([buf[8], buf[9], buf[10], buf[11]]) as usize;
    if len > max {
        return Err(anyhow!("payload too large"));
    }
    let frame_len = 12usize
        .checked_add(len)
        .and_then(|v| v.checked_add(32))
        .ok_or_else(|| anyhow!("frame length overflow"))?;
    if buf.len() != frame_len {
        return Err(anyhow!("frame length mismatch"));
    }
    Ok(())
}

fn try_extract_frame(acc: &mut Vec<u8>, max: usize) -> Result<Option<Vec<u8>>> {
    if acc.len() < 12 {
        return Ok(None);
    }
    let len = u32::from_be_bytes([acc[8], acc[9], acc[10], acc[11]]) as usize;
    if len > max {
        return Err(anyhow!("payload too large"));
    }
    let frame_len = 12usize
        .checked_add(len)
        .and_then(|v| v.checked_add(32))
        .ok_or_else(|| anyhow!("frame length overflow"))?;
    if acc.len() < frame_len {
        return Ok(None);
    }
    let pkt: Vec<u8> = acc.drain(..frame_len).collect();
    Ok(Some(pkt))
}

async fn load_tls(cert_path: PathBuf, key_path: PathBuf) -> Result<RustlsConfig> {
    let cfg = RustlsConfig::from_pem_file(cert_path, key_path).await?;
    Ok(cfg)
}
