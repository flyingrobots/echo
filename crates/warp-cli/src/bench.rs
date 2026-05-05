// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! `echo-cli bench` — run benchmarks and format results.
//!
//! Shells out to `cargo bench -p warp-benches`, recursively parses Criterion
//! JSON from `target/criterion/**/new/estimates.json`, and renders an ASCII
//! table or JSON array.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use comfy_table::{ContentArrangement, Table};
use serde::{Deserialize, Serialize};

use crate::cli::OutputFormat;
use crate::output::emit;

/// Parsed benchmark result from Criterion's `estimates.json`.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct BenchResult {
    pub(crate) name: String,
    pub(crate) mean_ns: f64,
    pub(crate) median_ns: f64,
    pub(crate) stddev_ns: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) baseline_ns: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) delta_pct: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) baseline_status: Option<String>,
}

/// Baseline metadata included in JSON output when `--baseline` is supplied.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct BaselineInfo {
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) found: bool,
}

/// Raw Criterion estimates JSON structure.
#[derive(Debug, Deserialize)]
pub(crate) struct CriterionEstimates {
    pub(crate) mean: Estimate,
    pub(crate) median: Estimate,
    pub(crate) std_dev: Estimate,
}

/// A single Criterion estimate.
#[derive(Debug, Deserialize)]
pub(crate) struct Estimate {
    pub(crate) point_estimate: f64,
}

/// Describes a process exit caused by a signal (Unix) or unknown termination.
fn format_signal(status: &std::process::ExitStatus) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        match status.signal() {
            Some(sig) => format!("killed by signal {sig}"),
            None => "unknown termination".to_string(),
        }
    }
    #[cfg(not(unix))]
    {
        let _ = status;
        "unknown termination".to_string()
    }
}

/// Builds the `cargo bench` command with optional Criterion regex filter.
pub(crate) fn build_bench_command(filter: Option<&str>) -> Command {
    let mut cmd = Command::new("cargo");
    cmd.args(["bench", "-p", "warp-benches"]);

    if let Some(f) = filter {
        cmd.args(["--", f]);
    }

    // Inherit stdout/stderr so Criterion progress is visible.
    cmd.stdout(std::process::Stdio::inherit());
    cmd.stderr(std::process::Stdio::inherit());

    cmd
}

/// Runs the bench subcommand.
pub(crate) fn run(
    filter: Option<&str>,
    baseline: Option<&str>,
    format: &OutputFormat,
) -> Result<()> {
    // 1. Shell out to cargo bench.
    let mut cmd = build_bench_command(filter);

    let status = cmd
        .status()
        .context("failed to run cargo bench (is cargo available?)")?;

    if !status.success() {
        let code_desc = match status.code() {
            Some(code) => format!("exit code {code}"),
            None => format_signal(&status),
        };
        bail!("cargo bench failed: {code_desc}");
    }

    // 2. Parse Criterion JSON results.
    let mut results = collect_criterion_results(Path::new("target/criterion"), filter)?;
    let baseline_info = if let Some(name) = baseline {
        Some(apply_named_baseline(name, &mut results)?)
    } else {
        None
    };

    if results.is_empty() {
        let text = "No benchmark results found.\n";
        let json = serde_json::json!({ "benchmarks": [], "baseline": baseline_info, "message": "no results found" });
        eprintln!("warning: no benchmark results found in target/criterion/");
        emit(format, text, &json)?;
        return Ok(());
    }

    // 3. Format output.
    let text = format_table(&results, baseline_info.as_ref());
    let json = serde_json::to_value(&results).context("failed to serialize bench results")?;
    let json = serde_json::json!({ "benchmarks": json, "baseline": baseline_info });

    emit(format, &text, &json)?;
    Ok(())
}

/// Recursively scans `criterion_dir` for `new/estimates.json` files.
///
/// Criterion stores grouped and parameterised benchmarks in nested directories
/// (e.g. `group/bench/new/estimates.json` or `group/bench/param/new/estimates.json`).
/// The benchmark name is derived from the path relative to `criterion_dir`.
pub(crate) fn collect_criterion_results(
    criterion_dir: &Path,
    filter: Option<&str>,
) -> Result<Vec<BenchResult>> {
    let mut results = Vec::new();

    if !criterion_dir.is_dir() {
        return Ok(results);
    }

    let filter_re = filter
        .map(|f| regex::Regex::new(f).with_context(|| format!("invalid filter regex: {f}")))
        .transpose()?;

    collect_estimates_recursive(
        criterion_dir,
        criterion_dir,
        filter_re.as_ref(),
        &mut results,
    )?;

    results.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(results)
}

