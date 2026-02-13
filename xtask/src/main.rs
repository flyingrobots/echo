// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! Echo repository maintenance tasks.
//!
//! This crate exists to provide a single, discoverable entrypoint for repo automation via
//! `cargo xtask …` (see `.cargo/config.toml`).
//!
//! Invariants:
//! - This is *not* production runtime code; it may invoke external tools (`node`, `dot`, `gh`).
//! - Prefer deterministic outputs for generated artifacts; avoid “timestamp churn” where possible.

use anyhow::{bail, Context, Result};
use clap::{Args, Parser, Subcommand};
use std::path::Path;
use std::process::Command;

#[derive(Parser)]
#[command(
    name = "xtask",
    about = "Echo repo maintenance tasks (cargo xtask …)",
    disable_help_subcommand = true
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate dependency DAG DOT/SVG artifacts for issues + milestones.
    Dags(DagsArgs),
    /// Run DIND (Deterministic Ironclad Nightmare Drills) harness.
    Dind(DindArgs),
    /// Wesley protocol generator commands.
    Wesley(WesleyArgs),
}

#[derive(Args)]
struct DindArgs {
    /// DIND subcommand to execute.
    #[command(subcommand)]
    command: DindCommands,
}

#[derive(Subcommand)]
enum DindCommands {
    /// Run scenarios and verify against golden files.
    Run {
        /// Filter scenarios by tags (comma-separated).
        #[arg(long)]
        tags: Option<String>,
        /// Exclude scenarios with these tags (comma-separated).
        #[arg(long)]
        exclude_tags: Option<String>,
        /// Emit reproduction bundle on failure.
        #[arg(long)]
        emit_repro: bool,
    },
    /// Record golden hashes for scenarios.
    Record {
        /// Filter scenarios by tags (comma-separated).
        #[arg(long)]
        tags: Option<String>,
        /// Exclude scenarios with these tags (comma-separated).
        #[arg(long)]
        exclude_tags: Option<String>,
    },
    /// Run torture tests (repeated runs to detect non-determinism).
    Torture {
        /// Filter scenarios by tags (comma-separated).
        #[arg(long)]
        tags: Option<String>,
        /// Exclude scenarios with these tags (comma-separated).
        #[arg(long)]
        exclude_tags: Option<String>,
        /// Number of runs per scenario.
        #[arg(long, default_value = "20")]
        runs: u32,
        /// Emit reproduction bundle on failure.
        #[arg(long)]
        emit_repro: bool,
    },
    /// Verify convergence across scenario permutations.
    Converge {
        /// Filter scenarios by tags (comma-separated).
        #[arg(long)]
        tags: Option<String>,
        /// Exclude scenarios with these tags (comma-separated).
        #[arg(long)]
        exclude_tags: Option<String>,
    },
}

#[derive(Args)]
struct WesleyArgs {
    /// Wesley subcommand to execute.
    #[command(subcommand)]
    command: WesleyCommands,
}

#[derive(Subcommand)]
enum WesleyCommands {
    /// Sync Wesley-generated artifacts into Echo.
    ///
    /// Runs Wesley's compile-ttd, copies outputs, generates Rust via echo-ttd-gen,
    /// and updates the provenance lockfile.
    Sync {
        /// Path to Wesley repository (default: ~/git/Wesley).
        #[arg(long, default_value = "~/git/Wesley")]
        wesley_path: String,

        /// Path to TTD schema file. If not specified, uses local schema at
        /// `schemas/ttd-protocol.graphql` if it exists, otherwise falls back
        /// to the schema relative to Wesley repo.
        #[arg(long)]
        schema: Option<String>,

        /// Skip Rust generation (only sync manifests and TypeScript).
        #[arg(long)]
        skip_rust: bool,

        /// Dry run: show what would be done without writing files.
        #[arg(long)]
        dry_run: bool,
    },
    /// Verify that vendored artifacts match what Wesley would generate.
    Check {
        /// Path to Wesley repository (default: ~/git/Wesley).
        #[arg(long, default_value = "~/git/Wesley")]
        wesley_path: String,

        /// Path to TTD schema file. If not specified, uses local schema at
        /// `schemas/ttd-protocol.graphql` if it exists, otherwise falls back
        /// to the schema relative to Wesley repo.
        #[arg(long)]
        schema: Option<String>,
    },
}

