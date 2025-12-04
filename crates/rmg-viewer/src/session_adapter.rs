// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Notification source adapter for the session hub.
//! Provides a best-effort Unix socket CBOR listener; falls back to a no-op.

use echo_session_proto::{wire, Message, Notification};
use std::io::Read;
use std::os::unix::net::UnixStream;
use std::sync::mpsc::{self, Receiver};
use std::thread;

const DEFAULT_SOCK: &str = "/tmp/echo-session.sock";

/// Connect to the session hub notification stream. If unavailable, returns an empty receiver.
pub fn connect_default() -> Receiver<Notification> {
    let (tx, rx) = mpsc::channel();
    let path = DEFAULT_SOCK.to_string();
    thread::spawn(move || {
        if let Ok(mut stream) = UnixStream::connect(path) {
            loop {
                let mut len_buf = [0u8; 4];
                if stream.read_exact(&mut len_buf).is_err() {
                    break;
                }
                let len = u32::from_be_bytes(len_buf) as usize;
                let mut body = vec![0u8; len];
                if stream.read_exact(&mut body).is_err() {
                    break;
                }
                if let Ok(Message::Notification(n)) = wire::from_cbor(&body) {
                    let _ = tx.send(n);
                }
            }
        }
    });
    rx
}
