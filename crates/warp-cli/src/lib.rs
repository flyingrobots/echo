// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Echo CLI library — re-exports CLI types for man page generation.
//!
//! The library target exists solely to let `xtask` import the `Cli` struct
//! for `clap_mangen` man page generation. The output module is included for
//! completeness but its functions are only called by the binary target.
#![allow(dead_code)]

pub mod cli;
pub(crate) mod output;