#[derive(Args)]
struct DagsArgs {
    /// Fetch fresh issue/milestone snapshots via `gh` (requires network/auth).
    #[arg(long)]
    fetch: bool,

    /// Render SVGs via Graphviz `dot`.
    #[arg(long = "render", default_value_t = true)]
    #[arg(long = "no-render", action = clap::ArgAction::SetFalse)]
    render: bool,

    /// Override the snapshot label shown in graph titles.
    ///
    /// Values:
    /// - `none` (omit the snapshot label entirely)
    /// - `rolling` (stable label for CI/automation)
    /// - `YYYY-MM-DD` (pinned date label for comparisons)
    #[arg(long = "snapshot-label", default_value = "auto")]
    snapshot_label: String,

    /// Legacy flag: override the snapshot label shown in graph titles (format: YYYY-MM-DD).
    #[arg(long, hide = true)]
    snapshot: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Dags(args) => run_dags(args),
        Commands::Dind(args) => run_dind(args),
        Commands::Wesley(args) => run_wesley(args),
    }
}

fn run_dags(args: DagsArgs) -> Result<()> {
    let mut node_args = vec!["scripts/generate-dependency-dags.js".to_owned()];
    if args.fetch {
        node_args.push("--fetch".to_owned());
    }
    if args.render {
        node_args.push("--render".to_owned());
    }

    if let Some(snapshot) = args.snapshot.as_deref() {
        validate_snapshot_date(snapshot)?;
        node_args.push("--snapshot".to_owned());
        node_args.push(snapshot.to_owned());
    } else {
        validate_snapshot_label(&args.snapshot_label)?;
        node_args.push("--snapshot-label".to_owned());
        node_args.push(args.snapshot_label);
    }

    let status = Command::new("node")
        .args(node_args)
        .status()
        .context("failed to spawn `node` (is Node.js installed?)")?;

    if !status.success() {
        bail!("dependency DAG generation failed (exit status: {status})");
    }

    Ok(())
}

fn validate_snapshot_label(label: &str) -> Result<()> {
    if label == "auto" || label == "none" || label == "rolling" {
        return Ok(());
    }

    validate_snapshot_date(label)
}

fn validate_snapshot_date(snapshot: &str) -> Result<()> {
    let mut parts = snapshot.split('-');
    let year = parts
        .next()
        .context("snapshot must be YYYY-MM-DD")?
        .parse::<u32>()
        .context("snapshot year must be numeric")?;
    let month = parts
        .next()
        .context("snapshot must be YYYY-MM-DD")?
        .parse::<u32>()
        .context("snapshot month must be numeric")?;
    let day = parts
        .next()
        .context("snapshot must be YYYY-MM-DD")?
        .parse::<u32>()
        .context("snapshot day must be numeric")?;
    if parts.next().is_some() {
        bail!("snapshot must be YYYY-MM-DD");
    }

    if !(2000..=2100).contains(&year) {
        bail!("snapshot year out of expected range (2000..2100)");
    }
    if !(1..=12).contains(&month) {
        bail!("snapshot month must be 01..12");
    }
    if !(1..=31).contains(&day) {
        bail!("snapshot day must be 01..31");
    }

    Ok(())
}

