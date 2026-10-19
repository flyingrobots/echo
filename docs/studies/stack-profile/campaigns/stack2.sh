#!/bin/bash
set -u
S=/private/tmp/claude-501/-Users-j-git-reader/a1c895b9-b432-44d0-8f03-86a8cf7ca98f/scratchpad
WT=/Users/j/git/echo-claude-stack; R=$S/m0e1/run.sh
co() { (cd $WT && env -u GIT_LOCKS_STORE git locks with --job "agent-$(uuidgen)" --holder claude-reader-a1c895b9 --ttl 120 --wait 600 --note "Claude: stack checkout" .git/ -- git checkout -q "$1") 2>&1 | grep -v '"event"'; echo "== HEAD $(git -C $WT rev-parse --short HEAD)"; }
export ECHO_BENCH_REWRITES=100,10000 ECHO_BENCH_WORK=0,100 ECHO_BENCH_WORKERS=1,18 ECHO_BENCH_ROUNDS=25
for V in base:a0bd72da new:claude/stack-envelope; do
  name=${V%%:*}; rev=${V##*:}; co $rev
  $R $WT s2-$name-workers bench --locked -p warp-benches --bench scheduler_workers
  $R $WT s2-$name-f5 bench --locked -p warp-core --features native_rule_bootstrap,trusted_runtime,application_root_profile --bench scene_fixtures
done
$R $WT s2-new-full test --locked -p warp-core --features native_rule_bootstrap,trusted_runtime,host_test,delta_validate,application_root_profile,det_fixed --no-fail-fast
$S/combo/checks.sh
echo CAMPAIGN DONE
