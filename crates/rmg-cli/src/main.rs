//! Echo CLI entrypoint.
//!
//! Provides developer-facing commands for working with Echo projects. *Planned*
//! subcommands include `echo demo` (run deterministic demo suites), `echo
//! bench` (execute Criterion benchmarks), and `echo inspect` (open the
//! inspector tooling).
//!
//! # Usage
//! ```text
//! echo <command> [options]
//! ```
//!
//! The CLI exits with code `0` on success and non-zero on error. Until the
//! subcommands are implemented the binary simply prints a placeholder message.

#![deny(rust_2018_idioms)]
#![deny(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::dbg_macro
)]
// The CLI is expected to print to stdout/stderr.
#![allow(clippy::print_stdout, clippy::print_stderr)]

fn main() {
    println!("Echo CLI: commands coming soon. Use 'cargo test' to run the engine tests.");
}
