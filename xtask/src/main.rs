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
use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

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
    /// Bake benchmark report artifacts and export benchmark data.
    Bench(BenchArgs),
    /// Generate dependency DAG DOT/SVG artifacts for issues + milestones.
    Dags(DagsArgs),
    /// Emit agent-native Doghouse recorder events.
    Doghouse(DoghouseArgs),
    /// Summarize current-head PR status, unresolved threads, and check state.
    PrStatus(PrStatusArgs),
    /// Record a durable PR review-state snapshot under local ignored artifacts.
    PrSnapshot(PrSnapshotArgs),
    /// List, reply to, or resolve PR review threads via `gh`.
    PrThreads(PrThreadsArgs),
    /// Run the high-signal local gate before opening a PR.
    PrPreflight(PrPreflightArgs),
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
    /// METHOD workspace operations (status, backlog inspection).
    Method(MethodArgs),
    /// Wesley consumer artifact maintenance.
    Wesley(WesleyArgs),
    /// Run a narrow local test slice with explicit Cargo target selection.
    TestSlice(TestSliceArgs),
}

#[derive(Args)]
struct TestSliceArgs {
    /// Named local iteration slice to run.
    #[arg(value_enum)]
    slice: TestSlice,
    /// Print the exact commands without running them.
    #[arg(long)]
    dry_run: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum TestSlice {
    /// Strand contract and live-basis integration tests.
    Strand,
    /// Settlement module unit tests only.
    Settlement,
    /// Observation module unit tests only.
    Observation,
    /// Neighborhood module unit tests only.
    Neighborhood,
    /// High-signal warp-core smoke without compiling every integration-test target.
    WarpCoreSmoke,
}

#[derive(Args)]
struct MethodArgs {
    /// METHOD subcommand to execute.
    #[command(subcommand)]
    command: MethodCommand,
}

#[derive(Args)]
struct WesleyArgs {
    /// Wesley maintenance subcommand to execute.
    #[command(subcommand)]
    command: WesleyCommand,
}

#[derive(Subcommand)]
enum WesleyCommand {
    /// Verify Echo's downstream Wesley-generated protocol consumer artifacts.
    Sync(WesleySyncArgs),
}

#[derive(Args)]
struct WesleySyncArgs {
    /// Output as JSON (agent surface).
    #[arg(long)]
    json: bool,
}

#[derive(Subcommand)]
enum MethodCommand {
    /// Capture a backlog note in inbox/.
    Inbox(MethodInboxArgs),
    /// Scaffold a retro and witness directory for an active cycle.
    Close(MethodCloseArgs),
    /// Promote a backlog item into the next numbered design cycle.
    Pull(MethodPullArgs),
    /// Check playback questions against committed tests.
    Drift(MethodDriftArgs),
    /// Show backlog lanes, active cycles, and legend load.
    Status(MethodStatusArgs),
    /// Regenerate METHOD task matrix markdown and CSV.
    Matrix(MethodMatrixArgs),
    /// Regenerate METHOD task DAG DOT and SVG.
    Dag(MethodDagArgs),
    /// Show tasks with no unresolved backlog-task blockers.
    Frontier(MethodFrontierArgs),
    /// Show the unweighted longest dependency chain.
    CriticalPath(MethodCriticalPathArgs),
    /// Verify METHOD graph artifacts are up to date.
    CheckDag(MethodCheckDagArgs),
}

#[derive(Args)]
struct MethodInboxArgs {
    /// Idea title or one-line note to capture.
    title: String,
}

#[derive(Args)]
struct MethodCloseArgs {
    /// Cycle number or full cycle directory name. Defaults to most recent active cycle.
    cycle: Option<String>,
}

#[derive(Args)]
struct MethodPullArgs {
    /// Backlog item path, file stem, METHOD task id, or native task id.
    item: String,
}

#[derive(Args)]
struct MethodDriftArgs {
    /// Cycle number or full cycle directory name. Defaults to most recent active cycle.
    cycle: Option<String>,
    /// Output as JSON (agent surface).
    #[arg(long)]
    json: bool,
}

#[derive(Args)]
struct MethodStatusArgs {
    /// Output as JSON (agent surface).
    #[arg(long)]
    json: bool,
}

#[derive(Args)]
struct MethodMatrixArgs {
    /// Check generated matrix artifacts without writing them.
    #[arg(long)]
    check: bool,
}

#[derive(Args)]
struct MethodDagArgs {
    /// Check generated DAG artifacts without writing them.
    #[arg(long)]
    check: bool,
    /// Skip rendering SVG with Graphviz; write/check DOT only.
    #[arg(long)]
    no_render: bool,
}

#[derive(Args)]
struct MethodFrontierArgs {
    /// Output as JSON (agent surface).
    #[arg(long)]
    json: bool,
    /// Maximum number of tasks to print in human mode.
    #[arg(long, default_value = "25")]
    limit: usize,
}

#[derive(Args)]
struct MethodCriticalPathArgs {
    /// Output as JSON (agent surface).
    #[arg(long)]
    json: bool,
}

#[derive(Args)]
struct MethodCheckDagArgs {
    /// Skip checking rendered SVG freshness.
    #[arg(long)]
    no_render: bool,
}

#[derive(Args)]
struct BenchArgs {
    /// Benchmark maintenance subcommand to execute.
    #[command(subcommand)]
    command: BenchCommand,
}

#[derive(Subcommand)]
enum BenchCommand {
    /// Bake the unified benchmark report and refresh policy JSON from Criterion outputs.
    Bake(BenchBakeArgs),
    /// Export the parallel policy matrix benchmark as raw JSON.
    PolicyExport(BenchPolicyExportArgs),
    /// Verify that committed benchmark artifacts match the current benchmark inputs.
    CheckArtifacts(BenchCheckArtifactsArgs),
}

#[derive(Args)]
struct BenchBakeArgs {
    /// Output path for the baked offline-friendly report.
    #[arg(long, default_value = "docs/benchmarks/report-inline.html")]
    out: PathBuf,
    /// HTML template used for the unified benchmark page.
    #[arg(long, default_value = "docs/benchmarks/index.html")]
    template: PathBuf,
    /// Criterion root for core benchmark groups.
    #[arg(long, default_value = "target/criterion")]
    criterion_root: PathBuf,
    /// Output path for the refreshed policy matrix JSON payload.
    #[arg(long, default_value = "docs/benchmarks/parallel-policy-matrix.json")]
    policy_json_out: PathBuf,
}

#[derive(Args)]
struct BenchPolicyExportArgs {
    /// Criterion root for the parallel policy matrix benchmark group.
    #[arg(long, default_value = "target/criterion/parallel_policy_matrix")]
    criterion_root: PathBuf,
    /// Output path for the raw policy matrix JSON payload.
    #[arg(long, default_value = "docs/benchmarks/parallel-policy-matrix.json")]
    json_out: PathBuf,
}

#[derive(Args)]
struct BenchCheckArtifactsArgs {
    /// Path to the baked inline benchmark report.
    #[arg(long, default_value = "docs/benchmarks/report-inline.html")]
    html: PathBuf,
    /// Path to the exported policy-matrix JSON payload.
    #[arg(long, default_value = "docs/benchmarks/parallel-policy-matrix.json")]
    json: PathBuf,
}

#[derive(Args)]
struct DindArgs {
    /// DIND subcommand to execute.
    #[command(subcommand)]
    command: DindCommands,
}

#[derive(Args)]
struct DoghouseArgs {
    /// Doghouse subcommand to execute.
    #[command(subcommand)]
    command: DoghouseCommand,
}

#[derive(Subcommand)]
enum DoghouseCommand {
    /// Capture a sortie and emit JSONL events with the current verdict.
    Sortie(DoghouseSortieArgs),
    /// Post a CodeRabbit review/resume command on the PR when Rabbit is actionable again.
    NudgeCoderabbit(DoghouseNudgeCoderabbitArgs),
    /// Re-enable CodeRabbit by toggling its summary-comment checkboxes when it is paused behind them.
    RearmCoderabbit(DoghouseRearmCoderabbitArgs),
}

#[derive(Args)]
struct DoghouseSortieArgs {
    /// Optional PR number or selector understood by `gh pr view`.
    selector: Option<String>,
    /// Why this Doghouse sortie is being captured.
    #[arg(long, value_enum, default_value_t = DoghouseSortieIntent::ManualProbe)]
    intent: DoghouseSortieIntent,
    /// Local artifact root for recorded PR snapshots.
    #[arg(long, default_value = "artifacts/pr-review")]
    out_dir: PathBuf,
}

#[derive(Args)]
struct DoghouseRearmCoderabbitArgs {
    /// Optional PR number or selector understood by `gh pr view`.
    selector: Option<String>,
    /// Confirm that Doghouse should edit the CodeRabbit summary comment.
    #[arg(long)]
    yes: bool,
}

#[derive(Args)]
struct DoghouseNudgeCoderabbitArgs {
    /// Optional PR number or selector understood by `gh pr view`.
    selector: Option<String>,
    /// Confirm that Doghouse should post the CodeRabbit nudge comment.
    #[arg(long)]
    yes: bool,
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
    /// Check committed man pages without writing.
    #[arg(long)]
    check: bool,
}

#[derive(Args)]
struct PrStatusArgs {
    /// Optional PR number or selector understood by `gh pr view`.
    selector: Option<String>,
}

#[derive(Args)]
struct PrSnapshotArgs {
    /// Optional PR number or selector understood by `gh pr view`.
    selector: Option<String>,
    /// Why this Doghouse snapshot is being captured.
    #[arg(long, value_enum, default_value_t = DoghouseSortieIntent::ManualProbe)]
    intent: DoghouseSortieIntent,
    /// Local artifact root for recorded PR snapshots.
    #[arg(long, default_value = "artifacts/pr-review")]
    out_dir: PathBuf,
}

#[derive(Args)]
struct PrThreadsArgs {
    /// PR review thread action to execute.
    #[command(subcommand)]
    command: PrThreadsCommand,
}

#[derive(Subcommand)]
enum PrThreadsCommand {
    /// List unresolved review threads for the current PR (or an explicit selector).
    List {
        /// Optional PR number or selector understood by `gh pr view`.
        selector: Option<String>,
    },
    /// Reply to an inline review comment by comment id.
    Reply(PrThreadsReplyArgs),
    /// Resolve one or more review threads, or all unresolved threads for a PR.
    Resolve(PrThreadsResolveArgs),
}

#[derive(Args)]
struct PrThreadsReplyArgs {
    /// Numeric GitHub pull-request review comment id to reply to.
    comment_id: u64,
    /// Optional PR number or selector understood by `gh pr view`.
    #[arg(long)]
    selector: Option<String>,
    /// Reply body text.
    #[arg(long, conflicts_with = "body_file")]
    body: Option<String>,
    /// Path to a file containing the reply body.
    #[arg(long = "body-file", conflicts_with = "body")]
    body_file: Option<PathBuf>,
}

#[derive(Args)]
struct PrThreadsResolveArgs {
    /// Resolve every unresolved thread on the selected PR.
    #[arg(long)]
    all: bool,
    /// Optional PR number or selector understood by `gh pr view` (used with `--all`).
    #[arg(long)]
    selector: Option<String>,
    /// Confirm that the selected thread ids should be resolved.
    #[arg(long)]
    yes: bool,
    /// One or more GitHub review thread node ids to resolve.
    thread_ids: Vec<String>,
}

#[derive(Args)]
struct PrPreflightArgs {
    /// Base ref used to compute changed-scope checks in default mode.
    #[arg(long, default_value = "origin/main")]
    base: String,
    /// Run the broader explicit full preflight instead of changed-scope mode.
    #[arg(long)]
    full: bool,
}

fn main() -> Result<()> {
    // Ensure CWD is the repo root so that relative paths like "docs/",
    // "scripts/ensure_spdx.sh", and git-ls-files all work regardless of
    // where `cargo xtask` is invoked from.
    let repo_root = find_repo_root()?;
    std::env::set_current_dir(&repo_root)
        .with_context(|| format!("failed to chdir to {}", repo_root.display()))?;

    let cli = Cli::parse();

    match cli.command {
        Commands::Bench(args) => run_bench(args),
        Commands::Dags(args) => run_dags(args),
        Commands::Doghouse(args) => run_doghouse(args),
        Commands::PrStatus(args) => run_pr_status(args),
        Commands::PrSnapshot(args) => run_pr_snapshot(args),
        Commands::PrThreads(args) => run_pr_threads(args),
        Commands::PrPreflight(args) => run_pr_preflight(args),
        Commands::Dind(args) => run_dind(args),
        Commands::ManPages(args) => run_man_pages(args),
        Commands::LintDeadRefs(args) => run_lint_dead_refs(args),
        Commands::MarkdownFix(args) => run_markdown_fix(&args),
        Commands::DocsLint(args) => run_docs_lint(args),
        Commands::Method(args) => run_method(args),
        Commands::Wesley(args) => run_wesley(args),
        Commands::TestSlice(args) => run_test_slice(args),
    }
}

fn run_test_slice(args: TestSliceArgs) -> Result<()> {
    let commands = build_test_slice_commands(args.slice);
    println!("test-slice {:?}: {} command(s)", args.slice, commands.len());

    for mut command in commands {
        println!("  {}", display_command(&command));
        if args.dry_run {
            continue;
        }

        let status = command
            .status()
            .with_context(|| format!("failed to spawn {}", display_command(&command)))?;
        if !status.success() {
            bail!(
                "test-slice {:?} failed while running `{}` (exit status: {status})",
                args.slice,
                display_command(&command)
            );
        }
    }

    Ok(())
}

fn build_test_slice_commands(slice: TestSlice) -> Vec<Command> {
    match slice {
        TestSlice::Strand => vec![cargo_command([
            "test",
            "-p",
            "warp-core",
            "--test",
            "strand_contract_tests",
        ])],
        TestSlice::Settlement => vec![cargo_command([
            "test",
            "-p",
            "warp-core",
            "--lib",
            "settlement::tests",
        ])],
        TestSlice::Observation => vec![cargo_command([
            "test",
            "-p",
            "warp-core",
            "--lib",
            "observation::tests",
        ])],
        TestSlice::Neighborhood => vec![cargo_command([
            "test",
            "-p",
            "warp-core",
            "--lib",
            "neighborhood::tests",
        ])],
        TestSlice::WarpCoreSmoke => vec![
            cargo_command(["test", "-p", "warp-core", "--lib"]),
            cargo_command(["test", "-p", "warp-core", "--test", "strand_contract_tests"]),
        ],
    }
}

fn cargo_command<const N: usize>(args: [&str; N]) -> Command {
    let mut command = Command::new("cargo");
    command.args(args);
    command
}

fn display_command(command: &Command) -> String {
    let mut parts = Vec::new();
    parts.push(command.get_program().to_string_lossy().into_owned());
    parts.extend(
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned()),
    );
    parts.join(" ")
}

#[derive(Serialize)]
struct WesleySyncReport {
    ok: bool,
    canonical_schema: String,
    rust_schema_sha256: Option<String>,
    typescript_schema_sha256: Option<String>,
    checks: Vec<WesleySyncCheck>,
}

#[derive(Serialize)]
struct WesleySyncCheck {
    name: String,
    ok: bool,
    detail: String,
}

fn run_wesley(args: WesleyArgs) -> Result<()> {
    match args.command {
        WesleyCommand::Sync(sync_args) => run_wesley_sync(sync_args),
    }
}

fn run_wesley_sync(args: WesleySyncArgs) -> Result<()> {
    let repo_root = find_repo_root()?;
    let report = build_wesley_sync_report(&repo_root)?;

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report)
                .context("failed to serialize Wesley sync report")?
        );
    } else {
        print_wesley_sync_report(&report);
    }

    if report.ok {
        Ok(())
    } else {
        bail!("Wesley protocol consumer check failed")
    }
}

fn build_wesley_sync_report(repo_root: &Path) -> Result<WesleySyncReport> {
    let rust_cargo = read_repo_file(repo_root, "crates/ttd-protocol-rs/Cargo.toml")?;
    let rust_lib = read_repo_file(repo_root, "crates/ttd-protocol-rs/lib.rs")?;
    let ts_package = read_repo_file(repo_root, "packages/ttd-protocol-ts/package.json")?;
    let ts_index = read_repo_file(repo_root, "packages/ttd-protocol-ts/index.ts")?;
    let ts_types = read_repo_file(repo_root, "packages/ttd-protocol-ts/types.ts")?;
    let ts_registry = read_repo_file(repo_root, "packages/ttd-protocol-ts/registry.ts")?;
    let ts_zod = read_repo_file(repo_root, "packages/ttd-protocol-ts/zod.ts")?;
    let echo_ttd_cargo = read_repo_file(repo_root, "crates/echo-ttd/Cargo.toml")?;

    let rust_schema_sha256 = extract_assignment_string(&rust_lib, "SCHEMA_SHA256");
    let typescript_schema_sha256 = extract_assignment_string(&ts_registry, "SCHEMA_HASH");
    let mut checks = Vec::new();

    push_check(
        &mut checks,
        "local-ttd-schema-absent",
        !repo_root.join("schemas/ttd-protocol.graphql").exists(),
        "Echo must not carry a backup source-of-truth TTD protocol schema",
    );
    push_check(
        &mut checks,
        "rust-crate-canonical-owner",
        rust_cargo.contains("canonical warp-ttd protocol")
            && rust_cargo.contains("cargo xtask wesley sync"),
        "Rust consumer crate must name canonical warp-ttd ownership and the local check command",
    );
    push_check(
        &mut checks,
        "rust-lib-generated-marker",
        rust_lib.contains("Generated code") && rust_lib.contains("SCHEMA_SHA256"),
        "Rust lib.rs must remain a generated protocol artifact with schema identity",
    );
    push_check(
        &mut checks,
        "typescript-package-canonical-owner",
        ts_package.contains("canonical warp-ttd protocol") && ts_package.contains("DO NOT EDIT"),
        "TypeScript package must advertise downstream generated-consumer status",
    );
    push_check(
        &mut checks,
        "typescript-generated-markers",
        [&ts_index, &ts_types, &ts_registry, &ts_zod]
            .into_iter()
            .all(|content| content.contains("Auto-generated by @wesley/generator-ttd")),
        "TypeScript generated files must retain generator markers",
    );
    push_check(
        &mut checks,
        "schema-hash-match",
        rust_schema_sha256.is_some()
            && rust_schema_sha256 == typescript_schema_sha256
            && rust_schema_sha256.as_deref().is_some_and(is_sha256_hex),
        "Rust and TypeScript generated consumers must name the same 64-hex schema hash",
    );
    push_check(
        &mut checks,
        "echo-ttd-runtime-separate",
        !echo_ttd_cargo.contains("ttd-protocol-rs"),
        "Echo runtime-side compliance must not depend on host-neutral generated protocol nouns",
    );

    let ok = checks.iter().all(|check| check.ok);
    Ok(WesleySyncReport {
        ok,
        canonical_schema: "warp-ttd/schemas/warp-ttd-protocol.graphql".to_owned(),
        rust_schema_sha256,
        typescript_schema_sha256,
        checks,
    })
}

fn push_check(checks: &mut Vec<WesleySyncCheck>, name: &str, ok: bool, detail: &str) {
    checks.push(WesleySyncCheck {
        name: name.to_owned(),
        ok,
        detail: detail.to_owned(),
    });
}

fn print_wesley_sync_report(report: &WesleySyncReport) {
    println!(
        "Wesley protocol consumer check: {}",
        if report.ok { "ok" } else { "failed" }
    );
    println!("Canonical schema: {}", report.canonical_schema);
    println!(
        "Rust schema SHA-256: {}",
        report.rust_schema_sha256.as_deref().unwrap_or("<missing>")
    );
    println!(
        "TypeScript schema SHA-256: {}",
        report
            .typescript_schema_sha256
            .as_deref()
            .unwrap_or("<missing>")
    );
    for check in &report.checks {
        println!(
            "  {} {} — {}",
            if check.ok { "ok" } else { "FAIL" },
            check.name,
            check.detail
        );
    }
}

fn read_repo_file(repo_root: &Path, relative: &str) -> Result<String> {
    let path = repo_root.join(relative);
    std::fs::read_to_string(&path).with_context(|| format!("failed to read {relative}"))
}

fn extract_assignment_string(contents: &str, name: &str) -> Option<String> {
    contents
        .lines()
        .find(|line| line.contains(name))
        .and_then(|line| {
            let start = line.find('"').or_else(|| line.find('\''))?;
            let quote = line.as_bytes()[start];
            let rest = &line[start + 1..];
            let end = rest
                .as_bytes()
                .iter()
                .position(|candidate| *candidate == quote)?;
            Some(rest[..end].to_owned())
        })
}

