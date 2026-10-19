// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! A compressed binary radix (Patricia) Merkle trie (profile `radix-v0`).
//!
//! A branch names the first bit at which its subtree's keys differ. Its
//! left subtree holds keys with that bit clear. A key set has exactly one
//! trie, so independent rebuilds must agree on the root.
//!
//! ```text
//! EMPTY            = H("e1.radix.empty.v0")
//! leaf(k, v)       = H("e1.radix.leaf.v0" || k || v)
//! branch(b, l, r)  = H("e1.radix.branch.v0" || u16be(b) || l || r)
//! ```
//!
//! Proof encoding: `0x01 || kind || u16be(n) || n × (u16be(bit) || sibling)
//! || terminal`. `kind` is `0x00` for the empty map (n = 0, no terminal),
//! `0x01` for membership (terminal: value digest) and `0x02` for absence
//! (terminal: the reached leaf's key and value digest). Bits must strictly
//! increase top-down.
//!
//! Each stored branch also keeps one representative key of its subtree. It
//! is not hashed; the update algorithm reads it to place diverging keys
//! without descending. It is counted in written bytes.

use crate::{
    AuthMap, Claim, Hash, Key, Meter, NodeTable, Op, ProofError, bit, first_difference, normalize,
};

const LEAF: &[u8] = b"e1.radix.leaf.v0";
const BRANCH: &[u8] = b"e1.radix.branch.v0";
const EMPTY: &[u8] = b"e1.radix.empty.v0";

#[derive(Clone, Copy, Debug)]
enum Node {
    Leaf(Key, Hash),
    Branch {
        bit: u16,
        left: Hash,
        right: Hash,
        representative: Key,
    },
}

fn empty_root() -> Hash {
    *blake3::hash(EMPTY).as_bytes()
}

/// A persistent compressed radix Merkle trie.
pub struct RadixMap {
    nodes: NodeTable<Node>,
    empty: Hash,
    meter: Meter,
}

#[expect(
    clippy::panic,
    reason = "a missing node is a broken table, not an input error"
)]
fn unreachable_node() -> ! {
    panic!("radix node table lost a reachable node")
}

impl RadixMap {
    fn node(&mut self, hash: &Hash) -> Node {
        self.meter.nodes_read += 1;
        match self.nodes.get(hash) {
            Some(node) => *node,
            None => unreachable_node(),
        }
    }

    fn store(&mut self, hash: Hash, node: Node, bytes: u64) {
        if self.nodes.insert(hash, node).is_none() {
            self.meter.nodes_created += 1;
            self.meter.node_bytes_written += bytes;
        }
    }

    fn leaf(&mut self, key: Key, value: Hash) -> Hash {
        let hash = self.meter.hash(&[LEAF, &key, &value]);
        self.store(hash, Node::Leaf(key, value), 64);
        hash
    }

    fn representative(&mut self, hash: &Hash) -> Key {
        match self.node(hash) {
            Node::Leaf(key, _) => key,
            Node::Branch { representative, .. } => representative,
        }
    }

    fn join(&mut self, split: usize, left: Option<Hash>, right: Option<Hash>) -> Option<Hash> {
        let (left, right) = match (left, right) {
            (None, side) | (side, None) => return side,
            (Some(left), Some(right)) => (left, right),
        };
        let representative = self.representative(&left);
        #[expect(clippy::cast_possible_truncation, reason = "split < 256")]
        let bit = split as u16;
        let hash = self
            .meter
            .hash(&[BRANCH, &bit.to_be_bytes(), &left, &right]);
        self.store(
            hash,
            Node::Branch {
                bit,
                left,
                right,
                representative,
            },
            98,
        );
        Some(hash)
    }

