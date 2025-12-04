// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Skeleton headless session hub. Networking to be added later.

use anyhow::Result;
use echo_session_proto::{Message, Notification, NotifyKind, NotifyScope};

fn main() -> Result<()> {
    // Placeholder: in the future this will spin up the socket server and host the session core.
    println!("echo-session-service: skeleton hub running (no transport yet)");

    // Emit a dummy notification to show shape.
    let _ = Message::Notification(Notification {
        kind: NotifyKind::Info,
        scope: NotifyScope::Global,
        title: "Session service stub".into(),
        body: Some("Transport not implemented yet.".into()),
    });

    Ok(())
}
