// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

#![allow(missing_docs)]
use proptest::prelude::*;
use proptest::test_runner::{Config as PropConfig, RngAlgorithm, TestRng, TestRunner};

use warp_core::{
    decode_motion_atom_payload, decode_motion_payload, encode_motion_atom_payload,
    encode_motion_payload, make_node_id, make_type_id, ApplyResult, AttachmentValue, Engine,
    GraphStore, NodeRecord, MOTION_RULE_NAME,
};

// Demonstrates how to pin a deterministic seed for property tests so failures
// are reproducible across machines and CI.
//
// To re-run with a different seed locally, you can set PROPTEST_SEED, e.g.:
//   PROPTEST_SEED=0000000000000000000000000000000000000000000000000000000000000042 cargo test -p warp-core -- proptest_seed_pinned_motion_updates
// Or update the `SEED_BYTES` below for a committed example.

#[test]
fn proptest_seed_pinned_motion_updates() {
    // Pin a seed for deterministic case generation. Using a small numeric
    // value is enough; TestRng::from_seed expects 32 bytes.
    const SEED_BYTES: [u8; 32] = [
        0x42, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0,
    ];

    let rng = TestRng::from_seed(RngAlgorithm::ChaCha, &SEED_BYTES);
    let mut runner = TestRunner::new_with_rng(PropConfig::default(), rng);

    // Strategy: finite f32 in a sane range to avoid infinities/NaNs.
    let scalar = any::<f32>().prop_filter("finite", |v| v.is_finite() && v.abs() < 1.0e6);
    let vec3 = prop::array::uniform3(scalar.clone());

    let prop = (vec3.clone(), vec3).prop_map(|(pos, vel)| (pos, vel));

    runner
        .run(&prop, |(pos, vel)| {
            // Build a fresh engine for each case (property tests are short).
            let entity = make_node_id("prop-entity");
            let entity_ty = make_type_id("entity");

            let mut store = GraphStore::default();
            store.insert_node(entity, NodeRecord { ty: entity_ty });
            store.set_node_attachment(
                entity,
                Some(AttachmentValue::Atom(encode_motion_atom_payload(pos, vel))),
            );

            let mut engine = Engine::new(store, entity);
            engine
                .register_rule(warp_core::motion_rule())
                .expect("register motion rule");

            let tx = engine.begin();
            let res = engine.apply(tx, MOTION_RULE_NAME, &entity).expect("apply");
            prop_assert!(matches!(res, ApplyResult::Applied));
            engine.commit(tx).expect("commit");

            let payload = engine
                .node_attachment(&entity)
                .expect("node_attachment ok")
                .expect("payload present");
            let AttachmentValue::Atom(payload) = payload else {
                panic!("expected Atom payload, got {payload:?}");
            };
            let (new_pos, new_vel) = decode_motion_atom_payload(payload).expect("decode");

            // Payloads are canonicalized to Q32.32 at the boundary, so the effective
            // inputs and outputs are the fixed-point quantized values.
            let (pos_q, vel_q) = decode_motion_payload(&encode_motion_payload(pos, vel))
                .expect("encode/decode canonical inputs");
            let updated_pos = [
                pos_q[0] + vel_q[0],
                pos_q[1] + vel_q[1],
                pos_q[2] + vel_q[2],
            ];
            let (expected_pos, expected_vel) =
                decode_motion_payload(&encode_motion_payload(updated_pos, vel_q))
                    .expect("encode/decode canonical outputs");

            for i in 0..3 {
                prop_assert_eq!(new_vel[i].to_bits(), expected_vel[i].to_bits());
                prop_assert_eq!(new_pos[i].to_bits(), expected_pos[i].to_bits());
            }
            Ok(())
        })
        .expect("proptest with pinned seed should complete");
}