    /// Builds the unique trie for sorted, distinct `entries`.
    fn build(&mut self, entries: &[(Key, Hash)]) -> Option<Hash> {
        match entries {
            [] => None,
            [(key, value)] => Some(self.leaf(*key, *value)),
            [first, .., last] => {
                let split = first_difference(&first.0, &last.0).unwrap_or(0);
                let at = entries.partition_point(|(key, _)| !bit(key, split));
                let left = self.build(&entries[..at]);
                let right = self.build(&entries[at..]);
                self.join(split, left, right)
            }
        }
    }

    fn inserts(ops: &[(Key, Option<Hash>)]) -> Vec<(Key, Hash)> {
        ops.iter()
            .filter_map(|(key, value)| value.map(|value| (*key, value)))
            .collect()
    }

    fn update(&mut self, subtree: Option<Hash>, ops: &[(Key, Option<Hash>)]) -> Option<Hash> {
        if ops.is_empty() {
            return subtree;
        }
        let Some(hash) = subtree else {
            return self.build(&Self::inserts(ops));
        };
        match self.node(&hash) {
            Node::Leaf(key, value) => {
                let mut merged = ops.to_vec();
                if let Err(at) = merged.binary_search_by(|(k, _)| k.cmp(&key)) {
                    merged.insert(at, (key, Some(value)));
                }
                self.build(&Self::inserts(&merged))
            }
            Node::Branch {
                bit: split,
                left,
                right,
                representative,
            } => {
                let split = usize::from(split);
                let diverging = ops
                    .iter()
                    .filter_map(|(key, _)| first_difference(key, &representative))
                    .filter(|&at| at < split)
                    .min();
                if let Some(at) = diverging {
                    // Every op agrees with the subtree above `at`; those on the
                    // other side of `at` form a new sibling subtree.
                    let cut = ops.partition_point(|(key, _)| !bit(key, at));
                    let (zero, one) = ops.split_at(cut);
                    let (same, other) = if bit(&representative, at) {
                        (one, zero)
                    } else {
                        (zero, one)
                    };
                    let inner = self.update(Some(hash), same);
                    let outer = self.build(&Self::inserts(other));
                    if bit(&representative, at) {
                        self.join(at, outer, inner)
                    } else {
                        self.join(at, inner, outer)
                    }
                } else {
                    let cut = ops.partition_point(|(key, _)| !bit(key, split));
                    let left = self.update(Some(left), &ops[..cut]);
                    let right = self.update(Some(right), &ops[cut..]);
                    self.join(split, left, right)
                }
            }
        }
    }
}

impl AuthMap for RadixMap {
    const PROFILE: &'static str = "radix-v0";

    fn new() -> Self {
        Self {
            nodes: NodeTable::default(),
            empty: empty_root(),
            meter: Meter::default(),
        }
    }

    fn empty_root(&self) -> Hash {
        self.empty
    }

    fn apply(&mut self, root: Hash, ops: &[Op]) -> Hash {
        let ops = normalize(ops);
        let digests: Vec<(Key, Option<Hash>)> = ops
            .iter()
            .map(|(key, value)| (*key, value.as_deref().map(|v| self.meter.value_digest(v))))
            .collect();
        let subtree = (root != self.empty).then_some(root);
        self.update(subtree, &digests).unwrap_or(self.empty)
    }

    fn get(&mut self, root: Hash, key: &Key) -> Option<Hash> {
        if root == self.empty {
            return None;
        }
        let mut hash = root;
        loop {
            match self.node(&hash) {
                Node::Leaf(found, value) => return (found == *key).then_some(value),
                Node::Branch {
                    bit: split,
                    left,
                    right,
                    ..
                } => {
                    hash = if bit(key, usize::from(split)) {
                        right
                    } else {
                        left
                    }
                }
            }
        }
    }

