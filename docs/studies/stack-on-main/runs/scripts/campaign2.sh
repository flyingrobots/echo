#!/bin/bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
R=/private/tmp/claude-501/-Users-j-git-echo/2df23d48-982e-4dec-bd5a-3de7c779ebaf/scratchpad/runners
$R/checks2.sh /Users/j/git/echo-claude-game f6
$R/run.sh /Users/j/git/echo-claude-game f6-test test --locked -p warp-core --features native_rule_bootstrap,trusted_runtime --test scene_fixture_tests
$R/run.sh /Users/j/git/echo-claude-game f6-main-bench bench --locked -p warp-core --features native_rule_bootstrap,trusted_runtime,application_root_profile --bench game_scenes
grep '^F6 ' $R/logs/f6-main-bench.log
$R/run.sh /Users/j/git/echo-claude-bench f5d-dev bench --locked -p warp-core --features native_rule_bootstrap,trusted_runtime,application_root_profile --bench f5_durability --no-run
