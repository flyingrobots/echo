// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Study probe, not for merge: records every whole-state root computation.
//!
//! `host_test` only. Each record is the reachable node count and the elapsed
//! time of one computation. Tests read and reset the log around the phase
//! they measure.

use std::sync::Mutex;
use std::time::Duration;

static LOG: Mutex<Vec<(usize, Duration)>> = Mutex::new(Vec::new());

pub(crate) fn record(reachable: usize, elapsed: Duration) {
    if let Ok(mut log) = LOG.lock() {
        log.push((reachable, elapsed));
    }
}

/// Removes and returns every record since the last call.
pub fn take() -> Vec<(usize, Duration)> {
    LOG.lock()
        .map(|mut log| std::mem::take(&mut *log))
        .unwrap_or_default()
}
