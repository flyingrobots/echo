// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! DIND (Deterministic Ironclad Nightmare Drills) scenario runner.
//!
//! This module provides the CLI and core logic for running determinism test scenarios.

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use echo_dind_tests::codecs::SCHEMA_HASH;
use echo_dind_tests::EchoKernel;
use echo_wasm_abi::{read_elog_frame, read_elog_header, ElogHeader};
use std::collections::{BTreeSet, VecDeque};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use warp_core::{make_node_id, make_warp_id, AttachmentValue, GraphStore};

// -----------------------------------------------------------------------------
// Permutation-mode termination constants
// -----------------------------------------------------------------------------

/// Maximum consecutive no-progress steps before declaring a stall.
///
/// In permutation mode, the engine may occasionally report no progress if rules
/// don't match in a given tick. This budget allows transient stalls (e.g., waiting
/// for physics to settle) without aborting prematurely. 64 is generous for typical
/// scenarios while still catching genuine infinite loops within reasonable time.
const PERMUTATION_STALL_BUDGET: u64 = 64;

/// Multiplier for computing max ticks from initial pending count.
///
/// Most scenarios should complete in exactly `pending_count` ticks (one intent per
/// tick). The 4× multiplier provides headroom for multi-phase rules, deferred intents,
/// and physics simulation steps that don't consume intents.
const PERMUTATION_TICK_MULTIPLIER: u64 = 4;

/// Minimum tick ceiling for permutation mode.
///
/// Even if a scenario has few pending intents, we allow at least this many ticks to
/// handle edge cases like physics warm-up or delayed rule matching.
const PERMUTATION_MIN_TICKS: u64 = 256;

/// CLI interface for the DIND harness.
///
/// Parse with `Cli::parse()` and dispatch via `entrypoint()`.
#[derive(Parser)]
#[command(name = "echo-dind")]
#[command(about = "Deterministic Ironclad Nightmare Drills Harness")]
pub struct Cli {
    /// The subcommand to execute.
    #[command(subcommand)]
    pub command: Commands,
}

/// Available DIND subcommands.
#[derive(Subcommand)]
pub enum Commands {
    /// Run a scenario and optionally check against a golden file
    Run {
        /// Path to .eintlog file
        scenario: PathBuf,
        /// Optional path to golden hashes.json
        #[arg(long)]
        golden: Option<PathBuf>,
        /// Optional path to emit reproduction bundle on failure
        #[arg(long)]
        emit_repro: Option<PathBuf>,
    },
    /// Record a scenario and output a golden hashes.json
    Record {
        /// Path to .eintlog file
        scenario: PathBuf,
        /// Path to output hashes.json
        #[arg(long)]
        out: PathBuf,
    },
    /// Run a scenario repeatedly to detect non-determinism
    Torture {
        /// Path to .eintlog file
        scenario: PathBuf,
        /// Number of runs
        #[arg(long, default_value = "20")]
        runs: u32,
        /// Optional path to emit reproduction bundle on failure
        #[arg(long)]
        emit_repro: Option<PathBuf>,
    },
    /// Verify that multiple scenarios converge to the same final state hash
    Converge {
        /// Paths to .eintlog files
        scenarios: Vec<PathBuf>,
        /// Override converge scope (DANGEROUS; use only for ad-hoc debugging)
        #[arg(long)]
        scope: Option<String>,
        /// Required when using --scope to acknowledge non-canonical behavior
        #[arg(long)]
        i_know_what_im_doing: bool,
    },
}

/// Golden file format for DIND scenario verification.
///
/// Contains metadata and the expected sequence of state hashes after each step.
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Golden {
    /// Event log format version (currently 1).
    pub elog_version: u16,
    /// Hex-encoded schema hash identifying the codec version.
    pub schema_hash_hex: String,
    /// Hash domain identifier (e.g., "DIND_STATE_HASH_V2").
    pub hash_domain: String,
    /// Hash algorithm name (e.g., "BLAKE3").
    pub hash_alg: String,
    /// Hex-encoded state hashes, one per step (including initial state at index 0).
    pub hashes_hex: Vec<String>,
}

#[derive(serde::Deserialize)]
struct ManifestEntry {
    path: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    converge_scope: Option<String>,
}

