// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Wire schema for the Echo session hub (RMG streams, commands, notifications).
//! Transport-agnostic; serialized with serde by adapters.

use serde::{Deserialize, Serialize};

/// Logical identifier for a session.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub String);

/// Logical identifier for an RMG authority/stream.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RmgId(pub String);

/// Role a participant declares when registering an RMG.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RmgRole {
    /// Publishes authoritative diffs/snapshots.
    Publisher,
    /// Subscribes for viewing/inspection only.
    Subscriber,
}

/// Severity for notifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotifyKind {
    /// Informational event.
    Info,
    /// Warning that may need attention.
    Warn,
    /// Error requiring awareness/action.
    Error,
}

/// Initial hello/handshake.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hello {
    /// Human-readable tool name/version (e.g., "rmg-viewer/0.1").
    pub agent: String,
    /// Protocol version for compatibility checks.
    pub proto_version: u16,
}

/// Register an RMG stream with the hub.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRmg {
    /// Target RMG identifier.
    pub rmg: RmgId,
    /// Desired role (publisher or subscriber).
    pub role: RmgRole,
}

/// RMG state snapshot (opaque payload defined by core/engine).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RmgSnapshot {
    /// RMG identifier.
    pub rmg: RmgId,
    /// Snapshot revision number.
    pub revision: u64,
    /// Opaque serialized snapshot bytes.
    pub bytes: Vec<u8>,
}

/// RMG diff/incremental update.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RmgDiff {
    /// RMG identifier.
    pub rmg: RmgId,
    /// Base revision.
    pub from_rev: u64,
    /// Target revision.
    pub to_rev: u64,
    /// Opaque serialized diff bytes.
    pub bytes: Vec<u8>,
}

/// Notification broadcast.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    /// Severity.
    pub kind: NotifyKind,
    /// Scope for delivery.
    pub scope: NotifyScope,
    /// Short title line.
    pub title: String,
    /// Optional detail text.
    pub body: Option<String>,
}

/// Scope for notifications.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotifyScope {
    /// Visible to all participants.
    Global,
    /// Scoped to a specific session.
    Session(SessionId),
    /// Scoped to a specific RMG.
    Rmg(RmgId),
    /// Local-only (originating tool decides visibility).
    Local,
}

/// Command sent to an RMG authority.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    /// RMG identifier.
    pub rmg: RmgId,
    /// Opaque serialized command payload.
    pub bytes: Vec<u8>,
}

/// Command acknowledgment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandAck {
    /// RMG identifier.
    pub rmg: RmgId,
    /// Whether the command succeeded.
    pub ok: bool,
    /// Optional human-readable message.
    pub message: Option<String>,
}

/// Envelope for all protocol messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    /// Initial handshake.
    Hello(Hello),
    /// Register an RMG stream.
    RegisterRmg(RegisterRmg),
    /// Full snapshot push.
    RmgSnapshot(RmgSnapshot),
    /// Incremental diff.
    RmgDiff(RmgDiff),
    /// Notification broadcast.
    Notification(Notification),
    /// Command from a client.
    Command(Command),
    /// Command acknowledgment.
    CommandAck(CommandAck),
}

pub mod wire;
