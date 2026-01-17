// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Determinism Drill Sergeant™ test suite for MaterializationBus.
//!
//! These tests prove (and continuously re-prove) that bus semantics are:
//! - **Order-independent**: Same emissions in any order → identical output
//! - **Confluence-safe**: No silent winner-picks, no timing leaks
//! - **Wire-stable**: Frame format is frozen
//!
//! # Tier 0: Hard Invariants (Unit Tests)
//! Frame invariants, EmitKey ordering, bus order-independence, policy semantics.
//!
//! # Tier 2: Port/Subscription Tests
//! Subscription filtering, replay(1) semantics.
//!
//! # Tier 3: Permutation Tests
//! Exhaustive permutation testing for small N.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use warp_core::materialization::{
    decode_frames, encode_frames, make_channel_id, ChannelId, ChannelPolicy, EmitKey,
    MaterializationBus, MaterializationFrame, MaterializationPort, FRAME_MAGIC, FRAME_VERSION,
};

// =============================================================================
// TIER 0: FRAME INVARIANTS (A)
// =============================================================================

/// Magic bytes must be exactly "MBUS" (0x4D, 0x42, 0x55, 0x53).
#[test]
fn frame_magic_is_mbus() {
    assert_eq!(FRAME_MAGIC, [0x4D, 0x42, 0x55, 0x53]);
    assert_eq!(&FRAME_MAGIC, b"MBUS");
}

/// Version must be 1 (little-endian 0x0001).
#[test]
fn frame_version_is_one() {
    assert_eq!(FRAME_VERSION, 1);
}

/// Encoded frame header contains magic, version, and reserved=0.
#[test]
fn frame_header_format_stable() {
    let ch = make_channel_id("test:header");
    let frame = MaterializationFrame::new(ch, vec![1, 2, 3]);
    let encoded = frame.encode();

    // Magic (4 bytes)
    assert_eq!(&encoded[0..4], &FRAME_MAGIC);

    // Version (2 bytes, little-endian)
    assert_eq!(u16::from_le_bytes([encoded[4], encoded[5]]), FRAME_VERSION);

    // Reserved (2 bytes, must be 0)
    assert_eq!(&encoded[6..8], &[0, 0]);
}

/// Single frame encode → decode roundtrip produces identical frame.
#[test]
fn frame_roundtrip_single() {
    let ch = make_channel_id("test:roundtrip");
    let data = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let frame = MaterializationFrame::new(ch, data.clone());

    let encoded = frame.encode();
    let decoded = MaterializationFrame::decode(&encoded).expect("decode");

    assert_eq!(decoded.channel, ch);
    assert_eq!(decoded.data, data);

    // Re-encode must produce identical bytes
    let re_encoded = decoded.encode();
    assert_eq!(encoded, re_encoded);
}

/// Multi-frame concatenation roundtrip.
#[test]
fn frame_roundtrip_multi_concat() {
    let frames = vec![
        MaterializationFrame::new(make_channel_id("ch:one"), vec![1, 2, 3]),
        MaterializationFrame::new(make_channel_id("ch:two"), vec![4, 5]),
        MaterializationFrame::new(make_channel_id("ch:three"), vec![6, 7, 8, 9]),
    ];

    let encoded = encode_frames(&frames);
    let decoded = decode_frames(&encoded).expect("decode multi");

    assert_eq!(decoded.len(), 3);
    for (orig, dec) in frames.iter().zip(decoded.iter()) {
        assert_eq!(orig.channel, dec.channel);
        assert_eq!(orig.data, dec.data);
    }

    // Re-encode must produce identical bytes
    let re_encoded = encode_frames(&decoded);
    assert_eq!(encoded, re_encoded);
}

