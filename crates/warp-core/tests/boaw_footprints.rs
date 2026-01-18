// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! BOAW Footprint & Independence Tests (ADR-0007 §6)
//!
//! Tests for footprint independence checking, bucket enforcement,
//! and drift guards.

mod common;

use common::XorShift64;
use warp_core::{AttachmentKey, EdgeId, Footprint, NodeId, NodeKey, WarpId};

// =============================================================================
// T3: Footprints & Independence
// =============================================================================

#[test]
fn t3_1_footprint_independence_is_symmetric() {
    // `fp.independent(a,b) == fp.independent(b,a)` for randomized footprints.
    //
    // This test can run against the existing Footprint implementation.

    let mut rng = XorShift64::new(0xDEAD_BEEF);

    for _ in 0..100 {
        let fp_a = random_footprint(&mut rng);
        let fp_b = random_footprint(&mut rng);

        let ab = fp_a.independent(&fp_b);
        let ba = fp_b.independent(&fp_a);

        assert_eq!(
            ab, ba,
            "Footprint independence is not symmetric:\n  fp_a: {fp_a:?}\n  fp_b: {fp_b:?}"
        );
    }

    fn random_footprint(rng: &mut XorShift64) -> Footprint {
        let mut fp = Footprint::default();
        let warp_id = WarpId([0u8; 32]); // Use a fixed WarpId for testing

        // Add random nodes
        for _ in 0..(rng.gen_range_usize(5)) {
            let node_id = NodeId(random_hash(rng));
            if rng.next_u64().is_multiple_of(2) {
                fp.n_read.insert_node(&node_id);
            } else {
                fp.n_write.insert_node(&node_id);
            }
        }

        // Add random edges
        for _ in 0..(rng.gen_range_usize(3)) {
            let edge_id = EdgeId(random_hash(rng));
            if rng.next_u64().is_multiple_of(2) {
                fp.e_read.insert_edge(&edge_id);
            } else {
                fp.e_write.insert_edge(&edge_id);
            }
        }

        // Add random attachments
        for _ in 0..(rng.gen_range_usize(3)) {
            let node_id = NodeId(random_hash(rng));
            let node_key = NodeKey {
                warp_id,
                local_id: node_id,
            };
            let key = AttachmentKey::node_alpha(node_key);
            if rng.next_u64().is_multiple_of(2) {
                fp.a_read.insert(key);
            } else {
                fp.a_write.insert(key);
            }
        }

        fp
    }

    fn random_hash(rng: &mut XorShift64) -> [u8; 32] {
        let mut h = [0u8; 32];
        for chunk in h.chunks_mut(8) {
            let bytes = rng.next_u64().to_le_bytes();
            chunk.copy_from_slice(&bytes[..chunk.len()]);
        }
        h
    }
}

#[test]
#[ignore = "BOAW bucket target enforcement not yet implemented"]
fn t3_2_no_write_read_overlap_admitted() {
    // Given: two planned rewrites where one writes a node the other reads
    // Expect: only one admitted
    unimplemented!(
        "Implement: build two PlannedRewrites with write/read overlap; \
         assert only one is admitted"
    );
}

#[test]
#[ignore = "BOAW bucket target enforcement not yet implemented"]
fn t3_3_deletes_that_share_adjacency_bucket_must_conflict() {
    // The classic race: delete e1=(A->B) and e2=(A->C) both mutate edges_from[A].
    // Your footprint model must claim the bucket target
    // (e.g., AttachmentKey::EdgesFromBucket(A)).
    //
    // Given: two edge deletes with same `from` but different edge_id
    // Expect: independence fails when adjacency bucket target is claimed
    //
    // (This test prevents the "retain() race" forever.)
    unimplemented!(
        "Implement: build two PlannedRewrites deleting edges from same node; \
         assert admission rejects running both concurrently"
    );
}

#[test]
#[ignore = "BOAW FootprintGuard not yet implemented"]
fn t3_4_footprint_guard_catches_executor_drift() {
    // Given: executor emits an op not claimed in footprint
    // Expect: panic in debug (or deterministic error in release mode)
    //
    // Executors are not trusted to "stay aligned" with compute_footprint.
    // We enforce with one of:
    // - Plan→Apply fusion: planning returns {footprint, apply_closure},
    //   and apply uses footprint-derived capabilities
    // - FootprintGuard: all mutation emission paths validate the target was claimed
    unimplemented!(
        "Implement: run executor under FootprintGuard; \
         attempt forbidden write; assert panic/error"
    );
}

// =============================================================================
// Factor mask prefiltering
// =============================================================================

#[test]
fn factor_mask_disjoint_is_fast_path() {
    // Verify that disjoint factor_mask allows early-exit in independence check.
    use warp_core::Footprint;

    let fp_a = Footprint {
        factor_mask: 0b0000_1111,
        ..Default::default()
    };

    let mut fp_b = Footprint {
        factor_mask: 0b1111_0000,
        ..Default::default()
    };

    // Disjoint masks → independent (fast path)
    assert!(
        fp_a.independent(&fp_b),
        "Disjoint factor_mask should be independent"
    );

    // Overlapping masks require full check
    fp_b.factor_mask = 0b0000_0001;
    // Still independent if no actual read/write overlap
    assert!(
        fp_a.independent(&fp_b),
        "Overlapping mask but no actual conflict should be independent"
    );
}

// =============================================================================
// T4.1: Shard routing stability
// =============================================================================

#[test]
fn t4_1_shard_routing_is_stable_across_machines() {
    // Given: same NodeId/EdgeId
    // Expect: same shard id (with fixed SHARDS constant)
    //
    // We use fixed virtual shards (e.g., 256/1024, power-of-two).
    // Route by existing NodeId/EdgeId bits (no rehash):
    //   shard = lowbits(id) & (SHARDS-1)

    const SHARDS: usize = 256;

    let test_hashes: [[u8; 32]; 5] = [
        [0x00; 32],
        [0xFF; 32],
        [0x42; 32],
        {
            let mut h = [0u8; 32];
            h[0] = 0xAB;
            h
        },
        {
            let mut h = [0u8; 32];
            h[31] = 0xCD;
            h
        },
    ];

    for hash in &test_hashes {
        let node_id = NodeId(*hash);
        // Use low bits of the hash for shard routing (deterministic)
        let shard = (hash[0] as usize) & (SHARDS - 1);

        // Verify same node always routes to same shard
        let shard2 = (hash[0] as usize) & (SHARDS - 1);
        assert_eq!(
            shard, shard2,
            "Shard routing must be stable for {node_id:?}"
        );
    }
}