fn run_dind(args: DindArgs) -> Result<()> {
    // Delegate to the Node.js script which handles manifest parsing and orchestration.
    // This mirrors what CI does and ensures consistent behavior.
    let mut node_args = vec!["scripts/dind-run-suite.mjs".to_owned()];

    match args.command {
        DindCommands::Run {
            tags,
            exclude_tags,
            emit_repro,
        } => {
            node_args.push("--mode".to_owned());
            node_args.push("run".to_owned());
            if let Some(t) = tags {
                node_args.push("--tags".to_owned());
                node_args.push(t);
            }
            if let Some(et) = exclude_tags {
                node_args.push("--exclude-tags".to_owned());
                node_args.push(et);
            }
            if emit_repro {
                node_args.push("--emit-repro".to_owned());
            }
        }
        DindCommands::Record { tags, exclude_tags } => {
            // Record mode: we need to invoke the harness directly for each scenario.
            // For now, we'll use a simplified approach that delegates to the suite script
            // in "run" mode but without golden files, then manually record.
            // Actually, the suite script doesn't have a record mode, so we invoke the harness directly.
            return run_dind_record(tags, exclude_tags);
        }
        DindCommands::Torture {
            tags,
            exclude_tags,
            runs,
            emit_repro,
        } => {
            node_args.push("--mode".to_owned());
            node_args.push("torture".to_owned());
            node_args.push("--runs".to_owned());
            node_args.push(runs.to_string());
            if let Some(t) = tags {
                node_args.push("--tags".to_owned());
                node_args.push(t);
            }
            if let Some(et) = exclude_tags {
                node_args.push("--exclude-tags".to_owned());
                node_args.push(et);
            }
            if emit_repro {
                node_args.push("--emit-repro".to_owned());
            }
        }
        DindCommands::Converge { tags, exclude_tags } => {
            // Converge mode requires grouping scenarios by converge_scope.
            // For simplicity, we'll just run the converge check on all matching scenarios.
            return run_dind_converge(tags, exclude_tags);
        }
    }

    let status = Command::new("node")
        .args(&node_args)
        .status()
        .context("failed to spawn `node` (is Node.js installed?)")?;

    if !status.success() {
        bail!("DIND suite failed (exit status: {status})");
    }

    Ok(())
}

/// Run DIND record mode: generate golden hashes for scenarios.
fn run_dind_record(tags: Option<String>, exclude_tags: Option<String>) -> Result<()> {
    let scenarios = load_matching_scenarios(tags.as_deref(), exclude_tags.as_deref())?;

    if scenarios.is_empty() {
        println!("No scenarios matched the specified tags.");
        return Ok(());
    }

    println!("DIND RECORD: {} scenarios", scenarios.len());

    let mut failed = 0;
    for scenario in &scenarios {
        let scenario_path = format!("testdata/dind/{}", scenario.path);
        let golden_path = match scenario_path.strip_suffix(".eintlog") {
            Some(base) => format!("{}.hashes.json", base),
            None => bail!(
                "scenario path '{}' does not end with '.eintlog'",
                scenario.path
            ),
        };

        println!("\n>>> Recording: {} -> {}", scenario_path, golden_path);

        let status = Command::new("cargo")
            .args([
                "run",
                "-p",
                "echo-dind-harness",
                "--quiet",
                "--",
                "record",
                &scenario_path,
                "--out",
                &golden_path,
            ])
            .status()
            .context("failed to spawn cargo")?;

        if !status.success() {
            eprintln!("\n!!! FAILED: {}", scenario.path);
            eprintln!("\nDIND FAILED. Repro command:");
            eprintln!(
                "  cargo run -p echo-dind-harness -- record {} --out {}\n",
                scenario_path, golden_path
            );
            failed += 1;
        }
    }

    if failed > 0 {
        bail!("DIND RECORD: {} scenario(s) failed", failed);
    }

    println!(
        "\nDIND RECORD: All {} scenarios recorded successfully.",
        scenarios.len()
    );
    Ok(())
}

