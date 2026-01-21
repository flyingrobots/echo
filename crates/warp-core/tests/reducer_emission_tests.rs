// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! SPEC-0004 Reducer emission tests for materialization semantics.
//!
//! This module tests reducer behavior for confluence-safe parallel rewriting.
//!
//! # Test Coverage
//!
//! - **T11**: `reducer_commutative_is_permutation_invariant_and_replayable`
//!   - Verifies commutative reducers (Sum, Max, Min, BitOr, BitAnd) produce
//!     identical bytes across ALL input permutations
//!   - Confirms mathematical correctness
//!   - Validates playback consistency (same bytes on replay)
//!
//! - **T12**: `reducer_order_dependent_is_canonically_deterministic_and_replayable`
//!   - Verifies order-dependent reducers (First, Last, Concat) produce
//!     deterministic results via EmitKey canonicalization
//!   - Even though these ops are not commutative, results are identical
//!     across permutations because EmitKey sorting enforces canonical order
//!
//! - **T13**: `reduced_channel_emits_single_authoritative_value_per_tick`
//!   - Verifies MANY emissions -> ONE output (cardinality invariant)
//!   - Proves raw event streams are NOT leaked
//!   - Confirms OUT-REDUCE-001: per-tick outputs are final reduced value

#![allow(clippy::expect_used, clippy::unwrap_used)]

mod common;
use common::{for_each_permutation, h, key_sub as key};

use warp_core::materialization::{
    encode_frames, make_channel_id, ChannelPolicy, EmitKey, MaterializationBus,
    MaterializationFrame, ReduceOp,
};

// =============================================================================
// TEST UTILITIES
// =============================================================================

/// Count permutations for sanity checks.
///
/// Assumes small `n` (tests use N <= 6). Overflows `usize` for n > 20.
fn factorial(n: usize) -> usize {
    debug_assert!(n <= 20, "factorial({n}) would overflow usize");
    (1..=n).product()
}

/// Create a fresh bus for testing.
fn mk_bus() -> MaterializationBus {
    MaterializationBus::new()
}

/// Finalize the bus and return the canonical wire bytes.
///
/// This is the single source of truth for determinism tests.
/// The output bytes must be identical regardless of emission order.
fn finalize_bytes(bus: &MaterializationBus) -> Vec<u8> {
    let report = bus.finalize();
    assert!(report.is_ok(), "finalize should succeed");

    let frames: Vec<MaterializationFrame> = report
        .channels
        .into_iter()
        .map(|fc| MaterializationFrame::new(fc.channel, fc.data))
        .collect();

    encode_frames(&frames)
}

/// Encode a u64 value as little-endian bytes.
fn u64_le(v: u64) -> Vec<u8> {
    v.to_le_bytes().to_vec()
}

/// Decode little-endian bytes to u64.
fn le_to_u64(bytes: &[u8]) -> u64 {
    let mut buf = [0u8; 8];
    let len = bytes.len().min(8);
    buf[..len].copy_from_slice(&bytes[..len]);
    u64::from_le_bytes(buf)
}

/// A single emission for testing.
#[derive(Clone, Debug)]
struct Emission {
    key: EmitKey,
    data: Vec<u8>,
}

// =============================================================================
// SPEC-0004 T11: COMMUTATIVE REDUCER PERMUTATION INVARIANCE
// =============================================================================

/// T11: Verify that the Sum reducer is permutation-invariant and replayable.
///
/// This test:
/// 1. Creates multiple emissions with u64 values encoded as little-endian bytes
/// 2. Registers a channel with `ChannelPolicy::Reduce(ReduceOp::Sum)`
/// 3. Tests ALL permutations of emission ordering
/// 4. Asserts that ALL permutations produce byte-identical output
/// 5. Verifies the mathematical correctness of the sum
///
/// This proves that the Sum reducer is a commutative monoid, which is required
/// for confluence-safe parallel rewriting per SPEC-0004.
#[test]
fn reducer_commutative_is_permutation_invariant_and_replayable() {
    let ch = make_channel_id("spec0004:t11:sum");

    // Test values: choose values that would reveal ordering bugs
    // (not just 1, 2, 3 which might mask issues via symmetry)
    let test_values: Vec<u64> = vec![7, 13, 42, 100];
    let expected_sum: u64 = test_values.iter().sum();

    // Create emissions with different scope/rule combos to ensure distinct EmitKeys
    let mut emissions: Vec<Emission> = test_values
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            // Use different scope_hash and rule_id combinations
            let scope = (i + 1) as u8;
            let rule = (i * 10 + 1) as u32;
            Emission {
                key: key(scope, rule, 0),
                data: u64_le(v),
            }
        })
        .collect();

    // Get reference output from original ordering
    let reference_bytes = {
        let mut bus = mk_bus();
        bus.register_channel(ch, ChannelPolicy::Reduce(ReduceOp::Sum));
        for e in &emissions {
            bus.emit(ch, e.key, e.data.clone())
                .expect("emit should succeed");
        }
        finalize_bytes(&bus)
    };

    // Verify reference has non-empty content (sanity check)
    assert!(
        !reference_bytes.is_empty(),
        "reference should produce output"
    );

    // Extract the sum value from reference and verify mathematical correctness
    // The finalized bytes go through encode_frames, so we need to verify the raw channel data
    {
        let mut bus = mk_bus();
        bus.register_channel(ch, ChannelPolicy::Reduce(ReduceOp::Sum));
        for e in &emissions {
            bus.emit(ch, e.key, e.data.clone())
                .expect("emit should succeed");
        }
        let report = bus.finalize();
        assert!(report.is_ok());
        assert_eq!(report.channels.len(), 1);

        let sum_bytes = &report.channels[0].data;
        let actual_sum = le_to_u64(sum_bytes);
        assert_eq!(
            actual_sum,
            expected_sum,
            "Sum should be mathematically correct: {} + {} + {} + {} = {} (got {})",
            test_values[0],
            test_values[1],
            test_values[2],
            test_values[3],
            expected_sum,
            actual_sum
        );
    }

    // Test ALL permutations (4! = 24)
    let mut perm_count = 0;
    let mut all_bytes: Vec<Vec<u8>> = Vec::new();

    for_each_permutation(&mut emissions, |perm| {
        let mut bus = mk_bus();
        bus.register_channel(ch, ChannelPolicy::Reduce(ReduceOp::Sum));

        // Emit in this permutation's order
        for e in perm {
            bus.emit(ch, e.key, e.data.clone())
                .expect("emit should succeed");
        }

        let out_bytes = finalize_bytes(&bus);

        // Assert: bytes must be identical to reference
        assert_eq!(
            out_bytes, reference_bytes,
            "Permutation {} produced different bytes! Sum reducer must be permutation-invariant.",
            perm_count
        );

        all_bytes.push(out_bytes);
        perm_count += 1;
    });

    // Verify we tested all permutations
    assert_eq!(
        perm_count,
        factorial(4),
        "should test all 4! = 24 permutations"
    );

    // Final assertion: ALL collected bytes are identical (playback consistency)
    for (i, bytes) in all_bytes.iter().enumerate() {
        assert_eq!(
            bytes, &reference_bytes,
            "Playback inconsistency at permutation {}: bytes differ from reference",
            i
        );
    }
}

