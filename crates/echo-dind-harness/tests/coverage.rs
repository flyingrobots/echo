// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! DIND scenario coverage tests.

use anyhow::Result;
use echo_dind_harness::dind::{run_scenario, ELOG_VERSION, HASH_ALG, HASH_DOMAIN};
use echo_dind_tests::SCHEMA_HASH;
use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

#[derive(Deserialize)]
struct ManifestEntry {
    path: String,
    // we might use tags later for filtering
    #[allow(dead_code)]
    tags: Vec<String>,
    #[allow(dead_code)]
    desc: String,
}

#[test]
fn test_dind_coverage() -> Result<()> {
    // Locate manifest relative to the crate or workspace root
    // We assume running from workspace root or crate root.
    // Let's try to find testdata/dind/MANIFEST.json

    let manifest_path = PathBuf::from("../../testdata/dind/MANIFEST.json");
    if !manifest_path.exists() {
        // Fallback if running from workspace root
        if PathBuf::from("testdata/dind/MANIFEST.json").exists() {
            return run_suite(PathBuf::from("testdata/dind/MANIFEST.json"));
        }
        panic!("Could not find MANIFEST.json at {:?}", manifest_path);
    }

    run_suite(manifest_path)
}

/// Check if env var is set to a truthy value ("1", "true", "yes").
fn is_env_truthy(var: &str) -> bool {
    std::env::var(var)
        .map(|v| matches!(v.trim().to_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(false)
}

fn run_suite(manifest_path: PathBuf) -> Result<()> {
    let f = File::open(&manifest_path)?;
    let manifest: Vec<ManifestEntry> = serde_json::from_reader(BufReader::new(f))?;

    let base_dir = manifest_path.parent().unwrap();
    let update_golden = is_env_truthy("DIND_UPDATE_GOLDEN");

    for entry in manifest {
        let scenario_path = base_dir.join(&entry.path);
        eprintln!("Coverage running: {:?}", scenario_path);

        let (hashes, _) = run_scenario(&scenario_path)?;

        // Basic assertion that we got hashes
        assert!(!hashes.is_empty(), "Scenario produced no hashes");

        // Check if there is a golden file to verify against
        let golden_path = base_dir.join(entry.path.replace(".eintlog", ".hashes.json"));

        if update_golden {
            // Update mode: write new golden file
            let golden = echo_dind_harness::dind::Golden {
                elog_version: ELOG_VERSION,
                schema_hash_hex: SCHEMA_HASH.to_string(),
                hash_domain: HASH_DOMAIN.to_string(),
                hash_alg: HASH_ALG.to_string(),
                hashes_hex: hashes.clone(),
            };
            let mut f_out = std::fs::File::create(&golden_path)?;
            serde_json::to_writer_pretty(&mut f_out, &golden)?;
            f_out.sync_all()?;
            eprintln!("Updated: {:?}", golden_path);
        } else if golden_path.exists() {
            let f_golden = File::open(&golden_path)?;
            let expected: echo_dind_harness::dind::Golden =
                serde_json::from_reader(BufReader::new(f_golden))?;
            assert_eq!(
                hashes, expected.hashes_hex,
                "Hash mismatch for {}",
                entry.path
            );
        }
    }

    Ok(())
}
