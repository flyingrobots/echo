#!/bin/bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
# Usage: checks.sh WORKTREE LABEL [suite]  The handoff's `checks` sequence (and optionally the full
# feature suite) on a clean, committed worktree. Void unless HEAD and a clean tree hold at both ends.
set -u
S="$(cd "$(dirname "$0")" && pwd)"; WT="$1"; L="$2"; SUITE="${3:-}"
N=/Users/j/.rustup/toolchains/1.96.0-aarch64-apple-darwin/bin
export PATH="$N:$PATH" RUSTC="$N/rustc" CARGO_TARGET_DIR="$S/target" CARGO_INCREMENTAL=0 TMPDIR="$S/tmp"
mkdir -p "$TMPDIR" "$S/logs"; cd "$WT" || exit 1
H0=$(git rev-parse HEAD); D0=$(git status --porcelain | wc -l | tr -d ' ')
echo "== $L start HEAD=$H0 dirty=$D0"; [ "$D0" = 0 ] || { echo "== VOID: dirty at start"; exit 2; }
fail=0
step() { n=$1; shift; "$@" > "$S/logs/$L-$n.log" 2>&1; rc=$?; echo "== $n rc=$rc"; [ $rc -eq 0 ] || { fail=1; grep -E '^(error|warning)|FAILED|panicked|failures:' "$S/logs/$L-$n.log" | head -20; }; }
F=native_rule_bootstrap,trusted_runtime,host_test,delta_validate,application_root_profile
step fmt cargo fmt --all -- --check
step clippy-all cargo clippy --locked -p warp-core -p warp-benches --all-targets --features warp-core/native_rule_bootstrap,warp-core/trusted_runtime,warp-core/host_test,warp-core/delta_validate,warp-core/application_root_profile -- -D warnings
step clippy-lib cargo clippy --locked -p warp-core --lib -- -D warnings -D missing_docs
step spdx scripts/check_spdx.sh --check --all
step gated python3 scripts/check-feature-gated-tests.py
step globals scripts/ban-globals.sh
if [ -n "$SUITE" ]; then
  step suite cargo test --locked -p warp-core --features $F,det_fixed --no-fail-fast
  grep -E '^test result' "$S/logs/$L-suite.log" | awk '{p+=$4; f+=$6} END {print "== suite passed=" p " failed=" f}'
  grep -E '^test .* FAILED$' "$S/logs/$L-suite.log"
fi
H1=$(git rev-parse HEAD); D1=$(git status --porcelain | wc -l | tr -d ' ')
echo "== $L end HEAD=$H1 dirty=$D1"; [ "$H0" = "$H1" ] && [ "$D1" = 0 ] || { echo "== VOID: worktree changed during run"; exit 2; }
echo "== $L RESULT fail=$fail"; exit $fail
