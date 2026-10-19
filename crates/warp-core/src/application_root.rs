// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Experimental incremental application root, kept in shadow (M1).
//!
//! Feature `application_root_shadow` only. This is the M1 stage of
//! *Echo: Causal Commitments and Authenticated WARP Readings*: an incremental
//! authenticated reading kept beside the legacy state root, which it does not
//! replace or alter. The profile is E1's `radix-v0` compressed binary radix
//! trie, fixed here as the experimental application profile
//! [`APPLICATION_ROOT_PROFILE`]. It is not a production commitment format.
//!
//! # Scope and records
//!
//! The reading covers every node, edge and attachment record in every warp of
//! a worldline state, reachable or not. Each record is a leaf:
//!
//! ```text
//! key   = H("echo.application-root.record.v0" || kind || ids)
//! value = H("echo.application-root.value.v0" || canonical record bytes)
//! ```
//!
//! Boundary ports and warp-instance headers are outside this experimental
//! scope.
//!
//! # Trie
//!
//! ```text
//! EMPTY            = H("echo.application-root.empty.v0")
//! leaf(k, v)       = H("echo.application-root.leaf.v0" || k || v)
//! branch(b, l, r)  = H("echo.application-root.branch.v0" || u16be(b) || l || r)
//! ```
//!
//! A branch names the first bit at which its subtree's keys differ, so a key
//! set has exactly one trie. Nodes are immutable and shared through [`Arc`]:
//! a version is a root handle, cloning one is constant time, and earlier
//! versions stay readable while anything holds them.

use std::collections::BTreeSet;
use std::sync::Arc;

use crate::attachment::{AttachmentKey, AttachmentOwner, AttachmentPlane, AttachmentValue};
use crate::ident::{EdgeKey, Hash, NodeKey};
use crate::tick_patch::{SlotId, WarpOp};
use crate::warp_state::WarpState;

/// Identifier of the experimental application-root profile.
pub const APPLICATION_ROOT_PROFILE: &str = "echo.application-root.radix.v0";

const RECORD_DOMAIN: &[u8] = b"echo.application-root.record.v0";
const VALUE_DOMAIN: &[u8] = b"echo.application-root.value.v0";
const LEAF_DOMAIN: &[u8] = b"echo.application-root.leaf.v0";
const BRANCH_DOMAIN: &[u8] = b"echo.application-root.branch.v0";
const EMPTY_DOMAIN: &[u8] = b"echo.application-root.empty.v0";
const PROOF_VERSION: u8 = 1;

type Key = [u8; 32];

fn hash_parts(parts: &[&[u8]]) -> Hash {
    let mut hasher = blake3::Hasher::new();
    for part in parts {
        hasher.update(part);
    }
    hasher.finalize().into()
}

fn bit(key: &Key, index: usize) -> bool {
    key[index / 8] & (0x80 >> (index % 8)) != 0
}

fn first_difference(a: &Key, b: &Key) -> Option<usize> {
    a.iter()
        .zip(b)
        .position(|(x, y)| x != y)
        .map(|byte| byte * 8 + (a[byte] ^ b[byte]).leading_zeros() as usize)
}

fn split_bit_bytes(split: usize) -> [u8; 2] {
    u16::try_from(split).unwrap_or(u16::MAX).to_be_bytes()
}

/// The root of the empty application reading.
#[must_use]
pub fn empty_application_root() -> Hash {
    hash_parts(&[EMPTY_DOMAIN])
}

/// The leaf key of a record slot, or `None` for a boundary port.
#[must_use]
pub fn application_record_key(slot: &SlotId) -> Option<Hash> {
    Some(match slot {
        SlotId::Node(node) => hash_parts(&[RECORD_DOMAIN, &[1], &node.warp_id.0, &node.local_id.0]),
        SlotId::Edge(edge) => hash_parts(&[RECORD_DOMAIN, &[2], &edge.warp_id.0, &edge.local_id.0]),
        SlotId::Attachment(key) => {
            let (tag, warp_id, local_id) = match key.owner {
                AttachmentOwner::Node(node) => (3_u8, node.warp_id, node.local_id.0),
                AttachmentOwner::Edge(edge) => (4_u8, edge.warp_id, edge.local_id.0),
            };
            let plane = [u8::from(matches!(key.plane, AttachmentPlane::Beta))];
            hash_parts(&[RECORD_DOMAIN, &[tag], &warp_id.0, &local_id, &plane])
        }
        SlotId::Port(_) => return None,
    })
}

