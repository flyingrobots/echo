// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! SPEC Police: Exhaustive permutation tests for MaterializationBus determinism.
//!
//! These tests prove that:
//! - Emission order cannot affect finalized bytes
//! - Duplicate keys are rejected
//! - ReduceOp algebra claims match reality
//!
//! # Design Principles
//!
//! - **No RNG**: All permutations are deterministic (Heap's algorithm)
//! - **Exhaustive for small N**: Tests all N! orderings for N ≤ 6
//! - **Byte-for-byte comparison**: Brutal diffs on failure
//! - **Single source of truth**: `finalize_bytes()` is the canonical output

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::collections::BTreeMap;

use warp_core::materialization::{
    encode_frames, make_channel_id, ChannelId, ChannelPolicy, DuplicateEmission, EmitKey,
    MaterializationBus, MaterializationFrame,
};

// =============================================================================
// PERMUTATION ENGINE (Heap's Algorithm - No RNG)
// =============================================================================

/// Calls `f` for every permutation of `items` in-place.
/// Deterministic: Heap's algorithm generates all N! permutations.
fn for_each_permutation<T: Clone>(items: &mut [T], mut f: impl FnMut(&[T])) {
    let n = items.len();
    if n == 0 {
        f(items);
        return;
    }

    let mut c = vec![0usize; n];

    // First permutation (original order)
    f(items);

    let mut i = 0usize;
    while i < n {
        if c[i] < i {
            if i.is_multiple_of(2) {
                items.swap(0, i);
            } else {
                items.swap(c[i], i);
            }
            f(items);
            c[i] += 1;
            i = 0;
        } else {
            c[i] = 0;
            i += 1;
        }
    }
}

/// Count permutations for sanity checks.
fn factorial(n: usize) -> usize {
    (1..=n).product()
}

// =============================================================================
// BYTE DIFF UTILITY
// =============================================================================

/// Panic with a useful byte diff showing where bytes diverge.
fn assert_bytes_eq(label: &str, expected: &[u8], actual: &[u8]) {
    if expected == actual {
        return;
    }

    let min_len = expected.len().min(actual.len());
    let first_diff = (0..min_len)
        .find(|&i| expected[i] != actual[i])
        .unwrap_or(min_len);

    let context_start = first_diff.saturating_sub(16);
    let context_end_expected = (first_diff + 16).min(expected.len());
    let context_end_actual = (first_diff + 16).min(actual.len());

    panic!(
        "{label}: bytes differ\n\
         len(expected) = {}\n\
         len(actual)   = {}\n\
         first_diff_at = {}\n\
         expected[{context_start}..{context_end_expected}] = {:02x?}\n\
         actual[{context_start}..{context_end_actual}]   = {:02x?}\n",
        expected.len(),
        actual.len(),
        first_diff,
        &expected[context_start..context_end_expected],
        &actual[context_start..context_end_actual],
    );
}

// =============================================================================
// TEST FIXTURE TYPES
// =============================================================================

/// A single emission for testing.
#[derive(Clone, Debug)]
struct Emission {
    ch: ChannelId,
    key: EmitKey,
    data: Vec<u8>,
}

/// Create a deterministic scope_hash for tests (no blake3, just stable bytes).
fn scope_hash(tag: u8) -> [u8; 32] {
    let mut h = [0u8; 32];
    h[0] = 0xE5; // "E" for Echo
    h[1] = 0xC0; // "C" for test Case
    h[31] = tag; // Distinguishing byte
    h
}

/// Create an EmitKey for tests.
fn key(scope_tag: u8, rule_id: u32, subkey: u32) -> EmitKey {
    EmitKey::with_subkey(scope_hash(scope_tag), rule_id, subkey)
}

// =============================================================================
// BUS ADAPTERS
// =============================================================================

/// Create a fresh bus for testing.
fn mk_bus() -> MaterializationBus {
    MaterializationBus::new()
}

