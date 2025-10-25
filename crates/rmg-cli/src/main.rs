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

fn main() {
    println!("Hello, world!");
}
