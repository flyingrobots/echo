// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Session wire schema for Echo hub (WARP snapshots/diffs + notifications).
//! WARP frames use the canonical `echo-graph` types and are transported in
//! deterministic JS-ABI v1.0 OpEnvelopes (ADR/ARCH-0013).

pub use echo_graph::{
    EdgeId, EpochId, Hash32, NodeId, RenderEdge, RenderGraph, RenderNode, WarpDiff, WarpFrame,
    WarpHello, WarpId, WarpOp, WarpSnapshot,
};
mod canonical;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

/// Default Unix socket path for the session hub.
///
/// Prefers a per-user runtime dir (XDG_RUNTIME_DIR) and falls back to `/tmp`
/// when unavailable.
pub fn default_socket_path() -> PathBuf {
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    base.join("echo-session.sock")
}

/// Canonical OpEnvelope carried as the payload of a JS-ABI packet.
///
/// * `op` – operation name (see ADR-0013).
/// * `ts` – logical timestamp (authoritative on the server side).
/// * `payload` – operation specific body.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OpEnvelope<P> {
    /// Operation name (e.g., "handshake", "handshake_ack", "error", "warp_stream").
    pub op: String,
    /// Logical timestamp (monotonic per-host clock).
    pub ts: u64,
    /// Operation-specific body.
    pub payload: P,
}

/// Logical session identifier (stub; adjust when multi-session is added).
pub type SessionId = u64;

/// Notification severity for session bus.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum NotifyKind {
    /// Informational notification.
    Info,
    /// Warning notification.
    Warn,
    /// Error notification.
    Error,
}

/// Scope for notifications.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NotifyScope {
    /// Visible to all participants.
    Global,
    /// Scoped to a specific session.
    Session(SessionId),
    /// Scoped to a specific WARP stream.
    Warp(WarpId),
    /// Local-only to the emitting tool.
    Local,
}

/// Notification broadcast frame.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Notification {
    /// Severity of the notification.
    pub kind: NotifyKind,
    /// Delivery scope.
    pub scope: NotifyScope,
    /// Short title line.
    pub title: String,
    /// Optional details.
    pub body: Option<String>,
}

/// Client kind (for logging / policy; optional for now).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClientKind {
    /// WARP viewer / tool.
    Viewer,
    /// Engine or producer of authoritative WARP.
    Engine,
    /// Other tool.
    Tool,
}

/// Error payload used in error and handshake_ack responses.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ErrorPayload {
    /// Numeric error code (e.g., 1, 2, 500).
    pub code: u32,
    /// Stable identifier (e.g., "E_INVALID_OP").
    pub name: String,
    /// Optional machine-readable details.
    pub details: Option<ciborium::value::Value>,
    /// Human readable message.
    pub message: String,
}

impl serde::Serialize for ErrorPayload {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut m = serializer.serialize_map(Some(4))?;
        m.serialize_entry("code", &self.code)?;
        m.serialize_entry("name", &self.name)?;
        m.serialize_entry("details", &self.details)?;
        m.serialize_entry("message", &self.message)?;
        m.end()
    }
}

/// Handshake request payload (client → host).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HandshakePayload {
    /// Optional agent identifier.
    pub agent_id: Option<String>,
    /// Capability identifiers (e.g., "compression:zstd").
    pub capabilities: Vec<String>,
    /// Implementation version (not wire version).
    pub client_version: u32,
    /// Optional free-form session metadata.
    pub session_meta: Option<BTreeMap<String, ciborium::value::Value>>,
}

/// Handshake acknowledgement payload (host → client).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HandshakeAckPayload {
    /// Status of the handshake.
    pub status: AckStatus,
    /// Server implementation version (not wire version).
    pub server_version: u32,
    /// Capabilities enabled for this session.
    pub capabilities: Vec<String>,
    /// Session identifier.
    pub session_id: String,
    /// Optional error payload when status == Error.
    pub error: Option<ErrorPayload>,
}

/// Subscribe payload (consumer → host).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubscribeWarpPayload {
    /// Identifier of the WARP stream to receive.
    #[serde(alias = "rmg_id")]
    pub warp_id: WarpId,
}

/// WARP stream payload (producer/host → consumers).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WarpStreamPayload {
    /// Stream identifier.
    #[serde(alias = "rmg_id")]
    pub warp_id: WarpId,
    /// Snapshot or diff.
    pub frame: WarpFrame,
}

/// Status enumeration for handshake ack.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AckStatus {
    /// Handshake succeeded.
    #[serde(rename = "OK")]
    Ok,
    /// Handshake failed.
    #[serde(rename = "ERROR")]
    Error,
}

/// Wire message kinds carried inside OpEnvelope payloads.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Message {
    /// Handshake request (op = "handshake").
    Handshake(HandshakePayload),
    /// Handshake acknowledgement (op = "handshake_ack").
    HandshakeAck(HandshakeAckPayload),
    /// Protocol or processing error (op = "error").
    Error(ErrorPayload),
    /// Subscribe to a specific WARP stream (op = "subscribe_warp").
    SubscribeWarp {
        /// Identifier of the WARP stream to receive.
        warp_id: WarpId,
    },
    /// WARP state frame (snapshot or diff) for a specific stream (op = "warp_stream").
    WarpStream {
        /// Stream identifier.
        warp_id: WarpId,
        /// Snapshot or diff.
        frame: WarpFrame,
    },
    /// Notification broadcast (op = "notification").
    Notification(Notification),
}

impl Message {
    /// Canonical op string for this message variant.
    pub fn op_name(&self) -> &'static str {
        match self {
            Message::Handshake(_) => "handshake",
            Message::HandshakeAck(_) => "handshake_ack",
            Message::Error(_) => "error",
            Message::SubscribeWarp { .. } => "subscribe_warp",
            Message::WarpStream { .. } => "warp_stream",
            Message::Notification(_) => "notification",
        }
    }
}

pub mod wire;
