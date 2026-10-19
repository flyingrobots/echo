// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Study probe, not for merge: an M1-style shadow application root.
//!
//! Feature `shadow_root_probe` only. After each runtime commit, the records
//! at the patch's written slots are read back from the committed state and
//! applied to a persistent compressed radix map (E1's `radix-v0` shape under
//! an `m1.` domain). The shadow covers every node, edge and attachment record
//! in every warp, reachable or not. Legacy roots are unchanged and still
//! computed; this probe adds its cost and measures it.
//!
//! Not covered: boundary ports, warp-instance headers, and commits that are
//! later rolled back by the state journal.
#![allow(
    missing_docs,
    clippy::pedantic,
    clippy::nursery,
    clippy::panic,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::cast_possible_truncation
)]

use std::collections::{BTreeMap, HashMap};
use std::hash::{BuildHasherDefault, Hasher};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::attachment::{AttachmentOwner, AttachmentValue};
use crate::ident::WarpId;
use crate::tick_patch::SlotId;
use crate::warp_state::WarpState;
use crate::WorldlineId;

type Key = [u8; 32];
type Hash = [u8; 32];

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

const LEAF: &[u8] = b"m1.radix.leaf.v0";
const BRANCH: &[u8] = b"m1.radix.branch.v0";
const EMPTY: &[u8] = b"m1.radix.empty.v0";

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

impl RadixMap {
    fn new() -> Self {
        Self {
            nodes: NodeTable::default(),
            empty: empty_root(),
            meter: Meter::default(),
        }
    }

    /// Applies sorted-or-not `(key, value digest)` updates; the last wins.
    fn apply_digests(&mut self, root: Hash, ops: &mut Vec<(Key, Option<Hash>)>) -> Hash {
        let mut indexed: Vec<(usize, (Key, Option<Hash>))> = ops.drain(..).enumerate().collect();
        indexed.sort_by(|(i, a), (j, b)| a.0.cmp(&b.0).then(j.cmp(i)));
        indexed.dedup_by(|later, earlier| later.1 .0 == earlier.1 .0);
        let ops: Vec<(Key, Option<Hash>)> = indexed.into_iter().map(|(_, op)| op).collect();
        let subtree = (root != self.empty).then_some(root);
        self.update(subtree, &ops).unwrap_or(self.empty)
    }
}

fn h(parts: &[&[u8]]) -> Hash {
    let mut hasher = blake3::Hasher::new();
    for part in parts {
        hasher.update(part);
    }
    *hasher.finalize().as_bytes()
}

fn slot_key(slot: &SlotId) -> Option<Key> {
    Some(match slot {
        SlotId::Node(node) => h(&[b"m1.record.v0", &[1], &node.warp_id.0, &node.local_id.0]),
        SlotId::Edge(edge) => h(&[b"m1.record.v0", &[2], &edge.warp_id.0, &edge.local_id.0]),
        SlotId::Attachment(key) => {
            let (tag, warp, id) = match key.owner {
                AttachmentOwner::Node(node) => (3_u8, node.warp_id, node.local_id.0),
                AttachmentOwner::Edge(edge) => (4_u8, edge.warp_id, edge.local_id.0),
            };
            let plane = [matches!(key.plane, crate::attachment::AttachmentPlane::Beta) as u8];
            h(&[b"m1.record.v0", &[tag], &warp.0, &id, &plane])
        }
        SlotId::Port(_) => return None,
    })
}

fn attachment_bytes(value: &AttachmentValue) -> Vec<u8> {
    match value {
        AttachmentValue::Atom(atom) => {
            let mut out = vec![0];
            out.extend_from_slice(&atom.type_id.0);
            out.extend_from_slice(&atom.bytes);
            out
        }
        AttachmentValue::Descend(warp) => {
            let mut out = vec![1];
            out.extend_from_slice(&warp.0);
            out
        }
    }
}

