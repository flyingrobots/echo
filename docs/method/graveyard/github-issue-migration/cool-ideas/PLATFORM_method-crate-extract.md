<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Extract method crate to its own repo

Status: active cool idea. Echo has a Rust `crates/method` library with no Echo
dependencies and a working `cargo xtask method status --json` command. The
external `/Users/james/git/method` repo already contains the TypeScript METHOD
CLI/library, drift detector, MCP server, and tests. What remains is deciding
whether the Rust crate should be extracted into that repo, published separately,
or kept as Echo-local compatibility glue.

The `crates/method/` library has zero Echo dependencies. It could
live in `~/git/method/` alongside the existing TS CLI and serve
both Rust and JS projects. The TS CLI and Rust crate would share
the same METHOD doctrine but implement it in their native ecosystems.