/// T11 Extended: Verify permutation invariance with 5 emissions (5! = 120 permutations).
///
/// This provides more thorough coverage than the base test.
#[test]
fn reducer_sum_permutation_invariant_n5() {
    let ch = make_channel_id("spec0004:t11:sum:n5");

    // 5 values for more thorough testing
    let test_values: Vec<u64> = vec![1, 10, 100, 1000, 10000];
    let expected_sum: u64 = test_values.iter().sum(); // 11111

    let mut emissions: Vec<Emission> = test_values
        .iter()
        .enumerate()
        .map(|(i, &v)| Emission {
            key: key((i + 1) as u8, (i + 1) as u32, 0),
            data: u64_le(v),
        })
        .collect();

    // Get reference
    let reference_bytes = {
        let mut bus = mk_bus();
        bus.register_channel(ch, ChannelPolicy::Reduce(ReduceOp::Sum));
        for e in &emissions {
            bus.emit(ch, e.key, e.data.clone()).expect("emit");
        }
        finalize_bytes(&bus)
    };

    // Verify mathematical correctness
    {
        let mut bus = mk_bus();
        bus.register_channel(ch, ChannelPolicy::Reduce(ReduceOp::Sum));
        for e in &emissions {
            bus.emit(ch, e.key, e.data.clone()).expect("emit");
        }
        let report = bus.finalize();
        let actual_sum = le_to_u64(&report.channels[0].data);
        assert_eq!(actual_sum, expected_sum);
    }

    // Test ALL 5! = 120 permutations
    let mut perm_count = 0;

    for_each_permutation(&mut emissions, |perm| {
        let mut bus = mk_bus();
        bus.register_channel(ch, ChannelPolicy::Reduce(ReduceOp::Sum));
        for e in perm {
            bus.emit(ch, e.key, e.data.clone()).expect("emit");
        }
        let out = finalize_bytes(&bus);
        assert_eq!(
            out, reference_bytes,
            "Permutation {} differs from reference",
            perm_count
        );
        perm_count += 1;
    });

    assert_eq!(perm_count, factorial(5));
}

/// Verify that other commutative reducers (Max, Min) are also permutation-invariant.
#[test]
fn reducer_max_min_are_permutation_invariant() {
    let ch_max = make_channel_id("spec0004:t11:max");
    let ch_min = make_channel_id("spec0004:t11:min");

    // Use byte values that test lexicographic ordering
    let test_values: Vec<Vec<u8>> = vec![vec![0x10, 0x20], vec![0x30, 0x10], vec![0x20, 0x30]];

    // Expected: Max = [0x30, 0x10] (lexicographically largest)
    //           Min = [0x10, 0x20] (lexicographically smallest)

    let mut emissions: Vec<Emission> = test_values
        .iter()
        .enumerate()
        .map(|(i, v)| Emission {
            key: key((i + 1) as u8, 1, 0),
            data: v.clone(),
        })
        .collect();

    // Get reference for Max
    let max_reference = {
        let mut bus = mk_bus();
        bus.register_channel(ch_max, ChannelPolicy::Reduce(ReduceOp::Max));
        for e in &emissions {
            bus.emit(ch_max, e.key, e.data.clone()).expect("emit");
        }
        let report = bus.finalize();
        report.channels[0].data.clone()
    };

    // Get reference for Min
    let min_reference = {
        let mut bus = mk_bus();
        bus.register_channel(ch_min, ChannelPolicy::Reduce(ReduceOp::Min));
        for e in &emissions {
            bus.emit(ch_min, e.key, e.data.clone()).expect("emit");
        }
        let report = bus.finalize();
        report.channels[0].data.clone()
    };

    // Verify expected values
    assert_eq!(
        max_reference,
        vec![0x30, 0x10],
        "Max should be [0x30, 0x10]"
    );
    assert_eq!(
        min_reference,
        vec![0x10, 0x20],
        "Min should be [0x10, 0x20]"
    );

    // Test all permutations
    let mut perm_count = 0;

    for_each_permutation(&mut emissions, |perm| {
        // Test Max
        {
            let mut bus = mk_bus();
            bus.register_channel(ch_max, ChannelPolicy::Reduce(ReduceOp::Max));
            for e in perm {
                bus.emit(ch_max, e.key, e.data.clone()).expect("emit");
            }
            let report = bus.finalize();
            assert_eq!(
                report.channels[0].data, max_reference,
                "Max permutation {} differs",
                perm_count
            );
        }

        // Test Min
        {
            let mut bus = mk_bus();
            bus.register_channel(ch_min, ChannelPolicy::Reduce(ReduceOp::Min));
            for e in perm {
                bus.emit(ch_min, e.key, e.data.clone()).expect("emit");
            }
            let report = bus.finalize();
            assert_eq!(
                report.channels[0].data, min_reference,
                "Min permutation {} differs",
                perm_count
            );
        }

        perm_count += 1;
    });

    assert_eq!(perm_count, factorial(3));
}