/// Truncation at any point must be rejected.
#[test]
fn frame_rejects_truncated() {
    let frame = MaterializationFrame::new(make_channel_id("test:trunc"), vec![1, 2, 3, 4, 5]);
    let encoded = frame.encode();

    // Try truncating at every byte boundary
    for len in 0..encoded.len() {
        let truncated = &encoded[..len];
        assert!(
            MaterializationFrame::decode(truncated).is_none(),
            "should reject truncation at byte {}",
            len
        );
    }
}

/// Wrong magic must be rejected.
#[test]
fn frame_rejects_bad_magic() {
    let frame = MaterializationFrame::new(make_channel_id("test:magic"), vec![1, 2, 3]);
    let mut bad = frame.encode();
    bad[0] = 0xFF; // Corrupt magic
    assert!(MaterializationFrame::decode(&bad).is_none());
}

/// Wrong version must be rejected.
#[test]
fn frame_rejects_bad_version() {
    let frame = MaterializationFrame::new(make_channel_id("test:version"), vec![1, 2, 3]);
    let mut bad = frame.encode();
    bad[4] = 0xFF; // Corrupt version
    assert!(MaterializationFrame::decode(&bad).is_none());
}

/// Payload length < 32 (channel ID size) must be rejected.
#[test]
fn frame_rejects_payload_too_small() {
    // Craft a header with payload_len = 16 (< 32)
    let mut bad = Vec::new();
    bad.extend_from_slice(&FRAME_MAGIC);
    bad.extend_from_slice(&FRAME_VERSION.to_le_bytes());
    bad.extend_from_slice(&0u16.to_le_bytes()); // reserved
    bad.extend_from_slice(&16u32.to_le_bytes()); // payload_len < 32
    bad.extend_from_slice(&[0u8; 16]); // fake payload

    assert!(MaterializationFrame::decode(&bad).is_none());
}

// =============================================================================
// TIER 0: EMITKEY ORDERING INVARIANTS (B)
// =============================================================================

/// EmitKey ordering is stable across insertions.
#[test]
fn emit_key_total_order_is_stable() {
    use std::collections::BTreeMap;

    fn h(n: u8) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        bytes[31] = n;
        bytes
    }

    // Create keys in "wrong" order
    let keys = [
        EmitKey::with_subkey(h(2), 1, 0),
        EmitKey::with_subkey(h(1), 2, 0),
        EmitKey::with_subkey(h(1), 1, 1),
        EmitKey::with_subkey(h(1), 1, 0),
    ];

    // Insert into BTreeMap
    let mut map = BTreeMap::new();
    for (i, key) in keys.iter().enumerate() {
        map.insert(*key, i);
    }

    // Iteration order must be canonical (lexicographic)
    let iterated: Vec<_> = map.keys().copied().collect();

    // Expected order: (h(1), 1, 0) < (h(1), 1, 1) < (h(1), 2, 0) < (h(2), 1, 0)
    assert_eq!(iterated[0], EmitKey::with_subkey(h(1), 1, 0));
    assert_eq!(iterated[1], EmitKey::with_subkey(h(1), 1, 1));
    assert_eq!(iterated[2], EmitKey::with_subkey(h(1), 2, 0));
    assert_eq!(iterated[3], EmitKey::with_subkey(h(2), 1, 0));
}

/// EmitKey can be constructed from executor-available fields only.
/// This test documents the API contract: no scheduler internals required.
#[test]
fn emit_key_is_computable_from_executor_context() {
    // Simulate executor context
    let scope_hash: [u8; 32] = [0xAB; 32]; // Available: scope node hash
    let rule_id: u32 = 42; // Available: compact rule ID
    let subkey: u32 = 0; // Available: caller-provided

    // Must be able to construct EmitKey from these fields alone
    let key = EmitKey::with_subkey(scope_hash, rule_id, subkey);

    assert_eq!(key.scope_hash, scope_hash);
    assert_eq!(key.rule_id, rule_id);
    assert_eq!(key.subkey, subkey);
}

/// EmitKey::new() sets subkey to 0.
#[test]
fn emit_key_new_defaults_subkey_to_zero() {
    let key = EmitKey::new([0u8; 32], 1);
    assert_eq!(key.subkey, 0);
}

