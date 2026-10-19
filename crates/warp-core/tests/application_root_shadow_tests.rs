// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![cfg(all(
    feature = "application_root_shadow",
    feature = "native_rule_bootstrap",
    feature = "trusted_runtime"
))]
#![allow(clippy::expect_used, clippy::panic)]
//! The experimental incremental application root (M1) through the F5 scene
//! fixture: the live root equals its full rebuild after every append, replay
//! reproduces it, and the legacy state root is unaffected.

#[path = "../benches/support/action_scene.rs"]
#[allow(dead_code)]
mod action_scene;
#[path = "../benches/support/fixtures/mod.rs"]
#[allow(dead_code)]
mod fixtures;

use fixtures::think_append;

#[test]
fn f5_application_root_is_incremental_exact_and_replayable() {
    // Tier M replays 5,000 appends several times; it is for optimised
    // builds. Debug builds, where `delta_validate` may also check every
    // commit against a full rebuild, run tier S.
    let tiers: &[think_append::Tier] = if cfg!(debug_assertions) {
        &[think_append::S]
    } else {
        &[think_append::S, think_append::M]
    };
    for &tier in tiers {
        let obs = think_append::run(tier, 7);
        assert_eq!(obs.accounting.committed, tier.appends, "tier {}", tier.name);
        assert_eq!(
            obs.application_root,
            Some((true, true)),
            "tier {}: incremental equals rebuild, and replay reproduces it",
            tier.name
        );
        assert_eq!(obs.replay_root, Some(obs.live_root), "tier {}", tier.name);
    }
}
