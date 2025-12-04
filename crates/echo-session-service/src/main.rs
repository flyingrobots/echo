// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Minimal Unix-socket CBOR hub skeleton.

use anyhow::Result;
use echo_session_proto::{wire::Packet, Message, Notification, NotifyKind, NotifyScope};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tracing::{info, warn};

const SOCKET_PATH: &str = "/tmp/echo-session.sock";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Remove stale socket if present
    let _ = std::fs::remove_file(SOCKET_PATH);
    let listener = UnixListener::bind(SOCKET_PATH)?;
    info!("session hub listening at {}", SOCKET_PATH);

    loop {
        let (stream, _) = listener.accept().await?;
        tokio::spawn(async move {
            if let Err(err) = handle_client(stream).await {
                warn!(?err, "client handler error");
            }
        });
    }
}

async fn handle_client(mut stream: UnixStream) -> Result<()> {
    // Send a hello notification on connect (stub)
    let hello = Message::Notification(Notification {
        kind: NotifyKind::Info,
        scope: NotifyScope::Global,
        title: "session-service".into(),
        body: Some("stub transport online".into()),
    });
    let packet = Packet::encode(&hello)?;
    stream.write_all(&packet).await?;

    let mut buf = vec![0u8; 16 * 1024];
    loop {
        let n = stream.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        let slice = &buf[..n];
        match Packet::decode(slice) {
            Ok((msg, _used)) => {
                info!(?msg, "received message (stub no-op)");
            }
            Err(err) => {
                warn!(?err, "failed to decode packet");
            }
        }
    }
    Ok(())
}
