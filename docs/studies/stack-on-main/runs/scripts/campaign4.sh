#!/bin/bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
# Campaign 4: final checks on every candidate; allocation census; profiles of the 10k tick and F6 pile at the stack tip.
set -u
R=/private/tmp/claude-501/-Users-j-git-echo/2df23d48-982e-4dec-bd5a-3de7c779ebaf/scratchpad/runners; WT=/Users/j/git/echo-claude-run; TIP=$1
N=/Users/j/.rustup/toolchains/1.96.0-aarch64-apple-darwin/bin
co() { (cd $WT && git locks with --job "agent-$(uuidgen)" --holder claude-echo-2df23d48 --ttl 120 --wait 600 --note "Claude: run checkout" .git/ -- git checkout -q --detach "$1"); h=$(git -C $WT rev-parse HEAD); echo "== HEAD $h"; case $h in $1*) ;; *) echo "== ABORT: checkout of $1 gave $h"; exit 1;; esac; [ -z "$(git -C $WT status --porcelain)" ] || { echo "== ABORT: dirty"; exit 1; }; }
$R/checks2.sh /Users/j/git/echo-claude-inline c4-inline
$R/checks2.sh /Users/j/git/echo-claude-commitpath c4-commitpath
$R/checks2.sh /Users/j/git/echo-claude-game c4-f6
co $TIP
$R/checks2.sh $WT c4-tip suite
$R/run.sh $WT c4-alloc test --locked -p echo-alloc-witness --test tick_allocation_census -- --ignored --nocapture
grep '^ALLOC' $R/logs/c4-alloc.log
F=native_rule_bootstrap,trusted_runtime,application_root_profile
prof() { # LABEL LOG DELAY SECONDS env... -- binary-args
  label=$1; bin=$(grep -o '(/[^)]*)' $R/logs/$2.log | tr -d '()' | tail -1); delay=$3; secs=$4; shift 4
  echo "== profile $label binary $bin"
  (cd $WT && env "$@" "$bin" > $R/logs/$label-run.log 2>&1) & pid=$!
  sleep $delay; child=$(pgrep -P $pid | head -1); target=${child:-$pid}
  sample $target $secs 1 -file $R/logs/$label.txt > /dev/null 2>&1; echo "== sample rc=$? target=$target"
  pkill -P $pid 2>/dev/null; kill $pid 2>/dev/null; wait $pid 2>/dev/null
}
$R/run.sh $WT c4-build-workers bench --locked -p warp-benches --bench scheduler_workers --no-run
prof c4-prof-10k c4-build-workers 4 20 ECHO_BENCH_REWRITES=10000 ECHO_BENCH_WORK=0 ECHO_BENCH_WORKERS=1 ECHO_BENCH_ROUNDS=400
$R/run.sh $WT c4-build-f6 bench --locked -p warp-core --features $F --bench game_scenes --no-run
prof c4-prof-pile c4-build-f6 10 20 ECHO_UNUSED=1
echo CAMPAIGN DONE
