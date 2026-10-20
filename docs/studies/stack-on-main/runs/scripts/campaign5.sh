#!/bin/bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
# Campaign 5: hello-echo CI sequence against Echo main 0faaf58a; checks on the stack tip after the bench lint fix.
set -u
R=/private/tmp/claude-501/-Users-j-git-echo/2df23d48-982e-4dec-bd5a-3de7c779ebaf/scratchpad/runners; WT=/Users/j/git/echo-claude-run
co() { (cd $WT && git locks with --job "agent-$(uuidgen)" --holder claude-echo-2df23d48 --ttl 120 --wait 600 --note "Claude: run checkout" .git/ -- git checkout -q --detach "$1"); h=$(git -C $WT rev-parse HEAD); echo "== HEAD $h"; case $h in $1*) ;; *) echo "== ABORT: checkout of $1 gave $h"; exit 1;; esac; [ -z "$(git -C $WT status --porcelain)" ] || { echo "== ABORT: dirty"; exit 1; }; }
echo "== hello @ $(git -C /Users/j/git/hello-echo-claude rev-parse --short HEAD), echo $(jq -r .echo.commit /Users/j/git/hello-echo-claude/producers.lock.json | cut -c1-8), edict $(jq -r .edict.commit /Users/j/git/hello-echo-claude/producers.lock.json | cut -c1-7)"
$R/hello-fix.sh; echo "== hello rc=$?"
co b0cef149
$R/checks2.sh $WT c5-tip
echo CAMPAIGN DONE
