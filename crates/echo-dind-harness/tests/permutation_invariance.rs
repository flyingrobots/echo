// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
use std::path::PathBuf;

use anyhow::Result;
use echo_dind_harness::dind::run_scenario;

#[test]
fn permutation_invariance_050_seeds_produce_identical_full_hash_chains() -> Result<()> {
    // Anchor testdata path to CARGO_MANIFEST_DIR (crate root at compile time).
    let base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../testdata/dind");

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
