#!/bin/bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
# hello-echo's CI sequence against the advanced pins, in the claude/advance-producers worktree.
set -u
S=/private/tmp/claude-501/-Users-j-git-reader/a1c895b9-b432-44d0-8f03-86a8cf7ca98f/scratchpad
N=/Users/j/.rustup/toolchains/1.96.0-aarch64-apple-darwin/bin
export PATH="$N:/Users/j/.cargo/bin:$PATH" RUSTC="$N/rustc" CARGO_TARGET_DIR="$S/m0e1/target" CARGO_INCREMENTAL=0 TMPDIR="$S/tmp"
export EDICT_REPO=/Users/j/git/edict-claude-pin ECHO_REPO=/Users/j/git/echo-claude-hello
HE=/Users/j/git/hello-echo-claude
checkout() {
  mkdir -p "$TMPDIR"
  (cd "$1" && env -u GIT_LOCKS_STORE git locks with --job "agent-$(uuidgen)" --holder claude-reader-a1c895b9 --ttl 120 --wait 600 --note "Claude: hello-echo fix checkout" .git/ -- git -C "$2" checkout -q --detach "$3") > /dev/null 2>&1
  [ "$(git -C "$2" rev-parse HEAD)" = "$(jq -r ".$4.commit" $HE/producers.lock.json)" ] || { echo "== ABORT: $2 is not at the locked $4 commit"; exit 1; }
}
checkout /Users/j/git/edict $EDICT_REPO "$(jq -r .edict.commit $HE/producers.lock.json)" edict
checkout /Users/j/git/echo $ECHO_REPO "$(jq -r .echo.commit $HE/producers.lock.json)" echo
cd $HE
step() { label=$1; shift; echo "== $label :: $*"; "$@" > $S/m0e1/logs/hfix-$label.log 2>&1; rc=$?; echo "== rc=$rc"; grep -v '^{"checked' $S/m0e1/logs/hfix-$label.log | tail -4; [ $rc -eq 0 ] || exit $rc; }
step syntax sh -c 'for s in tests/*.sh tests/lib/*.sh; do test -e "$s" || continue; sh -n "$s" || exit 1; done'
step lock ./tests/producer-lock.sh
step runtime sh -x ./tests/runtime.sh
step effect sh -x ./tests/effect-runtime.sh
step patch sh -x ./tests/patch-runtime.sh
step cleans ./tests/build-cleans-output.sh
step assertions ./tests/writer-epoch-assertions.sh
step host-fmt sh -c 'cargo fmt --manifest-path .build/effect/host/Cargo.toml -- --check && cargo fmt --manifest-path .build/patch/host/Cargo.toml -- --check'
step host-clippy sh -c 'cargo clippy --manifest-path .build/effect/host/Cargo.toml --target-dir .build/effect/host-target --all-targets -- -D warnings && cargo clippy --manifest-path .build/patch/host/Cargo.toml --target-dir .build/patch/host-target --all-targets -- -D warnings'
echo "HELLO FIX DONE"
