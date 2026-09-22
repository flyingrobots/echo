<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Runtime for Rust

`flyingrobots-echo-runtime` exposes the Rust library name `echo_runtime`. It is
the first sealed host facade for constructing and recovering a local Echo
runtime and WAL shell without exposing native rule registration or raw receipt,
authority-epoch, and commit constructors.

Package installation, application submission, scheduling, and receipt access
are not yet exposed through this facade. Those capabilities remain release
engineering work and must be added only through bounded, authority-safe APIs.

This package is an unreleased alpha boundary with `publish = false`. It is not
available on crates.io and does not authorize publication.
