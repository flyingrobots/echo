#!/bin/bash
# Build hello-echo's application at Echo main and at the stack, and hash what the build produced.
set -u
S=/private/tmp/claude-501/-Users-j-git-reader/a1c895b9-b432-44d0-8f03-86a8cf7ca98f/scratchpad
N=/Users/j/.rustup/toolchains/1.96.0-aarch64-apple-darwin/bin
export PATH="$N:$PATH" RUSTC="$N/rustc" CARGO_TARGET_DIR="$S/m0e1/target" CARGO_INCREMENTAL=0 TMPDIR="$S/tmp"
export EDICT_REPO=/Users/j/git/edict-claude-pin ECHO_REPO=/Users/j/git/echo-claude-hello
mkdir -p "$TMPDIR"; HE=/Users/j/git/hello-echo-claude
for V in pin:490134c0753a3df6a74da366cc71c1248764dc5a main:4e801ef2 stack:f118d7ef; do
  name=${V%%:*}; rev=${V##*:}
  (cd /Users/j/git/echo && env -u GIT_LOCKS_STORE git locks with --job "agent-$(uuidgen)" --holder claude-reader-a1c895b9 --ttl 120 --wait 600 --note "Claude: hello-echo hash checkout" .git/ -- git -C $ECHO_REPO checkout -q --detach "$rev") > /dev/null 2>&1
  full=$(git -C $ECHO_REPO rev-parse HEAD); want=$(git -C $ECHO_REPO rev-parse "$rev^{commit}")
  [ "$full" = "$want" ] || { echo "ABORT $name: echo at $full"; exit 1; }
  (cd $HE && jq --arg c "$full" '.echo.commit = $c' producers.lock.json > p.tmp && mv p.tmp producers.lock.json && ./tests/build.sh > /dev/null 2>&1); rc=$?
  echo "$name echo=$(git -C $ECHO_REPO rev-parse --short HEAD) build_rc=$rc package=$(shasum -a 256 $HE/.build/application/executable-operation-package.cbor | cut -c1-16) report=$(shasum -a 256 $HE/.build/application/verification-report.cbor | cut -c1-16)"
done
(cd $HE && git checkout -q -- producers.lock.json)
grep -o 'expected_[a-z_]*=[0-9a-f]\{16\}' $HE/tests/runtime.sh
