// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Deterministic test kernel used by the DIND harness.

use echo_dry_tests::build_motion_demo_engine;
use warp_core::{make_node_id, ApplyResult, DispatchDisposition, Engine};

/// Auto-generated codec and type definitions.
pub mod generated;
/// DIND test rules and state management.
pub mod rules;

use rules::{
    ball_physics_rule, drop_ball_rule, route_push_rule, set_theme_rule, toast_rule, toggle_nav_rule,
};

#[cfg(feature = "dind_ops")]
use rules::put_kv_rule;

/// The deterministic kernel used for DIND scenarios.
pub struct EchoKernel {
    engine: Engine,
}

impl Default for EchoKernel {
    fn default() -> Self {
        Self::new()
    }
}

impl EchoKernel {
    /// Create a new kernel instance with all DIND rules registered.
    pub fn new() -> Self {
        let mut e = build_motion_demo_engine();
        e.register_rule(toast_rule())
            .expect("toast_rule registration failed");
        e.register_rule(route_push_rule())
            .expect("route_push_rule registration failed");
        e.register_rule(set_theme_rule())
            .expect("set_theme_rule registration failed");
        e.register_rule(toggle_nav_rule())
            .expect("toggle_nav_rule registration failed");
        e.register_rule(drop_ball_rule())
            .expect("drop_ball_rule registration failed");
        e.register_rule(ball_physics_rule())
            .expect("ball_physics_rule registration failed");
        #[cfg(feature = "dind_ops")]
        e.register_rule(put_kv_rule())
            .expect("put_kv_rule registration failed");
        e.register_rule(warp_core::inbox::ack_pending_rule())
            .expect("ack_pending_rule registration failed");

        Self { engine: e }
    }

    /// Dispatch an intent (canonical bytes) with an auto-assigned sequence number.
    pub fn dispatch_intent(&mut self, intent_bytes: &[u8]) {
        // Canonical ingress: content-addressed, idempotent on `intent_id`.
        // Bytes are opaque to the core engine; validation is the caller's responsibility.
        let _ = self
            .engine
            .ingest_intent(intent_bytes)
            .expect("ingest intent");
    }

    /// Run a deterministic step.
    pub fn step(&mut self) -> bool {
        let tx = self.engine.begin();
        let ball_id = make_node_id("ball");
        let mut dirty = false;

        // Consume exactly one pending intent per tick, using canonical `intent_id` order.
        let dispatch = self
            .engine
            .dispatch_next_intent(tx)
            .expect("dispatch_next_intent");
        if matches!(dispatch, DispatchDisposition::Consumed { .. }) {
            dirty = true;
        }

        if matches!(
            self.engine
                .apply(tx, rules::BALL_PHYSICS_RULE_NAME, &ball_id)
                .expect("apply physics rule"),
            ApplyResult::Applied
        ) {
            dirty = true;
        }

        if dirty {
            // Commit must succeed in test kernel; failure indicates corruption.
            self.engine
                .commit(tx)
                .expect("commit failed - test kernel corruption");
            true
        } else {
            self.engine.abort(tx);
            false
        }
    }

    /// Access the underlying engine (read-only).
    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// Access the underlying engine (mutable).
    pub fn engine_mut(&mut self) -> &mut Engine {
        &mut self.engine
    }

    /// Canonical state hash of the root warp.
    ///
    /// # Panics
    /// Panics if the root warp does not exist (indicates test kernel misconfiguration).
    pub fn state_hash(&self) -> [u8; 32] {
        self.engine
            .state()
            .store(&warp_core::make_warp_id("root"))
            .expect("root warp must exist in test kernel")
            .canonical_state_hash()
    }
}
