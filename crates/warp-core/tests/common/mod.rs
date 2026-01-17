// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(dead_code)]

use warp_core::{Engine, GraphStore, NodeId};

/// Compute the snapshot hash for a graph rooted at the given node.
///
/// Constructs an [`Engine`] from the provided `store` and `root`, then
/// returns the 32-byte hash of its current snapshot.
///
/// # Panics
///
/// Panics if the `root` node does not exist in `store`.
pub fn snapshot_hash_of(store: GraphStore, root: NodeId) -> [u8; 32] {
    let engine = Engine::new(store, root);
    engine.snapshot().hash
}

// =============================================================================
// MATERIALIZATION TEST UTILITIES
// =============================================================================

use warp_core::materialization::EmitKey;

/// Create a deterministic 32-byte hash for tests.
/// Sets the last byte to `n`, all other bytes are zero.
pub fn h(n: u8) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[31] = n;
    bytes
}

/// Create an EmitKey for tests with subkey=0.
pub fn key(scope: u8, rule: u32) -> EmitKey {
    EmitKey::new(h(scope), rule)
}

/// Create an EmitKey for tests with explicit subkey.
pub fn key_sub(scope: u8, rule: u32, subkey: u32) -> EmitKey {
    EmitKey::with_subkey(h(scope), rule, subkey)
}

/// Calls `f` for every permutation of `items` in-place.
/// Deterministic: Heap's algorithm generates all N! permutations.
pub fn for_each_permutation<T: Clone>(items: &mut [T], mut f: impl FnMut(&[T])) {
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