/// Finalize the bus and return the canonical wire bytes.
///
/// This is the single source of truth for determinism tests.
/// The output bytes must be identical regardless of emission order.
fn finalize_bytes(bus: &MaterializationBus) -> Vec<u8> {
    // Finalize to FinalizedChannel list
    let report = bus.finalize();
    assert!(report.is_ok(), "finalize should succeed");

    // Convert to MaterializationFrames
    let frames: Vec<MaterializationFrame> = report
        .channels
        .into_iter()
        .map(|fc| MaterializationFrame::new(fc.channel, fc.data))
        .collect();

    // Encode to wire bytes
    encode_frames(&frames)
}

/// Emit all items to the bus. Panics on duplicate key.
fn emit_all(bus: &MaterializationBus, emissions: &[Emission]) {
    for e in emissions {
        bus.emit(e.ch, e.key, e.data.clone())
            .expect("duplicate key in test emissions");
    }
}

/// Emit all items to the bus, returning error on duplicate key.
fn emit_all_result(
    bus: &MaterializationBus,
    emissions: &[Emission],
) -> Result<(), DuplicateEmission> {
    for e in emissions {
        bus.emit(e.ch, e.key, e.data.clone())?;
    }
    Ok(())
}

// =============================================================================
// TIER 1: EMITKEY ORDERING TESTS
// =============================================================================

#[test]
fn emit_key_ord_is_lexicographic_scope_rule_subkey() {
    // Keys with different scope_hash
    let k1 = key(1, 10, 0);
    let k2 = key(2, 5, 0);
    assert!(k1 < k2, "lower scope_hash should come first");

    // Same scope, different rule_id
    let k3 = key(1, 5, 99);
    let k4 = key(1, 10, 0);
    assert!(k3 < k4, "same scope, lower rule_id first");

    // Same scope+rule, different subkey
    let k5 = key(1, 5, 0);
    let k6 = key(1, 5, 1);
    assert!(k5 < k6, "same scope+rule, lower subkey first");
}

#[test]
fn emit_key_btreemap_iteration_is_canonical() {
    // Insert in "wrong" order, verify BTreeMap iteration is canonical
    let mut map = BTreeMap::new();
    map.insert(key(2, 10, 0), "d");
    map.insert(key(1, 5, 1), "b");
    map.insert(key(1, 10, 0), "c");
    map.insert(key(1, 5, 0), "a");

    let values: Vec<_> = map.values().copied().collect();
    assert_eq!(values, vec!["a", "b", "c", "d"]);
}

#[test]
fn emit_key_subkey_from_hash_is_deterministic() {
    let hash = scope_hash(42);
    let s1 = EmitKey::subkey_from_hash(&hash);
    let s2 = EmitKey::subkey_from_hash(&hash);
    assert_eq!(s1, s2, "same input should produce same subkey");

    // Different inputs should produce different subkeys
    // (use a hash that differs in the first 4 bytes, which subkey_from_hash reads)
    let mut hash2 = [0u8; 32];
    hash2[0] = 0xFF;
    hash2[1] = 0xEE;
    let s3 = EmitKey::subkey_from_hash(&hash2);
    assert_ne!(s1, s3, "different inputs should produce different subkeys");
}

// =============================================================================
// TIER 2: DUPLICATE KEY REJECTION
// =============================================================================

#[test]
fn bus_rejects_duplicate_key_same_channel() {
    let ch = make_channel_id("specpolice:dup");
    let k = key(1, 42, 0);

    let bus = mk_bus();
    bus.emit(ch, k, vec![1, 2, 3]).expect("first emit");

    let err = bus
        .emit(ch, k, vec![9, 9, 9])
        .expect_err("duplicate should fail");
    assert_eq!(err.channel, ch);
    assert_eq!(err.key, k);
}

#[test]
fn bus_allows_same_key_different_channels() {
    let ch1 = make_channel_id("specpolice:ch1");
    let ch2 = make_channel_id("specpolice:ch2");
    let k = key(1, 42, 0);

    let bus = mk_bus();
    bus.emit(ch1, k, vec![1]).expect("first channel");
    bus.emit(ch2, k, vec![2])
        .expect("second channel - same key OK");
}