fn is_sha256_hex(candidate: &str) -> bool {
    candidate.len() == 64 && candidate.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn run_method(args: MethodArgs) -> Result<()> {
    match args.command {
        MethodCommand::Inbox(inbox_args) => run_method_inbox(inbox_args),
        MethodCommand::Close(close_args) => run_method_close(close_args),
        MethodCommand::Pull(pull_args) => run_method_pull(pull_args),
        MethodCommand::Drift(drift_args) => run_method_drift(drift_args),
        MethodCommand::Status(status_args) => run_method_status(status_args),
        MethodCommand::Matrix(matrix_args) => run_method_matrix(matrix_args),
        MethodCommand::Dag(dag_args) => run_method_dag(dag_args),
        MethodCommand::Frontier(frontier_args) => run_method_frontier(frontier_args),
        MethodCommand::CriticalPath(path_args) => run_method_critical_path(path_args),
        MethodCommand::CheckDag(check_args) => run_method_check_dag(check_args),
    }
}

fn method_workspace() -> Result<method::workspace::MethodWorkspace> {
    let root = std::env::current_dir().context("failed to get current dir")?;
    method::workspace::MethodWorkspace::discover(&root).map_err(|e| anyhow::anyhow!(e))
}

fn run_method_inbox(args: MethodInboxArgs) -> Result<()> {
    let root = std::env::current_dir().context("failed to get current dir")?;
    let workspace = method_workspace()?;
    let path = method::inbox::create_inbox_item(&workspace, &args.title)
        .map_err(|e| anyhow::anyhow!(e))?;
    let display_path = path.strip_prefix(&root).unwrap_or(&path);
    println!("{}", display_path.display());
    Ok(())
}

fn run_method_close(args: MethodCloseArgs) -> Result<()> {
    let root = std::env::current_dir().context("failed to get current dir")?;
    let workspace = method_workspace()?;
    let result = method::close::close_cycle(&workspace, args.cycle.as_deref())
        .map_err(|e| anyhow::anyhow!(e))?;
    let retro_path = result
        .retro_path
        .strip_prefix(&root)
        .unwrap_or(&result.retro_path);
    let witness_dir = result
        .witness_dir
        .strip_prefix(&root)
        .unwrap_or(&result.witness_dir);

    println!("closed {}", result.cycle);
    println!("retro {}", retro_path.display());
    println!("witness {}", witness_dir.display());
    Ok(())
}

fn run_method_pull(args: MethodPullArgs) -> Result<()> {
    let root = std::env::current_dir().context("failed to get current dir")?;
    let workspace = method_workspace()?;
    let result =
        method::pull::pull_backlog_item(&workspace, &args.item).map_err(|e| anyhow::anyhow!(e))?;
    let design_path = result
        .design_path
        .strip_prefix(&root)
        .unwrap_or(&result.design_path);

    println!("pulled {}", result.cycle_number);
    println!("cycle {}", result.cycle);
    println!("design {}", design_path.display());
    Ok(())
}

fn run_method_drift(args: MethodDriftArgs) -> Result<()> {
    let workspace = method_workspace()?;
    let report = method::drift::drift_report(&workspace, args.cycle.as_deref())
        .map_err(|e| anyhow::anyhow!(e))?;

    if args.json {
        let json =
            serde_json::to_string_pretty(&report).context("failed to serialize drift report")?;
        println!("{json}");
    } else {
        print_drift_human(&report);
    }

    if !report.covered() {
        bail!(
            "METHOD drift check failed: {} playback question(s) lack matching tests",
            report.missing_count()
        );
    }
    Ok(())
}

fn run_method_status(args: MethodStatusArgs) -> Result<()> {
    let workspace = method_workspace()?;
    let report = method::status::StatusReport::build(&workspace).map_err(|e| anyhow::anyhow!(e))?;

    if args.json {
        let json =
            serde_json::to_string_pretty(&report).context("failed to serialize status report")?;
        println!("{json}");
    } else {
        print_status_human(&report);
    }
    Ok(())
}

fn print_drift_human(report: &method::drift::DriftReport) {
    println!("Drift check: {}", report.cycle);
    println!("  design files: {}", report.design_paths.len());
    println!("  playback questions: {}", report.questions.len());
    println!("  missing coverage: {}", report.missing_count());
    for question in &report.questions {
        let status = if question.matches.is_empty() {
            "MISS"
        } else {
            "ok"
        };
        println!("  {status} {}", question.question);
        for path in &question.matches {
            println!("      {}", path.display());
        }
    }
}

fn run_method_matrix(args: MethodMatrixArgs) -> Result<()> {
    let workspace = method_workspace()?;
    let graph = method::graph::TaskGraph::build(&workspace).map_err(|e| anyhow::anyhow!(e))?;
    let artifacts = method::graph::GraphArtifacts::render(&graph);
    let paths = method::graph::GraphArtifactPaths::defaults(&workspace);

    let checks = [
        (
            "matrix markdown",
            &paths.matrix_md,
            artifacts.matrix_md.as_bytes(),
        ),
        (
            "matrix csv",
            &paths.matrix_csv,
            artifacts.matrix_csv.as_bytes(),
        ),
    ];
    if args.check {
        check_artifacts_current(&checks)?;
        println!("METHOD matrix artifacts are current");
    } else {
        write_artifact(&paths.matrix_md, artifacts.matrix_md.as_bytes())?;
        write_artifact(&paths.matrix_csv, artifacts.matrix_csv.as_bytes())?;
        println!("wrote {}", paths.matrix_md.display());
        println!("wrote {}", paths.matrix_csv.display());
    }
    Ok(())
}

fn run_method_dag(args: MethodDagArgs) -> Result<()> {
    let workspace = method_workspace()?;
    let graph = method::graph::TaskGraph::build(&workspace).map_err(|e| anyhow::anyhow!(e))?;
    let artifacts = method::graph::GraphArtifacts::render(&graph);
    let paths = method::graph::GraphArtifactPaths::defaults(&workspace);

    if args.check {
        let rendered_svg = if args.no_render {
            None
        } else {
            Some(render_dot_to_svg(&artifacts.dot)?)
        };
        let mut checks = vec![("task dag dot", &paths.dot, artifacts.dot.as_bytes())];
        if let Some(svg) = rendered_svg.as_ref() {
            checks.push(("task dag svg", &paths.svg, svg.as_slice()));
        }
        check_artifacts_current(&checks)?;
        println!("METHOD DAG artifacts are current");
    } else {
        write_artifact(&paths.dot, artifacts.dot.as_bytes())?;
        println!("wrote {}", paths.dot.display());
        if !args.no_render {
            let svg = render_dot_to_svg(&artifacts.dot)?;
            write_artifact(&paths.svg, &svg)?;
            println!("wrote {}", paths.svg.display());
        }
    }
    Ok(())
}

fn run_method_frontier(args: MethodFrontierArgs) -> Result<()> {
    let workspace = method_workspace()?;
    let graph = method::graph::TaskGraph::build(&workspace).map_err(|e| anyhow::anyhow!(e))?;
    let frontier = graph.frontier();

    if args.json {
        let json =
            serde_json::to_string_pretty(&frontier).context("failed to serialize frontier")?;
        println!("{json}");
        return Ok(());
    }

    println!("Open frontier: {} task(s)", frontier.len());
    for task in frontier.into_iter().take(args.limit) {
        let native = task
            .task
            .native_id
            .as_ref()
            .map(|id| format!(" {id}"))
            .unwrap_or_default();
        println!(
            "  {} [{}]{} {}",
            task.task.id, task.task.lane, native, task.task.title
        );
        println!(
            "      unlocks: {}, downstream depth: {}, source: {}",
            task.downstream_count, task.downstream_depth, task.task.source_path
        );
    }
    Ok(())
}

fn run_method_critical_path(args: MethodCriticalPathArgs) -> Result<()> {
    let workspace = method_workspace()?;
    let graph = method::graph::TaskGraph::build(&workspace).map_err(|e| anyhow::anyhow!(e))?;
    let path = graph.critical_path();

    if args.json {
        let json =
            serde_json::to_string_pretty(&path).context("failed to serialize critical path")?;
        println!("{json}");
        return Ok(());
    }

    println!("Critical path: {} task(s)", path.len());
    for (idx, task) in path.iter().enumerate() {
        let native = task
            .native_id
            .as_ref()
            .map(|id| format!(" {id}"))
            .unwrap_or_default();
        println!(
            "  {}. {} [{}]{} {}",
            idx + 1,
            task.id,
            task.lane,
            native,
            task.title
        );
    }
    Ok(())
}

fn run_method_check_dag(args: MethodCheckDagArgs) -> Result<()> {
    run_method_matrix(MethodMatrixArgs { check: true })?;
    run_method_dag(MethodDagArgs {
        check: true,
        no_render: args.no_render,
    })
}

fn write_artifact(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    std::fs::write(path, bytes).with_context(|| format!("failed to write {}", path.display()))
}

fn check_artifacts_current(checks: &[(&str, &PathBuf, &[u8])]) -> Result<()> {
    let mut stale = Vec::new();
    for (label, path, expected) in checks {
        match std::fs::read(path) {
            Ok(actual) if actual == *expected => {}
            Ok(_) => stale.push(format!("{label}: {} is stale", path.display())),
            Err(err) => stale.push(format!("{label}: {} missing ({err})", path.display())),
        }
    }
    if stale.is_empty() {
        Ok(())
    } else {
        bail!(
            "METHOD graph artifacts are not current:\n{}",
            stale
                .into_iter()
                .map(|line| format!("  - {line}"))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}

fn render_dot_to_svg(dot: &str) -> Result<Vec<u8>> {
    use std::io::Write;

    let mut child = Command::new("dot")
        .arg("-Tsvg")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("failed to spawn `dot` (is Graphviz installed?)")?;

    {
        let stdin = child.stdin.as_mut().context("failed to open dot stdin")?;
        stdin
            .write_all(dot.as_bytes())
            .context("failed to write DOT to Graphviz")?;
    }

    let output = child
        .wait_with_output()
        .context("failed to wait for Graphviz")?;
    if !output.status.success() {
        bail!(
            "Graphviz failed (exit status: {}):\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(output.stdout)
}

fn print_status_human(report: &method::status::StatusReport) {
    println!("Backlog");
    for (lane, count) in &report.lanes {
        println!("  {lane}: {count}");
    }
    println!();

    println!("Active cycles");
    if report.active_cycles.is_empty() {
        println!("  (none)");
    } else {
        for cycle in &report.active_cycles {
            println!("  {}-{}", cycle.number, cycle.slug);
        }
    }
    println!();

    println!("Legend load");
    for (legend, count) in &report.legend_load {
        println!("  {legend}: {count}");
    }
    println!();

    println!("Total: {}", report.total_items);
}

fn run_bench(args: BenchArgs) -> Result<()> {
    match args.command {
        BenchCommand::Bake(args) => run_bench_bake(args),
        BenchCommand::PolicyExport(args) => run_bench_policy_export(args),
        BenchCommand::CheckArtifacts(args) => run_bench_check_artifacts(args),
    }
}

fn run_doghouse(args: DoghouseArgs) -> Result<()> {
    match args.command {
        DoghouseCommand::Sortie(args) => run_doghouse_sortie(args),
        DoghouseCommand::NudgeCoderabbit(args) => run_doghouse_nudge_coderabbit(args),
        DoghouseCommand::RearmCoderabbit(args) => run_doghouse_rearm_coderabbit(args),
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

fn run_pr_status(args: PrStatusArgs) -> Result<()> {
    let script = pr_status_script_path();
    let mut command = build_pr_status_command(&script, args.selector.as_deref());

    let status = command
        .status()
        .with_context(|| format!("failed to run {}", script.display()))?;
    if !status.success() {
        bail!("PR status command failed (exit status: {status})");
    }

    Ok(())
}

fn run_pr_snapshot(args: PrSnapshotArgs) -> Result<()> {
    let overview = fetch_pr_overview(args.selector.as_deref())?;
    let checks = fetch_pr_checks(&overview)?;
    let threads = fetch_unresolved_review_threads(&overview)?;
    let code_rabbit = fetch_code_rabbit_state(&overview)?;
    let snapshot =
        build_pr_snapshot_artifact(&overview, &checks, &threads, args.intent, code_rabbit)?;
    let previous_snapshot = load_latest_pr_snapshot(snapshot.pr.number, &args.out_dir)?;
    let delta = previous_snapshot
        .as_ref()
        .map(|previous| build_pr_snapshot_delta(previous, &snapshot));
    let paths = write_pr_snapshot_artifact(&snapshot, &args.out_dir)?;
    let delta_paths = if let Some(delta) = delta.as_ref() {
        Some(write_pr_snapshot_delta_artifact(
            delta,
            snapshot.pr.number,
            &snapshot.filename_stamp,
            &snapshot.pr.head_sha_short,
            &args.out_dir,
        )?)
    } else {
        None
    };

    println!(
        "Doghouse flight recorder captured PR #{}.",
        snapshot.pr.number
    );
    println!("Title: {}", snapshot.pr.title);
    println!("Head SHA: {}", snapshot.pr.head_sha_short);
    println!("Recorded at: {}", snapshot.recorded_at);
    println!(
        "Sortie intent: {}",
        snapshot
            .sortie_intent
            .as_ref()
            .map_or_else(|| "unknown".to_owned(), doghouse_sortie_intent_label)
    );
    println!("Current blockers:");
    if snapshot.blockers.is_empty() {
        println!("- none detected");
    } else {
        for blocker in &snapshot.blockers {
            println!("- {blocker}");
        }
    }
    println!("Snapshot JSON: {}", paths.snapshot_json.display());
    println!("Snapshot Markdown: {}", paths.snapshot_markdown.display());
    println!("Latest JSON: {}", paths.latest_json.display());
    println!("Latest Markdown: {}", paths.latest_markdown.display());
    if let Some(delta) = delta.as_ref() {
        print_pr_snapshot_delta_summary(delta);
    } else {
        println!("Previous snapshot: none detected");
    }
    if let Some(delta_paths) = delta_paths.as_ref() {
        println!("Delta JSON: {}", delta_paths.snapshot_json.display());
        println!(
            "Delta Markdown: {}",
            delta_paths.snapshot_markdown.display()
        );
        println!("Latest Delta JSON: {}", delta_paths.latest_json.display());
        println!(
            "Latest Delta Markdown: {}",
            delta_paths.latest_markdown.display()
        );
    }

    Ok(())
}

fn run_doghouse_sortie(args: DoghouseSortieArgs) -> Result<()> {
    let overview = fetch_pr_overview(args.selector.as_deref())?;
    let checks = fetch_pr_checks(&overview)?;
    let threads = fetch_unresolved_review_threads(&overview)?;
    let code_rabbit = fetch_code_rabbit_state(&overview)?;
    let snapshot =
        build_pr_snapshot_artifact(&overview, &checks, &threads, args.intent, code_rabbit)?;
    let prior_snapshots = load_prior_pr_snapshots(snapshot.pr.number, &args.out_dir)?;
    let baseline = select_doghouse_baseline(&prior_snapshots, &snapshot);
    let delta = baseline
        .as_ref()
        .map(|selection| build_pr_snapshot_delta(&selection.snapshot, &snapshot));
    let comparison = assess_doghouse_comparison(&snapshot, baseline.as_ref(), delta.as_ref());
    let snapshot_paths = write_pr_snapshot_artifact(&snapshot, &args.out_dir)?;
    let delta_paths = if let Some(delta) = delta.as_ref() {
        Some(write_pr_snapshot_delta_artifact(
            delta,
            snapshot.pr.number,
            &snapshot.filename_stamp,
            &snapshot.pr.head_sha_short,
            &args.out_dir,
        )?)
    } else {
        None
    };
    let next_action = determine_doghouse_next_action(&snapshot, &comparison, delta.as_ref());
    let event_lines = build_doghouse_sortie_events(
        &snapshot,
        baseline.as_ref(),
        &comparison,
        delta.as_ref(),
        &next_action,
    )?;
    let jsonl_paths = write_doghouse_jsonl_events(
        &event_lines,
        snapshot.pr.number,
        &snapshot.filename_stamp,
        &snapshot.pr.head_sha_short,
        &args.out_dir,
    )?;

    for line in event_lines {
        println!("{line}");
    }
    println!(
        "{}",
        serde_json::to_string(&DoghouseArtifactsEvent {
            kind: "doghouse.artifacts",
            snapshot_json: snapshot_paths.snapshot_json.display().to_string(),
            snapshot_markdown: snapshot_paths.snapshot_markdown.display().to_string(),
            latest_json: snapshot_paths.latest_json.display().to_string(),
            latest_markdown: snapshot_paths.latest_markdown.display().to_string(),
            delta_json: delta_paths
                .as_ref()
                .map(|paths| paths.snapshot_json.display().to_string()),
            delta_markdown: delta_paths
                .as_ref()
                .map(|paths| paths.snapshot_markdown.display().to_string()),
            latest_delta_json: delta_paths
                .as_ref()
                .map(|paths| paths.latest_json.display().to_string()),
            latest_delta_markdown: delta_paths
                .as_ref()
                .map(|paths| paths.latest_markdown.display().to_string()),
            sortie_jsonl: jsonl_paths.snapshot_jsonl.display().to_string(),
            latest_sortie_jsonl: jsonl_paths.latest_jsonl.display().to_string(),
        })
        .context("failed to serialize doghouse artifacts event")?
    );

    Ok(())
}

fn run_doghouse_rearm_coderabbit(args: DoghouseRearmCoderabbitArgs) -> Result<()> {
    let overview = fetch_pr_overview(args.selector.as_deref())?;
    let Some(comment) = fetch_latest_code_rabbit_summary_comment(&overview)? else {
        bail!(
            "No CodeRabbit summary comment was found for PR #{}.",
            overview.number
        );
    };
    let Some(state) = analyze_code_rabbit_summary_comment(comment.clone()) else {
        bail!(
            "The latest CodeRabbit summary comment on PR #{} does not expose a recognizable state.",
            overview.number
        );
    };

    if !state.rearm_actionable {
        bail!(
            "CodeRabbit is not currently blocked behind a checkbox rearm on PR #{}.",
            overview.number
        );
    }
    if !args.yes {
        bail!(
            "Refusing to edit the CodeRabbit summary comment without --yes for PR #{}.",
            overview.number
        );
    }

    let Some(database_id) = state.summary_comment_database_id else {
        bail!(
            "CodeRabbit summary comment on PR #{} does not expose a database id.",
            overview.number
        );
    };
    let Some((updated_body, toggled_checkbox_count)) = build_code_rabbit_rearm_body(&comment.body)
    else {
        bail!(
            "CodeRabbit summary comment on PR #{} did not contain any actionable unchecked checkboxes.",
            overview.number
        );
    };

    let route = format!(
        "repos/{}/{}/issues/comments/{}",
        overview.owner, overview.repo, database_id
    );
    let _ = run_gh_capture([
        "api",
        &route,
        "--method",
        "PATCH",
        "-f",
        &format!("body={updated_body}"),
    ])?;

    println!(
        "{}",
        serde_json::to_string(&DoghouseCodeRabbitRearmEvent {
            kind: "doghouse.coderabbit_rearm",
            pr_number: overview.number,
            summary_comment_url: state.summary_comment_url,
            summary_comment_database_id: Some(database_id),
            toggled_checkbox_count,
            updated: true,
        })
        .context("failed to serialize doghouse CodeRabbit rearm event")?
    );

    Ok(())
}

fn run_doghouse_nudge_coderabbit(args: DoghouseNudgeCoderabbitArgs) -> Result<()> {
    let overview = fetch_pr_overview(args.selector.as_deref())?;
    let checks = fetch_pr_checks(&overview)?;
    if checks
        .iter()
        .any(|check| check.name.eq_ignore_ascii_case("CodeRabbit") && check.bucket == "pending")
    {
        bail!(
            "CodeRabbit is already actively reviewing PR #{}.",
            overview.number
        );
    }

    let comment = fetch_latest_code_rabbit_summary_comment(&overview)?;
    let analyzed_state = comment
        .as_ref()
        .and_then(|summary| analyze_code_rabbit_summary_comment(summary.clone()));

    if let Some(state) = analyzed_state.as_ref() {
        if state.rearm_actionable {
            bail!(
                "CodeRabbit on PR #{} is paused behind a checkbox rearm; run `cargo xtask doghouse rearm-coderabbit {} --yes` first.",
                overview.number,
                overview.number
            );
        }
        if state.cooldown_active {
            bail!(
                "CodeRabbit on PR #{} is still cooling down until {}.",
                overview.number,
                state.cooldown_expires_at.as_deref().unwrap_or("<unknown>")
            );
        }
        if !state.request_review_actionable {
            bail!(
                "CodeRabbit is not currently actionable for a manual nudge on PR #{}.",
                overview.number
            );
        }
    }

    if !args.yes {
        bail!(
            "Refusing to post a CodeRabbit nudge without --yes for PR #{}.",
            overview.number
        );
    }

    let nudge_body = select_code_rabbit_nudge_body(comment.as_ref(), analyzed_state.as_ref());
    let route = format!(
        "repos/{}/{}/issues/{}/comments",
        overview.owner, overview.repo, overview.number
    );
    let output = run_gh_capture([
        "api",
        &route,
        "--method",
        "POST",
        "-f",
        &format!("body={nudge_body}"),
    ])?;
    let posted_comment: GhIssueComment =
        serde_json::from_str(&output).context("failed to parse CodeRabbit nudge comment")?;

    println!(
        "{}",
        serde_json::to_string(&DoghouseCodeRabbitNudgeEvent {
            kind: "doghouse.coderabbit_nudge",
            pr_number: overview.number,
            command: nudge_body.to_owned(),
            posted_comment_url: posted_comment.html_url,
            posted_comment_database_id: Some(posted_comment.id),
            summary_comment_url: analyzed_state
                .as_ref()
                .map(|state| state.summary_comment_url.clone()),
            summary_comment_database_id: analyzed_state
                .as_ref()
                .and_then(|state| state.summary_comment_database_id),
        })
        .context("failed to serialize doghouse CodeRabbit nudge event")?
    );

    Ok(())
}

fn run_pr_threads(args: PrThreadsArgs) -> Result<()> {
    match args.command {
        PrThreadsCommand::List { selector } => run_pr_threads_list(selector.as_deref()),
        PrThreadsCommand::Reply(args) => run_pr_threads_reply(args),
        PrThreadsCommand::Resolve(args) => run_pr_threads_resolve(args),
    }
}

fn run_pr_preflight(args: PrPreflightArgs) -> Result<()> {
    let changed_files = if args.full {
        Vec::new()
    } else {
        collect_pr_preflight_changed_files(&args.base)?
    };
    let mut plan = build_pr_preflight_plan(&changed_files, args.full);

    println!(
        "PR preflight mode: {}",
        if args.full { "full" } else { "changed-scope" }
    );
    if !args.full {
        println!("Base ref: {}", args.base);
        println!("Changed files: {}", changed_files.len());
    }
    println!("Checks:");
    for check in &plan {
        println!("- {}", check.label);
    }

    let mut failures = Vec::new();
    for check in &mut plan {
        println!("\n==> {}", check.label);
        let status = check
            .command
            .status()
            .with_context(|| format!("failed to start {}", check.label))?;
        if status.success() {
            println!("OK: {}", check.label);
            continue;
        }
        eprintln!("FAIL: {} (exit status: {status})", check.label);
        failures.push(check.label.clone());
    }

    if failures.is_empty() {
        println!("\nPR preflight passed ({} checks).", plan.len());
        return Ok(());
    }

    eprintln!("\nPR preflight failed ({} checks):", failures.len());
    for failure in &failures {
        eprintln!("- {failure}");
    }
    bail!("fix the failing preflight checks before opening the PR");
}

const BENCH_CORE_GROUP_KEYS: &[&str] = &[
    "snapshot_hash",
    "scheduler_drain",
    "scheduler_drain/enqueue",
    "scheduler_drain/drain",
];
const BENCH_CORE_INPUTS: &[u32] = &[10, 100, 1000, 3000, 10000, 30000];
const BENCH_POLICY_GROUP: &str = "parallel_policy_matrix";
const BENCH_INLINE_DATA_MARKER: &str = "<script>\n            const GROUPS = [";
const BENCH_OPEN_PROPS_INLINE_MARKER: &str = r#"data-bench-inline="open-props""#;
const BENCH_NORMALIZE_DARK_INLINE_MARKER: &str = r#"data-bench-inline="normalize-dark""#;

#[derive(Clone, Debug, Serialize)]
struct CoreBenchRow {
    group: String,
    n: u32,
    mean: f64,
    lb: Option<f64>,
    ub: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
struct MissingBenchRow {
    group: String,
    n: u32,
    path: String,
    error: String,
}

#[derive(Clone, Debug)]
struct CriterionEstimate {
    path: String,
    mean: f64,
    lb: Option<f64>,
    ub: Option<f64>,
    modified_unix_nanos: u128,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct BenchMachineDescriptor {
    os: String,
    arch: String,
    hostname: Option<String>,
    label: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct PolicyMatrixPayload {
    group: String,
    #[serde(alias = "generated_at")]
    baked_at: Option<String>,
    #[serde(alias = "git_sha")]
    baked_git_sha: Option<String>,
    #[serde(alias = "source_digest")]
    baked_source_digest: Option<String>,
    template_path: Option<String>,
    machine: Option<BenchMachineDescriptor>,
    criterion_root: Option<String>,
    results: Vec<PolicyMatrixRow>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct PolicyMatrixRow {
    policy: String,
    workers: String,
    worker_hint: Option<String>,
    selected_policy: Option<String>,
    selected_workers: Option<String>,
    selected_series: Option<String>,
    load: u32,
    path: String,
    mean_ns: f64,
    lb_ns: Option<f64>,
    ub_ns: Option<f64>,
    series: String,
}

#[derive(Clone, Debug, PartialEq)]
struct ParsedPolicyCase {
    policy: String,
    workers: String,
    worker_hint: Option<String>,
    selected_policy: Option<String>,
    selected_workers: Option<String>,
    load: u32,
}

fn run_bench_bake(args: BenchBakeArgs) -> Result<()> {
    let criterion_root = args.criterion_root;
    let template_path = args.template;
    let report_out = args.out;
    let policy_json_out = args.policy_json_out;
    let repo_root = find_repo_root()?;

    let (core_data, core_missing) = collect_core_benchmark_rows(&criterion_root, &repo_root);
    let policy_criterion_root = criterion_root.join(BENCH_POLICY_GROUP);
    let policy_results = collect_policy_matrix_rows(&policy_criterion_root, &repo_root)?;
    let baked_source_digest = Some(compute_benchmark_artifact_source_digest(
        &repo_root,
        &template_path,
        &core_data,
        &core_missing,
        &policy_results,
    )?);
    let policy_payload = build_policy_matrix_payload(
        &policy_criterion_root,
        &repo_root,
        policy_results,
        Some(&template_path),
        baked_source_digest,
    )?;
    write_policy_matrix_payload(&policy_payload, &policy_json_out)?;

    let template = std::fs::read_to_string(&template_path)
        .with_context(|| format!("failed to read {}", template_path.display()))?;
    let baked_html = bake_benchmark_report(
        &template,
        &core_data,
        &core_missing,
        &policy_payload,
        &repo_root,
    )?;

    if let Some(parent) = report_out.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    std::fs::write(&report_out, baked_html)
        .with_context(|| format!("failed to write {}", report_out.display()))?;

    println!(
        "[bench-bake] Wrote {}",
        display_repo_relative(&report_out, &repo_root)
    );
    println!(
        "[bench-bake] Refreshed {}",
        display_repo_relative(&policy_json_out, &repo_root)
    );
    Ok(())
}

fn run_bench_policy_export(args: BenchPolicyExportArgs) -> Result<()> {
    let repo_root = find_repo_root()?;
    let default_template = PathBuf::from("docs/benchmarks/index.html");
    let core_criterion_root = args
        .criterion_root
        .parent()
        .map_or_else(|| args.criterion_root.clone(), Path::to_path_buf);
    let (core_data, core_missing) = collect_core_benchmark_rows(&core_criterion_root, &repo_root);
    let policy_results = collect_policy_matrix_rows(&args.criterion_root, &repo_root)?;
    let baked_source_digest = Some(compute_benchmark_artifact_source_digest(
        &repo_root,
        &default_template,
        &core_data,
        &core_missing,
        &policy_results,
    )?);
    let payload = build_policy_matrix_payload(
        &args.criterion_root,
        &repo_root,
        policy_results,
        Some(&default_template),
        baked_source_digest,
    )?;
    write_policy_matrix_payload(&payload, &args.json_out)?;
    println!(
        "[bench-policy-export] Wrote {}",
        display_repo_relative(&args.json_out, &repo_root)
    );
    Ok(())
}

fn run_bench_check_artifacts(args: BenchCheckArtifactsArgs) -> Result<()> {
    let repo_root = find_repo_root()?;
    let payload_json = std::fs::read_to_string(&args.json)
        .with_context(|| format!("failed to read {}", args.json.display()))?;
    let payload: PolicyMatrixPayload = serde_json::from_str(&payload_json)
        .with_context(|| format!("failed to parse {}", args.json.display()))?;
    let baked_source_digest = payload.baked_source_digest.as_deref().ok_or_else(|| {
        anyhow::anyhow!(
            "{} is missing `baked_source_digest` metadata",
            args.json.display()
        )
    })?;
    let criterion_root = policy_matrix_criterion_root_from_payload(&payload, &repo_root);
    let core_criterion_root = criterion_root
        .parent()
        .map_or_else(|| repo_root.join("target/criterion"), Path::to_path_buf);
    let (core_data, core_missing) = collect_core_benchmark_rows(&core_criterion_root, &repo_root);
    let current_results = collect_policy_matrix_rows(&criterion_root, &repo_root)?;
    if payload.results != current_results {
        bail!(
            "benchmark payload results are stale: {} no longer matches current Criterion estimates",
            args.json.display()
        );
    }

    let template_path = benchmark_template_path_from_payload(&payload, &repo_root);
    let expected_template_path = display_repo_relative(&template_path, &repo_root);
    if payload.template_path.as_deref() != Some(expected_template_path.as_str()) {
        bail!(
            "benchmark payload template_path is stale: {} says {:?}, expected {}",
            args.json.display(),
            payload.template_path,
            expected_template_path
        );
    }

    let expected_criterion_root = display_repo_relative(&criterion_root, &repo_root);
    if payload.criterion_root.as_deref() != Some(expected_criterion_root.as_str()) {
        bail!(
            "benchmark payload criterion_root is stale: {} says {:?}, expected {}",
            args.json.display(),
            payload.criterion_root,
            expected_criterion_root
        );
    }

    let current_source_digest = compute_benchmark_artifact_source_digest(
        &repo_root,
        &template_path,
        &core_data,
        &core_missing,
        &current_results,
    )
    .with_context(|| {
        format!(
            "failed to compute current benchmark input digest for {}",
            args.json.display()
        )
    })?;
    if baked_source_digest != current_source_digest {
        bail!(
            "benchmark payload is stale: {} says baked_source_digest={}, but current inputs hash to {}",
            args.json.display(),
            baked_source_digest,
            current_source_digest
        );
    }

    let html = std::fs::read_to_string(&args.html)
        .with_context(|| format!("failed to read {}", args.html.display()))?;
    let expected_inline = build_benchmark_inline_script(&core_data, &core_missing, &payload)
        .context("failed to build expected inline benchmark payload")?;
    if !html.contains(&expected_inline) {
        bail!(
            "baked report is stale: {} does not contain the expected inline benchmark payload",
            args.html.display(),
        );
    }

    println!(
        "[bench-check-artifacts] Artifacts match current benchmark inputs ({current_source_digest})"
    );
    Ok(())
}

fn collect_core_benchmark_rows(
    criterion_root: &Path,
    repo_root: &Path,
) -> (Vec<CoreBenchRow>, Vec<MissingBenchRow>) {
    let mut data = Vec::new();
    let mut missing = Vec::new();

    for group in BENCH_CORE_GROUP_KEYS {
        for &n in BENCH_CORE_INPUTS {
            let bench_dir = criterion_root.join(group).join(n.to_string());
            match load_criterion_estimate(&bench_dir, repo_root) {
                Ok(estimate) => data.push(CoreBenchRow {
                    group: (*group).to_owned(),
                    n,
                    mean: estimate.mean,
                    lb: estimate.lb,
                    ub: estimate.ub,
                }),
                Err((path, error)) => missing.push(MissingBenchRow {
                    group: (*group).to_owned(),
                    n,
                    path,
                    error,
                }),
            }
        }
    }

    (data, missing)
}

fn build_policy_matrix_payload(
    criterion_root: &Path,
    repo_root: &Path,
    results: Vec<PolicyMatrixRow>,
    template_path: Option<&Path>,
    baked_source_digest: Option<String>,
) -> Result<PolicyMatrixPayload> {
    let baked_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .context("failed to format benchmark payload timestamp")?;
    let baked_git_sha = Some(git_short_head_sha()?);
    let machine = Some(local_benchmark_machine_descriptor());
    let criterion_root_display = Some(display_repo_relative(criterion_root, repo_root));
    let template_path_display = template_path.map(|path| display_repo_relative(path, repo_root));

    Ok(PolicyMatrixPayload {
        group: BENCH_POLICY_GROUP.to_owned(),
        baked_at: Some(baked_at),
        baked_git_sha,
        baked_source_digest,
        template_path: template_path_display,
        machine,
        criterion_root: criterion_root_display,
        results,
    })
}

fn policy_matrix_criterion_root_from_payload(
    payload: &PolicyMatrixPayload,
    repo_root: &Path,
) -> PathBuf {
    match payload.criterion_root.as_deref() {
        Some(path) => repo_root.join(path),
        None => repo_root.join("target/criterion").join(BENCH_POLICY_GROUP),
    }
}

fn benchmark_template_path_from_payload(
    payload: &PolicyMatrixPayload,
    repo_root: &Path,
) -> PathBuf {
    match payload.template_path.as_deref() {
        Some(path) => repo_root.join(path),
        None => repo_root.join("docs/benchmarks/index.html"),
    }
}

fn compute_benchmark_artifact_source_digest(
    repo_root: &Path,
    template_path: &Path,
    core_data: &[CoreBenchRow],
    core_missing: &[MissingBenchRow],
    policy_results: &[PolicyMatrixRow],
) -> Result<String> {
    let mut inputs = Vec::new();
    for relative in [
        "xtask/src/main.rs",
        "crates/warp-benches/benches/parallel_baseline.rs",
        "crates/warp-core/src/parallel/exec.rs",
        "docs/benchmarks/vendor/open-props.min.css",
        "docs/benchmarks/vendor/normalize.dark.min.css",
    ] {
        let path = repo_root.join(relative);
        let bytes = std::fs::read(&path)
            .with_context(|| format!("failed to read benchmark input {}", path.display()))?;
        inputs.push((relative.to_owned(), bytes));
    }

    let template_bytes = std::fs::read(template_path).with_context(|| {
        format!(
            "failed to read benchmark template {}",
            template_path.display()
        )
    })?;
    inputs.push((
        display_repo_relative(template_path, repo_root),
        template_bytes,
    ));

    inputs.push((
        "derived/core_data.json".to_owned(),
        serde_json::to_vec(core_data)
            .context("failed to serialize core benchmark rows for digest")?,
    ));
    inputs.push((
        "derived/core_missing.json".to_owned(),
        serde_json::to_vec(core_missing)
            .context("failed to serialize missing core benchmark rows for digest")?,
    ));
    inputs.push((
        "derived/policy_results.json".to_owned(),
        serde_json::to_vec(policy_results)
            .context("failed to serialize policy benchmark rows for digest")?,
    ));

    inputs.sort_by(|left, right| left.0.cmp(&right.0));
    let mut hasher = Sha256::new();
    for (label, bytes) in inputs {
        hasher.update(label.as_bytes());
        hasher.update([0_u8]);
        hasher.update(&bytes);
        hasher.update([0xff_u8]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn collect_policy_matrix_rows(
    criterion_root: &Path,
    repo_root: &Path,
) -> Result<Vec<PolicyMatrixRow>> {
    if !criterion_root.is_dir() {
        return Ok(Vec::new());
    }

    let mut by_case: BTreeMap<(String, String, u32), (PolicyMatrixRow, u128)> = BTreeMap::new();
    let mut stack = vec![criterion_root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path.clone());
            }
            if !path.is_dir() {
                continue;
            }
            let Ok(relative_dir) = path.strip_prefix(criterion_root) else {
                continue;
            };
            let case = match parse_policy_case(relative_dir).map_err(|err| {
                anyhow::anyhow!(
                    "invalid policy matrix benchmark case `{}` under {}: {err}",
                    relative_dir.display(),
                    criterion_root.display()
                )
            })? {
                Some(case) => case,
                None => continue,
            };
            let Ok(estimate) = load_criterion_estimate(&path, repo_root) else {
                continue;
            };
            let row = PolicyMatrixRow {
                policy: case.policy.clone(),
                workers: case.workers.clone(),
                worker_hint: case.worker_hint.clone(),
                selected_policy: case.selected_policy.clone(),
                selected_workers: case.selected_workers.clone(),
                selected_series: case
                    .selected_policy
                    .as_ref()
                    .zip(case.selected_workers.as_ref())
                    .map(|(policy, workers)| format!("{policy}:{workers}")),
                load: case.load,
                path: estimate.path,
                mean_ns: estimate.mean,
                lb_ns: estimate.lb,
                ub_ns: estimate.ub,
                series: format!("{}:{}", case.policy, case.workers),
            };
            insert_policy_matrix_row(
                &mut by_case,
                criterion_root,
                row,
                estimate.modified_unix_nanos,
            );
        }
    }

    let mut results: Vec<_> = by_case.into_values().map(|(row, _)| row).collect();
    results.sort_by(|left, right| {
        left.workers
            .cmp(&right.workers)
            .then_with(|| left.policy.cmp(&right.policy))
            .then_with(|| left.load.cmp(&right.load))
    });
    Ok(results)
}

fn insert_policy_matrix_row(
    by_case: &mut BTreeMap<(String, String, u32), (PolicyMatrixRow, u128)>,
    criterion_root: &Path,
    row: PolicyMatrixRow,
    modified_unix_nanos: u128,
) {
    let key = (row.policy.clone(), row.workers.clone(), row.load);
    match by_case.entry(key) {
        std::collections::btree_map::Entry::Vacant(entry) => {
            entry.insert((row, modified_unix_nanos));
        }
        std::collections::btree_map::Entry::Occupied(mut entry) => {
            let (existing_row, existing_mtime) = entry.get();
            assert!(
                !policy_matrix_rows_conflict(existing_row, &row),
                "conflicting benchmark rows for {}:{} load {}: {} vs {}; clean {} before rebaking",
                row.policy,
                row.workers,
                row.load,
                existing_row.path,
                row.path,
                criterion_root.display(),
            );
            if prefer_policy_matrix_row(existing_row, *existing_mtime, &row, modified_unix_nanos) {
                entry.insert((row, modified_unix_nanos));
            }
        }
    }
}

fn policy_matrix_rows_conflict(left: &PolicyMatrixRow, right: &PolicyMatrixRow) -> bool {
    left.selected_policy.is_some()
        && left.selected_workers.is_some()
        && right.selected_policy.is_some()
        && right.selected_workers.is_some()
        && (left.selected_policy != right.selected_policy
            || left.selected_workers != right.selected_workers)
}

fn prefer_policy_matrix_row(
    existing: &PolicyMatrixRow,
    existing_modified_unix_nanos: u128,
    candidate: &PolicyMatrixRow,
    candidate_modified_unix_nanos: u128,
) -> bool {
    (
        candidate.selected_policy.is_some(),
        candidate_modified_unix_nanos,
        candidate.path.as_str(),
    ) > (
        existing.selected_policy.is_some(),
        existing_modified_unix_nanos,
        existing.path.as_str(),
    )
}

fn parse_policy_case(relative_dir: &Path) -> Result<Option<ParsedPolicyCase>, String> {
    let parts: Vec<String> = relative_dir
        .iter()
        .map(|part| part.to_string_lossy().into_owned())
        .collect();
    match parts.as_slice() {
        [policy_case, load] => {
            let Ok(load) = load.parse() else {
                return Ok(None);
            };
            if policy_case.starts_with("adaptive_shard_routing__hint_") {
                return parse_adaptive_policy_case(policy_case, load).map(Some);
            }
            if let Some((policy, workers)) = split_policy_case(policy_case)? {
                Ok(Some(ParsedPolicyCase {
                    policy,
                    workers,
                    worker_hint: None,
                    selected_policy: None,
                    selected_workers: None,
                    load,
                }))
            } else if policy_case.ends_with('w')
                && (policy_case.contains('_') || policy_case.contains('-'))
            {
                Err(format!(
                    "policy case `{policy_case}` ends with a worker suffix but does not match `<policy>_<workers>` or `<policy>-<workers>`"
                ))
            } else {
                Ok(Some(ParsedPolicyCase {
                    policy: policy_case.clone(),
                    workers: "dedicated".to_owned(),
                    worker_hint: None,
                    selected_policy: None,
                    selected_workers: None,
                    load,
                }))
            }
        }
        [policy, workers, load] => {
            let Ok(load) = load.parse() else {
                return Ok(None);
            };
            let workers = parse_worker_suffix(workers).ok_or_else(|| {
                format!(
                    "worker segment `{workers}` in benchmark case `{}` must be `<positive integer>w` without leading zeros",
                    relative_dir.display()
                )
            })?;
            Ok(Some(ParsedPolicyCase {
                policy: policy.clone(),
                workers,
                worker_hint: None,
                selected_policy: None,
                selected_workers: None,
                load,
            }))
        }
        _ => Ok(None),
    }
}

fn parse_adaptive_policy_case(policy_case: &str, load: u32) -> Result<ParsedPolicyCase, String> {
    let rest = policy_case
        .strip_prefix("adaptive_shard_routing__hint_")
        .ok_or_else(|| {
            format!("adaptive benchmark case `{policy_case}` is missing the expected hint prefix")
        })?;
    let (worker_hint, selected_case) = rest.split_once("__selected_").ok_or_else(|| {
        format!("adaptive benchmark case `{policy_case}` is missing the `__selected_...` segment")
    })?;
    let worker_hint = parse_worker_suffix(worker_hint).ok_or_else(|| {
        format!(
            "adaptive benchmark case `{policy_case}` has invalid worker hint `{worker_hint}`; expected `<positive integer>w` without leading zeros"
        )
    })?;
    let (selected_policy, selected_workers) = split_policy_case(selected_case)?.ok_or_else(|| {
        format!(
            "adaptive benchmark case `{policy_case}` has invalid selected plan `{selected_case}`; expected `<policy>_<workers>` or `<policy>-<workers>`"
        )
    })?;
    let selected_workers = parse_worker_suffix(&selected_workers).ok_or_else(|| {
        format!(
            "adaptive benchmark case `{policy_case}` has invalid selected worker suffix `{selected_workers}`; expected `<positive integer>w` without leading zeros"
        )
    })?;

    Ok(ParsedPolicyCase {
        policy: "adaptive_shard_routing".to_owned(),
        workers: worker_hint.clone(),
        worker_hint: Some(worker_hint),
        selected_policy: Some(selected_policy),
        selected_workers: Some(selected_workers),
        load,
    })
}

fn parse_worker_suffix(label: &str) -> Option<String> {
    let digits = label.strip_suffix('w')?;
    if digits.is_empty() || digits.starts_with('0') || !digits.chars().all(|ch| ch.is_ascii_digit())
    {
        return None;
    }
    Some(label.to_owned())
}

fn split_policy_case(policy_case: &str) -> Result<Option<(String, String)>, String> {
    for separator in ['_', '-'] {
        let Some((policy, workers)) = policy_case.rsplit_once(separator) else {
            continue;
        };
        if !workers.ends_with('w') {
            continue;
        }
        let Some(workers) = parse_worker_suffix(workers) else {
            return Err(format!(
                "policy case `{policy_case}` has invalid worker suffix `{workers}`; expected `<positive integer>w` without leading zeros"
            ));
        };
        return Ok(Some((policy.to_owned(), workers)));
    }
    Ok(None)
}

fn load_criterion_estimate(
    bench_dir: &Path,
    repo_root: &Path,
) -> Result<CriterionEstimate, (String, String)> {
    let candidate_paths =
        ["new", "base", "change"].map(|kind| bench_dir.join(kind).join("estimates.json"));
    for path in &candidate_paths {
        if !path.exists() {
            continue;
        }
        let contents = std::fs::read_to_string(path).map_err(|err| {
            (
                display_repo_relative(path, repo_root),
                format!("read error: {err}"),
            )
        })?;
        let value: serde_json::Value = serde_json::from_str(&contents).map_err(|err| {
            (
                display_repo_relative(path, repo_root),
                format!("parse error: {err}"),
            )
        })?;

        let mean = estimate_number(&value, &["mean", "point_estimate"])
            .or_else(|| estimate_number(&value, &["Mean", "point_estimate"]))
            .ok_or_else(|| {
                (
                    display_repo_relative(path, repo_root),
                    "missing mean.point_estimate".to_owned(),
                )
            })?;
        let lb = estimate_number(&value, &["mean", "confidence_interval", "lower_bound"])
            .or_else(|| estimate_number(&value, &["Mean", "confidence_interval", "lower_bound"]));
        let ub = estimate_number(&value, &["mean", "confidence_interval", "upper_bound"])
            .or_else(|| estimate_number(&value, &["Mean", "confidence_interval", "upper_bound"]));

        return Ok(CriterionEstimate {
            path: display_repo_relative(path, repo_root),
            mean,
            lb,
            ub,
            modified_unix_nanos: path
                .metadata()
                .ok()
                .and_then(|metadata| metadata.modified().ok())
                .and_then(|modified| modified.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |duration| duration.as_nanos()),
        });
    }

    Err((
        display_repo_relative(&candidate_paths[0], repo_root),
        "not found (tried new/base/change)".to_owned(),
    ))
}

fn estimate_number(value: &serde_json::Value, path: &[&str]) -> Option<f64> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    current.as_f64()
}

fn write_policy_matrix_payload(payload: &PolicyMatrixPayload, out_path: &Path) -> Result<()> {
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(payload)
        .context("failed to serialize parallel policy matrix payload")?;
    std::fs::write(out_path, format!("{json}\n"))
        .with_context(|| format!("failed to write {}", out_path.display()))
}

fn bake_benchmark_report(
    template: &str,
    core_data: &[CoreBenchRow],
    core_missing: &[MissingBenchRow],
    policy_payload: &PolicyMatrixPayload,
    repo_root: &Path,
) -> Result<String> {
    let mut html = inline_benchmark_vendor_styles(template, repo_root)?;
    let inject = build_benchmark_inline_script(core_data, core_missing, policy_payload)?;
    if html.contains(BENCH_INLINE_DATA_MARKER) {
        html = html.replacen(
            BENCH_INLINE_DATA_MARKER,
            &(inject + BENCH_INLINE_DATA_MARKER),
            1,
        );
    } else {
        html = html.replace("</body>", &(inject + "</body>"));
    }
    Ok(html)
}

fn inline_benchmark_vendor_styles(template: &str, repo_root: &Path) -> Result<String> {
    let open_props = std::fs::read_to_string("docs/benchmarks/vendor/open-props.min.css")
        .context("failed to read docs/benchmarks/vendor/open-props.min.css")?;
    let normalize_dark = std::fs::read_to_string("docs/benchmarks/vendor/normalize.dark.min.css")
        .context("failed to read docs/benchmarks/vendor/normalize.dark.min.css")?;

    let mut html = template.to_owned();
    html = replace_inline_link_tag(
        &html,
        BENCH_OPEN_PROPS_INLINE_MARKER,
        &format!(
            "<style data-bench-inline=\"open-props\">\n{}\n</style>",
            open_props.trim()
        ),
        repo_root,
    )?;
    html = replace_inline_link_tag(
        &html,
        BENCH_NORMALIZE_DARK_INLINE_MARKER,
        &format!(
            "<style data-bench-inline=\"normalize-dark\">\n{}\n</style>",
            normalize_dark.trim()
        ),
        repo_root,
    )?;
    Ok(html)
}

fn replace_inline_link_tag(
    haystack: &str,
    marker: &str,
    replacement: &str,
    repo_root: &Path,
) -> Result<String> {
    let Some(marker_index) = haystack.find(marker) else {
        bail!(
            "benchmark template is missing expected marker `{marker}` while baking from {}",
            display_repo_relative(Path::new("docs/benchmarks/index.html"), repo_root)
        );
    };
    let start = haystack[..marker_index].rfind("<link").with_context(|| {
        format!("failed to locate opening <link> for benchmark marker `{marker}`")
    })?;
    let relative_end = haystack[marker_index..]
        .find('>')
        .with_context(|| format!("failed to locate closing > for benchmark marker `{marker}`"))?;
    let end = marker_index + relative_end + 1;

    let mut result = String::with_capacity(haystack.len() + replacement.len());
    result.push_str(&haystack[..start]);
    result.push_str(replacement);
    result.push_str(&haystack[end..]);
    Ok(result)
}

fn build_benchmark_inline_script(
    core_data: &[CoreBenchRow],
    core_missing: &[MissingBenchRow],
    policy_payload: &PolicyMatrixPayload,
) -> Result<String> {
    let data_json =
        serde_json::to_string(core_data).context("failed to serialize core benchmark rows")?;
    let missing_json = serde_json::to_string(core_missing)
        .context("failed to serialize missing core benchmark rows")?;
    let policy_json = serde_json::to_string(policy_payload)
        .context("failed to serialize policy payload for inline report")?;

    Ok(format!(
        "<script>\nwindow.__CRITERION_DATA__ = {data_json};\nwindow.__CRITERION_MISSING__ = {missing_json};\nwindow.__POLICY_MATRIX__ = {policy_json};\n</script>\n"
    ))
}

fn display_repo_relative(path: &Path, repo_root: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .display()
        .to_string()
        .replace('\\', "/")
}

fn git_short_head_sha() -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .context("failed to run `git rev-parse --short HEAD`")?;
    if !output.status.success() {
        bail!("git rev-parse --short HEAD failed with {}", output.status);
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn local_benchmark_machine_descriptor() -> BenchMachineDescriptor {
    let hostname = std::env::var("HOSTNAME")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            std::env::var("COMPUTERNAME")
                .ok()
                .filter(|value| !value.trim().is_empty())
        });
    let os = std::env::consts::OS.to_owned();
    let arch = std::env::consts::ARCH.to_owned();
    let label = hostname.as_ref().map_or_else(
        || format!("{os}/{arch}"),
        |host| format!("{os}/{arch} on {host}"),
    );

    BenchMachineDescriptor {
        os,
        arch,
        hostname,
        label,
    }
}

fn run_pr_threads_list(selector: Option<&str>) -> Result<()> {
    let overview = fetch_pr_overview(selector)?;
    let threads = fetch_unresolved_review_threads(&overview)?;

    println!("PR #{}", overview.number);
    println!("URL: {}", overview.url);
    println!("Head SHA: {}", overview.short_head_sha());
    println!("Unresolved threads: {}", threads.len());

    if threads.is_empty() {
        println!("\nNo unresolved review threads.");
        return Ok(());
    }

    for (idx, thread) in threads.iter().enumerate() {
        println!("\n{}. {}", idx + 1, thread.thread_id);
        if let Some(comment_id) = thread.comment_id {
            println!("   Comment ID: {comment_id}");
        } else {
            println!("   Comment ID: unavailable");
        }
        println!(
            "   Author: {}",
            thread.author.as_deref().unwrap_or("unknown")
        );
        println!("   Path: {}", thread.display_location());
        println!(
            "   Outdated: {}",
            if thread.is_outdated { "yes" } else { "no" }
        );
        if let Some(url) = thread.url.as_deref() {
            println!("   URL: {url}");
        }
        println!("   Preview: {}", thread.preview);
    }

    Ok(())
}

fn run_pr_threads_reply(args: PrThreadsReplyArgs) -> Result<()> {
    let body = load_reply_body(args.body.as_deref(), args.body_file.as_deref())?;
    let overview = fetch_pr_overview(args.selector.as_deref())?;
    let route = build_review_reply_route(&overview);

    let output = run_gh_capture([
        "api",
        &route,
        "--method",
        "POST",
        "-f",
        &format!("body={body}"),
        "-F",
        &format!("in_reply_to={}", args.comment_id),
    ])?;
    let reply: ReviewReplyResponse =
        serde_json::from_str(&output).context("failed to parse review reply response")?;
    let url = reply
        .html_url
        .or(reply.url)
        .unwrap_or_else(|| "<no reply url returned>".to_owned());

    println!("Replied to review comment {}: {url}", args.comment_id);
    Ok(())
}

fn run_pr_threads_resolve(args: PrThreadsResolveArgs) -> Result<()> {
    let targets = resolve_thread_targets(&args)?;
    if targets.is_empty() {
        println!("No unresolved review threads matched the requested action.");
        return Ok(());
    }

    if !args.yes {
        eprintln!("Refusing to resolve review threads without --yes.");
        for thread in &targets {
            eprintln!("- {} ({})", thread.thread_id, thread.display_location());
        }
        bail!("rerun with --yes after confirming the selected thread ids");
    }

    for (resolved, thread) in targets.iter().enumerate() {
        if let Err(err) = run_gh_capture([
            "api",
            "graphql",
            "-F",
            &format!("threadId={}", thread.thread_id),
            "-f",
            "query=mutation($threadId:ID!) { resolveReviewThread(input:{threadId:$threadId}) { thread { id isResolved } } }",
        ]) {
            if resolved > 0 {
                eprintln!(
                    "Resolved {resolved}/{} threads before failure.",
                    targets.len()
                );
            }
            return Err(err);
        }
        println!(
            "Resolved {} ({})",
            thread.thread_id,
            thread.display_location()
        );
    }

    Ok(())
}

fn pr_status_script_path() -> PathBuf {
    Path::new("scripts").join("pr-status.sh")
}

fn build_pr_status_command(script: &Path, selector: Option<&str>) -> Command {
    let mut command = Command::new(script);
    if let Some(selector) = selector {
        command.arg(selector);
    }
    command
}

fn build_review_reply_route(overview: &PrOverview) -> String {
    format!(
        "repos/{}/{}/pulls/{}/comments",
        overview.owner, overview.repo, overview.number
    )
}

fn build_pr_snapshot_artifact(
    overview: &PrOverview,
    checks: &[PrCheckSummary],
    threads: &[ReviewThreadSummary],
    sortie_intent: DoghouseSortieIntent,
    code_rabbit: Option<DoghouseCodeRabbitState>,
) -> Result<PrSnapshotArtifact> {
    let recorded_at = OffsetDateTime::now_utc();
    let grouped_checks = group_pr_checks(checks);

    Ok(PrSnapshotArtifact {
        recorded_at: recorded_at
            .format(&Rfc3339)
            .context("failed to format snapshot timestamp")?,
        filename_stamp: format_snapshot_filename_stamp(recorded_at),
        sortie_intent: Some(sortie_intent),
        code_rabbit,
        pr: PrSnapshotOverview {
            number: overview.number,
            url: overview.url.clone(),
            title: overview.title.clone(),
            state: overview.state.clone(),
            head_sha: overview.head_sha.clone(),
            head_sha_short: overview.short_head_sha(),
            review_decision: overview.review_decision.clone(),
            merge_state: overview.merge_state.clone(),
        },
        blockers: collect_pr_blockers(overview, &grouped_checks, threads.len()),
        checks: checks.to_vec(),
        grouped_checks,
        unresolved_threads: threads.to_vec(),
    })
}

fn load_latest_pr_snapshot(pr_number: u64, out_dir: &Path) -> Result<Option<PrSnapshotArtifact>> {
    let latest_json = out_dir.join(format!("pr-{pr_number}")).join("latest.json");
    if !latest_json.is_file() {
        return Ok(None);
    }

    let contents = std::fs::read_to_string(&latest_json)
        .with_context(|| format!("failed to read {}", latest_json.display()))?;
    let snapshot: PrSnapshotArtifact = serde_json::from_str(&contents)
        .with_context(|| format!("failed to parse {}", latest_json.display()))?;
    Ok(Some(snapshot))
}

fn load_prior_pr_snapshots(pr_number: u64, out_dir: &Path) -> Result<Vec<PrSnapshotArtifact>> {
    let pr_dir = out_dir.join(format!("pr-{pr_number}"));
    if !pr_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut snapshots = Vec::new();
    for entry in std::fs::read_dir(&pr_dir)
        .with_context(|| format!("failed to read {}", pr_dir.display()))?
    {
        let entry = entry.with_context(|| format!("failed to read {}", pr_dir.display()))?;
        let path = entry.path();
        let file_name = path.file_name().and_then(|value| value.to_str());
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        if file_name.is_some_and(|value| {
            value == "latest.json" || value.ends_with(".delta.json") || value == "latest.delta.json"
        }) {
            continue;
        }
        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let snapshot: PrSnapshotArtifact = serde_json::from_str(&contents)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        snapshots.push(snapshot);
    }

    snapshots.sort_by(|left, right| {
        left.recorded_at
            .cmp(&right.recorded_at)
            .then_with(|| left.filename_stamp.cmp(&right.filename_stamp))
    });
    Ok(snapshots)
}

fn select_doghouse_baseline(
    snapshots: &[PrSnapshotArtifact],
    current: &PrSnapshotArtifact,
) -> Option<DoghouseBaselineSelection> {
    if snapshots.is_empty() {
        return None;
    }

    if let Some(index) = snapshots
        .iter()
        .rposition(|snapshot| snapshot.pr.head_sha != current.pr.head_sha)
    {
        return Some(build_doghouse_baseline_selection(
            snapshots,
            current,
            index,
            DoghouseBaselineStrategy::PreviousDifferentHead,
        ));
    }

    if let Some(index) = snapshots
        .iter()
        .rposition(|snapshot| snapshot_semantically_differs(snapshot, current))
    {
        return Some(build_doghouse_baseline_selection(
            snapshots,
            current,
            index,
            DoghouseBaselineStrategy::PreviousSemanticChange,
        ));
    }

    Some(build_doghouse_baseline_selection(
        snapshots,
        current,
        snapshots.len() - 1,
        DoghouseBaselineStrategy::ImmediatePrevious,
    ))
}

fn build_doghouse_baseline_selection(
    snapshots: &[PrSnapshotArtifact],
    current: &PrSnapshotArtifact,
    index: usize,
    strategy: DoghouseBaselineStrategy,
) -> DoghouseBaselineSelection {
    let snapshot = snapshots[index].clone();
    let newer_snapshots = &snapshots[index + 1..];

    DoghouseBaselineSelection {
        strategy,
        snapshot,
        newer_snapshot_count: newer_snapshots.len(),
        newer_semantic_change_count: newer_snapshots
            .iter()
            .filter(|candidate| snapshot_semantically_differs(candidate, current))
            .count(),
    }
}

fn assess_doghouse_comparison(
    current: &PrSnapshotArtifact,
    selection: Option<&DoghouseBaselineSelection>,
    delta: Option<&PrSnapshotDelta>,
) -> DoghouseComparisonAssessment {
    match selection {
        Some(selection) => {
            let same_head = selection.snapshot.pr.head_sha == current.pr.head_sha;
            let semantically_changed = delta.is_some_and(pr_snapshot_delta_has_changes);
            let baseline_age_seconds = doghouse_baseline_age_seconds(
                &selection.snapshot.recorded_at,
                &current.recorded_at,
            );
            let stale = baseline_age_seconds.is_some_and(|seconds| {
                seconds > doghouse_stale_threshold_seconds(&selection.strategy)
            });
            let noisy = selection.newer_semantic_change_count > 0;
            let (trust, mut reasons) = match selection.strategy {
                DoghouseBaselineStrategy::PreviousDifferentHead => (
                    DoghouseComparisonTrust::Strong,
                    vec![
                        "baseline uses a different head SHA".to_owned(),
                        "comparison spans a real push boundary".to_owned(),
                    ],
                ),
                DoghouseBaselineStrategy::PreviousSemanticChange => (
                    DoghouseComparisonTrust::Usable,
                    vec![
                        "baseline captures the most recent semantically different snapshot"
                            .to_owned(),
                        "comparison stays on the same head but still reflects a meaningful state change"
                            .to_owned(),
                    ],
                ),
                DoghouseBaselineStrategy::ImmediatePrevious => (
                    DoghouseComparisonTrust::Weak,
                    vec![
                        "no earlier different-head or semantically different baseline was available"
                            .to_owned(),
                        "comparison falls back to the immediately previous recorder capture"
                            .to_owned(),
                    ],
                ),
            };

            if let Some(seconds) = baseline_age_seconds {
                reasons.push(format!("baseline age is {seconds}s"));
            } else {
                reasons.push(
                    "baseline age could not be computed from the recorded timestamps".to_owned(),
                );
            }

            if same_head {
                reasons.push("baseline head matches the current head".to_owned());
            } else {
                reasons.push("baseline head differs from the current head".to_owned());
            }

            if semantically_changed {
                reasons
                    .push("selected baseline still shows a semantic state transition".to_owned());
            } else {
                reasons
                    .push("selected baseline does not show a semantic state transition".to_owned());
            }

            if stale {
                reasons.push(format!(
                    "baseline exceeded the {}s stale threshold for this selection strategy",
                    doghouse_stale_threshold_seconds(&selection.strategy)
                ));
            }

            if noisy {
                reasons.push(format!(
                    "comparison skips {} newer semantically different snapshot(s)",
                    selection.newer_semantic_change_count
                ));
            } else if selection.newer_snapshot_count > 0 {
                reasons.push(format!(
                    "{} newer snapshot(s) were skipped, but they were semantically equivalent to the current state",
                    selection.newer_snapshot_count
                ));
            }

            let quality = match (stale, noisy) {
                (true, true) => DoghouseComparisonQuality::StaleAndNoisy,
                (true, false) => DoghouseComparisonQuality::Stale,
                (false, true) => DoghouseComparisonQuality::Noisy,
                (false, false) => DoghouseComparisonQuality::GoodEnough,
            };

            DoghouseComparisonAssessment {
                selection: Some(selection.strategy.clone()),
                trust,
                quality,
                same_head: Some(same_head),
                semantically_changed: Some(semantically_changed),
                baseline_age_seconds,
                newer_snapshot_count: Some(selection.newer_snapshot_count),
                newer_semantic_change_count: Some(selection.newer_semantic_change_count),
                stale,
                noisy,
                reasons,
            }
        }
        None => DoghouseComparisonAssessment {
            selection: None,
            trust: DoghouseComparisonTrust::None,
            quality: DoghouseComparisonQuality::InitialCapture,
            same_head: None,
            semantically_changed: None,
            baseline_age_seconds: None,
            newer_snapshot_count: None,
            newer_semantic_change_count: None,
            stale: false,
            noisy: false,
            reasons: vec![
                "no prior snapshot was available".to_owned(),
                "treat this sortie as the initial capture, not a comparison".to_owned(),
            ],
        },
    }
}

fn doghouse_baseline_age_seconds(
    previous_recorded_at: &str,
    current_recorded_at: &str,
) -> Option<i64> {
    let previous = OffsetDateTime::parse(previous_recorded_at, &Rfc3339).ok()?;
    let current = OffsetDateTime::parse(current_recorded_at, &Rfc3339).ok()?;
    Some((current - previous).whole_seconds())
}

fn doghouse_sortie_intent_label(intent: &DoghouseSortieIntent) -> String {
    match intent {
        DoghouseSortieIntent::ManualProbe => "manual_probe".to_owned(),
        DoghouseSortieIntent::PostPush => "post_push".to_owned(),
        DoghouseSortieIntent::FixBatch => "fix_batch".to_owned(),
        DoghouseSortieIntent::MergeCheck => "merge_check".to_owned(),
        DoghouseSortieIntent::Resume => "resume".to_owned(),
    }
}

fn doghouse_stale_threshold_seconds(strategy: &DoghouseBaselineStrategy) -> i64 {
    match strategy {
        DoghouseBaselineStrategy::PreviousDifferentHead => 72 * 60 * 60,
        DoghouseBaselineStrategy::PreviousSemanticChange => 12 * 60 * 60,
        DoghouseBaselineStrategy::ImmediatePrevious => 2 * 60 * 60,
    }
}

fn pr_snapshot_delta_has_changes(delta: &PrSnapshotDelta) -> bool {
    !delta.blockers_added.is_empty()
        || !delta.blockers_removed.is_empty()
        || !delta.threads_opened.is_empty()
        || !delta.threads_resolved.is_empty()
        || !delta.improved_checks.is_empty()
        || !delta.regressed_checks.is_empty()
        || !delta.shifted_checks.is_empty()
        || !delta.added_checks.is_empty()
        || !delta.removed_checks.is_empty()
}

fn snapshot_semantically_differs(left: &PrSnapshotArtifact, right: &PrSnapshotArtifact) -> bool {
    left.pr.state != right.pr.state
        || left.pr.review_decision != right.pr.review_decision
        || left.pr.merge_state != right.pr.merge_state
        || left.blockers != right.blockers
        || left.checks != right.checks
        || left
            .unresolved_threads
            .iter()
            .map(|thread| thread.thread_id.as_str())
            .collect::<Vec<_>>()
            != right
                .unresolved_threads
                .iter()
                .map(|thread| thread.thread_id.as_str())
                .collect::<Vec<_>>()
}

fn write_pr_snapshot_artifact(
    snapshot: &PrSnapshotArtifact,
    out_dir: &Path,
) -> Result<PrSnapshotPaths> {
    let pr_dir = out_dir.join(format!("pr-{}", snapshot.pr.number));
    std::fs::create_dir_all(&pr_dir)
        .with_context(|| format!("failed to create {}", pr_dir.display()))?;

    let basename = format!("{}-{}", snapshot.filename_stamp, snapshot.pr.head_sha_short);
    let snapshot_json = pr_dir.join(format!("{basename}.json"));
    let snapshot_markdown = pr_dir.join(format!("{basename}.md"));
    let latest_json = pr_dir.join("latest.json");
    let latest_markdown = pr_dir.join("latest.md");

    let json = serde_json::to_string_pretty(snapshot)
        .context("failed to serialize PR snapshot JSON")?
        + "\n";
    let markdown = render_pr_snapshot_markdown(snapshot);

    std::fs::write(&snapshot_json, &json)
        .with_context(|| format!("failed to write {}", snapshot_json.display()))?;
    std::fs::write(&snapshot_markdown, &markdown)
        .with_context(|| format!("failed to write {}", snapshot_markdown.display()))?;
    std::fs::write(&latest_json, &json)
        .with_context(|| format!("failed to write {}", latest_json.display()))?;
    std::fs::write(&latest_markdown, &markdown)
        .with_context(|| format!("failed to write {}", latest_markdown.display()))?;

    Ok(PrSnapshotPaths {
        snapshot_json,
        snapshot_markdown,
        latest_json,
        latest_markdown,
    })
}

fn write_doghouse_jsonl_events(
    lines: &[String],
    pr_number: u64,
    filename_stamp: &str,
    head_sha_short: &str,
    out_dir: &Path,
) -> Result<DoghouseJsonlPaths> {
    let pr_dir = out_dir.join(format!("pr-{pr_number}"));
    std::fs::create_dir_all(&pr_dir)
        .with_context(|| format!("failed to create {}", pr_dir.display()))?;

    let basename = format!("{filename_stamp}-{head_sha_short}.sortie.jsonl");
    let snapshot_jsonl = pr_dir.join(basename);
    let latest_jsonl = pr_dir.join("latest.sortie.jsonl");
    let payload = format!("{}\n", lines.join("\n"));

    std::fs::write(&snapshot_jsonl, &payload)
        .with_context(|| format!("failed to write {}", snapshot_jsonl.display()))?;
    std::fs::write(&latest_jsonl, &payload)
        .with_context(|| format!("failed to write {}", latest_jsonl.display()))?;

    Ok(DoghouseJsonlPaths {
        snapshot_jsonl,
        latest_jsonl,
    })
}

fn write_pr_snapshot_delta_artifact(
    delta: &PrSnapshotDelta,
    pr_number: u64,
    filename_stamp: &str,
    head_sha_short: &str,
    out_dir: &Path,
) -> Result<PrSnapshotDeltaPaths> {
    let pr_dir = out_dir.join(format!("pr-{pr_number}"));
    std::fs::create_dir_all(&pr_dir)
        .with_context(|| format!("failed to create {}", pr_dir.display()))?;

    let basename = format!("{filename_stamp}-{head_sha_short}.delta");
    let snapshot_json = pr_dir.join(format!("{basename}.json"));
    let snapshot_markdown = pr_dir.join(format!("{basename}.md"));
    let latest_json = pr_dir.join("latest.delta.json");
    let latest_markdown = pr_dir.join("latest.delta.md");

    let json =
        serde_json::to_string_pretty(delta).context("failed to serialize PR delta JSON")? + "\n";
    let markdown = render_pr_snapshot_delta_markdown(delta);

    std::fs::write(&snapshot_json, &json)
        .with_context(|| format!("failed to write {}", snapshot_json.display()))?;
    std::fs::write(&snapshot_markdown, &markdown)
        .with_context(|| format!("failed to write {}", snapshot_markdown.display()))?;
    std::fs::write(&latest_json, &json)
        .with_context(|| format!("failed to write {}", latest_json.display()))?;
    std::fs::write(&latest_markdown, &markdown)
        .with_context(|| format!("failed to write {}", latest_markdown.display()))?;

    Ok(PrSnapshotDeltaPaths {
        snapshot_json,
        snapshot_markdown,
        latest_json,
        latest_markdown,
    })
}

fn build_pr_snapshot_delta(
    previous: &PrSnapshotArtifact,
    current: &PrSnapshotArtifact,
) -> PrSnapshotDelta {
    let previous_blockers = previous.blockers.iter().cloned().collect::<BTreeSet<_>>();
    let current_blockers = current.blockers.iter().cloned().collect::<BTreeSet<_>>();

    let blockers_added = current_blockers
        .difference(&previous_blockers)
        .cloned()
        .collect::<Vec<_>>();
    let blockers_removed = previous_blockers
        .difference(&current_blockers)
        .cloned()
        .collect::<Vec<_>>();
    let blockers_unchanged = current_blockers
        .intersection(&previous_blockers)
        .cloned()
        .collect::<Vec<_>>();

    let previous_threads = previous
        .unresolved_threads
        .iter()
        .map(|thread| (thread.thread_id.clone(), thread))
        .collect::<BTreeMap<_, _>>();
    let current_threads = current
        .unresolved_threads
        .iter()
        .map(|thread| (thread.thread_id.clone(), thread))
        .collect::<BTreeMap<_, _>>();

    let threads_opened = current_threads
        .iter()
        .filter(|(thread_id, _)| !previous_threads.contains_key(*thread_id))
        .map(|(_, thread)| (*thread).clone())
        .collect::<Vec<_>>();
    let threads_resolved = previous_threads
        .iter()
        .filter(|(thread_id, _)| !current_threads.contains_key(*thread_id))
        .map(|(_, thread)| (*thread).clone())
        .collect::<Vec<_>>();
    let threads_persisting = current_threads
        .iter()
        .filter(|(thread_id, _)| previous_threads.contains_key(*thread_id))
        .map(|(_, thread)| (*thread).clone())
        .collect::<Vec<_>>();

    let previous_checks = previous
        .checks
        .iter()
        .map(|check| (check.name.clone(), check))
        .collect::<BTreeMap<_, _>>();
    let current_checks = current
        .checks
        .iter()
        .map(|check| (check.name.clone(), check))
        .collect::<BTreeMap<_, _>>();

    let mut improved_checks = Vec::new();
    let mut regressed_checks = Vec::new();
    let mut shifted_checks = Vec::new();
    let mut added_checks = Vec::new();
    let mut removed_checks = Vec::new();

    for (name, current_check) in &current_checks {
        match previous_checks.get(name) {
            Some(previous_check) => {
                if previous_check == current_check {
                    continue;
                }
                let transition = build_pr_check_transition(previous_check, current_check);
                match transition.kind {
                    PrCheckTransitionKind::Improved => improved_checks.push(transition),
                    PrCheckTransitionKind::Regressed => regressed_checks.push(transition),
                    PrCheckTransitionKind::Shifted => shifted_checks.push(transition),
                }
            }
            None => added_checks.push((*current_check).clone()),
        }
    }

    for (name, previous_check) in &previous_checks {
        if !current_checks.contains_key(name) {
            removed_checks.push((*previous_check).clone());
        }
    }

    PrSnapshotDelta {
        previous: PrSnapshotDeltaRef::from_snapshot(previous),
        current: PrSnapshotDeltaRef::from_snapshot(current),
        blockers_added,
        blockers_removed,
        blockers_unchanged,
        threads_opened,
        threads_resolved,
        threads_persisting,
        improved_checks,
        regressed_checks,
        shifted_checks,
        added_checks,
        removed_checks,
    }
}

fn build_pr_check_transition(
    previous: &PrCheckSummary,
    current: &PrCheckSummary,
) -> PrCheckTransition {
    let previous_score = check_bucket_score(&previous.bucket);
    let current_score = check_bucket_score(&current.bucket);
    let kind = match current_score.cmp(&previous_score) {
        std::cmp::Ordering::Greater => PrCheckTransitionKind::Improved,
        std::cmp::Ordering::Less => PrCheckTransitionKind::Regressed,
        std::cmp::Ordering::Equal => PrCheckTransitionKind::Shifted,
    };

    PrCheckTransition {
        name: current.name.clone(),
        previous_bucket: previous.bucket.clone(),
        previous_state: previous.state.clone(),
        current_bucket: current.bucket.clone(),
        current_state: current.state.clone(),
        kind,
    }
}

fn check_bucket_score(bucket: &str) -> i8 {
    match bucket {
        "fail" => 0,
        "pending" => 2,
        "skipping" => 3,
        "pass" => 4,
        _ => 1,
    }
}

fn format_snapshot_filename_stamp(timestamp: OffsetDateTime) -> String {
    format!(
        "{:04}{:02}{:02}T{:02}{:02}{:02}_{:09}Z",
        timestamp.year(),
        u8::from(timestamp.month()),
        timestamp.day(),
        timestamp.hour(),
        timestamp.minute(),
        timestamp.second(),
        timestamp.nanosecond(),
    )
}

fn collect_pr_blockers(
    overview: &PrOverview,
    grouped_checks: &BTreeMap<String, Vec<String>>,
    unresolved_threads: usize,
) -> Vec<String> {
    let mut blockers = Vec::new();

    if unresolved_threads > 0 {
        blockers.push(format!("unresolved review threads: {unresolved_threads}"));
    }
    if let Some(failing) = grouped_checks
        .get("fail")
        .filter(|checks| !checks.is_empty())
    {
        blockers.push(format!("failing checks: {}", failing.join(", ")));
    }
    if let Some(pending) = grouped_checks
        .get("pending")
        .filter(|checks| !checks.is_empty())
    {
        blockers.push(format!("pending checks: {}", pending.join(", ")));
    }
    if overview.state == "OPEN" && overview.review_decision != "APPROVED" {
        blockers.push(format!("review decision: {}", overview.review_decision));
    }
    if overview.state == "OPEN" && !matches!(overview.merge_state.as_str(), "CLEAN" | "HAS_HOOKS") {
        blockers.push(format!("merge state: {}", overview.merge_state));
    }

    blockers
}

fn group_pr_checks(checks: &[PrCheckSummary]) -> BTreeMap<String, Vec<String>> {
    let mut grouped = BTreeMap::<String, Vec<String>>::new();
    for check in checks {
        grouped
            .entry(check.bucket.clone())
            .or_default()
            .push(check.name.clone());
    }
    for names in grouped.values_mut() {
        names.sort();
    }
    grouped
}

fn render_pr_snapshot_markdown(snapshot: &PrSnapshotArtifact) -> String {
    let mut sections = Vec::new();
    sections.push(format!(
        "# Doghouse Flight Recorder: PR #{}\n",
        snapshot.pr.number
    ));
    sections.push(format!("URL: {}\n", snapshot.pr.url));
    sections.push(format!("Title: {}\n", snapshot.pr.title));
    sections.push(format!("Recorded at: {}\n", snapshot.recorded_at));
    if let Some(intent) = snapshot.sortie_intent.as_ref() {
        sections.push(format!(
            "Sortie intent: `{}`\n",
            doghouse_sortie_intent_label(intent)
        ));
    }
    sections.push(format!("PR state: `{}`\n", snapshot.pr.state));
    sections.push(format!("Head SHA: `{}`\n", snapshot.pr.head_sha));
    sections.push(format!(
        "Review decision: `{}`\n",
        snapshot.pr.review_decision
    ));
    sections.push(format!("Merge state: `{}`\n", snapshot.pr.merge_state));
    sections.push(format!(
        "Unresolved threads: {}\n",
        snapshot.unresolved_threads.len()
    ));

    sections.push("\n## Current Blockers\n".to_owned());
    if snapshot.blockers.is_empty() {
        sections.push("- none detected\n".to_owned());
    } else {
        for blocker in &snapshot.blockers {
            sections.push(format!("- {blocker}\n"));
        }
    }

    sections.push("\n## Checks\n".to_owned());
    for bucket in ["fail", "pending", "pass", "skipping", "cancel", "unknown"] {
        if let Some(names) = snapshot.grouped_checks.get(bucket) {
            if names.is_empty() {
                continue;
            }
            sections.push(format!(
                "\n### {} ({})\n",
                display_check_bucket(bucket),
                names.len()
            ));
            for name in names {
                sections.push(format!("- {name}\n"));
            }
        }
    }

    sections.push("\n## Unresolved Threads\n".to_owned());
    if snapshot.unresolved_threads.is_empty() {
        sections.push("No unresolved review threads.\n".to_owned());
    } else {
        for (idx, thread) in snapshot.unresolved_threads.iter().enumerate() {
            sections.push(format!(
                "\n{}. `{}` by `{}` at `{}`\n",
                idx + 1,
                thread.thread_id,
                thread.author.as_deref().unwrap_or("unknown"),
                thread.display_location()
            ));
            if let Some(comment_id) = thread.comment_id {
                sections.push(format!("Comment ID: `{comment_id}`\n"));
            }
            if let Some(url) = thread.url.as_deref() {
                sections.push(format!("URL: {url}\n"));
            }
            sections.push(format!("Preview: {}\n", thread.preview));
        }
    }

    sections.concat()
}

fn render_pr_snapshot_delta_markdown(delta: &PrSnapshotDelta) -> String {
    let mut sections = Vec::new();
    sections.push(format!("# Doghouse Delta: PR #{}\n", delta.current.number));
    sections.push(format!(
        "Compared at: current snapshot `{}`\n",
        delta.current.recorded_at
    ));
    sections.push(format!(
        "Previous snapshot: `{}` at `{}`\n",
        delta.previous.head_sha_short, delta.previous.recorded_at
    ));
    sections.push(format!(
        "Current snapshot: `{}` at `{}`\n",
        delta.current.head_sha_short, delta.current.recorded_at
    ));

    sections.push("\n## Head Transition\n".to_owned());
    if delta.previous.head_sha == delta.current.head_sha {
        sections.push(format!(
            "Head unchanged: `{}`\n",
            delta.current.head_sha_short
        ));
    } else {
        sections.push(format!(
            "`{}` -> `{}`\n",
            delta.previous.head_sha_short, delta.current.head_sha_short
        ));
    }

    sections.push("\n## Blocker Transition\n".to_owned());
    if delta.blockers_added.is_empty()
        && delta.blockers_removed.is_empty()
        && delta.blockers_unchanged.is_empty()
    {
        sections.push("No blockers in either snapshot.\n".to_owned());
    } else {
        render_string_section(&mut sections, "Added blockers", &delta.blockers_added, "- ");
        render_string_section(
            &mut sections,
            "Removed blockers",
            &delta.blockers_removed,
            "- ",
        );
        render_string_section(
            &mut sections,
            "Unchanged blockers",
            &delta.blockers_unchanged,
            "- ",
        );
    }

    sections.push("\n## Thread Transition\n".to_owned());
    let thread_transition_count =
        delta.threads_opened.len() + delta.threads_resolved.len() + delta.threads_persisting.len();
    if thread_transition_count == 0 {
        sections.push("No unresolved-thread transition detected.\n".to_owned());
    } else {
        render_thread_section(
            &mut sections,
            "Opened unresolved threads",
            &delta.threads_opened,
        );
        render_thread_section(
            &mut sections,
            "Resolved unresolved threads",
            &delta.threads_resolved,
        );
        render_thread_section(
            &mut sections,
            "Still-open unresolved threads",
            &delta.threads_persisting,
        );
    }

    sections.push("\n## Check Transition\n".to_owned());
    let check_transition_count = delta.improved_checks.len()
        + delta.regressed_checks.len()
        + delta.shifted_checks.len()
        + delta.added_checks.len()
        + delta.removed_checks.len();
    if check_transition_count == 0 {
        sections.push("No check transition detected.\n".to_owned());
    } else {
        render_check_transition_section(&mut sections, "Improved checks", &delta.improved_checks);
        render_check_transition_section(&mut sections, "Regressed checks", &delta.regressed_checks);
        render_check_transition_section(&mut sections, "Shifted checks", &delta.shifted_checks);
        render_check_section(&mut sections, "Added checks", &delta.added_checks);
        render_check_section(&mut sections, "Removed checks", &delta.removed_checks);
    }

    sections.concat()
}

fn render_string_section(
    sections: &mut Vec<String>,
    heading: &str,
    values: &[String],
    prefix: &str,
) {
    if values.is_empty() {
        return;
    }
    sections.push(format!("\n### {heading} ({})\n", values.len()));
    for value in values {
        sections.push(format!("{prefix}{value}\n"));
    }
}

fn render_thread_section(
    sections: &mut Vec<String>,
    heading: &str,
    threads: &[ReviewThreadSummary],
) {
    if threads.is_empty() {
        return;
    }
    sections.push(format!("\n### {heading} ({})\n", threads.len()));
    for thread in threads {
        sections.push(format!(
            "- `{}` at `{}` by `{}`\n",
            thread.thread_id,
            thread.display_location(),
            thread.author.as_deref().unwrap_or("unknown")
        ));
    }
}

fn render_check_transition_section(
    sections: &mut Vec<String>,
    heading: &str,
    transitions: &[PrCheckTransition],
) {
    if transitions.is_empty() {
        return;
    }
    sections.push(format!("\n### {heading} ({})\n", transitions.len()));
    for transition in transitions {
        sections.push(format!(
            "- {}: `{}/{}` -> `{}/{}`\n",
            transition.name,
            transition.previous_bucket,
            transition.previous_state,
            transition.current_bucket,
            transition.current_state
        ));
    }
}

fn render_check_section(sections: &mut Vec<String>, heading: &str, checks: &[PrCheckSummary]) {
    if checks.is_empty() {
        return;
    }
    sections.push(format!("\n### {heading} ({})\n", checks.len()));
    for check in checks {
        sections.push(format!(
            "- {}: `{}/{}`\n",
            check.name, check.bucket, check.state
        ));
    }
}

fn print_pr_snapshot_delta_summary(delta: &PrSnapshotDelta) {
    println!(
        "Delta since previous snapshot ({} @ {}):",
        delta.previous.recorded_at, delta.previous.head_sha_short
    );
    if delta.previous.head_sha == delta.current.head_sha {
        println!("- head: unchanged ({})", delta.current.head_sha_short);
    } else {
        println!(
            "- head: {} -> {}",
            delta.previous.head_sha_short, delta.current.head_sha_short
        );
    }
    println!(
        "- blockers: +{} / -{} / {} unchanged",
        delta.blockers_added.len(),
        delta.blockers_removed.len(),
        delta.blockers_unchanged.len()
    );
    println!(
        "- threads: +{} opened / -{} resolved / {} still open",
        delta.threads_opened.len(),
        delta.threads_resolved.len(),
        delta.threads_persisting.len()
    );
    println!(
        "- checks: {} improved / {} regressed / {} shifted / {} added / {} removed",
        delta.improved_checks.len(),
        delta.regressed_checks.len(),
        delta.shifted_checks.len(),
        delta.added_checks.len(),
        delta.removed_checks.len()
    );
}

fn build_doghouse_sortie_events(
    snapshot: &PrSnapshotArtifact,
    baseline: Option<&DoghouseBaselineSelection>,
    comparison: &DoghouseComparisonAssessment,
    delta: Option<&PrSnapshotDelta>,
    next_action: &DoghouseNextAction,
) -> Result<Vec<String>> {
    let mut lines = vec![
        serde_json::to_string(&DoghouseSnapshotEvent {
            kind: "doghouse.snapshot",
            pr_number: snapshot.pr.number,
            pr_url: snapshot.pr.url.clone(),
            pr_title: snapshot.pr.title.clone(),
            pr_state: snapshot.pr.state.clone(),
            recorded_at: snapshot.recorded_at.clone(),
            sortie_intent: snapshot.sortie_intent,
            head_sha: snapshot.pr.head_sha.clone(),
            head_sha_short: snapshot.pr.head_sha_short.clone(),
            review_decision: snapshot.pr.review_decision.clone(),
            merge_state: snapshot.pr.merge_state.clone(),
            blocker_count: snapshot.blockers.len(),
            blockers: snapshot.blockers.clone(),
            unresolved_thread_count: snapshot.unresolved_threads.len(),
            check_counts: summarize_check_counts(&snapshot.grouped_checks),
        })
        .context("failed to serialize doghouse snapshot event")?,
        serde_json::to_string(&DoghouseIntentEvent::from_snapshots(snapshot, baseline))
            .context("failed to serialize doghouse intent event")?,
        serde_json::to_string(&DoghouseCodeRabbitEvent::from_snapshot(snapshot))
            .context("failed to serialize doghouse CodeRabbit event")?,
        serde_json::to_string(&DoghouseBaselineEvent::from_selection(baseline))
            .context("failed to serialize doghouse baseline event")?,
        serde_json::to_string(&DoghouseComparisonEvent::from_assessment(comparison))
            .context("failed to serialize doghouse comparison event")?,
    ];

    if let Some(delta) = delta {
        lines.push(
            serde_json::to_string(&DoghouseDeltaEvent::from_delta(delta))
                .context("failed to serialize doghouse delta event")?,
        );
    }

    lines.push(
        serde_json::to_string(next_action)
            .context("failed to serialize doghouse next-action event")?,
    );

    Ok(lines)
}

fn summarize_check_counts(
    grouped_checks: &BTreeMap<String, Vec<String>>,
) -> BTreeMap<String, usize> {
    grouped_checks
        .iter()
        .map(|(bucket, checks)| (bucket.clone(), checks.len()))
        .collect()
}

fn determine_doghouse_next_action(
    snapshot: &PrSnapshotArtifact,
    comparison: &DoghouseComparisonAssessment,
    delta: Option<&PrSnapshotDelta>,
) -> DoghouseNextAction {
    let sortie_intent = snapshot
        .sortie_intent
        .unwrap_or(DoghouseSortieIntent::ManualProbe);
    let code_rabbit_request_actionable = doghouse_coderabbit_request_review_actionable(snapshot);
    let mut action = if snapshot.pr.state == "MERGED" {
        "complete_merged"
    } else if !snapshot.unresolved_threads.is_empty() {
        "fix_unresolved_threads"
    } else if snapshot
        .grouped_checks
        .get("fail")
        .is_some_and(|checks| !checks.is_empty())
    {
        "fix_failing_checks"
    } else if snapshot
        .grouped_checks
        .get("pending")
        .is_some_and(|checks| !checks.is_empty())
    {
        "wait_for_pending_checks"
    } else if snapshot
        .code_rabbit
        .as_ref()
        .is_some_and(|state| state.rearm_actionable)
    {
        "rearm_coderabbit"
    } else if snapshot.pr.review_decision != "APPROVED" {
        match sortie_intent {
            DoghouseSortieIntent::MergeCheck => "merge_ready_pending_approval",
            DoghouseSortieIntent::PostPush => "wait_for_review_feedback",
            DoghouseSortieIntent::FixBatch
            | DoghouseSortieIntent::Resume
            | DoghouseSortieIntent::ManualProbe => {
                if code_rabbit_request_actionable {
                    "nudge_coderabbit"
                } else {
                    "request_review"
                }
            }
        }
    } else if matches!(snapshot.pr.merge_state.as_str(), "CLEAN" | "HAS_HOOKS") {
        "ready_for_merge"
    } else if matches!(sortie_intent, DoghouseSortieIntent::MergeCheck) {
        "merge_blocked_investigate_state"
    } else {
        "investigate_merge_state"
    };

    let recapture_required = doghouse_comparison_requires_recapture(comparison)
        && doghouse_action_requires_trusted_comparison(action);

    if recapture_required {
        action = "capture_fresh_sortie";
    }

    let mut reasons = vec![format!(
        "sortie intent is {}",
        doghouse_sortie_intent_label(&sortie_intent)
    )];
    if snapshot.pr.state == "MERGED" {
        reasons.push("PR state is MERGED".to_owned());
    }
    if !snapshot.unresolved_threads.is_empty() {
        reasons.push(format!(
            "{} unresolved review thread(s) remain",
            snapshot.unresolved_threads.len()
        ));
    }
    if let Some(failing) = snapshot
        .grouped_checks
        .get("fail")
        .filter(|checks| !checks.is_empty())
    {
        reasons.push(format!("failing checks: {}", failing.join(", ")));
    }
    if let Some(pending) = snapshot
        .grouped_checks
        .get("pending")
        .filter(|checks| !checks.is_empty())
    {
        reasons.push(format!("pending checks: {}", pending.join(", ")));
    }
    if snapshot.pr.state == "OPEN" && snapshot.pr.review_decision != "APPROVED" {
        reasons.push(format!(
            "review decision is {}",
            snapshot.pr.review_decision
        ));
    }
    if snapshot.pr.state == "OPEN"
        && !matches!(snapshot.pr.merge_state.as_str(), "CLEAN" | "HAS_HOOKS")
    {
        reasons.push(format!("merge state is {}", snapshot.pr.merge_state));
    }
    if recapture_required {
        reasons.push(format!(
            "comparison quality is {}; capture another sortie before trusting an affirmative workflow move",
            doghouse_comparison_quality_label(&comparison.quality)
        ));
    }
    if let Some(code_rabbit) = snapshot.code_rabbit.as_ref() {
        if code_rabbit.rearm_actionable {
            let unchecked = code_rabbit
                .checkboxes
                .iter()
                .filter(|checkbox| !checkbox.checked)
                .count();
            reasons.push(format!(
                "CodeRabbit summary comment wants manual rearm via {unchecked} unchecked checkbox(es); keep human and Codex review state separate while re-enabling Rabbit"
            ));
        }
        if code_rabbit.cooldown_active {
            reasons.push(format!(
                "CodeRabbit summary comment is rate-limited until {} ({}s remaining); do not request another Rabbit round yet, but keep tracking human and Codex review state separately",
                code_rabbit
                    .cooldown_expires_at
                    .as_deref()
                    .unwrap_or("<unknown>"),
                code_rabbit.cooldown_remaining_seconds.unwrap_or_default()
            ));
        }
    } else if snapshot
        .checks
        .iter()
        .find(|check| check.name.eq_ignore_ascii_case("CodeRabbit"))
        .is_some_and(|check| check.bucket == "pending")
    {
        reasons.push(
            "CodeRabbit is actively reviewing this head; do not request another Rabbit round yet, but keep tracking human and Codex review state separately"
                .to_owned(),
        );
    }
    match (sortie_intent, action) {
        (DoghouseSortieIntent::MergeCheck, "merge_ready_pending_approval") => reasons.push(
            "merge-check intent found only a formal approval blocker, not a live code or CI blocker"
                .to_owned(),
        ),
        (DoghouseSortieIntent::MergeCheck, "ready_for_merge") => reasons
            .push("merge-check intent found no live blockers".to_owned()),
        (DoghouseSortieIntent::MergeCheck, "wait_for_pending_checks") => reasons.push(
            "merge-check intent cannot conclude until pending checks clear".to_owned(),
        ),
        (DoghouseSortieIntent::MergeCheck, "merge_blocked_investigate_state") => reasons.push(
            "merge-check intent found a non-clean merge state after code and CI blockers were cleared"
                .to_owned(),
        ),
        (DoghouseSortieIntent::PostPush, "wait_for_review_feedback") => reasons.push(
            "post-push intent stays in observation mode until the new head gets review feedback"
                .to_owned(),
        ),
        (DoghouseSortieIntent::PostPush, "wait_for_pending_checks") => reasons.push(
            "post-push intent stays in observation mode until CI settles".to_owned(),
        ),
        (DoghouseSortieIntent::FixBatch, "request_review") => reasons.push(
            "fix-batch intent finished the repair pass; the next move is another review round"
                .to_owned(),
        ),
        (_, "nudge_coderabbit") => reasons.push(
            "CodeRabbit is actionable again, so the immediate next move is to post a review/resume nudge"
                .to_owned(),
        ),
        (_, "rearm_coderabbit") => reasons.push(
            "the immediate next mechanical step is to re-enable CodeRabbit on its own summary comment before asking it for more work"
                .to_owned(),
        ),
        (_, "capture_fresh_sortie") => reasons.push(
            "Doghouse wants a fresher comparison before recommending the next affirmative workflow step"
                .to_owned(),
        ),
        (DoghouseSortieIntent::Resume, "ready_for_merge") => reasons.push(
            "resume intent reconstructed a merge-ready state".to_owned(),
        ),
        _ => {}
    }
    if let Some(delta) = delta {
        reasons.push(format!(
            "delta summary: {} blocker(s) added, {} removed; {} thread(s) opened, {} resolved",
            delta.blockers_added.len(),
            delta.blockers_removed.len(),
            delta.threads_opened.len(),
            delta.threads_resolved.len()
        ));
    }

    DoghouseNextAction {
        kind: "doghouse.next_action",
        sortie_intent: Some(sortie_intent),
        action: action.to_owned(),
        reasons,
    }
}

fn doghouse_comparison_requires_recapture(comparison: &DoghouseComparisonAssessment) -> bool {
    matches!(
        comparison.quality,
        DoghouseComparisonQuality::Stale
            | DoghouseComparisonQuality::Noisy
            | DoghouseComparisonQuality::StaleAndNoisy
    )
}

fn doghouse_action_requires_trusted_comparison(action: &str) -> bool {
    matches!(
        action,
        "nudge_coderabbit"
            | "request_review"
            | "merge_ready_pending_approval"
            | "ready_for_merge"
            | "merge_blocked_investigate_state"
            | "investigate_merge_state"
    )
}

fn doghouse_comparison_quality_label(quality: &DoghouseComparisonQuality) -> &'static str {
    match quality {
        DoghouseComparisonQuality::GoodEnough => "good_enough",
        DoghouseComparisonQuality::Stale => "stale",
        DoghouseComparisonQuality::Noisy => "noisy",
        DoghouseComparisonQuality::StaleAndNoisy => "stale_and_noisy",
        DoghouseComparisonQuality::InitialCapture => "initial_capture",
    }
}

fn doghouse_coderabbit_request_review_actionable(snapshot: &PrSnapshotArtifact) -> bool {
    let currently_reviewing = snapshot
        .checks
        .iter()
        .find(|check| check.name.eq_ignore_ascii_case("CodeRabbit"))
        .is_some_and(|check| check.bucket == "pending");

    snapshot
        .code_rabbit
        .as_ref()
        .is_some_and(|state| state.request_review_actionable && !currently_reviewing)
}

fn display_check_bucket(bucket: &str) -> &'static str {
    match bucket {
        "fail" => "Failing checks",
        "pending" => "Pending checks",
        "pass" => "Passing checks",
        "skipping" => "Skipped checks",
        "cancel" => "Cancelled checks",
        _ => "Other checks",
    }
}

fn build_pr_preflight_plan(changed_files: &[String], full: bool) -> Vec<PreflightCheck> {
    let scope = analyze_pr_preflight_scope(changed_files, full);
    let mut checks = Vec::new();

    checks.push(PreflightCheck::new(
        format!("local verification ({})", scope.verify_mode),
        build_verify_local_command(scope.verify_mode),
    ));

    if scope.run_dead_refs {
        let command = build_lint_dead_refs_command(scope.markdown_files.as_deref(), full);
        checks.push(PreflightCheck::new("docs dead refs", command));
    }

    if let Some(markdown_files) = scope.markdown_files.as_ref() {
        let command = build_markdownlint_command(markdown_files);
        checks.push(PreflightCheck::new("markdownlint", command));
    }

    if scope.run_runtime_schema_explicit {
        let mut command = Command::new("pnpm");
        command.arg("schema:runtime:check");
        checks.push(PreflightCheck::new("runtime schema validation", command));
    }

    if scope.run_feature_contracts {
        checks.push(PreflightCheck::new(
            "feature contract: echo-runtime-schema --no-default-features",
            build_cargo_check_command(&[
                "check",
                "-p",
                "echo-runtime-schema",
                "--no-default-features",
                "--message-format",
                "short",
            ]),
        ));
        checks.push(PreflightCheck::new(
            "feature contract: echo-wasm-abi --no-default-features",
            build_cargo_check_command(&[
                "check",
                "-p",
                "echo-wasm-abi",
                "--no-default-features",
                "--message-format",
                "short",
            ]),
        ));
    }

    if let Some(shell_files) = scope.shell_files.as_ref() {
        let mut command = Command::new("bash");
        command.arg("-n");
        for file in shell_files {
            command.arg(file);
        }
        checks.push(PreflightCheck::new("shell syntax", command));
    }

    checks
}

fn path_has_extension(path: &str, extension: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case(extension))
}

fn repo_relative_file_exists(path: &str) -> bool {
    let direct = Path::new(path);
    if direct.is_file() {
        return true;
    }

    find_repo_root()
        .ok()
        .is_some_and(|root| root.join(path).is_file())
}

fn collect_pr_preflight_changed_files(base: &str) -> Result<Vec<String>> {
    let mut files = BTreeSet::new();
    for file in run_git_lines(["diff", "--name-only", &format!("{base}...HEAD")])? {
        files.insert(file);
    }
    for file in run_git_lines(["diff", "--name-only", "--cached"])? {
        files.insert(file);
    }
    for file in run_git_lines(["diff", "--name-only"])? {
        files.insert(file);
    }
    for file in run_git_lines(["ls-files", "--others", "--exclude-standard"])? {
        files.insert(file);
    }
    Ok(files.into_iter().collect())
}

fn run_git_lines<I, S>(args: I) -> Result<Vec<String>>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = Command::new("git")
        .args(args)
        .output()
        .context("failed to spawn git")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let message = stderr.trim();
        if message.is_empty() {
            bail!("git command failed with exit status {}", output.status);
        }
        bail!("{message}");
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

fn analyze_pr_preflight_scope(changed_files: &[String], full: bool) -> PreflightScope {
    let mut scope = PreflightScope {
        verify_mode: if full { "full" } else { "pr" },
        run_dead_refs: full,
        markdown_files: None,
        run_runtime_schema_explicit: full,
        run_feature_contracts: full,
        shell_files: None,
    };

    if full {
        let all_markdown = tracked_files_matching(["ls-files", "*.md"]).unwrap_or_default();
        if !all_markdown.is_empty() {
            scope.markdown_files = Some(all_markdown);
        }
        let shell_files = tracked_shell_files().unwrap_or_default();
        if !shell_files.is_empty() {
            scope.shell_files = Some(shell_files);
        }
        return scope;
    }

    let markdown_files: Vec<String> = changed_files
        .iter()
        .filter(|path| path_has_extension(path, "md") && repo_relative_file_exists(path))
        .cloned()
        .collect();
    if !markdown_files.is_empty() {
        scope.markdown_files = Some(markdown_files);
        scope.run_dead_refs = true;
    }

    if changed_files.iter().any(|path| {
        matches!(
            path.as_str(),
            "package.json"
                | "pnpm-lock.yaml"
                | "scripts/validate-runtime-schema-fragments.mjs"
                | "tests/hooks/test_runtime_schema_validation.sh"
        ) || path.starts_with("schemas/runtime/")
    }) {
        scope.run_runtime_schema_explicit = true;
    }

    if changed_files.iter().any(|path| {
        matches!(
            path.as_str(),
            "Cargo.toml" | "Cargo.lock" | "docs/workflows.md"
        ) || path.starts_with("crates/echo-runtime-schema/")
            || path.starts_with("crates/echo-wasm-abi/")
    }) {
        scope.run_feature_contracts = true;
    }

    let shell_files: Vec<String> = changed_files
        .iter()
        .filter(|path| is_maintained_shell_path(path) && repo_relative_file_exists(path))
        .cloned()
        .collect();
    if !shell_files.is_empty() {
        scope.shell_files = Some(shell_files);
    }

    scope
}

fn tracked_files_matching<I, S>(args: I) -> Result<Vec<String>>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    run_git_lines(args)
}

fn tracked_shell_files() -> Result<Vec<String>> {
    Ok(
        run_git_lines(["ls-files", ".githooks", "scripts", "tests/hooks"])?
            .into_iter()
            .filter(|path| is_maintained_shell_path(path))
            .collect(),
    )
}

fn is_maintained_shell_path(path: &str) -> bool {
    let path_ref = Path::new(path);
    let extension = path_ref.extension().and_then(|value| value.to_str());

    if path.starts_with(".githooks/") {
        return extension.is_none()
            || extension.is_some_and(|value| value.eq_ignore_ascii_case("sh"));
    }

    if path.starts_with("scripts/hooks/") {
        return extension.is_none()
            || extension.is_some_and(|value| value.eq_ignore_ascii_case("sh"));
    }

    if path.starts_with("scripts/") || path.starts_with("tests/hooks/") {
        return extension.is_some_and(|value| value.eq_ignore_ascii_case("sh"));
    }

    false
}

fn build_verify_local_command(mode: &str) -> Command {
    let mut command = Command::new(Path::new("scripts").join("verify-local.sh"));
    command.arg(mode);
    command
}

fn build_lint_dead_refs_command(markdown_files: Option<&[String]>, full: bool) -> Command {
    let mut command = Command::new("cargo");
    command.args(["xtask", "lint-dead-refs"]);
    if let Some(markdown_files) = markdown_files {
        command.args(["--root", "docs"]);
        for file in markdown_files {
            command.arg("--file");
            command.arg(file);
        }
    } else if full {
        command.args(["--root", "docs"]);
    }
    command
}

fn build_markdownlint_command(markdown_files: &[String]) -> Command {
    let mut command = Command::new("pnpm");
    command.args(["exec", "markdownlint-cli2"]);
    for file in markdown_files {
        command.arg(file);
    }
    command
}

fn build_cargo_check_command(args: &[&str]) -> Command {
    let mut command = Command::new("cargo");
    command.args(args);
    command
}

fn fetch_pr_overview(selector: Option<&str>) -> Result<PrOverview> {
    let mut args = vec![
        "pr",
        "view",
        "--json",
        "number,title,url,state,headRefOid,reviewDecision,mergeStateStatus",
    ];
    if let Some(selector) = selector {
        args.insert(2, selector);
    }

    let output = run_gh_capture(args)?;
    let view: GhPrView =
        serde_json::from_str(&output).context("failed to parse `gh pr view` JSON")?;
    let (owner, repo) = parse_pr_owner_name(&view.url)?;

    Ok(PrOverview {
        owner,
        repo,
        number: view.number,
        title: view.title,
        url: view.url,
        state: view.state,
        head_sha: view.head_ref_oid,
        review_decision: view.review_decision.unwrap_or_else(|| "NONE".to_owned()),
        merge_state: view
            .merge_state_status
            .unwrap_or_else(|| "UNKNOWN".to_owned()),
    })
}

fn fetch_pr_checks(pr: &PrOverview) -> Result<Vec<PrCheckSummary>> {
    let output = run_gh_capture_allow_exit_codes(
        [
            "pr",
            "checks",
            pr.url.as_str(),
            "--json",
            "name,bucket,state",
        ],
        &[8],
    )?;
    let mut checks: Vec<GhPrCheck> =
        serde_json::from_str(&output).context("failed to parse `gh pr checks` JSON")?;
    checks.sort_by(|left, right| {
        left.bucket
            .cmp(&right.bucket)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.state.cmp(&right.state))
    });

    Ok(checks
        .into_iter()
        .map(|check| PrCheckSummary {
            name: check.name,
            bucket: if check.bucket.trim().is_empty() {
                "unknown".to_owned()
            } else {
                check.bucket
            },
            state: if check.state.trim().is_empty() {
                "UNKNOWN".to_owned()
            } else {
                check.state
            },
        })
        .collect())
}

fn fetch_code_rabbit_state(pr: &PrOverview) -> Result<Option<DoghouseCodeRabbitState>> {
    Ok(fetch_latest_code_rabbit_summary_comment(pr)?.and_then(analyze_code_rabbit_summary_comment))
}

fn fetch_latest_code_rabbit_summary_comment(
    pr: &PrOverview,
) -> Result<Option<CodeRabbitSummaryComment>> {
    let output = run_gh_capture([
        "api",
        "--paginate",
        "--slurp",
        &format!(
            "repos/{}/{}/issues/{}/comments?per_page=100&sort=updated&direction=desc",
            pr.owner, pr.repo, pr.number
        ),
    ])?;
    let pages: Vec<Vec<GhIssueComment>> =
        serde_json::from_str(&output).context("failed to parse CodeRabbit issue comments")?;
    let comments = pages
        .into_iter()
        .flatten()
        .map(CodeRabbitSummaryComment::from_issue_comment)
        .collect::<Vec<_>>();
    Ok(select_latest_code_rabbit_summary_comment(comments))
}

fn select_latest_code_rabbit_summary_comment<I>(comments: I) -> Option<CodeRabbitSummaryComment>
where
    I: IntoIterator<Item = CodeRabbitSummaryComment>,
{
    comments
        .into_iter()
        .filter(|comment| {
            comment
                .author
                .as_ref()
                .is_some_and(|author| is_code_rabbit_author_login(&author.login))
        })
        .filter(|comment| {
            comment
                .body
                .contains("This is an auto-generated comment: summarize by coderabbit.ai")
        })
        .max_by(|left, right| left.updated_at.cmp(&right.updated_at))
}

fn is_code_rabbit_author_login(login: &str) -> bool {
    matches!(
        login.to_ascii_lowercase().as_str(),
        "coderabbitai" | "coderabbitai[bot]"
    )
}

fn analyze_code_rabbit_summary_comment(
    comment: CodeRabbitSummaryComment,
) -> Option<DoghouseCodeRabbitState> {
    let callout = extract_code_rabbit_callout(&comment.body);
    let checkboxes = extract_code_rabbit_checkboxes(&comment.body)
        .into_iter()
        .filter(|checkbox| is_code_rabbit_review_action_checkbox(&checkbox.label))
        .collect::<Vec<_>>();
    let active_changes_gate = code_rabbit_summary_mentions_active_changes(&comment.body)
        || callout
            .as_ref()
            .is_some_and(|callout| code_rabbit_summary_mentions_active_changes(&callout.text));
    let rearm_actionable =
        active_changes_gate && checkboxes.iter().any(|checkbox| !checkbox.checked);

    let mut state = DoghouseCodeRabbitState {
        summary_comment_url: comment.url,
        summary_comment_database_id: comment.database_id,
        summary_comment_updated_at: comment.updated_at.clone(),
        summary_state: DoghouseCodeRabbitSummaryState::Ready,
        callout_present: callout.is_some(),
        callout_kind: callout.as_ref().map(|callout| callout.kind.clone()),
        callout_title: callout.as_ref().and_then(|callout| callout.title.clone()),
        cooldown_active: false,
        cooldown_expires_at: None,
        cooldown_remaining_seconds: None,
        rearm_actionable,
        checkboxes,
        request_review_actionable: true,
    };

    if comment
        .body
        .contains("This is an auto-generated comment: rate limited by coderabbit.ai")
        || comment.body.contains("## Rate limit exceeded")
    {
        let wait_phrase = extract_code_rabbit_wait_phrase(&comment.body)?;
        let wait_seconds = parse_code_rabbit_wait_seconds(&wait_phrase)?;
        let updated_at = OffsetDateTime::parse(&comment.updated_at, &Rfc3339).ok()?;
        let expires_at = updated_at + time::Duration::seconds(wait_seconds);
        let now = OffsetDateTime::now_utc();
        let remaining_seconds = (expires_at - now).whole_seconds().max(0);

        state.cooldown_active = remaining_seconds > 0;
        state.cooldown_expires_at = Some(expires_at.format(&Rfc3339).ok()?);
        state.cooldown_remaining_seconds = Some(remaining_seconds);
        state.summary_state = if state.cooldown_active {
            DoghouseCodeRabbitSummaryState::RateLimited
        } else if state.rearm_actionable {
            DoghouseCodeRabbitSummaryState::RearmRequired
        } else {
            DoghouseCodeRabbitSummaryState::CalloutPresent
        };
        state.request_review_actionable = remaining_seconds == 0 && !state.rearm_actionable;
        return Some(state);
    }

    if state.rearm_actionable {
        state.summary_state = DoghouseCodeRabbitSummaryState::RearmRequired;
        state.request_review_actionable = false;
    } else if state.callout_present {
        state.summary_state = DoghouseCodeRabbitSummaryState::CalloutPresent;
    }

    Some(state)
}

fn extract_code_rabbit_wait_phrase(body: &str) -> Option<String> {
    let start = body.find("Please wait **")?;
    let remainder = &body[start + "Please wait **".len()..];
    let end = remainder.find("** before requesting another review")?;
    Some(remainder[..end].trim().to_owned())
}

fn parse_code_rabbit_wait_seconds(phrase: &str) -> Option<i64> {
    let normalized = phrase.replace(',', " ");
    let mut total = 0_i64;
    let mut tokens = normalized.split_whitespace();

    while let Some(token) = tokens.next() {
        if token.eq_ignore_ascii_case("and") {
            continue;
        }

        let value = token.parse::<i64>().ok()?;
        let unit = tokens.next()?;
        let unit = unit.trim_end_matches(',');
        let factor = if unit.starts_with("hour") {
            60 * 60
        } else if unit.starts_with("minute") {
            60
        } else if unit.starts_with("second") {
            1
        } else {
            return None;
        };
        total += value * factor;
    }

    Some(total)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CodeRabbitCallout {
    kind: String,
    title: Option<String>,
    text: String,
}

fn extract_code_rabbit_callout(body: &str) -> Option<CodeRabbitCallout> {
    let lines = body.lines().collect::<Vec<_>>();
    let start = lines
        .iter()
        .position(|line| line.trim_start().starts_with("> [!"))?;

    let mut block_lines = Vec::new();
    for line in &lines[start..] {
        let trimmed = line.trim_start();
        if !trimmed.starts_with('>') {
            break;
        }
        let content = trimmed.strip_prefix('>').map_or(trimmed, str::trim_start);
        block_lines.push(content.to_owned());
    }

    if block_lines.is_empty() {
        return None;
    }

    let kind = block_lines[0]
        .strip_prefix("[!")
        .and_then(|line| line.strip_suffix(']'))
        .map(str::to_owned)?;

    let title = block_lines
        .iter()
        .skip(1)
        .map(|line| line.trim())
        .find_map(|line| {
            if line.starts_with("## ") {
                Some(line.trim_start_matches("## ").trim().to_owned())
            } else if !line.is_empty() && !line.starts_with('<') {
                Some(line.to_owned())
            } else {
                None
            }
        });

    let text = block_lines
        .iter()
        .skip(1)
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    Some(CodeRabbitCallout { kind, title, text })
}

fn extract_code_rabbit_checkboxes(body: &str) -> Vec<DoghouseCodeRabbitCheckbox> {
    body.lines()
        .filter_map(parse_code_rabbit_checkbox_line)
        .collect()
}

fn parse_code_rabbit_checkbox_line(line: &str) -> Option<DoghouseCodeRabbitCheckbox> {
    let trimmed = line.trim_start();
    let trimmed = trimmed.strip_prefix('>').map_or(trimmed, str::trim_start);

    let (checked, rest) = if let Some(rest) = trimmed.strip_prefix("- [ ]") {
        (false, rest)
    } else if let Some(rest) = trimmed.strip_prefix("- [x]") {
        (true, rest)
    } else if let Some(rest) = trimmed.strip_prefix("- [X]") {
        (true, rest)
    } else {
        return None;
    };

    if !rest.starts_with(char::is_whitespace) {
        return None;
    }

    let checkbox_id = rest.find("\"checkboxId\": \"").and_then(|start| {
        let remainder = &rest[start + "\"checkboxId\": \"".len()..];
        remainder.find('"').map(|end| remainder[..end].to_owned())
    });
    let label = rest
        .split_once("-->")
        .map_or(rest, |(_, tail)| tail)
        .trim()
        .to_owned();

    if label.is_empty() {
        return None;
    }

    Some(DoghouseCodeRabbitCheckbox {
        checked,
        label,
        checkbox_id,
    })
}

fn build_code_rabbit_rearm_body(body: &str) -> Option<(String, usize)> {
    if !code_rabbit_summary_mentions_active_changes(body) {
        return None;
    }

    let mut toggled_checkbox_count = 0_usize;
    let mut rewritten = Vec::new();
    for line in body.lines() {
        if parse_code_rabbit_checkbox_line(line).is_some_and(|checkbox| {
            !checkbox.checked && is_code_rabbit_review_action_checkbox(&checkbox.label)
        }) {
            rewritten.push(line.replacen("- [ ]", "- [x]", 1));
            toggled_checkbox_count += 1;
        } else {
            rewritten.push(line.to_owned());
        }
    }

    if toggled_checkbox_count == 0 {
        return None;
    }

    Some((rewritten.join("\n"), toggled_checkbox_count))
}

fn select_code_rabbit_nudge_body(
    comment: Option<&CodeRabbitSummaryComment>,
    state: Option<&DoghouseCodeRabbitState>,
) -> &'static str {
    if matches!(
        state.map(|state| &state.summary_state),
        Some(DoghouseCodeRabbitSummaryState::RearmRequired)
    ) || comment
        .as_ref()
        .is_some_and(|comment| code_rabbit_summary_mentions_active_changes(&comment.body))
    {
        "@coderabbitai resume"
    } else {
        "@coderabbitai review"
    }
}

fn is_code_rabbit_review_action_checkbox(label: &str) -> bool {
    let lowered = label.to_ascii_lowercase();
    (lowered.contains("resume") && lowered.contains("review"))
        || (lowered.contains("running") && lowered.contains("review"))
        || lowered.contains("trigger review")
        || lowered.contains("@coderabbitai resume")
        || lowered.contains("@coderabbitai review")
}

fn code_rabbit_summary_mentions_active_changes(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    [
        "active changes",
        "active pull request changes",
        "active development",
        "under active development",
        "actively reviewing",
        "still making changes",
        "changes are still being made",
        "reviews paused",
        "review paused",
        "review has been paused",
        "paused this review",
        "resume review",
        "resume reviews",
        "continue review",
        "re-enable",
        "enable coderabbit",
        "resume automatic review",
        "resume automatic reviews",
        "@coderabbitai resume",
        "@coderabbitai review",
        "auto_pause_after_reviewed_commits",
        "manual intervention",
    ]
    .iter()
    .any(|needle| lowered.contains(needle))
}

fn fetch_unresolved_review_threads(pr: &PrOverview) -> Result<Vec<ReviewThreadSummary>> {
    let mut threads = Vec::new();
    let mut cursor: Option<String> = None;

    loop {
        let mut args = vec![
            "api".to_owned(),
            "graphql".to_owned(),
            "-F".to_owned(),
            format!("owner={}", pr.owner),
            "-F".to_owned(),
            format!("name={}", pr.repo),
            "-F".to_owned(),
            format!("number={}", pr.number),
        ];
        if let Some(cursor_value) = cursor.as_deref() {
            args.push("-F".to_owned());
            args.push(format!("cursor={cursor_value}"));
        }
        args.push("-f".to_owned());
        args.push(
            "query=query($owner:String!, $name:String!, $number:Int!, $cursor:String) { repository(owner:$owner, name:$name) { pullRequest(number:$number) { reviewThreads(first:100, after:$cursor) { nodes { id isResolved isOutdated path line originalLine comments(first:1) { nodes { databaseId url body author { login } } } } pageInfo { hasNextPage endCursor } } } } }".to_owned(),
        );

        let output = run_gh_capture(args)?;
        let page: ReviewThreadsQueryResponse = serde_json::from_str(&output)
            .context("failed to parse review thread GraphQL response")?;
        let connection = page.data.repository.pull_request.review_threads;

        for thread in connection.nodes {
            if thread.is_resolved {
                continue;
            }
            let first_comment = thread.comments.nodes.into_iter().next();
            let comment_id = first_comment
                .as_ref()
                .and_then(|comment| comment.database_id);
            let author = first_comment
                .as_ref()
                .and_then(|comment| comment.author.as_ref().map(|author| author.login.clone()));
            let url = first_comment.as_ref().map(|comment| comment.url.clone());
            let preview = first_comment.as_ref().map_or_else(
                || "<no comment preview>".to_owned(),
                |comment| preview_comment_body(&comment.body),
            );
            threads.push(ReviewThreadSummary {
                thread_id: thread.id,
                comment_id,
                author,
                url,
                path: thread.path,
                line: thread.line.or(thread.original_line),
                is_outdated: thread.is_outdated,
                preview,
            });
        }

        if !connection.page_info.has_next_page {
            break;
        }
        cursor = connection.page_info.end_cursor;
    }

    Ok(threads)
}

fn resolve_thread_targets(args: &PrThreadsResolveArgs) -> Result<Vec<ReviewThreadSummary>> {
    if args.all {
        if !args.thread_ids.is_empty() {
            bail!("thread ids cannot be combined with --all");
        }
        let overview = fetch_pr_overview(args.selector.as_deref())?;
        return fetch_unresolved_review_threads(&overview);
    }

    if args.selector.is_some() {
        bail!("--selector can only be used with --all");
    }
    if args.thread_ids.is_empty() {
        bail!("provide one or more thread ids, or use --all");
    }

    Ok(args
        .thread_ids
        .iter()
        .map(|thread_id| ReviewThreadSummary {
            thread_id: thread_id.clone(),
            comment_id: None,
            author: None,
            url: None,
            path: None,
            line: None,
            is_outdated: false,
            preview: "<explicit thread id>".to_owned(),
        })
        .collect())
}

fn load_reply_body(body: Option<&str>, body_file: Option<&Path>) -> Result<String> {
    match (body, body_file) {
        (Some(_), Some(_)) | (None, None) => {
            bail!("pass exactly one of --body or --body-file")
        }
        (Some(body), None) => Ok(body.to_owned()),
        (None, Some(path)) => std::fs::read_to_string(path)
            .with_context(|| format!("failed to read reply body from {}", path.display())),
    }
}

const COMMENT_PREVIEW_LIMIT: usize = 100;

fn preview_comment_body(body: &str) -> String {
    let first_line = body
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("<empty>");
    if first_line.chars().count() <= COMMENT_PREVIEW_LIMIT {
        return first_line.to_owned();
    }
    let truncated: String = first_line.chars().take(COMMENT_PREVIEW_LIMIT - 1).collect();
    format!("{truncated}…")
}

fn parse_pr_owner_name(url: &str) -> Result<(String, String)> {
    let trimmed = url.trim();
    let path_start = trimmed
        .find("github.com/")
        .map(|idx| idx + "github.com/".len())
        .context("unexpected PR URL: missing github.com/ segment")?;
    let path = &trimmed[path_start..];
    let parts: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
    if parts.len() < 4 || parts[2] != "pull" {
        bail!("unexpected PR URL: {trimmed}");
    }
    Ok((parts[0].to_owned(), parts[1].to_owned()))
}

fn run_gh_capture<I, S>(args: I) -> Result<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    run_gh_capture_allow_exit_codes(args, &[])
}

fn run_gh_capture_allow_exit_codes<I, S>(args: I, allowed_exit_codes: &[i32]) -> Result<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = Command::new("gh")
        .args(args)
        .output()
        .context("failed to spawn `gh` (is GitHub CLI installed?)")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");
    if output.status.success()
        || output
            .status
            .code()
            .is_some_and(|code| allowed_exit_codes.contains(&code))
    {
        return Ok(stdout.into_owned());
    }
    if is_gh_auth_error(&combined) {
        bail!("Auth error—run `gh auth login` and retry.");
    }
    let message = combined.trim();
    if message.is_empty() {
        bail!("gh command failed with exit status {}", output.status);
    }
    bail!("{message}");
}

fn is_gh_auth_error(output: &str) -> bool {
    let lowered = output.to_ascii_lowercase();
    lowered.contains("authentication")
        || lowered.contains("to authenticate")
        || lowered.contains("authentication required")
        || lowered.contains("you must authenticate")
        || lowered.contains("bad credentials")
        || lowered.contains("not logged in")
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PrOverview {
    owner: String,
    repo: String,
    number: u64,
    title: String,
    url: String,
    state: String,
    head_sha: String,
    review_decision: String,
    merge_state: String,
}

impl PrOverview {
    fn short_head_sha(&self) -> String {
        self.head_sha.chars().take(12).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ReviewThreadSummary {
    thread_id: String,
    comment_id: Option<u64>,
    author: Option<String>,
    url: Option<String>,
    path: Option<String>,
    line: Option<u32>,
    is_outdated: bool,
    preview: String,
}

impl ReviewThreadSummary {
    fn display_location(&self) -> String {
        match (&self.path, self.line) {
            (Some(path), Some(line)) => format!("{path}:{line}"),
            (Some(path), None) => path.clone(),
            (None, _) => "<unknown path>".to_owned(),
        }
    }
}

struct PreflightCheck {
    label: String,
    command: Command,
}

impl PreflightCheck {
    fn new(label: impl Into<String>, command: Command) -> Self {
        Self {
            label: label.into(),
            command,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PreflightScope {
    verify_mode: &'static str,
    run_dead_refs: bool,
    markdown_files: Option<Vec<String>>,
    run_runtime_schema_explicit: bool,
    run_feature_contracts: bool,
    shell_files: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct GhPrView {
    number: u64,
    title: String,
    url: String,
    state: String,
    #[serde(rename = "headRefOid")]
    head_ref_oid: String,
    #[serde(rename = "reviewDecision")]
    review_decision: Option<String>,
    #[serde(rename = "mergeStateStatus")]
    merge_state_status: Option<String>,
}

#[derive(Deserialize)]
struct GhPrCheck {
    name: String,
    #[serde(default)]
    bucket: String,
    #[serde(default)]
    state: String,
}

#[derive(Deserialize)]
struct GhIssueComment {
    id: u64,
    #[serde(rename = "html_url")]
    html_url: String,
    body: String,
    #[serde(rename = "updated_at")]
    updated_at: String,
    user: Option<GhIssueCommentUser>,
}

#[derive(Deserialize)]
struct GhIssueCommentUser {
    login: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct CodeRabbitSummaryComment {
    url: String,
    #[serde(rename = "databaseId")]
    database_id: Option<u64>,
    body: String,
    #[serde(rename = "updatedAt")]
    updated_at: String,
    author: Option<CodeRabbitCommentAuthor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct CodeRabbitCommentAuthor {
    login: String,
}

impl CodeRabbitSummaryComment {
    fn from_issue_comment(comment: GhIssueComment) -> Self {
        Self {
            url: comment.html_url,
            database_id: Some(comment.id),
            body: comment.body,
            updated_at: comment.updated_at,
            author: comment
                .user
                .map(|user| CodeRabbitCommentAuthor { login: user.login }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PrCheckSummary {
    name: String,
    bucket: String,
    state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PrSnapshotArtifact {
    recorded_at: String,
    filename_stamp: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sortie_intent: Option<DoghouseSortieIntent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    code_rabbit: Option<DoghouseCodeRabbitState>,
    pr: PrSnapshotOverview,
    #[serde(default)]
    blockers: Vec<String>,
    #[serde(default)]
    checks: Vec<PrCheckSummary>,
    #[serde(default)]
    grouped_checks: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    unresolved_threads: Vec<ReviewThreadSummary>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
#[value(rename_all = "snake_case")]
enum DoghouseSortieIntent {
    ManualProbe,
    PostPush,
    FixBatch,
    MergeCheck,
    Resume,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum DoghouseCodeRabbitSummaryState {
    Ready,
    RateLimited,
    RearmRequired,
    CalloutPresent,
}

fn default_code_rabbit_summary_state() -> DoghouseCodeRabbitSummaryState {
    DoghouseCodeRabbitSummaryState::Ready
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct DoghouseCodeRabbitCheckbox {
    checked: bool,
    label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    checkbox_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct DoghouseCodeRabbitState {
    summary_comment_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    summary_comment_database_id: Option<u64>,
    summary_comment_updated_at: String,
    #[serde(default = "default_code_rabbit_summary_state")]
    summary_state: DoghouseCodeRabbitSummaryState,
    #[serde(default)]
    callout_present: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    callout_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    callout_title: Option<String>,
    cooldown_active: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cooldown_expires_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cooldown_remaining_seconds: Option<i64>,
    #[serde(default)]
    rearm_actionable: bool,
    #[serde(default)]
    checkboxes: Vec<DoghouseCodeRabbitCheckbox>,
    request_review_actionable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PrSnapshotOverview {
    number: u64,
    url: String,
    title: String,
    #[serde(default = "default_pr_state")]
    state: String,
    head_sha: String,
    head_sha_short: String,
    #[serde(default = "default_review_decision")]
    review_decision: String,
    #[serde(default = "default_merge_state")]
    merge_state: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PrSnapshotPaths {
    snapshot_json: PathBuf,
    snapshot_markdown: PathBuf,
    latest_json: PathBuf,
    latest_markdown: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DoghouseJsonlPaths {
    snapshot_jsonl: PathBuf,
    latest_jsonl: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PrSnapshotDeltaPaths {
    snapshot_json: PathBuf,
    snapshot_markdown: PathBuf,
    latest_json: PathBuf,
    latest_markdown: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DoghouseBaselineSelection {
    strategy: DoghouseBaselineStrategy,
    snapshot: PrSnapshotArtifact,
    newer_snapshot_count: usize,
    newer_semantic_change_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum DoghouseBaselineStrategy {
    PreviousDifferentHead,
    PreviousSemanticChange,
    ImmediatePrevious,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DoghouseComparisonAssessment {
    selection: Option<DoghouseBaselineStrategy>,
    trust: DoghouseComparisonTrust,
    quality: DoghouseComparisonQuality,
    same_head: Option<bool>,
    semantically_changed: Option<bool>,
    baseline_age_seconds: Option<i64>,
    newer_snapshot_count: Option<usize>,
    newer_semantic_change_count: Option<usize>,
    stale: bool,
    noisy: bool,
    reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum DoghouseComparisonTrust {
    Strong,
    Usable,
    Weak,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum DoghouseComparisonQuality {
    GoodEnough,
    Stale,
    Noisy,
    StaleAndNoisy,
    InitialCapture,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PrSnapshotDelta {
    previous: PrSnapshotDeltaRef,
    current: PrSnapshotDeltaRef,
    blockers_added: Vec<String>,
    blockers_removed: Vec<String>,
    blockers_unchanged: Vec<String>,
    threads_opened: Vec<ReviewThreadSummary>,
    threads_resolved: Vec<ReviewThreadSummary>,
    threads_persisting: Vec<ReviewThreadSummary>,
    improved_checks: Vec<PrCheckTransition>,
    regressed_checks: Vec<PrCheckTransition>,
    shifted_checks: Vec<PrCheckTransition>,
    added_checks: Vec<PrCheckSummary>,
    removed_checks: Vec<PrCheckSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PrSnapshotDeltaRef {
    number: u64,
    recorded_at: String,
    state: String,
    head_sha: String,
    head_sha_short: String,
    blocker_count: usize,
    unresolved_thread_count: usize,
}

impl PrSnapshotDeltaRef {
    fn from_snapshot(snapshot: &PrSnapshotArtifact) -> Self {
        Self {
            number: snapshot.pr.number,
            recorded_at: snapshot.recorded_at.clone(),
            state: snapshot.pr.state.clone(),
            head_sha: snapshot.pr.head_sha.clone(),
            head_sha_short: snapshot.pr.head_sha_short.clone(),
            blocker_count: snapshot.blockers.len(),
            unresolved_thread_count: snapshot.unresolved_threads.len(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum PrCheckTransitionKind {
    Improved,
    Regressed,
    Shifted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PrCheckTransition {
    name: String,
    previous_bucket: String,
    previous_state: String,
    current_bucket: String,
    current_state: String,
    kind: PrCheckTransitionKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct DoghouseSnapshotEvent {
    kind: &'static str,
    pr_number: u64,
    pr_url: String,
    pr_title: String,
    pr_state: String,
    recorded_at: String,
    sortie_intent: Option<DoghouseSortieIntent>,
    head_sha: String,
    head_sha_short: String,
    review_decision: String,
    merge_state: String,
    blocker_count: usize,
    blockers: Vec<String>,
    unresolved_thread_count: usize,
    check_counts: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct DoghouseIntentEvent {
    kind: &'static str,
    current_intent: Option<DoghouseSortieIntent>,
    baseline_intent: Option<DoghouseSortieIntent>,
    baseline_intent_known: bool,
    changed_from_baseline: Option<bool>,
}

impl DoghouseIntentEvent {
    fn from_snapshots(
        snapshot: &PrSnapshotArtifact,
        baseline: Option<&DoghouseBaselineSelection>,
    ) -> Self {
        let current_intent = snapshot.sortie_intent;
        let baseline_intent = baseline.and_then(|selection| selection.snapshot.sortie_intent);

        Self {
            kind: "doghouse.intent",
            current_intent,
            baseline_intent,
            baseline_intent_known: baseline_intent.is_some(),
            changed_from_baseline: current_intent
                .zip(baseline_intent)
                .map(|(current, previous)| current != previous),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct DoghouseCodeRabbitEvent {
    kind: &'static str,
    summary_state: DoghouseCodeRabbitSummaryState,
    currently_reviewing: bool,
    check_bucket: Option<String>,
    check_state: Option<String>,
    callout_present: bool,
    callout_kind: Option<String>,
    callout_title: Option<String>,
    cooldown_active: bool,
    cooldown_expires_at: Option<String>,
    cooldown_remaining_seconds: Option<i64>,
    checkbox_count: usize,
    unchecked_checkbox_count: usize,
    checkboxes: Vec<DoghouseCodeRabbitCheckbox>,
    rearm_actionable: bool,
    request_review_actionable: bool,
    summary_comment_url: Option<String>,
    summary_comment_database_id: Option<u64>,
    summary_comment_updated_at: Option<String>,
}

impl DoghouseCodeRabbitEvent {
    fn from_snapshot(snapshot: &PrSnapshotArtifact) -> Self {
        let check = snapshot
            .checks
            .iter()
            .find(|check| check.name.eq_ignore_ascii_case("CodeRabbit"));
        let currently_reviewing = check.is_some_and(|check| check.bucket == "pending");
        let summary_state = snapshot
            .code_rabbit
            .as_ref()
            .map_or(DoghouseCodeRabbitSummaryState::Ready, |state| {
                state.summary_state.clone()
            });
        let checkbox_count = snapshot
            .code_rabbit
            .as_ref()
            .map_or(0, |state| state.checkboxes.len());
        let unchecked_checkbox_count = snapshot.code_rabbit.as_ref().map_or(0, |state| {
            state
                .checkboxes
                .iter()
                .filter(|checkbox| !checkbox.checked)
                .count()
        });

        Self {
            kind: "doghouse.coderabbit",
            summary_state,
            currently_reviewing,
            check_bucket: check.map(|check| check.bucket.clone()),
            check_state: check.map(|check| check.state.clone()),
            callout_present: snapshot
                .code_rabbit
                .as_ref()
                .is_some_and(|state| state.callout_present),
            callout_kind: snapshot
                .code_rabbit
                .as_ref()
                .and_then(|state| state.callout_kind.clone()),
            callout_title: snapshot
                .code_rabbit
                .as_ref()
                .and_then(|state| state.callout_title.clone()),
            cooldown_active: snapshot
                .code_rabbit
                .as_ref()
                .is_some_and(|state| state.cooldown_active),
            cooldown_expires_at: snapshot
                .code_rabbit
                .as_ref()
                .and_then(|state| state.cooldown_expires_at.clone()),
            cooldown_remaining_seconds: snapshot
                .code_rabbit
                .as_ref()
                .and_then(|state| state.cooldown_remaining_seconds),
            checkbox_count,
            unchecked_checkbox_count,
            checkboxes: snapshot
                .code_rabbit
                .as_ref()
                .map_or_else(Vec::new, |state| state.checkboxes.clone()),
            rearm_actionable: snapshot
                .code_rabbit
                .as_ref()
                .is_some_and(|state| state.rearm_actionable),
            request_review_actionable: snapshot.code_rabbit.as_ref().map_or_else(
                || !currently_reviewing,
                |state| state.request_review_actionable && !currently_reviewing,
            ),
            summary_comment_url: snapshot
                .code_rabbit
                .as_ref()
                .map(|state| state.summary_comment_url.clone()),
            summary_comment_database_id: snapshot
                .code_rabbit
                .as_ref()
                .and_then(|state| state.summary_comment_database_id),
            summary_comment_updated_at: snapshot
                .code_rabbit
                .as_ref()
                .map(|state| state.summary_comment_updated_at.clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct DoghouseCodeRabbitRearmEvent {
    kind: &'static str,
    pr_number: u64,
    summary_comment_url: String,
    summary_comment_database_id: Option<u64>,
    toggled_checkbox_count: usize,
    updated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct DoghouseCodeRabbitNudgeEvent {
    kind: &'static str,
    pr_number: u64,
    command: String,
    posted_comment_url: String,
    posted_comment_database_id: Option<u64>,
    summary_comment_url: Option<String>,
    summary_comment_database_id: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct DoghouseBaselineEvent {
    kind: &'static str,
    selection: String,
    found: bool,
    recorded_at: Option<String>,
    head_sha: Option<String>,
    head_sha_short: Option<String>,
    blocker_count: Option<usize>,
    unresolved_thread_count: Option<usize>,
}

impl DoghouseBaselineEvent {
    fn selection_label(strategy: &DoghouseBaselineStrategy) -> String {
        match strategy {
            DoghouseBaselineStrategy::PreviousDifferentHead => "previous_different_head".to_owned(),
            DoghouseBaselineStrategy::PreviousSemanticChange => {
                "previous_semantic_change".to_owned()
            }
            DoghouseBaselineStrategy::ImmediatePrevious => "immediate_previous".to_owned(),
        }
    }

    fn from_selection(selection: Option<&DoghouseBaselineSelection>) -> Self {
        match selection {
            Some(selection) => Self {
                kind: "doghouse.baseline",
                selection: Self::selection_label(&selection.strategy),
                found: true,
                recorded_at: Some(selection.snapshot.recorded_at.clone()),
                head_sha: Some(selection.snapshot.pr.head_sha.clone()),
                head_sha_short: Some(selection.snapshot.pr.head_sha_short.clone()),
                blocker_count: Some(selection.snapshot.blockers.len()),
                unresolved_thread_count: Some(selection.snapshot.unresolved_threads.len()),
            },
            None => Self {
                kind: "doghouse.baseline",
                selection: "none".to_owned(),
                found: false,
                recorded_at: None,
                head_sha: None,
                head_sha_short: None,
                blocker_count: None,
                unresolved_thread_count: None,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct DoghouseComparisonEvent {
    kind: &'static str,
    selection: String,
    trust: DoghouseComparisonTrust,
    quality: DoghouseComparisonQuality,
    baseline_found: bool,
    same_head: Option<bool>,
    semantically_changed: Option<bool>,
    baseline_age_seconds: Option<i64>,
    newer_snapshot_count: Option<usize>,
    newer_semantic_change_count: Option<usize>,
    stale: bool,
    noisy: bool,
    reasons: Vec<String>,
}

impl DoghouseComparisonEvent {
    fn from_assessment(assessment: &DoghouseComparisonAssessment) -> Self {
        Self {
            kind: "doghouse.comparison",
            selection: assessment
                .selection
                .as_ref()
                .map_or_else(|| "none".to_owned(), DoghouseBaselineEvent::selection_label),
            trust: assessment.trust.clone(),
            quality: assessment.quality.clone(),
            baseline_found: assessment.selection.is_some(),
            same_head: assessment.same_head,
            semantically_changed: assessment.semantically_changed,
            baseline_age_seconds: assessment.baseline_age_seconds,
            newer_snapshot_count: assessment.newer_snapshot_count,
            newer_semantic_change_count: assessment.newer_semantic_change_count,
            stale: assessment.stale,
            noisy: assessment.noisy,
            reasons: assessment.reasons.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct DoghouseDeltaEvent {
    kind: &'static str,
    previous_head_sha: String,
    previous_head_sha_short: String,
    current_head_sha: String,
    current_head_sha_short: String,
    blockers_added: Vec<String>,
    blockers_removed: Vec<String>,
    blockers_unchanged_count: usize,
    threads_opened: Vec<ReviewThreadSummary>,
    threads_resolved: Vec<ReviewThreadSummary>,
    threads_persisting_count: usize,
    improved_checks: Vec<PrCheckTransition>,
    regressed_checks: Vec<PrCheckTransition>,
    shifted_checks: Vec<PrCheckTransition>,
    added_checks: Vec<PrCheckSummary>,
    removed_checks: Vec<PrCheckSummary>,
}

impl DoghouseDeltaEvent {
    fn from_delta(delta: &PrSnapshotDelta) -> Self {
        Self {
            kind: "doghouse.delta",
            previous_head_sha: delta.previous.head_sha.clone(),
            previous_head_sha_short: delta.previous.head_sha_short.clone(),
            current_head_sha: delta.current.head_sha.clone(),
            current_head_sha_short: delta.current.head_sha_short.clone(),
            blockers_added: delta.blockers_added.clone(),
            blockers_removed: delta.blockers_removed.clone(),
            blockers_unchanged_count: delta.blockers_unchanged.len(),
            threads_opened: delta.threads_opened.clone(),
            threads_resolved: delta.threads_resolved.clone(),
            threads_persisting_count: delta.threads_persisting.len(),
            improved_checks: delta.improved_checks.clone(),
            regressed_checks: delta.regressed_checks.clone(),
            shifted_checks: delta.shifted_checks.clone(),
            added_checks: delta.added_checks.clone(),
            removed_checks: delta.removed_checks.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct DoghouseNextAction {
    kind: &'static str,
    sortie_intent: Option<DoghouseSortieIntent>,
    action: String,
    reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct DoghouseArtifactsEvent {
    kind: &'static str,
    snapshot_json: String,
    snapshot_markdown: String,
    latest_json: String,
    latest_markdown: String,
    delta_json: Option<String>,
    delta_markdown: Option<String>,
    latest_delta_json: Option<String>,
    latest_delta_markdown: Option<String>,
    sortie_jsonl: String,
    latest_sortie_jsonl: String,
}

fn default_pr_state() -> String {
    "OPEN".to_owned()
}

fn default_review_decision() -> String {
    "NONE".to_owned()
}

fn default_merge_state() -> String {
    "UNKNOWN".to_owned()
}

#[derive(Deserialize)]
struct ReviewThreadsQueryResponse {
    data: ReviewThreadsQueryData,
}

#[derive(Deserialize)]
struct ReviewThreadsQueryData {
    repository: ReviewThreadsRepository,
}

#[derive(Deserialize)]
struct ReviewThreadsRepository {
    #[serde(rename = "pullRequest")]
    pull_request: ReviewThreadsPullRequest,
}

#[derive(Deserialize)]
struct ReviewThreadsPullRequest {
    #[serde(rename = "reviewThreads")]
    review_threads: ReviewThreadsConnection,
}

#[derive(Deserialize)]
struct ReviewThreadsConnection {
    nodes: Vec<ReviewThreadNode>,
    #[serde(rename = "pageInfo")]
    page_info: ReviewThreadsPageInfo,
}

#[derive(Deserialize)]
struct ReviewThreadsPageInfo {
    #[serde(rename = "hasNextPage")]
    has_next_page: bool,
    #[serde(rename = "endCursor")]
    end_cursor: Option<String>,
}

#[derive(Deserialize)]
struct ReviewThreadNode {
    id: String,
    #[serde(rename = "isResolved")]
    is_resolved: bool,
    #[serde(rename = "isOutdated")]
    is_outdated: bool,
    path: Option<String>,
    line: Option<u32>,
    #[serde(rename = "originalLine")]
    original_line: Option<u32>,
    comments: ReviewThreadComments,
}

#[derive(Deserialize)]
struct ReviewThreadComments {
    nodes: Vec<ReviewThreadCommentNode>,
}

#[derive(Deserialize)]
struct ReviewThreadCommentNode {
    #[serde(rename = "databaseId")]
    database_id: Option<u64>,
    url: String,
    body: String,
    author: Option<ReviewThreadAuthor>,
}

#[derive(Deserialize)]
struct ReviewThreadAuthor {
    login: String,
}

#[derive(Deserialize)]
struct ReviewReplyResponse {
    url: Option<String>,
    #[serde(rename = "html_url")]
    html_url: Option<String>,
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

    /// Explicit markdown files to scan instead of walking a root directory.
    #[arg(long = "file")]
    files: Vec<PathBuf>,

    /// Also check non-markdown links (images, HTML, etc.).
    #[arg(long)]
    all: bool,
}

fn run_lint_dead_refs(args: LintDeadRefsArgs) -> Result<()> {
    let (docs_root, mut md_files) = if args.files.is_empty() {
        let root = &args.root;
        if !root.is_dir() {
            bail!("{} is not a directory", root.display());
        }

        // Derive the VitePress docs root for root-relative link resolution.
        // When --root is a subdirectory (e.g. docs/meta), root-relative links
        // like `/guide/...` must still resolve against the top-level docs dir.
        let docs_root = find_docs_root(root);
        let mut md_files = Vec::new();
        collect_md_files(root, &mut md_files)?;
        (docs_root, md_files)
    } else {
        let mut md_files = Vec::new();
        for file in &args.files {
            if !file.is_file() {
                bail!("{} is not a file", file.display());
            }
            if file.extension().and_then(|ext| ext.to_str()) != Some("md") {
                bail!("{} is not a markdown file", file.display());
            }
            md_files.push(file.clone());
        }
        (find_docs_root(&args.root), md_files)
    };
    md_files.sort();

    let mut broken: Vec<(PathBuf, usize, String, PathBuf)> = Vec::new();

    for file in &md_files {
        let content = std::fs::read_to_string(file)
            .with_context(|| format!("failed to read {}", file.display()))?;

        for (raw_target, line_no) in extract_link_targets(&content) {
            // Skip external URLs
            if raw_target.starts_with("http://")
                || raw_target.starts_with("https://")
                || raw_target.starts_with("mailto:")
            {
                continue;
            }
            // Skip pure anchors
            if raw_target.starts_with('#') {
                continue;
            }

            // Strip fragment anchor
            let target = raw_target.split('#').next().unwrap_or(&raw_target);

            // By default, only check .md and extensionless links.
            // With --all, check everything.
            if !args.all {
                let ext = Path::new(target).extension().and_then(|e| e.to_str());
                if ext.is_some_and(|e| e != "md") {
                    continue;
                }
            }

            if let Some(resolved) = try_resolve_link(file, target, &docs_root) {
                broken.push((file.clone(), line_no, raw_target, resolved));
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

/// Extract markdown link destinations using `pulldown-cmark`.
///
/// Returns `(destination, line_number)` pairs. Handles title text,
/// balanced parentheses in URLs, and angle-bracketed links correctly.
fn extract_link_targets(content: &str) -> Vec<(String, usize)> {
    use pulldown_cmark::{Event, Options, Parser, Tag};

    let parser = Parser::new_ext(content, Options::all());
    let mut results = Vec::new();
    // Track byte offset → line number
    let line_starts: Vec<usize> = std::iter::once(0)
        .chain(content.match_indices('\n').map(|(i, _)| i + 1))
        .collect();

    for (event, range) in parser.into_offset_iter() {
        if let Event::Start(Tag::Link { dest_url, .. }) = event {
            let dest = dest_url.into_string();
            if !dest.is_empty() {
                let line = line_starts.partition_point(|&s| s <= range.start);
                results.push((dest, line));
            }
        }
    }

    results
}

/// Find the git repository root via `git rev-parse --show-toplevel`.
fn find_repo_root() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("failed to run git rev-parse --show-toplevel")?;
    if !output.status.success() {
        bail!("not inside a git repository");
    }
    Ok(PathBuf::from(std::str::from_utf8(&output.stdout)?.trim()))
}

/// Find the VitePress docs root directory.
///
/// Walks up from `start` looking for `.vitepress/config.ts`. Falls back
/// to `start` itself if no config is found (single-level scan).
fn find_docs_root(start: &Path) -> PathBuf {
    let abs = std::fs::canonicalize(start).unwrap_or_else(|_| start.to_path_buf());
    let mut dir = abs.as_path();
    loop {
        if dir.join(".vitepress/config.ts").exists() || dir.join(".vitepress/config.mts").exists() {
            return dir.to_path_buf();
        }
        match dir.parent() {
            Some(parent) if parent != dir => dir = parent,
            _ => break,
        }
    }
    start.to_path_buf()
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
    let has_extension = Path::new(target).extension().is_some();
    if !has_extension {
        let stem = primary.file_name().unwrap_or_default().to_string_lossy();
        candidates.push(primary.with_file_name(format!("{stem}.md")));
        candidates.push(primary.with_file_name(format!("{stem}.html")));
    }

    candidates
}

/// Collect `.md` files under `dir` from git-tracked and untracked (but not
/// ignored) paths.
///
/// Uses `git ls-files --cached --others --exclude-standard` so that
/// gitignored files (e.g. build artifacts in `.vitepress/dist/`) are
/// excluded, while new files not yet staged are still picked up.
fn collect_md_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let output = Command::new("git")
        .args([
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
        ])
        .arg(dir)
        .output()
        .context("failed to run git ls-files")?;
    if !output.status.success() {
        bail!(
            "git ls-files failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    for entry in output.stdout.split(|&b| b == 0) {
        if entry.is_empty() {
            continue;
        }
        let path = PathBuf::from(std::str::from_utf8(entry).with_context(|| {
            format!(
                "git ls-files entry is not valid UTF-8: {:?}",
                String::from_utf8_lossy(entry)
            )
        })?);
        if path.extension().is_some_and(|ext| ext == "md") {
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

    // 1) SPDX header repair (scoped to `root` so --root is respected)
    if Path::new("scripts/ensure_spdx.sh").exists() {
        println!("markdown-fix: repairing SPDX headers...");
        let spdx_paths: Vec<&str> = md_files.iter().filter_map(|p| p.to_str()).collect();
        if !spdx_paths.is_empty() {
            let mut cmd = Command::new("bash");
            cmd.arg("scripts/ensure_spdx.sh")
                .env("ECHO_AUTO_FMT", "1")
                .args(&spdx_paths);
            let status = cmd
                .status()
                .context("failed to run scripts/ensure_spdx.sh")?;
            if !status.success() {
                bail!("markdown-fix: SPDX header repair failed");
            }
        }
    }

    let file_args: Vec<&str> = md_files
        .iter()
        .filter_map(|p| {
            if let Some(s) = p.to_str() {
                Some(s)
            } else {
                eprintln!("markdown-fix: skipping non-UTF-8 path: {}", p.display());
                None
            }
        })
        .collect();

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
    Command::new(cmd)
        .arg("--version")
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
        files: Vec::new(),
        all: args.all,
    };
    run_lint_dead_refs(refs_args)
}

fn run_man_pages(args: ManPagesArgs) -> Result<()> {
    let out_dir = &args.out;
    let pages = render_man_pages()?;

    if args.check {
        let checks = pages
            .iter()
            .map(|(filename, bytes)| (filename.as_str(), out_dir.join(filename), bytes.as_slice()))
            .collect::<Vec<_>>();
        let checks = checks
            .iter()
            .map(|(label, path, bytes)| (*label, path, *bytes))
            .collect::<Vec<_>>();
        check_artifacts_current(&checks)?;
        check_no_stale_man_pages(out_dir, &pages)?;
        println!("Man pages are current in {}", out_dir.display());
        return Ok(());
    }

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

    for (filename, bytes) in pages {
        let path = out_dir.join(&filename);
        std::fs::write(&path, &bytes)
            .with_context(|| format!("failed to write {}", path.display()))?;
        println!("  wrote {}", path.display());
    }

    println!("Man pages generated in {}", out_dir.display());
    Ok(())
}

fn render_man_pages() -> Result<Vec<(String, Vec<u8>)>> {
    use clap::CommandFactory;

    let cmd = warp_cli::cli::Cli::command();
    let mut pages = Vec::new();

    let man = clap_mangen::Man::new(cmd.clone());
    let mut buf: Vec<u8> = Vec::new();
    man.render(&mut buf)
        .context("failed to render echo-cli.1")?;
    trim_trailing_ascii_whitespace(&mut buf);
    pages.push(("echo-cli.1".to_string(), buf));

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
        trim_trailing_ascii_whitespace(&mut buf);
        pages.push((format!("echo-cli-{sub_name}.1"), buf));
    }

    Ok(pages)
}

fn trim_trailing_ascii_whitespace(bytes: &mut Vec<u8>) {
    let mut out = Vec::with_capacity(bytes.len());
    for line in bytes.split_inclusive(|byte| *byte == b'\n') {
        let has_newline = line.last() == Some(&b'\n');
        let body = if has_newline {
            &line[..line.len() - 1]
        } else {
            line
        };
        let trimmed_len = body
            .iter()
            .rposition(|byte| !byte.is_ascii_whitespace())
            .map_or(0, |idx| idx + 1);
        out.extend_from_slice(&body[..trimmed_len]);
        if has_newline {
            out.push(b'\n');
        }
    }
    *bytes = out;
}

fn check_no_stale_man_pages(out_dir: &Path, pages: &[(String, Vec<u8>)]) -> Result<()> {
    let expected = pages
        .iter()
        .map(|(filename, _)| filename.as_str())
        .collect::<BTreeSet<_>>();
    let mut stale = Vec::new();
    if let Ok(entries) = std::fs::read_dir(out_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("echo-cli")
                && name.ends_with(".1")
                && !expected.contains(name.as_ref())
            {
                stale.push(entry.path());
            }
        }
    }
    if stale.is_empty() {
        Ok(())
    } else {
        bail!(
            "stale man page(s):\n{}",
            stale
                .into_iter()
                .map(|path| format!("  - {}", path.display()))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn assert_ok<T, E: std::fmt::Display>(result: Result<T, E>, context: &str) -> T {
        match result {
            Ok(value) => value,
            Err(err) => unreachable!("{context}: {err}"),
        }
    }

    fn command_program_and_args(command: &Command) -> (String, Vec<String>) {
        let program = command.get_program().to_string_lossy().into_owned();
        let args = command
            .get_args()
            .map(|value| value.to_string_lossy().into_owned())
            .collect();
        (program, args)
    }

    #[test]
    fn man_pages_render_top_level_and_subcommands() {
        let pages = assert_ok(render_man_pages(), "man pages should render");
        let filenames = pages
            .iter()
            .map(|(filename, _)| filename.as_str())
            .collect::<BTreeSet<_>>();

        assert!(filenames.contains("echo-cli.1"));
        assert!(filenames.contains("echo-cli-verify.1"));
        assert!(filenames.contains("echo-cli-bench.1"));
        assert!(filenames.contains("echo-cli-inspect.1"));
    }

    #[test]
    fn test_slice_settlement_uses_lib_filter_not_integration_scan() {
        let commands = build_test_slice_commands(TestSlice::Settlement);
        assert_eq!(commands.len(), 1);
        let (program, args) = command_program_and_args(&commands[0]);
        assert_eq!(program, "cargo");
        assert_eq!(
            args,
            vec!["test", "-p", "warp-core", "--lib", "settlement::tests"]
        );
    }

    #[test]
    fn test_slice_warp_core_smoke_stays_explicit() {
        let commands = build_test_slice_commands(TestSlice::WarpCoreSmoke);
        assert_eq!(commands.len(), 2);

        let (program, args) = command_program_and_args(&commands[0]);
        assert_eq!(program, "cargo");
        assert_eq!(args, vec!["test", "-p", "warp-core", "--lib"]);

        let (program, args) = command_program_and_args(&commands[1]);
        assert_eq!(program, "cargo");
        assert_eq!(
            args,
            vec!["test", "-p", "warp-core", "--test", "strand_contract_tests"]
        );
    }

    #[test]
    fn wesley_sync_extracts_rust_and_typescript_schema_hashes() {
        let hash = "d55d6000b43562e7be04702cdd4335452d1eb6df1f0fbea924e4c6434fff2871";
        assert_eq!(
            extract_assignment_string(
                &format!("pub const SCHEMA_SHA256: &str = \"{hash}\";"),
                "SCHEMA_SHA256"
            ),
            Some(hash.to_owned())
        );
        assert_eq!(
            extract_assignment_string(
                &format!("export const SCHEMA_HASH = '{hash}';"),
                "SCHEMA_HASH"
            ),
            Some(hash.to_owned())
        );
    }

    #[test]
    fn wesley_sync_accepts_only_sha256_hex_schema_hashes() {
        assert!(is_sha256_hex(
            "d55d6000b43562e7be04702cdd4335452d1eb6df1f0fbea924e4c6434fff2871"
        ));
        assert!(!is_sha256_hex("d55d6000"));
        assert!(!is_sha256_hex(
            "z55d6000b43562e7be04702cdd4335452d1eb6df1f0fbea924e4c6434fff2871"
        ));
    }

    fn sample_pr_overview() -> PrOverview {
        PrOverview {
            owner: "flyingrobots".to_owned(),
            repo: "echo".to_owned(),
            number: 308,
            title: "Add PR workflow hardening".to_owned(),
            url: "https://github.com/flyingrobots/echo/pull/308".to_owned(),
            state: "OPEN".to_owned(),
            head_sha: "a2ee2f56336295783719ba9e0be52c4f2f0670e2".to_owned(),
            review_decision: "REVIEW_REQUIRED".to_owned(),
            merge_state: "BLOCKED".to_owned(),
        }
    }

    fn sample_review_thread() -> ReviewThreadSummary {
        ReviewThreadSummary {
            thread_id: "THREAD_1".to_owned(),
            comment_id: Some(123456),
            author: Some("coderabbitai".to_owned()),
            url: Some(
                "https://github.com/flyingrobots/echo/pull/308#discussion_r123456".to_owned(),
            ),
            path: Some("xtask/src/main.rs".to_owned()),
            line: Some(42),
            is_outdated: false,
            preview: "Please tighten this branch.".to_owned(),
        }
    }

    fn snapshot_fixture(
        recorded_at: &str,
        filename_stamp: &str,
        overview: PrOverview,
        blockers: Vec<&str>,
        checks: Vec<PrCheckSummary>,
        unresolved_threads: Vec<ReviewThreadSummary>,
    ) -> PrSnapshotArtifact {
        let head_sha_short = overview.short_head_sha();
        PrSnapshotArtifact {
            recorded_at: recorded_at.to_owned(),
            filename_stamp: filename_stamp.to_owned(),
            sortie_intent: Some(DoghouseSortieIntent::ManualProbe),
            code_rabbit: None,
            pr: PrSnapshotOverview {
                number: overview.number,
                url: overview.url,
                title: overview.title,
                state: overview.state,
                head_sha: overview.head_sha,
                head_sha_short,
                review_decision: overview.review_decision,
                merge_state: overview.merge_state,
            },
            blockers: blockers.into_iter().map(str::to_owned).collect(),
            grouped_checks: group_pr_checks(&checks),
            checks,
            unresolved_threads,
        }
    }

    fn initial_comparison(snapshot: &PrSnapshotArtifact) -> DoghouseComparisonAssessment {
        assess_doghouse_comparison(snapshot, None, None)
    }

    fn unique_temp_path(prefix: &str) -> PathBuf {
        let unique = assert_ok(
            SystemTime::now().duration_since(UNIX_EPOCH),
            "time should advance for temp-path generation",
        )
        .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{unique}"))
    }

    // ── extract_link_targets ──────────────────────────────────────────

    #[test]
    fn extracts_plain_links() {
        let md = "[hello](guide/start-here.md)\n";
        let links = extract_link_targets(md);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].0, "guide/start-here.md");
    }

    #[test]
    fn handles_title_text() {
        let md = r#"[hello](guide/start-here.md "Start here")"#;
        let links = extract_link_targets(md);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].0, "guide/start-here.md");
    }

    #[test]
    fn handles_fragment_in_url() {
        let md = "[link](spec/SPEC-0004.md#section)\n";
        let links = extract_link_targets(md);
        assert_eq!(links[0].0, "spec/SPEC-0004.md#section");
    }

    #[test]
    fn handles_balanced_parens_in_anchor() {
        let md = "[link](spec/SPEC-0004.md#foo(bar))\n";
        let links = extract_link_targets(md);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].0, "spec/SPEC-0004.md#foo(bar)");
    }

    #[test]
    fn skips_images() {
        let md = "![alt](image.png)\n[real](doc.md)\n";
        let links = extract_link_targets(md);
        // pulldown-cmark reports images as Image, not Link
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].0, "doc.md");
    }

    #[test]
    fn reports_correct_line_numbers() {
        let md = "line one\n[a](a.md)\nline three\n[b](b.md)\n";
        let links = extract_link_targets(md);
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].1, 2); // 1-indexed
        assert_eq!(links[1].1, 4);
    }

    // ── build_candidates ──────────────────────────────────────────────

    #[test]
    fn relative_link_resolves_from_source_dir() {
        let source = Path::new("docs/guide/start-here.md");
        let docs_root = Path::new("docs");
        let candidates = build_candidates(source, "../spec.md", docs_root);
        // ../spec.md from docs/guide/ resolves to docs/guide/../spec.md
        assert!(candidates
            .iter()
            .any(|p| p == Path::new("docs/guide/../spec.md")));
    }

    #[test]
    fn root_relative_link_resolves_against_docs_root() {
        let source = Path::new("docs/index.md");
        let docs_root = Path::new("docs");
        let candidates = build_candidates(source, "/guide/start-here.md", docs_root);
        assert!(candidates
            .iter()
            .any(|p| p.ends_with("docs/guide/start-here.md")));
    }

    #[test]
    fn extensionless_link_tries_md_and_html() {
        let source = Path::new("docs/index.md");
        let docs_root = Path::new("docs");
        let candidates = build_candidates(source, "guide/start-here", docs_root);
        assert!(candidates
            .iter()
            .any(|p| p.ends_with("guide/start-here.md")));
        assert!(candidates
            .iter()
            .any(|p| p.ends_with("guide/start-here.html")));
    }

    #[test]
    fn public_asset_resolution() {
        let source = Path::new("docs/index.md");
        let docs_root = Path::new("docs");
        let candidates = build_candidates(source, "/collision-dpo-tour.html", docs_root);
        assert!(candidates
            .iter()
            .any(|p| p.ends_with("docs/public/collision-dpo-tour.html")));
    }

    // ── pr_status helpers ────────────────────────────────────────────

    #[test]
    fn pr_status_command_omits_selector_when_absent() {
        let script = Path::new("scripts/pr-status.sh");
        let command = build_pr_status_command(script, None);

        assert_eq!(command.get_program(), script);
        assert_eq!(command.get_args().count(), 0);
    }

    #[test]
    fn pr_status_command_passes_selector() {
        let script = Path::new("scripts/pr-status.sh");
        let command = build_pr_status_command(script, Some("306"));
        let args: Vec<_> = command.get_args().collect();

        assert_eq!(command.get_program(), script);
        assert_eq!(args.len(), 1);
        assert_eq!(args[0].to_str(), Some("306"));
    }

    #[cfg(unix)]
    #[test]
    fn pr_status_command_can_execute_explicit_script_path() {
        let unique = assert_ok(
            SystemTime::now().duration_since(UNIX_EPOCH),
            "time should be monotonic enough for test path generation",
        )
        .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("xtask-pr-status-{unique}"));
        let script_path = temp_dir.join("pr-status.sh");
        let output_path = temp_dir.join("output.txt");

        assert_ok(
            fs::create_dir_all(&temp_dir),
            "temp dir should be creatable",
        );
        let script_body = [
            "#!/usr/bin/env bash".to_owned(),
            "set -euo pipefail".to_owned(),
            "printf '%s' \"$".to_owned()
                + "{"
                + "1:-none"
                + "}\" > "
                + &output_path.display().to_string(),
            String::new(),
        ]
        .join("\n");
        assert_ok(
            fs::write(&script_path, script_body),
            "script should be writable",
        );

        let mut permissions =
            assert_ok(fs::metadata(&script_path), "script metadata should exist").permissions();
        permissions.set_mode(0o755);
        assert_ok(
            fs::set_permissions(&script_path, permissions),
            "script should be executable",
        );

        let status = assert_ok(
            build_pr_status_command(&script_path, Some("302")).status(),
            "script should run",
        );
        assert!(status.success());
        assert_eq!(
            assert_ok(
                fs::read_to_string(&output_path),
                "output should be readable",
            ),
            "302"
        );

        fs::remove_file(&output_path).ok();
        fs::remove_file(&script_path).ok();
        fs::remove_dir(&temp_dir).ok();
    }

    // ── pr_threads helpers ───────────────────────────────────────────

    #[test]
    fn parses_pr_owner_and_repo_from_url() {
        let (owner, repo) = assert_ok(
            parse_pr_owner_name("https://github.com/flyingrobots/echo/pull/308"),
            "owner/repo should parse",
        );
        assert_eq!(owner, "flyingrobots");
        assert_eq!(repo, "echo");
    }

    #[test]
    fn preview_comment_body_uses_first_non_empty_line() {
        let preview = preview_comment_body("\n\n  first useful line  \nsecond line");
        assert_eq!(preview, "first useful line");
    }

    #[test]
    fn preview_comment_body_truncates_long_lines() {
        let preview = preview_comment_body(&"abcdefghijklmnopqrstuvwxyz".repeat(5));
        assert!(preview.ends_with('…'));
        assert!(preview.chars().count() <= 100);
    }

    #[test]
    fn load_reply_body_requires_exactly_one_source() {
        assert!(load_reply_body(None, None).is_err());
        assert!(load_reply_body(Some("body"), Some(Path::new("reply.md"))).is_err());
        assert_eq!(
            assert_ok(load_reply_body(Some("body"), None), "body text should load"),
            "body"
        );
    }

    #[test]
    fn review_reply_route_uses_pr_owner_and_repo() {
        let overview = PrOverview {
            owner: "upstream".to_owned(),
            repo: "fork-target".to_owned(),
            number: 308,
            title: "Review helper".to_owned(),
            url: "https://github.com/upstream/fork-target/pull/308".to_owned(),
            state: "OPEN".to_owned(),
            head_sha: "deadbeefcafebabefeedface1234567890abcdef".to_owned(),
            review_decision: "NONE".to_owned(),
            merge_state: "UNKNOWN".to_owned(),
        };

        assert_eq!(
            build_review_reply_route(&overview),
            "repos/upstream/fork-target/pulls/308/comments"
        );
    }

    #[test]
    fn snapshot_artifact_collects_expected_blockers() {
        let overview = sample_pr_overview();
        let checks = vec![
            PrCheckSummary {
                name: "Clippy".to_owned(),
                bucket: "fail".to_owned(),
                state: "FAILURE".to_owned(),
            },
            PrCheckSummary {
                name: "Tests".to_owned(),
                bucket: "pending".to_owned(),
                state: "PENDING".to_owned(),
            },
            PrCheckSummary {
                name: "fmt".to_owned(),
                bucket: "pass".to_owned(),
                state: "SUCCESS".to_owned(),
            },
        ];
        let threads = vec![sample_review_thread()];

        let snapshot = assert_ok(
            build_pr_snapshot_artifact(
                &overview,
                &checks,
                &threads,
                DoghouseSortieIntent::ManualProbe,
                None,
            ),
            "snapshot should build",
        );

        assert_eq!(snapshot.pr.head_sha_short, "a2ee2f563362");
        assert_eq!(
            snapshot.sortie_intent,
            Some(DoghouseSortieIntent::ManualProbe)
        );
        assert!(snapshot
            .blockers
            .contains(&"unresolved review threads: 1".to_owned()));
        assert!(snapshot
            .blockers
            .contains(&"failing checks: Clippy".to_owned()));
        assert!(snapshot
            .blockers
            .contains(&"pending checks: Tests".to_owned()));
        assert!(snapshot
            .blockers
            .contains(&"review decision: REVIEW_REQUIRED".to_owned()));
        assert!(snapshot
            .blockers
            .contains(&"merge state: BLOCKED".to_owned()));
    }

    #[test]
    fn snapshot_markdown_includes_core_sections() {
        let overview = sample_pr_overview();
        let checks = vec![PrCheckSummary {
            name: "fmt".to_owned(),
            bucket: "pass".to_owned(),
            state: "SUCCESS".to_owned(),
        }];
        let threads = vec![sample_review_thread()];
        let snapshot = assert_ok(
            build_pr_snapshot_artifact(
                &overview,
                &checks,
                &threads,
                DoghouseSortieIntent::ManualProbe,
                None,
            ),
            "snapshot should build",
        );

        let markdown = render_pr_snapshot_markdown(&snapshot);

        assert!(markdown.contains("# Doghouse Flight Recorder: PR #308"));
        assert!(markdown.contains("PR state: `OPEN`"));
        assert!(markdown.contains("## Current Blockers"));
        assert!(markdown.contains("## Checks"));
        assert!(markdown.contains("## Unresolved Threads"));
        assert!(markdown.contains("Please tighten this branch."));
    }

    #[test]
    fn snapshot_blockers_ignore_review_and_merge_state_for_merged_prs() {
        let mut overview = sample_pr_overview();
        overview.state = "MERGED".to_owned();
        overview.review_decision = "NONE".to_owned();
        overview.merge_state = "UNKNOWN".to_owned();
        let checks = vec![PrCheckSummary {
            name: "fmt".to_owned(),
            bucket: "pass".to_owned(),
            state: "SUCCESS".to_owned(),
        }];

        let snapshot = assert_ok(
            build_pr_snapshot_artifact(
                &overview,
                &checks,
                &[],
                DoghouseSortieIntent::ManualProbe,
                None,
            ),
            "snapshot should build",
        );

        assert!(snapshot.blockers.is_empty());
    }

    #[test]
    fn snapshot_writer_creates_timestamped_and_latest_outputs() {
        let overview = sample_pr_overview();
        let checks = vec![PrCheckSummary {
            name: "fmt".to_owned(),
            bucket: "pass".to_owned(),
            state: "SUCCESS".to_owned(),
        }];
        let threads = Vec::new();
        let snapshot = assert_ok(
            build_pr_snapshot_artifact(
                &overview,
                &checks,
                &threads,
                DoghouseSortieIntent::ManualProbe,
                None,
            ),
            "snapshot should build",
        );
        let temp_dir = unique_temp_path("xtask-pr-snapshot");

        let paths = assert_ok(
            write_pr_snapshot_artifact(&snapshot, &temp_dir),
            "snapshot artifacts should write",
        );

        assert!(paths.snapshot_json.is_file());
        assert!(paths.snapshot_markdown.is_file());
        assert!(paths.latest_json.is_file());
        assert!(paths.latest_markdown.is_file());
        assert!(paths
            .snapshot_json
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.ends_with("-a2ee2f563362.json")));
        assert_eq!(
            assert_ok(
                fs::read_to_string(&paths.latest_json),
                "latest json should be readable",
            ),
            assert_ok(
                fs::read_to_string(&paths.snapshot_json),
                "snapshot json should be readable",
            )
        );

        fs::remove_file(&paths.snapshot_json).ok();
        fs::remove_file(&paths.snapshot_markdown).ok();
        fs::remove_file(&paths.latest_json).ok();
        fs::remove_file(&paths.latest_markdown).ok();
        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn snapshot_filename_stamp_distinguishes_same_second_captures() {
        let first = assert_ok(
            OffsetDateTime::parse("2026-03-26T04:38:02.000000001Z", &Rfc3339),
            "first timestamp should parse",
        );
        let second = assert_ok(
            OffsetDateTime::parse("2026-03-26T04:38:02.000000002Z", &Rfc3339),
            "second timestamp should parse",
        );

        assert_ne!(
            format_snapshot_filename_stamp(first),
            format_snapshot_filename_stamp(second)
        );
    }

    #[test]
    fn snapshot_delta_reports_meaningful_transitions() {
        let mut previous_overview = sample_pr_overview();
        previous_overview.head_sha = "11111111111195783719ba9e0be52c4f2f0670e2".to_owned();
        let previous = snapshot_fixture(
            "2026-03-25T08:00:00Z",
            "20260325T080000Z",
            previous_overview,
            vec![
                "unresolved review threads: 2",
                "pending checks: Tests",
                "review decision: REVIEW_REQUIRED",
            ],
            vec![
                PrCheckSummary {
                    name: "Tests".to_owned(),
                    bucket: "pending".to_owned(),
                    state: "PENDING".to_owned(),
                },
                PrCheckSummary {
                    name: "Clippy".to_owned(),
                    bucket: "fail".to_owned(),
                    state: "FAILURE".to_owned(),
                },
            ],
            vec![
                sample_review_thread(),
                ReviewThreadSummary {
                    thread_id: "THREAD_OLD".to_owned(),
                    comment_id: Some(333),
                    author: Some("codex".to_owned()),
                    url: None,
                    path: Some("docs/workflows.md".to_owned()),
                    line: Some(12),
                    is_outdated: false,
                    preview: "old thread".to_owned(),
                },
            ],
        );

        let mut current_overview = sample_pr_overview();
        current_overview.head_sha = "22222222222295783719ba9e0be52c4f2f0670e2".to_owned();
        current_overview.review_decision = "APPROVED".to_owned();
        current_overview.merge_state = "CLEAN".to_owned();
        let current = snapshot_fixture(
            "2026-03-25T08:20:00Z",
            "20260325T082000Z",
            current_overview,
            vec!["failing checks: Docs"],
            vec![
                PrCheckSummary {
                    name: "Tests".to_owned(),
                    bucket: "pass".to_owned(),
                    state: "SUCCESS".to_owned(),
                },
                PrCheckSummary {
                    name: "Clippy".to_owned(),
                    bucket: "pending".to_owned(),
                    state: "PENDING".to_owned(),
                },
                PrCheckSummary {
                    name: "Docs".to_owned(),
                    bucket: "fail".to_owned(),
                    state: "FAILURE".to_owned(),
                },
            ],
            vec![
                sample_review_thread(),
                ReviewThreadSummary {
                    thread_id: "THREAD_NEW".to_owned(),
                    comment_id: Some(444),
                    author: Some("coderabbitai".to_owned()),
                    url: None,
                    path: Some("xtask/src/main.rs".to_owned()),
                    line: Some(99),
                    is_outdated: false,
                    preview: "new thread".to_owned(),
                },
            ],
        );

        let delta = build_pr_snapshot_delta(&previous, &current);

        assert_eq!(delta.previous.head_sha_short, "111111111111");
        assert_eq!(delta.current.head_sha_short, "222222222222");
        assert_eq!(
            delta.blockers_added,
            vec!["failing checks: Docs".to_owned()]
        );
        assert!(delta
            .blockers_removed
            .contains(&"pending checks: Tests".to_owned()));
        assert!(delta
            .blockers_removed
            .contains(&"review decision: REVIEW_REQUIRED".to_owned()));
        assert_eq!(
            delta
                .threads_opened
                .iter()
                .map(|thread| thread.thread_id.as_str())
                .collect::<Vec<_>>(),
            vec!["THREAD_NEW"]
        );
        assert_eq!(
            delta
                .threads_resolved
                .iter()
                .map(|thread| thread.thread_id.as_str())
                .collect::<Vec<_>>(),
            vec!["THREAD_OLD"]
        );
        assert_eq!(
            delta
                .threads_persisting
                .iter()
                .map(|thread| thread.thread_id.as_str())
                .collect::<Vec<_>>(),
            vec!["THREAD_1"]
        );
        assert_eq!(delta.improved_checks.len(), 2);
        assert_eq!(delta.regressed_checks.len(), 0);
        assert_eq!(delta.shifted_checks.len(), 0);
        assert_eq!(
            delta
                .added_checks
                .iter()
                .map(|check| check.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Docs"]
        );
    }

    #[test]
    fn snapshot_delta_markdown_includes_transition_sections() {
        let previous = snapshot_fixture(
            "2026-03-25T08:00:00Z",
            "20260325T080000Z",
            sample_pr_overview(),
            vec!["pending checks: Tests"],
            vec![PrCheckSummary {
                name: "Tests".to_owned(),
                bucket: "pending".to_owned(),
                state: "PENDING".to_owned(),
            }],
            vec![sample_review_thread()],
        );
        let mut current_overview = sample_pr_overview();
        current_overview.head_sha = "bbbbbbbbbbbb95783719ba9e0be52c4f2f0670e2".to_owned();
        let current = snapshot_fixture(
            "2026-03-25T08:10:00Z",
            "20260325T081000Z",
            current_overview,
            vec![],
            vec![PrCheckSummary {
                name: "Tests".to_owned(),
                bucket: "pass".to_owned(),
                state: "SUCCESS".to_owned(),
            }],
            vec![],
        );

        let markdown =
            render_pr_snapshot_delta_markdown(&build_pr_snapshot_delta(&previous, &current));

        assert!(markdown.contains("# Doghouse Delta: PR #308"));
        assert!(markdown.contains("## Head Transition"));
        assert!(markdown.contains("## Blocker Transition"));
        assert!(markdown.contains("## Thread Transition"));
        assert!(markdown.contains("## Check Transition"));
        assert!(markdown.contains("Removed blockers"));
        assert!(markdown.contains("Improved checks"));
    }

    #[test]
    fn doghouse_baseline_prefers_previous_different_head() {
        let latest_same_head = snapshot_fixture(
            "2026-03-25T08:10:00Z",
            "20260325T081000Z",
            sample_pr_overview(),
            vec![],
            vec![],
            vec![],
        );
        let mut older_overview = sample_pr_overview();
        older_overview.head_sha = "bbbbbbbbbbbb95783719ba9e0be52c4f2f0670e2".to_owned();
        let older_different_head = snapshot_fixture(
            "2026-03-25T08:00:00Z",
            "20260325T080000Z",
            older_overview,
            vec!["review decision: REVIEW_REQUIRED"],
            vec![],
            vec![],
        );
        let snapshots = vec![older_different_head.clone(), latest_same_head];
        let current = snapshot_fixture(
            "2026-03-25T08:20:00Z",
            "20260325T082000Z",
            sample_pr_overview(),
            vec![],
            vec![],
            vec![],
        );

        let selection = select_doghouse_baseline(&snapshots, &current)
            .unwrap_or_else(|| unreachable!("baseline should be selected"));

        assert_eq!(
            selection.strategy,
            DoghouseBaselineStrategy::PreviousDifferentHead
        );
        assert_eq!(
            selection.snapshot.pr.head_sha,
            older_different_head.pr.head_sha
        );
        assert_eq!(selection.newer_snapshot_count, 1);
        assert_eq!(selection.newer_semantic_change_count, 0);
    }

    #[test]
    fn doghouse_baseline_falls_back_to_previous_semantic_change() {
        let mut changed_overview = sample_pr_overview();
        changed_overview.merge_state = "CLEAN".to_owned();
        changed_overview.review_decision = "APPROVED".to_owned();
        let older_changed = snapshot_fixture(
            "2026-03-25T08:05:00Z",
            "20260325T080500Z",
            changed_overview,
            vec![],
            vec![],
            vec![],
        );
        let latest_changed = snapshot_fixture(
            "2026-03-25T08:10:00Z",
            "20260325T081000Z",
            sample_pr_overview(),
            vec!["review decision: REVIEW_REQUIRED"],
            vec![],
            vec![],
        );
        let current = snapshot_fixture(
            "2026-03-25T08:20:00Z",
            "20260325T082000Z",
            sample_pr_overview(),
            vec![],
            vec![],
            vec![],
        );

        let selection =
            select_doghouse_baseline(&[older_changed, latest_changed.clone()], &current)
                .unwrap_or_else(|| unreachable!("baseline should be selected"));

        assert_eq!(
            selection.strategy,
            DoghouseBaselineStrategy::PreviousSemanticChange
        );
        assert_eq!(selection.snapshot.recorded_at, latest_changed.recorded_at);
        assert_eq!(selection.newer_snapshot_count, 0);
        assert_eq!(selection.newer_semantic_change_count, 0);
    }

    #[test]
    fn doghouse_comparison_marks_different_head_baselines_strong() {
        let mut previous_overview = sample_pr_overview();
        previous_overview.head_sha = "bbbbbbbbbbbb95783719ba9e0be52c4f2f0670e2".to_owned();
        let previous = snapshot_fixture(
            "2026-03-25T08:00:00Z",
            "20260325T080000Z",
            previous_overview,
            vec!["review decision: REVIEW_REQUIRED"],
            vec![],
            vec![],
        );
        let current = snapshot_fixture(
            "2026-03-25T08:20:00Z",
            "20260325T082000Z",
            sample_pr_overview(),
            vec![],
            vec![],
            vec![],
        );
        let selection = select_doghouse_baseline(std::slice::from_ref(&previous), &current)
            .unwrap_or_else(|| unreachable!("baseline should be selected"));
        let delta = build_pr_snapshot_delta(&selection.snapshot, &current);

        let assessment = assess_doghouse_comparison(&current, Some(&selection), Some(&delta));

        assert_eq!(assessment.trust, DoghouseComparisonTrust::Strong);
        assert_eq!(assessment.quality, DoghouseComparisonQuality::GoodEnough);
        assert_eq!(assessment.same_head, Some(false));
        assert_eq!(assessment.semantically_changed, Some(true));
        assert!(!assessment.stale);
        assert!(!assessment.noisy);
    }

    #[test]
    fn doghouse_comparison_marks_old_same_head_baselines_stale() {
        let previous = snapshot_fixture(
            "2026-03-24T20:00:00Z",
            "20260324T200000Z",
            sample_pr_overview(),
            vec!["review decision: REVIEW_REQUIRED"],
            vec![],
            vec![],
        );
        let current = snapshot_fixture(
            "2026-03-25T12:30:00Z",
            "20260325T123000Z",
            sample_pr_overview(),
            vec![],
            vec![],
            vec![],
        );
        let selection = select_doghouse_baseline(std::slice::from_ref(&previous), &current)
            .unwrap_or_else(|| unreachable!("baseline should be selected"));
        let delta = build_pr_snapshot_delta(&selection.snapshot, &current);

        let assessment = assess_doghouse_comparison(&current, Some(&selection), Some(&delta));

        assert_eq!(assessment.trust, DoghouseComparisonTrust::Usable);
        assert_eq!(assessment.quality, DoghouseComparisonQuality::Stale);
        assert_eq!(assessment.same_head, Some(true));
        assert!(assessment.stale);
        assert!(!assessment.noisy);
        assert!(
            assessment.baseline_age_seconds.is_some_and(
                |seconds| seconds > doghouse_stale_threshold_seconds(&selection.strategy)
            )
        );
    }

    #[test]
    fn doghouse_comparison_marks_skipped_semantic_changes_noisy() {
        let mut different_head_overview = sample_pr_overview();
        different_head_overview.head_sha = "bbbbbbbbbbbb95783719ba9e0be52c4f2f0670e2".to_owned();
        let previous_different_head = snapshot_fixture(
            "2026-03-25T08:00:00Z",
            "20260325T080000Z",
            different_head_overview,
            vec!["review decision: REVIEW_REQUIRED"],
            vec![],
            vec![],
        );
        let mut intermediate_overview = sample_pr_overview();
        intermediate_overview.merge_state = "CLEAN".to_owned();
        let intermediate_same_head_change = snapshot_fixture(
            "2026-03-25T08:10:00Z",
            "20260325T081000Z",
            intermediate_overview,
            vec!["merge state: CLEAN"],
            vec![],
            vec![],
        );
        let current = snapshot_fixture(
            "2026-03-25T08:20:00Z",
            "20260325T082000Z",
            sample_pr_overview(),
            vec![],
            vec![],
            vec![],
        );
        let snapshots = vec![previous_different_head, intermediate_same_head_change];
        let selection = select_doghouse_baseline(&snapshots, &current)
            .unwrap_or_else(|| unreachable!("baseline should be selected"));
        let delta = build_pr_snapshot_delta(&selection.snapshot, &current);

        let assessment = assess_doghouse_comparison(&current, Some(&selection), Some(&delta));

        assert_eq!(
            selection.strategy,
            DoghouseBaselineStrategy::PreviousDifferentHead
        );
        assert_eq!(selection.newer_semantic_change_count, 1);
        assert_eq!(assessment.quality, DoghouseComparisonQuality::Noisy);
        assert!(assessment.noisy);
    }

    #[test]
    fn doghouse_comparison_marks_initial_capture_as_none() {
        let current = snapshot_fixture(
            "2026-03-25T08:20:00Z",
            "20260325T082000Z",
            sample_pr_overview(),
            vec![],
            vec![],
            vec![],
        );

        let assessment = assess_doghouse_comparison(&current, None, None);

        assert_eq!(assessment.trust, DoghouseComparisonTrust::None);
        assert_eq!(
            assessment.quality,
            DoghouseComparisonQuality::InitialCapture
        );
        assert_eq!(assessment.same_head, None);
        assert_eq!(assessment.semantically_changed, None);
        assert!(!assessment.stale);
        assert!(!assessment.noisy);
    }

    #[test]
    fn doghouse_intent_event_tracks_current_and_baseline_intent() {
        let mut baseline_snapshot = snapshot_fixture(
            "2026-03-25T08:00:00Z",
            "20260325T080000Z",
            sample_pr_overview(),
            vec![],
            vec![],
            vec![],
        );
        baseline_snapshot.sortie_intent = Some(DoghouseSortieIntent::FixBatch);
        let selection = DoghouseBaselineSelection {
            strategy: DoghouseBaselineStrategy::ImmediatePrevious,
            snapshot: baseline_snapshot,
            newer_snapshot_count: 0,
            newer_semantic_change_count: 0,
        };

        let mut current = snapshot_fixture(
            "2026-03-25T08:20:00Z",
            "20260325T082000Z",
            sample_pr_overview(),
            vec![],
            vec![],
            vec![],
        );
        current.sortie_intent = Some(DoghouseSortieIntent::MergeCheck);

        let event = DoghouseIntentEvent::from_snapshots(&current, Some(&selection));

        assert_eq!(event.current_intent, Some(DoghouseSortieIntent::MergeCheck));
        assert_eq!(event.baseline_intent, Some(DoghouseSortieIntent::FixBatch));
        assert_eq!(event.changed_from_baseline, Some(true));
    }

    #[test]
    fn detects_coderabbit_cooldown_from_summary_comment() {
        let updated_at = OffsetDateTime::now_utc();
        let comment = CodeRabbitSummaryComment {
            database_id: Some(4123437114),
            url: "https://github.com/flyingrobots/echo/pull/308#issuecomment-1".to_owned(),
            body: [
                "<!-- This is an auto-generated comment: summarize by coderabbit.ai -->",
                "<!-- This is an auto-generated comment: rate limited by coderabbit.ai -->",
                "> [!WARNING]",
                "> ## Rate limit exceeded",
                ">",
                "> Please wait **5 minutes and 59 seconds** before requesting another review.",
            ]
            .join("\n"),
            updated_at: assert_ok(updated_at.format(&Rfc3339), "test timestamp should format"),
            author: Some(CodeRabbitCommentAuthor {
                login: "coderabbitai".to_owned(),
            }),
        };

        let cooldown = analyze_code_rabbit_summary_comment(comment)
            .unwrap_or_else(|| unreachable!("CodeRabbit state should be detected"));

        assert_eq!(
            cooldown.summary_state,
            DoghouseCodeRabbitSummaryState::RateLimited
        );
        assert_eq!(cooldown.summary_comment_database_id, Some(4123437114));
        assert!(cooldown.callout_present);
        assert_eq!(cooldown.callout_kind.as_deref(), Some("WARNING"));
        assert_eq!(
            cooldown.callout_title.as_deref(),
            Some("Rate limit exceeded")
        );
        assert!(cooldown.cooldown_active);
        assert!(cooldown
            .cooldown_remaining_seconds
            .is_some_and(|seconds| seconds > 0));
        assert!(cooldown
            .cooldown_remaining_seconds
            .is_some_and(|seconds| seconds <= 359));
        assert_eq!(
            cooldown.cooldown_expires_at,
            Some(assert_ok(
                (updated_at + time::Duration::seconds(359)).format(&Rfc3339),
                "cooldown expiry should format",
            ))
        );
        assert!(!cooldown.request_review_actionable);
    }

    #[test]
    fn selects_latest_coderabbit_summary_from_paginated_issue_comments() {
        let comments = vec![
            CodeRabbitSummaryComment::from_issue_comment(GhIssueComment {
                id: 1,
                html_url: "https://github.com/flyingrobots/echo/pull/309#issuecomment-1".to_owned(),
                body: "ordinary human comment".to_owned(),
                updated_at: "2026-03-26T03:30:00Z".to_owned(),
                user: Some(GhIssueCommentUser {
                    login: "alice".to_owned(),
                }),
            }),
            CodeRabbitSummaryComment::from_issue_comment(GhIssueComment {
                id: 2,
                html_url: "https://github.com/flyingrobots/echo/pull/309#issuecomment-2".to_owned(),
                body: [
                    "<!-- This is an auto-generated comment: summarize by coderabbit.ai -->",
                    "older summary",
                ]
                .join("\n"),
                updated_at: "2026-03-26T03:31:00Z".to_owned(),
                user: Some(GhIssueCommentUser {
                    login: "coderabbitai[bot]".to_owned(),
                }),
            }),
            CodeRabbitSummaryComment::from_issue_comment(GhIssueComment {
                id: 3,
                html_url: "https://github.com/flyingrobots/echo/pull/309#issuecomment-3".to_owned(),
                body: [
                    "<!-- This is an auto-generated comment: summarize by coderabbit.ai -->",
                    "newer summary",
                ]
                .join("\n"),
                updated_at: "2026-03-26T03:32:00Z".to_owned(),
                user: Some(GhIssueCommentUser {
                    login: "coderabbitai[bot]".to_owned(),
                }),
            }),
        ];

        let comment = select_latest_code_rabbit_summary_comment(comments)
            .unwrap_or_else(|| unreachable!("latest CodeRabbit summary should be selected"));

        assert_eq!(comment.database_id, Some(3));
        assert!(comment.body.contains("newer summary"));
    }

    #[test]
    fn detects_coderabbit_rearm_gate_from_active_changes_callout() {
        let comment = CodeRabbitSummaryComment {
            database_id: Some(4123999999),
            url: "https://github.com/flyingrobots/echo/pull/308#issuecomment-2".to_owned(),
            body: [
                "<!-- This is an auto-generated comment: summarize by coderabbit.ai -->",
                "> [!IMPORTANT]",
                "> ## Active changes detected",
                ">",
                "> You are still making active changes on this PR. Resume review when you are ready.",
                "",
                "- [ ] <!-- {\"checkboxId\": \"abc-123\"} --> Resume CodeRabbit review",
            ]
            .join("\n"),
            updated_at: "2026-03-25T07:01:37Z".to_owned(),
            author: Some(CodeRabbitCommentAuthor {
                login: "coderabbitai".to_owned(),
            }),
        };

        let state = analyze_code_rabbit_summary_comment(comment)
            .unwrap_or_else(|| unreachable!("CodeRabbit state should be detected"));

        assert_eq!(
            state.summary_state,
            DoghouseCodeRabbitSummaryState::RearmRequired
        );
        assert!(state.callout_present);
        assert!(state.rearm_actionable);
        assert_eq!(state.checkboxes.len(), 1);
        assert_eq!(state.checkboxes[0].label, "Resume CodeRabbit review");
        assert_eq!(state.checkboxes[0].checkbox_id.as_deref(), Some("abc-123"));
        assert!(!state.request_review_actionable);
    }

    #[test]
    fn coderabbit_checkbox_parser_ignores_markdown_link_bullets() {
        assert_eq!(
            parse_code_rabbit_checkbox_line(
                "- [X](https://twitter.com/intent/tweet?text=hello) share"
            ),
            None
        );
    }

    #[test]
    fn coderabbit_rearm_body_toggles_only_task_checkboxes() {
        let body = [
            "> [!IMPORTANT]",
            "> ## Active changes detected",
            ">",
            "> Resume review when you are ready.",
            "",
            "- [ ] <!-- {\"checkboxId\": \"abc-123\"} --> Resume CodeRabbit review",
            "- [ ] <!-- {\"checkboxId\": \"def-456\"} --> Create PR with unit tests",
            "- [X](https://example.com/share) share",
        ]
        .join("\n");

        let (updated, toggled) = build_code_rabbit_rearm_body(&body)
            .unwrap_or_else(|| unreachable!("rearm body should be rewritable"));

        assert_eq!(toggled, 1);
        assert!(updated
            .contains("- [x] <!-- {\"checkboxId\": \"abc-123\"} --> Resume CodeRabbit review"));
        assert!(updated
            .contains("- [ ] <!-- {\"checkboxId\": \"def-456\"} --> Create PR with unit tests"));
        assert!(updated.contains("- [X](https://example.com/share) share"));
    }

    #[test]
    fn detects_coderabbit_reviews_paused_hibernation_shape() {
        let comment = CodeRabbitSummaryComment {
            database_id: Some(4124000000),
            url: "https://github.com/flyingrobots/echo/pull/308#issuecomment-3".to_owned(),
            body: [
                "<!-- This is an auto-generated comment: summarize by coderabbit.ai -->",
                "Reviews paused",
                "",
                "It looks like this branch is under active development.",
                "To avoid overwhelming you with review comments due to an influx of new commits, CodeRabbit has automatically paused this review.",
                "You can configure this behavior by changing the reviews.auto_review.auto_pause_after_reviewed_commits setting.",
                "",
                "Use the following commands to manage reviews:",
                "",
                "@coderabbitai resume to resume automatic reviews.",
                "@coderabbitai review to trigger a single review.",
                "Use the checkboxes below for quick actions:",
                "",
                "- [ ] <!-- {\"checkboxId\": \"resume-123\"} --> ▶️ Resume reviews",
                "- [x] <!-- {\"checkboxId\": \"running-456\"} --> 🔄 Running review...",
            ]
            .join("\n"),
            updated_at: "2026-03-25T08:01:37Z".to_owned(),
            author: Some(CodeRabbitCommentAuthor {
                login: "coderabbitai".to_owned(),
            }),
        };

        let state = analyze_code_rabbit_summary_comment(comment)
            .unwrap_or_else(|| unreachable!("CodeRabbit state should be detected"));

        assert_eq!(
            state.summary_state,
            DoghouseCodeRabbitSummaryState::RearmRequired
        );
        assert!(!state.callout_present);
        assert!(state.rearm_actionable);
        assert_eq!(state.checkboxes.len(), 2);
        assert_eq!(state.checkboxes[0].label, "▶️ Resume reviews");
        assert_eq!(state.checkboxes[1].label, "🔄 Running review...");
        assert!(!state.request_review_actionable);
    }

    #[test]
    fn selects_resume_nudge_for_active_changes_summary() {
        let comment = CodeRabbitSummaryComment {
            database_id: Some(4124000001),
            url: "https://github.com/flyingrobots/echo/pull/308#issuecomment-4".to_owned(),
            body: [
                "<!-- This is an auto-generated comment: summarize by coderabbit.ai -->",
                "Reviews paused",
                "",
                "It looks like this branch is under active development.",
                "",
                "@coderabbitai resume to resume automatic reviews.",
                "@coderabbitai review to trigger a single review.",
                "",
                "- [ ] <!-- {\"checkboxId\": \"resume-123\"} --> ▶️ Resume reviews",
            ]
            .join("\n"),
            updated_at: "2026-03-25T08:01:37Z".to_owned(),
            author: Some(CodeRabbitCommentAuthor {
                login: "coderabbitai".to_owned(),
            }),
        };
        let state = analyze_code_rabbit_summary_comment(comment.clone())
            .unwrap_or_else(|| unreachable!("CodeRabbit state should be detected"));

        assert_eq!(
            select_code_rabbit_nudge_body(Some(&comment), Some(&state)),
            "@coderabbitai resume"
        );
    }

    #[test]
    fn selects_review_nudge_for_ready_summary() {
        let comment = CodeRabbitSummaryComment {
            database_id: Some(4124000002),
            url: "https://github.com/flyingrobots/echo/pull/308#issuecomment-5".to_owned(),
            body: [
                "<!-- This is an auto-generated comment: summarize by coderabbit.ai -->",
                "CodeRabbit ready for another pass.",
            ]
            .join("\n"),
            updated_at: "2026-03-25T08:11:37Z".to_owned(),
            author: Some(CodeRabbitCommentAuthor {
                login: "coderabbitai".to_owned(),
            }),
        };
        let state = analyze_code_rabbit_summary_comment(comment.clone())
            .unwrap_or_else(|| unreachable!("CodeRabbit state should be detected"));

        assert_eq!(
            select_code_rabbit_nudge_body(Some(&comment), Some(&state)),
            "@coderabbitai review"
        );
    }

    #[test]
    fn coderabbit_event_reflects_pending_check_and_cooldown() {
        let mut snapshot = snapshot_fixture(
            "2026-03-25T08:20:00Z",
            "20260325T082000Z",
            sample_pr_overview(),
            vec!["review decision: REVIEW_REQUIRED"],
            vec![PrCheckSummary {
                name: "CodeRabbit".to_owned(),
                bucket: "pending".to_owned(),
                state: "PENDING".to_owned(),
            }],
            vec![],
        );
        snapshot.code_rabbit = Some(DoghouseCodeRabbitState {
            summary_comment_url: "https://github.com/flyingrobots/echo/pull/308#issuecomment-1"
                .to_owned(),
            summary_comment_database_id: Some(4123437114),
            summary_comment_updated_at: "2026-03-25T07:01:37Z".to_owned(),
            summary_state: DoghouseCodeRabbitSummaryState::RateLimited,
            callout_present: true,
            callout_kind: Some("WARNING".to_owned()),
            callout_title: Some("Rate limit exceeded".to_owned()),
            cooldown_active: true,
            cooldown_expires_at: Some("2026-03-25T07:07:36Z".to_owned()),
            cooldown_remaining_seconds: Some(359),
            rearm_actionable: false,
            checkboxes: Vec::new(),
            request_review_actionable: false,
        });

        let event = DoghouseCodeRabbitEvent::from_snapshot(&snapshot);

        assert_eq!(
            event.summary_state,
            DoghouseCodeRabbitSummaryState::RateLimited
        );
        assert!(event.currently_reviewing);
        assert_eq!(event.check_bucket.as_deref(), Some("pending"));
        assert_eq!(event.check_state.as_deref(), Some("PENDING"));
        assert!(event.cooldown_active);
        assert!(event.callout_present);
        assert_eq!(event.callout_kind.as_deref(), Some("WARNING"));
        assert!(!event.request_review_actionable);
        assert_eq!(event.cooldown_remaining_seconds, Some(359));
    }

    #[test]
    fn doghouse_next_action_prioritizes_unresolved_threads() {
        let snapshot = snapshot_fixture(
            "2026-03-25T08:20:00Z",
            "20260325T082000Z",
            sample_pr_overview(),
            vec!["unresolved review threads: 1"],
            vec![PrCheckSummary {
                name: "Tests".to_owned(),
                bucket: "pending".to_owned(),
                state: "PENDING".to_owned(),
            }],
            vec![sample_review_thread()],
        );

        let comparison = initial_comparison(&snapshot);
        let action = determine_doghouse_next_action(&snapshot, &comparison, None);

        assert_eq!(action.action, "fix_unresolved_threads");
    }

    #[test]
    fn doghouse_next_action_merge_check_reports_pending_approval() {
        let mut overview = sample_pr_overview();
        overview.merge_state = "BLOCKED".to_owned();
        let mut snapshot = snapshot_fixture(
            "2026-03-25T08:20:00Z",
            "20260325T082000Z",
            overview,
            vec!["review decision: REVIEW_REQUIRED", "merge state: BLOCKED"],
            vec![PrCheckSummary {
                name: "Tests".to_owned(),
                bucket: "pass".to_owned(),
                state: "SUCCESS".to_owned(),
            }],
            vec![],
        );
        snapshot.sortie_intent = Some(DoghouseSortieIntent::MergeCheck);

        let comparison = initial_comparison(&snapshot);
        let action = determine_doghouse_next_action(&snapshot, &comparison, None);

        assert_eq!(action.sortie_intent, Some(DoghouseSortieIntent::MergeCheck));
        assert_eq!(action.action, "merge_ready_pending_approval");
        assert!(action
            .reasons
            .iter()
            .any(|reason| reason.contains("formal approval blocker")));
    }

    #[test]
    fn doghouse_next_action_post_push_waits_for_review_feedback() {
        let mut overview = sample_pr_overview();
        overview.merge_state = "BLOCKED".to_owned();
        let mut snapshot = snapshot_fixture(
            "2026-03-25T08:20:00Z",
            "20260325T082000Z",
            overview,
            vec!["review decision: REVIEW_REQUIRED", "merge state: BLOCKED"],
            vec![PrCheckSummary {
                name: "Tests".to_owned(),
                bucket: "pass".to_owned(),
                state: "SUCCESS".to_owned(),
            }],
            vec![],
        );
        snapshot.sortie_intent = Some(DoghouseSortieIntent::PostPush);

        let comparison = initial_comparison(&snapshot);
        let action = determine_doghouse_next_action(&snapshot, &comparison, None);

        assert_eq!(action.sortie_intent, Some(DoghouseSortieIntent::PostPush));
        assert_eq!(action.action, "wait_for_review_feedback");
        assert!(action
            .reasons
            .iter()
            .any(|reason| reason.contains("observation mode")));
    }

    #[test]
    fn doghouse_next_action_requests_review_when_only_formal_review_state_remains() {
        let mut overview = sample_pr_overview();
        overview.merge_state = "BLOCKED".to_owned();
        let snapshot = snapshot_fixture(
            "2026-03-25T08:20:00Z",
            "20260325T082000Z",
            overview,
            vec!["review decision: REVIEW_REQUIRED", "merge state: BLOCKED"],
            vec![PrCheckSummary {
                name: "Tests".to_owned(),
                bucket: "pass".to_owned(),
                state: "SUCCESS".to_owned(),
            }],
            vec![],
        );

        let comparison = initial_comparison(&snapshot);
        let action = determine_doghouse_next_action(&snapshot, &comparison, None);

        assert_eq!(
            action.sortie_intent,
            Some(DoghouseSortieIntent::ManualProbe)
        );
        assert_eq!(action.action, "request_review");
    }

    #[test]
    fn doghouse_next_action_keeps_human_review_flow_when_rabbit_is_on_cooldown() {
        let mut overview = sample_pr_overview();
        overview.merge_state = "BLOCKED".to_owned();
        let mut snapshot = snapshot_fixture(
            "2026-03-25T08:20:00Z",
            "20260325T082000Z",
            overview,
            vec!["review decision: REVIEW_REQUIRED", "merge state: BLOCKED"],
            vec![PrCheckSummary {
                name: "Tests".to_owned(),
                bucket: "pass".to_owned(),
                state: "SUCCESS".to_owned(),
            }],
            vec![],
        );
        snapshot.code_rabbit = Some(DoghouseCodeRabbitState {
            summary_comment_url: "https://github.com/flyingrobots/echo/pull/308#issuecomment-1"
                .to_owned(),
            summary_comment_database_id: Some(4123437114),
            summary_comment_updated_at: "2026-03-25T07:01:37Z".to_owned(),
            summary_state: DoghouseCodeRabbitSummaryState::RateLimited,
            callout_present: true,
            callout_kind: Some("WARNING".to_owned()),
            callout_title: Some("Rate limit exceeded".to_owned()),
            cooldown_active: true,
            cooldown_expires_at: Some("2026-03-25T07:07:36Z".to_owned()),
            cooldown_remaining_seconds: Some(359),
            rearm_actionable: false,
            checkboxes: Vec::new(),
            request_review_actionable: false,
        });

        let comparison = initial_comparison(&snapshot);
        let action = determine_doghouse_next_action(&snapshot, &comparison, None);

        assert_eq!(action.action, "request_review");
        assert!(action
            .reasons
            .iter()
            .any(|reason| { reason.contains("CodeRabbit summary comment is rate-limited until") }));
        assert!(action
            .reasons
            .iter()
            .any(|reason| reason.contains("human and Codex review state separately")));
    }

    #[test]
    fn doghouse_next_action_rearms_coderabbit_when_checkbox_gate_is_immediate_blocker() {
        let mut overview = sample_pr_overview();
        overview.merge_state = "BLOCKED".to_owned();
        let mut snapshot = snapshot_fixture(
            "2026-03-25T08:20:00Z",
            "20260325T082000Z",
            overview,
            vec!["review decision: REVIEW_REQUIRED", "merge state: BLOCKED"],
            vec![PrCheckSummary {
                name: "Tests".to_owned(),
                bucket: "pass".to_owned(),
                state: "SUCCESS".to_owned(),
            }],
            vec![],
        );
        snapshot.code_rabbit = Some(DoghouseCodeRabbitState {
            summary_comment_url: "https://github.com/flyingrobots/echo/pull/308#issuecomment-2"
                .to_owned(),
            summary_comment_database_id: Some(4123999999),
            summary_comment_updated_at: "2026-03-25T07:11:37Z".to_owned(),
            summary_state: DoghouseCodeRabbitSummaryState::RearmRequired,
            callout_present: true,
            callout_kind: Some("IMPORTANT".to_owned()),
            callout_title: Some("Active changes detected".to_owned()),
            cooldown_active: false,
            cooldown_expires_at: None,
            cooldown_remaining_seconds: None,
            rearm_actionable: true,
            checkboxes: vec![DoghouseCodeRabbitCheckbox {
                checked: false,
                label: "Resume CodeRabbit review".to_owned(),
                checkbox_id: Some("abc-123".to_owned()),
            }],
            request_review_actionable: false,
        });

        let comparison = initial_comparison(&snapshot);
        let action = determine_doghouse_next_action(&snapshot, &comparison, None);

        assert_eq!(action.action, "rearm_coderabbit");
        assert!(action
            .reasons
            .iter()
            .any(|reason| reason.contains("manual rearm via 1 unchecked checkbox")));
    }

    #[test]
    fn doghouse_next_action_nudges_coderabbit_when_it_is_actionable() {
        let mut overview = sample_pr_overview();
        overview.merge_state = "BLOCKED".to_owned();
        let mut snapshot = snapshot_fixture(
            "2026-03-25T08:20:00Z",
            "20260325T082000Z",
            overview,
            vec!["review decision: REVIEW_REQUIRED", "merge state: BLOCKED"],
            vec![PrCheckSummary {
                name: "Tests".to_owned(),
                bucket: "pass".to_owned(),
                state: "SUCCESS".to_owned(),
            }],
            vec![],
        );
        snapshot.code_rabbit = Some(DoghouseCodeRabbitState {
            summary_comment_url: "https://github.com/flyingrobots/echo/pull/308#issuecomment-6"
                .to_owned(),
            summary_comment_database_id: Some(4124000003),
            summary_comment_updated_at: "2026-03-25T08:12:37Z".to_owned(),
            summary_state: DoghouseCodeRabbitSummaryState::Ready,
            callout_present: false,
            callout_kind: None,
            callout_title: None,
            cooldown_active: false,
            cooldown_expires_at: None,
            cooldown_remaining_seconds: None,
            rearm_actionable: false,
            checkboxes: Vec::new(),
            request_review_actionable: true,
        });

        let comparison = initial_comparison(&snapshot);
        let action = determine_doghouse_next_action(&snapshot, &comparison, None);

        assert_eq!(action.action, "nudge_coderabbit");
        assert!(action
            .reasons
            .iter()
            .any(|reason| reason.contains("CodeRabbit is actionable again")));
    }

    #[test]
    fn doghouse_next_action_recaptures_when_affirmative_move_uses_stale_comparison() {
        let previous = snapshot_fixture(
            "2026-03-24T20:00:00Z",
            "20260324T200000Z",
            sample_pr_overview(),
            vec!["review decision: REVIEW_REQUIRED", "merge state: BLOCKED"],
            vec![PrCheckSummary {
                name: "Tests".to_owned(),
                bucket: "pass".to_owned(),
                state: "SUCCESS".to_owned(),
            }],
            vec![],
        );
        let mut overview = sample_pr_overview();
        overview.merge_state = "BLOCKED".to_owned();
        let mut current = snapshot_fixture(
            "2026-03-25T12:30:00Z",
            "20260325T123000Z",
            overview,
            vec!["review decision: REVIEW_REQUIRED", "merge state: BLOCKED"],
            vec![PrCheckSummary {
                name: "Tests".to_owned(),
                bucket: "pass".to_owned(),
                state: "SUCCESS".to_owned(),
            }],
            vec![],
        );
        current.code_rabbit = Some(DoghouseCodeRabbitState {
            summary_comment_url: "https://github.com/flyingrobots/echo/pull/308#issuecomment-7"
                .to_owned(),
            summary_comment_database_id: Some(4124000004),
            summary_comment_updated_at: "2026-03-25T12:29:37Z".to_owned(),
            summary_state: DoghouseCodeRabbitSummaryState::Ready,
            callout_present: false,
            callout_kind: None,
            callout_title: None,
            cooldown_active: false,
            cooldown_expires_at: None,
            cooldown_remaining_seconds: None,
            rearm_actionable: false,
            checkboxes: Vec::new(),
            request_review_actionable: true,
        });
        let selection = select_doghouse_baseline(std::slice::from_ref(&previous), &current)
            .unwrap_or_else(|| unreachable!("baseline should be selected"));
        let delta = build_pr_snapshot_delta(&selection.snapshot, &current);
        let comparison = assess_doghouse_comparison(&current, Some(&selection), Some(&delta));

        assert_eq!(comparison.quality, DoghouseComparisonQuality::Stale);

        let action = determine_doghouse_next_action(&current, &comparison, Some(&delta));

        assert_eq!(action.action, "capture_fresh_sortie");
        assert!(action
            .reasons
            .iter()
            .any(|reason| reason.contains("comparison quality is stale")));
    }

    #[test]
    fn doghouse_sortie_events_emit_expected_kinds() {
        let snapshot = snapshot_fixture(
            "2026-03-25T08:20:00Z",
            "20260325T082000Z",
            sample_pr_overview(),
            vec!["review decision: REVIEW_REQUIRED"],
            vec![PrCheckSummary {
                name: "Tests".to_owned(),
                bucket: "pass".to_owned(),
                state: "SUCCESS".to_owned(),
            }],
            vec![],
        );
        let comparison = assess_doghouse_comparison(&snapshot, None, None);
        let action = determine_doghouse_next_action(&snapshot, &comparison, None);
        let lines = assert_ok(
            build_doghouse_sortie_events(&snapshot, None, &comparison, None, &action),
            "sortie events should serialize",
        );

        assert_eq!(lines.len(), 6);
        assert!(lines[0].contains("\"kind\":\"doghouse.snapshot\""));
        assert!(lines[1].contains("\"kind\":\"doghouse.intent\""));
        assert!(lines[2].contains("\"kind\":\"doghouse.coderabbit\""));
        assert!(lines[3].contains("\"kind\":\"doghouse.baseline\""));
        assert!(lines[4].contains("\"kind\":\"doghouse.comparison\""));
        assert!(lines[5].contains("\"kind\":\"doghouse.next_action\""));
    }

    #[test]
    fn snapshot_loading_defaults_missing_state_for_older_artifacts() {
        let snapshot: PrSnapshotArtifact = assert_ok(
            serde_json::from_str(
                r#"{
                  "recorded_at": "2026-03-25T08:00:00Z",
                  "filename_stamp": "20260325T080000Z",
                  "pr": {
                    "number": 308,
                    "url": "https://github.com/flyingrobots/echo/pull/308",
                    "title": "Older recorder shape",
                    "head_sha": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    "head_sha_short": "aaaaaaaaaaaa",
                    "review_decision": "REVIEW_REQUIRED",
                    "merge_state": "BLOCKED"
                  },
                  "blockers": [],
                  "checks": [],
                  "grouped_checks": {},
                  "unresolved_threads": []
                }"#,
            ),
            "older snapshot shape should deserialize",
        );

        assert_eq!(snapshot.pr.state, "OPEN");
        assert_eq!(snapshot.sortie_intent, None);
        assert_eq!(snapshot.code_rabbit, None);
    }

    #[test]
    fn resolve_targets_require_ids_or_all() {
        let args = PrThreadsResolveArgs {
            all: false,
            selector: None,
            yes: false,
            thread_ids: Vec::new(),
        };
        assert!(resolve_thread_targets(&args).is_err());
    }

    #[test]
    fn resolve_targets_reject_selector_without_all() {
        let args = PrThreadsResolveArgs {
            all: false,
            selector: Some("308".to_owned()),
            yes: false,
            thread_ids: vec!["THREAD".to_owned()],
        };
        assert!(resolve_thread_targets(&args).is_err());
    }

    #[test]
    fn resolve_targets_wrap_explicit_thread_ids() {
        let args = PrThreadsResolveArgs {
            all: false,
            selector: None,
            yes: true,
            thread_ids: vec!["THREAD_A".to_owned(), "THREAD_B".to_owned()],
        };
        let targets = assert_ok(
            resolve_thread_targets(&args),
            "explicit thread ids should work",
        );
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].thread_id, "THREAD_A");
        assert_eq!(targets[1].thread_id, "THREAD_B");
    }

    #[test]
    fn review_thread_summary_formats_locations() {
        let thread = ReviewThreadSummary {
            thread_id: "THREAD".to_owned(),
            comment_id: Some(42),
            author: Some("coderabbitai".to_owned()),
            url: Some("https://example.invalid".to_owned()),
            path: Some("xtask/src/main.rs".to_owned()),
            line: Some(123),
            is_outdated: false,
            preview: "preview".to_owned(),
        };
        assert_eq!(thread.display_location(), "xtask/src/main.rs:123");
    }

    #[test]
    fn review_thread_page_deserializes_expected_fields() {
        let page: ReviewThreadsQueryResponse = assert_ok(
            serde_json::from_str(
                r#"{
              "data": {
                "repository": {
                  "pullRequest": {
                    "reviewThreads": {
                      "nodes": [
                        {
                          "id": "THREAD_1",
                          "isResolved": false,
                          "isOutdated": true,
                          "path": "xtask/src/main.rs",
                          "line": 42,
                          "originalLine": 40,
                          "comments": {
                            "nodes": [
                              {
                                "databaseId": 123456,
                                "url": "https://github.com/flyingrobots/echo/pull/308#discussion_r123456",
                                "body": "Please tighten this branch.",
                                "author": { "login": "coderabbitai" }
                              }
                            ]
                          }
                        }
                      ],
                      "pageInfo": {
                        "hasNextPage": true,
                        "endCursor": "page-2"
                      }
                    }
                  }
                }
              }
            }"#,
            ),
            "page should deserialize",
        );

        let connection = page.data.repository.pull_request.review_threads;
        assert_eq!(connection.nodes.len(), 1);
        assert!(connection.page_info.has_next_page);
        assert_eq!(connection.page_info.end_cursor.as_deref(), Some("page-2"));
        let thread = &connection.nodes[0];
        assert_eq!(thread.id, "THREAD_1");
        assert!(thread.is_outdated);
        assert_eq!(thread.comments.nodes[0].database_id, Some(123456));
        assert_eq!(
            thread.comments.nodes[0]
                .author
                .as_ref()
                .map(|author| author.login.as_str()),
            Some("coderabbitai")
        );
    }

    // ── pr_preflight helpers ────────────────────────────────────────

    #[test]
    fn preflight_scope_for_docs_only_branch_enables_docs_checks() {
        let scope = analyze_pr_preflight_scope(
            &[
                "docs/BEARING.md".to_owned(),
                "scripts/hooks/README.md".to_owned(),
            ],
            false,
        );

        assert_eq!(scope.verify_mode, "pr");
        assert!(scope.run_dead_refs);
        assert_eq!(
            scope.markdown_files,
            Some(vec![
                "docs/BEARING.md".to_owned(),
                "scripts/hooks/README.md".to_owned()
            ])
        );
        assert!(!scope.run_feature_contracts);
        assert!(scope.shell_files.is_none());
    }

    #[test]
    fn preflight_scope_for_schema_changes_enables_schema_validation() {
        let scope = analyze_pr_preflight_scope(
            &[
                "schemas/runtime/artifact-a-identifiers.graphql".to_owned(),
                "scripts/validate-runtime-schema-fragments.mjs".to_owned(),
            ],
            false,
        );

        assert!(scope.run_runtime_schema_explicit);
        assert!(!scope.run_feature_contracts);
    }

    #[test]
    fn preflight_scope_for_feature_crates_enables_feature_contracts() {
        let scope = analyze_pr_preflight_scope(
            &[
                "crates/echo-runtime-schema/src/lib.rs".to_owned(),
                "docs/workflows.md".to_owned(),
            ],
            false,
        );

        assert!(scope.run_feature_contracts);
        assert!(scope.run_dead_refs);
    }

    #[test]
    fn preflight_scope_collects_changed_shell_files() {
        let scope = analyze_pr_preflight_scope(
            &[
                "scripts/pr-status.sh".to_owned(),
                "tests/hooks/test_pr_status.sh".to_owned(),
                "scripts/hooks/README.md".to_owned(),
            ],
            false,
        );

        assert_eq!(
            scope.shell_files,
            Some(vec![
                "scripts/pr-status.sh".to_owned(),
                "tests/hooks/test_pr_status.sh".to_owned()
            ])
        );
    }

    #[test]
    fn preflight_scope_skips_deleted_markdown_and_shell_paths() {
        let scope = analyze_pr_preflight_scope(
            &[
                "docs/workflows.md".to_owned(),
                "docs/not-here-anymore.md".to_owned(),
                "scripts/pr-status.sh".to_owned(),
                "scripts/not-here-anymore.sh".to_owned(),
            ],
            false,
        );

        assert_eq!(
            scope.markdown_files,
            Some(vec!["docs/workflows.md".to_owned()])
        );
        assert_eq!(
            scope.shell_files,
            Some(vec!["scripts/pr-status.sh".to_owned()])
        );
    }

    #[test]
    fn preflight_plan_includes_expected_changed_scope_checks() {
        let plan = build_pr_preflight_plan(
            &[
                "docs/workflows.md".to_owned(),
                "scripts/pr-status.sh".to_owned(),
                "crates/echo-runtime-schema/src/lib.rs".to_owned(),
            ],
            false,
        );
        let labels: Vec<_> = plan.iter().map(|check| check.label.as_str()).collect();

        assert_eq!(labels[0], "local verification (pr)");
        assert!(labels.contains(&"docs dead refs"));
        assert!(labels.contains(&"markdownlint"));
        assert!(labels.contains(&"feature contract: echo-runtime-schema --no-default-features"));
        assert!(labels.contains(&"feature contract: echo-wasm-abi --no-default-features"));
        assert!(labels.contains(&"shell syntax"));
    }

    #[test]
    fn full_preflight_dead_refs_scans_all_markdown_files_explicitly() {
        let files = vec!["README.md".to_owned(), "docs/workflows.md".to_owned()];
        let command = build_lint_dead_refs_command(Some(&files), true);
        let (program, args) = command_program_and_args(&command);

        assert_eq!(program, "cargo");
        assert!(args.starts_with(&[
            "xtask".to_owned(),
            "lint-dead-refs".to_owned(),
            "--root".to_owned(),
            "docs".to_owned(),
        ]));
        assert!(args.iter().any(|value| value == "--file"));
        assert!(args.iter().any(|value| value == "README.md"));
        assert!(args.iter().any(|value| value == "docs/workflows.md"));
    }

    #[test]
    fn preflight_markdownlint_uses_pinned_pnpm_entrypoint() {
        let command = build_markdownlint_command(&["docs/workflows.md".to_owned()]);
        let (program, args) = command_program_and_args(&command);

        assert_eq!(program, "pnpm");
        assert_eq!(
            args,
            vec![
                "exec".to_owned(),
                "markdownlint-cli2".to_owned(),
                "docs/workflows.md".to_owned()
            ]
        );
    }

    #[test]
    fn maintained_shell_path_filter_excludes_non_shell_assets() {
        assert!(is_maintained_shell_path("scripts/pr-status.sh"));
        assert!(is_maintained_shell_path(".githooks/pre-commit"));
        assert!(is_maintained_shell_path("scripts/hooks/pre-commit"));
        assert!(!is_maintained_shell_path("scripts/hooks/README.md"));
        assert!(!is_maintained_shell_path(
            "scripts/generate-dependency-dags.js"
        ));
        assert!(!is_maintained_shell_path("scripts/bench_bake.py"));
        assert!(!is_maintained_shell_path("scripts/generate_evidence.cjs"));
    }

    #[test]
    fn auth_error_detection_avoids_author_false_positive() {
        assert!(!is_gh_auth_error("author lookup failed"));
        assert!(is_gh_auth_error("authentication required"));
        assert!(is_gh_auth_error("you must authenticate with GitHub"));
        assert!(is_gh_auth_error("bad credentials"));
    }

    #[test]
    fn parse_policy_case_handles_worker_suffix_form() {
        let Ok(Some(case)) = parse_policy_case(Path::new("dynamic_per_worker_4w/1000")) else {
            unreachable!("expected worker suffix policy case");
        };

        assert_eq!(
            case,
            ParsedPolicyCase {
                policy: "dynamic_per_worker".to_owned(),
                workers: "4w".to_owned(),
                worker_hint: None,
                selected_policy: None,
                selected_workers: None,
                load: 1000,
            }
        );
    }

    #[test]
    fn parse_policy_case_handles_unlisted_worker_suffix_form() {
        let Ok(Some(case)) = parse_policy_case(Path::new("static_per_worker_16w/1000")) else {
            unreachable!("expected generic worker suffix policy case");
        };

        assert_eq!(
            case,
            ParsedPolicyCase {
                policy: "static_per_worker".to_owned(),
                workers: "16w".to_owned(),
                worker_hint: None,
                selected_policy: None,
                selected_workers: None,
                load: 1000,
            }
        );
    }

    #[test]
    fn parse_policy_case_handles_dedicated_two_segment_form() {
        let Ok(Some(case)) = parse_policy_case(Path::new("dedicated_per_shard/100")) else {
            unreachable!("expected dedicated policy case");
        };

        assert_eq!(
            case,
            ParsedPolicyCase {
                policy: "dedicated_per_shard".to_owned(),
                workers: "dedicated".to_owned(),
                worker_hint: None,
                selected_policy: None,
                selected_workers: None,
                load: 100,
            }
        );
    }

    #[test]
    fn parse_policy_case_handles_adaptive_selected_plan_form() {
        let Ok(Some(case)) = parse_policy_case(Path::new(
            "adaptive_shard_routing__hint_8w__selected_dynamic_per_shard_4w/10000",
        )) else {
            unreachable!("expected adaptive selected-plan policy case");
        };

        assert_eq!(
            case,
            ParsedPolicyCase {
                policy: "adaptive_shard_routing".to_owned(),
                workers: "8w".to_owned(),
                worker_hint: Some("8w".to_owned()),
                selected_policy: Some("dynamic_per_shard".to_owned()),
                selected_workers: Some("4w".to_owned()),
                load: 10000,
            }
        );
    }

    #[test]
    fn parse_policy_case_errors_on_invalid_adaptive_worker_hint_form() {
        let err = match parse_policy_case(Path::new(
            "adaptive_shard_routing__hint_bad__selected_dynamic_per_worker_1w/1000",
        )) {
            Ok(value) => unreachable!("expected error, got {value:?}"),
            Err(err) => err,
        };
        assert!(
            err.contains("invalid worker hint"),
            "expected invalid worker hint error, got: {err}"
        );
    }

    #[test]
    fn parse_policy_case_errors_on_invalid_adaptive_selected_worker_form() {
        let zero = match parse_policy_case(Path::new(
            "adaptive_shard_routing__hint_8w__selected_dynamic_per_worker_0w/1000",
        )) {
            Ok(value) => unreachable!("expected error, got {value:?}"),
            Err(err) => err,
        };
        let zero_padded = match parse_policy_case(Path::new(
            "adaptive_shard_routing__hint_8w__selected_dynamic_per_worker_01w/1000",
        )) {
            Ok(value) => unreachable!("expected error, got {value:?}"),
            Err(err) => err,
        };

        assert!(
            zero.contains("invalid worker suffix") || zero.contains("invalid selected plan"),
            "expected invalid selected worker error, got: {zero}"
        );
        assert!(
            zero_padded.contains("invalid worker suffix")
                || zero_padded.contains("invalid selected worker suffix")
                || zero_padded.contains("invalid selected plan"),
            "expected zero-padded selected worker error, got: {zero_padded}"
        );
    }

    #[test]
    fn parse_policy_case_errors_on_zero_padded_fixed_worker_suffix() {
        let err = match parse_policy_case(Path::new("dynamic_per_worker_01w/1000")) {
            Ok(value) => unreachable!("expected error, got {value:?}"),
            Err(err) => err,
        };
        assert!(
            err.contains("invalid worker suffix"),
            "expected invalid worker suffix error, got: {err}"
        );
    }

    #[test]
    fn collect_policy_matrix_rows_errors_on_malformed_policy_case() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "echo-xtask-policy-case-{}-{unique}",
            std::process::id()
        ));
        let malformed =
            root.join("adaptive_shard_routing__hint_8w__selected_dynamic_per_worker_01w/1000");
        assert!(
            std::fs::create_dir_all(&malformed).is_ok(),
            "failed to create malformed benchmark case fixture"
        );

        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().map_or_else(
            || PathBuf::from(env!("CARGO_MANIFEST_DIR")),
            Path::to_path_buf,
        );
        let err = match collect_policy_matrix_rows(&root, &repo_root) {
            Ok(value) => unreachable!("expected error, got {value:?}"),
            Err(err) => err,
        };
        let _ = std::fs::remove_dir_all(&root);

        assert!(
            err.to_string().contains("invalid worker suffix"),
            "expected invalid worker suffix error, got: {err:#}"
        );
    }

    #[test]
    fn prefer_policy_matrix_row_keeps_selected_plan_metadata_when_present() {
        let stale = PolicyMatrixRow {
            policy: "adaptive_shard_routing".to_owned(),
            workers: "4w".to_owned(),
            worker_hint: None,
            selected_policy: None,
            selected_workers: None,
            selected_series: None,
            load: 1000,
            path: "target/criterion/parallel_policy_matrix/adaptive_shard_routing_4w/1000/new/estimates.json".to_owned(),
            mean_ns: 1.0,
            lb_ns: None,
            ub_ns: None,
            series: "adaptive_shard_routing:4w".to_owned(),
        };
        let truthful = PolicyMatrixRow {
            policy: "adaptive_shard_routing".to_owned(),
            workers: "4w".to_owned(),
            worker_hint: Some("4w".to_owned()),
            selected_policy: Some("dynamic_per_worker".to_owned()),
            selected_workers: Some("1w".to_owned()),
            selected_series: Some("dynamic_per_worker:1w".to_owned()),
            load: 1000,
            path: "target/criterion/parallel_policy_matrix/adaptive_shard_routing__hint_4w__selected_dynamic_per_worker_1w/1000/new/estimates.json".to_owned(),
            mean_ns: 1.0,
            lb_ns: None,
            ub_ns: None,
            series: "adaptive_shard_routing:4w".to_owned(),
        };

        assert!(prefer_policy_matrix_row(&stale, 1, &truthful, 1));
        assert!(!prefer_policy_matrix_row(&truthful, 1, &stale, 1));
    }

    #[test]
    fn prefer_policy_matrix_row_prefers_newer_truthful_row_for_same_case() {
        let older = PolicyMatrixRow {
            policy: "adaptive_shard_routing".to_owned(),
            workers: "4w".to_owned(),
            worker_hint: Some("4w".to_owned()),
            selected_policy: Some("dynamic_per_worker".to_owned()),
            selected_workers: Some("1w".to_owned()),
            selected_series: Some("dynamic_per_worker:1w".to_owned()),
            load: 1000,
            path: "target/criterion/parallel_policy_matrix/adaptive_shard_routing__hint_4w__selected_dynamic_per_worker_1w/1000/new/estimates.json".to_owned(),
            mean_ns: 1.0,
            lb_ns: None,
            ub_ns: None,
            series: "adaptive_shard_routing:4w".to_owned(),
        };
        let newer = PolicyMatrixRow {
            path: "target/criterion/parallel_policy_matrix/adaptive_shard_routing__hint_4w__selected_dynamic_per_worker_1w/1000/base/estimates.json".to_owned(),
            ..older.clone()
        };

        assert!(prefer_policy_matrix_row(&older, 10, &newer, 20));
        assert!(!prefer_policy_matrix_row(&newer, 20, &older, 10));
    }

    #[test]
    fn policy_matrix_rows_conflict_detects_disagreeing_truthful_rows() {
        let left = PolicyMatrixRow {
            policy: "adaptive_shard_routing".to_owned(),
            workers: "4w".to_owned(),
            worker_hint: Some("4w".to_owned()),
            selected_policy: Some("dynamic_per_worker".to_owned()),
            selected_workers: Some("1w".to_owned()),
            selected_series: Some("dynamic_per_worker:1w".to_owned()),
            load: 1000,
            path: "left".to_owned(),
            mean_ns: 1.0,
            lb_ns: None,
            ub_ns: None,
            series: "adaptive_shard_routing:4w".to_owned(),
        };
        let right = PolicyMatrixRow {
            path: "right".to_owned(),
            selected_policy: Some("dynamic_per_shard".to_owned()),
            selected_workers: Some("4w".to_owned()),
            selected_series: Some("dynamic_per_shard:4w".to_owned()),
            ..left.clone()
        };

        assert!(policy_matrix_rows_conflict(&left, &right));
        assert!(!policy_matrix_rows_conflict(&left, &left));
    }

    #[test]
    #[should_panic(expected = "conflicting benchmark rows for adaptive_shard_routing:4w load 1000")]
    fn insert_policy_matrix_row_rejects_conflicting_truthful_rows() {
        let mut by_case = BTreeMap::new();
        let left = PolicyMatrixRow {
            policy: "adaptive_shard_routing".to_owned(),
            workers: "4w".to_owned(),
            worker_hint: Some("4w".to_owned()),
            selected_policy: Some("dynamic_per_worker".to_owned()),
            selected_workers: Some("1w".to_owned()),
            selected_series: Some("dynamic_per_worker:1w".to_owned()),
            load: 1000,
            path: "target/criterion/left/new/estimates.json".to_owned(),
            mean_ns: 1.0,
            lb_ns: None,
            ub_ns: None,
            series: "adaptive_shard_routing:4w".to_owned(),
        };
        let right = PolicyMatrixRow {
            path: "target/criterion/right/new/estimates.json".to_owned(),
            selected_policy: Some("dynamic_per_shard".to_owned()),
            selected_workers: Some("4w".to_owned()),
            selected_series: Some("dynamic_per_shard:4w".to_owned()),
            ..left.clone()
        };

        insert_policy_matrix_row(&mut by_case, Path::new("target/criterion"), left, 10);
        insert_policy_matrix_row(&mut by_case, Path::new("target/criterion"), right, 20);
    }

    #[test]
    fn benchmark_inline_script_embeds_policy_payload_metadata() {
        let script = assert_ok(
            build_benchmark_inline_script(
                &[],
                &[],
                &PolicyMatrixPayload {
                    group: BENCH_POLICY_GROUP.to_owned(),
                    baked_at: Some("2026-03-28T22:40:30Z".to_owned()),
                    baked_git_sha: Some("deadbeef".to_owned()),
                    baked_source_digest: Some("cafebabe".to_owned()),
                    template_path: Some("docs/benchmarks/index.html".to_owned()),
                    machine: Some(BenchMachineDescriptor {
                        os: "macos".to_owned(),
                        arch: "aarch64".to_owned(),
                        hostname: None,
                        label: "macos/aarch64".to_owned(),
                    }),
                    criterion_root: Some("target/criterion/parallel_policy_matrix".to_owned()),
                    results: vec![PolicyMatrixRow {
                        policy: "static_per_worker".to_owned(),
                        workers: "4w".to_owned(),
                        worker_hint: None,
                        selected_policy: None,
                        selected_workers: None,
                        selected_series: None,
                        load: 1000,
                        path: "target/criterion/parallel_policy_matrix/static_per_worker_4w/1000/new/estimates.json".to_owned(),
                        mean_ns: 138309.37,
                        lb_ns: Some(137130.76),
                        ub_ns: Some(139395.20),
                        series: "static_per_worker:4w".to_owned(),
                    }],
                },
            ),
            "inline benchmark script should serialize",
        );

        assert!(script.contains("window.__POLICY_MATRIX__ ="));
        assert!(script.contains("\"baked_at\":\"2026-03-28T22:40:30Z\""));
        assert!(script.contains("\"baked_git_sha\":\"deadbeef\""));
        assert!(script.contains("\"baked_source_digest\":\"cafebabe\""));
        assert!(script.contains("\"template_path\":\"docs/benchmarks/index.html\""));
        assert!(script.contains("\"criterion_root\":\"target/criterion/parallel_policy_matrix\""));
    }
}