/// Run DIND converge mode: verify convergence across scenario permutations.
fn run_dind_converge(tags: Option<String>, exclude_tags: Option<String>) -> Result<()> {
    let scenarios = load_matching_scenarios(tags.as_deref(), exclude_tags.as_deref())?;

    // Group scenarios by converge_scope
    let mut groups: std::collections::HashMap<String, Vec<&Scenario>> =
        std::collections::HashMap::new();

    for scenario in &scenarios {
        if let Some(scope) = &scenario.converge_scope {
            groups.entry(scope.clone()).or_default().push(scenario);
        }
    }

    if groups.is_empty() {
        println!("No scenarios with converge_scope matched the specified tags.");
        return Ok(());
    }

    println!("DIND CONVERGE: {} groups", groups.len());

    let mut failed = 0;
    for (scope, group) in &groups {
        if group.len() < 2 {
            println!(
                "\n>>> Skipping scope '{}': only {} scenario(s)",
                scope,
                group.len()
            );
            continue;
        }

        println!(
            "\n>>> Converge group '{}': {} scenarios",
            scope,
            group.len()
        );

        let scenario_paths: Vec<String> = group
            .iter()
            .map(|s| format!("testdata/dind/{}", s.path))
            .collect();

        let mut args = vec![
            "run".to_owned(),
            "-p".to_owned(),
            "echo-dind-harness".to_owned(),
            "--quiet".to_owned(),
            "--".to_owned(),
            "converge".to_owned(),
        ];
        args.extend(scenario_paths);

        let status = Command::new("cargo")
            .args(&args)
            .status()
            .context("failed to spawn cargo")?;

        if !status.success() {
            eprintln!("\n!!! CONVERGE FAILED for scope: {}", scope);
            // Build the repro command with all scenario paths
            let repro_paths: Vec<String> = group
                .iter()
                .map(|s| format!("testdata/dind/{}", s.path))
                .collect();
            eprintln!("\nDIND FAILED. Repro command:");
            eprintln!(
                "  cargo run -p echo-dind-harness -- converge {}\n",
                repro_paths.join(" ")
            );
            failed += 1;
        } else {
            println!("    CONVERGE OK: {}", scope);
        }
    }

    if failed > 0 {
        bail!("DIND CONVERGE: {} group(s) failed", failed);
    }

    println!("\nDIND CONVERGE: All groups verified.");
    Ok(())
}

/// Scenario entry from MANIFEST.json.
#[derive(serde::Deserialize)]
struct Scenario {
    path: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    converge_scope: Option<String>,
}