#[test]
fn bus_rejects_duplicate_key_even_if_bytes_identical() {
    let ch = make_channel_id("specpolice:dup-identical");
    let k = key(1, 42, 0);
    let data = vec![0xDE, 0xAD, 0xBE, 0xEF];

    let bus = mk_bus();
    bus.emit(ch, k, data.clone()).expect("first emit");

    // Same key, same bytes - should STILL fail
    let err = bus
        .emit(ch, k, data)
        .expect_err("duplicate should fail even if identical");
    assert_eq!(err.channel, ch);
}

#[test]
fn permutation_invariance_should_fail_fast_on_duplicate_key() {
    // Test that duplicate detection is consistent across all permutations.
    // Even though different orderings might encounter the duplicate at different
    // points, they should ALL fail with the same error (same channel, same key).
    let ch = make_channel_id("specpolice:dup-perm");

    // Create emissions with an intentional duplicate: key(1, 42, 0) appears twice
    let dup_key = key(1, 42, 0);
    let mut emissions = vec![
        Emission {
            ch,
            key: key(1, 1, 0),
            data: vec![0x01],
        },
        Emission {
            ch,
            key: dup_key, // First occurrence
            data: vec![0xAA],
        },
        Emission {
            ch,
            key: key(2, 1, 0),
            data: vec![0x02],
        },
        Emission {
            ch,
            key: dup_key, // Duplicate!
            data: vec![0xBB],
        },
    ];

    // Verify that every permutation fails with the same error
    let mut error_count = 0;
    let mut success_count = 0;

    for_each_permutation(&mut emissions, |perm| {
        let bus = mk_bus();
        match emit_all_result(&bus, perm) {
            Ok(()) => {
                success_count += 1;
            }
            Err(err) => {
                // The error should always identify the duplicated key
                assert_eq!(err.channel, ch, "error should identify the channel");
                assert_eq!(err.key, dup_key, "error should identify the duplicated key");
                error_count += 1;
            }
        }
    });

    // All permutations must fail - no silent success based on insertion order
    assert_eq!(success_count, 0, "no permutation should succeed");
    assert_eq!(
        error_count,
        factorial(4),
        "all 4! permutations should fail with DuplicateEmission"
    );
}

// =============================================================================
// TIER 3: PERMUTATION INVARIANCE ("SPEC POLICE")
// =============================================================================

#[test]
fn log_finalize_is_permutation_invariant_n5() {
    // Two channels, five emissions, deliberately interleaved keys.
    let ch_a = make_channel_id("specpolice:a");
    let ch_b = make_channel_id("specpolice:b");

    let mut emissions = vec![
        Emission {
            ch: ch_a,
            key: key(2, 10, 0),
            data: vec![0xA1],
        },
        Emission {
            ch: ch_b,
            key: key(1, 3, 7),
            data: vec![0xB1, 0xB2],
        },
        Emission {
            ch: ch_a,
            key: key(1, 2, 0),
            data: vec![0xA2, 0xA3],
        },
        Emission {
            ch: ch_b,
            key: key(2, 9, 0),
            data: vec![0xB3],
        },
        Emission {
            ch: ch_a,
            key: key(1, 2, 5),
            data: vec![0xA4],
        },
    ];

    // Reference = emit in original order
    let reference = {
        let bus = mk_bus();
        emit_all(&bus, &emissions);
        finalize_bytes(&bus)
    };

    // Verify we're actually testing all permutations
    let mut perm_count = 0;

    // Brute-force all 5! = 120 permutations
    for_each_permutation(&mut emissions, |perm| {
        let bus = mk_bus();
        emit_all(&bus, perm);
        let out = finalize_bytes(&bus);
        assert_bytes_eq("permutation invariance failed", &reference, &out);
        perm_count += 1;
    });

    assert_eq!(perm_count, factorial(5), "should test all 5! permutations");
}

