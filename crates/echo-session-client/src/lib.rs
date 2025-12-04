// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Client helper for talking to the Echo session hub over Unix sockets (CBOR-framed).

use anyhow::Result;
use echo_session_proto::{wire::Packet, Command, Message, Notification, RmgDiff, RmgSnapshot};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

/// Minimal async client over Unix sockets.
pub struct SessionClient {
    stream: UnixStream,
}

impl SessionClient {
    /// Connect to the hub at the given Unix socket path.
    pub async fn connect(path: &str) -> Result<Self> {
        let stream = UnixStream::connect(path).await?;
        Ok(Self { stream })
    }

    /// Publish an RMG diff.
    pub async fn publish_diff(&mut self, diff: RmgDiff) -> Result<()> {
        let packet = Packet::encode(&Message::RmgDiff(diff))?;
        self.stream.write_all(&packet).await?;
        Ok(())
    }

    /// Publish an RMG snapshot.
    pub async fn publish_snapshot(&mut self, snap: RmgSnapshot) -> Result<()> {
        let packet = Packet::encode(&Message::RmgSnapshot(snap))?;
        self.stream.write_all(&packet).await?;
        Ok(())
    }

    /// Send a command to the hub.
    pub async fn send_command(&mut self, cmd: Command) -> Result<()> {
        let packet = Packet::encode(&Message::Command(cmd))?;
        self.stream.write_all(&packet).await?;
        Ok(())
    }

    /// Poll a single message if available (non-blocking). Returns Ok(None) when no complete frame is present.
    pub async fn poll_message(&mut self) -> Result<Option<Message>> {
        let mut len_buf = [0u8; 4];
        let n = self.stream.read(&mut len_buf).await?;
        if n == 0 {
            return Ok(None);
        }
        if n < 4 {
            // simplistic handling: treat incomplete header as no message
            return Ok(None);
        }
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut body = vec![0u8; len];
        self.stream.read_exact(&mut body).await?;
        let msg = echo_session_proto::wire::from_cbor(&body)?;
        Ok(Some(msg))
    }

    /// Convenience: drain messages until none are immediately available.
    pub async fn poll_notifications(&mut self) -> Result<Vec<Notification>> {
        let mut out = Vec::new();
        while let Some(msg) = self.poll_message().await? {
            if let Message::Notification(n) = msg {
                out.push(n);
            }
        }
        Ok(out)
    }

    /// Expose the underlying stream (e.g., for select!).
    pub fn stream(&mut self) -> &mut UnixStream {
        &mut self.stream
    }
}