/// Walks `dir` recursively, collecting any `new/estimates.json` it finds.
fn collect_estimates_recursive(
    root: &Path,
    dir: &Path,
    filter_re: Option<&regex::Regex>,
    results: &mut Vec<BenchResult>,
) -> Result<()> {
    let entries =
        std::fs::read_dir(dir).with_context(|| format!("failed to read {}", dir.display()))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Skip Criterion metadata directories.
        if dir_name.starts_with('.') || dir_name == "report" {
            continue;
        }

        let estimates_path = path.join("new").join("estimates.json");
        if estimates_path.is_file() {
            let bench_name = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");

            // Apply regex filter (matches Criterion's own regex semantics).
            if let Some(re) = filter_re {
                if !re.is_match(&bench_name) {
                    continue;
                }
            }

            match parse_estimates(&bench_name, &estimates_path) {
                Ok(result) => results.push(result),
                Err(e) => eprintln!("warning: skipping {bench_name}: {e:#}"),
            }
        } else {
            // No estimates here — recurse deeper.
            collect_estimates_recursive(root, &path, filter_re, results)?;
        }
    }

    Ok(())
}

/// Parses a single `estimates.json` file into a `BenchResult`.
pub(crate) fn parse_estimates(name: &str, path: &Path) -> Result<BenchResult> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let estimates: CriterionEstimates = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse {}", path.display()))?;

    Ok(BenchResult {
        name: name.to_string(),
        mean_ns: estimates.mean.point_estimate,
        median_ns: estimates.median.point_estimate,
        stddev_ns: estimates.std_dev.point_estimate,
        baseline_ns: None,
        delta_pct: None,
        baseline_status: None,
    })
}

fn baseline_path(name: &str) -> PathBuf {
    if name == "main" {
        PathBuf::from("perf-baseline.json")
    } else {
        PathBuf::from(format!("perf-baseline.{name}.json"))
    }
}

fn apply_named_baseline(name: &str, results: &mut [BenchResult]) -> Result<BaselineInfo> {
    let path = baseline_path(name);
    let info = BaselineInfo {
        name: name.to_string(),
        path: path.display().to_string(),
        found: path.is_file(),
    };

    if !info.found {
        return Ok(info);
    }

    let baseline = load_baseline(&path)?;
    apply_baseline(results, &baseline);
    Ok(info)
}

fn load_baseline(path: &Path) -> Result<BTreeMap<String, f64>> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read baseline {}", path.display()))?;
    serde_json::from_str(&content)
        .with_context(|| format!("failed to parse baseline {}", path.display()))
}

fn apply_baseline(results: &mut [BenchResult], baseline: &BTreeMap<String, f64>) {
    for result in results {
        let Some(base) = baseline.get(&result.name).copied() else {
            result.baseline_status = Some("NEW".to_string());
            continue;
        };

        result.baseline_ns = Some(base);
        if base > 0.0 && base.is_finite() {
            result.delta_pct = Some(((result.median_ns - base) / base) * 100.0);
            result.baseline_status = Some("OK".to_string());
        } else {
            result.baseline_status = Some("INVALID_BASELINE".to_string());
        }
    }
}

/// Formats benchmark results as an ASCII table.
pub(crate) fn format_table(results: &[BenchResult], baseline: Option<&BaselineInfo>) -> String {
    use std::fmt::Write as _;

    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    let show_baseline = baseline.is_some_and(|info| info.found);
    if show_baseline {
        table.set_header(vec![
            "Benchmark",
            "Mean",
            "Median",
            "Std Dev",
            "Baseline",
            "Delta",
            "Status",
        ]);
    } else {
        table.set_header(vec!["Benchmark", "Mean", "Median", "Std Dev"]);
    }

    for r in results {
        let mut row = vec![
            r.name.clone(),
            format_duration(r.mean_ns),
            format_duration(r.median_ns),
            format_duration(r.stddev_ns),
        ];
        if show_baseline {
            row.push(
                r.baseline_ns
                    .map_or_else(|| "\u{2014}".to_string(), format_duration),
            );
            row.push(
                r.delta_pct
                    .map_or_else(|| "\u{2014}".to_string(), |delta| format!("{delta:+.1}%")),
            );
            row.push(
                r.baseline_status
                    .as_deref()
                    .unwrap_or("\u{2014}")
                    .to_string(),
            );
        }
        table.add_row(row);
    }

    let mut out = String::new();
    if let Some(info) = baseline {
        if info.found {
            let _ = writeln!(out, "Baseline: {} ({})", info.name, info.path);
        } else {
            let _ = writeln!(
                out,
                "No baseline found at {}; showing absolute values only.",
                info.path
            );
        }
        let _ = writeln!(out);
    }
    let _ = writeln!(out, "{table}");
    out
}

