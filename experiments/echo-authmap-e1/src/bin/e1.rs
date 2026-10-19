// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! E1 measurement harness. Prints one JSON object per line.
//!
//! Usage: `e1 <workload>...` where a workload is `w1`..`w10` or `all`. The
//! workloads, sizes and seeds are the ones declared in the E1 study before
//! any measurement ran; changing them makes a new experiment.
#![allow(clippy::print_stdout, clippy::cast_precision_loss, missing_docs)]

use std::collections::BTreeMap;
use std::time::Instant;

use echo_authmap_e1::{
    AuthMap, Claim, Hash, Key, Meter, Op, SplitMix, oracle, radix::RadixMap, sparse::SparseMap,
    value_digest,
};

type Records = BTreeMap<Key, Hash>;

trait Profile: AuthMap {
    fn oracle(records: &Records) -> Hash;
}

impl Profile for SparseMap {
    fn oracle(records: &Records) -> Hash {
        oracle::sparse_root(records)
    }
}

impl Profile for RadixMap {
    fn oracle(records: &Records) -> Hash {
        oracle::radix_root(records)
    }
}

const SCALES: [usize; 4] = [1_000, 4_000, 16_000, 64_000];
const VALUE_LEN: usize = 64;

#[derive(Default)]
struct Samples {
    meters: Vec<Meter>,
    nanos: Vec<f64>,
}

impl Samples {
    fn push(&mut self, meter: Meter, nanos: f64) {
        self.meters.push(meter);
        self.nanos.push(nanos);
    }

    fn json(&self) -> String {
        let field = |f: fn(&Meter) -> u64| {
            let values: Vec<f64> = self.meters.iter().map(|m| f(m) as f64).collect();
            summary(&values)
        };
        format!(
            "\"ops\":{},\"hash_bytes\":{},\"hash_calls\":{},\"payload_bytes\":{},\"nodes_created\":{},\"node_bytes_written\":{},\"nodes_read\":{},\"nanos\":{}",
            self.meters.len(),
            field(|m| m.hash_bytes),
            field(|m| m.hash_calls),
            field(|m| m.payload_bytes),
            field(|m| m.nodes_created),
            field(|m| m.node_bytes_written),
            field(|m| m.nodes_read),
            summary(&self.nanos)
        )
    }
}

