// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

#![allow(missing_docs)]
//! Negative/edge-case tests for the motion rule under deterministic payload semantics.
//!
//! The motion payload is now canonicalized into a Q32.32 fixed-point encoding (v2) so that
//! the attachment-plane bytes are stable across platforms and language runtimes.
//!
//! Compatibility:
//! - Legacy v0 payloads (`payload/motion/v0`, 24 bytes = 6×f32) are accepted for decode/match.
//! - On write, the motion executor upgrades to the canonical v2 payload (`payload/motion/v2`,
//!   48 bytes = 6×i64 Q32.32).
//!
//! Non-finite values have no representation in Q32.32 and are deterministically mapped:
//! - `NaN` → `0`
//! - `+∞`/`-∞` → saturated extrema (≈ ±2^31 when decoded as `f32`)

use bytes::Bytes;
use warp_core::{
    decode_motion_atom_payload, make_node_id, make_type_id, motion_payload_type_id, ApplyResult,
    AtomPayload, AttachmentValue, Engine, GraphStore, NodeRecord, MOTION_RULE_NAME,
};

fn encode_motion_payload_v0_bytes(position: [f32; 3], velocity: [f32; 3]) -> Bytes {
    let mut buf = Vec::with_capacity(24);
    for v in position.into_iter().chain(velocity.into_iter()) {
        buf.extend_from_slice(&v.to_le_bytes());
    }
    Bytes::from(buf)
}

fn run_motion_once_with_payload(payload: AtomPayload) -> (warp_core::TypeId, [f32; 3], [f32; 3]) {
    let ent = make_node_id("case");
    let ty = make_type_id("entity");
    let mut store = GraphStore::default();
    store.insert_node(ent, NodeRecord { ty });
    store.set_node_attachment(ent, Some(AttachmentValue::Atom(payload)));

    let mut engine = Engine::new(store, ent);
    engine
        .register_rule(warp_core::motion_rule())
        .expect("register motion rule");

    let tx = engine.begin();
    let _ = engine.apply(tx, MOTION_RULE_NAME, &ent).expect("apply");
    engine.commit(tx).expect("commit");

    let payload = engine
        .node_attachment(&ent)
        .expect("node_attachment ok")
        .expect("payload present");
    let AttachmentValue::Atom(payload) = payload else {
        panic!("expected Atom payload, got {payload:?}");
    };
    let ty = payload.type_id;
    let (pos, vel) = decode_motion_atom_payload(payload).expect("decode");
    (ty, pos, vel)
}

#[test]
fn motion_invalid_payload_size_returns_nomatch() {
    let ent = make_node_id("bad-payload-size");
    let ty = make_type_id("entity");
    let mut store = GraphStore::default();
    store.insert_node(ent, NodeRecord { ty });
    store.set_node_attachment(
        ent,
        Some(AttachmentValue::Atom(AtomPayload::new(
            motion_payload_type_id(),
            Bytes::from(vec![0u8; 10]),
        ))),
    );

    let mut engine = Engine::new(store, ent);
    engine
        .register_rule(warp_core::motion_rule())
        .expect("register motion rule");
    let tx = engine.begin();
    let res = engine.apply(tx, MOTION_RULE_NAME, &ent).expect("apply");
    assert!(matches!(res, ApplyResult::NoMatch));
}

#[test]
fn motion_v0_payload_is_accepted_and_upgraded_to_v2() {
    let v0_type_id = make_type_id("payload/motion/v0");
    let payload = AtomPayload::new(
        v0_type_id,
        encode_motion_payload_v0_bytes([1.0, 2.0, 3.0], [0.5, -1.0, 0.25]),
    );

    let (ty, pos, vel) = run_motion_once_with_payload(payload);
    assert_eq!(
        ty,
        motion_payload_type_id(),
        "executor should upgrade to v2"
    );
    assert_eq!(pos, [1.5, 1.0, 3.25]);
    assert_eq!(vel, [0.5, -1.0, 0.25]);
}

#[test]
fn motion_v0_payload_nan_clamps_to_zero_on_upgrade() {
    let v0_type_id = make_type_id("payload/motion/v0");
    let payload = AtomPayload::new(
        v0_type_id,
        encode_motion_payload_v0_bytes([f32::NAN, 0.0, 1.0], [0.0, f32::NAN, 2.0]),
    );

    let (ty, pos, vel) = run_motion_once_with_payload(payload);
    assert_eq!(ty, motion_payload_type_id());

    // NaNs clamp to 0 at the fixed-point boundary.
    assert_eq!(pos[0].to_bits(), 0.0f32.to_bits());
    assert_eq!(pos[1].to_bits(), 0.0f32.to_bits());
    assert_eq!(pos[2].to_bits(), 3.0f32.to_bits());
    assert_eq!(vel[0].to_bits(), 0.0f32.to_bits());
    assert_eq!(vel[1].to_bits(), 0.0f32.to_bits());
    assert_eq!(vel[2].to_bits(), 2.0f32.to_bits());
}

#[test]
fn motion_v0_payload_infinity_saturates_on_upgrade() {
    let v0_type_id = make_type_id("payload/motion/v0");
    let payload = AtomPayload::new(
        v0_type_id,
        encode_motion_payload_v0_bytes([f32::INFINITY, 1.0, f32::NEG_INFINITY], [1.0, 2.0, 3.0]),
    );

    let (ty, pos, vel) = run_motion_once_with_payload(payload);
    assert_eq!(ty, motion_payload_type_id());

    // Saturated Q32.32 extrema decode to ±2^31 as f32.
    assert_eq!(pos[0].to_bits(), 2147483648.0f32.to_bits());
    assert_eq!(pos[1].to_bits(), 3.0f32.to_bits());
    assert_eq!(pos[2].to_bits(), (-2147483648.0f32).to_bits());
    assert_eq!(vel, [1.0, 2.0, 3.0]);
}

#[test]
fn motion_signed_zero_is_canonicalized_to_positive_zero() {
    let v0_type_id = make_type_id("payload/motion/v0");
    let payload = AtomPayload::new(
        v0_type_id,
        encode_motion_payload_v0_bytes([0.0f32, -0.0, 0.0], [-0.0f32, 0.0, -0.0]),
    );

    let (_ty, pos, vel) = run_motion_once_with_payload(payload);
    for i in 0..3 {
        assert_eq!(pos[i].to_bits(), 0.0f32.to_bits());
        assert_eq!(vel[i].to_bits(), 0.0f32.to_bits());
    }
}

#[test]
fn motion_subnormal_inputs_are_flushed_to_zero() {
    let v0_type_id = make_type_id("payload/motion/v0");
    let sub = f32::from_bits(1); // smallest positive subnormal
    let payload = AtomPayload::new(
        v0_type_id,
        encode_motion_payload_v0_bytes([sub, sub, sub], [sub, sub, sub]),
    );

    let (_ty, pos, vel) = run_motion_once_with_payload(payload);
    assert_eq!(pos, [0.0, 0.0, 0.0]);
    assert_eq!(vel, [0.0, 0.0, 0.0]);
}
