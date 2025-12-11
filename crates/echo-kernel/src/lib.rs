// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Echo Kernel
//!
//! The core kernel logic for the JITOS operating system.
//! Manages the System RMG, Shadow Working Sets, and process lifecycle.

use anyhow::Result;
use echo_sched::Scheduler;
use rmg_core::GraphStore; // The canonical system RMG
use tracing::{info, instrument};

/// The JITOS Kernel Core.
/// Owns the system RMG(s) and manages the overall state.
pub struct Kernel {
    system_rmg: GraphStore, // The canonical system RMG
    // In the future: sws_pool: HashMap<SwsId, SwsInstance>,
    // In the future: processes: HashMap<ProcessId, Process>,
    scheduler: Scheduler,
}

impl Default for Kernel {
    fn default() -> Self {
        Self::new()
    }
}

impl Kernel {
    /// Initializes a new JITOS Kernel instance.
    ///
    /// This sets up the in-memory System RMG and the default scheduler.
    #[instrument]
    pub fn new() -> Self {
        info!("Initializing JITOS Kernel...");
        // Placeholder for real RMG initialization
        let system_rmg = GraphStore::default(); // Assuming GraphStore::default() exists
        let scheduler = Scheduler::new(1000); // 1-second tick interval for now

        Self {
            system_rmg,
            scheduler,
        }
    }

    /// Starts the kernel's main execution loop.
    ///
    /// This hands control over to the scheduler.
    #[instrument(skip(self))]
    pub async fn run(&mut self) -> Result<()> {
        info!("JITOS Kernel running.");
        // The kernel's main loop will delegate to the scheduler
        self.scheduler.run().await
    }

    /// Placeholder for a simple API to submit rewrites to the system RMG.
    #[instrument(skip(self))]
    pub fn submit_rewrite(&mut self, _rewrite: String) -> Result<()> {
        // In the future, this would parse the rewrite and apply it to system_rmg
        info!("Rewrite submitted (placeholder).");
        Ok(())
    }

    /// Retrieves the current state of the system RMG as a serialized string.
    #[instrument(skip(self))]
    pub fn get_rmg_state(&self) -> String {
        serde_json::to_string(&self.system_rmg)
            .unwrap_or_else(|e| format!("Error serializing RMG: {}", e))
    }
}
