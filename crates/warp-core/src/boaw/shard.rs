// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Virtual shard partitioning for Phase 6B parallel execution.
//!
//! # Shard Routing Specification (FROZEN)
//!
//! ```text
//! shard = LE_u64(node_id.as_bytes()[0..8]) & (NUM_SHARDS - 1)
//! ```
//!
//! This formula is **frozen once shipped**. Any change to shard routing
//! is a protocol-breaking change that requires version bumping.
//!
//! # Why Virtual Shards?
//!
//! Phase 6A used stride partitioning: worker N gets items [N, N+W, N+2W, ...].
//! This works but has poor cache locality—items touching related nodes scatter
//! across workers.
//!
//! Virtual shards group items by scope locality. Items with the same
//! `shard_of(scope)` are processed together, improving cache hit rates.
//! Workers dynamically claim shards via atomic counter (work-stealing).
//!
//! Determinism is still enforced by canonical merge, not execution order.

use super::ExecItem;
use crate::NodeId;

/// Number of virtual shards (power of two for fast modulo).
///
/// # Protocol Constant
///
/// This value is recorded in the commit hash domain via `compute_patch_digest_v2`.
/// Changing it would produce different hashes for the same world state.
///
/// If you ever need to change this, it requires a protocol version bump
/// and explicit migration handling.
///
/// # Value Choice
///
/// 256 shards provides:
/// - Fast modulo via `& 0xFF`
/// - Good load balance (256 >> typical worker counts of 8-64)
/// - Low overhead for small workloads
pub const NUM_SHARDS: usize = 256;

// Compile-time assertion: NUM_SHARDS must be a power of two for SHARD_MASK to work.
const _: () = assert!(
    NUM_SHARDS.is_power_of_two(),
    "NUM_SHARDS must be a power of two"
);

const SHARD_MASK: u64 = (NUM_SHARDS - 1) as u64;

/// Compute shard ID from a scope `NodeId`.
///
/// # Routing Formula (STABLE - DO NOT CHANGE)
///
/// ```text
/// shard = LE_u64(node_id.as_bytes()[0..8]) & (NUM_SHARDS - 1)
/// ```
///
/// This takes the first 8 bytes of the `NodeId`'s 32-byte hash,
/// interprets them as a little-endian u64, and masks to shard count.
///
/// # Stability Guarantee
///
/// Same `NodeId` → same shard, on every platform, forever.
/// This function's implementation is frozen once shipped.
///
/// # Example
///
/// ```
/// use warp_core::boaw::shard::shard_of;
/// use warp_core::NodeId;
///
/// let bytes: [u8; 32] = [
///     0xBE, 0xBA, 0xFE, 0xCA, 0xEF, 0xBE, 0xAD, 0xDE,
///     0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
///     0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
///     0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
/// ];
/// let node = NodeId(bytes);
/// // LE_u64 of first 8 bytes = 0xDEADBEEFCAFEBABE
/// // 0xDEADBEEFCAFEBABE & 0xFF = 0xBE = 190
/// assert_eq!(shard_of(&node), 190);
/// ```
#[inline]
pub fn shard_of(scope: &NodeId) -> usize {
    let bytes = scope.as_bytes();
    // NodeId is [u8; 32], so first 8 bytes always exist.
    // Copy directly to avoid try_into() + expect().
    let first_8: [u8; 8] = [
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ];
    let val = u64::from_le_bytes(first_8);
    (val & SHARD_MASK) as usize
}

/// A virtual shard containing `ExecItem`s that share locality.
#[derive(Debug, Default)]
pub struct VirtualShard {
    /// Items assigned to this shard.
    pub items: Vec<ExecItem>,
}

