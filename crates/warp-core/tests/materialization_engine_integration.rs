// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Engine Integration Tests for MaterializationBus.
//!
//! # Tier 5: Engine Integration (RFC mat-bus-finish.md)
//!
//! These tests prove that:
//! - **Order independence**: Rewrite application order doesn't affect materialization output
//! - **Deterministic errors**: StrictSingle failures are identical regardless of order
//! - **Reduce correctness**: Reduce ops produce correct results through the bus
//! - **Commit timing**: Emissions are finalized only after commit
//!
//! # Architecture Notes
//!
//! The integration pattern demonstrated here is:
//! ```text
//! Engine::commit_with_receipt()
//!     │
//!     ├─► apply_reserved_rewrites()
//!     │       │
//!     │       └─► For each rewrite:
//!     │               let emitter = ScopedEmitter::new(&bus, scope_hash, rule_id);
//!     │               rule.execute(context, &emitter)?;
//!     │
//!     ├─► bus.finalize()  ← Produces FinalizedChannels
//!     │
//!     └─► MaterializationPort::receive_finalized()
//! ```

#![allow(clippy::expect_used, clippy::unwrap_used)]

use warp_core::materialization::{
    encode_frames, make_channel_id, ChannelId, ChannelPolicy, EmissionPort, FinalizeReport,
    FinalizedChannel, MaterializationBus, MaterializationFrame, ReduceOp, ScopedEmitter,
};
use warp_core::Hash;

// =============================================================================
// TEST FIXTURE TYPES
// =============================================================================

/// Simulates a rule execution context.
#[derive(Clone, Debug)]
struct SimulatedRuleExec {
    /// Content hash of the scope node (deterministic per scope).
    scope_hash: Hash,
    /// Compact rule ID from the registry.
    rule_id: u32,
    /// Channel to emit to.
    channel: ChannelId,
    /// Data to emit.
    data: Vec<u8>,
}

/// Create a deterministic scope hash for tests.
fn scope_hash(tag: u8) -> Hash {
    let mut h = [0u8; 32];
    h[0] = 0xE5; // "E" for Echo
    h[1] = 0xC0; // "C" for test Case
    h[31] = tag;
    h
}

/// Simulate the full commit path: apply rewrites via ScopedEmitter, then finalize.
///
/// Returns a `FinalizeReport` which contains both successful channels and any errors.
fn simulate_commit(bus: &MaterializationBus, executions: &[SimulatedRuleExec]) -> FinalizeReport {
    for exec in executions {
        let emitter = ScopedEmitter::new(bus, exec.scope_hash, exec.rule_id);
        emitter
            .emit(exec.channel, exec.data.clone())
            .expect("emit should succeed within test");
    }
    bus.finalize()
}

/// Convert finalized channels to wire bytes for comparison.
fn finalized_to_bytes(finalized: &[FinalizedChannel]) -> Vec<u8> {
    let frames: Vec<MaterializationFrame> = finalized
        .iter()
        .map(|fc| MaterializationFrame::new(fc.channel, fc.data.clone()))
        .collect();
    encode_frames(&frames)
}

// =============================================================================
// TIER 5: ENGINE INTEGRATION TESTS
// =============================================================================

/// Log emissions are stable regardless of rewrite application order.
///
/// This test simulates two different orderings of rewrite execution
/// and verifies that the finalized output is byte-for-byte identical.
#[test]
fn engine_log_emissions_stable_across_apply_order() {
    let ch = make_channel_id("engine:log-order");

    // Two "rewrites" that emit to the same channel.
    // In a real engine, these would be PendingRewrites with different scopes.
    let exec_a = SimulatedRuleExec {
        scope_hash: scope_hash(1),
        rule_id: 42,
        channel: ch,
        data: vec![0xAA, 0xBB],
    };
    let exec_b = SimulatedRuleExec {
        scope_hash: scope_hash(2),
        rule_id: 42,
        channel: ch,
        data: vec![0xCC, 0xDD],
    };

    // Order 1: A then B
    let bus1 = MaterializationBus::new();
    let report1 = simulate_commit(&bus1, &[exec_a.clone(), exec_b.clone()]);
    assert!(report1.is_ok());
    let bytes1 = finalized_to_bytes(&report1.channels);

    // Order 2: B then A
    let bus2 = MaterializationBus::new();
    let report2 = simulate_commit(&bus2, &[exec_b, exec_a]);
    assert!(report2.is_ok());
    let bytes2 = finalized_to_bytes(&report2.channels);

    // Wire bytes must be identical regardless of execution order
    assert_eq!(
        bytes1, bytes2,
        "log emissions must be identical regardless of apply order"
    );
}