    fn prove(&mut self, root: Hash, key: &Key) -> Vec<u8> {
        let mut proof = vec![0x01];
        if root == self.empty {
            proof.extend_from_slice(&[0x00, 0, 0]);
            return proof;
        }
        let mut path = Vec::new();
        let mut hash = root;
        let terminal = loop {
            match self.node(&hash) {
                Node::Leaf(found, value) => break (found, value),
                Node::Branch {
                    bit: split,
                    left,
                    right,
                    ..
                } => {
                    let (next, sibling) = if bit(key, usize::from(split)) {
                        (right, left)
                    } else {
                        (left, right)
                    };
                    path.push((split, sibling));
                    hash = next;
                }
            }
        };
        proof.push(if terminal.0 == *key { 0x01 } else { 0x02 });
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a path has at most 256 branches"
        )]
        proof.extend_from_slice(&(path.len() as u16).to_be_bytes());
        for (split, sibling) in &path {
            proof.extend_from_slice(&split.to_be_bytes());
            proof.extend_from_slice(sibling);
        }
        if terminal.0 != *key {
            proof.extend_from_slice(&terminal.0);
        }
        proof.extend_from_slice(&terminal.1);
        proof
    }

    fn verify(root: &Hash, key: &Key, proof: &[u8]) -> Result<Claim, ProofError> {
        let [0x01, kind, high, low, rest @ ..] = proof else {
            return Err(ProofError::Malformed);
        };
        let count = usize::from(u16::from_be_bytes([*high, *low]));
        let terminal_len = match kind {
            0x00 => 0,
            0x01 => 32,
            0x02 => 64,
            _ => return Err(ProofError::Malformed),
        };
        if count > 256 || rest.len() != count * 34 + terminal_len || (*kind == 0x00 && count != 0) {
            return Err(ProofError::Malformed);
        }
        if *kind == 0x00 {
            return if *root == empty_root() {
                Ok(Claim::Absent)
            } else {
                Err(ProofError::RootMismatch)
            };
        }
        let (steps, terminal) = rest.split_at(count * 34);
        let mut path = Vec::with_capacity(count);
        for step in steps.chunks_exact(34) {
            let split = usize::from(u16::from_be_bytes([step[0], step[1]]));
            if split > 255 || path.last().is_some_and(|(previous, _)| *previous >= split) {
                return Err(ProofError::Malformed);
            }
            let mut sibling = [0; 32];
            sibling.copy_from_slice(&step[2..]);
            path.push((split, sibling));
        }
        let (leaf_key, value) = if *kind == 0x01 {
            (*key, &terminal[..32])
        } else {
            let mut found = [0; 32];
            found.copy_from_slice(&terminal[..32]);
            if found == *key
                || path
                    .iter()
                    .any(|(split, _)| bit(&found, *split) != bit(key, *split))
            {
                return Err(ProofError::Malformed);
            }
            (found, &terminal[32..])
        };
        let mut digest = [0; 32];
        digest.copy_from_slice(value);
        let mut hash = {
            let mut hasher = blake3::Hasher::new();
            hasher.update(LEAF);
            hasher.update(&leaf_key);
            hasher.update(&digest);
            *hasher.finalize().as_bytes()
        };
        for (split, sibling) in path.iter().rev() {
            let (left, right) = if bit(key, *split) {
                (sibling, &hash)
            } else {
                (&hash, sibling)
            };
            let mut hasher = blake3::Hasher::new();
            hasher.update(BRANCH);
            #[expect(clippy::cast_possible_truncation, reason = "split < 256")]
            hasher.update(&(*split as u16).to_be_bytes());
            hasher.update(left);
            hasher.update(right);
            hash = *hasher.finalize().as_bytes();
        }
        if hash != *root {
            return Err(ProofError::RootMismatch);
        }
        Ok(if *kind == 0x01 {
            Claim::Present(digest)
        } else {
            Claim::Absent
        })
    }

    fn meter(&self) -> Meter {
        self.meter
    }

    fn reset_meter(&mut self) {
        self.meter = Meter::default();
    }

    fn retained_nodes(&self) -> usize {
        self.nodes.len()
    }
}