#[test]
fn log_finalize_is_permutation_invariant_n4_single_channel() {
    let ch = make_channel_id("specpolice:single");

    let mut emissions = vec![
        Emission {
            ch,
            key: key(3, 1, 0),
            data: vec![0x33],
        },
        Emission {
            ch,
            key: key(1, 1, 0),
            data: vec![0x11],
        },
        Emission {
            ch,
            key: key(2, 1, 0),
            data: vec![0x22],
        },
        Emission {
            ch,
            key: key(1, 2, 0),
            data: vec![0x12],
        },
    ];

    let reference = {
        let bus = mk_bus();
        emit_all(&bus, &emissions);
        finalize_bytes(&bus)
    };

    let mut perm_count = 0;

    for_each_permutation(&mut emissions, |perm| {
        let bus = mk_bus();
        emit_all(&bus, perm);
        let out = finalize_bytes(&bus);
        assert_bytes_eq("single channel permutation", &reference, &out);
        perm_count += 1;
    });

    assert_eq!(perm_count, factorial(4));
}

#[test]
fn bus_channel_iteration_is_canonical() {
    // Insert channels in "wrong" order, verify output order is canonical
    let ch_z = make_channel_id("channel:zzz");
    let ch_a = make_channel_id("channel:aaa");
    let ch_m = make_channel_id("channel:mmm");

    let bus = mk_bus();
    bus.emit(ch_z, key(1, 1, 0), vec![0x7A]).expect("emit"); // 'z'
    bus.emit(ch_a, key(1, 1, 0), vec![0x61]).expect("emit"); // 'a'
    bus.emit(ch_m, key(1, 1, 0), vec![0x6D]).expect("emit"); // 'm'

    let report = bus.finalize();
    assert!(report.is_ok());
    let channel_ids: Vec<_> = report.channels.iter().map(|f| f.channel).collect();

    // Channels should be in BTreeMap order (deterministic hash order)
    // We don't know the exact order of make_channel_id hashes, but it must be stable
    let bus2 = mk_bus();
    bus2.emit(ch_a, key(1, 1, 0), vec![0x61]).expect("emit");
    bus2.emit(ch_m, key(1, 1, 0), vec![0x6D]).expect("emit");
    bus2.emit(ch_z, key(1, 1, 0), vec![0x7A]).expect("emit");

    let report2 = bus2.finalize();
    assert!(report2.is_ok());
    let channel_ids2: Vec<_> = report2.channels.iter().map(|f| f.channel).collect();

    assert_eq!(
        channel_ids, channel_ids2,
        "channel order must be deterministic"
    );
}

#[test]
fn bus_log_preserves_all_emissions_no_drops() {
    let ch = make_channel_id("specpolice:preserve");

    let bus = mk_bus();
    for i in 0..5u8 {
        bus.emit(ch, key(i, 1, 0), vec![i, i + 1]).expect("emit");
    }

    let report = bus.finalize();
    assert!(report.is_ok());
    assert_eq!(report.channels.len(), 1, "one channel");

    // Log format: each entry is [u32 len][data...]
    // 5 entries × (4 byte len + 2 byte data) = 30 bytes
    let data = &report.channels[0].data;
    assert_eq!(data.len(), 5 * (4 + 2), "all entries preserved");

    // Count entries
    let mut offset = 0;
    let mut count = 0;
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
    assert_eq!(count, 5, "5 emissions preserved, no coalescing");
}

// =============================================================================
// TIER 4: REDUCE OP CLASSIFICATION (Truth Serum)
// =============================================================================

use warp_core::materialization::ReduceOp;

#[test]
fn reduce_op_commutativity_table_is_honest() {
    // Commutative monoids - must return true
    let commutative = [
        ReduceOp::Sum,
        ReduceOp::Max,
        ReduceOp::Min,
        ReduceOp::BitOr,
        ReduceOp::BitAnd,
    ];

    // Order-dependent - must return false
    let ordered = [ReduceOp::First, ReduceOp::Last, ReduceOp::Concat];

    for op in &commutative {
        assert!(
            op.is_commutative(),
            "{:?} claims to be commutative in docs, is_commutative() must agree",
            op
        );
    }

    for op in &ordered {
        assert!(
            !op.is_commutative(),
            "{:?} is order-dependent, is_commutative() must return false",
            op
        );
    }
}

