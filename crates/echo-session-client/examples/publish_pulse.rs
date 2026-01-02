// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Minimal CLI publisher for exercising the session hub / WVP pipeline.
//!
//! This example connects to the Unix socket session hub, sends a handshake,
//! subscribes to a `warp_id`, then publishes:
//! - a snapshot at epoch 0
//! - N empty diffs (gapless epochs)
//!
//! It exists purely to make it easy to verify that `echo-session-service` +
//! `echo-session-ws-gateway` + `/dashboard` are alive without needing to run the
//! full GUI viewer.

use anyhow::{Context, Result};
use echo_session_proto::{
    wire::encode_message, HandshakePayload, Message, RenderGraph, WarpDiff, WarpFrame, WarpId,
    WarpSnapshot,
};
use std::io::Write;
use std::os::unix::net::UnixStream;
use std::time::Duration;

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let socket_path = args
        .next()
        .unwrap_or_else(|| "/tmp/echo-session.sock".to_string());
    let warp_id: WarpId = args
        .next()
        .as_deref()
        .unwrap_or("1")
        .parse()
        .context("parse warp_id")?;
    let diffs: u64 = args
        .next()
        .as_deref()
        .unwrap_or("5")
        .parse()
        .context("parse diffs")?;
    let delay_ms: u64 = args
        .next()
        .as_deref()
        .unwrap_or("250")
        .parse()
        .context("parse delay_ms")?;

    let mut stream =
        UnixStream::connect(&socket_path).with_context(|| format!("connect {socket_path}"))?;

    // Best-effort: reduce latency if supported.
    let _ = stream.set_nonblocking(false);

    let pkt = encode_message(
        Message::Handshake(HandshakePayload {
            client_version: 1,
            capabilities: vec!["demo:pulse".into()],
            agent_id: Some("echo-session-client-example:publish_pulse".into()),
            session_meta: None,
        }),
        0,
    )
    .context("encode handshake")?;
    stream.write_all(&pkt).context("write handshake")?;

    let pkt = encode_message(Message::SubscribeWarp { warp_id }, 0).context("encode subscribe")?;
    stream.write_all(&pkt).context("write subscribe")?;

    let snapshot = WarpFrame::Snapshot(WarpSnapshot {
        epoch: 0,
        graph: RenderGraph::default(),
        state_hash: None,
    });
    let pkt = encode_message(
        Message::WarpStream {
            warp_id,
            frame: snapshot,
        },
        0,
    )
    .context("encode snapshot")?;
    stream.write_all(&pkt).context("write snapshot")?;

    for i in 0..diffs {
        let diff = WarpFrame::Diff(WarpDiff {
            from_epoch: i,
            to_epoch: i.saturating_add(1),
            ops: vec![],
            state_hash: None,
        });
        let pkt = encode_message(
            Message::WarpStream {
                warp_id,
                frame: diff,
            },
            0,
        )
        .context("encode diff")?;
        stream.write_all(&pkt).context("write diff")?;
        std::thread::sleep(Duration::from_millis(delay_ms));
    }

    Ok(())
}