/// Create a reproduction bundle for debugging determinism failures.
///
/// Writes to `out_dir`:
/// - `scenario.eintlog`: Copy of the input scenario
/// - `actual.hashes.json`: The hashes produced by this run
/// - `expected.hashes.json`: The expected golden hashes (if provided)
/// - `diff.txt`: A description of the failure
pub fn create_repro_bundle(
    out_dir: &Path,
    scenario_path: &Path,
    actual_hashes: &[String],
    header: &ElogHeader,
    expected_golden: Option<&Golden>,
    failure_msg: &str,
) -> Result<()> {
    std::fs::create_dir_all(out_dir).context("failed to create repro dir")?;

    // 1. Copy scenario
    std::fs::copy(scenario_path, out_dir.join("scenario.eintlog"))
        .context("failed to copy scenario")?;

    // 2. Write actual hashes
    let actual_golden = Golden {
        elog_version: 1,
        schema_hash_hex: hex::encode(header.schema_hash),
        hash_domain: "DIND_STATE_HASH_V2".to_string(),
        hash_alg: "BLAKE3".to_string(),
        hashes_hex: actual_hashes.to_vec(),
    };
    let f_actual = File::create(out_dir.join("actual.hashes.json"))
        .context("failed to create actual.hashes.json")?;
    serde_json::to_writer_pretty(f_actual, &actual_golden)
        .context("failed to serialize actual.hashes.json")?;

    // 3. Write expected hashes if available
    if let Some(exp) = expected_golden {
        let f_exp = File::create(out_dir.join("expected.hashes.json"))
            .context("failed to create expected.hashes.json")?;
        serde_json::to_writer_pretty(f_exp, exp)
            .context("failed to serialize expected.hashes.json")?;
    }

    // 4. Write diff.txt
    std::fs::write(out_dir.join("diff.txt"), failure_msg).context("failed to write diff.txt")?;

    Ok(())
}

