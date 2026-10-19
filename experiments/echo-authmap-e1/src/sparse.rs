// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! A fixed-depth sparse Merkle tree over 256-bit keys (profile `sparse-v0`).
//!
//! Every key has one leaf slot at depth 256. An empty subtree at depth `d`
//! hashes to the precomputed `EMPTY[d]` and is never stored.
//!
//! ```text
//! EMPTY[256]   = H("e1.sparse.empty.v0")
//! EMPTY[d]     = H("e1.sparse.node.v0" || EMPTY[d+1] || EMPTY[d+1])
//! leaf(k, v)   = H("e1.sparse.leaf.v0" || k || v)
//! node(l, r)   = H("e1.sparse.node.v0" || l || r)
//! ```
//!
//! Proof encoding: `0x01 || kind || [value digest] || bitmap[32] || siblings`.
//! `kind` is `0x00` for absence and `0x01` for membership. Bit `d` of the
//! bitmap (most significant first) marks a non-empty sibling at depth `d`;
//! only those siblings follow, top-down. A flagged sibling equal to the
//! empty hash is non-canonical and refused.

use crate::{AuthMap, Claim, Hash, Key, Meter, NodeTable, Op, ProofError, bit, normalize};

const DEPTH: usize = 256;
const LEAF: &[u8] = b"e1.sparse.leaf.v0";
const NODE: &[u8] = b"e1.sparse.node.v0";
const EMPTY: &[u8] = b"e1.sparse.empty.v0";

#[derive(Clone, Copy, Debug)]
enum Node {
    Branch(Hash, Hash),
    Leaf(Key, Hash),
}

/// Empty-subtree hashes, `empty[d]` for depths `0..=256`, computed once.
fn empty_hashes() -> &'static [Hash] {
    static EMPTY_HASHES: std::sync::OnceLock<Vec<Hash>> = std::sync::OnceLock::new();
    EMPTY_HASHES.get_or_init(compute_empty_hashes)
}

fn compute_empty_hashes() -> Vec<Hash> {
    let mut empty = vec![[0; 32]; DEPTH + 1];
    empty[DEPTH] = *blake3::hash(EMPTY).as_bytes();
    for depth in (0..DEPTH).rev() {
        let mut hasher = blake3::Hasher::new();
        hasher.update(NODE);
        hasher.update(&empty[depth + 1]);
        hasher.update(&empty[depth + 1]);
        empty[depth] = *hasher.finalize().as_bytes();
    }
    empty
}

/// A persistent fixed-depth sparse Merkle tree.
pub struct SparseMap {
    nodes: NodeTable<Node>,
    empty: &'static [Hash],
    meter: Meter,
}

impl SparseMap {
    fn store(&mut self, hash: Hash, node: Node) {
        if self.nodes.insert(hash, node).is_none() {
            self.meter.nodes_created += 1;
            self.meter.node_bytes_written += 64;
        }
    }

    fn children(&mut self, hash: Hash, depth: usize) -> (Hash, Hash) {
        if hash == self.empty[depth] {
            return (self.empty[depth + 1], self.empty[depth + 1]);
        }
        self.meter.nodes_read += 1;
        match self.nodes.get(&hash) {
            Some(Node::Branch(left, right)) => (*left, *right),
            Some(Node::Leaf(..)) | None => unreachable_node(),
        }
    }

    fn update(&mut self, hash: Hash, depth: usize, ops: &[(Key, Option<Hash>)]) -> Hash {
        if ops.is_empty() {
            return hash;
        }
        if depth == DEPTH {
            let (key, value) = ops[ops.len() - 1];
            return match value {
                None => self.empty[DEPTH],
                Some(value) => {
                    let leaf = self.meter.hash(&[LEAF, &key, &value]);
                    self.store(leaf, Node::Leaf(key, value));
                    leaf
                }
            };
        }
        let (left, right) = self.children(hash, depth);
        let split = ops.partition_point(|(key, _)| !bit(key, depth));
        let left = self.update(left, depth + 1, &ops[..split]);
        let right = self.update(right, depth + 1, &ops[split..]);
        if left == self.empty[depth + 1] && right == self.empty[depth + 1] {
            return self.empty[depth];
        }
        let node = self.meter.hash(&[NODE, &left, &right]);
        self.store(node, Node::Branch(left, right));
        node
    }

    /// The sibling hashes on `key`'s path, top-down, and the leaf slot hash.
    fn path(&mut self, root: Hash, key: &Key) -> (Vec<Hash>, Hash) {
        let mut siblings = Vec::with_capacity(DEPTH);
        let mut hash = root;
        for depth in 0..DEPTH {
            let (left, right) = self.children(hash, depth);
            if bit(key, depth) {
                siblings.push(left);
                hash = right;
            } else {
                siblings.push(right);
                hash = left;
            }
        }
        (siblings, hash)
    }
}

