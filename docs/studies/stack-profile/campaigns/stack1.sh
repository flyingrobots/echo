#!/bin/bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
set -u
S=/private/tmp/claude-501/-Users-j-git-reader/a1c895b9-b432-44d0-8f03-86a8cf7ca98f/scratchpad
WT=/Users/j/git/echo-claude-stack; R=$S/m0e1/run.sh
co() { (cd $WT && env -u GIT_LOCKS_STORE git locks with --job "agent-$(uuidgen)" --holder claude-reader-a1c895b9 --ttl 120 --wait 600 --note "Claude: stack checkout" .git/ -- git checkout -q "$1") 2>&1 | grep -v '"event"'; echo "== HEAD $(git -C $WT rev-parse --short HEAD)"; }
co f5d8b71b
$R $WT stack-workers-thread bench --locked -p warp-benches --bench scheduler_workers
co claude/stack-envelope
$R $WT stack-workers-inline bench --locked -p warp-benches --bench scheduler_workers
$R $WT stack-inline-f5 bench --locked -p warp-core --features native_rule_bootstrap,trusted_runtime,application_root_profile --bench scene_fixtures
$R $WT stack-inline-blame bench --locked -p warp-benches --bench scheduler_conflict_blame
$R $WT stack-inline-full test --locked -p warp-core --features native_rule_bootstrap,trusted_runtime,host_test,delta_validate,application_root_profile,det_fixed --no-fail-fast
echo CAMPAIGN DONE
