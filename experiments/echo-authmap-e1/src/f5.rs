// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! The F5 scene fixture's write stream, replayed as authenticated-map records.
//!
//! [`script`] reproduces `warp-core/benches/support/fixtures/think_append.rs`
//! (fixture version 1, generator version 1) exactly; [`script_digest`] uses
//! the fixture's domain, so the pinned golden ties the two together. This
//! crate stays outside Echo's workspace, so it copies the generator rather
//! than importing it.
//!
//! Each append writes what the F5 rule writes: the entry node, its alpha
//! attachment and the bucket-to-entry edge. Records are keyed by
//! `H("e1.record.v0" || kind || id)`. This record mapping is the experiment's,
//! not an Echo profile, and engine-side ingress records are not modeled.

use crate::{Key, Op};

const KEY_LEN: usize = 16;

/// An F5 size tier (the subset of dimensions the write stream depends on).
#[derive(Clone, Copy, Debug)]
pub struct Tier {
    /// Tier name.
    pub name: &'static str,
    /// Bucket nodes under the root.
    pub buckets: u64,
    /// Appends, one per tick.
    pub appends: u64,
    /// Payload bytes per entry.
    pub payload_bytes: u64,
}

/// Tier S.
pub const S: Tier = Tier {
    name: "S",
    buckets: 8,
    appends: 200,
    payload_bytes: 16,
};
/// Tier M.
pub const M: Tier = Tier {
    name: "M",
    buckets: 32,
    appends: 5_000,
    payload_bytes: 16,
};

/// One scripted append.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Append {
    /// Bucket index.
    pub bucket: u32,
    /// Entry key.
    pub key: [u8; KEY_LEN],
    /// Entry payload.
    pub payload: Vec<u8>,
}

/// The fixture's SplitMix64 with its rejection-sampled `below`.
struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, bound: u64) -> u64 {
        let threshold = bound.wrapping_neg() % bound;
        loop {
            let draw = self.next_u64();
            if draw >= threshold {
                return draw % bound;
            }
        }
    }

    fn bytes(&mut self, len: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(len);
        while out.len() < len {
            let word = self.next_u64().to_le_bytes();
            let take = (len - out.len()).min(word.len());
            out.extend_from_slice(&word[..take]);
        }
        out
    }
}

/// The logical script: bucket, then key, then payload per append.
#[must_use]
pub fn script(tier: Tier, seed: u64) -> Vec<Append> {
    let mut rng = SplitMix64(seed);
    let payload_len = usize::try_from(tier.payload_bytes).unwrap_or(0);
    (0..tier.appends)
        .map(|_| {
            let bucket = u32::try_from(rng.below(tier.buckets)).unwrap_or(u32::MAX);
            let mut key = [0; KEY_LEN];
            key.copy_from_slice(&rng.bytes(KEY_LEN));
            Append {
                bucket,
                key,
                payload: rng.bytes(payload_len),
            }
        })
        .collect()
}

/// The fixture's script digest.
#[must_use]
pub fn script_digest(script: &[Append]) -> [u8; 32] {
    let mut hash = blake3::Hasher::new();
    hash.update(b"echo.bench.fixture.f5.script/v1\0");
    hash.update(&(script.len() as u64).to_le_bytes());
    for append in script {
        hash.update(&append.bucket.to_le_bytes());
        hash.update(&append.key);
        hash.update(&(append.payload.len() as u64).to_le_bytes());
        hash.update(&append.payload);
    }
    hash.finalize().into()
}

fn labeled(prefix: &[u8], label: &str) -> [u8; 32] {
    let mut hash = blake3::Hasher::new();
    hash.update(prefix);
    hash.update(label.as_bytes());
    hash.finalize().into()
}

fn domain(domain: &[u8], key: &[u8]) -> [u8; 32] {
    let mut hash = blake3::Hasher::new();
    hash.update(domain);
    hash.update(key);
    hash.finalize().into()
}

fn record_key(kind: u8, id: &[u8; 32]) -> Key {
    let mut hash = blake3::Hasher::new();
    hash.update(b"e1.record.v0");
    hash.update(&[kind]);
    hash.update(id);
    hash.finalize().into()
}

const NODE: u8 = 0;
const ALPHA: u8 = 1;
const EDGE: u8 = 2;

fn bucket_node(index: u32) -> [u8; 32] {
    labeled(b"node:", &format!("f5-bucket-{index:04}"))
}

/// The initial scene: a root linked to each bucket.
#[must_use]
pub fn initial_ops(tier: Tier) -> Vec<Op> {
    let root = labeled(b"node:", "f5-root");
    let mut ops = vec![(
        record_key(NODE, &root),
        Some(labeled(b"type:", "bench/f5/root").to_vec()),
    )];
    for index in 0..u32::try_from(tier.buckets).unwrap_or(0) {
        let bucket = bucket_node(index);
        ops.push((
            record_key(NODE, &bucket),
            Some(labeled(b"type:", "bench/f5/bucket").to_vec()),
        ));
        let edge = labeled(b"edge:", &format!("f5-root-bucket-{index:04}"));
        let mut value = root.to_vec();
        value.extend_from_slice(&bucket);
        value.extend_from_slice(&labeled(b"type:", "bench/f5/bucket-of"));
        ops.push((record_key(EDGE, &edge), Some(value)));
    }
    ops
}

/// The three record writes of one append.
#[must_use]
pub fn append_ops(append: &Append) -> Vec<Op> {
    let entry = domain(b"echo.bench.fixture.f5.entry/v1\0", &append.key);
    let edge = domain(b"echo.bench.fixture.f5.edge/v1\0", &append.key);
    let entry_type = labeled(b"type:", "bench/f5/entry");
    let mut alpha = entry_type.to_vec();
    alpha.extend_from_slice(&append.payload);
    let mut edge_value = bucket_node(append.bucket).to_vec();
    edge_value.extend_from_slice(&entry);
    edge_value.extend_from_slice(&labeled(b"type:", "bench/f5/contains"));
    vec![
        (record_key(NODE, &entry), Some(entry_type.to_vec())),
        (record_key(ALPHA, &entry), Some(alpha)),
        (record_key(EDGE, &edge), Some(edge_value)),
    ]
}