/// Main CLI entrypoint for the DIND harness.
///
/// Parses command-line arguments and dispatches to the appropriate subcommand.
/// Returns an error if the scenario fails validation or encounters an I/O error.
pub fn entrypoint() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            scenario,
            golden,
            emit_repro,
        } => {
            let (hashes, header) = run_scenario(&scenario)?;

            if let Some(golden_path) = golden {
                let f = File::open(&golden_path).context("failed to open golden file")?;
                let expected: Golden =
                    serde_json::from_reader(BufReader::new(f)).with_context(|| {
                        format!("failed to parse golden file: {}", golden_path.display())
                    })?;

                // Validate metadata
                if expected.schema_hash_hex != hex::encode(header.schema_hash) {
                    let msg = format!(
                        "Golden schema hash mismatch. Expected {}, found {}",
                        expected.schema_hash_hex,
                        hex::encode(header.schema_hash)
                    );
                    if let Some(repro_path) = emit_repro {
                        create_repro_bundle(
                            &repro_path,
                            &scenario,
                            &hashes,
                            &header,
                            Some(&expected),
                            &msg,
                        )?;
                        bail!("{}\nRepro bundle emitted to {:?}", msg, repro_path);
                    }
                    bail!("{}", msg);
                }

                // Check length first to avoid silent truncation from zip
                if hashes.len() != expected.hashes_hex.len() {
                    let msg = format!(
                        "Length mismatch. Run has {} steps, golden has {}.",
                        hashes.len(),
                        expected.hashes_hex.len()
                    );
                    if let Some(repro_path) = emit_repro {
                        create_repro_bundle(
                            &repro_path,
                            &scenario,
                            &hashes,
                            &header,
                            Some(&expected),
                            &msg,
                        )?;
                        bail!("{}\nRepro bundle emitted to {:?}", msg, repro_path);
                    }
                    bail!("{}", msg);
                }

                // Compare hashes (length already validated above)
                for (i, (actual, expect)) in
                    hashes.iter().zip(expected.hashes_hex.iter()).enumerate()
                {
                    if actual != expect {
                        let msg = format!(
                            "Hash mismatch at step {}.\nActual:   {}\nExpected: {}",
                            i, actual, expect
                        );
                        if let Some(repro_path) = emit_repro {
                            create_repro_bundle(
                                &repro_path,
                                &scenario,
                                &hashes,
                                &header,
                                Some(&expected),
                                &msg,
                            )?;
                            bail!("{}\nRepro bundle emitted to {:?}", msg, repro_path);
                        }
                        bail!("{}", msg);
                    }
                }

                println!("DIND: OK. {} steps verified.", hashes.len());
            } else {
                println!("DIND: Run complete. {} steps executed.", hashes.len());
            }
        }
        Commands::Record { scenario, out } => {
            let (hashes, header) = run_scenario(&scenario)?;

            let golden = Golden {
                elog_version: 1,
                schema_hash_hex: hex::encode(header.schema_hash),
                hash_domain: "DIND_STATE_HASH_V2".to_string(),
                hash_alg: "BLAKE3".to_string(),
                hashes_hex: hashes,
            };

            let f = File::create(&out).context("failed to create output file")?;
            serde_json::to_writer_pretty(f, &golden)
                .context("failed to serialize golden output")?;
            println!(
                "DIND: Recorded {} steps to {:?}",
                golden.hashes_hex.len(),
                out
            );
        }
        Commands::Torture {
            scenario,
            runs,
            emit_repro,
        } => {
            println!("DIND: Torture starting. {} runs on {:?}", runs, scenario);
            let (baseline_hashes, header) =
                run_scenario(&scenario).context("Run 1 (Baseline) failed")?;

            // Construct a synthetic "Golden" from baseline for reuse in repro
            let baseline_golden = Golden {
                elog_version: 1,
                schema_hash_hex: hex::encode(header.schema_hash),
                hash_domain: "DIND_STATE_HASH_V2".to_string(),
                hash_alg: "BLAKE3".to_string(),
                hashes_hex: baseline_hashes.clone(),
            };

            for i in 2..=runs {
                let (hashes, _) = run_scenario(&scenario).context(format!("Run {} failed", i))?;

                if hashes != baseline_hashes {
                    let mut failure_msg = String::new();
                    // Find first divergence
                    for (step, (base, current)) in
                        baseline_hashes.iter().zip(hashes.iter()).enumerate()
                    {
                        if base != current {
                            failure_msg = format!(
                                "DIND: DIVERGENCE DETECTED in Run {} at Step {}.\nBaseline: {}
Current:  {}",
                                i, step, base, current
                            );
                            break;
                        }
                    }
                    if failure_msg.is_empty() && hashes.len() != baseline_hashes.len() {
                        failure_msg = format!("DIND: DIVERGENCE DETECTED in Run {}. Length mismatch: Baseline {}, Current {}", i, baseline_hashes.len(), hashes.len());
                    }

                    if let Some(repro_path) = emit_repro {
                        create_repro_bundle(
                            &repro_path,
                            &scenario,
                            &hashes,
                            &header,
                            Some(&baseline_golden),
                            &failure_msg,
                        )?;
                        bail!("{}\nRepro bundle emitted to {:?}", failure_msg, repro_path);
                    }
                    bail!("{}", failure_msg);
                }
                // Optional: print progress every 10/100 runs
                if i % 10 == 0 {
                    println!("DIND: {}/{} runs clean...", i, runs);
                }
            }
            println!("DIND: Torture complete. {} runs identical.", runs);
        }
        Commands::Converge {
            scenarios,
            scope,
            i_know_what_im_doing,
        } => {
            if scenarios.is_empty() {
                bail!("No scenarios provided for convergence check.");
            }

            let converge_scope = if let Some(scope) = scope {
                if !i_know_what_im_doing {
                    bail!("--scope requires --i-know-what-im-doing");
                }
                println!(
                    "WARNING: Overriding converge scope; results may not reflect canonical test expectations."
                );
                Some(scope)
            } else {
                resolve_converge_scope(&scenarios)?
            };

            println!(
                "DIND: Checking convergence across {} scenarios...",
                scenarios.len()
            );

            let baseline = &scenarios[0];
            let (hashes, _, kernel) =
                run_scenario_with_kernel(baseline).context("Failed to run baseline")?;
            let baseline_full = hashes.last().cloned().unwrap_or_default();
            let baseline_proj = match &converge_scope {
                Some(scope) => hex::encode(projected_state_hash(&kernel, scope)),
                None => baseline_full.clone(),
            };

            println!(
                "Baseline established from {:?}: {}",
                baseline, baseline_full
            );
            if let Some(scope) = &converge_scope {
                println!("Convergence scope: {}", scope);
                println!("Baseline projected hash: {}", baseline_proj);
            }

            for path in scenarios.iter().skip(1) {
                let (hashes, _, kernel) =
                    run_scenario_with_kernel(path).context(format!("Failed to run {:?}", path))?;
                let full_hash = hashes.last().cloned().unwrap_or_default();
                let projected_hash = match &converge_scope {
                    Some(scope) => hex::encode(projected_state_hash(&kernel, scope)),
                    None => full_hash.clone(),
                };
                if projected_hash != baseline_proj {
                    bail!(
                        "DIND: CONVERGENCE FAILURE.\nBaseline ({:?}): {}\nCurrent  ({:?}): {}",
                        baseline,
                        baseline_proj,
                        path,
                        projected_hash
                    );
                }
                if converge_scope.is_some() {
                    println!("Converged (projected): {:?} => {}", path, projected_hash);
                    if full_hash != baseline_full {
                        println!("  Note: full hash differs (expected for commutative scenarios).");
                    }
                }
            }
            println!("DIND: CONVERGENCE OK. Projected hashes identical across all scenarios.");
        }
    }

    Ok(())
}

