// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Echo Scheduler
//!
//! A deterministic tick-based scheduler for the JITOS kernel.

use anyhow::Result;
use std::time::Duration;
use tokio::time;
use tracing::{info, instrument};

/// A simple scheduler that ticks at a fixed interval.
/// In the future, this will manage the deterministic "Echo" loop.
pub struct Scheduler {
    interval: Duration,
    tick_count: u64,
}

impl Scheduler {
    /// Creates a new scheduler with the specified tick interval in milliseconds.
    pub fn new(interval_ms: u64) -> Self {
        Self {
            interval: Duration::from_millis(interval_ms),
            tick_count: 0,
        }
    }

    /// Starts the scheduler loop.
    ///
    /// This function will loop indefinitely, triggering a tick at the configured interval.
    #[instrument(skip(self))]
    pub async fn run(&mut self) -> Result<()> {
        let mut interval = time::interval(self.interval);

        info!("Scheduler started. Tick interval: {:?}", self.interval);

        loop {
            interval.tick().await;
            self.tick().await?;
        }
    }

    async fn tick(&mut self) -> Result<()> {
        self.tick_count += 1;
        // Placeholder: This is where we would lock the RMG, check for runnable
        // rewrites/tasks, and apply them.
        if self.tick_count.is_multiple_of(10) {
            info!("Tick #{}", self.tick_count);
        }
        Ok(())
    }
}
