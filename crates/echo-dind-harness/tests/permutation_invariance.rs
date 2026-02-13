// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Tests for permutation invariance of the deterministic kernel.

use std::path::PathBuf;

use anyhow::Result;
use echo_dind_harness::dind::run_scenario;

#[test]
fn permutation_invariance_050_seeds_produce_identical_full_hash_chains() -> Result<()> {
    // Anchor testdata path to CARGO_MANIFEST_DIR (crate root at compile time).
    let base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/dind");

    let scenarios = [
        "050_randomized_order_small_seed0001.eintlog",
        "050_randomized_order_small_seed0002.eintlog",
        "050_randomized_order_small_seed0003.eintlog",
    ];

    let mut baseline: Option<Vec<String>> = None;
    for name in scenarios {
        let path = base_dir.join(name);
        let (hashes, _header) = run_scenario(&path)?;
        match &baseline {
            None => baseline = Some(hashes),
            Some(base) => assert_eq!(
                &hashes, base,
                "expected permutation-invariant full hash chain for {:?}",
                path
            ),
        }
    }

    Ok(())
}

#[test]
fn convergence_051_seeds_produce_identical_final_hash() -> Result<()> {
    // Anchor testdata path to CARGO_MANIFEST_DIR (crate root at compile time).
    let base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/dind");

    let scenarios = [
        "051_randomized_convergent_seed0001.eintlog",
        "051_randomized_convergent_seed0002.eintlog",
        "051_randomized_convergent_seed0003.eintlog",
    ];

    let mut baseline_final_hash: Option<String> = None;
    for name in scenarios {
        let path = base_dir.join(name);
        let (hashes, _header) = run_scenario(&path)?;
        let final_hash = hashes.last().expect("scenario must have at least one hash");

        match &baseline_final_hash {
            None => baseline_final_hash = Some(final_hash.clone()),
            Some(base) => assert_eq!(
                final_hash, base,
                "expected convergent final hash for {:?}",
                path
            ),
        }
    }

    Ok(())
}