fn probe_interop(kernel: &EchoKernel) {
    let ball_id = make_node_id("ball");
    if let Ok(Some(AttachmentValue::Atom(atom))) = kernel.engine().node_attachment(&ball_id) {
        // This exercises payload.rs, fixed_q32_32.rs, and scalar.rs via the canonical decoder
        let _ = warp_core::decode_motion_atom_payload(atom);
    }
}

/// Run a DIND scenario and return the sequence of state hashes.
///
/// # Arguments
/// * `path` - Path to the `.eintlog` scenario file
///
/// # Returns
/// A tuple of (state hashes as hex strings, event log header).
pub fn run_scenario(path: &Path) -> Result<(Vec<String>, ElogHeader)> {
    let (hashes, header, _) = run_scenario_with_kernel(path)?;
    Ok((hashes, header))
}

/// Run a DIND scenario and return both hashes and the kernel state.
///
/// Like [`run_scenario`], but also returns the kernel for further inspection.
pub fn run_scenario_with_kernel(path: &Path) -> Result<(Vec<String>, ElogHeader, EchoKernel)> {
    let f = File::open(path).context("failed to open scenario file")?;
    let mut r = BufReader::new(f);

    let header = read_elog_header(&mut r)?;
    let kernel_schema_hash = hex::decode(SCHEMA_HASH).context("invalid kernel schema hash hex")?;

    if header.schema_hash.as_slice() != kernel_schema_hash.as_slice() {
        bail!(
            "Scenario schema hash mismatch.\nScenario: {}
Kernel:   {}",
            hex::encode(header.schema_hash),
            SCHEMA_HASH
        );
    }

    let mut kernel = EchoKernel::new();
    let mut hashes = Vec::new();

    // Initial state hash (step 0)
    hashes.push(hex::encode(kernel.state_hash()));

    let ingest_all_first = scenario_requires_ingest_all_first(path)?;

    if ingest_all_first {
        // Permutation-invariance mode: ingest all frames first, then step until the pending set is empty.
        //
        // This enforces identical tick membership across permutations (same set → same full graph).
        let mut frame_count: u64 = 0;
        while let Some(frame) = read_elog_frame(&mut r)? {
            kernel.dispatch_intent(&frame);
            frame_count += 1;
        }

        let mut ticks: u64 = 0;
        let mut stall_budget: u64 = PERMUTATION_STALL_BUDGET;
        let mut last_pending = kernel
            .engine()
            .pending_intent_count()
            .context("pending_intent_count failed")? as u64;
        let max_ticks: u64 = last_pending
            .saturating_mul(PERMUTATION_TICK_MULTIPLIER)
            .max(PERMUTATION_MIN_TICKS);

        while kernel
            .engine()
            .pending_intent_count()
            .context("pending_intent_count failed")?
            > 0
        {
            if ticks >= max_ticks {
                bail!(
                    "Permutation-mode hard stop: exceeded max_ticks={max_ticks} (frames={frame_count}, pending_initial={last_pending})"
                );
            }

            let progressed = kernel.step();
            probe_interop(&kernel);
            if !progressed {
                stall_budget = stall_budget.saturating_sub(1);
                if stall_budget == 0 {
                    bail!(
                        "Permutation-mode hard stop: no progress budget exhausted (ticks={ticks}, pending={last_pending})"
                    );
                }
                continue;
            }

            ticks += 1;
            hashes.push(hex::encode(kernel.state_hash()));

            let pending_now = kernel
                .engine()
                .pending_intent_count()
                .context("pending_intent_count failed")? as u64;
            if pending_now < last_pending {
                last_pending = pending_now;
                stall_budget = PERMUTATION_STALL_BUDGET;
            } else {
                stall_budget = stall_budget.saturating_sub(1);
                if stall_budget == 0 {
                    bail!(
                        "Permutation-mode hard stop: pending set not shrinking (ticks={ticks}, pending={pending_now})"
                    );
                }
            }
        }
    } else {
        while let Some(frame) = read_elog_frame(&mut r)? {
            kernel.dispatch_intent(&frame);
            kernel.step(); // Budget? Just run it.
            probe_interop(&kernel); // Exercise boundary code
            hashes.push(hex::encode(kernel.state_hash()));
        }
    }

    Ok((hashes, header, kernel))
}

