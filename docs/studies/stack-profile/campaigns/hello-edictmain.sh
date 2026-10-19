#!/bin/bash
# hello-echo's runtime witness with Edict at its origin/main tip, and Echo at main and at the stack.
# No expected hash is overridden.
set -u
S=/private/tmp/claude-501/-Users-j-git-reader/a1c895b9-b432-44d0-8f03-86a8cf7ca98f/scratchpad
N=/Users/j/.rustup/toolchains/1.96.0-aarch64-apple-darwin/bin
export PATH="$N:$PATH" RUSTC="$N/rustc" CARGO_TARGET_DIR="$S/m0e1/target" CARGO_INCREMENTAL=0 TMPDIR="$S/tmp"
export EDICT_REPO=/Users/j/git/edict-claude-pin ECHO_REPO=/Users/j/git/echo-claude-hello
HE=/Users/j/git/hello-echo-claude
EDICT_REV=0c8cf37
mkdir -p "$TMPDIR"
checkout() { # repo-main-dir worktree rev
  mkdir -p "$TMPDIR"
  (cd "$1" && env -u GIT_LOCKS_STORE git locks with --job "agent-$(uuidgen)" --holder claude-reader-a1c895b9 --ttl 120 --wait 600 --note "Claude: hello-echo checkout" .git/ -- git -C "$2" checkout -q --detach "$3") 2>&1
  [ "$(git -C "$2" rev-parse HEAD)" = "$(git -C "$2" rev-parse "$3^{commit}")" ] || { echo "== ABORT: $2 is not at $3"; exit 1; }
}
checkout /Users/j/git/edict $EDICT_REPO $EDICT_REV
for V in main:4e801ef2 stack:f118d7ef; do
  name=${V%%:*}; rev=${V##*:}
  checkout /Users/j/git/echo $ECHO_REPO "$rev"
  (cd $HE && jq --arg e "$(git -C $EDICT_REPO rev-parse HEAD)" --arg c "$(git -C $ECHO_REPO rev-parse HEAD)" '.edict.commit = $e | .echo.commit = $c' producers.lock.json > p.tmp && mv p.tmp producers.lock.json)
  echo "== hello-em-$name @ echo $(git -C $ECHO_REPO rev-parse --short HEAD), edict $(git -C $EDICT_REPO rev-parse --short HEAD)"
  (cd $HE && sh -x ./tests/runtime.sh) > $S/m0e1/logs/hello-em-$name.log 2>&1; rc=$?
  echo "== rc=$rc"; grep -v '^+' $S/m0e1/logs/hello-em-$name.log | tail -3; grep '^+' $S/m0e1/logs/hello-em-$name.log | tail -2
done
(cd $HE && git checkout -q -- producers.lock.json)
echo HELLO DONE