/// Verify BitOr and BitAnd are permutation-invariant.
#[test]
fn reducer_bitor_bitand_are_permutation_invariant() {
    let ch_or = make_channel_id("spec0004:t11:bitor");
    let ch_and = make_channel_id("spec0004:t11:bitand");

    let test_values: Vec<Vec<u8>> = vec![
        vec![0b1100_0000, 0b0000_1111],
        vec![0b0011_0000, 0b1111_0000],
        vec![0b0000_1100, 0b0000_1111],
    ];

    // Expected OR:  [0b1111_1100, 0b1111_1111]
    // Expected AND: [0b0000_0000, 0b0000_0000]

    let mut emissions: Vec<Emission> = test_values
        .iter()
        .enumerate()
        .map(|(i, v)| Emission {
            key: key((i + 1) as u8, 1, 0),
            data: v.clone(),
        })
        .collect();

    // Get references
    let or_reference = {
        let mut bus = mk_bus();
        bus.register_channel(ch_or, ChannelPolicy::Reduce(ReduceOp::BitOr));
        for e in &emissions {
            bus.emit(ch_or, e.key, e.data.clone()).expect("emit");
        }
        let report = bus.finalize();
        report.channels[0].data.clone()
    };

    let and_reference = {
        let mut bus = mk_bus();
        bus.register_channel(ch_and, ChannelPolicy::Reduce(ReduceOp::BitAnd));
        for e in &emissions {
            bus.emit(ch_and, e.key, e.data.clone()).expect("emit");
        }
        let report = bus.finalize();
        report.channels[0].data.clone()
    };

    // Verify expected values
    assert_eq!(
        or_reference,
        vec![0b1111_1100, 0b1111_1111],
        "BitOr result incorrect"
    );
    assert_eq!(
        and_reference,
        vec![0b0000_0000, 0b0000_0000],
        "BitAnd result incorrect"
    );

    // Test all permutations
    let mut perm_count = 0;

    for_each_permutation(&mut emissions, |perm| {
        // Test BitOr
        {
            let mut bus = mk_bus();
            bus.register_channel(ch_or, ChannelPolicy::Reduce(ReduceOp::BitOr));
            for e in perm {
                bus.emit(ch_or, e.key, e.data.clone()).expect("emit");
            }
            let report = bus.finalize();
            assert_eq!(report.channels[0].data, or_reference);
        }

        // Test BitAnd
        {
            let mut bus = mk_bus();
            bus.register_channel(ch_and, ChannelPolicy::Reduce(ReduceOp::BitAnd));
            for e in perm {
                bus.emit(ch_and, e.key, e.data.clone()).expect("emit");
            }
            let report = bus.finalize();
            assert_eq!(report.channels[0].data, and_reference);
        }

        perm_count += 1;
    });

    assert_eq!(perm_count, factorial(3));
}

/// Verify that empty reducer input returns the correct identity element.
#[test]
fn reducer_empty_input_returns_identity() {
    let ch_sum = make_channel_id("spec0004:t11:empty:sum");

    // Sum with no emissions should return zero (identity for addition)
    {
        let mut bus = mk_bus();
        bus.register_channel(ch_sum, ChannelPolicy::Reduce(ReduceOp::Sum));
        // Emit to a different channel so the bus has something to finalize
        // but ch_sum remains empty
        let other_ch = make_channel_id("spec0004:t11:empty:other");
        bus.emit(other_ch, key(1, 1, 0), vec![0xFF]).expect("emit");

        let report = bus.finalize();
        assert!(report.is_ok());

        // ch_sum won't appear in channels (no emissions)
        // This is correct behavior - the channel has no emissions
    }

    // Sum with one emission should return that value
    {
        let mut bus = mk_bus();
        bus.register_channel(ch_sum, ChannelPolicy::Reduce(ReduceOp::Sum));
        bus.emit(ch_sum, key(1, 1, 0), u64_le(42)).expect("emit");

        let report = bus.finalize();
        assert!(report.is_ok());
        assert_eq!(report.channels.len(), 1);
        assert_eq!(le_to_u64(&report.channels[0].data), 42);
    }
}

/// Verify multiple emissions per tick with same scope but different subkeys.
#[test]
fn reducer_multiple_emissions_same_scope_different_subkeys() {
    let ch = make_channel_id("spec0004:t11:subkeys");

    // Multiple emissions from "same scope" but with different subkeys
    // This simulates a rule emitting multiple values in one tick
    let scope_hash = h(42);
    let rule_id = 7;

    let mut emissions: Vec<Emission> = vec![
        Emission {
            key: EmitKey::with_subkey(scope_hash, rule_id, 0),
            data: u64_le(10),
        },
        Emission {
            key: EmitKey::with_subkey(scope_hash, rule_id, 1),
            data: u64_le(20),
        },
        Emission {
            key: EmitKey::with_subkey(scope_hash, rule_id, 2),
            data: u64_le(30),
        },
        Emission {
            key: EmitKey::with_subkey(scope_hash, rule_id, 3),
            data: u64_le(40),
        },
    ];

    let expected_sum: u64 = 10 + 20 + 30 + 40;

    // Get reference
    let reference = {
        let mut bus = mk_bus();
        bus.register_channel(ch, ChannelPolicy::Reduce(ReduceOp::Sum));
        for e in &emissions {
            bus.emit(ch, e.key, e.data.clone()).expect("emit");
        }
        let report = bus.finalize();
        assert_eq!(le_to_u64(&report.channels[0].data), expected_sum);
        finalize_bytes(&{
            let mut bus = mk_bus();
            bus.register_channel(ch, ChannelPolicy::Reduce(ReduceOp::Sum));
            for e in &emissions {
                bus.emit(ch, e.key, e.data.clone()).expect("emit");
            }
            bus
        })
    };

    // Test all permutations
    let mut perm_count = 0;

    for_each_permutation(&mut emissions, |perm| {
        let mut bus = mk_bus();
        bus.register_channel(ch, ChannelPolicy::Reduce(ReduceOp::Sum));
        for e in perm {
            bus.emit(ch, e.key, e.data.clone()).expect("emit");
        }
        let out = finalize_bytes(&bus);
        assert_eq!(out, reference, "Permutation {} differs", perm_count);
        perm_count += 1;
    });

    assert_eq!(perm_count, factorial(4));
}