/// StrictSingle errors are deterministic regardless of emission order.
///
/// When multiple rewrites emit to a StrictSingle channel, the error
/// must identify the same channel and count regardless of execution order.
#[test]
fn engine_strict_single_deterministic_failure() {
    let ch = make_channel_id("engine:strict-fail");

    let exec_a = SimulatedRuleExec {
        scope_hash: scope_hash(1),
        rule_id: 10,
        channel: ch,
        data: vec![0x11],
    };
    let exec_b = SimulatedRuleExec {
        scope_hash: scope_hash(2),
        rule_id: 20,
        channel: ch,
        data: vec![0x22],
    };

    // Order 1: A then B
    let mut bus1 = MaterializationBus::new();
    bus1.register_channel(ch, ChannelPolicy::StrictSingle);
    let report1 = simulate_commit(&bus1, &[exec_a.clone(), exec_b.clone()]);
    assert!(report1.has_errors(), "should have errors");
    let err1 = &report1.errors[0];

    // Order 2: B then A
    let mut bus2 = MaterializationBus::new();
    bus2.register_channel(ch, ChannelPolicy::StrictSingle);
    let report2 = simulate_commit(&bus2, &[exec_b, exec_a]);
    assert!(report2.has_errors(), "should have errors");
    let err2 = &report2.errors[0];

    // Error must be identical
    assert_eq!(err1.channel, err2.channel, "error channel must match");
    assert_eq!(
        err1.emission_count, err2.emission_count,
        "emission count must match"
    );
    assert_eq!(err1.emission_count, 2, "exactly 2 emissions");
}

/// Reduce(Sum) produces correct results through the commit path.
#[test]
fn engine_reduce_sum_stable_across_apply_order() {
    let ch = make_channel_id("engine:reduce-sum");

    // Three rewrites emitting u64 values
    let execs = vec![
        SimulatedRuleExec {
            scope_hash: scope_hash(1),
            rule_id: 1,
            channel: ch,
            data: 10u64.to_le_bytes().to_vec(),
        },
        SimulatedRuleExec {
            scope_hash: scope_hash(2),
            rule_id: 1,
            channel: ch,
            data: 20u64.to_le_bytes().to_vec(),
        },
        SimulatedRuleExec {
            scope_hash: scope_hash(3),
            rule_id: 1,
            channel: ch,
            data: 70u64.to_le_bytes().to_vec(),
        },
    ];

    // All permutations should produce sum = 100
    let expected_sum = 100u64.to_le_bytes().to_vec();

    // Test multiple orderings
    let orderings = [
        vec![0, 1, 2],
        vec![2, 1, 0],
        vec![1, 0, 2],
        vec![0, 2, 1],
        vec![2, 0, 1],
        vec![1, 2, 0],
    ];

    for ordering in orderings {
        let mut bus = MaterializationBus::new();
        bus.register_channel(ch, ChannelPolicy::Reduce(ReduceOp::Sum));

        let ordered_execs: Vec<_> = ordering.iter().map(|&i| execs[i].clone()).collect();
        let report = simulate_commit(&bus, &ordered_execs);
        assert!(report.is_ok());

        assert_eq!(report.channels.len(), 1);
        assert_eq!(
            report.channels[0].data, expected_sum,
            "Sum should be 100 regardless of order"
        );
    }
}

