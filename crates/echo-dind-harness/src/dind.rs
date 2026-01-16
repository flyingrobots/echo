use std::collections::{BTreeSet, VecDeque};
use std::path::PathBuf;
use std::fs::File;
use std::io::BufReader;
use clap::{Parser, Subcommand};
use anyhow::{Context, Result, bail};
use echo_wasm_abi::{read_elog_header, read_elog_frame, ElogHeader};
use echo_dind_tests::EchoKernel;
use echo_dind_tests::generated::codecs::SCHEMA_HASH;
use warp_core::{make_node_id, make_warp_id, AttachmentValue, GraphStore};

#[derive(Parser)]
#[command(name = "echo-dind")]
#[command(about = "Deterministic Ironclad Nightmare Drills Harness")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

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
    },
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Golden {
    pub elog_version: u16,
    pub schema_hash_hex: String,
    pub hash_domain: String,
    pub hash_alg: String,
    pub hashes_hex: Vec<String>,
}

#[derive(serde::Deserialize)]
struct ManifestEntry {
    path: String,
    #[serde(default)]
    converge_scope: Option<String>,
}

pub fn create_repro_bundle(
    out_dir: &PathBuf,
    scenario_path: &PathBuf,
    actual_hashes: &[String],
    header: &ElogHeader,
    expected_golden: Option<&Golden>,
    failure_msg: &str,
) -> Result<()> {
    std::fs::create_dir_all(out_dir).context("failed to create repro dir")?;
    
    // 1. Copy scenario
    std::fs::copy(scenario_path, out_dir.join("scenario.eintlog")).context("failed to copy scenario")?;
    
    // 2. Write actual hashes
    let actual_golden = Golden {
        elog_version: 1,
        schema_hash_hex: hex::encode(header.schema_hash),
        hash_domain: "DIND_STATE_HASH_V1".to_string(),
        hash_alg: "BLAKE3".to_string(),
        hashes_hex: actual_hashes.to_vec(),
    };
    let f_actual = File::create(out_dir.join("actual.hashes.json")).context("failed to create actual.hashes.json")?;
    serde_json::to_writer_pretty(f_actual, &actual_golden)?;
    
    // 3. Write expected hashes if available
    if let Some(exp) = expected_golden {
        let f_exp = File::create(out_dir.join("expected.hashes.json")).context("failed to create expected.hashes.json")?;
        serde_json::to_writer_pretty(f_exp, exp)?;
    }
    
    // 4. Write diff.txt
    std::fs::write(out_dir.join("diff.txt"), failure_msg).context("failed to write diff.txt")?;
    
    Ok(())
}