#[test]
fn reduce_empty_input_returns_specified_identity() {
    let empty: Vec<Vec<u8>> = vec![];

    // Sum returns [0u8; 8] (zero)
    assert_eq!(ReduceOp::Sum.apply(empty.clone()), vec![0u8; 8]);

    // All others return []
    assert_eq!(ReduceOp::Max.apply(empty.clone()), vec![]);
    assert_eq!(ReduceOp::Min.apply(empty.clone()), vec![]);
    assert_eq!(ReduceOp::First.apply(empty.clone()), vec![]);
    assert_eq!(ReduceOp::Last.apply(empty.clone()), vec![]);
    assert_eq!(ReduceOp::BitOr.apply(empty.clone()), vec![]);
    assert_eq!(ReduceOp::BitAnd.apply(empty.clone()), vec![]);
    assert_eq!(ReduceOp::Concat.apply(empty), vec![]);
}

#[test]
fn reduce_sum_is_permutation_invariant() {
    let values = vec![
        vec![1, 0, 0, 0, 0, 0, 0, 0], // 1 as u64 LE
        vec![2, 0, 0, 0, 0, 0, 0, 0], // 2
        vec![3, 0, 0, 0, 0, 0, 0, 0], // 3
    ];

    let expected = vec![6, 0, 0, 0, 0, 0, 0, 0]; // 1+2+3 = 6

    let mut values_mut = values.clone();
    for_each_permutation(&mut values_mut, |perm| {
        let result = ReduceOp::Sum.apply(perm.iter().cloned());
        assert_eq!(result, expected, "Sum must be permutation invariant");
    });
}

#[test]
fn reduce_first_picks_smallest_key_value() {
    // This test requires bus-level integration since ReduceOp.apply()
    // receives values in EmitKey order. The test verifies that
    // regardless of insertion order, First picks the smallest-key value.

    let ch = make_channel_id("reduce:first");
    let mut bus = mk_bus();
    bus.register_channel(ch, ChannelPolicy::Reduce(ReduceOp::First));

    // Emit in reverse key order
    bus.emit(ch, key(3, 1, 0), vec![0x33]).expect("emit");
    bus.emit(ch, key(1, 1, 0), vec![0x11]).expect("emit"); // This should win (smallest key)
    bus.emit(ch, key(2, 1, 0), vec![0x22]).expect("emit");

    let report = bus.finalize();
    assert!(report.is_ok());
    assert_eq!(
        report.channels[0].data,
        vec![0x11],
        "First should pick smallest key"
    );
}

// =============================================================================
// PERMUTATION HELPER: Reusable runner for policy tests
// =============================================================================

/// Run permutation invariance test for any policy.
/// Returns the reference bytes for further assertions.
#[allow(dead_code)]
fn run_permutation_invariance(
    emissions: Vec<Emission>,
    _policy_setup: impl Fn(&mut MaterializationBus),
) -> Vec<u8> {
    let mut emissions = emissions;

    let reference = {
        let mut bus = mk_bus();
        _policy_setup(&mut bus);
        emit_all(&bus, &emissions);
        finalize_bytes(&bus)
    };

    for_each_permutation(&mut emissions, |perm| {
        let mut bus = mk_bus();
        _policy_setup(&mut bus);
        emit_all(&bus, perm);
        let out = finalize_bytes(&bus);
        assert_bytes_eq("policy permutation invariance", &reference, &out);
    });

    reference
}

// =============================================================================
// SPEC GUARDRAIL: EmitKey computable from executor context
// =============================================================================

#[test]
fn emit_key_is_computable_without_scheduler_internals() {
    // This documents the API contract: EmitKey fields come from executor context.
    // If this test needs to change, it's a breaking change to the emission model.

    // Executor has access to:
    let scope_hash: [u8; 32] = [0xAB; 32]; // From scope node content hash
    let rule_id: u32 = 42; // From RuleRegistry
    let subkey: u32 = 7; // Caller-provided or from subkey_from_hash

    // EmitKey can be constructed from these fields alone
    let key = EmitKey::with_subkey(scope_hash, rule_id, subkey);

    assert_eq!(key.scope_hash, scope_hash);
    assert_eq!(key.rule_id, rule_id);
    assert_eq!(key.subkey, subkey);
}

