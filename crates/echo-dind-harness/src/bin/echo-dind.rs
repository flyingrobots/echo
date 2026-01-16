// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! CLI entry point for the DIND harness.

use anyhow::Result;
use echo_dind_harness::dind::entrypoint;

fn main() -> Result<()> {
    entrypoint()
}