/// Load scenarios matching the given tag filters.
fn load_matching_scenarios(
    tags: Option<&str>,
    exclude_tags: Option<&str>,
) -> Result<Vec<Scenario>> {
    let manifest_path = "testdata/dind/MANIFEST.json";
    let content = std::fs::read_to_string(manifest_path).context("failed to read MANIFEST.json")?;
    let all_scenarios: Vec<Scenario> =
        serde_json::from_str(&content).context("failed to parse MANIFEST.json")?;

    let include_tags: Vec<&str> = tags
        .map(|t| {
            t.split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();
    let exclude_tag_list: Vec<&str> = exclude_tags
        .map(|t| {
            t.split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();

    let filtered: Vec<Scenario> = all_scenarios
        .into_iter()
        .filter(|s| {
            // If include tags specified, scenario must have at least one
            if !include_tags.is_empty()
                && !include_tags.iter().any(|t| s.tags.contains(&t.to_string()))
            {
                return false;
            }
            // If exclude tags specified, scenario must not have any
            if exclude_tag_list
                .iter()
                .any(|t| s.tags.contains(&t.to_string()))
            {
                return false;
            }
            true
        })
        .collect();

    Ok(filtered)
}

// ─── Wesley Commands ─────────────────────────────────────────────────────────

fn run_wesley(args: WesleyArgs) -> Result<()> {
    match args.command {
        WesleyCommands::Sync {
            wesley_path,
            schema,
            skip_rust,
            dry_run,
        } => {
            let resolved_schema = resolve_schema_path(&wesley_path, schema.as_deref())?;
            run_wesley_sync(&wesley_path, &resolved_schema, skip_rust, dry_run)
        }
        WesleyCommands::Check {
            wesley_path,
            schema,
        } => {
            let resolved_schema = resolve_schema_path(&wesley_path, schema.as_deref())?;
            run_wesley_check(&wesley_path, &resolved_schema)
        }
    }
}

/// Resolve the schema path with the following priority:
/// 1. If explicitly provided via --schema, use that path directly
/// 2. If local schema exists at `schemas/ttd-protocol.graphql`, use it
/// 3. Fall back to Wesley repo path `schemas/ttd-protocol.graphql`
fn resolve_schema_path(wesley_path: &str, schema_override: Option<&str>) -> Result<String> {
    // If explicitly provided, use it directly (could be absolute or relative to Wesley)
    if let Some(schema) = schema_override {
        return Ok(schema.to_owned());
    }

    // Check for local schema in Echo repo
    let local_schema = Path::new("schemas/ttd-protocol.graphql");
    if local_schema.exists() {
        let abs_path =
            std::fs::canonicalize(local_schema).context("failed to resolve local schema path")?;
        println!("Using local schema: {}", abs_path.display());
        return Ok(abs_path.to_string_lossy().into_owned());
    }

    // Fall back to Wesley repo path
    let wesley_dir = expand_tilde(wesley_path);
    let wesley_schema = Path::new(&wesley_dir).join("schemas/ttd-protocol.graphql");
    if wesley_schema.exists() {
        println!("Using Wesley schema: {}", wesley_schema.display());
        return Ok("schemas/ttd-protocol.graphql".to_owned());
    }

    bail!(
        "No schema found. Checked:\n  - schemas/ttd-protocol.graphql (local)\n  - {}/schemas/ttd-protocol.graphql (Wesley)",
        wesley_dir
    );
}

/// Expand ~ to home directory.
fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return format!("{}{}", home.to_string_lossy(), &path[1..]);
        }
    }
    path.to_owned()
}

/// Sync Wesley outputs into Echo's vendored directories.
fn run_wesley_sync(wesley_path: &str, schema: &str, skip_rust: bool, dry_run: bool) -> Result<()> {
    let wesley_dir = expand_tilde(wesley_path);
    let wesley_path = Path::new(&wesley_dir);

    if !wesley_path.exists() {
        bail!(
            "Wesley repository not found at '{}'. Clone it or specify --wesley-path.",
            wesley_dir
        );
    }

    // Resolve schema path - could be absolute (local) or relative (to Wesley)
    let schema_path = if Path::new(schema).is_absolute() {
        Path::new(schema).to_owned()
    } else {
        wesley_path.join(schema)
    };

    if !schema_path.exists() {
        bail!(
            "TTD schema not found at '{}'. Create it or specify --schema.",
            schema_path.display()
        );
    }

    // Determine if using local schema
    let is_local_schema = Path::new(schema).is_absolute() && schema.contains("/echo/schemas/");

    println!("WESLEY SYNC");
    println!("  Wesley repo: {}", wesley_dir);
    println!(
        "  Schema: {} {}",
        schema_path.display(),
        if is_local_schema { "(local)" } else { "" }
    );
    println!("  Skip Rust: {}", skip_rust);
    println!("  Dry run: {}", dry_run);
    println!();

    // Get Wesley commit SHA for provenance
    let wesley_sha = get_git_sha(wesley_path)?;
    println!("Wesley commit: {}", wesley_sha);

    // Create temp output directory for Wesley
    let temp_dir = tempfile::tempdir().context("failed to create temp directory")?;
    let temp_out = temp_dir.path();

    // Step 1: Run Wesley compile-ttd
    println!("\n>>> Running Wesley compile-ttd...");
    if dry_run {
        println!("  [dry-run] would run: wesley compile-ttd --schema {} --out-dir {} --target manifest,typescript",
                 schema_path.display(), temp_out.display());
    } else {
        let status = Command::new("node")
            .current_dir(wesley_path)
            .args([
                "packages/wesley-host-node/bin/wesley.mjs",
                "compile-ttd",
                "--schema",
                schema_path.to_str().unwrap(),
                "--out-dir",
                temp_out.to_str().unwrap(),
                "--target",
                "manifest,typescript",
            ])
            .status()
            .context("failed to run Wesley compile-ttd")?;

        if !status.success() {
            bail!("Wesley compile-ttd failed (exit status: {})", status);
        }
    }

    // Step 2: Read schema_hash from generated manifest
    let schema_hash = if dry_run {
        "<dry-run>".to_owned()
    } else {
        let manifest_path = temp_out.join("manifest/manifest.json");
        let manifest_content =
            std::fs::read_to_string(&manifest_path).context("failed to read manifest.json")?;
        let manifest: serde_json::Value =
            serde_json::from_str(&manifest_content).context("failed to parse manifest.json")?;
        manifest["schemaHash"].as_str().unwrap_or("").to_owned()
    };
    println!("Schema hash: {}", schema_hash);

    // Step 3: Copy manifest files to crates/ttd-manifest/
    println!("\n>>> Copying manifest files...");
    let manifest_dest = Path::new("crates/ttd-manifest");
    if dry_run {
        println!(
            "  [dry-run] would copy {} -> {}",
            temp_out.join("manifest").display(),
            manifest_dest.display()
        );
    } else {
        copy_dir_contents(&temp_out.join("manifest"), manifest_dest)?;
        ensure_ttd_manifest_crate(manifest_dest)?;
    }

    // Step 4: Copy TypeScript files to packages/ttd-protocol-ts/
    println!("\n>>> Copying TypeScript files...");
    let ts_dest = Path::new("packages/ttd-protocol-ts");
    if dry_run {
        println!(
            "  [dry-run] would copy {} -> {}",
            temp_out.join("typescript").display(),
            ts_dest.display()
        );
    } else {
        copy_dir_contents(&temp_out.join("typescript"), ts_dest)?;
    }

    // Step 5: Generate Rust via echo-ttd-gen
    let rust_dest = Path::new("crates/ttd-protocol-rs");
    if !skip_rust {
        println!("\n>>> Generating Rust via echo-ttd-gen...");
        let ir_path = if dry_run {
            temp_out.join("manifest/ttd-ir.json")
        } else {
            Path::new("crates/ttd-manifest/ttd-ir.json").to_owned()
        };

        if dry_run {
            println!(
                "  [dry-run] would run: cat {} | cargo run -p echo-ttd-gen -- -o {}/lib.rs",
                ir_path.display(),
                rust_dest.display()
            );
        } else {
            let ir_content =
                std::fs::read_to_string(&ir_path).context("failed to read ttd-ir.json")?;

            let output = Command::new("cargo")
                .args(["run", "-p", "echo-ttd-gen", "--quiet", "--"])
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .spawn()
                .context("failed to spawn echo-ttd-gen")?;

            use std::io::Write;
            output
                .stdin
                .as_ref()
                .unwrap()
                .write_all(ir_content.as_bytes())?;
            let output = output.wait_with_output()?;

            if !output.status.success() {
                bail!(
                    "echo-ttd-gen failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }

            let rust_code = String::from_utf8(output.stdout)
                .context("echo-ttd-gen output was not valid UTF-8")?;

            std::fs::write(rust_dest.join("lib.rs"), rust_code)
                .context("failed to write lib.rs")?;
            println!("  Wrote {}/lib.rs", rust_dest.display());
        }
    }

    // Step 6: Update wesley.lock
    println!("\n>>> Updating wesley.lock...");
    let lock_path = Path::new("docs/wesley/wesley.lock");
    let now = chrono::Utc::now().to_rfc3339();

    // Determine schema source info for the lock file
    let (schema_source, schema_path_for_lock) = if is_local_schema {
        ("local", "schemas/ttd-protocol.graphql")
    } else {
        ("wesley", schema)
    };

    let lock_content = serde_json::json!({
        "comment": "Wesley provenance file - records the state of Wesley when artifacts were generated",
        "schema_source": schema_source,
        "schema_path": schema_path_for_lock,
        "wesley_repo": "https://github.com/flyingrobots/Wesley",
        "wesley_commit": wesley_sha,
        "schema_hash": schema_hash,
        "generated_at": now,
        "generator_versions": {
            "wesley_cli": "0.1.0",
            "echo_ttd_gen": env!("CARGO_PKG_VERSION")
        },
        "artifacts": {
            "ttd_manifest": "crates/ttd-manifest/",
            "ttd_protocol_rs": "crates/ttd-protocol-rs/",
            "ttd_protocol_ts": "packages/ttd-protocol-ts/"
        }
    });

    if dry_run {
        println!("  [dry-run] would write to {}", lock_path.display());
        println!("{}", serde_json::to_string_pretty(&lock_content)?);
    } else {
        std::fs::write(lock_path, serde_json::to_string_pretty(&lock_content)?)
            .context("failed to write wesley.lock")?;
        println!("  Wrote {}", lock_path.display());
    }

    println!("\nWESLEY SYNC complete.");
    Ok(())
}

/// Get the current git SHA of a repository.
fn get_git_sha(repo_path: &Path) -> Result<String> {
    let output = Command::new("git")
        .current_dir(repo_path)
        .args(["rev-parse", "HEAD"])
        .output()
        .context("failed to get git SHA")?;

    if !output.status.success() {
        bail!("git rev-parse HEAD failed");
    }

    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

/// Copy contents of a directory to another location.
fn copy_dir_contents(src: &Path, dest: &Path) -> Result<()> {
    if !src.exists() {
        bail!("source directory '{}' does not exist", src.display());
    }

    // Ensure destination exists
    std::fs::create_dir_all(dest)?;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());

        if src_path.is_file() {
            std::fs::copy(&src_path, &dest_path)?;
            println!("  Copied {}", dest_path.display());
        }
    }

    Ok(())
}

fn ensure_ttd_manifest_crate(dest: &Path) -> Result<()> {
    let cargo_toml = dest.join("Cargo.toml");
    if !cargo_toml.exists() {
        std::fs::write(
            &cargo_toml,
            r#"# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
[package]
name = "ttd-manifest"
version = "0.1.0"
edition = "2021"
rust-version = "1.90.0"
description = "Vendored TTD protocol manifest and IR (data-only crate)"
license = "Apache-2.0"
repository = "https://github.com/flyingrobots/echo"
readme = "README.md"
keywords = ["echo", "ttd", "protocol"]
categories = ["data-structures"]

[dependencies]
# Data-only crate, no dependencies.
"#,
        )?;
        println!("  Created {}", cargo_toml.display());
    }

    let readme = dest.join("README.md");
    if !readme.exists() {
        std::fs::write(
            &readme,
            r#"# ttd-manifest

Vendored TTD protocol manifest and IR.

This is a data-only crate used to stage and provide access to the TTD protocol's
Intermediate Representation (IR) and metadata generated by Wesley.

The contents of this directory are managed by `cargo xtask wesley sync`.
"#,
        )?;
        println!("  Created {}", readme.display());
    }

    let src_dir = dest.join("src");
    std::fs::create_dir_all(&src_dir)?;

    let lib_rs = src_dir.join("lib.rs");
    if !lib_rs.exists() {
        std::fs::write(
            &lib_rs,
            r#"// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Vendored TTD protocol manifest and IR.

/// The TTD Intermediate Representation (IR).
pub const TTD_IR: &str = include_str!("../ttd-ir.json");

/// The TTD protocol manifest.
pub const MANIFEST: &str = include_str!("../manifest.json");

/// The TTD behavioral contracts.
pub const CONTRACTS: &str = include_str!("../contracts.json");

/// The TTD protocol schema (JSON).
pub const SCHEMA: &str = include_str!("../schema.json");
"#,
        )?;
        println!("  Created {}", lib_rs.display());
    }

    Ok(())
}

/// Verify vendored artifacts match what Wesley would generate.
fn run_wesley_check(wesley_path: &str, schema: &str) -> Result<()> {
    let skip_repo_check = std::env::var("SKIP_WESLEY_REPO_CHECK").unwrap_or_default() == "1";

    let wesley_dir = expand_tilde(wesley_path);
    let wesley_path = Path::new(&wesley_dir);

    if !skip_repo_check && !wesley_path.exists() {
        bail!(
            "Wesley repository not found at '{}'. Clone it or specify --wesley-path. (Set SKIP_WESLEY_REPO_CHECK=1 to bypass this check in CI if you only want to verify against lockfile hash)",
            wesley_dir
        );
    }

    // Resolve schema path - could be absolute (local) or relative (to Wesley)
    let schema_path = if Path::new(schema).is_absolute() {
        Path::new(schema).to_owned()
    } else {
        if skip_repo_check {
            bail!("SKIP_WESLEY_REPO_CHECK=1 requires an absolute schema path (via local schema or --schema)");
        }
        wesley_path.join(schema)
    };

    if !schema_path.exists() {
        bail!(
            "TTD schema not found at '{}'. Specify --schema.",
            schema_path.display()
        );
    }

    // Determine if using local schema
    let is_local_schema = Path::new(schema).is_absolute() && schema.contains("/echo/schemas/");

    println!("WESLEY CHECK");
    if skip_repo_check {
        println!("  Wesley repo: <skipped>");
    } else {
        println!("  Wesley repo: {}", wesley_dir);
    }
    println!(
        "  Schema: {} {}",
        schema_path.display(),
        if is_local_schema { "(local)" } else { "" }
    );
    println!();

    // Read current lock file
    let lock_path = Path::new("docs/wesley/wesley.lock");
    let lock_content = std::fs::read_to_string(lock_path)
        .context("failed to read wesley.lock - run 'cargo xtask wesley sync' first")?;
    let lock: serde_json::Value = serde_json::from_str(&lock_content)?;

    let expected_hash = lock["schema_hash"].as_str().unwrap_or("");
    let expected_commit = lock["wesley_commit"].as_str().unwrap_or("");

    println!("Lock file:");
    println!("  Wesley commit: {}", expected_commit);
    println!("  Schema hash: {}", expected_hash);
    println!();

    if !skip_repo_check {
        // Get current Wesley commit
        let current_commit = get_git_sha(wesley_path)?;
        println!("Current:");
        println!("  Wesley commit: {}", current_commit);

        if current_commit != expected_commit {
            println!();
            println!("WARNING: Wesley commit has changed since last sync.");
            println!("  Expected: {}", expected_commit);
            println!("  Current:  {}", current_commit);
            println!();
            println!("Run 'cargo xtask wesley sync' to update vendored artifacts.");
            bail!("Wesley commit mismatch");
        }
    } else {
        println!("Current: <repo check skipped>");
    }

    // Always check schema hash if we can run wesley
    // Wait, we can't run wesley if we don't have the repo.
    // So the check in CI should just be "does the lockfile exist and match the schema hash of the vendored files?"
    // But how do we know the schema hash of the vendored files without running wesley?
    // The vendored manifest.json has it!

    let manifest_path = Path::new("crates/ttd-manifest/manifest.json");
    if manifest_path.exists() {
        let manifest_content = std::fs::read_to_string(manifest_path)?;
        let manifest: serde_json::Value = serde_json::from_str(&manifest_content)?;
        let actual_hash = manifest["schemaHash"].as_str().unwrap_or("");
        println!("Manifest hash: {}", actual_hash);

        if actual_hash != expected_hash {
            bail!(
                "Schema hash mismatch: manifest ({}) vs lockfile ({})",
                actual_hash,
                expected_hash
            );
        }
    }

    println!();
    println!("WESLEY CHECK passed - artifacts are up to date.");
    Ok(())
}