fn summary(values: &[f64]) -> String {
    if values.is_empty() {
        return "null".to_owned();
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let at = |q: f64| sorted[((sorted.len() - 1) as f64 * q).round() as usize];
    let mean = sorted.iter().sum::<f64>() / sorted.len() as f64;
    format!(
        "{{\"mean\":{mean:.1},\"p50\":{:.1},\"p99\":{:.1},\"max\":{:.1}}}",
        at(0.5),
        at(0.99),
        sorted[sorted.len() - 1]
    )
}

fn emit(workload: &str, profile: &str, fields: &str) {
    println!("{{\"workload\":\"{workload}\",\"profile\":\"{profile}\",{fields}}}");
}

/// Applies `ops` and returns the new root with its metered work and time.
fn measured<M: AuthMap>(map: &mut M, root: Hash, ops: &[Op]) -> (Hash, Meter, f64) {
    map.reset_meter();
    let start = Instant::now();
    let root = map.apply(root, ops);
    let nanos = start.elapsed().as_nanos() as f64;
    (root, map.meter(), nanos)
}

fn shadow(records: &mut Records, ops: &[Op]) {
    for (key, value) in ops {
        match value {
            Some(value) => records.insert(*key, value_digest(value)),
            None => records.remove(key),
        };
    }
}

struct Loaded<M> {
    map: M,
    root: Hash,
    keys: Vec<Key>,
    records: Records,
}

fn load<M: Profile>(n: usize, rng: &mut SplitMix) -> (Loaded<M>, Meter, f64) {
    let ops: Vec<Op> = (0..n)
        .map(|_| (rng.key(), Some(rng.value(VALUE_LEN))))
        .collect();
    let mut map = M::new();
    let empty = map.empty_root();
    let (root, meter, nanos) = measured(&mut map, empty, &ops);
    let mut records = Records::new();
    shadow(&mut records, &ops);
    let keys = ops.into_iter().map(|(key, _)| key).collect();
    (
        Loaded {
            map,
            root,
            keys,
            records,
        },
        meter,
        nanos,
    )
}

fn parity<M: Profile>(loaded: &Loaded<M>) -> bool {
    loaded.root == M::oracle(&loaded.records)
}

/// W1: load N records in one batch, then 2,000 single-key replacements.
fn w1<M: Profile>() {
    for n in SCALES {
        let mut rng = SplitMix::new(0xE1_0001 ^ n as u64);
        let (mut l, build, build_nanos): (Loaded<M>, _, _) = load(n, &mut rng);
        let mut samples = Samples::default();
        for _ in 0..2_000 {
            let key = l.keys[rng.below(n as u64) as usize];
            let ops = vec![(key, Some(rng.value(VALUE_LEN)))];
            let (root, meter, nanos) = measured(&mut l.map, l.root, &ops);
            l.root = root;
            shadow(&mut l.records, &ops);
            samples.push(meter, nanos);
        }
        emit(
            "w1",
            M::PROFILE,
            &format!(
                "\"n\":{n},\"build_hash_bytes\":{},\"build_nodes\":{},\"build_nanos\":{build_nanos:.0},\"retained_nodes\":{},\"parity\":{},{}",
                build.hash_bytes,
                build.nodes_created,
                l.map.retained_nodes(),
                parity(&l),
                samples.json()
            ),
        );
    }
}

/// W2: batches of 1, 16, 256 and 4,096 replacements at N = 16,000.
fn w2<M: Profile>() {
    for size in [1_usize, 16, 256, 4_096] {
        let mut rng = SplitMix::new(0xE1_0002 ^ size as u64);
        let (mut l, _, _): (Loaded<M>, _, _) = load(16_000, &mut rng);
        let mut samples = Samples::default();
        for _ in 0..(4_096 / size).clamp(8, 512) {
            let ops: Vec<Op> = (0..size)
                .map(|_| {
                    (
                        l.keys[rng.below(16_000) as usize],
                        Some(rng.value(VALUE_LEN)),
                    )
                })
                .collect();
            let (root, meter, nanos) = measured(&mut l.map, l.root, &ops);
            l.root = root;
            shadow(&mut l.records, &ops);
            samples.push(meter, nanos);
        }
        emit(
            "w2",
            M::PROFILE,
            &format!(
                "\"n\":16000,\"batch\":{size},\"parity\":{},{}",
                parity(&l),
                samples.json()
            ),
        );
    }
}

/// W3: 4,000 single ops alternating delete-existing and insert-new at N = 16,000.
fn w3<M: Profile>() {
    let mut rng = SplitMix::new(0xE1_0003);
    let (mut l, _, _): (Loaded<M>, _, _) = load(16_000, &mut rng);
    let (mut inserts, mut deletes) = (Samples::default(), Samples::default());
    for step in 0..4_000 {
        let ops: Vec<Op> = if step % 2 == 0 {
            let at = rng.below(l.keys.len() as u64) as usize;
            vec![(l.keys.swap_remove(at), None)]
        } else {
            let key = rng.key();
            l.keys.push(key);
            vec![(key, Some(rng.value(VALUE_LEN)))]
        };
        let (root, meter, nanos) = measured(&mut l.map, l.root, &ops);
        l.root = root;
        shadow(&mut l.records, &ops);
        if step % 2 == 0 {
            deletes.push(meter, nanos)
        } else {
            inserts.push(meter, nanos)
        }
    }
    let parity = parity(&l);
    let retained = l.map.retained_nodes();
    emit(
        "w3",
        M::PROFILE,
        &format!(
            "\"op\":\"delete\",\"parity\":{parity},\"retained_nodes\":{retained},{}",
            deletes.json()
        ),
    );
    emit(
        "w3",
        M::PROFILE,
        &format!(
            "\"op\":\"insert\",\"parity\":{parity},\"retained_nodes\":{retained},{}",
            inserts.json()
        ),
    );
}

/// W4: 20,000 single replacements at N = 4,000 with every version retained,
/// reported in windows of 2,000; then 20 old versions are read and proven.
fn w4<M: Profile>() {
    let mut rng = SplitMix::new(0xE1_0004);
    let (mut l, _, _): (Loaded<M>, _, _) = load(4_000, &mut rng);
    let mut snapshots = Vec::new();
    for window in 0..10 {
        let mut samples = Samples::default();
        for step in 0..2_000 {
            if step % 1_000 == 0 {
                snapshots.push((l.root, l.records.clone()));
            }
            let ops = vec![(
                l.keys[rng.below(4_000) as usize],
                Some(rng.value(VALUE_LEN)),
            )];
            let (root, meter, nanos) = measured(&mut l.map, l.root, &ops);
            l.root = root;
            shadow(&mut l.records, &ops);
            samples.push(meter, nanos);
        }
        emit(
            "w4",
            M::PROFILE,
            &format!(
                "\"window\":{window},\"retained_nodes\":{},{}",
                l.map.retained_nodes(),
                samples.json()
            ),
        );
    }
    let mut checked = 0;
    let mut wrong = 0;
    for (root, records) in &snapshots {
        for _ in 0..10 {
            let key = l.keys[rng.below(4_000) as usize];
            let truth = records
                .get(&key)
                .copied()
                .map_or(Claim::Absent, Claim::Present);
            let proof = l.map.prove(*root, &key);
            checked += 1;
            if M::verify(root, &key, &proof) != Ok(truth)
                || l.map.get(*root, &key).map_or(Claim::Absent, Claim::Present) != truth
            {
                wrong += 1;
            }
        }
    }
    emit(
        "w4",
        M::PROFILE,
        &format!(
            "\"old_versions\":{},\"old_claims_checked\":{checked},\"old_claims_wrong\":{wrong},\"parity\":{}",
            snapshots.len(),
            parity(&l)
        ),
    );
}

/// W5: 1,000 membership and 1,000 absence proofs at each scale.
fn w5<M: Profile>() {
    for n in SCALES {
        let mut rng = SplitMix::new(0xE1_0005 ^ n as u64);
        let (mut l, _, _): (Loaded<M>, _, _) = load(n, &mut rng);
        for kind in ["member", "absent"] {
            let (mut bytes, mut prove, mut verify) = (Vec::new(), Vec::new(), Vec::new());
            let mut failures = 0;
            for _ in 0..1_000 {
                let key = if kind == "member" {
                    l.keys[rng.below(n as u64) as usize]
                } else {
                    rng.key()
                };
                let start = Instant::now();
                let proof = l.map.prove(l.root, &key);
                prove.push(start.elapsed().as_nanos() as f64);
                let start = Instant::now();
                let claim = M::verify(&l.root, &key, &proof);
                verify.push(start.elapsed().as_nanos() as f64);
                let truth = l
                    .records
                    .get(&key)
                    .copied()
                    .map_or(Claim::Absent, Claim::Present);
                failures += usize::from(claim != Ok(truth));
                bytes.push(proof.len() as f64);
            }
            emit(
                "w5",
                M::PROFILE,
                &format!(
                    "\"n\":{n},\"kind\":\"{kind}\",\"failures\":{failures},\"proof_bytes\":{},\"prove_nanos\":{},\"verify_nanos\":{}",
                    summary(&bytes),
                    summary(&prove),
                    summary(&verify)
                ),
            );
        }
    }
}

/// W6: adversarial prefixes inside a random N = 16,000 map: a 256-key comb
/// (every shared-prefix length) and 4,000 keys sharing a 40-bit prefix.
fn w6<M: Profile>() {
    for case in ["comb", "prefix40"] {
        let mut rng = SplitMix::new(0xE1_0006);
        let (mut l, _, _): (Loaded<M>, _, _) = load(16_000, &mut rng);
        let crafted: Vec<Key> = if case == "comb" {
            (0..256)
                .map(|ones| {
                    let mut key = [0_u8; 32];
                    for index in 0..ones {
                        key[index / 8] |= 0x80 >> (index % 8);
                    }
                    key
                })
                .collect()
        } else {
            (0..4_000)
                .map(|_| {
                    let mut key = rng.key();
                    key[..5].copy_from_slice(&[0x5a; 5]);
                    key
                })
                .collect()
        };
        let ops: Vec<Op> = crafted
            .iter()
            .map(|key| (*key, Some(rng.value(VALUE_LEN))))
            .collect();
        l.root = l.map.apply(l.root, &ops);
        shadow(&mut l.records, &ops);
        let mut samples = Samples::default();
        let mut bytes = Vec::new();
        for key in crafted.iter().rev().take(256) {
            let ops = vec![(*key, Some(rng.value(VALUE_LEN)))];
            let (root, meter, nanos) = measured(&mut l.map, l.root, &ops);
            l.root = root;
            shadow(&mut l.records, &ops);
            samples.push(meter, nanos);
            bytes.push(l.map.prove(l.root, key).len() as f64);
        }
        emit(
            "w6",
            M::PROFILE,
            &format!(
                "\"case\":\"{case}\",\"parity\":{},\"proof_bytes\":{},{}",
                parity(&l),
                summary(&bytes),
                samples.json()
            ),
        );
    }
}

/// W7: mutated proofs at N = 16,000. Counts accepted false claims and
/// accepted encodings that differ from the honest proof for the same claim.
fn w7<M: Profile>() {
    let mut rng = SplitMix::new(0xE1_0007);
    let (mut l, _, _): (Loaded<M>, _, _) = load(16_000, &mut rng);
    let (mut trials, mut false_claims, mut malleable, mut true_transfers) =
        (0_u64, 0_u64, 0_u64, 0_u64);
    for index in 0..1_000 {
        let key = if index % 2 == 0 {
            l.keys[rng.below(16_000) as usize]
        } else {
            rng.key()
        };
        let truth = |records: &Records, key: &Key| {
            records
                .get(key)
                .copied()
                .map_or(Claim::Absent, Claim::Present)
        };
        let proof = l.map.prove(l.root, &key);
        let mut variants: Vec<Vec<u8>> = Vec::new();
        for _ in 0..64 {
            let mut bad = proof.clone();
            let at = rng.below(bad.len() as u64) as usize;
            bad[at] ^= 1 + rng.below(255) as u8;
            variants.push(bad);
        }
        for _ in 0..8 {
            variants.push(proof[..rng.below(proof.len() as u64) as usize].to_vec());
        }
        let mut longer = proof.clone();
        longer.push(rng.below(256) as u8);
        variants.push(longer);
        for bad in &variants {
            trials += 1;
            if let Ok(claim) = M::verify(&l.root, &key, bad) {
                if claim == truth(&l.records, &key) {
                    malleable += 1
                } else {
                    false_claims += 1
                }
            }
        }
        for _ in 0..4 {
            let other = rng.key();
            trials += 1;
            if let Ok(claim) = M::verify(&l.root, &other, &proof) {
                if claim == truth(&l.records, &other) {
                    true_transfers += 1
                } else {
                    false_claims += 1
                }
            }
        }
    }
    emit(
        "w7",
        M::PROFILE,
        &format!(
            "\"trials\":{trials},\"false_claims\":{false_claims},\"malleable_encodings\":{malleable},\"true_cross_key_transfers\":{true_transfers}"
        ),
    );
}

/// W8: single replacements at N = 16,000 with 32 B, 1 KiB and 64 KiB values.
fn w8<M: Profile>() {
    for len in [32_usize, 1_024, 65_536] {
        let mut rng = SplitMix::new(0xE1_0008 ^ len as u64);
        let (mut l, _, _): (Loaded<M>, _, _) = load(16_000, &mut rng);
        let mut samples = Samples::default();
        for _ in 0..500 {
            let ops = vec![(l.keys[rng.below(16_000) as usize], Some(rng.value(len)))];
            let (root, meter, nanos) = measured(&mut l.map, l.root, &ops);
            l.root = root;
            samples.push(meter, nanos);
        }
        emit(
            "w8",
            M::PROFILE,
            &format!("\"value_len\":{len},{}", samples.json()),
        );
    }
}

/// W9: D nested instances of 1,000 records each. A leaf write in the deepest
/// instance is committed either by re-embedding each child root in its
/// parent (Descend chain) or by one outer instance map of instance roots.
fn w9<M: Profile>() {
    for depth in [1_usize, 4, 16] {
        let mut rng = SplitMix::new(0xE1_0009 ^ depth as u64);
        let link = |i: usize| {
            let mut key = [0xee_u8; 32];
            key[0] = u8::try_from(i).unwrap_or(0);
            key
        };
        let instances: Vec<(M, Hash, Vec<Key>)> = (0..depth)
            .map(|_| {
                let ops: Vec<Op> = (0..1_000)
                    .map(|_| (rng.key(), Some(rng.value(VALUE_LEN))))
                    .collect();
                let mut map = M::new();
                let root = map.apply(map.empty_root(), &ops);
                (map, root, ops.into_iter().map(|(key, _)| key).collect())
            })
            .collect();
        for strategy in ["descend_chain", "outer_map"] {
            let mut maps: Vec<(M, Hash, Vec<Key>)> = instances
                .iter()
                .map(|(_, _, keys)| {
                    let ops: Vec<Op> = keys
                        .iter()
                        .map(|key| (*key, Some(vec![0; VALUE_LEN])))
                        .collect();
                    let mut map = M::new();
                    let root = map.apply(map.empty_root(), &ops);
                    (map, root, keys.clone())
                })
                .collect();
            // Parents reference children: by root (chain) or by stable id (outer).
            for i in (0..depth.saturating_sub(1)).rev() {
                let child = if strategy == "descend_chain" {
                    maps[i + 1].1.to_vec()
                } else {
                    link(i + 1).to_vec()
                };
                let (map, root, _) = &mut maps[i];
                *root = map.apply(*root, &[(link(i + 1), Some(child))]);
            }
            let mut outer = M::new();
            let mut outer_root = outer.empty_root();
            if strategy == "outer_map" {
                let ops: Vec<Op> = maps
                    .iter()
                    .enumerate()
                    .map(|(i, (_, root, _))| (link(i), Some(root.to_vec())))
                    .collect();
                outer_root = outer.apply(outer_root, &ops);
            }
            let mut samples = Samples::default();
            let mut proof_bytes = Vec::new();
            for _ in 0..200 {
                let deepest = depth - 1;
                let key = maps[deepest].2[rng.below(1_000) as usize];
                let mut total = Meter::default();
                let start = Instant::now();
                let ops = vec![(key, Some(rng.value(VALUE_LEN)))];
                let (map, root, _) = &mut maps[deepest];
                let (new_root, meter, _) = measured(map, *root, &ops);
                *root = new_root;
                add(&mut total, meter);
                if strategy == "descend_chain" {
                    for i in (0..deepest).rev() {
                        let child = maps[i + 1].1.to_vec();
                        let (map, root, _) = &mut maps[i];
                        let (new_root, meter, _) =
                            measured(map, *root, &[(link(i + 1), Some(child))]);
                        *root = new_root;
                        add(&mut total, meter);
                    }
                } else {
                    let (new_root, meter, _) = measured(
                        &mut outer,
                        outer_root,
                        &[(link(deepest), Some(maps[deepest].1.to_vec()))],
                    );
                    outer_root = new_root;
                    add(&mut total, meter);
                }
                samples.push(total, start.elapsed().as_nanos() as f64);
                let deepest_root = maps[deepest].1;
                let mut bytes = maps[deepest].0.prove(deepest_root, &key).len();
                if strategy == "descend_chain" {
                    for (i, (map, root, _)) in maps.iter_mut().enumerate().take(deepest) {
                        bytes += map.prove(*root, &link(i + 1)).len();
                    }
                } else {
                    bytes += outer.prove(outer_root, &link(deepest)).len();
                }
                proof_bytes.push(bytes as f64);
            }
            emit(
                "w9",
                M::PROFILE,
                &format!(
                    "\"depth\":{depth},\"strategy\":\"{strategy}\",\"read_proof_bytes\":{},{}",
                    summary(&proof_bytes),
                    samples.json()
                ),
            );
        }
    }
}

/// W10: the F5 scene fixture's write stream (tiers S and M, seed 7), three
/// record writes per tick, reported in ten windows of ticks. At each window
/// boundary the oracle also rebuilds the root from every record, a
/// whole-record rescan standing in for a per-tick full recomputation.
fn w10<M: Profile>() {
    use echo_authmap_e1::f5;
    for tier in [f5::S, f5::M] {
        let script = f5::script(tier, 7);
        let mut map = M::new();
        let mut records = Records::new();
        let initial = f5::initial_ops(tier);
        let mut root = map.apply(map.empty_root(), &initial);
        shadow(&mut records, &initial);
        let window_len = script.len() / 10;
        for (window, chunk) in script.chunks(window_len).enumerate() {
            let mut samples = Samples::default();
            for append in chunk {
                let ops = f5::append_ops(append);
                let (new_root, meter, nanos) = measured(&mut map, root, &ops);
                root = new_root;
                shadow(&mut records, &ops);
                samples.push(meter, nanos);
            }
            let start = Instant::now();
            let rebuilt = M::oracle(&records);
            let rescan_nanos = start.elapsed().as_nanos() as f64;
            emit(
                "w10",
                M::PROFILE,
                &format!(
                    "\"tier\":\"{}\",\"window\":{window},\"ticks_through\":{},\"records\":{},\"parity\":{},\"rescan_nanos\":{rescan_nanos:.0},\"retained_nodes\":{},{}",
                    tier.name,
                    (window + 1) * window_len,
                    records.len(),
                    rebuilt == root,
                    map.retained_nodes(),
                    samples.json()
                ),
            );
        }
    }
}

fn add(total: &mut Meter, meter: Meter) {
    total.hash_calls += meter.hash_calls;
    total.hash_bytes += meter.hash_bytes;
    total.payload_bytes += meter.payload_bytes;
    total.nodes_created += meter.nodes_created;
    total.node_bytes_written += meter.node_bytes_written;
    total.nodes_read += meter.nodes_read;
}

fn run<M: Profile>(workload: &str) {
    match workload {
        "w1" => w1::<M>(),
        "w2" => w2::<M>(),
        "w3" => w3::<M>(),
        "w4" => w4::<M>(),
        "w5" => w5::<M>(),
        "w6" => w6::<M>(),
        "w7" => w7::<M>(),
        "w8" => w8::<M>(),
        "w9" => w9::<M>(),
        "w10" => w10::<M>(),
        other => eprintln!("unknown workload {other}"),
    }
}

fn main() {
    let mut workloads: Vec<String> = std::env::args().skip(1).collect();
    if workloads.is_empty() || workloads.iter().any(|w| w == "all") {
        workloads = (1..=10).map(|i| format!("w{i}")).collect();
    }
    for workload in &workloads {
        run::<RadixMap>(workload);
        run::<SparseMap>(workload);
    }
}
