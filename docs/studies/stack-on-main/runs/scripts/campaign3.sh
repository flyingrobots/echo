#!/bin/bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
# Measurement campaign 3: main 0faaf58a + benches (73e17289), + inline worker (af3c3e7c), + commit path (21c6531e).
set -u
R=/private/tmp/claude-501/-Users-j-git-echo/2df23d48-982e-4dec-bd5a-3de7c779ebaf/scratchpad/runners; WT=/Users/j/git/echo-claude-run
co() { (cd $WT && git locks with --job "agent-$(uuidgen)" --holder claude-echo-2df23d48 --ttl 120 --wait 600 --note "Claude: run checkout" .git/ -- git checkout -q --detach "$1"); h=$(git -C $WT rev-parse HEAD); echo "== HEAD $h"; case $h in $1*) ;; *) echo "== ABORT: checkout of $1 gave $h"; exit 1;; esac; [ -z "$(git -C $WT status --porcelain)" ] || { echo "== ABORT: dirty"; exit 1; }; }
F=native_rule_bootstrap,trusted_runtime,application_root_profile
mkdir -p $R/wal
for V in base:73e17289 inline:af3c3e7c both:21c6531e; do
  n=${V%%:*}; rev=${V##*:}; co $rev
  ECHO_BENCH_REWRITES=100,10000 ECHO_BENCH_WORK=0,100 ECHO_BENCH_WORKERS=1,18 ECHO_BENCH_ROUNDS=25 $R/run.sh $WT c3-$n-workers bench --locked -p warp-benches --bench scheduler_workers
  $R/run.sh $WT c3-$n-f5 bench --locked -p warp-core --features $F --bench scene_fixtures
  $R/run.sh $WT c3-$n-f6 bench --locked -p warp-core --features $F --bench game_scenes
  grep '^F6 .*frames=' $R/logs/c3-$n-f6.log
done
for V in base:73e17289 both:21c6531e; do
  n=${V%%:*}; rev=${V##*:}; co $rev
  rm -rf $R/wal/*; ECHO_F5_WAL_DIR=$R/wal $R/run.sh $WT c3-$n-f5d bench --locked -p warp-core --features $F --bench f5_durability
  grep '^F5D .*summary\|^F5D census\|skipped' $R/logs/c3-$n-f5d.log
done
echo CAMPAIGN DONE
