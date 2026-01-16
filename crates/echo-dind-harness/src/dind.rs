use std::path::PathBuf;
use std::fs::File;
use std::io::BufReader;
use clap::{Parser, Subcommand};
use anyhow::{Context, Result, bail};
use echo_wasm_abi::{read_elog_header, read_elog_frame, ElogHeader};
use echo_dind_tests::EchoKernel;
use echo_dind_tests::generated::codecs::SCHEMA_HASH;
use warp_core::{make_node_id, AttachmentValue};

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
        Commands::Converge {
            scenarios,
        } => {
            if scenarios.is_empty() {
                bail!("No scenarios provided for convergence check.");
            }
            
            println!("DIND: Checking convergence across {} scenarios...", scenarios.len());
            
            let mut baseline_hash: Option<String> = None;
            let mut baseline_path: Option<PathBuf> = None;

            for path in &scenarios {
                let (hashes, _) = run_scenario(path).context(format!("Failed to run {:?}", path))?;
                let final_hash = hashes.last().context("Scenario produced no hashes")?;
                
                match &baseline_hash {
                    None => {
                        baseline_hash = Some(final_hash.clone());
                        baseline_path = Some(path.clone());
                        println!("Baseline established from {:?}: {}", path, final_hash);
                    }
                    Some(expected) => {
                        if final_hash != expected {
                            bail!("DIND: CONVERGENCE FAILURE.\nBaseline ({:?}): {}
Current  ({:?}): {}", 
                                  baseline_path.as_ref().unwrap(), expected, path, final_hash);
                        }
                    }
                }
            }
            println!("DIND: Convergence verified. All {} scenarios end in state {}.", scenarios.len(), baseline_hash.unwrap());
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
    
    Ok((hashes, header))
}