/// The leaf value digest of canonical record bytes.
#[must_use]
pub fn application_value_digest(record_bytes: &[u8]) -> Hash {
    hash_parts(&[VALUE_DOMAIN, record_bytes])
}

fn attachment_bytes(value: &AttachmentValue) -> Vec<u8> {
    match value {
        AttachmentValue::Atom(atom) => {
            let mut out = Vec::with_capacity(33 + atom.bytes.len());
            out.push(0);
            out.extend_from_slice(&atom.type_id.0);
            out.extend_from_slice(&atom.bytes);
            out
        }
        AttachmentValue::Descend(warp_id) => {
            let mut out = Vec::with_capacity(33);
            out.push(1);
            out.extend_from_slice(&warp_id.0);
            out
        }
    }
}

/// The canonical bytes of the record now at `slot`, or `None` when absent.
///
/// A node record is its type id; an edge record is its source, target and
/// type ids; an attachment is a tag and its atom type and bytes, or its
/// descended warp id.
#[must_use]
pub fn application_record_bytes(state: &WarpState, slot: &SlotId) -> Option<Vec<u8>> {
    match slot {
        SlotId::Node(node) => state
            .store(&node.warp_id)?
            .node(&node.local_id)
            .map(|record| record.ty.0.to_vec()),
        SlotId::Edge(edge) => state
            .store(&edge.warp_id)?
            .edge_index
            .get(&edge.local_id)
            .map(|ends| {
                let mut out = Vec::with_capacity(96);
                out.extend_from_slice(&ends.from.0);
                out.extend_from_slice(&ends.to.0);
                out.extend_from_slice(&ends.ty.0);
                out
            }),
        SlotId::Attachment(key) => match key.owner {
            AttachmentOwner::Node(node) => state
                .store(&node.warp_id)?
                .node_attachments
                .get(&node.local_id)
                .map(attachment_bytes),
            AttachmentOwner::Edge(edge) => state
                .store(&edge.warp_id)?
                .edge_attachments
                .get(&edge.local_id)
                .map(attachment_bytes),
        },
        SlotId::Port(_) => None,
    }
}

/// Every record slot in `state`.
fn record_slots(state: &WarpState) -> Vec<SlotId> {
    let mut slots = Vec::new();
    for (warp_id, store) in &state.stores {
        let warp_id = *warp_id;
        slots.extend(store.nodes.keys().map(|local_id| {
            SlotId::Node(NodeKey {
                warp_id,
                local_id: *local_id,
            })
        }));
        slots.extend(store.edge_index.keys().map(|local_id| {
            SlotId::Edge(EdgeKey {
                warp_id,
                local_id: *local_id,
            })
        }));
        slots.extend(store.node_attachments.keys().map(|local_id| {
            SlotId::Attachment(AttachmentKey::node_alpha(NodeKey {
                warp_id,
                local_id: *local_id,
            }))
        }));
        slots.extend(store.edge_attachments.keys().map(|local_id| {
            SlotId::Attachment(AttachmentKey::edge_beta(EdgeKey {
                warp_id,
                local_id: *local_id,
            }))
        }));
    }
    slots
}

#[derive(Debug)]
enum TrieNode {
    Leaf {
        key: Key,
        value: Hash,
        hash: Hash,
    },
    Branch {
        bit: usize,
        left: Arc<TrieNode>,
        right: Arc<TrieNode>,
        /// One key of the subtree, not hashed: it places diverging keys
        /// without descending.
        representative: Key,
        hash: Hash,
    },
}

impl TrieNode {
    fn hash(&self) -> Hash {
        match self {
            Self::Leaf { hash, .. } | Self::Branch { hash, .. } => *hash,
        }
    }

    fn representative(&self) -> Key {
        match self {
            Self::Leaf { key, .. } => *key,
            Self::Branch { representative, .. } => *representative,
        }
    }

    fn leaf(key: Key, value: Hash) -> Arc<Self> {
        Arc::new(Self::Leaf {
            key,
            value,
            hash: hash_parts(&[LEAF_DOMAIN, &key, &value]),
        })
    }
}

fn join(
    split: usize,
    left: Option<Arc<TrieNode>>,
    right: Option<Arc<TrieNode>>,
) -> Option<Arc<TrieNode>> {
    match (left, right) {
        (None, side) | (side, None) => side,
        (Some(left), Some(right)) => {
            let hash = hash_parts(&[
                BRANCH_DOMAIN,
                &split_bit_bytes(split),
                &left.hash(),
                &right.hash(),
            ]);
            Some(Arc::new(TrieNode::Branch {
                bit: split,
                representative: left.representative(),
                left,
                right,
                hash,
            }))
        }
    }
}