/// The canonical record bytes now at `slot`, or `None` when it is empty.
fn slot_value(state: &WarpState, slot: &SlotId) -> Option<Vec<u8>> {
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
                let mut out = ends.from.0.to_vec();
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

fn value_digest(meter: &mut Meter, bytes: &[u8]) -> Hash {
    meter.payload_bytes += bytes.len() as u64;
    h(&[b"m1.value.v0", bytes])
}

/// Every record in `state`, as the slots that address it.
fn all_slots(state: &WarpState) -> Vec<SlotId> {
    let mut slots = Vec::new();
    for (warp_id, store) in &state.stores {
        let warp_id: WarpId = *warp_id;
        for id in store.nodes.keys() {
            slots.push(SlotId::Node(crate::ident::NodeKey {
                warp_id,
                local_id: *id,
            }));
        }
        for id in store.edge_index.keys() {
            slots.push(SlotId::Edge(crate::ident::EdgeKey {
                warp_id,
                local_id: *id,
            }));
        }
        for id in store.node_attachments.keys() {
            slots.push(SlotId::Attachment(
                crate::attachment::AttachmentKey::node_alpha(crate::ident::NodeKey {
                    warp_id,
                    local_id: *id,
                }),
            ));
        }
        for id in store.edge_attachments.keys() {
            slots.push(SlotId::Attachment(
                crate::attachment::AttachmentKey::edge_beta(crate::ident::EdgeKey {
                    warp_id,
                    local_id: *id,
                }),
            ));
        }
    }
    slots
}

/// The independent rebuild: a fresh map over every record in `state`.
pub fn rebuild_root(state: &WarpState) -> Hash {
    let mut map = RadixMap::new();
    let mut ops: Vec<(Key, Option<Hash>)> = Vec::new();
    for slot in all_slots(state) {
        if let (Some(key), Some(bytes)) = (slot_key(&slot), slot_value(state, &slot)) {
            ops.push((key, Some(h(&[b"m1.value.v0", &bytes]))));
        }
    }
    let empty = map.empty;
    map.apply_digests(empty, &mut ops)
}

struct Shadow {
    map: RadixMap,
    root: Hash,
    mirror: BTreeMap<Key, (SlotId, Hash)>,
}

/// One measured shadow update.
#[derive(Clone, Debug)]
pub struct Update {
    pub slots: usize,
    pub port_slots: usize,
    pub hash_bytes: u64,
    pub nodes_created: u64,
    pub elapsed: Duration,
    pub bootstrap: bool,
    pub sample: String,
    pub declared: usize,
    pub from_ops: usize,
    pub undeclared: usize,
    pub instance_ops: usize,
}

#[derive(Default)]
struct Registry {
    shadows: BTreeMap<WorldlineId, Shadow>,
    log: Vec<Update>,
}

static REMOVED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static REGISTRY: Mutex<Option<Registry>> = Mutex::new(None);

/// Updates `worldline_id`'s shadow root from the records at `slots`.
pub(crate) fn after_commit(
    worldline_id: WorldlineId,
    state: &WarpState,
    ops_in: &[crate::tick_patch::WarpOp],
    declared: &[SlotId],
) {
    let start = Instant::now();
    let mut guard = REGISTRY.lock().unwrap();
    let registry = guard.get_or_insert_with(Registry::default);
    let bootstrap = !registry.shadows.contains_key(&worldline_id);
    let shadow = registry.shadows.entry(worldline_id).or_insert_with(|| {
        let map = RadixMap::new();
        let root = map.empty;
        Shadow {
            map,
            root,
            mirror: BTreeMap::new(),
        }
    });
    shadow.map.meter = Meter::default();
    // The write set corroboration now uses: declared `out_slots` plus every
    // slot the ops write; an instance-level op leaves it unknown.
    let mut written: std::collections::BTreeSet<SlotId> = declared.iter().copied().collect();
    let mut instance_ops = 0;
    for op in ops_in {
        if !op.extend_written_slots(&mut written) {
            instance_ops += 1;
        }
    }
    let from_ops: Vec<SlotId> = written.into_iter().collect();
    let undeclared: Vec<SlotId> = from_ops
        .iter()
        .filter(|slot| declared.binary_search(slot).is_err() && !declared.contains(slot))
        .copied()
        .collect();
    let rebuild = bootstrap || instance_ops > 0;
    if rebuild && !bootstrap {
        shadow.map = RadixMap::new();
        shadow.root = shadow.map.empty;
        shadow.mirror.clear();
    }
    let owned;
    let slots: &[SlotId] = if rebuild {
        owned = all_slots(state);
        &owned
    } else {
        &from_ops
    };
    let mut ops = Vec::with_capacity(slots.len());
    let mut port_slots = 0;
    for slot in slots {
        let Some(key) = slot_key(slot) else {
            port_slots += 1;
            continue;
        };
        let digest =
            slot_value(state, slot).map(|bytes| value_digest(&mut shadow.map.meter, &bytes));
        match digest {
            Some(d) => {
                shadow.mirror.insert(key, (*slot, d));
            }
            None => {
                REMOVED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                shadow.mirror.remove(&key);
            }
        }
        ops.push((key, digest));
    }
    let root = shadow.root;
    shadow.root = shadow.map.apply_digests(root, &mut ops);
    let meter = shadow.map.meter;
    registry.log.push(Update {
        slots: slots.len(),
        port_slots,
        hash_bytes: meter.hash_bytes,
        nodes_created: meter.nodes_created,
        elapsed: start.elapsed(),
        bootstrap,
        sample: if registry.log.len() == 3 {
            format!("undeclared={undeclared:?}")
        } else {
            String::new()
        },
        declared: declared.len(),
        from_ops: from_ops.len(),
        undeclared: undeclared.len(),
        instance_ops,
    });
}

/// The current shadow root of `worldline_id`, if one exists.
pub fn root(worldline_id: WorldlineId) -> Option<Hash> {
    REGISTRY
        .lock()
        .unwrap()
        .as_ref()?
        .shadows
        .get(&worldline_id)
        .map(|shadow| shadow.root)
}

/// Removes and returns the update log.
pub fn take_log() -> Vec<Update> {
    REGISTRY
        .lock()
        .unwrap()
        .as_mut()
        .map(|r| std::mem::take(&mut r.log))
        .unwrap_or_default()
}

/// Forgets every shadow and the log.
pub fn reset() {
    *REGISTRY.lock().unwrap() = None;
}

/// Records where the shadow and the committed state disagree:
/// (slot, shadow holds it, state holds it).
pub fn diff(worldline_id: WorldlineId, state: &WarpState) -> Vec<(String, bool, bool)> {
    let guard = REGISTRY.lock().unwrap();
    let Some(shadow) = guard.as_ref().and_then(|r| r.shadows.get(&worldline_id)) else {
        return vec![("no shadow".into(), false, false)];
    };
    let mut truth: BTreeMap<Key, (SlotId, Hash)> = BTreeMap::new();
    for slot in all_slots(state) {
        if let (Some(key), Some(bytes)) = (slot_key(&slot), slot_value(state, &slot)) {
            truth.insert(key, (slot, h(&[b"m1.value.v0", &bytes])));
        }
    }
    let mut out = vec![(
        format!(
            "mirror={} truth={} removed_total={}",
            shadow.mirror.len(),
            truth.len(),
            REMOVED.load(std::sync::atomic::Ordering::Relaxed)
        ),
        false,
        false,
    )];
    for (key, (slot, digest)) in &shadow.mirror {
        match truth.get(key) {
            Some((_, d)) if d == digest => {}
            Some(_) => out.push((format!("{slot:?} value differs"), true, true)),
            None => out.push((format!("{slot:?}"), true, false)),
        }
    }
    for (key, (slot, _)) in &truth {
        if !shadow.mirror.contains_key(key) {
            out.push((format!("{slot:?}"), false, true));
        }
    }
    out
}