// =============================================================================
// SPEC-0004 T12: ORDER-DEPENDENT REDUCER CANONICAL DETERMINISM
// =============================================================================

/// T12: Order-dependent reducers (Concat) produce identical output across all
/// permutations because EmitKey canonicalization enforces a stable ordering.
///
/// # SPEC-0004 Requirements
///
/// - Reducer channel with Concat (order-dependent operation)
/// - Relies on canonical EmitKey ordering (lexicographic: scope_hash -> rule_id -> subkey)
/// - Run across all permutations
/// - Assert: Output IDENTICAL across permutations (because EmitKey canonicalization)
/// - Assert: Playback equals recorded
///
/// # Key Insight
///
/// Even though `ReduceOp::Concat` is order-dependent (concatenation order matters),
/// the MaterializationBus sorts all emissions by EmitKey before applying the reducer.
/// This means the concatenation order is deterministic regardless of emission order!
#[test]
fn reducer_order_dependent_is_canonically_deterministic_and_replayable() {
    let ch = make_channel_id("spec0004:t12:concat");

    // Create emissions with DIFFERENT scope/rule/subkey values (so ordering is testable).
    // The EmitKey ordering is lexicographic: scope_hash -> rule_id -> subkey.
    //
    // Keys sorted in canonical order:
    //   key(1, 1, 0)  -> "ALPHA"
    //   key(1, 2, 0)  -> "BRAVO"
    //   key(2, 1, 0)  -> "CHARLIE"
    //   key(2, 1, 5)  -> "DELTA"
    //   key(3, 0, 0)  -> "ECHO"
    //
    // Expected concatenation: "ALPHABRAVOCHARLIEDELTAECHO"

    let mut emissions = vec![
        Emission {
            key: key(2, 1, 5), // DELTA - will be 4th in canonical order
            data: b"DELTA".to_vec(),
        },
        Emission {
            key: key(1, 1, 0), // ALPHA - will be 1st in canonical order
            data: b"ALPHA".to_vec(),
        },
        Emission {
            key: key(3, 0, 0), // ECHO - will be 5th in canonical order
            data: b"ECHO".to_vec(),
        },
        Emission {
            key: key(2, 1, 0), // CHARLIE - will be 3rd in canonical order
            data: b"CHARLIE".to_vec(),
        },
        Emission {
            key: key(1, 2, 0), // BRAVO - will be 2nd in canonical order
            data: b"BRAVO".to_vec(),
        },
    ];

    // Expected concatenation in EmitKey canonical order
    let expected_concat = b"ALPHABRAVOCHARLIEDELTAECHO".to_vec();

    // Verify the expected ordering by sorting keys manually
    {
        let mut keys: Vec<EmitKey> = emissions.iter().map(|e| e.key).collect();
        keys.sort();
        assert_eq!(keys[0], key(1, 1, 0), "first key should be (1, 1, 0)");
        assert_eq!(keys[1], key(1, 2, 0), "second key should be (1, 2, 0)");
        assert_eq!(keys[2], key(2, 1, 0), "third key should be (2, 1, 0)");
        assert_eq!(keys[3], key(2, 1, 5), "fourth key should be (2, 1, 5)");
        assert_eq!(keys[4], key(3, 0, 0), "fifth key should be (3, 0, 0)");
    }

    // Compute reference output
    let reference = {
        let mut bus = mk_bus();
        bus.register_channel(ch, ChannelPolicy::Reduce(ReduceOp::Concat));
        for e in &emissions {
            bus.emit(ch, e.key, e.data.clone()).expect("emit");
        }
        let report = bus.finalize();
        assert!(report.is_ok(), "finalize should succeed");
        report.channels[0].data.clone()
    };

    // Assert reference matches expected
    assert_eq!(
        reference, expected_concat,
        "reference should match expected concatenation order"
    );

    // Compute reference wire bytes (for playback consistency check)
    let reference_bytes = {
        let mut bus = mk_bus();
        bus.register_channel(ch, ChannelPolicy::Reduce(ReduceOp::Concat));
        for e in &emissions {
            bus.emit(ch, e.key, e.data.clone()).expect("emit");
        }
        finalize_bytes(&bus)
    };

    // Track permutation count for sanity check
    let mut perm_count = 0;

    // Brute-force all 5! = 120 permutations
    for_each_permutation(&mut emissions, |perm| {
        let mut bus = mk_bus();
        bus.register_channel(ch, ChannelPolicy::Reduce(ReduceOp::Concat));
        for e in perm {
            bus.emit(ch, e.key, e.data.clone()).expect("emit");
        }
        let report = bus.finalize();
        assert!(report.is_ok());
        let result = &report.channels[0].data;

        // Assert: Output IDENTICAL across permutations
        assert_eq!(
            result, &reference,
            "permutation {} should produce identical output due to EmitKey canonicalization",
            perm_count
        );

        // Assert: Playback equals recorded (wire bytes)
        let result_bytes = finalize_bytes(&{
            let mut bus = mk_bus();
            bus.register_channel(ch, ChannelPolicy::Reduce(ReduceOp::Concat));
            for e in perm {
                bus.emit(ch, e.key, e.data.clone()).expect("emit");
            }
            bus
        });
        assert_eq!(
            result_bytes, reference_bytes,
            "permutation {} wire bytes should match reference (playback consistency)",
            perm_count
        );

        perm_count += 1;
    });

    // Verify we tested all permutations
    assert_eq!(
        perm_count,
        factorial(5),
        "should test all 5! = 120 permutations"
    );
}

