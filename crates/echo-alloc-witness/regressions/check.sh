#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#
# Proves each allocation witness still catches the regression it exists for.
# For each pair, the witness must pass on the clean tree, then fail with its
# own assertion once the regression patch is applied. A build error or any
# other failure does not count as a catch. Run from the repository root,
# inside the guarded worker.
set -euo pipefail
dir="crates/echo-alloc-witness/regressions"
status=0
check() {
  local patch="$1" witness="$2" expected="$3" out
  if ! cargo test --locked -p echo-alloc-witness --test "$witness" >/dev/null 2>&1; then
    echo "FAIL: $witness does not pass on the clean tree" >&2
    status=1
    return
  fi
  git apply "$dir/$patch"
  out="$(cargo test --locked -p echo-alloc-witness --test "$witness" 2>&1 || true)"
  git apply -R "$dir/$patch"
  if grep -q "test result: FAILED" <<<"$out" && grep -qF "$expected" <<<"$out"; then
    echo "ok: $witness catches $patch"
  else
    echo "FAIL: $witness did not fail on its assertion with $patch applied" >&2
    tail -5 <<<"$out" >&2
    status=1
  fi
}
check bucket-copy.patch commit_hub_bucket "adding one edge allocated"
exit "$status"
