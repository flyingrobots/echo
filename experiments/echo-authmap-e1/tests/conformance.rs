// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! E1 gates G1, G2 and G4: rebuild parity, persistence and proof soundness,
//! run identically against both candidates.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic, missing_docs)]

use std::collections::BTreeMap;

use echo_authmap_e1::{
    AuthMap, Claim, Hash, Key, Op, ProofError, SplitMix, oracle, radix::RadixMap,
    sparse::SparseMap, value_digest,
};

type Records = BTreeMap<Key, Hash>;

trait Oracle {
    fn oracle(records: &Records) -> Hash;
}

impl Oracle for SparseMap {
    fn oracle(records: &Records) -> Hash {
        oracle::sparse_root(records)
    }
}

impl Oracle for RadixMap {
    fn oracle(records: &Records) -> Hash {
        oracle::radix_root(records)
    }
}

fn apply_shadow(records: &mut Records, ops: &[Op]) {
    for (key, value) in ops {
        match value {
            Some(value) => records.insert(*key, value_digest(value)),
            None => records.remove(key),
        };
    }
}

fn check_claim<M: AuthMap>(map: &mut M, root: Hash, records: &Records, key: &Key) {
    let expected = records.get(key).copied();
    assert_eq!(map.get(root, key), expected, "{}: get", M::PROFILE);
    let proof = map.prove(root, key);
    let claim = M::verify(&root, key, &proof).expect("an honest proof verifies");
    assert_eq!(
        claim,
        expected.map_or(Claim::Absent, Claim::Present),
        "{}: proof claim",
        M::PROFILE
    );
}

/// G1 and G2: random batches with inserts, replacements and deletes; parity
/// with the oracle after every batch; then every retained version is read and
/// proven again after all later writes.
fn random_histories<M: AuthMap + Oracle>() {
    for seed in 1..=8_u64 {
        let mut rng = SplitMix::new(seed);
        let mut map = M::new();
        let mut root = map.empty_root();
        let mut records = Records::new();
        let mut known: Vec<Key> = Vec::new();
        let mut versions = vec![(root, records.clone())];
        assert_eq!(root, M::oracle(&records));
        for _ in 0..150 {
            let size = 1 + rng.below(24) as usize;
            let mut ops = Vec::with_capacity(size);
            for _ in 0..size {
                let choice = rng.below(10);
                let key = if known.is_empty() || choice < 4 {
                    let key = rng.key();
                    known.push(key);
                    key
                } else {
                    known[rng.below(known.len() as u64) as usize]
                };
                let len = 1 + rng.below(64) as usize;
                let value = (choice < 8).then(|| rng.value(len));
                ops.push((key, value));
            }
            root = map.apply(root, &ops);
            apply_shadow(&mut records, &ops);
            assert_eq!(
                root,
                M::oracle(&records),
                "{} seed {seed}: parity",
                M::PROFILE
            );
            for _ in 0..4 {
                let key = known[rng.below(known.len() as u64) as usize];
                check_claim(&mut map, root, &records, &key);
                check_claim(&mut map, root, &records, &rng.key());
            }
            versions.push((root, records.clone()));
        }
        for (old_root, old_records) in &versions {
            for key in old_records.keys().take(3) {
                check_claim(&mut map, *old_root, old_records, key);
            }
            check_claim(&mut map, *old_root, old_records, &rng.key());
        }
    }
}

#[test]
fn g1_g2_sparse_random_histories() {
    random_histories::<SparseMap>();
}

#[test]
fn g1_g2_radix_random_histories() {
    random_histories::<RadixMap>();
}

/// One batch, single steps and a different insertion order agree; deleting
/// everything returns the empty root.
fn order_and_batch_independence<M: AuthMap + Oracle>() {
    let mut rng = SplitMix::new(99);
    let ops: Vec<Op> = (0..300).map(|_| (rng.key(), Some(rng.value(16)))).collect();
    let mut batched = M::new();
    let batched_root = batched.apply(batched.empty_root(), &ops);
    let mut stepped = M::new();
    let mut stepped_root = stepped.empty_root();
    for op in ops.iter().rev() {
        stepped_root = stepped.apply(stepped_root, std::slice::from_ref(op));
    }
    assert_eq!(batched_root, stepped_root, "{}", M::PROFILE);
    let deletes: Vec<Op> = ops.iter().map(|(key, _)| (*key, None)).collect();
    let cleared = batched.apply(batched_root, &deletes);
    assert_eq!(cleared, batched.empty_root(), "{}", M::PROFILE);
    let duplicate = vec![
        (ops[0].0, Some(b"first".to_vec())),
        (ops[0].0, Some(b"last".to_vec())),
    ];
    let mut records = Records::new();
    records.insert(ops[0].0, value_digest(b"last"));
    let last_wins = batched.apply(batched.empty_root(), &duplicate);
    assert_eq!(
        last_wins,
        M::oracle(&records),
        "{}: last op wins",
        M::PROFILE
    );
}

#[test]
fn g2_sparse_order_and_batch_independence() {
    order_and_batch_independence::<SparseMap>();
}

#[test]
fn g2_radix_order_and_batch_independence() {
    order_and_batch_independence::<RadixMap>();
}

