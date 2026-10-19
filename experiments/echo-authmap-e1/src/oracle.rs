// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Independent rebuild oracles.
//!
//! Each oracle computes a root from the complete record set, written from
//! the profile text rather than from the incremental code: no node table, no
//! persistence, no shared helpers beyond BLAKE3. The incremental maps must
//! agree with these roots after every checked step.

use std::collections::BTreeMap;

use crate::{Hash, Key};

fn h(parts: &[&[u8]]) -> Hash {
    let mut hasher = blake3::Hasher::new();
    for part in parts {
        hasher.update(part);
    }
    *hasher.finalize().as_bytes()
}

fn key_bit(key: &Key, index: usize) -> bool {
    (key[index >> 3] >> (7 - (index & 7))) & 1 == 1
}

/// The `sparse-v0` root of `records` (key to value digest).
#[must_use]
pub fn sparse_root(records: &BTreeMap<Key, Hash>) -> Hash {
    let mut empty = [[0_u8; 32]; 257];
    empty[256] = h(&[b"e1.sparse.empty.v0"]);
    for depth in (0..256).rev() {
        empty[depth] = h(&[b"e1.sparse.node.v0", &empty[depth + 1], &empty[depth + 1]]);
    }
    let entries: Vec<(&Key, &Hash)> = records.iter().collect();
    sparse_subtree(&entries, 0, &empty)
}

fn sparse_subtree(entries: &[(&Key, &Hash)], depth: usize, empty: &[Hash; 257]) -> Hash {
    if entries.is_empty() {
        return empty[depth];
    }
    if depth == 256 {
        let (key, value) = entries[0];
        return h(&[b"e1.sparse.leaf.v0", key, value]);
    }
    let ones = entries.iter().position(|(key, _)| key_bit(key, depth));
    let (left, right) = entries.split_at(ones.unwrap_or(entries.len()));
    h(&[
        b"e1.sparse.node.v0",
        &sparse_subtree(left, depth + 1, empty),
        &sparse_subtree(right, depth + 1, empty),
    ])
}

/// The `radix-v0` root of `records` (key to value digest).
#[must_use]
pub fn radix_root(records: &BTreeMap<Key, Hash>) -> Hash {
    let entries: Vec<(&Key, &Hash)> = records.iter().collect();
    if entries.is_empty() {
        return h(&[b"e1.radix.empty.v0"]);
    }
    radix_subtree(&entries)
}

fn radix_subtree(entries: &[(&Key, &Hash)]) -> Hash {
    if let [(key, value)] = entries {
        return h(&[b"e1.radix.leaf.v0", *key, *value]);
    }
    // The lowest bit at which any two keys of the set differ.
    let split = (0..256)
        .find(|&index| {
            let first = key_bit(entries[0].0, index);
            entries.iter().any(|(key, _)| key_bit(key, index) != first)
        })
        .unwrap_or(255);
    let ones = entries
        .iter()
        .position(|(key, _)| key_bit(key, split))
        .unwrap_or(0);
    let (left, right) = entries.split_at(ones);
    #[expect(clippy::cast_possible_truncation, reason = "split < 256")]
    let encoded = (split as u16).to_be_bytes();
    h(&[
        b"e1.radix.branch.v0",
        &encoded,
        &radix_subtree(left),
        &radix_subtree(right),
    ])
}