/// subkey_from_hash is deterministic.
#[test]
fn emit_key_subkey_from_hash_is_deterministic() {
    let hash: [u8; 32] = [0x42; 32];
    let s1 = EmitKey::subkey_from_hash(&hash);
    let s2 = EmitKey::subkey_from_hash(&hash);
    assert_eq!(s1, s2);
}

// =============================================================================
// TIER 0: BUS CORE SEMANTICS - ORDER INDEPENDENCE (C)
// =============================================================================

fn h(n: u8) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[31] = n;
    bytes
}

fn key(scope: u8, rule: u32) -> EmitKey {
    EmitKey::new(h(scope), rule)
}

fn key_sub(scope: u8, rule: u32, subkey: u32) -> EmitKey {
    EmitKey::with_subkey(h(scope), rule, subkey)
}

/// Log policy: insertion order doesn't affect finalized output.
#[test]
fn bus_log_is_order_independent() {
    let ch = make_channel_id("test:order");

    // Order A: (1,1), (2,1), (1,2)
    let bus_a = MaterializationBus::new();
    bus_a.emit(ch, key(1, 1), vec![0xAA]).expect("emit");
    bus_a.emit(ch, key(2, 1), vec![0xBB]).expect("emit");
    bus_a.emit(ch, key(1, 2), vec![0xCC]).expect("emit");

    // Order B: (2,1), (1,2), (1,1)
    let bus_b = MaterializationBus::new();
    bus_b.emit(ch, key(2, 1), vec![0xBB]).expect("emit");
    bus_b.emit(ch, key(1, 2), vec![0xCC]).expect("emit");
    bus_b.emit(ch, key(1, 1), vec![0xAA]).expect("emit");

    // Order C: (1,2), (1,1), (2,1)
    let bus_c = MaterializationBus::new();
    bus_c.emit(ch, key(1, 2), vec![0xCC]).expect("emit");
    bus_c.emit(ch, key(1, 1), vec![0xAA]).expect("emit");
    bus_c.emit(ch, key(2, 1), vec![0xBB]).expect("emit");

    let report_a = bus_a.finalize();
    assert!(report_a.is_ok());
    let report_b = bus_b.finalize();
    assert!(report_b.is_ok());
    let report_c = bus_c.finalize();
    assert!(report_c.is_ok());

    assert_eq!(
        report_a.channels[0].data, report_b.channels[0].data,
        "A == B"
    );
    assert_eq!(
        report_b.channels[0].data, report_c.channels[0].data,
        "B == C"
    );
}

/// Log policy preserves all emissions, no drops.
#[test]
fn bus_log_preserves_all_emissions_no_drops() {
    let ch = make_channel_id("test:preserve");
    let bus = MaterializationBus::new();

    // Emit 5 items
    for i in 0..5 {
        bus.emit(ch, key(i, 1), vec![i]).expect("emit");
    }

    let report = bus.finalize();
    assert!(report.is_ok());
    let data = &report.channels[0].data;

    // Count entries (each is 4-byte length + 1-byte data)
    let mut count = 0;
    let mut offset = 0;
    while offset < data.len() {
        let len = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        offset += 4 + len;
        count += 1;
    }

    assert_eq!(count, 5, "all 5 emissions must be preserved");
}

