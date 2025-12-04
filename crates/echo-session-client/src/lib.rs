// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Client helper for talking to the Echo session hub (skeleton).
//! Transport is intentionally abstract; today it is a no-op stub.

use anyhow::Result;
use echo_session_proto::{Command, Message, Notification, RmgDiff, RmgSnapshot};

/// Minimal client placeholder. Replace with real transport (Unix socket / TCP) later.
pub struct SessionClient {
    endpoint: String,
}

impl SessionClient {
    /// Create a stub client targeting a given endpoint string (not used yet).
    pub fn connect(endpoint: impl Into<String>) -> Result<Self> {
        Ok(Self {
            endpoint: endpoint.into(),
        })
    }

    /// Publish an RMG diff (stub).
    pub fn publish_diff(&self, _diff: RmgDiff) -> Result<()> {
        Ok(())
    }

    /// Publish an RMG snapshot (stub).
    pub fn publish_snapshot(&self, _snap: RmgSnapshot) -> Result<()> {
        Ok(())
    }

    /// Send a command to the hub (stub).
    pub fn send_command(&self, _cmd: Command) -> Result<()> {
        Ok(())
    }

    /// Fetch any pending notifications (stub returns empty).
    pub fn poll_notifications(&self) -> Result<Vec<Notification>> {
        Ok(Vec::new())
    }

    /// Fetch any pending messages (stub returns empty).
    pub fn poll_messages(&self) -> Result<Vec<Message>> {
        Ok(Vec::new())
    }

    /// Endpoint string (for debugging).
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }
}