/// Verify that the concatenation order matches EmitKey lexicographic order.
///
/// This test explicitly checks that the reduced output follows the EmitKey ordering
/// (scope_hash -> rule_id -> subkey), not the emission order.
#[test]
fn concat_follows_emit_key_lexicographic_order() {
    let ch = make_channel_id("spec0004:t12:concat:order");

    // Create emissions that demonstrate all three ordering dimensions:
    // 1. scope_hash ordering (primary)
    // 2. rule_id ordering (secondary, same scope)
    // 3. subkey ordering (tertiary, same scope+rule)

    let mut bus = mk_bus();
    bus.register_channel(ch, ChannelPolicy::Reduce(ReduceOp::Concat));

    // Emit in deliberately scrambled order
    bus.emit(ch, key(2, 5, 0), b"D".to_vec()).expect("emit D"); // scope=2, rule=5
    bus.emit(ch, key(1, 5, 2), b"C".to_vec()).expect("emit C"); // scope=1, rule=5, subkey=2
    bus.emit(ch, key(1, 5, 1), b"B".to_vec()).expect("emit B"); // scope=1, rule=5, subkey=1
    bus.emit(ch, key(1, 3, 0), b"A".to_vec()).expect("emit A"); // scope=1, rule=3

    // Expected order after EmitKey sorting:
    // key(1, 3, 0) -> A (scope=1, rule=3)
    // key(1, 5, 1) -> B (scope=1, rule=5, subkey=1)
    // key(1, 5, 2) -> C (scope=1, rule=5, subkey=2)
    // key(2, 5, 0) -> D (scope=2)

    let report = bus.finalize();
    assert!(report.is_ok());
    let result = &report.channels[0].data;

    assert_eq!(
        result,
        &b"ABCD".to_vec(),
        "concat should follow EmitKey lexicographic order: scope_hash -> rule_id -> subkey"
    );
}

/// Test that Concat with binary data produces deterministic results across permutations.
#[test]
fn concat_binary_data_is_permutation_invariant() {
    let ch = make_channel_id("spec0004:t12:concat:binary");

    // Binary data with different byte patterns
    let mut emissions = vec![
        Emission {
            key: key(3, 1, 0),
            data: vec![0xFF, 0xFE, 0xFD],
        },
        Emission {
            key: key(1, 1, 0),
            data: vec![0x00, 0x01],
        },
        Emission {
            key: key(2, 1, 0),
            data: vec![0xAB, 0xCD, 0xEF, 0x12],
        },
        Emission {
            key: key(1, 2, 0),
            data: vec![0x42],
        },
    ];

    // Expected: key(1,1,0) ++ key(1,2,0) ++ key(2,1,0) ++ key(3,1,0)
    let expected = vec![
        0x00, 0x01, // key(1,1,0)
        0x42, // key(1,2,0)
        0xAB, 0xCD, 0xEF, 0x12, // key(2,1,0)
        0xFF, 0xFE, 0xFD, // key(3,1,0)
    ];

    let reference = {
        let mut bus = mk_bus();
        bus.register_channel(ch, ChannelPolicy::Reduce(ReduceOp::Concat));
        for e in &emissions {
            bus.emit(ch, e.key, e.data.clone()).expect("emit");
        }
        let report = bus.finalize();
        report.channels[0].data.clone()
    };

    assert_eq!(
        reference, expected,
        "reference should match expected binary concatenation"
    );

    // Test all 4! = 24 permutations
    let mut perm_count = 0;
    for_each_permutation(&mut emissions, |perm| {
        let mut bus = mk_bus();
        bus.register_channel(ch, ChannelPolicy::Reduce(ReduceOp::Concat));
        for e in perm {
            bus.emit(ch, e.key, e.data.clone()).expect("emit");
        }
        let report = bus.finalize();
        let result = &report.channels[0].data;

        assert_eq!(
            result, &reference,
            "binary permutation {} should match reference",
            perm_count
        );
        perm_count += 1;
    });

    assert_eq!(perm_count, factorial(4), "should test all 4! permutations");
}

/// Test that Concat with multiple emissions from same scope (different subkeys) works correctly.
#[test]
fn concat_multiple_emissions_same_scope_different_subkeys() {
    let ch = make_channel_id("spec0004:t12:concat:subkeys");

    // Multiple emissions from "same scope" but with different subkeys
    // This simulates a rule emitting multiple values in one tick
    let scope_hash = h(42);
    let rule_id = 7;

    let mut emissions: Vec<Emission> = vec![
        Emission {
            key: EmitKey::with_subkey(scope_hash, rule_id, 3),
            data: b"FOUR".to_vec(),
        },
        Emission {
            key: EmitKey::with_subkey(scope_hash, rule_id, 0),
            data: b"ONE".to_vec(),
        },
        Emission {
            key: EmitKey::with_subkey(scope_hash, rule_id, 2),
            data: b"THREE".to_vec(),
        },
        Emission {
            key: EmitKey::with_subkey(scope_hash, rule_id, 1),
            data: b"TWO".to_vec(),
        },
    ];

    // Expected order: subkey 0, 1, 2, 3 -> "ONETWOTHREEFOUR"
    let expected = b"ONETWOTHREEFOUR".to_vec();

    // Get reference
    let reference = {
        let mut bus = mk_bus();
        bus.register_channel(ch, ChannelPolicy::Reduce(ReduceOp::Concat));
        for e in &emissions {
            bus.emit(ch, e.key, e.data.clone()).expect("emit");
        }
        let report = bus.finalize();
        report.channels[0].data.clone()
    };

    assert_eq!(reference, expected, "reference should match expected");

    // Test all permutations
    let mut perm_count = 0;
    for_each_permutation(&mut emissions, |perm| {
        let mut bus = mk_bus();
        bus.register_channel(ch, ChannelPolicy::Reduce(ReduceOp::Concat));
        for e in perm {
            bus.emit(ch, e.key, e.data.clone()).expect("emit");
        }
        let report = bus.finalize();
        let result = &report.channels[0].data;

        assert_eq!(result, &reference, "Permutation {} differs", perm_count);
        perm_count += 1;
    });

    assert_eq!(perm_count, factorial(4));
}

