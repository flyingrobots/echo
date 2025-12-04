// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! RMG stream adapter (Unix socket, CBOR-framed). Best-effort placeholder until real decoding.

use echo_session_proto::{wire, Message};
use std::io::Read;
use std::os::unix::net::UnixStream;
use std::sync::mpsc::{self, Receiver};
use std::thread;

const DEFAULT_SOCK: &str = "/tmp/echo-session.sock";

/// Simplified frame representing incoming RMG data.
#[derive(Debug, Clone)]
pub struct RmgFrame {
    pub revision: u64,
    #[allow(dead_code)]
    pub bytes: Vec<u8>,
}

/// Connect to the session hub and stream RMG snapshots/diffs.
pub fn connect_default() -> Receiver<RmgFrame> {
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
                if let Ok(msg) = wire::from_cbor(&body) {
                    match msg {
                        Message::RmgSnapshot(s) => {
                            let _ = tx.send(RmgFrame {
                                revision: s.revision,
                                bytes: s.bytes,
                            });
                        }
                        Message::RmgDiff(d) => {
                            let _ = tx.send(RmgFrame {
                                revision: d.to_rev,
                                bytes: d.bytes,
                            });
                        }
                        _ => {}
                    }
                }
            }
        }
    });
    rx
}
