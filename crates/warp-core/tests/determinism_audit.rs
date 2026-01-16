// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! Audit tests for floating-point determinism in hashing paths.
//!
//! These tests verify:
//! 1. Whether f32 bit-flips affect canonical hashes (Sensitivity).
//! 2. Whether identical inputs produce identical hashes (Repeatability).
//!
//! If sensitivity is high, then cross-platform determinism relies on
//! `F32Scalar` arithmetic being bit-perfect across architectures.

use warp_core::{
    encode_motion_atom_payload_v0, encode_motion_payload, make_node_id, make_type_id, make_warp_id,
    AtomPayload, AttachmentValue, Engine, GraphStore, Hash, NodeRecord,
};

/// Helper to compute the commit hash for a single-node state with a given payload.
fn compute_hash_for_payload(payload: AtomPayload) -> Hash {
    let warp_id = make_warp_id("audit");
    let mut store = GraphStore::new(warp_id);
    let node_id = make_node_id("node");

    store.insert_node(
        node_id,
        NodeRecord {
            ty: make_type_id("test_entity"),
        },
    );
    store.set_node_attachment(node_id, Some(AttachmentValue::Atom(payload)));

    // Create a fresh engine. It has no history, so no parents.
    // Committing immediately will hash the initial state.
    let mut engine = Engine::new(store, node_id);
    let tx = engine.begin();
    let snapshot = engine.commit(tx).expect("commit should succeed");

    snapshot.hash
}

#[test]
fn audit_float_sensitivity_v0() {
    // V0 payload is raw f32 bytes.
    // Changing the LSB of an f32 MUST change the commit hash.

    let pos_a = [1.0, 2.0, 3.0];
    let vel = [0.0, 0.0, 0.0];

    // Create pos_b differing by 1 ULP in the first component
    let pos_bits = 1.0f32.to_bits();
    let pos_b_val = f32::from_bits(pos_bits + 1);
    let pos_b = [pos_b_val, 2.0, 3.0];

    assert_ne!(pos_a[0], pos_b[0]);

    let payload_a = encode_motion_atom_payload_v0(pos_a, vel);
    let payload_b = encode_motion_atom_payload_v0(pos_b, vel);

    let hash_a = compute_hash_for_payload(payload_a);
    let hash_b = compute_hash_for_payload(payload_b);

    assert_ne!(
        hash_a, hash_b,
        "Commit hash MUST change when f32 payload changes by 1 ULP (v0)"
    );
}

#[test]
fn audit_float_sensitivity_v2() {
    // V2 payload is Q32.32 derived from f32.
    // Q32.32 has higher precision than f32, so 1 ULP change in f32
    // should still result in a different Q32.32 integer, and thus a different hash.

    let pos_a = [1.0, 2.0, 3.0];
    let vel = [0.0, 0.0, 0.0];

    // Create pos_b differing by 1 ULP in the first component
    let pos_bits = 1.0f32.to_bits();
    let pos_b_val = f32::from_bits(pos_bits + 1);
    let pos_b = [pos_b_val, 2.0, 3.0];

    // Encode using canonical V2
    let bytes_a = encode_motion_payload(pos_a, vel);
    let bytes_b = encode_motion_payload(pos_b, vel);

    // Verify the bytes differ (Q32.32 captured the f32 diff)
    assert_ne!(
        bytes_a, bytes_b,
        "Q32.32 encoding MUST capture 1 ULP f32 difference"
    );

    let payload_a = AtomPayload::new(warp_core::motion_payload_type_id(), bytes_a);
    let payload_b = AtomPayload::new(warp_core::motion_payload_type_id(), bytes_b);

    let hash_a = compute_hash_for_payload(payload_a);
    let hash_b = compute_hash_for_payload(payload_b);

    assert_ne!(
        hash_a, hash_b,
        "Commit hash MUST change when input f32 changes by 1 ULP (v2)"
    );
}

#[test]
fn audit_repeatability() {
    // Verify that encoding the same float values repeatedly produces identical hashes.
    // This guards against non-deterministic iteration or internal state leakage.

    let pos = [1.23456, -7.89012, 3.0];
    let vel = [0.001, -0.002, 0.003];

    let mut hashes = Vec::new();

    for _ in 0..100 {
        let payload = AtomPayload::new(
            warp_core::motion_payload_type_id(),
            encode_motion_payload(pos, vel),
        );
        hashes.push(compute_hash_for_payload(payload));
    }

    let first = hashes[0];
    for (i, h) in hashes.iter().enumerate().skip(1) {
        assert_eq!(*h, first, "Hash mismatch at iteration {}", i);
    }
}
