// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Experiment E1: two complete authenticated maps over 32-byte keys.
//!
//! [`sparse::SparseMap`] is a fixed-depth (256-level) sparse Merkle tree.
//! [`radix::RadixMap`] is a compressed binary radix (Patricia) Merkle trie.
//! Both are persistent: every update returns a new root and leaves every
//! earlier root readable and provable. Both expose membership and absence
//! proofs with strict canonical codecs, and both are checked against the
//! independent rebuilds in [`oracle`].
//!
//! The codecs here are experimental `v0` profiles fixed for repeatability.
//! They are not Echo commitment profiles and nothing in Echo reads them.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

pub mod f5;
pub mod oracle;
pub mod radix;
pub mod sparse;

/// A 32-byte map key. Bit 0 is the most significant bit of byte 0.
pub type Key = [u8; 32];
/// A 32-byte BLAKE3 digest.
pub type Hash = [u8; 32];

/// Bit `index` of `key`, most significant first.
#[must_use]
pub fn bit(key: &Key, index: usize) -> bool {
    key[index / 8] & (0x80 >> (index % 8)) != 0
}

/// The first bit index where `a` and `b` differ, or `None` when equal.
#[must_use]
pub fn first_difference(a: &Key, b: &Key) -> Option<usize> {
    a.iter()
        .zip(b)
        .position(|(x, y)| x != y)
        .map(|byte| byte * 8 + (a[byte] ^ b[byte]).leading_zeros() as usize)
}

/// Structural work counters. Payload hashing is counted apart from
/// structural hashing so value size does not hide maintenance cost.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Meter {
    /// Structural hash invocations (nodes, leaves, empty markers).
    pub hash_calls: u64,
    /// Bytes fed to structural hash invocations.
    pub hash_bytes: u64,
    /// Bytes of value payload hashed into value digests.
    pub payload_bytes: u64,
    /// Nodes newly inserted into the node store.
    pub nodes_created: u64,
    /// Serialized bytes of newly inserted nodes.
    pub node_bytes_written: u64,
    /// Node-store lookups.
    pub nodes_read: u64,
}

impl Meter {
    /// Hashes `parts` under the structural meter.
    pub fn hash(&mut self, parts: &[&[u8]]) -> Hash {
        let mut hasher = blake3::Hasher::new();
        for part in parts {
            hasher.update(part);
            self.hash_bytes += part.len() as u64;
        }
        self.hash_calls += 1;
        *hasher.finalize().as_bytes()
    }

    /// Hashes a value payload into its value digest.
    pub fn value_digest(&mut self, value: &[u8]) -> Hash {
        self.payload_bytes += value.len() as u64;
        value_digest(value)
    }
}

/// The value digest recorded in a leaf: `BLAKE3("e1.value.v0" || value)`.
#[must_use]
pub fn value_digest(value: &[u8]) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"e1.value.v0");
    hasher.update(value);
    *hasher.finalize().as_bytes()
}

/// Identity hasher for digest-keyed tables: the key is already uniform.
#[derive(Default)]
pub struct DigestHasher(u64);

impl Hasher for DigestHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        let mut word = [0; 8];
        let n = bytes.len().min(8);
        word[..n].copy_from_slice(&bytes[..n]);
        self.0 ^= u64::from_le_bytes(word);
    }
}

/// A content-addressed node table shared by every version of one map.
pub type NodeTable<N> = HashMap<Hash, N, BuildHasherDefault<DigestHasher>>;

/// One mutation: `Some(value)` sets the key, `None` deletes it.
pub type Op = (Key, Option<Vec<u8>>);

/// Why a proof was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProofError {
    /// The bytes are not one canonical encoding of a proof.
    Malformed,
    /// The proof is well formed but does not reach the trusted root.
    RootMismatch,
}

/// What a verified proof establishes about the queried key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Claim {
    /// The key is present with this value digest.
    Present(Hash),
    /// The key is absent.
    Absent,
}

/// The operations both candidates implement.
pub trait AuthMap {
    /// A short profile label used in measurement output.
    const PROFILE: &'static str;

    /// A map holding only the empty version.
    fn new() -> Self;
    /// The root of the empty map.
    fn empty_root(&self) -> Hash;
    /// Applies `ops` to the version at `root` and returns the new root.
    /// Within one batch, the last op for a key wins. `root` stays readable.
    fn apply(&mut self, root: Hash, ops: &[Op]) -> Hash;
    /// The value digest of `key` at `root`.
    fn get(&mut self, root: Hash, key: &Key) -> Option<Hash>;
    /// A canonical membership or absence proof for `key` at `root`.
    fn prove(&mut self, root: Hash, key: &Key) -> Vec<u8>;
    /// Verifies `proof` for `key` against `root` without any node table.
    ///
    /// # Errors
    ///
    /// Refuses non-canonical bytes and proofs that do not reach `root`.
    fn verify(root: &Hash, key: &Key, proof: &[u8]) -> Result<Claim, ProofError>;
    /// The work counters since the last reset.
    fn meter(&self) -> Meter;
    /// Resets the work counters.
    fn reset_meter(&mut self);
    /// Nodes retained across all versions.
    fn retained_nodes(&self) -> usize;
}

/// Sorts `ops` by key and keeps the last op for each key.
#[must_use]
pub fn normalize(ops: &[Op]) -> Vec<Op> {
    let mut indexed: Vec<(usize, &Op)> = ops.iter().enumerate().collect();
    indexed.sort_by(|(i, a), (j, b)| a.0.cmp(&b.0).then(j.cmp(i)));
    indexed.dedup_by(|later, earlier| later.1.0 == earlier.1.0);
    indexed.into_iter().map(|(_, op)| op.clone()).collect()
}

/// A small deterministic generator (SplitMix64) for reproducible workloads.
#[derive(Clone, Debug)]
pub struct SplitMix(u64);

impl SplitMix {
    /// A generator seeded with `seed`.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// The next 64-bit output.
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// A uniformly random key.
    pub fn key(&mut self) -> Key {
        let mut key = [0; 32];
        for chunk in key.chunks_mut(8) {
            chunk.copy_from_slice(&self.next_u64().to_le_bytes());
        }
        key
    }

    /// A value of `len` pseudo-random bytes.
    pub fn value(&mut self, len: usize) -> Vec<u8> {
        let mut value = Vec::with_capacity(len + 8);
        while value.len() < len {
            value.extend_from_slice(&self.next_u64().to_le_bytes());
        }
        value.truncate(len);
        value
    }

    /// A value below `bound`.
    pub fn below(&mut self, bound: u64) -> u64 {
        self.next_u64() % bound
    }
}