/// Formats nanosecond durations in human-readable form.
fn format_duration(ns: f64) -> String {
    if ns.is_nan() || ns.is_infinite() || ns < 0.0 {
        return "N/A".to_string();
    }
    if ns >= 1_000_000_000.0 {
        format!("{:.2} s", ns / 1_000_000_000.0)
    } else if ns >= 1_000_000.0 {
        format!("{:.2} ms", ns / 1_000_000.0)
    } else if ns >= 1_000.0 {
        #[allow(clippy::unicode_not_nfc)]
        {
            format!("{:.2} \u{00b5}s", ns / 1_000.0)
        }
    } else {
        format!("{ns:.2} ns")
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::fs;

    fn make_estimates_json(mean: f64, median: f64, stddev: f64) -> String {
        serde_json::json!({
            "mean": { "confidence_interval": { "confidence_level": 0.95, "lower_bound": mean - 10.0, "upper_bound": mean + 10.0 }, "point_estimate": mean, "standard_error": 1.0 },
            "median": { "confidence_interval": { "confidence_level": 0.95, "lower_bound": median - 10.0, "upper_bound": median + 10.0 }, "point_estimate": median, "standard_error": 1.0 },
            "std_dev": { "confidence_interval": { "confidence_level": 0.95, "lower_bound": stddev - 1.0, "upper_bound": stddev + 1.0 }, "point_estimate": stddev, "standard_error": 0.5 },
            "median_abs_dev": { "confidence_interval": { "confidence_level": 0.95, "lower_bound": 0.0, "upper_bound": 10.0 }, "point_estimate": 5.0, "standard_error": 1.0 },
            "slope": null
        })
        .to_string()
    }

    #[test]
    fn parse_mock_criterion_json() {
        let dir = tempfile::tempdir().unwrap();
        let bench_dir = dir.path().join("my_bench").join("new");
        fs::create_dir_all(&bench_dir).unwrap();

        let estimates = make_estimates_json(1_234_567.0, 1_200_000.0, 50_000.0);
        fs::write(bench_dir.join("estimates.json"), &estimates).unwrap();

        let results = collect_criterion_results(dir.path(), None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "my_bench");
        assert!((results[0].mean_ns - 1_234_567.0).abs() < 0.01);
        assert!((results[0].median_ns - 1_200_000.0).abs() < 0.01);
        assert!((results[0].stddev_ns - 50_000.0).abs() < 0.01);
    }

    #[test]
    fn table_formatter_produces_expected_output() {
        let results = vec![
            BenchResult {
                name: "tick_pipeline".to_string(),
                mean_ns: 1_230_000.0,
                median_ns: 1_210_000.0,
                stddev_ns: 120_000.0,
                baseline_ns: None,
                delta_pct: None,
                baseline_status: None,
            },
            BenchResult {
                name: "materialize".to_string(),
                mean_ns: 456_700.0,
                median_ns: 450_200.0,
                stddev_ns: 32_100.0,
                baseline_ns: None,
                delta_pct: None,
                baseline_status: None,
            },
        ];

        let table = format_table(&results, None);
        assert!(
            table.contains("tick_pipeline"),
            "table should contain bench name"
        );
        assert!(
            table.contains("1.23 ms"),
            "table should contain formatted mean"
        );
        assert!(table.contains("Benchmark"), "table should have header");
    }

    #[test]
    fn json_output_is_valid_json() {
        let results = vec![BenchResult {
            name: "test".to_string(),
            mean_ns: 100.0,
            median_ns: 95.0,
            stddev_ns: 5.0,
            baseline_ns: None,
            delta_pct: None,
            baseline_status: None,
        }];

        let json = serde_json::to_value(&results).unwrap();
        assert!(json.is_array());
        assert_eq!(json.as_array().unwrap().len(), 1);
        assert_eq!(json[0]["name"], "test");
    }

    #[test]
    fn filter_applies_correctly() {
        let dir = tempfile::tempdir().unwrap();

        for name in &["alpha_bench", "beta_bench", "gamma_bench"] {
            let bench_dir = dir.path().join(name).join("new");
            fs::create_dir_all(&bench_dir).unwrap();
            let est = make_estimates_json(1000.0, 1000.0, 10.0);
            fs::write(bench_dir.join("estimates.json"), &est).unwrap();
        }

        let results = collect_criterion_results(dir.path(), Some("beta")).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "beta_bench");
    }

    #[test]
    fn filter_uses_regex_semantics() {
        let dir = tempfile::tempdir().unwrap();

        for name in &["hotpath_alloc", "hotpath_dealloc", "coldpath_alloc"] {
            let bench_dir = dir.path().join(name).join("new");
            fs::create_dir_all(&bench_dir).unwrap();
            let est = make_estimates_json(1000.0, 1000.0, 10.0);
            fs::write(bench_dir.join("estimates.json"), &est).unwrap();
        }

        // Regex anchor should work, not just substring contains.
        let results = collect_criterion_results(dir.path(), Some("^hotpath")).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.name.starts_with("hotpath")));

        // Exact match via anchors.
        let results = collect_criterion_results(dir.path(), Some("^coldpath_alloc$")).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "coldpath_alloc");
    }

    #[test]
    fn collects_nested_grouped_benchmarks() {
        let dir = tempfile::tempdir().unwrap();

        // Simulate Criterion benchmark_group layout:
        // group/bench_a/new/estimates.json
        // group/bench_b/new/estimates.json
        for name in &["bench_a", "bench_b"] {
            let bench_dir = dir.path().join("my_group").join(name).join("new");
            fs::create_dir_all(&bench_dir).unwrap();
            let est = make_estimates_json(2000.0, 1900.0, 100.0);
            fs::write(bench_dir.join("estimates.json"), &est).unwrap();
        }

        let results = collect_criterion_results(dir.path(), None).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|r| r.name == "my_group/bench_a"));
        assert!(results.iter().any(|r| r.name == "my_group/bench_b"));
    }

    #[test]
    fn collects_deeply_nested_parameterised_benchmarks() {
        let dir = tempfile::tempdir().unwrap();

        // Simulate BenchmarkId layout:
        // group/bench/param_1/new/estimates.json
        // group/bench/param_2/new/estimates.json
        for param in &["param_1", "param_2"] {
            let bench_dir = dir
                .path()
                .join("group")
                .join("bench")
                .join(param)
                .join("new");
            fs::create_dir_all(&bench_dir).unwrap();
            let est = make_estimates_json(3000.0, 2900.0, 200.0);
            fs::write(bench_dir.join("estimates.json"), &est).unwrap();
        }

        let results = collect_criterion_results(dir.path(), None).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|r| r.name == "group/bench/param_1"));
        assert!(results.iter().any(|r| r.name == "group/bench/param_2"));
    }

    #[test]
    fn no_results_returns_empty_vec() {
        let dir = tempfile::tempdir().unwrap();
        let results = collect_criterion_results(dir.path(), None).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn format_duration_scales() {
        assert_eq!(format_duration(500.0), "500.00 ns");
        assert_eq!(format_duration(1_500.0), "1.50 \u{00b5}s");
        assert_eq!(format_duration(1_500_000.0), "1.50 ms");
        assert_eq!(format_duration(1_500_000_000.0), "1.50 s");
    }

    #[test]
    fn format_duration_nan_returns_na() {
        assert_eq!(format_duration(f64::NAN), "N/A");
    }

    #[test]
    fn format_duration_negative_returns_na() {
        assert_eq!(format_duration(-1.0), "N/A");
    }

    #[test]
    fn format_duration_infinity_returns_na() {
        assert_eq!(format_duration(f64::INFINITY), "N/A");
        assert_eq!(format_duration(f64::NEG_INFINITY), "N/A");
    }

    #[test]
    fn nonexistent_criterion_dir_returns_empty() {
        let results = collect_criterion_results(Path::new("/nonexistent/criterion"), None).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn build_bench_command_with_filter_passes_criterion_regex() {
        let cmd = build_bench_command(Some("hotpath"));
        let args: Vec<&std::ffi::OsStr> = cmd.get_args().collect();
        // Filter should appear after "--" (Criterion regex), not "--bench" (cargo target).
        assert!(
            args.contains(&std::ffi::OsStr::new("--")),
            "command should contain '--' separator"
        );
        assert!(
            args.contains(&std::ffi::OsStr::new("hotpath")),
            "command should contain filter pattern"
        );
        // Ensure "--bench" is NOT used for filter.
        let bench_pos = args.iter().position(|a| *a == "--bench");
        assert!(
            bench_pos.is_none(),
            "command should not use --bench for filter"
        );
        // Ensure "--" precedes the filter pattern.
        let sep_pos = args.iter().position(|a| *a == "--").expect("missing --");
        let filter_pos = args
            .iter()
            .position(|a| *a == "hotpath")
            .expect("missing filter");
        assert!(sep_pos < filter_pos, "'--' must precede filter pattern");
    }

    #[test]
    fn build_bench_command_without_filter_omits_separator() {
        let cmd = build_bench_command(None);
        let args: Vec<&std::ffi::OsStr> = cmd.get_args().collect();
        assert!(
            !args.contains(&std::ffi::OsStr::new("--")),
            "command without filter should not contain '--'"
        );
    }

    #[test]
    fn baseline_path_main_uses_repo_baseline() {
        assert_eq!(baseline_path("main"), PathBuf::from("perf-baseline.json"));
        assert_eq!(
            baseline_path("feature"),
            PathBuf::from("perf-baseline.feature.json")
        );
    }

    #[test]
    fn apply_baseline_adds_delta_status() {
        let mut results = vec![BenchResult {
            name: "tick_pipeline".to_string(),
            mean_ns: 120.0,
            median_ns: 110.0,
            stddev_ns: 3.0,
            baseline_ns: None,
            delta_pct: None,
            baseline_status: None,
        }];
        let baseline = BTreeMap::from([("tick_pipeline".to_string(), 100.0)]);

        apply_baseline(&mut results, &baseline);

        assert_eq!(results[0].baseline_ns, Some(100.0));
        assert_eq!(results[0].delta_pct, Some(10.0));
        assert_eq!(results[0].baseline_status.as_deref(), Some("OK"));
    }

    #[test]
    fn apply_baseline_marks_new_benchmark() {
        let mut results = vec![BenchResult {
            name: "new_bench".to_string(),
            mean_ns: 120.0,
            median_ns: 110.0,
            stddev_ns: 3.0,
            baseline_ns: None,
            delta_pct: None,
            baseline_status: None,
        }];
        apply_baseline(&mut results, &BTreeMap::new());

        assert_eq!(results[0].baseline_ns, None);
        assert_eq!(results[0].delta_pct, None);
        assert_eq!(results[0].baseline_status.as_deref(), Some("NEW"));
    }

    #[test]
    fn table_with_baseline_shows_delta_columns() {
        let results = vec![BenchResult {
            name: "tick_pipeline".to_string(),
            mean_ns: 120.0,
            median_ns: 110.0,
            stddev_ns: 3.0,
            baseline_ns: Some(100.0),
            delta_pct: Some(10.0),
            baseline_status: Some("OK".to_string()),
        }];
        let info = BaselineInfo {
            name: "main".to_string(),
            path: "perf-baseline.json".to_string(),
            found: true,
        };

        let table = format_table(&results, Some(&info));

        assert!(table.contains("Baseline: main (perf-baseline.json)"));
        assert!(table.contains("Delta"));
        assert!(table.contains("+10.0%"));
        assert!(table.contains("OK"));
    }

    #[test]
    fn missing_baseline_keeps_absolute_table() {
        let results = vec![BenchResult {
            name: "tick_pipeline".to_string(),
            mean_ns: 120.0,
            median_ns: 110.0,
            stddev_ns: 3.0,
            baseline_ns: None,
            delta_pct: None,
            baseline_status: None,
        }];
        let info = BaselineInfo {
            name: "main".to_string(),
            path: "perf-baseline.json".to_string(),
            found: false,
        };

        let table = format_table(&results, Some(&info));

        assert!(table.contains("No baseline found at perf-baseline.json"));
        assert!(!table.contains("Delta"));
        assert!(table.contains("tick_pipeline"));
    }
}