/// Multiple channels finalize in deterministic order (BTreeMap order).
#[test]
fn bus_deterministic_channel_iteration() {
    let ch1 = make_channel_id("channel:aaa");
    let ch2 = make_channel_id("channel:bbb");
    let ch3 = make_channel_id("channel:ccc");

    // Insert in "wrong" order
    let bus = MaterializationBus::new();
    bus.emit(ch2, key(1, 1), vec![2]).expect("emit");
    bus.emit(ch3, key(1, 1), vec![3]).expect("emit");
    bus.emit(ch1, key(1, 1), vec![1]).expect("emit");

    let report = bus.finalize();
    assert!(report.is_ok());
    assert_eq!(report.channels.len(), 3);

    // Channels must be in ChannelId order (which is deterministic per label)
    let ids: Vec<ChannelId> = report.channels.iter().map(|r| r.channel).collect();

    // Verify ordering is consistent
    assert!(ids[0] != ids[1], "channels should differ");
    assert!(ids[1] != ids[2], "channels should differ");

    // More importantly: re-run produces same order
    let bus2 = MaterializationBus::new();
    bus2.emit(ch3, key(1, 1), vec![3]).expect("emit");
    bus2.emit(ch1, key(1, 1), vec![1]).expect("emit");
    bus2.emit(ch2, key(1, 1), vec![2]).expect("emit");

    let report2 = bus2.finalize();
    assert!(report2.is_ok());
    let ids2: Vec<ChannelId> = report2.channels.iter().map(|r| r.channel).collect();

    assert_eq!(ids, ids2, "channel order must be deterministic");
}

// =============================================================================
// TIER 0: POLICY SEMANTICS (D)
// =============================================================================

/// Log policy outputs all emissions in EmitKey order.
#[test]
fn log_policy_outputs_all_in_emitkey_order() {
    let ch = make_channel_id("test:log-order");
    let bus = MaterializationBus::new();

    // Emit in reverse order
    bus.emit(ch, key(3, 1), vec![0x33]).expect("emit");
    bus.emit(ch, key(1, 1), vec![0x11]).expect("emit");
    bus.emit(ch, key(2, 1), vec![0x22]).expect("emit");

    let report = bus.finalize();
    assert!(report.is_ok());
    let data = &report.channels[0].data;

    // Extract values in order
    let mut values = Vec::new();
    let mut offset = 0;
    while offset < data.len() {
        let len = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        values.push(data[offset + 4]);
        offset += 4 + len;
    }

    // Must be in EmitKey order: h(1) < h(2) < h(3)
    assert_eq!(values, vec![0x11, 0x22, 0x33]);
}

/// StrictSingle accepts exactly one emission.
#[test]
fn strict_single_accepts_one_emission() {
    let ch = make_channel_id("test:strict-one");
    let mut bus = MaterializationBus::new();
    bus.register_channel(ch, ChannelPolicy::StrictSingle);

    bus.emit(ch, key(1, 1), vec![42]).expect("emit");

    let report = bus.finalize();
    assert!(report.is_ok());
    assert_eq!(report.channels.len(), 1);
    assert_eq!(report.channels[0].data, vec![42]);
}

/// StrictSingle rejects two emissions (deterministic error).
#[test]
fn strict_single_rejects_two_emissions() {
    let ch = make_channel_id("test:strict-two");
    let mut bus = MaterializationBus::new();
    bus.register_channel(ch, ChannelPolicy::StrictSingle);

    bus.emit(ch, key(1, 1), vec![1]).expect("emit");
    bus.emit(ch, key(2, 1), vec![2]).expect("emit");

    let report = bus.finalize();
    assert!(report.has_errors());

    let err = &report.errors[0];
    assert_eq!(err.channel, ch);
    assert_eq!(err.emission_count, 2);
}

/// StrictSingle error is deterministic regardless of emission order.
#[test]
fn strict_single_error_is_deterministic() {
    let ch = make_channel_id("test:strict-det");

    // Order A
    let mut bus_a = MaterializationBus::new();
    bus_a.register_channel(ch, ChannelPolicy::StrictSingle);
    bus_a.emit(ch, key(1, 1), vec![1]).expect("emit");
    bus_a.emit(ch, key(2, 1), vec![2]).expect("emit");

    // Order B (reversed)
    let mut bus_b = MaterializationBus::new();
    bus_b.register_channel(ch, ChannelPolicy::StrictSingle);
    bus_b.emit(ch, key(2, 1), vec![2]).expect("emit");
    bus_b.emit(ch, key(1, 1), vec![1]).expect("emit");

    let report_a = bus_a.finalize();
    assert!(report_a.has_errors());
    let report_b = bus_b.finalize();
    assert!(report_b.has_errors());

    assert_eq!(report_a.errors[0].channel, report_b.errors[0].channel);
    assert_eq!(
        report_a.errors[0].emission_count,
        report_b.errors[0].emission_count
    );
}