/// Emissions are only visible after finalize() (commit semantics).
///
/// This test verifies that pending emissions don't affect output
/// until the commit path calls finalize().
#[test]
fn engine_emits_only_post_commit() {
    let ch = make_channel_id("engine:commit-timing");

    let bus = MaterializationBus::new();

    // Emit before "commit"
    let emitter = ScopedEmitter::new(&bus, scope_hash(1), 42);
    emitter.emit(ch, vec![0xDE, 0xAD]).expect("emit");

    // Before finalize: bus has pending emissions
    assert!(!bus.is_empty(), "bus should have pending emissions");

    // Finalize (simulates commit)
    let report = bus.finalize();
    assert!(report.is_ok());

    // After finalize: bus is empty
    assert!(bus.is_empty(), "bus should be empty after finalize");

    // Result should contain the emission
    assert_eq!(report.channels.len(), 1);
    assert_eq!(report.channels[0].channel, ch);
}

/// Multiple channels finalize in deterministic order.
#[test]
fn engine_multi_channel_deterministic_order() {
    let ch_alpha = make_channel_id("channel:alpha");
    let ch_beta = make_channel_id("channel:beta");
    let ch_gamma = make_channel_id("channel:gamma");

    let execs = vec![
        SimulatedRuleExec {
            scope_hash: scope_hash(1),
            rule_id: 1,
            channel: ch_gamma, // Emit to "last" channel first
            data: vec![0x03],
        },
        SimulatedRuleExec {
            scope_hash: scope_hash(2),
            rule_id: 1,
            channel: ch_alpha, // Emit to "first" channel second
            data: vec![0x01],
        },
        SimulatedRuleExec {
            scope_hash: scope_hash(3),
            rule_id: 1,
            channel: ch_beta, // Emit to "middle" channel last
            data: vec![0x02],
        },
    ];

    // Run twice with different emission orderings
    let bus1 = MaterializationBus::new();
    let report1 = simulate_commit(&bus1, &execs);
    assert!(report1.is_ok());

    let reversed_execs: Vec<_> = execs.into_iter().rev().collect();
    let bus2 = MaterializationBus::new();
    let report2 = simulate_commit(&bus2, &reversed_execs);
    assert!(report2.is_ok());

    // Channel order must be deterministic
    let ids1: Vec<ChannelId> = report1.channels.iter().map(|r| r.channel).collect();
    let ids2: Vec<ChannelId> = report2.channels.iter().map(|r| r.channel).collect();
    assert_eq!(ids1, ids2, "channel order must be deterministic");

    // Data must match
    for (r1, r2) in report1.channels.iter().zip(report2.channels.iter()) {
        assert_eq!(r1.data, r2.data, "channel data must match");
    }
}

/// ScopedEmitter correctly derives EmitKey from execution context.
#[test]
fn scoped_emitter_derives_correct_emit_key() {
    let ch = make_channel_id("emitter:key-derivation");
    let bus = MaterializationBus::new();

    // Two emitters with same scope but different rule IDs should produce different keys
    let scope = scope_hash(42);
    let emitter_rule1 = ScopedEmitter::new(&bus, scope, 1);
    let emitter_rule2 = ScopedEmitter::new(&bus, scope, 2);

    emitter_rule1.emit(ch, vec![0x11]).expect("emit rule 1");
    emitter_rule2.emit(ch, vec![0x22]).expect("emit rule 2");

    let report = bus.finalize();
    assert!(report.is_ok());
    assert_eq!(report.channels.len(), 1);

    // Both emissions should be present (different EmitKeys)
    // Log format: [len][data][len][data]
    let data = &report.channels[0].data;
    assert_eq!(data.len(), 4 + 1 + 4 + 1, "two entries in log");
}

/// Subkeys allow multiple emissions from the same rule execution.
#[test]
fn scoped_emitter_subkey_differentiates_emissions() {
    let ch = make_channel_id("emitter:subkey");
    let bus = MaterializationBus::new();

    let scope = scope_hash(1);
    let emitter = ScopedEmitter::new(&bus, scope, 42);

    // Emit multiple values with different subkeys
    for i in 0..5u32 {
        emitter
            .emit_with_subkey(ch, i, vec![i as u8])
            .expect("emit with subkey");
    }

    let report = bus.finalize();
    assert!(report.is_ok());
    assert_eq!(report.channels.len(), 1);

    // All 5 emissions should be present
    let data = &report.channels[0].data;
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
    assert_eq!(count, 5, "all 5 subkey emissions preserved");
}

