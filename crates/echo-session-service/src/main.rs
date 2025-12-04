// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Minimal Unix-socket CBOR hub skeleton.

use anyhow::Result;
use echo_graph::{
    EdgeData, EdgeKind, NodeData, NodeDataPatch, NodeKind, RenderGraph, RenderNode, RmgDiff,
    RmgFrame, RmgOp, RmgSnapshot,
};
use echo_session_proto::{wire::Packet, Message, Notification, NotifyKind, NotifyScope};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::{info, warn};

const SOCKET_PATH: &str = "/tmp/echo-session.sock";

#[derive(Clone, Default)]
struct GraphState {
    graph: RenderGraph,
    epoch: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let state = Arc::new(Mutex::new(seed_graph()));

    // Remove stale socket if present
    let _ = std::fs::remove_file(SOCKET_PATH);
    let listener = UnixListener::bind(SOCKET_PATH)?;
    info!("session hub listening at {}", SOCKET_PATH);

    loop {
        let (stream, _) = listener.accept().await?;
        let shared = state.clone();
        tokio::spawn(async move {
            if let Err(err) = handle_client(stream, shared).await {
                warn!(?err, "client handler error");
            }
        });
    }
}

async fn handle_client(mut stream: UnixStream, shared: Arc<Mutex<GraphState>>) -> Result<()> {
    // Send a hello notification on connect (stub)
    let hello = Message::Notification(Notification {
        kind: NotifyKind::Info,
        scope: NotifyScope::Global,
        title: "session-service".into(),
        body: Some("stub transport online".into()),
    });
    let packet = Packet::encode(&hello)?;
    stream.write_all(&packet).await?;

    // Send initial snapshot
    {
        let state = shared.lock().await;
        let snap = RmgSnapshot {
            epoch: state.epoch,
            graph: state.graph.clone(),
            state_hash: Some(state.graph.compute_hash()),
        };
        let frame = Message::Rmg(RmgFrame::Snapshot(snap));
        let pkt = Packet::encode(&frame)?;
        stream.write_all(&pkt).await?;
    }

    // Spawn a writer that emits gapless diffs for this client
    let writer_state = shared.clone();
    tokio::spawn(async move {
        if let Err(err) = emit_diffs(stream, writer_state).await {
            warn!(?err, "writer loop error");
        }
    });

    Ok(())
}

async fn emit_diffs(mut stream: UnixStream, shared: Arc<Mutex<GraphState>>) -> Result<()> {
    loop {
        sleep(Duration::from_millis(500)).await;
        let mut state = shared.lock().await;

        // Increment a simple counter in node 1's payload to simulate updates
        state.epoch += 1;
        let from = state.epoch - 1;
        let to = state.epoch;

        let mut counter_bytes = [0u8; 8];
        counter_bytes.copy_from_slice(&to.to_le_bytes());
        let op = RmgOp::UpdateNode {
            id: 1,
            data: NodeDataPatch::Replace(NodeData {
                raw: counter_bytes.to_vec(),
            }),
        };
        state.graph.apply_op(op.clone())?;
        let diff = RmgDiff {
            from_epoch: from,
            to_epoch: to,
            ops: vec![op],
            state_hash: Some(state.graph.compute_hash()),
        };
        let frame = Message::Rmg(RmgFrame::Diff(diff));
        let pkt = Packet::encode(&frame)?;
        stream.write_all(&pkt).await?;
    }
}

fn seed_graph() -> GraphState {
    let mut graph = RenderGraph::default();
    graph.nodes.push(RenderNode {
        id: 1,
        kind: NodeKind::Generic,
        data: NodeData {
            raw: b"seed".to_vec(),
        },
    });
    graph.nodes.push(RenderNode {
        id: 2,
        kind: NodeKind::Generic,
        data: NodeData {
            raw: b"leaf".to_vec(),
        },
    });
    graph.edges.push(echo_graph::RenderEdge {
        id: 1,
        src: 1,
        dst: 2,
        kind: EdgeKind::Generic,
        data: EdgeData { raw: Vec::new() },
    });
    GraphState { graph, epoch: 0 }
}