/// StrictSingle with zero emissions returns empty.
#[test]
fn strict_single_zero_emissions_is_empty() {
    let ch = make_channel_id("test:strict-zero");
    let mut bus = MaterializationBus::new();
    bus.register_channel(ch, ChannelPolicy::StrictSingle);

    // Don't emit anything to this channel
    let report = bus.finalize();
    assert!(report.is_ok());
    assert!(report.channels.is_empty());
}

/// Default policy is Log.
#[test]
fn default_policy_is_log() {
    let bus = MaterializationBus::new();
    let ch = make_channel_id("test:unregistered");
    assert_eq!(bus.policy(&ch), ChannelPolicy::Log);
}

// =============================================================================
// TIER 2: PORT/SUBSCRIPTION TESTS (H, I)
// =============================================================================

/// Port only emits frames for subscribed channels.
#[test]
fn port_only_emits_subscribed_channels() {
    let mut port = MaterializationPort::new();
    let ch1 = make_channel_id("ch:subscribed");
    let ch2 = make_channel_id("ch:not-subscribed");

    port.subscribe(ch1);
    // ch2 NOT subscribed

    use warp_core::materialization::FinalizedChannel;
    port.receive_finalized(vec![
        FinalizedChannel {
            channel: ch1,
            data: vec![1],
        },
        FinalizedChannel {
            channel: ch2,
            data: vec![2],
        },
    ]);

    let frames = port.drain();
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].channel, ch1);
}

/// Subscribe then drain receives next commit.
#[test]
fn port_subscribe_then_drain_receives_next_commit() {
    let mut port = MaterializationPort::new();
    let ch = make_channel_id("test:sub-drain");

    port.subscribe(ch);

    use warp_core::materialization::FinalizedChannel;
    port.receive_finalized(vec![FinalizedChannel {
        channel: ch,
        data: vec![0xDE, 0xAD],
    }]);

    let frames = port.drain();
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].data, vec![0xDE, 0xAD]);
}

/// Unsubscribe stops future delivery but cache updates.
#[test]
fn port_unsubscribe_stops_delivery_cache_still_updates() {
    let mut port = MaterializationPort::new();
    let ch = make_channel_id("test:unsub");

    port.subscribe(ch);

    use warp_core::materialization::FinalizedChannel;

    // First commit
    port.receive_finalized(vec![FinalizedChannel {
        channel: ch,
        data: vec![1],
    }]);
    assert_eq!(port.drain().len(), 1);

    // Unsubscribe
    port.unsubscribe(ch);

    // Second commit
    port.receive_finalized(vec![FinalizedChannel {
        channel: ch,
        data: vec![2],
    }]);

    // No frames queued
    assert!(port.drain().is_empty());

    // But cache is updated
    assert_eq!(port.peek_cache(&ch), Some(&vec![2]));
}

/// Late subscriber gets cached value (replay(1)).
#[test]
fn late_subscriber_gets_cached_value() {
    let mut port = MaterializationPort::new();
    let ch = make_channel_id("test:late");

    use warp_core::materialization::FinalizedChannel;

    // Data arrives before subscription
    port.receive_finalized(vec![FinalizedChannel {
        channel: ch,
        data: vec![0xCA, 0xFE],
    }]);

    // Late subscribe - should get cached value
    let cached = port.subscribe(ch);
    assert_eq!(cached, Some(vec![0xCA, 0xFE]));
}

