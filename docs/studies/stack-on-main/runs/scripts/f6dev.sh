#!/bin/bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
# F6 development build in /Users/j/git/echo-claude-game: the debug test, then the bench build.
R="$(cd "$(dirname "$0")" && pwd)"; WT=/Users/j/git/echo-claude-game
$R/run.sh $WT f6dev-test test --locked -p warp-core --features native_rule_bootstrap,trusted_runtime --test scene_fixture_tests
$R/run.sh $WT f6dev-bench bench --locked -p warp-core --features native_rule_bootstrap,trusted_runtime,application_root_profile --bench game_scenes --no-run
