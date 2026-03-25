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
use serde::Deserialize;
use std::collections::BTreeSet;
use std::ffi::OsStr;
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
    /// Summarize current-head PR status, unresolved threads, and check state.
    PrStatus(PrStatusArgs),
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

#[derive(Args)]
struct PrStatusArgs {
    /// Optional PR number or selector understood by `gh pr view`.
    selector: Option<String>,
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
        Commands::Dags(args) => run_dags(args),
        Commands::PrStatus(args) => run_pr_status(args),
        Commands::PrThreads(args) => run_pr_threads(args),
        Commands::PrPreflight(args) => run_pr_preflight(args),
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

fn run_pr_threads_list(selector: Option<&str>) -> Result<()> {
    let overview = fetch_pr_overview(selector)?;
    let threads = fetch_unresolved_review_threads(&overview)?;

    println!("PR #{}", overview.number);
    println!("URL: {}", overview.url);
    println!("Head SHA: {}", overview.head_sha);
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
    let route = build_review_reply_route(&overview, args.comment_id);

    let output = run_gh_capture([
        "api",
        &route,
        "--method",
        "POST",
        "-f",
        &format!("body={body}"),
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

fn build_review_reply_route(overview: &PrOverview, comment_id: u64) -> String {
    format!(
        "repos/{}/{}/pulls/comments/{comment_id}/replies",
        overview.owner, overview.repo
    )
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
            "Cargo.toml" | "Cargo.lock" | "docs/guide/cargo-features.md"
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
        "number,url,headRefOid,reviewDecision,mergeStateStatus",
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
        url: view.url,
        head_sha: view.head_ref_oid.chars().take(12).collect(),
    })
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
    let output = Command::new("gh")
        .args(args)
        .output()
        .context("failed to spawn `gh` (is GitHub CLI installed?)")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");
    if output.status.success() {
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
    url: String,
    head_sha: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
    url: String,
    #[serde(rename = "headRefOid")]
    head_ref_oid: String,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;
    #[cfg(unix)]
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
        let source = Path::new("docs/meta/docs-index.md");
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
            url: "https://github.com/upstream/fork-target/pull/308".to_owned(),
            head_sha: "deadbeefcafe".to_owned(),
        };

        assert_eq!(
            build_review_reply_route(&overview, 42),
            "repos/upstream/fork-target/pulls/comments/42/replies"
        );
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
                "docs/workflows.md".to_owned(),
                "scripts/hooks/README.md".to_owned(),
            ],
            false,
        );

        assert_eq!(scope.verify_mode, "pr");
        assert!(scope.run_dead_refs);
        assert_eq!(
            scope.markdown_files,
            Some(vec![
                "docs/workflows.md".to_owned(),
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
                "docs/guide/cargo-features.md".to_owned(),
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
}
