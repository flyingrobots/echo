#!/bin/bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
# Combo campaign: M2 alone (2e79511c) vs M2+lazy (e9f29548), without footprint_enforce_release
# (Edict checks footprints at compile time), then the combo's full suite with every other compatible feature.
set -u
S=/private/tmp/claude-501/-Users-j-git-reader/a1c895b9-b432-44d0-8f03-86a8cf7ca98f/scratchpad
WT=/Users/j/git/echo-claude-combo; R=$S/m0e1/run.sh
co() { (cd $WT && env -u GIT_LOCKS_STORE git locks with --job "agent-$(uuidgen)" --holder claude-reader-a1c895b9 --ttl 120 --wait 600 --note "Claude: combo checkout" .git/ -- git checkout -q "$1") 2>&1 | grep -v '"event"'; echo "== HEAD $(git -C $WT rev-parse --short HEAD)"; }
F5=native_rule_bootstrap,trusted_runtime,application_root_profile
for V in m2:2e79511c combo:e9f29548; do
  name=${V%%:*}; rev=${V##*:}; co $rev
  [ $name = m2 ] || $R $WT combo-$name-blame bench --locked -p warp-benches --bench scheduler_conflict_blame
  $R $WT combo-$name-f5 bench --locked -p warp-core --features $F5 --bench scene_fixtures
done
co claude/combo-m2-lazy
$R $WT combo-full-all test --locked -p warp-core --features native_rule_bootstrap,trusted_runtime,host_test,delta_validate,application_root_profile,det_fixed --no-fail-fast
echo CAMPAIGN DONE
