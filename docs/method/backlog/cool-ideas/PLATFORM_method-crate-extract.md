<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Extract method crate to its own repo

The `crates/method/` library has zero Echo dependencies. It could
live in `~/git/method/` alongside the existing TS CLI and serve
both Rust and JS projects. The TS CLI and Rust crate would share
the same METHOD doctrine but implement it in their native ecosystems.