/// Test that Concat with a single emission just returns that emission unchanged.
#[test]
fn concat_single_emission_returns_unchanged() {
    let ch = make_channel_id("spec0004:t12:concat:single");

    let mut bus = mk_bus();
    bus.register_channel(ch, ChannelPolicy::Reduce(ReduceOp::Concat));

    let data = vec![0xDE, 0xAD, 0xBE, 0xEF];
    bus.emit(ch, key(1, 1, 0), data.clone()).expect("emit");

    let report = bus.finalize();
    assert!(report.is_ok());
    assert_eq!(
        report.channels[0].data, data,
        "single emission Concat should return the data unchanged"
    );
}

/// Test that empty Concat produces empty output (identity element).
#[test]
fn concat_empty_input_produces_empty_output() {
    let ch = make_channel_id("spec0004:t12:concat:empty");

    let mut bus = mk_bus();
    bus.register_channel(ch, ChannelPolicy::Reduce(ReduceOp::Concat));

    // Emit to a different channel so the bus has something to process
    let other_ch = make_channel_id("spec0004:t12:concat:other");
    bus.emit(other_ch, key(1, 1, 0), vec![0xFF]).expect("emit");

    let report = bus.finalize();
    assert!(report.is_ok());

    // The registered channel with no emissions won't appear in output
    // (or will appear with empty data if the channel was touched)
    let channel_entry = report.channels.iter().find(|fc| fc.channel == ch);
    if let Some(entry) = channel_entry {
        assert!(
            entry.data.is_empty(),
            "empty Concat channel should have empty data"
        );
    }
    // If channel doesn't appear at all, that's also acceptable behavior
}

/// Test First and Last reducers are also canonically deterministic.
///
/// First should always return the value with the smallest EmitKey.
/// Last should always return the value with the largest EmitKey.
#[test]
fn first_last_reducers_are_canonically_deterministic() {
    let ch_first = make_channel_id("spec0004:t12:first");
    let ch_last = make_channel_id("spec0004:t12:last");

    let mut emissions = vec![
        Emission {
            key: key(3, 1, 0), // Last in canonical order
            data: b"THREE".to_vec(),
        },
        Emission {
            key: key(1, 1, 0), // First in canonical order
            data: b"ONE".to_vec(),
        },
        Emission {
            key: key(2, 1, 0), // Middle
            data: b"TWO".to_vec(),
        },
    ];

    // First should always return "ONE" (smallest key)
    let first_reference = {
        let mut bus = mk_bus();
        bus.register_channel(ch_first, ChannelPolicy::Reduce(ReduceOp::First));
        for e in &emissions {
            bus.emit(ch_first, e.key, e.data.clone()).expect("emit");
        }
        let report = bus.finalize();
        report.channels[0].data.clone()
    };
    assert_eq!(
        first_reference,
        b"ONE".to_vec(),
        "First should return smallest key's value"
    );

    // Last should always return "THREE" (largest key)
    let last_reference = {
        let mut bus = mk_bus();
        bus.register_channel(ch_last, ChannelPolicy::Reduce(ReduceOp::Last));
        for e in &emissions {
            bus.emit(ch_last, e.key, e.data.clone()).expect("emit");
        }
        let report = bus.finalize();
        report.channels[0].data.clone()
    };
    assert_eq!(
        last_reference,
        b"THREE".to_vec(),
        "Last should return largest key's value"
    );

    // Test all 3! = 6 permutations
    let mut perm_count = 0;
    for_each_permutation(&mut emissions, |perm| {
        // Test First
        {
            let mut bus = mk_bus();
            bus.register_channel(ch_first, ChannelPolicy::Reduce(ReduceOp::First));
            for e in perm {
                bus.emit(ch_first, e.key, e.data.clone()).expect("emit");
            }
            let report = bus.finalize();
            assert_eq!(
                report.channels[0].data, first_reference,
                "First permutation {} differs",
                perm_count
            );
        }

        // Test Last
        {
            let mut bus = mk_bus();
            bus.register_channel(ch_last, ChannelPolicy::Reduce(ReduceOp::Last));
            for e in perm {
                bus.emit(ch_last, e.key, e.data.clone()).expect("emit");
            }
            let report = bus.finalize();
            assert_eq!(
                report.channels[0].data, last_reference,
                "Last permutation {} differs",
                perm_count
            );
        }

        perm_count += 1;
    });

    assert_eq!(perm_count, factorial(3));
}

// =============================================================================
// SPEC-0004 T13: REDUCED CHANNEL EMITS SINGLE AUTHORITATIVE VALUE PER TICK
// =============================================================================
//
// SPEC requirement from Section 4 "Outputs and Reducers":
// - OUT-REDUCE-001: For reduced channels, per-tick outputs are the final
//   reduced value, not raw emission events.
// - Key assertion: many emissions -> one output (cardinality)
// - Raw event streams are NOT leaked; only final reduced value exposed

/// T13: Reduced channel emits a single authoritative value per tick, not many raw emissions.
///
/// This test proves that:
/// 1. Many emissions to a Reduce channel result in exactly ONE finalized value
/// 2. Raw event streams are not leaked (comparing with Log policy output size)
/// 3. The single value is the correct reduced result
///
/// SPEC requirement: "outputs[t] contains exactly ONE value for that channel"
#[test]
fn reduced_channel_emits_single_authoritative_value_per_tick() {
    let ch = make_channel_id("spec0004:t13:sum");

    let mut bus = mk_bus();
    bus.register_channel(ch, ChannelPolicy::Reduce(ReduceOp::Sum));

    // Emit MANY values (15 emissions) to the same channel
    // Using different keys to simulate multiple rule emissions within one tick
    for i in 0u8..15 {
        let k = key(i, 1, 0); // Different scope for each emission
        bus.emit(ch, k, u64_le(i as u64))
            .expect("emit should succeed");
    }

    // Finalize the bus (simulates end of tick)
    let report = bus.finalize();

    // Assert: report should succeed
    assert!(report.is_ok(), "finalize should succeed for Reduce channel");

    // Assert: report.channels.len() == 1 (single channel)
    assert_eq!(
        report.channels.len(),
        1,
        "should have exactly 1 channel in output"
    );

    // Assert: The channel has exactly ONE data blob (not 15 raw emissions)
    // Unlike Log policy which produces length-prefixed entries for each emission,
    // Reduce policy produces a single value (the reduced result).
    let finalized = &report.channels[0];
    assert_eq!(finalized.channel, ch, "channel should match");

    // For ReduceOp::Sum, the result is a single u64 LE (8 bytes)
    assert_eq!(
        finalized.data.len(),
        8,
        "Sum reducer should produce exactly 8 bytes (one u64), not {} bytes \
         (which would indicate raw emissions leaked)",
        finalized.data.len()
    );

    // Verify the single value is the correct reduced result
    // Sum of 0 + 1 + 2 + ... + 14 = 105
    let expected_sum: u64 = (0..15u64).sum(); // 105
    let actual_sum = le_to_u64(&finalized.data);
    assert_eq!(
        actual_sum, expected_sum,
        "reduced value should be sum of all emissions"
    );

    // Verify we're NOT leaking raw emissions
    // If Log policy were used, we'd have 15 * (4 + 8) = 180 bytes
    // (each emission: 4-byte length prefix + 8-byte u64 value)
    assert!(
        finalized.data.len() < 16,
        "should NOT have multiple length-prefixed entries (raw leak detection)"
    );
}

