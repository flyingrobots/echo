// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Echo Kernel
//!
//! The core kernel logic for the JITOS operating system.
//! Manages the System RMG, Shadow Working Sets, and process lifecycle.

use anyhow::{Context, Result};
use echo_sched::Scheduler;
use echo_tasks::{slaps::Slaps, Planner};
use rmg_core::GraphStore; // The canonical system RMG
use std::collections::HashMap;
use tracing::{info, instrument};

/// Identifier for a Shadow Working Set (SWS).
pub type SwsId = u64;

/// A Shadow Working Set: an isolated branch of the causal graph.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SwsContext {
    /// Unique identifier for this Shadow Working Set.
    pub id: SwsId,
    /// The graph store associated with this SWS (isolated overlay).
    pub store: GraphStore,
    /// The epoch of the System RMG from which this SWS was forked.
    pub base_epoch: u64,
}

/// The JITOS Kernel Core.
/// Owns the system RMG(s) and manages the overall state.
pub struct Kernel {
    system_rmg: GraphStore, // The canonical system RMG
    system_epoch: u64,
    sws_pool: HashMap<SwsId, SwsContext>,
    next_sws_id: SwsId,
    scheduler: Scheduler,
    planner: Planner,
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
        let system_rmg = GraphStore::default();
        let scheduler = Scheduler::new(1000); // 1-second tick interval for now
        let planner = Planner::new();

        Self {
            system_rmg,
            system_epoch: 0, // Initialize system epoch to 0
            sws_pool: HashMap::new(),
            next_sws_id: 1,
            scheduler,
            planner,
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

    /// Submits a high-level intent (SLAPS) to the kernel.
    ///
    /// The kernel will:
    /// 1. Plan the execution using the HTN Planner.
    /// 2. Create a new Shadow Working Set (SWS) for the task.
    /// 3. (Future) Dispatch tasks to the scheduler.
    #[instrument(skip(self))]
    pub fn submit_intent(&mut self, slaps: Slaps) -> Result<SwsId> {
        info!("Received intent: {}", slaps.intent);

        // 1. Plan
        // Note: Planner currently returns a String description.
        // In the future, this will return a DAG.
        match self.planner.plan(&slaps) {
            Ok(plan_desc) => info!("Generated Plan: {}", plan_desc),
            Err(e) => info!("Planning failed (using fallback/empty): {}", e),
        }

        // 2. Create SWS for this process
        let sws_id = self.create_sws();

        Ok(sws_id)
    }

    /// Creates a new Shadow Working Set (SWS) by forking the current System RMG.
    #[instrument(skip(self))]
    pub fn create_sws(&mut self) -> SwsId {
        let id = self.next_sws_id;
        self.next_sws_id += 1;

        let sws = SwsContext {
            id,
            store: self.system_rmg.clone(), // Copy-on-write (expensive clone for now)
            base_epoch: self.system_epoch,  // SWS forks from current system epoch
        };

        self.sws_pool.insert(id, sws);
        info!(
            "Created SWS #{} from system epoch {}",
            id, self.system_epoch
        );
        id
    }

    /// Applies a rewrite to a specific SWS.
    ///
    /// For now, this is a placeholder that logs the intent.
    #[instrument(skip(self))]
    pub fn apply_rewrite_sws(&mut self, sws_id: SwsId, _rewrite: String) -> Result<()> {
        let _sws = self.sws_pool.get_mut(&sws_id).context("SWS not found")?;
        info!(
            "Applying rewrite to SWS #{} (base epoch {})",
            sws_id, _sws.base_epoch
        );
        // Logic to parse rewrite and update sws.store goes here
        Ok(())
    }

    /// Collapses (merges) a SWS back into the System RMG.
    ///
    /// This makes the speculative state canonical.
    #[instrument(skip(self))]
    pub fn collapse_sws(&mut self, sws_id: SwsId) -> Result<()> {
        let sws = self.sws_pool.remove(&sws_id).context("SWS not found")?;
        info!(
            "Collapsing SWS #{} (base epoch {}) into System RMG (epoch {})",
            sws_id, sws.base_epoch, self.system_epoch
        );

        // Naive merge: Replace system store with SWS store.
        // In a real implementation, we would compute deltas and apply them transactionally
        // and reconcile any divergence if sws.base_epoch is older than system_epoch.
        self.system_rmg = sws.store;
        self.system_epoch += 1; // Advance system epoch on successful collapse
        info!("System RMG epoch advanced to {}", self.system_epoch);
        Ok(())
    }

    /// Placeholder for a simple API to submit rewrites to the system RMG directly.
    #[instrument(skip(self))]
    pub fn submit_rewrite(&mut self, _rewrite: String) -> Result<()> {
        info!("Rewrite submitted to System RMG (placeholder).");
        // This should also advance the system_epoch
        self.system_epoch += 1;
        info!("System RMG epoch advanced to {}", self.system_epoch);
        Ok(())
    }

    /// Retrieves the current state of the system RMG as a serialized string.
    #[instrument(skip(self))]
    pub fn get_rmg_state(&self) -> String {
        serde_json::to_string(&self.system_rmg)
            .unwrap_or_else(|e| format!("Error serializing RMG: {}", e))
    }
}
