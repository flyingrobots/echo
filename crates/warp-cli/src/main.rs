// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
// CLI binary — printing to stdout/stderr is the primary interface.
#![allow(clippy::print_stdout, clippy::print_stderr)]
//! Echo CLI entrypoint.
//!
//! Provides developer-facing commands for working with Echo snapshots:
//!
//! - `echo-cli verify <snapshot>` — validate WSC snapshot integrity
//! - `echo-cli bench [--filter <pattern>]` — run and format benchmarks
//! - `echo-cli inspect <snapshot> [--tree] [--raw]` — display snapshot metadata
//!
//! # Usage
//! ```text
//! echo-cli [--format text|json] <command> [options]
//! ```

use anyhow::Result;
use clap::Parser;

mod bench;
mod cli;
mod inspect;
mod output;
mod verify;
mod wsc_loader;

use cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Verify {
            ref snapshot,
            ref expected,
        } => verify::run(snapshot, expected.as_deref(), &cli.format),
        Commands::Bench {
            ref filter,
            ref baseline,
        } => bench::run(filter.as_deref(), baseline.as_deref(), &cli.format),
        Commands::Inspect {
            ref snapshot,
            tree,
            raw,
        } => inspect::run(snapshot, tree, raw, &cli.format),
    }
}
