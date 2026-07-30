// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Bounded child-process execution for filesystem recovery fixtures.

use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

const CHILD_PHASE_TIMEOUT: Duration = Duration::from_secs(30);
const CHILD_PHASE_POLL_INTERVAL: Duration = Duration::from_millis(10);

/// Runs one ignored integration-test entrypoint with a fixed timeout.
pub fn run_child_phase(
    executable: &Path,
    test_name: &str,
    phase: &str,
    root_env_key: &str,
    root_env_value: &OsStr,
    phase_env_key: &str,
    cleanup_root: &Path,
) {
    let mut child = match Command::new(executable)
        .args([
            "--ignored",
            "--exact",
            test_name,
            "--nocapture",
            "--test-threads=1",
        ])
        .env(root_env_key, root_env_value)
        .env(phase_env_key, phase)
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            let _ = fs::remove_dir_all(cleanup_root);
            panic!("child phase `{phase}` failed to spawn: {error}");
        }
    };
    let deadline = Instant::now() + CHILD_PHASE_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return,
            Ok(Some(status)) => {
                let _ = fs::remove_dir_all(cleanup_root);
                panic!("child phase `{phase}` failed: {status}");
            }
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = fs::remove_dir_all(cleanup_root);
                panic!(
                    "child phase `{phase}` exceeded the {}s deadline",
                    CHILD_PHASE_TIMEOUT.as_secs()
                );
            }
            Ok(None) => thread::sleep(CHILD_PHASE_POLL_INTERVAL),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = fs::remove_dir_all(cleanup_root);
                panic!("child phase `{phase}` wait failed: {error}");
            }
        }
    }
}