/// Replay is stable (same cached value on re-subscribe).
#[test]
fn replay_is_stable() {
    let mut port = MaterializationPort::new();
    let ch = make_channel_id("test:replay-stable");

    use warp_core::materialization::FinalizedChannel;

    port.receive_finalized(vec![FinalizedChannel {
        channel: ch,
        data: vec![0x42],
    }]);

    // Subscribe twice
    let cached1 = port.subscribe(ch);
    port.unsubscribe(ch);
    let cached2 = port.subscribe(ch);

    assert_eq!(cached1, cached2);
}

// =============================================================================
// TIER 3: PERMUTATION TESTS (J)
// =============================================================================

/// Generate all permutations of a slice.
fn permutations<T: Clone>(items: &[T]) -> Vec<Vec<T>> {
    if items.is_empty() {
        return vec![vec![]];
    }
    if items.len() == 1 {
        return vec![vec![items[0].clone()]];
    }

    let mut result = Vec::new();
    for i in 0..items.len() {
        let mut rest: Vec<T> = items.to_vec();
        let item = rest.remove(i);
        for mut perm in permutations(&rest) {
            perm.insert(0, item.clone());
            result.push(perm);
        }
    }
    result
}

/// Exhaustive permutation test for N=4 emissions.
#[test]
fn permutation_suite_n4_is_order_independent() {
    let ch = make_channel_id("test:perm4");

    // 4 emissions with distinct keys
    let emissions: Vec<(EmitKey, Vec<u8>)> = vec![
        (key(1, 1), vec![0x11]),
        (key(1, 2), vec![0x12]),
        (key(2, 1), vec![0x21]),
        (key(2, 2), vec![0x22]),
    ];

    // Get reference result (natural order)
    let ref_bus = MaterializationBus::new();
    for (k, d) in &emissions {
        ref_bus.emit(ch, *k, d.clone()).expect("emit");
    }
    let ref_report = ref_bus.finalize();
    assert!(ref_report.is_ok());
    let ref_data = &ref_report.channels[0].data;

    // Test all 4! = 24 permutations
    let perms = permutations(&emissions);
    assert_eq!(perms.len(), 24);

    for (i, perm) in perms.iter().enumerate() {
        let bus = MaterializationBus::new();
        for (k, d) in perm {
            bus.emit(ch, *k, d.clone()).expect("emit");
        }
        let report = bus.finalize();
        assert!(!report.has_errors(), "perm {}", i);
        assert_eq!(
            &report.channels[0].data, ref_data,
            "permutation {} should match reference",
            i
        );
    }
}

/// Exhaustive permutation test with subkeys.
#[test]
fn permutation_suite_with_subkeys() {
    let ch = make_channel_id("test:perm-subkey");

    // Same (scope, rule) but different subkeys
    let emissions: Vec<(EmitKey, Vec<u8>)> = vec![
        (key_sub(1, 1, 2), vec![0x02]),
        (key_sub(1, 1, 0), vec![0x00]),
        (key_sub(1, 1, 1), vec![0x01]),
    ];

    // Reference
    let ref_bus = MaterializationBus::new();
    for (k, d) in &emissions {
        ref_bus.emit(ch, *k, d.clone()).expect("emit");
    }
    let ref_report = ref_bus.finalize();
    assert!(ref_report.is_ok());
    let ref_data = &ref_report.channels[0].data;

    // All 3! = 6 permutations
    let perms = permutations(&emissions);
    assert_eq!(perms.len(), 6);

    for (i, perm) in perms.iter().enumerate() {
        let bus = MaterializationBus::new();
        for (k, d) in perm {
            bus.emit(ch, *k, d.clone()).expect("emit");
        }
        let report = bus.finalize();
        assert!(!report.has_errors(), "perm {}", i);
        assert_eq!(
            &report.channels[0].data, ref_data,
            "permutation {} should match",
            i
        );
    }
}

