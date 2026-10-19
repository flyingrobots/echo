#!/bin/bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
# hello-echo's runtime witness against Echo at its pin, at main, and at the stack.
set -u
S=/private/tmp/claude-501/-Users-j-git-reader/a1c895b9-b432-44d0-8f03-86a8cf7ca98f/scratchpad
N=/Users/j/.rustup/toolchains/1.96.0-aarch64-apple-darwin/bin
export PATH="$N:$PATH" RUSTC="$N/rustc" CARGO_TARGET_DIR="$S/m0e1/target" CARGO_INCREMENTAL=0 TMPDIR="$S/tmp"
export EDICT_REPO=/Users/j/git/edict-claude-pin ECHO_REPO=/Users/j/git/echo-claude-hello
HE=/Users/j/git/hello-echo-claude
mkdir -p "$TMPDIR"
for V in main:4e801ef2 stack:f118d7ef; do
  (cd $HE && sed -i '' 's/^expected_verification_report_sha256=.*/expected_verification_report_sha256=e3e3f37cf970fc12c630f1bdc649a1941aebe36fc38dfcfe55f22ef8933c3f78/' tests/runtime.sh)
  name=${V%%:*}; rev=${V##*:}
  mkdir -p "$TMPDIR"
  (cd /Users/j/git/echo && env -u GIT_LOCKS_STORE git locks with --job "agent-$(uuidgen)" --holder claude-reader-a1c895b9 --ttl 120 --wait 600 --note "Claude: hello-echo echo checkout" .git/ -- git -C $ECHO_REPO checkout -q --detach "$rev") 2>&1
  full=$(git -C $ECHO_REPO rev-parse HEAD)
  want=$(git -C $ECHO_REPO rev-parse "$rev^{commit}")
  if [ "$full" != "$want" ]; then echo "== ABORT: echo is at $full, wanted $want"; exit 1; fi
  (cd $HE && jq --arg c "$full" '.echo.commit = $c' producers.lock.json > producers.lock.json.tmp && mv producers.lock.json.tmp producers.lock.json)
  echo "== hello-override-$name @ echo $(git -C $ECHO_REPO rev-parse --short HEAD), edict $(git -C $EDICT_REPO rev-parse --short HEAD)"
  (cd $HE && ./tests/runtime.sh) > $S/m0e1/logs/hello-override-$name.log 2>&1; rc=$?
  echo "== rc=$rc"; tail -4 $S/m0e1/logs/hello-override-$name.log
done
(cd $HE && git checkout -q -- producers.lock.json tests/runtime.sh)
echo HELLO DONE