/// Builds the unique trie for sorted, distinct `entries`.
fn build(entries: &[(Key, Hash)]) -> Option<Arc<TrieNode>> {
    match entries {
        [] => None,
        [(key, value)] => Some(TrieNode::leaf(*key, *value)),
        [first, .., last] => {
            let split = first_difference(&first.0, &last.0).unwrap_or(0);
            let at = entries.partition_point(|(key, _)| !bit(key, split));
            join(split, build(&entries[..at]), build(&entries[at..]))
        }
    }
}

fn inserts(ops: &[(Key, Option<Hash>)]) -> Vec<(Key, Hash)> {
    ops.iter()
        .filter_map(|(key, value)| value.map(|value| (*key, value)))
        .collect()
}

/// Applies sorted, distinct `ops` to `subtree`.
fn update(subtree: Option<&Arc<TrieNode>>, ops: &[(Key, Option<Hash>)]) -> Option<Arc<TrieNode>> {
    if ops.is_empty() {
        return subtree.cloned();
    }
    let Some(node) = subtree else {
        return build(&inserts(ops));
    };
    match node.as_ref() {
        TrieNode::Leaf { key, value, .. } => {
            let mut merged = ops.to_vec();
            if let Err(at) = merged.binary_search_by(|(k, _)| k.cmp(key)) {
                merged.insert(at, (*key, Some(*value)));
            }
            build(&inserts(&merged))
        }
        TrieNode::Branch {
            bit: split,
            left,
            right,
            representative,
            ..
        } => {
            let split = *split;
            let diverging = ops
                .iter()
                .filter_map(|(key, _)| first_difference(key, representative))
                .filter(|&at| at < split)
                .min();
            if let Some(at) = diverging {
                // Every op agrees with the subtree above `at`; those on the
                // other side of `at` form a new sibling subtree.
                let cut = ops.partition_point(|(key, _)| !bit(key, at));
                let (zero, one) = ops.split_at(cut);
                let on_one = bit(representative, at);
                let (same, other) = if on_one { (one, zero) } else { (zero, one) };
                let inner = update(Some(node), same);
                let outer = build(&inserts(other));
                if on_one {
                    join(at, outer, inner)
                } else {
                    join(at, inner, outer)
                }
            } else {
                let cut = ops.partition_point(|(key, _)| !bit(key, split));
                join(
                    split,
                    update(Some(left), &ops[..cut]),
                    update(Some(right), &ops[cut..]),
                )
            }
        }
    }
}

/// A persistent version of the application reading.
#[derive(Clone, Debug, Default)]
pub(crate) struct ApplicationTrie {
    root: Option<Arc<TrieNode>>,
}

/// Why an application-root opening was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ApplicationOpeningError {
    /// The bytes are not one canonical encoding of an opening.
    #[error("malformed application-root opening")]
    Malformed,
    /// The opening is well formed but does not reach the trusted root.
    #[error("application-root opening does not reach the root")]
    RootMismatch,
    /// The slot is a boundary port, outside the reading's scope.
    #[error("slot is outside the application reading")]
    OutOfScope,
}

impl ApplicationTrie {
    /// A full rebuild from every record in `state`: the independent oracle
    /// the incremental path is checked against.
    pub(crate) fn rebuild(state: &WarpState) -> Self {
        let mut entries: Vec<(Key, Hash)> = record_slots(state)
            .iter()
            .filter_map(|slot| {
                let key = application_record_key(slot)?;
                let bytes = application_record_bytes(state, slot)?;
                Some((key, application_value_digest(&bytes)))
            })
            .collect();
        entries.sort_unstable_by_key(|entry| entry.0);
        Self {
            root: build(&entries),
        }
    }

    /// The root hash of this version.
    pub(crate) fn root(&self) -> Hash {
        self.root
            .as_ref()
            .map_or_else(empty_application_root, |node| node.hash())
    }

    /// The version after a commit that wrote `ops` (and declared
    /// `out_slots`), reading the written records from `state` after the
    /// commit. `None` when an op changes a whole warp instance, whose writes
    /// are not slot-addressable; the caller then rebuilds.
    pub(crate) fn advanced(
        &self,
        state: &WarpState,
        ops: &[WarpOp],
        out_slots: &[SlotId],
    ) -> Option<Self> {
        let mut written: BTreeSet<SlotId> = out_slots.iter().copied().collect();
        for op in ops {
            if !op.extend_written_slots(&mut written) {
                return None;
            }
        }
        let mut updates: Vec<(Key, Option<Hash>)> = written
            .iter()
            .filter_map(|slot| {
                let key = application_record_key(slot)?;
                let value = application_record_bytes(state, slot)
                    .map(|bytes| application_value_digest(&bytes));
                Some((key, value))
            })
            .collect();
        updates.sort_unstable_by_key(|update| update.0);
        updates.dedup_by(|later, earlier| later.0 == earlier.0);
        Some(Self {
            root: update(self.root.as_ref(), &updates),
        })
    }

