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
    #[arg(
        long,
        global = true,
        default_value = "text",
        value_enum,
        hide_possible_values = true
    )]
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

        /// Compare current medians against a saved baseline.
        #[arg(long)]
        baseline: Option<String>,
    },

    /// Inspect a WSC snapshot.
    Inspect {
        /// Path to WSC snapshot file.
        snapshot: PathBuf,

        /// Show ASCII tree of graph structure.
        #[arg(long)]
        tree: bool,

        /// Show attachment payload bytes as hex instead of decoding known payloads.
        #[arg(long)]
        raw: bool,
    },

    /// Inspect Echo WAL recovery posture without mutating storage.
    Wal {
        /// WAL inspection command.
        #[command(subcommand)]
        command: WalCommands,
    },
}

/// WAL inspection subcommands.
#[derive(Subcommand, Debug)]
pub enum WalCommands {
    /// Report read-only WAL recovery posture.
    Doctor {
        /// Filesystem WAL root to inspect.
        #[arg(default_value = ".")]
        root: PathBuf,
    },
    /// Report recovered posture for one submission id/envelope pair.
    SubmissionPosture {
        /// Filesystem WAL root to inspect.
        root: PathBuf,
        /// 64-character hex submission id.
        #[arg(long)]
        submission_id: String,
        /// 64-character hex canonical envelope digest.
        #[arg(long)]
        canonical_envelope_digest: String,
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
    fn invalid_format_is_error() {
        let result = Cli::try_parse_from(["echo-cli", "--format", "yaml", "bench"]);
        assert!(result.is_err());
    }

    #[test]
    fn parse_bench_no_filter() {
        let cli = Cli::try_parse_from(["echo-cli", "bench"]).unwrap();
        match cli.command {
            Commands::Bench {
                ref filter,
                ref baseline,
            } => {
                assert!(filter.is_none());
                assert!(baseline.is_none());
            }
            _ => panic!("expected Bench command"),
        }
    }

    #[test]
    fn parse_bench_with_filter() {
        let cli = Cli::try_parse_from(["echo-cli", "bench", "--filter", "hotpath"]).unwrap();
        match cli.command {
            Commands::Bench { ref filter, .. } => {
                assert_eq!(filter.as_deref(), Some("hotpath"));
            }
            _ => panic!("expected Bench command"),
        }
    }

    #[test]
    fn parse_bench_with_baseline() {
        let cli = Cli::try_parse_from(["echo-cli", "bench", "--baseline", "main"]).unwrap();
        match cli.command {
            Commands::Bench { ref baseline, .. } => {
                assert_eq!(baseline.as_deref(), Some("main"));
            }
            _ => panic!("expected Bench command"),
        }
    }

    #[test]
    fn parse_inspect_basic() {
        let cli = Cli::try_parse_from(["echo-cli", "inspect", "state.wsc"]).unwrap();
        match cli.command {
            Commands::Inspect {
                ref snapshot,
                tree,
                raw,
            } => {
                assert_eq!(snapshot, &PathBuf::from("state.wsc"));
                assert!(!tree);
                assert!(!raw);
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
    fn parse_inspect_with_raw() {
        let cli = Cli::try_parse_from(["echo-cli", "inspect", "state.wsc", "--raw"]).unwrap();
        match cli.command {
            Commands::Inspect { raw, .. } => assert!(raw),
            _ => panic!("expected Inspect command"),
        }
    }

    #[test]
    fn parse_wal_doctor_with_root() {
        let cli = Cli::try_parse_from(["echo-cli", "wal", "doctor", "runtime.wal"]).unwrap();
        match cli.command {
            Commands::Wal {
                command: WalCommands::Doctor { ref root },
            } => assert_eq!(root, &PathBuf::from("runtime.wal")),
            _ => panic!("expected Wal doctor command"),
        }
    }

    #[test]
    fn parse_wal_submission_posture() {
        let cli = Cli::try_parse_from([
            "echo-cli",
            "wal",
            "submission-posture",
            "runtime.wal",
            "--submission-id",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "--canonical-envelope-digest",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        ])
        .unwrap();
        match cli.command {
            Commands::Wal {
                command:
                    WalCommands::SubmissionPosture {
                        ref root,
                        ref submission_id,
                        ref canonical_envelope_digest,
                    },
            } => {
                assert_eq!(root, &PathBuf::from("runtime.wal"));
                assert_eq!(
                    submission_id,
                    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                );
                assert_eq!(
                    canonical_envelope_digest,
                    "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                );
            }
            _ => panic!("expected Wal submission-posture command"),
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