pub fn entrypoint() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { scenario, golden, emit_repro } => {
            let (hashes, header) = run_scenario(&scenario)?;
            
            if let Some(golden_path) = golden {
                let f = File::open(&golden_path).context("failed to open golden file")?;
                let expected: Golden = serde_json::from_reader(BufReader::new(f))?;
                
                // Validate metadata
                if expected.schema_hash_hex != hex::encode(header.schema_hash) {
                    let msg = format!("Golden schema hash mismatch. Expected {}, found {}", expected.schema_hash_hex, hex::encode(header.schema_hash));
                    if let Some(repro_path) = emit_repro {
                        create_repro_bundle(&repro_path, &scenario, &hashes, &header, Some(&expected), &msg)?;
                        bail!("{}\nRepro bundle emitted to {:?}", msg, repro_path);
                    }
                    bail!("{}", msg);
                }
                
                // Compare hashes
                for (i, (actual, expect)) in hashes.iter().zip(expected.hashes_hex.iter()).enumerate() {
                    if actual != expect {
                        let msg = format!("Hash mismatch at step {}.\nActual:   {}
Expected: {}", i, actual, expect);
                        if let Some(repro_path) = emit_repro {
                            create_repro_bundle(&repro_path, &scenario, &hashes, &header, Some(&expected), &msg)?;
                            bail!("{}\nRepro bundle emitted to {:?}", msg, repro_path);
                        }
                        bail!("{}", msg);
                    }
                }
                
                if hashes.len() != expected.hashes_hex.len() {
                    let msg = format!("Length mismatch. Run has {} steps, golden has {}.", hashes.len(), expected.hashes_hex.len());
                    if let Some(repro_path) = emit_repro {
                        create_repro_bundle(&repro_path, &scenario, &hashes, &header, Some(&expected), &msg)?;
                        bail!("{}\nRepro bundle emitted to {:?}", msg, repro_path);
                    }
                    bail!("{}", msg);
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
                hash_domain: "DIND_STATE_HASH_V1".to_string(),
                hash_alg: "BLAKE3".to_string(),
                hashes_hex: hashes,
            };
            
            let f = File::create(&out).context("failed to create output file")?;
            serde_json::to_writer_pretty(f, &golden)?;
            println!("DIND: Recorded {} steps to {:?}", golden.hashes_hex.len(), out);
        }
        Commands::Torture { scenario, runs, emit_repro } => {
            println!("DIND: Torture starting. {} runs on {:?}", runs, scenario);
            let (baseline_hashes, header) = run_scenario(&scenario).context("Run 1 (Baseline) failed")?;
            
            // Construct a synthetic "Golden" from baseline for reuse in repro
            let baseline_golden = Golden {
                elog_version: 1,
                schema_hash_hex: hex::encode(header.schema_hash),
                hash_domain: "DIND_STATE_HASH_V1".to_string(),
                hash_alg: "BLAKE3".to_string(),
                hashes_hex: baseline_hashes.clone(),
            };

            for i in 2..=runs {
                let (hashes, _) = run_scenario(&scenario).context(format!("Run {} failed", i))?;
                
                if hashes != baseline_hashes {
                    let mut failure_msg = String::new();
                    // Find first divergence
                    for (step, (base, current)) in baseline_hashes.iter().zip(hashes.iter()).enumerate() {
                        if base != current {
                            failure_msg = format!("DIND: DIVERGENCE DETECTED in Run {} at Step {}.\nBaseline: {}
Current:  {}", i, step, base, current);
                            break;
                        }
                    }
                    if failure_msg.is_empty() && hashes.len() != baseline_hashes.len() {
                         failure_msg = format!("DIND: DIVERGENCE DETECTED in Run {}. Length mismatch: Baseline {}, Current {}", i, baseline_hashes.len(), hashes.len());
                    }

                    if let Some(repro_path) = emit_repro {
                        create_repro_bundle(&repro_path, &scenario, &hashes, &header, Some(&baseline_golden), &failure_msg)?;
                        bail!("{}\nRepro bundle emitted to {:?}", failure_msg, repro_path);
                    }
                    bail!("{}", failure_msg);
                }
                // Optional: print progress every 10/100 runs
                if i % 10 == 0 { println!("DIND: {}/{} runs clean...", i, runs); }
            }
            println!("DIND: Torture complete. {} runs identical.", runs);
        }
        Commands::Converge { scenarios } => {
            if scenarios.is_empty() {
                bail!("No scenarios provided for convergence check.");
            }

            let converge_scope = resolve_converge_scope(&scenarios)?;

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

            println!("Baseline established from {:?}: {}", baseline, baseline_full);
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
            println!(
                "DIND: CONVERGENCE OK. Projected hashes identical across all scenarios."
            );
        }
    }

    Ok(())
}

fn probe_interop(kernel: &EchoKernel) {
    let ball_id = make_node_id("ball");
    if let Ok(Some(AttachmentValue::Atom(atom))) = kernel.engine().node_attachment(&ball_id) {
        // This exercises payload.rs, fixed_q32_32.rs, and scalar.rs via the canonical decoder
        let _ = warp_core::decode_motion_atom_payload(&atom);
    } else {
        // println!("Probe: Ball not found yet.");
    }
}

pub fn run_scenario(path: &PathBuf) -> Result<(Vec<String>, ElogHeader)> {
    let (hashes, header, _) = run_scenario_with_kernel(path)?;
    Ok((hashes, header))
}

pub fn run_scenario_with_kernel(path: &PathBuf) -> Result<(Vec<String>, ElogHeader, EchoKernel)> {
    let f = File::open(path).context("failed to open scenario file")?;
    let mut r = BufReader::new(f);
    
    let header = read_elog_header(&mut r)?;
    let kernel_schema_hash = hex::decode(SCHEMA_HASH).context("invalid kernel schema hash hex")?;
    
    if header.schema_hash.as_slice() != kernel_schema_hash.as_slice() {
        bail!("Scenario schema hash mismatch.\nScenario: {}
Kernel:   {}", hex::encode(header.schema_hash), SCHEMA_HASH);
    }

    let mut kernel = EchoKernel::new();
    let mut hashes = Vec::new();
    
    // Initial state hash (step 0)
    hashes.push(hex::encode(kernel.state_hash()));

    while let Some(frame) = read_elog_frame(&mut r)? {
        kernel.dispatch_intent(&frame);
        kernel.step(1000); // Budget? Just run it.
        probe_interop(&kernel); // Exercise boundary code
        hashes.push(hex::encode(kernel.state_hash()));
    }
    
    Ok((hashes, header, kernel))
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
                    bail!("Converge scope mismatch: '{}' vs '{}'", existing, entry_scope);
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

fn manifest_path_for_scenario(scenario: &PathBuf) -> Option<PathBuf> {
    let parent = scenario.parent()?;
    let manifest = parent.join("MANIFEST.json");
    if manifest.exists() {
        Some(manifest)
    } else {
        None
    }
}

fn find_manifest_scope(manifest_path: &PathBuf, scenario: &PathBuf) -> Result<Option<String>> {
    let f = File::open(manifest_path).context("failed to open MANIFEST.json")?;
    let entries: Vec<ManifestEntry> = serde_json::from_reader(BufReader::new(f))?;
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
