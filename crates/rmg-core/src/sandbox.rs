//! Lightweight sandbox utilities for spinning up isolated Echo instances (Engine + `GraphStore`)
//! with configurable scheduler and seeds for determinism tests and A/B comparisons.

use std::sync::Arc;

use crate::engine_impl::Engine;
use crate::graph::GraphStore;
use crate::ident::NodeId;
use crate::rule::RewriteRule;
use crate::scheduler::SchedulerKind;
use crate::snapshot::Snapshot;

/// Describes how to construct an isolated Echo (Engine + `GraphStore`).
///
/// Seed and rules are provided as factories so that each instance receives a fresh graph
/// and rule table without sharing state.
#[derive(Clone)]
pub struct EchoConfig {
    /// Which scheduler implementation to use (Radix default, Legacy for comparison).
    pub scheduler: SchedulerKind,
    /// Whether the caller intends to run this Echo on its own thread (advisory only).
    pub threaded: bool,
    /// Human label for reports/benchmarks.
    pub label: String,
    /// Factory producing a fresh (`GraphStore`, root `NodeId`).
    pub seed: Arc<dyn Fn() -> (GraphStore, NodeId) + Send + Sync>,
    /// Factory producing the rewrite rules to register.
    pub rules: Arc<dyn Fn() -> Vec<RewriteRule> + Send + Sync>,
}

impl EchoConfig {
    /// Convenience constructor.
    pub fn new<FSeed, FRules>(
        label: impl Into<String>,
        scheduler: SchedulerKind,
        threaded: bool,
        seed: FSeed,
        rules: FRules,
    ) -> Self
    where
        FSeed: Fn() -> (GraphStore, NodeId) + Send + Sync + 'static,
        FRules: Fn() -> Vec<RewriteRule> + Send + Sync + 'static,
    {
        Self {
            scheduler,
            threaded,
            label: label.into(),
            seed: Arc::new(seed),
            rules: Arc::new(rules),
        }
    }
}

/// Determinism check failure.
#[derive(Debug, thiserror::Error)]
pub enum DeterminismError {
    /// Snapshot hashes diverged at a given step between two Echo instances.
    #[error("determinism mismatch at step {step}: {label_a}={hash_a:?} vs {label_b}={hash_b:?}")]
    SnapshotMismatch {
        /// Step index where divergence was detected.
        step: usize,
        /// Label of the first Echo.
        label_a: String,
        /// Label of the second Echo.
        label_b: String,
        /// Snapshot hash of the first Echo.
        hash_a: [u8; 32],
        /// Snapshot hash of the second Echo.
        hash_b: [u8; 32],
    },
}

/// Build a fresh Engine from an `EchoConfig`.
pub fn build_engine(cfg: &EchoConfig) -> Engine {
    let (store, root) = (cfg.seed)();
    let mut eng = Engine::with_scheduler(store, root, cfg.scheduler);
    for rule in (cfg.rules)() {
        // Rules are authored by the caller; propagate errors explicitly in the future.
        let _ = eng.register_rule(rule);
    }
    eng
}

/// Run two Echoes with identical step function and compare snapshot hashes each step.
///
/// This runs synchronously (same thread) to remove scheduling noise. For threaded runs,
/// callers can spawn threads and use this function's logic for final comparison.
///
/// # Errors
/// Returns `DeterminismError::SnapshotMismatch` when the two Echo instances
/// produce different snapshot hashes at the same step.
pub fn run_pair_determinism<F>(
    cfg_a: &EchoConfig,
    cfg_b: &EchoConfig,
    steps: usize,
    mut step_fn: F,
) -> Result<(), DeterminismError>
where
    F: FnMut(usize, &mut Engine) + Send,
{
    let mut a = build_engine(cfg_a);
    let mut b = build_engine(cfg_b);

    for step in 0..steps {
        step_fn(step, &mut a);
        let snap_a: Snapshot = a.snapshot();

        step_fn(step, &mut b);
        let snap_b: Snapshot = b.snapshot();

        if snap_a.hash != snap_b.hash {
            return Err(DeterminismError::SnapshotMismatch {
                step,
                label_a: cfg_a.label.clone(),
                label_b: cfg_b.label.clone(),
                hash_a: snap_a.hash,
                hash_b: snap_b.hash,
            });
        }
    }
    Ok(())
}
