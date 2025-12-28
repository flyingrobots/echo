#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

set -euo pipefail

script_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "${script_root}/../.." && pwd)"
checker="${repo_root}/scripts/check_task_lists.sh"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

with_tmp() (
  set -euo pipefail
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' EXIT
  cd "$tmp"
  "$@"
)

test_fails_when_no_files_found() {
  with_tmp bash -c '
    set -euo pipefail
    out="$({ "'"${checker}"'" 2>&1; } || true)"
    echo "$out" | grep -q "Error: no task list files found to validate"
  '
}

test_passes_with_one_existing_file() {
  with_tmp bash -c '
    set -euo pipefail
    printf "%s\n" "- [ ] Task A" > WASM-TASKS.md
    "'"${checker}"'" >/dev/null
  '
}

test_detects_case_insensitive_conflict_within_file() {
  with_tmp bash -c '
    set -euo pipefail
    cat > WASM-TASKS.md <<EOF
- [ ] Fix the WASM compiler
- [x] fix the wasm compiler
EOF
    out="$({ "'"${checker}"'" 2>&1; } || true)"
    echo "$out" | grep -q "Task list conflict"
  '
}

test_detects_case_insensitive_conflict_across_files() {
  with_tmp bash -c '
    set -euo pipefail
    mkdir -p docs
    printf "%s\n" "- [ ] Fix the WASM compiler" > WASM-TASKS.md
    printf "%s\n" "- [x] fix the wasm compiler" > docs/tasks.md
    out="$({ "'"${checker}"'" 2>&1; } || true)"
    echo "$out" | grep -q "Task list conflict"
  '
}

main() {
  [[ -x "$checker" ]] || fail "checker script missing or not executable: $checker"

  test_fails_when_no_files_found
  test_passes_with_one_existing_file
  test_detects_case_insensitive_conflict_within_file
  test_detects_case_insensitive_conflict_across_files
}

main "$@"