/// Same subkey from same scope+rule is rejected as duplicate.
#[test]
fn scoped_emitter_rejects_duplicate_subkey() {
    let ch = make_channel_id("emitter:dup-subkey");
    let bus = MaterializationBus::new();

    let emitter = ScopedEmitter::new(&bus, scope_hash(1), 42);

    emitter.emit_with_subkey(ch, 7, vec![0x11]).expect("first");
    let err = emitter
        .emit_with_subkey(ch, 7, vec![0x22])
        .expect_err("duplicate");

    assert_eq!(err.channel, ch);
}

// =============================================================================
// INTEGRATION: Bus + ReduceOp through ScopedEmitter
// =============================================================================

/// Reduce(Max) picks the lexicographically largest value.
#[test]
fn engine_reduce_max_deterministic() {
    let ch = make_channel_id("engine:reduce-max");

    let mut bus = MaterializationBus::new();
    bus.register_channel(ch, ChannelPolicy::Reduce(ReduceOp::Max));

    let execs = vec![
        SimulatedRuleExec {
            scope_hash: scope_hash(1),
            rule_id: 1,
            channel: ch,
            data: vec![0x10, 0xFF], // Smaller
        },
        SimulatedRuleExec {
            scope_hash: scope_hash(2),
            rule_id: 1,
            channel: ch,
            data: vec![0xFF, 0x00], // Larger (0xFF > 0x10 in first byte)
        },
    ];

    let report = simulate_commit(&bus, &execs);
    assert!(report.is_ok());
    assert_eq!(
        report.channels[0].data,
        vec![0xFF, 0x00],
        "Max should pick largest"
    );
}

/// Reduce(First) picks the value with the smallest EmitKey.
#[test]
fn engine_reduce_first_picks_smallest_key() {
    let ch = make_channel_id("engine:reduce-first");

    let mut bus = MaterializationBus::new();
    bus.register_channel(ch, ChannelPolicy::Reduce(ReduceOp::First));

    // scope_hash(1) < scope_hash(2), so exec_a's key is smaller
    let execs = vec![
        SimulatedRuleExec {
            scope_hash: scope_hash(2), // Larger key, but emitted first
            rule_id: 1,
            channel: ch,
            data: vec![0x22],
        },
        SimulatedRuleExec {
            scope_hash: scope_hash(1), // Smaller key, emitted second
            rule_id: 1,
            channel: ch,
            data: vec![0x11],
        },
    ];

    let report = simulate_commit(&bus, &execs);
    assert!(report.is_ok());
    assert_eq!(
        report.channels[0].data,
        vec![0x11],
        "First should pick smallest key (scope_hash(1))"
    );
}

/// Reduce(Concat) concatenates in EmitKey order.
#[test]
fn engine_reduce_concat_in_key_order() {
    let ch = make_channel_id("engine:reduce-concat");

    let mut bus = MaterializationBus::new();
    bus.register_channel(ch, ChannelPolicy::Reduce(ReduceOp::Concat));

    // Emit in "wrong" order to verify key-based ordering
    let execs = vec![
        SimulatedRuleExec {
            scope_hash: scope_hash(3),
            rule_id: 1,
            channel: ch,
            data: vec![0x33],
        },
        SimulatedRuleExec {
            scope_hash: scope_hash(1),
            rule_id: 1,
            channel: ch,
            data: vec![0x11],
        },
        SimulatedRuleExec {
            scope_hash: scope_hash(2),
            rule_id: 1,
            channel: ch,
            data: vec![0x22],
        },
    ];

    let report = simulate_commit(&bus, &execs);
    assert!(report.is_ok());

    // Concat should be in EmitKey order: scope_hash(1) < scope_hash(2) < scope_hash(3)
    assert_eq!(
        report.channels[0].data,
        vec![0x11, 0x22, 0x33],
        "Concat should be in EmitKey order"
    );
}

// =============================================================================
// BUS ABORT PATH
// =============================================================================

/// Clear removes all pending emissions (simulates abort).
#[test]
fn engine_abort_clears_pending_emissions() {
    let ch = make_channel_id("engine:abort");
    let bus = MaterializationBus::new();

    let emitter = ScopedEmitter::new(&bus, scope_hash(1), 42);
    emitter.emit(ch, vec![1, 2, 3]).expect("emit");
    assert!(!bus.is_empty());

    // Abort path: clear pending
    bus.clear();
    assert!(bus.is_empty());

    // Finalize after abort produces nothing
    let report = bus.finalize();
    assert!(report.is_ok());
    assert!(report.channels.is_empty());
}

