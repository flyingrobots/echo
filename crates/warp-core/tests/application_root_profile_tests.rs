// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! The experimental application-root commitment profile (M2) through the F5
//! scene fixture: the same workload validates under both profiles, the
//! application profile commits a different, tagged root, and replay and
//! readings at earlier coordinates verify under it.

#[cfg(all(
    feature = "application_root_profile",
    feature = "native_rule_bootstrap",
    feature = "trusted_runtime"
))]
#[path = "../benches/support/action_scene.rs"]
#[allow(dead_code)]
mod action_scene;
#[cfg(all(
    feature = "application_root_profile",
    feature = "native_rule_bootstrap",
    feature = "trusted_runtime"
))]
#[path = "../benches/support/fixtures/mod.rs"]
#[allow(dead_code)]
mod fixtures;

#[cfg(all(
    feature = "application_root_profile",
    feature = "native_rule_bootstrap",
    feature = "trusted_runtime"
))]
use fixtures::{manifest::hex32, think_append};
#[cfg(all(
    feature = "application_root_profile",
    feature = "native_rule_bootstrap",
    feature = "trusted_runtime"
))]
use warp_core::application_root::CommitmentProfile;

#[cfg(all(
    feature = "application_root_profile",
    feature = "native_rule_bootstrap",
    feature = "trusted_runtime"
))]
#[test]
fn f5_validates_under_the_application_profile() {
    let tiers: &[think_append::Tier] = if cfg!(debug_assertions) {
        &[think_append::S]
    } else {
        &[think_append::S, think_append::M]
    };
    for &tier in tiers {
        let script = think_append::script(tier, 7);
        let digest = hex32(&think_append::script_digest(&script));
        let expect = think_append::expectation(tier, Some(&digest));
        let legacy = think_append::run(tier, 7);
        let application =
            think_append::run_with_profile(tier, 7, CommitmentProfile::ApplicationRadixV0);
        for (label, obs) in [("legacy", &legacy), ("application", &application)] {
            let class = think_append::validate(tier, &script, &expect, obs);
            assert!(class.comparable(), "tier {} {label}: {class:?}", tier.name);
            assert_eq!(
                obs.replay_root,
                Some(obs.live_root),
                "tier {} {label}",
                tier.name
            );
        }
        assert_ne!(
            legacy.live_root, application.live_root,
            "tier {}: the profiles commit different roots for the same graph",
            tier.name
        );
        assert_eq!(application.application_root, Some((true, true)));
    }
}