    /// A canonical membership or absence opening for `slot`.
    ///
    /// Encoding: `0x01 || kind || u16be(n) || n × (u16be(bit) || sibling)
    /// || terminal`, where `kind` is `0x00` for the empty reading, `0x01`
    /// for membership (terminal: value digest) and `0x02` for absence
    /// (terminal: the reached leaf's key and value digest).
    pub(crate) fn opening(&self, slot: &SlotId) -> Result<Vec<u8>, ApplicationOpeningError> {
        let key = application_record_key(slot).ok_or(ApplicationOpeningError::OutOfScope)?;
        let mut proof = vec![PROOF_VERSION];
        let Some(mut node) = self.root.as_ref() else {
            proof.extend_from_slice(&[0x00, 0, 0]);
            return Ok(proof);
        };
        let mut path: Vec<(usize, Hash)> = Vec::new();
        let (found, value) = loop {
            match node.as_ref() {
                TrieNode::Leaf { key, value, .. } => break (*key, *value),
                TrieNode::Branch {
                    bit: split,
                    left,
                    right,
                    ..
                } => {
                    let (next, sibling) = if bit(&key, *split) {
                        (right, left)
                    } else {
                        (left, right)
                    };
                    path.push((*split, sibling.hash()));
                    node = next;
                }
            }
        };
        proof.push(if found == key { 0x01 } else { 0x02 });
        let count = u16::try_from(path.len()).map_err(|_| ApplicationOpeningError::Malformed)?;
        proof.extend_from_slice(&count.to_be_bytes());
        for (split, sibling) in &path {
            proof.extend_from_slice(&split_bit_bytes(*split));
            proof.extend_from_slice(sibling);
        }
        if found != key {
            proof.extend_from_slice(&found);
        }
        proof.extend_from_slice(&value);
        Ok(proof)
    }
}

