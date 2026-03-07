// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::print_stdout, clippy::print_stderr)]

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
use std::path::{Path, PathBuf};
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
    /// Generate man pages for echo-cli.
    ManPages(ManPagesArgs),
    /// Lint docs for dead cross-references (broken markdown links).
    LintDeadRefs(LintDeadRefsArgs),
    /// Auto-fix common markdown lint violations (SPDX headers, prettier, markdownlint).
    MarkdownFix(MarkdownFixArgs),
    /// Run all docs linters: markdown-fix (auto-fix) then lint-dead-refs (check).
    DocsLint(DocsLintArgs),
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

#[derive(Args)]
struct ManPagesArgs {
    /// Output directory for generated man pages.
    #[arg(long, default_value = "docs/man")]
    out: std::path::PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Dags(args) => run_dags(args),
        Commands::Dind(args) => run_dind(args),
        Commands::ManPages(args) => run_man_pages(args),
        Commands::LintDeadRefs(args) => run_lint_dead_refs(args),
        Commands::MarkdownFix(args) => run_markdown_fix(&args),
        Commands::DocsLint(args) => run_docs_lint(args),
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
            Some(base) => format!("{base}.hashes.json"),
            None => bail!(
                "scenario path '{}' does not end with '.eintlog'",
                scenario.path
            ),
        };

        println!("\n>>> Recording: {scenario_path} -> {golden_path}");

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
                "  cargo run -p echo-dind-harness -- record {scenario_path} --out {golden_path}\n"
            );
            failed += 1;
        }
    }

    if failed > 0 {
        bail!("DIND RECORD: {failed} scenario(s) failed");
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

        if status.success() {
            println!("    CONVERGE OK: {scope}");
        } else {
            eprintln!("\n!!! CONVERGE FAILED for scope: {scope}");
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
        }
    }

    if failed > 0 {
        bail!("DIND CONVERGE: {failed} group(s) failed");
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
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();
    let exclude_tag_list: Vec<&str> = exclude_tags
        .map(|t| {
            t.split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();

    let filtered: Vec<Scenario> = all_scenarios
        .into_iter()
        .filter(|s| {
            // If include tags specified, scenario must have at least one
            if !include_tags.is_empty()
                && !include_tags
                    .iter()
                    .any(|t| s.tags.contains(&(*t).to_string()))
            {
                return false;
            }
            // If exclude tags specified, scenario must not have any
            if exclude_tag_list
                .iter()
                .any(|t| s.tags.contains(&(*t).to_string()))
            {
                return false;
            }
            true
        })
        .collect();

    Ok(filtered)
}

#[derive(Args)]
struct LintDeadRefsArgs {
    /// Root directory to scan (default: `docs/`).
    #[arg(long, default_value = "docs")]
    root: PathBuf,

    /// Also check non-markdown links (images, HTML, etc.).
    #[arg(long)]
    all: bool,
}

fn run_lint_dead_refs(args: LintDeadRefsArgs) -> Result<()> {
    let root = &args.root;
    if !root.is_dir() {
        bail!("{} is not a directory", root.display());
    }

    let link_re =
        regex::Regex::new(r"\[(?:[^\]]*)\]\(([^)]+)\)").context("failed to compile link regex")?;

    let mut md_files = Vec::new();
    collect_md_files(root, &mut md_files)?;
    md_files.sort();

    let mut broken: Vec<(PathBuf, usize, String, PathBuf)> = Vec::new();

    for file in &md_files {
        let content = std::fs::read_to_string(file)
            .with_context(|| format!("failed to read {}", file.display()))?;

        for (line_no, line) in content.lines().enumerate() {
            for cap in link_re.captures_iter(line) {
                let raw_target = &cap[1];

                // Skip external URLs
                if raw_target.starts_with("http://") || raw_target.starts_with("https://") {
                    continue;
                }
                // Skip mailto
                if raw_target.starts_with("mailto:") {
                    continue;
                }
                // Skip pure anchors
                if raw_target.starts_with('#') {
                    continue;
                }

                // Strip fragment anchor
                let target = raw_target.split('#').next().unwrap_or(raw_target);

                // By default, only check .md and extensionless links.
                // With --all, check everything.
                if !args.all {
                    let ext = Path::new(target).extension().and_then(|e| e.to_str());
                    if ext.is_some_and(|e| e != "md") {
                        continue;
                    }
                }

                if let Some(resolved) = try_resolve_link(file, target, root) {
                    broken.push((file.clone(), line_no + 1, raw_target.to_string(), resolved));
                }
            }
        }
    }

    if broken.is_empty() {
        println!(
            "lint-dead-refs: scanned {} files, all links OK",
            md_files.len()
        );
        return Ok(());
    }

    eprintln!(
        "lint-dead-refs: {} broken link(s) in {} file(s):\n",
        broken.len(),
        md_files.len()
    );
    for (file, line, target, resolved) in &broken {
        eprintln!("  {}:{}: -> {}", file.display(), line, target);
        eprintln!("    resolved to: {} (not found)", resolved.display());
        eprintln!();
    }

    bail!("lint-dead-refs: {} broken link(s) found", broken.len());
}

/// Try to resolve a link target to an existing path.
///
/// Returns `None` if the link resolves successfully (target exists).
/// Returns `Some(best_guess_path)` if no resolution succeeded.
fn try_resolve_link(source_file: &Path, target: &str, docs_root: &Path) -> Option<PathBuf> {
    let candidates = build_candidates(source_file, target, docs_root);
    for candidate in &candidates {
        if candidate.exists() {
            return None; // Link is valid
        }
    }
    // Return the primary candidate for error reporting
    Some(candidates.into_iter().next().unwrap_or_default())
}

/// Build candidate paths for a link target.
///
/// - Root-relative links (`/foo/bar`) try `docs_root/foo/bar`, then repo root.
/// - Relative links (`../foo`) resolve from the source file's directory.
/// - Extensionless links also try with `.md` and `.html` appended.
fn build_candidates(source_file: &Path, target: &str, docs_root: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    let primary = if target.starts_with('/') {
        let stripped = target.trim_start_matches('/');
        // VitePress root-relative: try docs root first
        candidates.push(docs_root.join(stripped));
        // VitePress serves docs/public/ at root, so /foo.html may be docs/public/foo.html
        candidates.push(docs_root.join("public").join(stripped));
        // Also try repo root (for links like /crates/foo/README.md)
        if let Some(repo_root) = docs_root.parent() {
            candidates.push(repo_root.join(stripped));
        }
        docs_root.join(stripped)
    } else {
        let p = source_file
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(target);
        candidates.push(p.clone());
        p
    };

    // For extensionless links, also try .md and .html
    let has_extension = Path::new(target)
        .extension()
        .is_some_and(|ext| !ext.is_empty());
    if !has_extension {
        let stem = primary.file_name().unwrap_or_default().to_string_lossy();
        candidates.push(primary.with_file_name(format!("{stem}.md")));
        candidates.push(primary.with_file_name(format!("{stem}.html")));
    }

    candidates
}

/// Recursively collect `.md` files under `dir`.
fn collect_md_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let entries =
        std::fs::read_dir(dir).with_context(|| format!("failed to read {}", dir.display()))?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_md_files(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "md") {
            out.push(path);
        }
    }
    Ok(())
}

