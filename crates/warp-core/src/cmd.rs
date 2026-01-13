// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Command rewrite rules for warp-core.
//!
//! Generic engine-level commands (e.g. system management or GC triggers)
//! belong in this module. Application-specific commands should be defined
//! in application crates and registered with the engine at runtime.