/// Verifies an application-root opening for `slot` against `root`.
///
/// Returns the slot's value digest for membership and `None` for absence.
///
/// # Errors
///
/// Refuses non-canonical bytes, openings that do not reach `root`, and
/// out-of-scope slots.
pub fn verify_application_opening(
    root: &Hash,
    slot: &SlotId,
    proof: &[u8],
) -> Result<Option<Hash>, ApplicationOpeningError> {
    let key = application_record_key(slot).ok_or(ApplicationOpeningError::OutOfScope)?;
    let [PROOF_VERSION, kind, high, low, rest @ ..] = proof else {
        return Err(ApplicationOpeningError::Malformed);
    };
    let count = usize::from(u16::from_be_bytes([*high, *low]));
    let terminal_len = match kind {
        0x00 => 0,
        0x01 => 32,
        0x02 => 64,
        _ => return Err(ApplicationOpeningError::Malformed),
    };
    if count > 256 || rest.len() != count * 34 + terminal_len || (*kind == 0x00 && count != 0) {
        return Err(ApplicationOpeningError::Malformed);
    }
    if *kind == 0x00 {
        return if *root == empty_application_root() {
            Ok(None)
        } else {
            Err(ApplicationOpeningError::RootMismatch)
        };
    }
    let (steps, terminal) = rest.split_at(count * 34);
    let mut path: Vec<(usize, Hash)> = Vec::with_capacity(count);
    for step in steps.chunks_exact(34) {
        let split = usize::from(u16::from_be_bytes([step[0], step[1]]));
        if split > 255 || path.last().is_some_and(|(previous, _)| *previous >= split) {
            return Err(ApplicationOpeningError::Malformed);
        }
        let mut sibling = [0; 32];
        sibling.copy_from_slice(&step[2..]);
        path.push((split, sibling));
    }
    let mut value = [0; 32];
    let leaf_key = if *kind == 0x01 {
        value.copy_from_slice(&terminal[..32]);
        key
    } else {
        let mut found = [0; 32];
        found.copy_from_slice(&terminal[..32]);
        value.copy_from_slice(&terminal[32..]);
        if found == key
            || path
                .iter()
                .any(|(split, _)| bit(&found, *split) != bit(&key, *split))
        {
            return Err(ApplicationOpeningError::Malformed);
        }
        found
    };
    let mut hash = hash_parts(&[LEAF_DOMAIN, &leaf_key, &value]);
    for (split, sibling) in path.iter().rev() {
        let (left, right) = if bit(&key, *split) {
            (sibling, &hash)
        } else {
            (&hash, sibling)
        };
        hash = hash_parts(&[BRANCH_DOMAIN, &split_bit_bytes(*split), left, right]);
    }
    if hash != *root {
        return Err(ApplicationOpeningError::RootMismatch);
    }
    Ok((*kind == 0x01).then_some(value))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn key(seed: u64) -> Key {
        hash_parts(&[b"application-root-test-key", &seed.to_le_bytes()])
    }

    fn value(seed: u64) -> Hash {
        hash_parts(&[b"application-root-test-value", &seed.to_le_bytes()])
    }

    fn rebuilt(entries: &std::collections::BTreeMap<Key, Hash>) -> Hash {
        let entries: Vec<(Key, Hash)> = entries.iter().map(|(k, v)| (*k, *v)).collect();
        build(&entries).map_or_else(empty_application_root, |node| node.hash())
    }

    #[test]
    fn incremental_updates_match_rebuilds_and_old_versions_survive() {
        let mut records = std::collections::BTreeMap::new();
        let mut root: Option<Arc<TrieNode>> = None;
        let mut versions = Vec::new();
        let mut state = 0x5eed_u64;
        let mut next = || {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            state >> 33
        };
        for round in 0..200_u64 {
            let mut ops: Vec<(Key, Option<Hash>)> = (0..=next() % 12)
                .map(|_| {
                    let k = key(next() % 300);
                    let v = (next() % 4 != 0).then(|| value(round * 1_000 + next()));
                    (k, v)
                })
                .collect();
            ops.sort_by_key(|op| op.0);
            ops.dedup_by(|later, earlier| later.0 == earlier.0);
            for (k, v) in &ops {
                match v {
                    Some(v) => records.insert(*k, *v),
                    None => records.remove(k),
                };
            }
            root = update(root.as_ref(), &ops);
            let hash = root
                .as_ref()
                .map_or_else(empty_application_root, |node| node.hash());
            assert_eq!(hash, rebuilt(&records), "round {round}");
            versions.push((root.clone(), hash));
        }
        for (version, hash) in versions {
            let found = version.map_or_else(empty_application_root, |node| node.hash());
            assert_eq!(found, hash, "an old version changed");
        }
    }

    #[test]
    fn openings_verify_and_refuse_mutations() {
        let mut state = WarpState::new();
        let warp_id = crate::ident::make_warp_id("opening-test");
        let mut store = crate::graph::GraphStore::new(warp_id);
        let present = crate::ident::make_node_id("present");
        store.insert_node(
            present,
            crate::record::NodeRecord {
                ty: crate::ident::make_type_id("cell"),
            },
        );
        for index in 0..32 {
            store.insert_node(
                crate::ident::make_node_id(&format!("filler-{index}")),
                crate::record::NodeRecord {
                    ty: crate::ident::make_type_id("cell"),
                },
            );
        }
        state.upsert_instance(
            crate::warp_state::WarpInstance {
                warp_id,
                root_node: present,
                parent: None,
            },
            store,
        );
        let trie = ApplicationTrie::rebuild(&state);
        let root = trie.root();
        let member = SlotId::Node(NodeKey {
            warp_id,
            local_id: present,
        });
        let absent = SlotId::Node(NodeKey {
            warp_id,
            local_id: crate::ident::make_node_id("absent"),
        });
        let expected =
            application_record_bytes(&state, &member).map(|b| application_value_digest(&b));
        for (slot, want) in [(member, expected), (absent, None)] {
            let proof = trie.opening(&slot).expect("record slots are in scope");
            assert_eq!(verify_application_opening(&root, &slot, &proof), Ok(want));
            for at in 0..proof.len() {
                let mut bad = proof.clone();
                bad[at] ^= 0x01;
                assert!(
                    verify_application_opening(&root, &slot, &bad).is_err(),
                    "byte {at}"
                );
            }
            let mut longer = proof.clone();
            longer.push(0);
            assert_eq!(
                verify_application_opening(&root, &slot, &longer),
                Err(ApplicationOpeningError::Malformed)
            );
        }
        let transferred = trie.opening(&absent).expect("in scope");
        assert!(
            verify_application_opening(&root, &member, &transferred).is_err(),
            "an absence opening does not transfer to a present key"
        );
    }
}
