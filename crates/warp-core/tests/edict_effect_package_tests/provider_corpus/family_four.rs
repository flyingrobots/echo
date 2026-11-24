// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Family 4 under the shared-read schedule (echo#1016): `Option`
//! constructors, ordered records whose fields `fail`, and `fail` under
//! `attempt`.
//!
//! Each program runs, given exactly the lowerer's shared minimum as its Core
//! budget, to the copying executor's result on the same input and graph (a
//! differential), and its runtime charge stays within that static bound on
//! both axes. The minimums are the ones
//! `family_four_minimums_under_each_schedule` in `echo-edict-provider-lowerer`
//! measures and pins, and the verifier reaches independently.
use super::identity_witness::{run as run_witness, Witness};
use super::shared_schedule::shared;
use super::*;
use warp_core::{GraphStore, NodeId, NodeRecord, TypeId, WarpId};

const OPTION_WARP: WarpId = WarpId([0x41; 32]);
const OPTION_ROOT: NodeId = NodeId([0x42; 32]);

fn option_store() -> GraphStore {
    let mut store = GraphStore::new(OPTION_WARP);
    store.insert_node(
        OPTION_ROOT,
        NodeRecord {
            ty: TypeId([0x43; 32]),
        },
    );
    store
}

fn option_input(start: u64, end: u64) -> Vec<u8> {
    encoded(&map([
        ("anchor", V::Bytes(vec![0x5a; 32])),
        ("start", V::Integer(i128::from(start))),
        ("end", V::Integer(i128::from(end))),
    ]))
}

/// The public option witness on both of its helper's branches: `some(v)`
/// when `end > start`, `none()` otherwise.
const OPTION_WITNESSES: [(&str, Witness); 2] = [
    (
        "option: some",
        Witness {
            package: include_str!(
                "../../fixtures/edict-option-value/option-executable-operation-package.hex"
            ),
            warp: OPTION_WARP,
            store: option_store,
            root: OPTION_ROOT,
            input: || option_input(3, 9),
            reads: &[],
            minimum: (828, 942),
        },
    ),
    (
        "option: none",
        Witness {
            package: include_str!(
                "../../fixtures/edict-option-value/option-executable-operation-package.hex"
            ),
            warp: OPTION_WARP,
            store: option_store,
            root: OPTION_ROOT,
            input: || option_input(9, 3),
            reads: &[],
            minimum: (828, 942),
        },
    ),
];

#[test]
fn option_constructors_run_within_the_lowerer_minimum() {
    for (name, witness) in &OPTION_WITNESSES {
        let package = echo_edict_canonical::decode_canonical_cbor_v1(
            &hex::decode(witness.package.trim()).expect("hex"),
        )
        .expect("canonical package");
        let (copying, _, _) = run_witness(witness, &package);
        let expected = copying.unwrap_or_else(|error| panic!("{name}: copying: {error:?}"));
        let (steps, bytes) = witness.minimum;
        let exact = with_allocation(&with_budget(&shared(&package), steps), bytes);
        let (sharing, charged_steps, charged_bytes) = run_witness(witness, &exact);
        assert_eq!(
            sharing.as_deref(),
            Ok(expected.as_slice()),
            "{name}: result"
        );
        assert!(
            charged_steps <= steps && charged_bytes <= bytes,
            "{name}: runtime charge ({charged_steps}, {charged_bytes}) exceeds the static bound ({steps}, {bytes})"
        );
    }
}

/// The lowerer's shared minimums for the retained ordered-record programs.
const ORDERED_MINIMUMS: [(&str, u64, u64); 4] = [
    ("ordered-record-z-first", 110, 1489),
    ("ordered-record-a-first", 110, 1489),
    ("ordered-record-guarded-z-first", 250, 1745),
    ("ordered-record-guarded-a-first", 250, 1745),
];

/// Every `at` the guards distinguish: both fields fail, one fails, neither.
#[test]
fn ordered_records_fail_and_return_within_the_lowerer_minimum() {
    for (name, steps, bytes) in ORDERED_MINIMUMS {
        let package = retained(name);
        let exact = with_allocation(&with_budget(&shared(&package), steps), bytes);
        for at in 0..=4 {
            let input = ordered_record_input(at);
            let frame = Frame::widest(None);
            let copying = run_on(&package, &input, &frame);
            assert_eq!(run_on(&exact, &input, &frame), copying, "{name} at {at}");
        }
    }
}
