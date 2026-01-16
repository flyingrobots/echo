// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Deterministic test kernel used by the DIND harness.

use echo_wasm_abi::unpack_intent_v1;
use warp_core::{
    build_motion_demo_engine, make_edge_id, make_node_id, make_type_id, make_warp_id,
    AtomPayload, AttachmentValue, EdgeRecord, Engine, NodeRecord,
};

pub mod generated;
pub mod rules;

use rules::{
    ball_physics_rule, drop_ball_rule, route_push_rule, set_theme_rule, toast_rule,
    toggle_nav_rule,
};

#[cfg(feature = "dind_ops")]
use rules::put_kv_rule;

/// The deterministic kernel used for DIND scenarios.
pub struct EchoKernel {
    engine: Engine,
    intent_seq: u64,
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
        let _ = e.register_rule(toast_rule());
        let _ = e.register_rule(route_push_rule());
        let _ = e.register_rule(set_theme_rule());
        let _ = e.register_rule(toggle_nav_rule());
        let _ = e.register_rule(drop_ball_rule());
        let _ = e.register_rule(ball_physics_rule());
        #[cfg(feature = "dind_ops")]
        let _ = e.register_rule(put_kv_rule());

        Self {
            engine: e,
            intent_seq: 0,
        }
    }

    /// Dispatch an intent (canonical bytes) with an auto-assigned sequence number.
    pub fn dispatch_intent(&mut self, intent_bytes: &[u8]) {
        self.intent_seq += 1;
        self.dispatch_intent_with_seq(self.intent_seq, intent_bytes);
    }

    /// Dispatch an intent with an explicit sequence number.
    pub fn dispatch_intent_with_seq(&mut self, seq: u64, intent_bytes: &[u8]) {
        if let Ok((op_id, _)) = unpack_intent_v1(intent_bytes) {
            let payload = AtomPayload::new(
                make_type_id(&format!("intent:{}", op_id)),
                bytes::Bytes::copy_from_slice(intent_bytes),
            );
            // Ingest intent: creates sim/inbox/event:{seq} with intent payload
            let _ = self.engine.ingest_inbox_event(seq, &payload);

            // Create a sidecar metadata node for the sequence number
            let event_label = format!("sim/inbox/event:{:016}", seq);
            let event_id = make_node_id(&event_label);
            let meta_id = make_node_id(&format!("{}/meta", event_label));

            // Access the root store directly to add sidecar
            if let Some(store) = self.engine.state_mut().store_mut(&make_warp_id("root")) {
                store.insert_node(meta_id, NodeRecord { ty: make_type_id("sys/meta") });
                store.insert_edge(
                    event_id,
                    EdgeRecord {
                        id: make_edge_id(&format!("edge:{}/meta", event_label)),
                        from: event_id,
                        to: meta_id,
                        ty: make_type_id("edge:meta"),
                    },
                );

                let seq_payload = AtomPayload::new(
                    make_type_id("sys/seq"),
                    bytes::Bytes::copy_from_slice(&seq.to_le_bytes()),
                );
                store.set_node_attachment(meta_id, Some(AttachmentValue::Atom(seq_payload)));
            }

            if seq > self.intent_seq {
                self.intent_seq = seq;
            }
        }
    }

    /// Run a deterministic step with a fixed budget.
    pub fn step(&mut self, _budget: u32) -> bool {
        let tx = self.engine.begin();
        let inbox_id = make_node_id("sim/inbox");
        let ball_id = make_node_id("ball");
        let root_warp = make_warp_id("root");
        let mut applied_any = false;

        let cursor_ty = make_type_id("sim/inbox/cursor");
        let last_processed_seq = self
            .engine
            .node_attachment(&inbox_id)
            .ok()
            .flatten()
            .and_then(|v| {
                if let AttachmentValue::Atom(a) = v {
                    if a.type_id == cursor_ty {
                        let mut b = [0u8; 8];
                        b.copy_from_slice(&a.bytes[0..8]);
                        Some(u64::from_le_bytes(b))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .unwrap_or(0);

        let mut event_ids = Vec::new();
        if let Some(store) = self.engine.state().store(&root_warp) {
            event_ids = store.edges_from(&inbox_id).map(|e| e.to).collect();
        }
        event_ids.sort();

        let mut max_seq_processed = last_processed_seq;
        let mut intent_processed_this_tick = false;
        for event_id in &event_ids {
            let seq = self.extract_seq_from_node(event_id).unwrap_or(0);
            if seq <= last_processed_seq {
                continue;
            }

            let rules = [
                rules::ROUTE_PUSH_RULE_NAME,
                rules::SET_THEME_RULE_NAME,
                rules::TOGGLE_NAV_RULE_NAME,
                rules::TOAST_RULE_NAME,
                rules::DROP_BALL_RULE_NAME,
            ];
            for rule_name in rules {
                if let Ok(warp_core::ApplyResult::Applied) =
                    self.engine.apply(tx, rule_name, event_id)
                {
                    applied_any = true;
                    if seq > max_seq_processed {
                        max_seq_processed = seq;
                    }
                    intent_processed_this_tick = true;
                    break;
                }
            }
            if intent_processed_this_tick {
                break;
            }
        }

        if max_seq_processed > last_processed_seq {
            let cursor_payload = AtomPayload::new(
                cursor_ty,
                bytes::Bytes::copy_from_slice(&max_seq_processed.to_le_bytes()),
            );
            let _ = self
                .engine
                .set_node_attachment(inbox_id, Some(AttachmentValue::Atom(cursor_payload)));
        }

        if let Ok(warp_core::ApplyResult::Applied) =
            self.engine.apply(tx, rules::BALL_PHYSICS_RULE_NAME, &ball_id)
        {
            applied_any = true;
        }

        let dirty = applied_any || (max_seq_processed > last_processed_seq);

        if dirty {
            let res = self.engine.commit(tx);
            res.is_ok()
        } else {
            self.engine.abort(tx);
            false
        }
    }

    fn extract_seq_from_node(&self, id: &warp_core::NodeId) -> Option<u64> {
        let seq_ty = make_type_id("sys/seq");
        let root_warp = make_warp_id("root");
        if let Some(store) = self.engine.state().store(&root_warp) {
            for edge in store.edges_from(id) {
                if edge.ty == make_type_id("edge:meta") {
                    return self
                        .engine
                        .node_attachment(&edge.to)
                        .ok()
                        .flatten()
                        .and_then(|v| {
                            if let AttachmentValue::Atom(a) = v {
                                if a.type_id == seq_ty {
                                    let mut b = [0u8; 8];
                                    b.copy_from_slice(&a.bytes[0..8]);
                                    return Some(u64::from_le_bytes(b));
                                }
                            }
                            None
                        });
                }
            }
        }
        None
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
    pub fn state_hash(&self) -> [u8; 32] {
        if let Some(store) = self.engine.state().store(&make_warp_id("root")) {
            store.canonical_state_hash()
        } else {
            [0u8; 32]
        }
    }
}