// =============================================================================
// TIER 5: CONFLICT DOES NOT ERASE OTHER CHANNELS
// =============================================================================
//
// When one channel has a StrictSingle conflict (or any error), other channels
// must still be preserved in report.channels. The FinalizeReport pattern ensures
// that errors are partitioned from successes, not a short-circuit failure.

use warp_core::materialization::MaterializationErrorKind;

#[test]
fn conflict_in_one_channel_preserves_other_channels() {
    // Setup: Channel A has StrictSingle policy, Channel B uses default Log.
    // Channel A receives 2 emissions (conflict!), Channel B receives 1 emission (OK).
    // Expected: B appears in report.channels, A appears in report.errors.

    let ch_a = make_channel_id("conflict:strict_a");
    let ch_b = make_channel_id("conflict:log_b");

    let mut bus = mk_bus();
    bus.register_channel(ch_a, ChannelPolicy::StrictSingle);
    // ch_b uses default Log policy

    // Emit to both channels - A will conflict, B will succeed
    bus.emit(ch_a, key(1, 1, 0), vec![0xA1])
        .expect("emit A first");
    bus.emit(ch_a, key(2, 1, 0), vec![0xA2])
        .expect("emit A second - causes conflict");
    bus.emit(ch_b, key(1, 1, 0), vec![0xB1]).expect("emit B");

    let report = bus.finalize();

    // Channel B should be preserved in channels
    assert_eq!(report.channels.len(), 1, "one channel should succeed (B)");
    assert_eq!(
        report.channels[0].channel, ch_b,
        "channel B should be in report.channels"
    );

    // Channel A should be in errors
    assert_eq!(report.errors.len(), 1, "one channel should have error (A)");
    assert_eq!(
        report.errors[0].channel, ch_a,
        "channel A should be in report.errors"
    );
    assert_eq!(
        report.errors[0].kind,
        MaterializationErrorKind::StrictSingleConflict
    );
    assert_eq!(
        report.errors[0].emission_count, 2,
        "conflict should report 2 emissions"
    );

    // Overall report should have errors
    assert!(report.has_errors(), "report should indicate errors");
    assert!(!report.is_ok(), "report.is_ok() should be false");
}

#[test]
fn multiple_channels_with_conflicts_preserves_non_conflicting() {
    // Setup: 4 channels total
    //   - ch_strict1: StrictSingle, receives 2 emissions (conflict)
    //   - ch_strict2: StrictSingle, receives 3 emissions (conflict)
    //   - ch_log1: Log, receives 2 emissions (OK)
    //   - ch_log2: Log, receives 1 emission (OK)
    //
    // Expected:
    //   - report.channels contains ch_log1 and ch_log2
    //   - report.errors contains ch_strict1 and ch_strict2

    let ch_strict1 = make_channel_id("multi:strict1");
    let ch_strict2 = make_channel_id("multi:strict2");
    let ch_log1 = make_channel_id("multi:log1");
    let ch_log2 = make_channel_id("multi:log2");

    let mut bus = mk_bus();
    bus.register_channel(ch_strict1, ChannelPolicy::StrictSingle);
    bus.register_channel(ch_strict2, ChannelPolicy::StrictSingle);
    // ch_log1 and ch_log2 use default Log policy

    // Emit to all channels in interleaved order
    bus.emit(ch_log1, key(1, 1, 0), vec![0x11]).expect("emit");
    bus.emit(ch_strict1, key(1, 1, 0), vec![0x21])
        .expect("emit");
    bus.emit(ch_strict2, key(1, 1, 0), vec![0x31])
        .expect("emit");
    bus.emit(ch_log2, key(1, 1, 0), vec![0x41]).expect("emit");
    bus.emit(ch_strict1, key(2, 1, 0), vec![0x22])
        .expect("emit"); // conflict for strict1
    bus.emit(ch_strict2, key(2, 1, 0), vec![0x32])
        .expect("emit"); // conflict for strict2
    bus.emit(ch_strict2, key(3, 1, 0), vec![0x33])
        .expect("emit"); // more conflict for strict2
    bus.emit(ch_log1, key(2, 1, 0), vec![0x12]).expect("emit");

    let report = bus.finalize();

    // Both log channels should succeed
    assert_eq!(
        report.channels.len(),
        2,
        "two channels should succeed (log1 and log2)"
    );
    let successful_ids: Vec<_> = report.channels.iter().map(|c| c.channel).collect();
    assert!(
        successful_ids.contains(&ch_log1),
        "ch_log1 should be in report.channels"
    );
    assert!(
        successful_ids.contains(&ch_log2),
        "ch_log2 should be in report.channels"
    );

    // Both strict channels should have errors
    assert_eq!(
        report.errors.len(),
        2,
        "two channels should have errors (strict1 and strict2)"
    );
    let error_ids: Vec<_> = report.errors.iter().map(|e| e.channel).collect();
    assert!(
        error_ids.contains(&ch_strict1),
        "ch_strict1 should be in report.errors"
    );
    assert!(
        error_ids.contains(&ch_strict2),
        "ch_strict2 should be in report.errors"
    );

    // Verify emission counts
    let strict1_err = report
        .errors
        .iter()
        .find(|e| e.channel == ch_strict1)
        .unwrap();
    let strict2_err = report
        .errors
        .iter()
        .find(|e| e.channel == ch_strict2)
        .unwrap();
    assert_eq!(strict1_err.emission_count, 2, "strict1 had 2 emissions");
    assert_eq!(strict2_err.emission_count, 3, "strict2 had 3 emissions");
}

