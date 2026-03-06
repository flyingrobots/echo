// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! `echo-cli bench` — run benchmarks and format results.
//!
//! Shells out to `cargo bench -p warp-benches`, parses Criterion JSON from
//! `target/criterion/*/new/estimates.json`, and renders an ASCII table or
//! JSON array.

use std::path::Path;
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
pub(crate) fn run(filter: Option<&str>, format: &OutputFormat) -> Result<()> {
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
    let results = collect_criterion_results(Path::new("target/criterion"), filter)?;

    if results.is_empty() {
        let text = "No benchmark results found.\n";
        let json = serde_json::json!({ "benchmarks": [], "message": "no results found" });
        eprintln!("warning: no benchmark results found in target/criterion/");
        emit(format, text, &json)?;
        return Ok(());
    }

    // 3. Format output.
    let text = format_table(&results);
    let json = serde_json::to_value(&results).context("failed to serialize bench results")?;
    let json = serde_json::json!({ "benchmarks": json });

    emit(format, &text, &json)?;
    Ok(())
}

/// Scans `target/criterion/*/new/estimates.json` for benchmark results.
pub(crate) fn collect_criterion_results(
    criterion_dir: &Path,
    filter: Option<&str>,
) -> Result<Vec<BenchResult>> {
    let mut results = Vec::new();

    if !criterion_dir.is_dir() {
        return Ok(results);
    }

    let entries = std::fs::read_dir(criterion_dir)
        .with_context(|| format!("failed to read {}", criterion_dir.display()))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let bench_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        // Skip Criterion metadata directories.
        if bench_name.starts_with('.') || bench_name == "report" {
            continue;
        }

        // Apply filter if specified.
        if let Some(f) = filter {
            if !bench_name.contains(f) {
                continue;
            }
        }

        let estimates_path = path.join("new").join("estimates.json");
        if !estimates_path.is_file() {
            continue;
        }

        match parse_estimates(&bench_name, &estimates_path) {
            Ok(result) => results.push(result),
            Err(e) => eprintln!("warning: skipping {bench_name}: {e:#}"),
        }
    }

    results.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(results)
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
    })
}

/// Formats benchmark results as an ASCII table.
pub(crate) fn format_table(results: &[BenchResult]) -> String {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec!["Benchmark", "Mean", "Median", "Std Dev"]);

    for r in results {
        table.add_row(vec![
            r.name.clone(),
            format_duration(r.mean_ns),
            format_duration(r.median_ns),
            format_duration(r.stddev_ns),
        ]);
    }

    format!("{table}\n")
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
        format!("{:.2} ns", ns)
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
            },
            BenchResult {
                name: "materialize".to_string(),
                mean_ns: 456_700.0,
                median_ns: 450_200.0,
                stddev_ns: 32_100.0,
            },
        ];

        let table = format_table(&results);
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
}