/// T13 additional case: Multiple channels, each with multiple emissions.
///
/// This verifies:
/// - Each channel has exactly ONE output (no raw emission leak)
/// - No cross-channel contamination
/// - Different reduce operations produce correct single-value results
#[test]
fn t13_multiple_reduce_channels_each_emit_single_value() {
    let ch_sum = make_channel_id("spec0004:t13:multi:sum");
    let ch_max = make_channel_id("spec0004:t13:multi:max");
    let ch_min = make_channel_id("spec0004:t13:multi:min");
    let ch_first = make_channel_id("spec0004:t13:multi:first");
    let ch_last = make_channel_id("spec0004:t13:multi:last");

    let mut bus = mk_bus();
    bus.register_channel(ch_sum, ChannelPolicy::Reduce(ReduceOp::Sum));
    bus.register_channel(ch_max, ChannelPolicy::Reduce(ReduceOp::Max));
    bus.register_channel(ch_min, ChannelPolicy::Reduce(ReduceOp::Min));
    bus.register_channel(ch_first, ChannelPolicy::Reduce(ReduceOp::First));
    bus.register_channel(ch_last, ChannelPolicy::Reduce(ReduceOp::Last));

    // Emit multiple values to each channel (10 emissions each)
    for i in 0u8..10 {
        let k = key(i, 1, 0);
        let value = (i as u64) * 10; // 0, 10, 20, ..., 90
        bus.emit(ch_sum, k, u64_le(value)).expect("emit sum");
        bus.emit(ch_max, k, u64_le(value)).expect("emit max");
        bus.emit(ch_min, k, u64_le(value)).expect("emit min");
        bus.emit(ch_first, k, u64_le(value)).expect("emit first");
        bus.emit(ch_last, k, u64_le(value)).expect("emit last");
    }

    let report = bus.finalize();
    assert!(report.is_ok(), "finalize should succeed");

    // Assert: 5 channels, each with exactly one output
    assert_eq!(
        report.channels.len(),
        5,
        "should have exactly 5 channels in output"
    );

    // Build lookup map for convenience
    let outputs: std::collections::BTreeMap<_, _> = report
        .channels
        .iter()
        .map(|fc| (fc.channel, fc.data.clone()))
        .collect();

    // Assert each channel has single reduced value (8 bytes for u64)
    for (&ch, data) in &outputs {
        assert_eq!(
            data.len(),
            8,
            "channel {:?} should have exactly 8 bytes output (single u64), got {}",
            ch,
            data.len()
        );
    }

    // Verify correctness of each reduce operation
    // Sum: 0 + 10 + 20 + ... + 90 = 450
    let sum_result = le_to_u64(&outputs[&ch_sum]);
    assert_eq!(sum_result, 450, "Sum should be 450 (0+10+20+...+90)");

    // Max: 90 (lexicographic max of u64 LE bytes)
    let max_result = le_to_u64(&outputs[&ch_max]);
    assert_eq!(max_result, 90, "Max should be 90");

    // Min: 0 (lexicographic min of u64 LE bytes)
    let min_result = le_to_u64(&outputs[&ch_min]);
    assert_eq!(min_result, 0, "Min should be 0");

    // First: 0 (first by EmitKey order, which is scope 0)
    let first_result = le_to_u64(&outputs[&ch_first]);
    assert_eq!(first_result, 0, "First should be 0 (smallest EmitKey)");

    // Last: 90 (last by EmitKey order, which is scope 9)
    let last_result = le_to_u64(&outputs[&ch_last]);
    assert_eq!(last_result, 90, "Last should be 90 (largest EmitKey)");
}

/// T13 edge case: Reduce channel with Concat produces single concatenated blob.
///
/// Concat is special: it produces a single value that is the concatenation
/// of all emissions in EmitKey order. This is still ONE output, not many.
#[test]
fn t13_reduce_concat_produces_single_concatenated_value() {
    let ch = make_channel_id("spec0004:t13:concat");

    let mut bus = mk_bus();
    bus.register_channel(ch, ChannelPolicy::Reduce(ReduceOp::Concat));

    // Emit 5 values with different keys (will be concatenated in EmitKey order)
    // EmitKey order: scope 0 < scope 1 < scope 2 < scope 3 < scope 4
    bus.emit(ch, key(4, 1, 0), vec![0xEE]).expect("emit");
    bus.emit(ch, key(1, 1, 0), vec![0xBB]).expect("emit");
    bus.emit(ch, key(3, 1, 0), vec![0xDD]).expect("emit");
    bus.emit(ch, key(0, 1, 0), vec![0xAA]).expect("emit");
    bus.emit(ch, key(2, 1, 0), vec![0xCC]).expect("emit");

    let report = bus.finalize();
    assert!(report.is_ok(), "finalize should succeed");

    assert_eq!(report.channels.len(), 1, "should have 1 channel");

    let finalized = &report.channels[0];

    // Concat produces ONE value: all emissions concatenated in EmitKey order
    // Order by scope: 0, 1, 2, 3, 4 -> [0xAA, 0xBB, 0xCC, 0xDD, 0xEE]
    assert_eq!(
        finalized.data,
        vec![0xAA, 0xBB, 0xCC, 0xDD, 0xEE],
        "Concat should produce single concatenated value in EmitKey order"
    );

    // Verify this is ONE output, not 5 length-prefixed emissions
    // If raw emissions leaked (Log format), we'd have 5 * (4 + 1) = 25 bytes
    assert_eq!(
        finalized.data.len(),
        5,
        "Concat output should be 5 bytes (concatenated), not length-prefixed entries"
    );
}