#[test]
fn conflict_does_not_affect_other_channel_data_integrity() {
    // Verify that the actual DATA in non-conflicting channels is preserved correctly,
    // not just the count. This ensures finalization doesn't corrupt data on errors.

    let ch_conflict = make_channel_id("integrity:conflict");
    let ch_ok = make_channel_id("integrity:ok");

    let mut bus = mk_bus();
    bus.register_channel(ch_conflict, ChannelPolicy::StrictSingle);

    // Emit to OK channel with known data
    bus.emit(ch_ok, key(1, 1, 0), vec![0xDE, 0xAD])
        .expect("emit");
    bus.emit(ch_ok, key(2, 1, 0), vec![0xBE, 0xEF])
        .expect("emit");

    // Emit to conflict channel
    bus.emit(ch_conflict, key(1, 1, 0), vec![0x01])
        .expect("emit");
    bus.emit(ch_conflict, key(2, 1, 0), vec![0x02])
        .expect("emit");

    let report = bus.finalize();

    // Verify ch_ok data is intact (Log format: [u32 len][data][u32 len][data]...)
    assert_eq!(report.channels.len(), 1);
    let ok_channel = &report.channels[0];
    assert_eq!(ok_channel.channel, ch_ok);

    // Parse the Log-formatted data: should have two entries in EmitKey order
    let data = &ok_channel.data;
    // First entry: len=2, data=[0xDE, 0xAD]
    assert_eq!(&data[0..4], &2u32.to_le_bytes());
    assert_eq!(&data[4..6], &[0xDE, 0xAD]);
    // Second entry: len=2, data=[0xBE, 0xEF]
    assert_eq!(&data[6..10], &2u32.to_le_bytes());
    assert_eq!(&data[10..12], &[0xBE, 0xEF]);
    assert_eq!(data.len(), 12, "exactly 2 entries");

    // Verify error is present
    assert_eq!(report.errors.len(), 1);
    assert_eq!(report.errors[0].channel, ch_conflict);
}