fn scenario_requires_ingest_all_first(scenario: &Path) -> Result<bool> {
    let filename = scenario.file_name().and_then(|s| s.to_str()).unwrap_or("");
    // Fast-path for the current permutation-invariance suite.
    if filename.starts_with("050_") {
        return Ok(true);
    }

    let Some(manifest_path) = manifest_path_for_scenario(scenario) else {
        return Ok(false);
    };
    let f = File::open(&manifest_path).context("failed to open MANIFEST.json")?;
    let entries: Vec<ManifestEntry> = serde_json::from_reader(BufReader::new(f))
        .with_context(|| format!("failed to parse MANIFEST.json: {}", manifest_path.display()))?;
    let Some(entry) = entries.into_iter().find(|e| e.path == filename) else {
        return Ok(false);
    };

    Ok(entry
        .tags
        .iter()
        .any(|t| t == "ingest-all-first" || t == "permute-invariant" || t == "order-sensitive"))
}

fn resolve_converge_scope(scenarios: &[PathBuf]) -> Result<Option<String>> {
    let mut scope: Option<String> = None;
    let mut missing = Vec::new();

    for scenario in scenarios {
        let Some(manifest_path) = manifest_path_for_scenario(scenario) else {
            missing.push(scenario.clone());
            continue;
        };
        let Some(entry_scope) = find_manifest_scope(&manifest_path, scenario)? else {
            missing.push(scenario.clone());
            continue;
        };

        match &scope {
            None => scope = Some(entry_scope),
            Some(existing) => {
                if existing != &entry_scope {
                    bail!(
                        "Converge scope mismatch: '{}' vs '{}'",
                        existing,
                        entry_scope
                    );
                }
            }
        }
    }

    if scope.is_none() {
        return Ok(None);
    }
    if !missing.is_empty() {
        bail!("Converge scope missing for scenarios: {:?}", missing);
    }
    Ok(scope)
}

fn manifest_path_for_scenario(scenario: &Path) -> Option<PathBuf> {
    let parent = scenario.parent()?;
    let manifest = parent.join("MANIFEST.json");
    if manifest.exists() {
        Some(manifest)
    } else {
        None
    }
}

fn find_manifest_scope(manifest_path: &Path, scenario: &Path) -> Result<Option<String>> {
    let f = File::open(manifest_path).context("failed to open MANIFEST.json")?;
    let entries: Vec<ManifestEntry> = serde_json::from_reader(BufReader::new(f))
        .with_context(|| format!("failed to parse MANIFEST.json: {}", manifest_path.display()))?;
    let filename = scenario.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let entry = entries.into_iter().find(|e| e.path == filename);
    Ok(entry.and_then(|e| e.converge_scope))
}

fn projected_state_hash(kernel: &EchoKernel, scope: &str) -> [u8; 32] {
    let root_id = make_node_id(scope);
    let Some(store) = kernel.engine().state().store(&make_warp_id("root")) else {
        return [0u8; 32];
    };
    subgraph_hash(store, root_id)
}

fn subgraph_hash(store: &GraphStore, root: warp_core::NodeId) -> [u8; 32] {
    if store.node(&root).is_none() {
        return GraphStore::new(store.warp_id()).canonical_state_hash();
    }

    let mut nodes = BTreeSet::new();
    let mut queue = VecDeque::new();
    let mut edges = Vec::new();

    nodes.insert(root);
    queue.push_back(root);

    while let Some(node_id) = queue.pop_front() {
        for edge in store.edges_from(&node_id) {
            edges.push(edge.clone());
            if nodes.insert(edge.to) {
                queue.push_back(edge.to);
            }
        }
    }

    let mut sub = GraphStore::new(store.warp_id());

    for node_id in &nodes {
        if let Some(record) = store.node(node_id) {
            sub.insert_node(*node_id, record.clone());
            if let Some(att) = store.node_attachment(node_id) {
                sub.set_node_attachment(*node_id, Some(att.clone()));
            }
        }
    }

    for edge in edges {
        if nodes.contains(&edge.from) && nodes.contains(&edge.to) {
            sub.insert_edge(edge.from, edge.clone());
            if let Some(att) = store.edge_attachment(&edge.id) {
                sub.set_edge_attachment(edge.id, Some(att.clone()));
            }
        }
    }

    sub.canonical_state_hash()
}
