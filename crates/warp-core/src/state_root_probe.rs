// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Study probe, not for merge: records every whole-state root computation.
//!
//! `host_test` only. Each record is the reachable node count and the elapsed
//! time of one computation. Tests read and reset the log around the phase
//! they measure.

use std::cell::RefCell;
use std::time::Duration;

thread_local! {
    // Per thread, so recording takes no lock; a test reads the log on the
    // thread that ran the runtime.
    static LOG: RefCell<Vec<(usize, Duration)>> = const { RefCell::new(Vec::new()) };
}

pub(crate) fn record(reachable: usize, elapsed: Duration) {
    LOG.with(|log| log.borrow_mut().push((reachable, elapsed)));
}

/// Removes and returns every record this thread made since the last call.
pub fn take() -> Vec<(usize, Duration)> {
    LOG.with(|log| std::mem::take(&mut *log.borrow_mut()))
}
