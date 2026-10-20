// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Allocation witnesses for Echo's hot paths.
//!
//! Measuring allocation needs a counting `#[global_allocator]`, which is a
//! `static`. Echo's runtime crates admit no global state (`ban-globals`), so the
//! witnesses live here, in a test-only crate that the runtime never depends
//! on. Each file under `tests/` is its own binary with its own allocator and
//! one measurement, so nothing else allocates while it runs.
//!
//! A witness states an unwanted scaling as a failing test: it fails on the
//! code that scaled and passes on the fix (#903).
