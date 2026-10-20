#!/bin/bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
# Usage: run.sh WORKTREE LABEL cargo-args...  Runs cargo in WORKTREE with the pinned 1.96 toolchain,
# a scratch target and TMPDIR, logging to logs/LABEL.log.
set -uo pipefail
S="$(cd "$(dirname "$0")" && pwd)"; WT="$1"; L="$2"; shift 2
N=/Users/j/.rustup/toolchains/1.96.0-aarch64-apple-darwin/bin
export PATH="$N:$PATH" RUSTC="$N/rustc" CARGO_TARGET_DIR="$S/target" CARGO_INCREMENTAL=0 TMPDIR="$S/tmp"
mkdir -p "$TMPDIR" "$S/logs"; cd "$WT" || exit 1
echo "== $L @ $(git rev-parse --short HEAD) :: cargo $*"
cargo "$@" > "$S/logs/$L.log" 2>&1; rc=$?; echo "== rc=$rc"
grep -E 'test result|^error|^warning: unused|panicked|^test .* \.\.\. |^E1 ' "$S/logs/$L.log" | head -60
exit $rc