#[test]
fn all_channels_conflict_produces_empty_channels_vec() {
    // Edge case: ALL channels have conflicts. report.channels should be empty,
    // and report.errors should contain all of them.

    let ch1 = make_channel_id("allconflict:1");
    let ch2 = make_channel_id("allconflict:2");

    let mut bus = mk_bus();
    bus.register_channel(ch1, ChannelPolicy::StrictSingle);
    bus.register_channel(ch2, ChannelPolicy::StrictSingle);

    // Both channels conflict
    bus.emit(ch1, key(1, 1, 0), vec![1]).expect("emit");
    bus.emit(ch1, key(2, 1, 0), vec![2]).expect("emit");
    bus.emit(ch2, key(1, 1, 0), vec![3]).expect("emit");
    bus.emit(ch2, key(2, 1, 0), vec![4]).expect("emit");

    let report = bus.finalize();

    assert!(
        report.channels.is_empty(),
        "no channels should succeed when all conflict"
    );
    assert_eq!(report.errors.len(), 2, "both channels should be in errors");
    assert!(report.has_errors());
    assert!(!report.is_ok());
}

#[test]
fn single_channel_conflict_still_reported_correctly() {
    // Edge case: Only one channel exists and it has a conflict.
    // Ensures we don't have any special-case bugs for single-channel scenarios.

    let ch = make_channel_id("singleconflict:only");

    let mut bus = mk_bus();
    bus.register_channel(ch, ChannelPolicy::StrictSingle);

    bus.emit(ch, key(1, 1, 0), vec![0x01]).expect("emit");
    bus.emit(ch, key(2, 1, 0), vec![0x02]).expect("emit");

    let report = bus.finalize();

    assert!(report.channels.is_empty(), "no channels should succeed");
    assert_eq!(report.errors.len(), 1, "one error");
    assert_eq!(report.errors[0].channel, ch);
    assert_eq!(
        report.errors[0].kind,
        MaterializationErrorKind::StrictSingleConflict
    );
}

#[test]
fn conflict_partition_is_permutation_invariant() {
    // The partition of channels into successes/errors should be deterministic
    // regardless of emission order. This is a stricter test than just "data is
    // the same" - we verify the FinalizeReport structure itself is stable.

    let ch_strict = make_channel_id("partition:strict");
    let ch_log = make_channel_id("partition:log");

    let mut emissions = vec![
        Emission {
            ch: ch_strict,
            key: key(1, 1, 0),
            data: vec![0x01],
        },
        Emission {
            ch: ch_log,
            key: key(1, 1, 0),
            data: vec![0x11],
        },
        Emission {
            ch: ch_strict,
            key: key(2, 1, 0),
            data: vec![0x02], // This causes conflict
        },
        Emission {
            ch: ch_log,
            key: key(2, 1, 0),
            data: vec![0x12],
        },
    ];

    // Get reference report structure
    let reference_report = {
        let mut bus = mk_bus();
        bus.register_channel(ch_strict, ChannelPolicy::StrictSingle);
        emit_all(&bus, &emissions);
        bus.finalize()
    };

    // Verify all permutations produce the same partition
    let mut perm_count = 0;
    for_each_permutation(&mut emissions, |perm| {
        let mut bus = mk_bus();
        bus.register_channel(ch_strict, ChannelPolicy::StrictSingle);
        emit_all(&bus, perm);
        let report = bus.finalize();

        // Same number of successes and errors
        assert_eq!(
            report.channels.len(),
            reference_report.channels.len(),
            "success count should match"
        );
        assert_eq!(
            report.errors.len(),
            reference_report.errors.len(),
            "error count should match"
        );

        // Same channels in successes
        let ref_success_ids: Vec<_> = reference_report
            .channels
            .iter()
            .map(|c| c.channel)
            .collect();
        let success_ids: Vec<_> = report.channels.iter().map(|c| c.channel).collect();
        assert_eq!(success_ids, ref_success_ids, "same channels should succeed");

        // Same channels in errors
        let ref_error_ids: Vec<_> = reference_report.errors.iter().map(|e| e.channel).collect();
        let error_ids: Vec<_> = report.errors.iter().map(|e| e.channel).collect();
        assert_eq!(error_ids, ref_error_ids, "same channels should have errors");

        // Same data in successful channel
        assert_eq!(
            report.channels[0].data, reference_report.channels[0].data,
            "successful channel data should match"
        );

        perm_count += 1;
    });

    assert_eq!(perm_count, factorial(4), "should test all 4! permutations");
}