/// Partition `ExecItem`s into virtual shards by scope.
///
/// Items with the same `shard_of(scope)` are grouped together,
/// enabling cache locality when a worker processes a shard.
///
/// # Arguments
///
/// * `items` - Slice of execution items to partition. Items are copied into
///   their respective shards (since `ExecItem` is `Copy`).
///
/// # Returns
///
/// A vector of exactly `NUM_SHARDS` shards. Empty shards have empty `items` vecs.
pub fn partition_into_shards(items: &[ExecItem]) -> Vec<VirtualShard> {
    let mut shards: Vec<VirtualShard> = (0..NUM_SHARDS).map(|_| VirtualShard::default()).collect();

    for item in items {
        let shard_id = shard_of(&item.scope);
        shards[shard_id].items.push(*item);
    }

    shards
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // TEST VECTORS - These are regression tests. Do NOT change expected values.
    // If these fail, you broke shard routing determinism.
    // =========================================================================

    /// Test vector 1: Known bytes → known shard.
    ///
    /// `LE_u64`([0xBE, 0xBA, 0xFE, 0xCA, 0xEF, 0xBE, 0xAD, 0xDE]) = 0xDEADBEEFCAFEBABE
    /// 0xDEADBEEFCAFEBABE & 0xFF = 0xBE = 190
    #[test]
    fn test_vector_1_deadbeef() {
        let bytes: [u8; 32] = [
            0xBE, 0xBA, 0xFE, 0xCA, 0xEF, 0xBE, 0xAD, 0xDE, // first 8 bytes (LE)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let node = NodeId(bytes);
        assert_eq!(shard_of(&node), 190, "REGRESSION: shard routing changed!");
    }

    /// Test vector 2: All zeros → shard 0.
    #[test]
    fn test_vector_2_all_zeros() {
        let node = NodeId([0u8; 32]);
        assert_eq!(shard_of(&node), 0, "REGRESSION: shard routing changed!");
    }

    /// Test vector 3: Sequential low byte.
    ///
    /// `LE_u64`([0x2A, 0, 0, 0, 0, 0, 0, 0]) = 42
    /// 42 & 0xFF = 42
    #[test]
    fn test_vector_3_low_byte_42() {
        let mut bytes = [0u8; 32];
        bytes[0] = 42;
        let node = NodeId(bytes);
        assert_eq!(shard_of(&node), 42, "REGRESSION: shard routing changed!");
    }

    /// Test vector 4: High bits in first 8 bytes don't affect shard (masked out).
    ///
    /// `LE_u64`([0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]) = `u64::MAX`
    /// `u64::MAX` & 0xFF = 255
    #[test]
    fn test_vector_4_all_ones() {
        let mut bytes = [0u8; 32];
        bytes[0..8].copy_from_slice(&[0xFF; 8]);
        let node = NodeId(bytes);
        assert_eq!(shard_of(&node), 255, "REGRESSION: shard routing changed!");
    }

    /// Test vector 5: Bytes after index 7 are ignored.
    #[test]
    fn test_vector_5_only_first_8_bytes_matter() {
        let mut bytes1 = [0u8; 32];
        let mut bytes2 = [0u8; 32];
        bytes1[0..8].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        bytes2[0..8].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        bytes1[8] = 0xAA; // Different after byte 7
        bytes2[8] = 0xBB;
        bytes1[31] = 0xCC;
        bytes2[31] = 0xDD;

        assert_eq!(
            shard_of(&NodeId(bytes1)),
            shard_of(&NodeId(bytes2)),
            "Bytes after index 7 should not affect shard routing"
        );
    }

    // =========================================================================
    // Property tests
    // =========================================================================

    #[test]
    fn shard_routing_is_deterministic() {
        let node = crate::make_node_id("test-node-determinism");
        let shard1 = shard_of(&node);
        let shard2 = shard_of(&node);
        let shard3 = shard_of(&node);
        assert_eq!(shard1, shard2);
        assert_eq!(shard2, shard3);
    }

    #[test]
    fn shard_is_always_in_bounds() {
        // Test a variety of NodeIds
        for i in 0..1000 {
            let node = crate::make_node_id(&format!("node-{i}"));
            let shard = shard_of(&node);
            assert!(
                shard < NUM_SHARDS,
                "shard {shard} >= NUM_SHARDS {NUM_SHARDS} for node-{i}"
            );
        }
    }

    #[test]
    fn partition_creates_correct_shard_count() {
        let shards = partition_into_shards(&[]);
        assert_eq!(shards.len(), NUM_SHARDS);
    }

    #[test]
    fn partition_distributes_items_correctly() {
        use crate::graph_view::GraphView;
        use crate::tick_delta::{OpOrigin, TickDelta};

        fn dummy_exec(_: GraphView<'_>, _: &NodeId, _: &mut TickDelta) {}

        let items: Vec<ExecItem> = (0..100)
            .map(|i| {
                let node = crate::make_node_id(&format!("partition-test-{i}"));
                ExecItem::new(dummy_exec, node, OpOrigin::default())
            })
            .collect();

        let shards = partition_into_shards(&items);

        // Verify all items are accounted for
        let total_items: usize = shards.iter().map(|s| s.items.len()).sum();
        assert_eq!(total_items, 100);

        // Verify each item is in its correct shard
        for (shard_id, shard) in shards.iter().enumerate() {
            for item in &shard.items {
                assert_eq!(shard_of(&item.scope), shard_id, "Item in wrong shard");
            }
        }
    }
}
