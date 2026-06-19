#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

set -euo pipefail

script_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "${script_root}/../.." && pwd)"
checker="${repo_root}/scripts/check-wal-wsc-doctrine.sh"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

copy_fixture() {
  local tmp="$1"
  mkdir -p "${tmp}/docs/design"
  cp "${repo_root}/docs/BEARING.md" "${tmp}/docs/BEARING.md"
  cp "${repo_root}/docs/WorkItems.md" "${tmp}/docs/WorkItems.md"
  cp \
    "${repo_root}/docs/design/work-item-sequencing-and-prioritization.md" \
    "${tmp}/docs/design/work-item-sequencing-and-prioritization.md"
  cp \
    "${repo_root}/docs/design/causal-wal-end-to-end.md" \
    "${tmp}/docs/design/causal-wal-end-to-end.md"
  cp \
    "${repo_root}/docs/design/wal-wsc-durability-roadmap.md" \
    "${tmp}/docs/design/wal-wsc-durability-roadmap.md"
}

test_current_repo_passes() {
  "$checker" >/dev/null
}

test_isolated_fixture_passes() {
  local tmp
  tmp="$(mktemp -d)"
  copy_fixture "$tmp"
  ECHO_REPO_ROOT="$tmp" "$checker" >/dev/null
  rm -rf "$tmp"
}

test_missing_bootstrap_phrase_fails() {
  local tmp out
  tmp="$(mktemp -d)"
  copy_fixture "$tmp"

  awk '
    { gsub(/configured WAL root or storage manifest/, "configured in-memory graph facts"); print }
  ' "${tmp}/docs/design/causal-wal-end-to-end.md" \
    >"${tmp}/docs/design/causal-wal-end-to-end.md.tmp"
  mv \
    "${tmp}/docs/design/causal-wal-end-to-end.md.tmp" \
    "${tmp}/docs/design/causal-wal-end-to-end.md"

  out="$({ ECHO_REPO_ROOT="$tmp" "$checker"; } 2>&1 || true)"
  echo "$out" | grep -q "WAL design names recovery bootstrap source" || {
    echo "$out" >&2
    rm -rf "$tmp"
    fail "checker did not report the missing recovery bootstrap phrase"
  }
  rm -rf "$tmp"
}

test_stale_workitems_backlog_link_fails() {
  local tmp out
  tmp="$(mktemp -d)"
  copy_fixture "$tmp"

  cat >>"${tmp}/docs/WorkItems.md" <<'EOF'

- [WAL/WSC Storage Relationship](method/backlog/v0.1.0/PLATFORM_wal-wsc-storage-relationship.md)
EOF

  out="$({ ECHO_REPO_ROOT="$tmp" "$checker"; } 2>&1 || true)"
  echo "$out" | grep -q "WorkItems removes stale WAL/WSC backlog link" || {
    echo "$out" >&2
    rm -rf "$tmp"
    fail "checker did not report the stale WorkItems WAL/WSC backlog link"
  }
  rm -rf "$tmp"
}

main() {
  [[ -x "$checker" ]] || fail "checker script missing or not executable: $checker"

  test_current_repo_passes
  test_isolated_fixture_passes
  test_missing_bootstrap_phrase_fails
  test_stale_workitems_backlog_link_fails
}

main "$@"