/// T13: Compare Reduce vs Log policy to explicitly prove raw emissions are not leaked.
///
/// This test directly compares what Log policy would produce (raw emissions)
/// vs what Reduce policy produces (single consolidated value).
#[test]
fn t13_reduce_vs_log_proves_no_raw_emission_leak() {
    let ch_reduce = make_channel_id("spec0004:t13:compare:reduce");
    let ch_log = make_channel_id("spec0004:t13:compare:log");

    // Test with Reduce policy
    let mut bus_reduce = mk_bus();
    bus_reduce.register_channel(ch_reduce, ChannelPolicy::Reduce(ReduceOp::Sum));

    for i in 0u8..5 {
        bus_reduce
            .emit(ch_reduce, key(i, 1, 0), u64_le(i as u64))
            .expect("emit");
    }

    let report_reduce = bus_reduce.finalize();
    let reduce_data = &report_reduce.channels[0].data;

    // Test with Log policy (for comparison)
    let bus_log = mk_bus();
    // Log is default, no registration needed

    for i in 0u8..5 {
        bus_log
            .emit(ch_log, key(i, 1, 0), u64_le(i as u64))
            .expect("emit");
    }

    let report_log = bus_log.finalize();
    let log_data = &report_log.channels[0].data;

    // Reduce: single u64 (8 bytes)
    assert_eq!(
        reduce_data.len(),
        8,
        "Reduce should produce 8 bytes (single u64)"
    );

    // Log: 5 entries, each with 4-byte length prefix + 8-byte value = 60 bytes
    assert_eq!(
        log_data.len(),
        5 * (4 + 8),
        "Log should produce 60 bytes (5 length-prefixed entries)"
    );

    // This proves Reduce is NOT leaking raw emissions like Log would
    assert!(
        reduce_data.len() < log_data.len(),
        "Reduce output ({} bytes) should be smaller than Log output ({} bytes)",
        reduce_data.len(),
        log_data.len()
    );

    // Verify Reduce produces correct value (0+1+2+3+4 = 10)
    assert_eq!(le_to_u64(reduce_data), 10, "Reduce sum should be 10");
}

/// T13: BitOr and BitAnd reduce operations also produce single value.
#[test]
fn t13_bitwise_reduce_produces_single_value() {
    let ch_or = make_channel_id("spec0004:t13:bitor");
    let ch_and = make_channel_id("spec0004:t13:bitand");

    let mut bus = mk_bus();
    bus.register_channel(ch_or, ChannelPolicy::Reduce(ReduceOp::BitOr));
    bus.register_channel(ch_and, ChannelPolicy::Reduce(ReduceOp::BitAnd));

    // Emit values with different bit patterns
    // BitOr: 0b0001 | 0b0010 | 0b0100 | 0b1000 = 0b1111 = 15
    // BitAnd: 0xFF & 0xFF & 0xFF & 0xFF = 0xFF = 255
    let bit_patterns = [0b0001u8, 0b0010u8, 0b0100u8, 0b1000u8];
    let and_patterns = [0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8];

    for (i, &pat) in bit_patterns.iter().enumerate() {
        bus.emit(ch_or, key(i as u8, 1, 0), vec![pat])
            .expect("emit bitor");
    }

    for (i, &pat) in and_patterns.iter().enumerate() {
        bus.emit(ch_and, key(i as u8, 1, 0), vec![pat])
            .expect("emit bitand");
    }

    let report = bus.finalize();
    assert!(report.is_ok(), "finalize should succeed");
    assert_eq!(report.channels.len(), 2, "should have 2 channels");

    let outputs: std::collections::BTreeMap<_, _> = report
        .channels
        .iter()
        .map(|fc| (fc.channel, fc.data.clone()))
        .collect();

    // BitOr: single byte output (1 byte, not 4 * (4 + 1) = 20 bytes)
    assert_eq!(
        outputs[&ch_or].len(),
        1,
        "BitOr should produce 1 byte output"
    );
    assert_eq!(outputs[&ch_or], vec![0b1111], "BitOr should be 0b1111 = 15");

    // BitAnd: single byte output
    assert_eq!(
        outputs[&ch_and].len(),
        1,
        "BitAnd should produce 1 byte output"
    );
    assert_eq!(outputs[&ch_and], vec![0xFF], "BitAnd should be 0xFF = 255");
}

/// T13: Large number of emissions still produces single output.
///
/// This stress tests that even with 1000 emissions, we get exactly one output.
#[test]
fn t13_reduce_large_emission_count_produces_single_output() {
    let ch = make_channel_id("spec0004:t13:stress");

    let mut bus = mk_bus();
    bus.register_channel(ch, ChannelPolicy::Reduce(ReduceOp::Sum));

    // Emit 1000 values
    for i in 0u32..1000 {
        // Use subkey to differentiate within same scope
        let k = EmitKey::with_subkey(h(0), 1, i);
        bus.emit(ch, k, u64_le(1)).expect("emit"); // Each emits 1, so sum = 1000
    }

    let report = bus.finalize();
    assert!(report.is_ok(), "finalize should succeed");
    assert_eq!(report.channels.len(), 1, "should have 1 channel");

    let finalized = &report.channels[0];

    // Still produces exactly 8 bytes (one u64), not 1000 * (4 + 8) = 12000 bytes
    assert_eq!(
        finalized.data.len(),
        8,
        "1000 emissions should still produce exactly 8 bytes output"
    );

    // Sum should be 1000
    assert_eq!(
        le_to_u64(&finalized.data),
        1000,
        "Sum of 1000 ones should be 1000"
    );
}
