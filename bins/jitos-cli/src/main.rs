// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! JITOS CLI
//!
//! Command-line interface for interacting with the JITOS kernel.

use anyhow::Result;
use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Command to execute
    #[clap(subcommand)]
    cmd: Option<Command>,
}

#[derive(clap::Subcommand, Debug)]
enum Command {
    /// Get the current status of the kernel
    Status,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args.cmd {
        Some(Command::Status) => {
            println!("Status: Online (Placeholder)");
        }
        None => {
            println!("JITOS CLI. Use --help for usage.");
        }
    }

    Ok(())
}