#[expect(
    clippy::panic,
    reason = "a missing node is a broken table, not an input error"
)]
fn unreachable_node() -> ! {
    panic!("sparse node table lost a reachable branch")
}

impl AuthMap for SparseMap {
    const PROFILE: &'static str = "sparse-v0";

    fn new() -> Self {
        Self {
            nodes: NodeTable::default(),
            empty: empty_hashes(),
            meter: Meter::default(),
        }
    }

    fn empty_root(&self) -> Hash {
        self.empty[0]
    }

    fn apply(&mut self, root: Hash, ops: &[Op]) -> Hash {
        let ops = normalize(ops);
        let digests: Vec<(Key, Option<Hash>)> = ops
            .iter()
            .map(|(key, value)| (*key, value.as_deref().map(|v| self.meter.value_digest(v))))
            .collect();
        self.update(root, 0, &digests)
    }

    fn get(&mut self, root: Hash, key: &Key) -> Option<Hash> {
        let mut hash = root;
        for depth in 0..DEPTH {
            if hash == self.empty[depth] {
                return None;
            }
            let (left, right) = self.children(hash, depth);
            hash = if bit(key, depth) { right } else { left };
        }
        if hash == self.empty[DEPTH] {
            return None;
        }
        self.meter.nodes_read += 1;
        match self.nodes.get(&hash) {
            Some(Node::Leaf(found, value)) if found == key => Some(*value),
            Some(Node::Leaf(..) | Node::Branch(..)) | None => unreachable_node(),
        }
    }

    fn prove(&mut self, root: Hash, key: &Key) -> Vec<u8> {
        let (siblings, slot) = self.path(root, key);
        let mut proof = vec![0x01];
        if slot == self.empty[DEPTH] {
            proof.push(0x00);
        } else {
            self.meter.nodes_read += 1;
            let Some(Node::Leaf(_, value)) = self.nodes.get(&slot) else {
                unreachable_node()
            };
            proof.push(0x01);
            proof.extend_from_slice(value);
        }
        let mut bitmap = [0_u8; 32];
        for (depth, sibling) in siblings.iter().enumerate() {
            if *sibling != self.empty[depth + 1] {
                bitmap[depth / 8] |= 0x80 >> (depth % 8);
            }
        }
        proof.extend_from_slice(&bitmap);
        for (depth, sibling) in siblings.iter().enumerate() {
            if *sibling != self.empty[depth + 1] {
                proof.extend_from_slice(sibling);
            }
        }
        proof
    }

    fn verify(root: &Hash, key: &Key, proof: &[u8]) -> Result<Claim, ProofError> {
        let empty = empty_hashes();
        let (claim, rest) = match proof {
            [0x01, 0x00, rest @ ..] => (Claim::Absent, rest),
            [0x01, 0x01, rest @ ..] if rest.len() >= 32 => {
                let mut value = [0; 32];
                value.copy_from_slice(&rest[..32]);
                (Claim::Present(value), &rest[32..])
            }
            _ => return Err(ProofError::Malformed),
        };
        if rest.len() < 32 {
            return Err(ProofError::Malformed);
        }
        let (bitmap, siblings) = rest.split_at(32);
        let flagged = bitmap
            .iter()
            .map(|byte| byte.count_ones() as usize)
            .sum::<usize>();
        if siblings.len() != flagged * 32 {
            return Err(ProofError::Malformed);
        }
        let mut hash = match claim {
            Claim::Absent => empty[DEPTH],
            Claim::Present(value) => {
                let mut hasher = blake3::Hasher::new();
                hasher.update(LEAF);
                hasher.update(key);
                hasher.update(&value);
                *hasher.finalize().as_bytes()
            }
        };
        let mut next = flagged;
        for depth in (0..DEPTH).rev() {
            let sibling = if bitmap[depth / 8] & (0x80 >> (depth % 8)) == 0 {
                empty[depth + 1]
            } else {
                next -= 1;
                let mut sibling = [0; 32];
                sibling.copy_from_slice(&siblings[next * 32..next * 32 + 32]);
                if sibling == empty[depth + 1] {
                    return Err(ProofError::Malformed);
                }
                sibling
            };
            let (left, right) = if bit(key, depth) {
                (sibling, hash)
            } else {
                (hash, sibling)
            };
            hash = if left == empty[depth + 1] && right == empty[depth + 1] {
                empty[depth]
            } else {
                let mut hasher = blake3::Hasher::new();
                hasher.update(NODE);
                hasher.update(&left);
                hasher.update(&right);
                *hasher.finalize().as_bytes()
            };
        }
        if hash == *root {
            Ok(claim)
        } else {
            Err(ProofError::RootMismatch)
        }
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