#[derive(Args)]
struct MarkdownFixArgs {
    /// Root directory to fix (default: `docs/`).
    #[arg(long, default_value = "docs")]
    root: PathBuf,

    /// Skip prettier formatting.
    #[arg(long)]
    no_prettier: bool,

    /// Skip markdownlint --fix.
    #[arg(long)]
    no_lint: bool,
}

fn run_markdown_fix(args: &MarkdownFixArgs) -> Result<()> {
    let root = &args.root;
    if !root.is_dir() {
        bail!("{} is not a directory", root.display());
    }

    let mut md_files = Vec::new();
    collect_md_files(root, &mut md_files)?;
    md_files.sort();

    if md_files.is_empty() {
        println!("markdown-fix: no .md files found in {}", root.display());
        return Ok(());
    }

    println!(
        "markdown-fix: {} file(s) in {}",
        md_files.len(),
        root.display()
    );

    // 1) SPDX header repair
    if Path::new("scripts/ensure_spdx.sh").exists() {
        println!("markdown-fix: repairing SPDX headers...");
        let status = Command::new("bash")
            .args(["scripts/ensure_spdx.sh"])
            .env("ECHO_AUTO_FMT", "1")
            .status()
            .context("failed to run scripts/ensure_spdx.sh")?;
        if !status.success() {
            bail!("markdown-fix: SPDX header repair failed");
        }
    }

    let file_args: Vec<&str> = md_files.iter().filter_map(|p| p.to_str()).collect();

    // 2) Prettier formatting
    if !args.no_prettier {
        if command_exists("npx") {
            println!("markdown-fix: running prettier...");
            let status = Command::new("npx")
                .arg("prettier")
                .arg("--write")
                .args(&file_args)
                .stdout(std::process::Stdio::null())
                .status()
                .context("failed to run prettier")?;
            if !status.success() {
                bail!("markdown-fix: prettier failed");
            }
        } else {
            eprintln!("markdown-fix: npx not found, skipping prettier");
        }
    }

    // 3) markdownlint --fix
    if !args.no_lint {
        if command_exists("npx") {
            println!("markdown-fix: running markdownlint --fix...");
            let status = Command::new("npx")
                .arg("markdownlint-cli2")
                .arg("--fix")
                .args(&file_args)
                .status()
                .context("failed to run markdownlint-cli2")?;
            if !status.success() {
                // markdownlint --fix returns non-zero if unfixable errors remain.
                // This is expected — report but don't bail.
                eprintln!(
                    "markdown-fix: markdownlint reported errors that --fix could not resolve"
                );
                eprintln!("  Run `npx markdownlint-cli2 <file>` for details");
            }
        } else {
            eprintln!("markdown-fix: npx not found, skipping markdownlint");
        }
    }

    println!("markdown-fix: done");
    Ok(())
}

fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

#[derive(Args)]
struct DocsLintArgs {
    /// Root directory to lint (default: `docs/`).
    #[arg(long, default_value = "docs")]
    root: PathBuf,

    /// Also check non-markdown links (images, HTML, etc.) in lint-dead-refs.
    #[arg(long)]
    all: bool,
}

fn run_docs_lint(args: DocsLintArgs) -> Result<()> {
    // Phase 1: auto-fix
    let fix_args = MarkdownFixArgs {
        root: args.root.clone(),
        no_prettier: false,
        no_lint: false,
    };
    run_markdown_fix(&fix_args)?;

    println!();

    // Phase 2: check dead refs
    let refs_args = LintDeadRefsArgs {
        root: args.root,
        all: args.all,
    };
    run_lint_dead_refs(refs_args)
}

fn run_man_pages(args: ManPagesArgs) -> Result<()> {
    use clap::CommandFactory;

    let out_dir = &args.out;
    std::fs::create_dir_all(out_dir)
        .with_context(|| format!("failed to create output directory: {}", out_dir.display()))?;

    // Remove stale man pages so the output is an exact snapshot.
    if let Ok(entries) = std::fs::read_dir(out_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("echo-cli") && name.ends_with(".1") {
                std::fs::remove_file(entry.path()).with_context(|| {
                    format!(
                        "failed to remove stale man page: {}",
                        entry.path().display()
                    )
                })?;
            }
        }
    }

    let cmd = warp_cli::cli::Cli::command();
    let man = clap_mangen::Man::new(cmd.clone());
    let mut buf: Vec<u8> = Vec::new();
    man.render(&mut buf)
        .context("failed to render echo-cli.1")?;
    let path = out_dir.join("echo-cli.1");
    std::fs::write(&path, &buf).with_context(|| format!("failed to write {}", path.display()))?;
    println!("  wrote {}", path.display());

    for sub in cmd.get_subcommands() {
        let sub_name = sub.get_name().to_string();
        // Leak is fine: xtask is short-lived and we need 'static for clap::Str.
        let prefixed_name: &'static str =
            Box::leak(format!("echo-cli-{sub_name}").into_boxed_str());
        let prefixed = sub.clone().name(prefixed_name);
        let man = clap_mangen::Man::new(prefixed);
        let mut buf: Vec<u8> = Vec::new();
        man.render(&mut buf)
            .with_context(|| format!("failed to render echo-cli-{sub_name}.1"))?;
        let filename = format!("echo-cli-{sub_name}.1");
        let path = out_dir.join(&filename);
        std::fs::write(&path, &buf)
            .with_context(|| format!("failed to write {}", path.display()))?;
        println!("  wrote {}", path.display());
    }

    println!("Man pages generated in {}", out_dir.display());
    Ok(())
}