// =============================================================================
// PERMUTATION INVARIANCE: EXHAUSTIVE FOR SMALL N
// =============================================================================

/// Generate all permutations using Heap's algorithm.
fn for_each_permutation<T: Clone>(items: &mut [T], mut f: impl FnMut(&[T])) {
    let n = items.len();
    if n == 0 {
        f(items);
        return;
    }

    let mut c = vec![0usize; n];
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

/// Exhaustive permutation test for N=4 rule executions.
#[test]
fn engine_permutation_invariant_n4() {
    let ch = make_channel_id("engine:perm4");

    let mut execs = vec![
        SimulatedRuleExec {
            scope_hash: scope_hash(4),
            rule_id: 1,
            channel: ch,
            data: vec![0x44],
        },
        SimulatedRuleExec {
            scope_hash: scope_hash(1),
            rule_id: 2,
            channel: ch,
            data: vec![0x12],
        },
        SimulatedRuleExec {
            scope_hash: scope_hash(2),
            rule_id: 1,
            channel: ch,
            data: vec![0x21],
        },
        SimulatedRuleExec {
            scope_hash: scope_hash(1),
            rule_id: 1,
            channel: ch,
            data: vec![0x11],
        },
    ];

    // Reference output (fresh bus to get actual reference)
    let bus_ref = MaterializationBus::new();
    let ref_report = simulate_commit(&bus_ref, &execs);
    assert!(ref_report.is_ok());
    let reference = finalized_to_bytes(&ref_report.channels);

    // Test all 4! = 24 permutations
    let mut perm_count = 0;
    for_each_permutation(&mut execs, |perm| {
        let bus = MaterializationBus::new();
        let report = simulate_commit(&bus, perm);
        assert!(report.is_ok());
        let bytes = finalized_to_bytes(&report.channels);
        assert_eq!(
            bytes, reference,
            "permutation {} should match reference",
            perm_count
        );
        perm_count += 1;
    });

    assert_eq!(perm_count, 24, "should test all 4! permutations");
}

/// Exhaustive permutation test with mixed policies.
#[test]
fn engine_permutation_invariant_mixed_policies() {
    let ch_log = make_channel_id("engine:perm-log");
    let ch_sum = make_channel_id("engine:perm-sum");

    let mut execs = vec![
        SimulatedRuleExec {
            scope_hash: scope_hash(1),
            rule_id: 1,
            channel: ch_log,
            data: vec![0xAA],
        },
        SimulatedRuleExec {
            scope_hash: scope_hash(2),
            rule_id: 1,
            channel: ch_sum,
            data: 10u64.to_le_bytes().to_vec(),
        },
        SimulatedRuleExec {
            scope_hash: scope_hash(3),
            rule_id: 1,
            channel: ch_log,
            data: vec![0xBB],
        },
        SimulatedRuleExec {
            scope_hash: scope_hash(4),
            rule_id: 1,
            channel: ch_sum,
            data: 20u64.to_le_bytes().to_vec(),
        },
    ];

    // Reference with policies
    let reference = {
        let mut bus = MaterializationBus::new();
        bus.register_channel(ch_sum, ChannelPolicy::Reduce(ReduceOp::Sum));
        let report = simulate_commit(&bus, &execs);
        assert!(report.is_ok());
        finalized_to_bytes(&report.channels)
    };

    // Test all 4! = 24 permutations
    let mut perm_count = 0;
    for_each_permutation(&mut execs, |perm| {
        let mut bus = MaterializationBus::new();
        bus.register_channel(ch_sum, ChannelPolicy::Reduce(ReduceOp::Sum));
        let report = simulate_commit(&bus, perm);
        assert!(report.is_ok());
        let bytes = finalized_to_bytes(&report.channels);
        assert_eq!(
            bytes, reference,
            "mixed policy permutation {} should match",
            perm_count
        );
        perm_count += 1;
    });

    assert_eq!(perm_count, 24);
}