/// Keys sharing a prefix of every length from 0 to 255 bits: the deepest
/// possible radix trie.
fn comb_keys() -> Vec<Key> {
    (0..256)
        .map(|ones| {
            let mut key = [0_u8; 32];
            for index in 0..ones {
                key[index / 8] |= 0x80 >> (index % 8);
            }
            key
        })
        .collect()
}

fn adversarial_comb<M: AuthMap + Oracle>() {
    let keys = comb_keys();
    let ops: Vec<Op> = keys.iter().map(|key| (*key, Some(key.to_vec()))).collect();
    let mut map = M::new();
    let root = map.apply(map.empty_root(), &ops);
    let mut records = Records::new();
    apply_shadow(&mut records, &ops);
    assert_eq!(root, M::oracle(&records), "{}", M::PROFILE);
    for key in &keys {
        check_claim(&mut map, root, &records, key);
    }
    let mut absent = keys[255];
    absent[31] ^= 0x01;
    check_claim(&mut map, root, &records, &absent);
}

#[test]
fn g1_sparse_adversarial_comb() {
    adversarial_comb::<SparseMap>();
}

#[test]
fn g1_radix_adversarial_comb() {
    adversarial_comb::<RadixMap>();
}

/// G3 (conformance form): every single-byte change, truncation and extension
/// of an honest proof is refused, and no proof verifies for another key.
fn malformed_proofs<M: AuthMap>() {
    let mut rng = SplitMix::new(7);
    let ops: Vec<Op> = (0..64).map(|_| (rng.key(), Some(rng.value(8)))).collect();
    let mut map = M::new();
    let root = map.apply(map.empty_root(), &ops);
    let mut queries: Vec<Key> = ops.iter().take(4).map(|(key, _)| *key).collect();
    queries.push(rng.key());
    let empty_root = map.empty_root();
    for (root, key) in queries
        .iter()
        .map(|key| (root, *key))
        .chain([(empty_root, queries[0])])
    {
        let proof = map.prove(root, &key);
        M::verify(&root, &key, &proof).expect("an honest proof verifies");
        for at in 0..proof.len() {
            for flip in [0x01_u8, 0x80, 0xff] {
                let mut bad = proof.clone();
                bad[at] ^= flip;
                assert!(
                    M::verify(&root, &key, &bad).is_err(),
                    "{}: flip {at}/{flip:#x}",
                    M::PROFILE
                );
            }
        }
        for len in 0..proof.len() {
            assert!(
                M::verify(&root, &key, &proof[..len]).is_err(),
                "{}: truncated",
                M::PROFILE
            );
        }
        let mut longer = proof.clone();
        longer.push(0);
        assert_eq!(
            M::verify(&root, &key, &longer),
            Err(ProofError::Malformed),
            "{}",
            M::PROFILE
        );
        let mut other = key;
        other[0] ^= 0x01;
        // A proof may also serve another key only when its claim is true
        // there: an empty region proves every key in it absent.
        if let Ok(claim) = M::verify(&root, &other, &proof) {
            let truth = map.get(root, &other).map_or(Claim::Absent, Claim::Present);
            assert_eq!(
                claim,
                truth,
                "{}: a false claim verified for another key",
                M::PROFILE
            );
            assert_eq!(
                claim,
                Claim::Absent,
                "{}: a membership proof moved keys",
                M::PROFILE
            );
        }
    }
}

#[test]
fn g3_sparse_malformed_proofs() {
    malformed_proofs::<SparseMap>();
}

#[test]
fn g3_radix_malformed_proofs() {
    malformed_proofs::<RadixMap>();
}

/// G3: no honest proof for one key yields a false claim for another key.
fn no_false_cross_key_claims<M: AuthMap>() {
    let mut rng = SplitMix::new(11);
    let ops: Vec<Op> = (0..48).map(|_| (rng.key(), Some(rng.value(8)))).collect();
    let mut map = M::new();
    let root = map.apply(map.empty_root(), &ops);
    let mut keys: Vec<Key> = ops.iter().map(|(key, _)| *key).collect();
    keys.extend((0..16).map(|_| rng.key()));
    let proofs: Vec<Vec<u8>> = keys.iter().map(|key| map.prove(root, key)).collect();
    for key in &keys {
        let truth = map.get(root, key).map_or(Claim::Absent, Claim::Present);
        for proof in &proofs {
            if let Ok(claim) = M::verify(&root, key, proof) {
                assert_eq!(claim, truth, "{}: a false claim verified", M::PROFILE);
            }
        }
    }
}

#[test]
fn g3_sparse_no_false_cross_key_claims() {
    no_false_cross_key_claims::<SparseMap>();
}

#[test]
fn g3_radix_no_false_cross_key_claims() {
    no_false_cross_key_claims::<RadixMap>();
}

/// The replayed F5 stream is the fixture's: Echo pins this digest in
/// `warp-core/tests/scene_fixture_tests.rs` (`F5_S_SEED7_SCRIPT`).
#[test]
fn f5_script_matches_the_echo_fixture_golden() {
    use echo_authmap_e1::f5;
    let script = f5::script(f5::S, 7);
    assert_eq!(script.len(), 200);
    let hex: String = f5::script_digest(&script)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(
        hex,
        "efc633360e2e237b9ed238e1cb0f57b8c12e64c779a52bba6e223c9bdfd33d4d"
    );
}
