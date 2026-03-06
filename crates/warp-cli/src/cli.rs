// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! CLI type definitions for `echo-cli`.
//!
//! Extracted into a separate module for testability — `try_parse_from` lets
//! us verify argument parsing without spawning processes.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

/// Echo developer CLI.
#[derive(Parser, Debug)]
#[command(
    name = "echo-cli",
    about = "Echo developer CLI",
    version,
    disable_help_subcommand = true
)]
pub struct Cli {
    /// Output format (text or json).
    #[arg(long, global = true, default_value = "text", value_enum)]
    pub format: OutputFormat,

    /// Subcommand to execute.
    #[command(subcommand)]
    pub command: Commands,
}

/// Available subcommands.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Verify hash integrity of a WSC snapshot.
    Verify {
        /// Path to WSC snapshot file.
        snapshot: PathBuf,

        /// Expected state root hash (hex) for warp 0 only; additional warps
        /// report "unchecked".
        #[arg(long)]
        expected: Option<String>,
    },

    /// Run benchmarks and format results.
    Bench {
        /// Filter benchmarks by pattern.
        #[arg(long)]
        filter: Option<String>,
    },

    /// Inspect a WSC snapshot.
    Inspect {
        /// Path to WSC snapshot file.
        snapshot: PathBuf,

        /// Show ASCII tree of graph structure.
        #[arg(long)]
        tree: bool,
    },
}

/// Output format selector.
#[derive(Clone, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable text output.
    #[default]
    Text,
    /// Machine-readable JSON output.
    Json,
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn parse_verify_with_snapshot_path() {
        let cli = Cli::try_parse_from(["echo-cli", "verify", "state.wsc"]).unwrap();
        match cli.command {
            Commands::Verify {
                ref snapshot,
                ref expected,
            } => {
                assert_eq!(snapshot, &PathBuf::from("state.wsc"));
                assert!(expected.is_none());
            }
            _ => panic!("expected Verify command"),
        }
        assert_eq!(cli.format, OutputFormat::Text);
    }

    #[test]
    fn parse_verify_with_expected_hash() {
        let cli =
            Cli::try_parse_from(["echo-cli", "verify", "state.wsc", "--expected", "abcd1234"])
                .unwrap();
        match cli.command {
            Commands::Verify { ref expected, .. } => {
                assert_eq!(expected.as_deref(), Some("abcd1234"));
            }
            _ => panic!("expected Verify command"),
        }
    }

    #[test]
    fn format_json_before_subcommand() {
        let cli =
            Cli::try_parse_from(["echo-cli", "--format", "json", "verify", "test.wsc"]).unwrap();
        assert_eq!(cli.format, OutputFormat::Json);
        assert!(matches!(cli.command, Commands::Verify { .. }));
    }

    #[test]
    fn format_json_after_subcommand() {
        let cli =
            Cli::try_parse_from(["echo-cli", "verify", "test.wsc", "--format", "json"]).unwrap();
        assert_eq!(cli.format, OutputFormat::Json);
    }

    #[test]
    fn parse_bench_no_filter() {
        let cli = Cli::try_parse_from(["echo-cli", "bench"]).unwrap();
        match cli.command {
            Commands::Bench { ref filter } => assert!(filter.is_none()),
            _ => panic!("expected Bench command"),
        }
    }

    #[test]
    fn parse_bench_with_filter() {
        let cli = Cli::try_parse_from(["echo-cli", "bench", "--filter", "hotpath"]).unwrap();
        match cli.command {
            Commands::Bench { ref filter } => {
                assert_eq!(filter.as_deref(), Some("hotpath"));
            }
            _ => panic!("expected Bench command"),
        }
    }

    #[test]
    fn parse_inspect_basic() {
        let cli = Cli::try_parse_from(["echo-cli", "inspect", "state.wsc"]).unwrap();
        match cli.command {
            Commands::Inspect { ref snapshot, tree } => {
                assert_eq!(snapshot, &PathBuf::from("state.wsc"));
                assert!(!tree);
            }
            _ => panic!("expected Inspect command"),
        }
    }

    #[test]
    fn parse_inspect_with_tree() {
        let cli = Cli::try_parse_from(["echo-cli", "inspect", "state.wsc", "--tree"]).unwrap();
        match cli.command {
            Commands::Inspect { tree, .. } => assert!(tree),
            _ => panic!("expected Inspect command"),
        }
    }

    #[test]
    fn unknown_subcommand_is_error() {
        let result = Cli::try_parse_from(["echo-cli", "bogus"]);
        assert!(result.is_err());
    }

    #[test]
    fn no_subcommand_is_error() {
        let result = Cli::try_parse_from(["echo-cli"]);
        assert!(result.is_err());
    }

    #[test]
    fn default_format_is_text() {
        let cli = Cli::try_parse_from(["echo-cli", "bench"]).unwrap();
        assert_eq!(cli.format, OutputFormat::Text);
    }
}