/// Permutation test for multiple channels.
#[test]
fn permutation_suite_multi_channel() {
    let ch_a = make_channel_id("test:perm-ch-a");
    let ch_b = make_channel_id("test:perm-ch-b");

    // Emissions to two channels
    let emissions: Vec<(ChannelId, EmitKey, Vec<u8>)> = vec![
        (ch_a, key(1, 1), vec![0xA1]),
        (ch_b, key(1, 1), vec![0xB1]),
        (ch_a, key(2, 1), vec![0xA2]),
        (ch_b, key(2, 1), vec![0xB2]),
    ];

    // Reference
    let ref_bus = MaterializationBus::new();
    for (c, k, d) in &emissions {
        ref_bus.emit(*c, *k, d.clone()).expect("emit");
    }
    let ref_report = ref_bus.finalize();
    assert!(ref_report.is_ok());

    // All 4! = 24 permutations
    let perms = permutations(&emissions);
    assert_eq!(perms.len(), 24);

    for (i, perm) in perms.iter().enumerate() {
        let bus = MaterializationBus::new();
        for (c, k, d) in perm {
            bus.emit(*c, *k, d.clone()).expect("emit");
        }
        let report = bus.finalize();
        assert!(!report.has_errors(), "perm {}", i);

        // Must have same number of channels
        assert_eq!(
            report.channels.len(),
            ref_report.channels.len(),
            "perm {} channel count",
            i
        );

        // Each channel's data must match
        for (r, rr) in report.channels.iter().zip(ref_report.channels.iter()) {
            assert_eq!(r.channel, rr.channel, "perm {} channel id", i);
            assert_eq!(r.data, rr.data, "perm {} channel data", i);
        }
    }
}

// =============================================================================
// SPEC POLICE: ARCHITECTURE GUARDS
// =============================================================================

/// EmitKey does NOT depend on scheduler-internal nonce.
/// This test documents that EmitKey fields are computable from executor context.
#[test]
fn emit_key_uses_only_executor_available_fields() {
    // These fields are available in executor context:
    let _scope_hash: [u8; 32] = [0; 32]; // From scope node
    let _rule_id: u32 = 0; // From RuleRegistry
    let _subkey: u32 = 0; // Caller-provided

    // If EmitKey had a `nonce` field that required scheduler internals,
    // this test would need to document that dependency.
    //
    // Current EmitKey::new() only requires scope_hash and rule_id,
    // which are both available in the executor without scheduler access.

    let key = EmitKey::new([0; 32], 42);
    assert_eq!(key.subkey, 0, "default subkey must be 0");

    // subkey is caller-provided for multi-emission cases
    let key2 = EmitKey::with_subkey([0; 32], 42, 99);
    assert_eq!(key2.subkey, 99);
}

/// Bus clear removes all pending, not just some.
#[test]
fn bus_clear_removes_all_pending() {
    let bus = MaterializationBus::new();
    let ch1 = make_channel_id("ch:clear1");
    let ch2 = make_channel_id("ch:clear2");

    bus.emit(ch1, key(1, 1), vec![1]).expect("emit");
    bus.emit(ch2, key(1, 1), vec![2]).expect("emit");

    assert!(!bus.is_empty());
    bus.clear();
    assert!(bus.is_empty());

    let report = bus.finalize();
    assert!(report.is_ok());
    assert!(report.channels.is_empty());
}

/// Port clear removes subscriptions, cache, and pending.
#[test]
fn port_clear_removes_everything() {
    let mut port = MaterializationPort::new();
    let ch = make_channel_id("test:port-clear");

    port.subscribe(ch);

    use warp_core::materialization::FinalizedChannel;
    port.receive_finalized(vec![FinalizedChannel {
        channel: ch,
        data: vec![1],
    }]);

    assert!(port.is_subscribed(&ch));
    assert!(port.has_pending());
    assert!(port.peek_cache(&ch).is_some());

    port.clear();

    assert!(!port.is_subscribed(&ch));
    assert!(!port.has_pending());
    assert!(port.peek_cache(&ch).is_none());
}
