// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::print_stdout, clippy::print_stderr)]
//! Echo CLI library — re-exports CLI types for man page generation.
//!
//! The library target exists solely to let `xtask` import the `Cli` struct
//! for `clap_mangen` man page generation. The output module is included for
//! completeness but its functions are only called by the binary target.
pub mod cli;
#[allow(dead_code)]
pub(crate) mod output;
