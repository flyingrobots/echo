// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Session wire schema for Echo hub (RMG snapshots/diffs + notifications).
//! RMG frames use the canonical `echo-graph` types.

pub use echo_graph::*;
use serde::{Deserialize, Serialize};

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
    /// Scoped to a specific RMG stream.
    Rmg(RmgId),
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
    /// RMG viewer / tool.
    Viewer,
    /// Engine or producer of authoritative RMG.
    Engine,
    /// Other tool.
    Tool,
}

/// Wire envelope (matches earlier message enum; keep for compat and extend for subscriptions).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Message {
    /// Client hello / version info.
    Hello {
        /// Protocol version.
        protocol_version: u16,
        /// Declared client kind.
        client_kind: ClientKind,
    },
    /// Subscribe to a specific RMG stream.
    SubscribeRmg {
        /// Identifier of the RMG stream to receive.
        rmg_id: RmgId,
    },
    /// RMG state frame (snapshot or diff) for a specific stream.
    RmgStream {
        /// Stream identifier.
        rmg_id: RmgId,
        /// Snapshot or diff.
        frame: RmgFrame,
    },
    /// Notification broadcast.
    Notification(Notification),
}

pub mod wire;
