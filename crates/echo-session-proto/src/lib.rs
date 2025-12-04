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

/// Wire envelope (matches earlier message enum; keep for compat).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Message {
    /// RMG state frame (snapshot or diff).
    Rmg(RmgFrame),
    /// Notification broadcast.
    Notification(Notification),
}

pub mod wire;
