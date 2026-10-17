#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#
# Proves each allocation witness still catches the regression it exists for:
# apply the regression patch, require the witness to FAIL, then revert. A
# witness that passes with its regression applied has stopped measuring.
# Run from the repository root, inside the guarded worker.
set -euo pipefail
dir="crates/echo-alloc-witness/regressions"
status=0
check() {
  local patch="$1" witness="$2"
  git apply "$dir/$patch"
  if cargo test --locked -p echo-alloc-witness --test "$witness" >/dev/null 2>&1; then
    echo "FAIL: $witness passed with $patch applied" >&2
    status=1
  else
    echo "ok: $witness catches $patch"
  fi
  git apply -R "$dir/$patch"
}
check bucket-copy.patch commit_hub_bucket
exit "$status"
